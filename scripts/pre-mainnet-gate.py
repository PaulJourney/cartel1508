#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CONSTANTS = ROOT / "programs/service_referral_protocol/src/constants.rs"
CORE_LIB = ROOT / "programs/service_referral_protocol/src/lib.rs"
STATE = ROOT / "programs/service_referral_protocol/src/state.rs"
MATH = ROOT / "programs/service_referral_protocol/src/math.rs"
ANCHOR = ROOT / "Anchor.toml"
MANIFEST = ROOT / "release/mainnet-release.json"
CORE_ARTIFACT = ROOT / "target/deploy/service_referral_protocol.so"

EXPECTED = {
    "service_treasury": "AepYo8xanmKuRiLVeYQuCTJoQr1nyKiTApoKwHMEg8fn",
    "usdt_mint": "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB",
    "usdc_mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    "economics_profile": "SELF50_NETWORK43_PIONEER2_SERVICE5",
    "activity_profile": "ACTIVE_WEEKS_10_10_20_20_30_30_40_40_50_CAP",
    "depth_profile": "WEEKLY_DEPTH_10U3_25U4_50U5_100U6_200U7_350U8_500U9",
    "pioneer_position_cap": 100,
    "pioneer_single_purchase_units": 1000,
    "pioneer_rule_b": True,
}
DEVELOPMENT_PROGRAM_IDS = {"4AuoBkj4vkH2K1jwUuECtVBqF6Q74efjGbaw7btuNjRV"}
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
ACTIONS_RUN_RE = re.compile(r"^https://github\.com/[^/]+/[^/]+/actions/runs/[0-9]+(?:/.*)?$")


def extract(pattern: str, text: str, label: str) -> str:
    match = re.search(pattern, text)
    if not match:
        raise RuntimeError(f"cannot parse {label}")
    return match.group(1)


def declare_id(text: str) -> str:
    return extract(r'declare_id!\("([1-9A-HJ-NP-Za-km-z]+)"\)', text, "core declare_id")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def git_output(*args: str) -> str | None:
    try:
        return subprocess.check_output(["git", *args], cwd=ROOT, text=True, stderr=subprocess.DEVNULL).strip()
    except Exception:
        return None


def release_source_sha() -> str | None:
    return os.environ.get("RELEASE_SOURCE_SHA", "").strip() or git_output("rev-parse", "HEAD")


def tracked_secret_candidates() -> list[str]:
    tracked = git_output("ls-files")
    if tracked is None:
        return []
    bad: list[str] = []
    for name in tracked.splitlines():
        lowered = name.lower()
        if lowered.endswith("keypair.json") or "seed" in lowered or "private-key" in lowered or "private_key" in lowered:
            bad.append(name)
    return bad


