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
ANCHOR = ROOT / "Anchor.toml"
MANIFEST = ROOT / "release/mainnet-release.json"
CORE_ARTIFACT = ROOT / "target/deploy/service_referral_protocol.so"

EXPECTED = {
    "service_treasury": "AepYo8xanmKuRiLVeYQuCTJoQr1nyKiTApoKwHMEg8fn",
    "usdt_mint": "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB",
    "usdc_mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
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
    anchor = ANCHOR.read_text()

    program_id = declare_id(core_lib)
    treasury = extract(r'MAINNET_SERVICE_TREASURY: Pubkey = pubkey!\("([1-9A-HJ-NP-Za-km-z]+)"\)', constants, "mainnet treasury")
    usdt = extract(r'MAINNET_USDT_MINT: Pubkey = pubkey!\("([1-9A-HJ-NP-Za-km-z]+)"\)', constants, "USDT mint")
    usdc = extract(r'MAINNET_USDC_MINT: Pubkey = pubkey!\("([1-9A-HJ-NP-Za-km-z]+)"\)', constants, "USDC mint")
    registration_open = int(extract(r'MAINNET_REGISTRATION_OPEN_AT: i64 = (-?\d+)', constants, "registration open timestamp"))

    for field, actual in (("service_treasury", treasury), ("usdt_mint", usdt), ("usdc_mint", usdc)):
        if actual != EXPECTED[field]:
            blockers.append(f"{field} mismatch: {actual}")

    if program_id in DEVELOPMENT_PROGRAM_IDS:
        blockers.append("core Program ID is still a development identity")

    if 'PURCHASE_NETWORK_LEVEL_BPS: [u64; 9] = [1_500, 900, 600, 400, 250, 200, 150, 100, 200]' not in constants:
        blockers.append("final L2-L10 43% production schedule is not frozen")
    if 'split_purchase_amount(payment)' not in core_lib:
        blockers.append("purchase_and_distribute is not using the final purchase split")
    if 'for i in 0..9' not in core_lib:
        blockers.append("production referral traversal is not capped at genealogical L10")
    if 'LegacyRevenuePathDisabled' not in core_lib:
        blockers.append("legacy revenue paths are not fail-closed in production")

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
        "usdt_mint", "usdc_mint", "so_sha256", "audit_report_sha256",
        "verified_build_run_url", "audit_status", "smoke_test_plan_approved",
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
            blockers.append(f"release manifest {key} does not match frozen source")

    if source_sha and manifest.get("commit_sha") != source_sha:
        blockers.append("release manifest commit_sha does not match release source SHA")

    for key in ("so_sha256", "audit_report_sha256"):
        value = manifest.get(key)
        if value and not SHA256_RE.fullmatch(str(value)):
            blockers.append(f"release manifest {key} is not a lowercase SHA-256 digest")

    build_url = manifest.get("verified_build_run_url")
    if build_url and not ACTIONS_RUN_RE.fullmatch(str(build_url)):
        blockers.append("verified_build_run_url is not a GitHub Actions run URL")

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