def main() -> int:
    blockers: list[str] = []
    notes: list[str] = []

    constants = CONSTANTS.read_text()
    core_lib = CORE_LIB.read_text()
    state = STATE.read_text()
    math = MATH.read_text()
    anchor = ANCHOR.read_text()

    program_id = declare_id(core_lib)
    treasury = extract(r'MAINNET_SERVICE_TREASURY: Pubkey\s*=\s*pubkey!\("([1-9A-HJ-NP-Za-km-z]+)"\)', constants, "mainnet treasury")
    usdt = extract(r'MAINNET_USDT_MINT: Pubkey\s*=\s*pubkey!\("([1-9A-HJ-NP-Za-km-z]+)"\)', constants, "USDT mint")
    usdc = extract(r'MAINNET_USDC_MINT: Pubkey\s*=\s*pubkey!\("([1-9A-HJ-NP-Za-km-z]+)"\)', constants, "USDC mint")
    registration_open = int(extract(r'MAINNET_REGISTRATION_OPEN_AT: i64 = (-?\d+)', constants, "registration open timestamp"))

    for field, actual in (("service_treasury", treasury), ("usdt_mint", usdt), ("usdc_mint", usdc)):
        if actual != EXPECTED[field]:
            blockers.append(f"{field} mismatch: {actual}")

    if program_id in DEVELOPMENT_PROGRAM_IDS:
        blockers.append("core Program ID is still a development identity")

    if 'PURCHASE_NETWORK_LEVEL_BPS: [u64; 9]' not in constants or '[1_500, 900, 600, 400, 250, 200, 150, 100, 200]' not in constants:
        blockers.append("final nine-upline 43% production schedule is not frozen")
    if 'split_purchase_amount(payment)' not in core_lib:
        blockers.append("purchase_and_distribute is not using the final purchase split")
    if 'for i in 1..9' not in core_lib or 'pub upline_8:' not in core_lib or 'pub upline_9:' in core_lib:
        blockers.append("production referral traversal is not frozen to sponsor plus eight ancestors")
    if 'ACTIVE_WEEK_REQUIREMENT_UNITS: [u64; 5] = [10, 20, 30, 40, 50]' not in constants or 'ACTIVE_WEEK_TIER_SPAN: u32 = 2' not in constants:
        blockers.append("progressive ACTIVE weekly schedule is not frozen")
    if 'NETWORK_DEPTH_UNIT_THRESHOLDS: [u64; 7] = [10, 25, 50, 100, 200, 350, 500]' not in constants:
        blockers.append("weekly U1-U9 depth schedule is not frozen")
    if 'pub active_weeks_started: u32' not in state or 'pub current_week_units: u64' not in state:
        blockers.append("weekly activity/depth state is missing")
    if 'distribute_network_level(' not in core_lib or 'network_depth_for_units(user.current_week_units)' not in core_lib:
        blockers.append("network payouts are not gated by weekly personal-unit depth")
    if 'let pioneer = pioneer_due(user, p, mint)?' not in core_lib or 'preserve_self_and_pioneer' in core_lib:
        blockers.append("Pioneer is not strictly ACTIVE/GRACE-gated after inactivity")

    # Pioneer release invariants are mainnet gates, not documentation-only rules.
    if 'PIONEER_SLOTS: u16 = 100' not in constants:
        blockers.append("Pioneer global position cap is not frozen to 100")
    if 'PIONEER_POSITION_PURCHASE_UNITS: u64 = 1_000' not in constants:
        blockers.append("Pioneer single-purchase threshold is not frozen to 1000 units")
    if 'pub fn pioneer_positions_for_purchase' not in math:
        blockers.append("Pioneer purchase-position calculator is missing")
    else:
        if 'units / PIONEER_POSITION_PURCHASE_UNITS' not in math:
            blockers.append("Pioneer positions are not derived from one purchase only")
        if 'requested.min(remaining)' not in math or 'already_assigned >= PIONEER_SLOTS' not in math:
            blockers.append("Pioneer purchase-position calculator does not enforce the absolute 100 cap")
    if 'pub pioneer_positions_assigned: u16' not in state or 'pub pioneer_positions: u16' not in state:
        blockers.append("weighted Pioneer position state is not present")
    if 'pioneer_id' in state or 'pioneer_id' in core_lib:
        blockers.append("legacy one-Pioneer-ID-per-registration state remains")
    if 'u.pioneer_positions = 0' not in core_lib:
        blockers.append("registration does not explicitly start with zero Pioneer positions")
    accrue_at = core_lib.find('let pioneer_unassigned = accrue_pioneer')
    assign_at = core_lib.find('assign_pioneer_positions_after_purchase')
    if accrue_at < 0 or assign_at < 0 or accrue_at >= assign_at:
        blockers.append("Pioneer Rule B is not frozen: current-event accrual must precede new-position assignment")
    if 'checked_mul(user.pioneer_positions as u128)' not in core_lib:
        blockers.append("Pioneer entitlement is not weighted by wallet position count")

    if 'pub struct UnitsPurchased' not in core_lib or 'emit!(UnitsPurchased' not in core_lib:
        blockers.append("purchase unit ranges are not emitted as the final event-based audit trail")
    if 'pub pioneer_positions_added: u16' not in core_lib or 'pub pioneer_positions_total: u16' not in core_lib:
        blockers.append("purchase event does not expose Pioneer position additions/total")
    purchase_accounts = core_lib.split('pub struct PurchaseAndDistribute', 1)[1].split('pub struct SettleExpired', 1)[0]
    if 'UnitBatch' in core_lib or 'pub batch:' in purchase_accounts or 'pub system_program' in purchase_accounts:
        blockers.append("purchase still carries a per-purchase rent/account-creation surface")

    legacy_markers = [
        'purchase_service_units',
        'record_qualified_revenue',
        'RecordQualifiedRevenue',
        'qualified_revenue_source',
        'MAINNET_QUALIFIED_REVENUE_SOURCE',
        'MAINNET_REVENUE_ADAPTER_PROGRAM',
        'LegacyRevenuePathDisabled',
    ]
    for marker in legacy_markers:
        if marker in core_lib or marker in state or marker in constants:
            blockers.append(f"legacy revenue marker remains in final core: {marker}")

    if registration_open <= 0:
        blockers.append("registration_open_at is not frozen to a positive UTC unix timestamp")
    elif registration_open <= int(time.time()):
        blockers.append("registration_open_at is not in the future")

    mainnet_match = re.search(r'\[programs\.mainnet\][\s\S]*?service_referral_protocol\s*=\s*"([1-9A-HJ-NP-Za-km-z]+)"', anchor)
    if not mainnet_match:
        blockers.append("Anchor.toml has no [programs.mainnet] Program ID")
    elif mainnet_match.group(1) != program_id:
        blockers.append("Anchor.toml mainnet Program ID does not match core declare_id!")

    tracked = tracked_secret_candidates()
    if tracked:
        blockers.append("secret-like deployment files are tracked: " + ", ".join(tracked))

    source_sha = release_source_sha()
    if not source_sha:
        blockers.append("cannot resolve release source git SHA")
    else:
        notes.append(f"release source SHA: {source_sha}")

    git_status = git_output("status", "--porcelain", "--untracked-files=all")
    if git_status is None:
        blockers.append("cannot verify git working tree cleanliness")
    elif git_status:
        blockers.append("git working tree is not clean")

    if not MANIFEST.exists():
        blockers.append("release/mainnet-release.json is missing")
        manifest: dict = {}
    else:
        try:
            manifest = json.loads(MANIFEST.read_text())
        except Exception as exc:
            blockers.append(f"release manifest is invalid JSON: {exc}")
            manifest = {}

    required = [
        "commit_sha", "program_id", "registration_open_at", "service_treasury",
        "usdt_mint", "usdc_mint", "economics_profile", "activity_profile", "depth_profile", "pioneer_position_cap",
        "pioneer_single_purchase_units", "pioneer_rule_b", "so_sha256",
        "audit_report_sha256", "protocol_ci_run_url", "rustsec_run_url",
        "verified_build_run_url", "devnet_smoke_run_url",
        "devnet_smoke_evidence_sha256", "devnet_smoke_status", "audit_status",
        "smoke_test_plan_approved",
    ]
    for key in required:
        if manifest.get(key) in (None, "", 0, False):
            blockers.append(f"release manifest field not frozen: {key}")

    expected_manifest = {
        "program_id": program_id,
        "registration_open_at": registration_open,
        **EXPECTED,
    }
    for key, expected in expected_manifest.items():
        if manifest.get(key) != expected:
            blockers.append(f"release manifest {key} does not match frozen source/economics")

    if source_sha and manifest.get("commit_sha") != source_sha:
        blockers.append("release manifest commit_sha does not match release source SHA")

    for key in ("so_sha256", "audit_report_sha256", "devnet_smoke_evidence_sha256"):
        value = manifest.get(key)
        if value and not SHA256_RE.fullmatch(str(value)):
            blockers.append(f"release manifest {key} is not a lowercase SHA-256 digest")

    for key in ("protocol_ci_run_url", "rustsec_run_url", "verified_build_run_url", "devnet_smoke_run_url"):
        value = manifest.get(key)
        if value and not ACTIONS_RUN_RE.fullmatch(str(value)):
            blockers.append(f"{key} is not a GitHub Actions run URL")

    if manifest.get("devnet_smoke_status") != "passed":
        blockers.append("final devnet smoke status is not 'passed'")
    if manifest.get("audit_status") != "passed":
        blockers.append("independent audit status is not 'passed'")
    if manifest.get("smoke_test_plan_approved") is not True:
        blockers.append("mainnet smoke-test plan is not approved")

    expected_so_hash = manifest.get("so_sha256")
    if not CORE_ARTIFACT.exists():
        blockers.append("core production .so is missing; exact artifact verification is mandatory")
    elif expected_so_hash and SHA256_RE.fullmatch(str(expected_so_hash)):
        actual = sha256_file(CORE_ARTIFACT)
        if actual != expected_so_hash:
            blockers.append("local core .so SHA-256 does not match release manifest")
        else:
            notes.append(f"core artifact SHA-256 verified: {actual}")

    print("=== PRE-MAINNET GATE ===")
    print(f"Core Program ID:       {program_id}")
    print(f"Registration opens:    {registration_open}")
    print(f"Service treasury:      {treasury}")
    print(f"USDT mint:             {usdt}")
    print(f"USDC mint:             {usdc}")
    for note in notes:
        print(f"NOTE: {note}")

    if blockers:
        print("\nBLOCKED:")
        for blocker in blockers:
            print(f"- {blocker}")
        return 1

    print("\nPASS: final core-only release gate is green")
    return 0


if __name__ == "__main__":
    sys.exit(main())
