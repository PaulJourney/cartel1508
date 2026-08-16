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
ADAPTER_LIB = ROOT / "programs/revenue_adapter/src/lib.rs"
QUALIFICATION_LIB = ROOT / "programs/revenue_qualification/src/lib.rs"
EVIDENCE_LIB = ROOT / "programs/revenue_evidence/src/lib.rs"
QUALIFICATION_SPEC = ROOT / "QUALIFIED_REVENUE_QUALIFICATION_SPEC.md"
ANCHOR = ROOT / "Anchor.toml"
MANIFEST = ROOT / "release/mainnet-release.json"
CORE_ARTIFACT = ROOT / "target/deploy/service_referral_protocol.so"
ADAPTER_ARTIFACT = ROOT / "programs/revenue_adapter/target/deploy/revenue_adapter.so"
QUALIFICATION_ARTIFACT = ROOT / "programs/revenue_qualification/target/deploy/revenue_qualification.so"
EVIDENCE_ARTIFACT = ROOT / "programs/revenue_evidence/target/deploy/revenue_evidence.so"

EXPECTED = {
    "service_treasury": "AepYo8xanmKuRiLVeYQuCTJoQr1nyKiTApoKwHMEg8fn",
    "usdt_mint": "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB",
    "usdc_mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
}
SENTINEL = "11111111111111111111111111111111"
DEVELOPMENT_PROGRAM_IDS = {
    "4AuoBkj4vkH2K1jwUuECtVBqF6Q74efjGbaw7btuNjRV",
    "Gxf1ikEvwiusChxqj5jkQPvWhFhFw91nyYFNL6oT7ZLD",
    "AZQHbWahShE5oLKG3BXCrqmoMj6WWtiuxhnCcMp4YoE4",
}
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
ACTIONS_RUN_RE = re.compile(r"^https://github\.com/[^/]+/[^/]+/actions/runs/[0-9]+(?:/.*)?$")


def extract(pattern: str, text: str, label: str) -> str:
    match = re.search(pattern, text)
    if not match:
        raise RuntimeError(f"cannot parse {label}")
    return match.group(1)


def declare_id(text: str, label: str) -> str:
    return extract(r'declare_id!\("([1-9A-HJ-NP-Za-km-z]+)"\)', text, label)


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


def qualification_status(text: str) -> str | None:
    matches = re.findall(r"^QUALIFICATION_SPEC_STATUS:\s*([A-Z_]+)\s*$", text, flags=re.MULTILINE)
    return matches[0] if len(matches) == 1 else None


def validate_sha_field(manifest: dict, key: str, blockers: list[str]) -> str | None:
    value = manifest.get(key)
    if value and not SHA256_RE.fullmatch(str(value)):
        blockers.append(f"release manifest {key} is not a lowercase SHA-256 digest")
        return None
    return str(value) if value else None


def validate_action_url(manifest: dict, key: str, blockers: list[str]) -> None:
    value = manifest.get(key)
    if value and not ACTIONS_RUN_RE.fullmatch(str(value)):
        blockers.append(f"{key} is not a GitHub Actions run URL")


def verify_local_artifact(path: Path, expected_hash: str | None, label: str, blockers: list[str], notes: list[str]) -> None:
    if not path.exists():
        blockers.append(f"{label} production .so is missing; exact artifact verification is mandatory")
        return
    if expected_hash and SHA256_RE.fullmatch(expected_hash):
        actual = sha256_file(path)
        if actual != expected_hash:
            blockers.append(f"local {label} .so SHA-256 does not match release manifest")
        else:
            notes.append(f"{label} artifact SHA-256 verified: {actual}")


def main() -> int:
    blockers: list[str] = []
    notes: list[str] = []

    constants = CONSTANTS.read_text()
    core_lib = CORE_LIB.read_text()
    adapter_lib = ADAPTER_LIB.read_text()
    anchor = ANCHOR.read_text()

    core_program_id = declare_id(core_lib, "referral declare_id")
    adapter_program_id = declare_id(adapter_lib, "adapter declare_id")
    qualification_program_id: str | None = None
    evidence_program_id: str | None = None

    if not QUALIFICATION_LIB.exists():
        blockers.append("production revenue qualification source is missing")
        qualification_lib = ""
    else:
        qualification_lib = QUALIFICATION_LIB.read_text()
        try:
            qualification_program_id = declare_id(qualification_lib, "qualification declare_id")
        except RuntimeError as exc:
            blockers.append(str(exc))

    if not EVIDENCE_LIB.exists():
        blockers.append("production revenue evidence source is missing")
    else:
        try:
            evidence_program_id = declare_id(EVIDENCE_LIB.read_text(), "evidence declare_id")
        except RuntimeError as exc:
            blockers.append(str(exc))

    frozen_adapter_program_id = extract(
        r'MAINNET_REVENUE_ADAPTER_PROGRAM: Pubkey = pubkey!\("([1-9A-HJ-NP-Za-km-z]+)"\)',
        constants,
        "mainnet revenue adapter Program ID",
    )
    frozen_verifier_program_id = extract(
        r'MAINNET_VERIFIER_PROGRAM: Pubkey = pubkey!\("([1-9A-HJ-NP-Za-km-z]+)"\)',
        adapter_lib,
        "mainnet verifier Program ID",
    )
    frozen_qualification_adapter = extract(
        r'MAINNET_ADAPTER_PROGRAM: Pubkey = pubkey!\("([1-9A-HJ-NP-Za-km-z]+)"\)',
        qualification_lib,
        "qualification mainnet adapter Program ID",
    ) if qualification_lib else SENTINEL
    frozen_evidence_program_id = extract(
        r'MAINNET_EVIDENCE_PROGRAM: Pubkey = pubkey!\("([1-9A-HJ-NP-Za-km-z]+)"\)',
        qualification_lib,
        "mainnet evidence Program ID",
    ) if qualification_lib else SENTINEL

    treasury = extract(r'MAINNET_SERVICE_TREASURY: Pubkey = pubkey!\("([1-9A-HJ-NP-Za-km-z]+)"\)', constants, "mainnet treasury")
    usdt = extract(r'MAINNET_USDT_MINT: Pubkey = pubkey!\("([1-9A-HJ-NP-Za-km-z]+)"\)', constants, "USDT mint")
    usdc = extract(r'MAINNET_USDC_MINT: Pubkey = pubkey!\("([1-9A-HJ-NP-Za-km-z]+)"\)', constants, "USDC mint")
    revenue_source = extract(r'MAINNET_QUALIFIED_REVENUE_SOURCE: Pubkey = pubkey!\("([1-9A-HJ-NP-Za-km-z]+)"\)', constants, "qualified revenue source")
    registration_open = int(extract(r'MAINNET_REGISTRATION_OPEN_AT: i64 = (-?\d+)', constants, "registration open timestamp"))

    qualification_spec_hash: str | None = None
    if not QUALIFICATION_SPEC.exists():
        blockers.append("qualified-revenue product qualification specification is missing")
    else:
        specification = QUALIFICATION_SPEC.read_text()
        status = qualification_status(specification)
        if status is None:
            blockers.append("qualified-revenue product qualification specification has invalid or duplicate status marker")
        elif status != "FINAL":
            blockers.append("qualified-revenue product qualification specification is not FINAL")
        qualification_spec_hash = sha256_file(QUALIFICATION_SPEC)
        notes.append(f"qualification specification SHA-256: {qualification_spec_hash}")

    for field, actual in (("service_treasury", treasury), ("usdt_mint", usdt), ("usdc_mint", usdc)):
        if actual != EXPECTED[field]:
            blockers.append(f"{field} mismatch: {actual}")

    for label, program_id in (("referral", core_program_id), ("adapter", adapter_program_id), ("qualification", qualification_program_id)):
        if program_id in DEVELOPMENT_PROGRAM_IDS:
            blockers.append(f"{label} Program ID is still a development identity")

    if frozen_adapter_program_id == SENTINEL:
        blockers.append("core Revenue Adapter Program ID is still the fail-closed sentinel")
    elif frozen_adapter_program_id != adapter_program_id:
        blockers.append("core frozen Revenue Adapter Program ID does not match adapter declare_id!")

    if frozen_verifier_program_id == SENTINEL:
        blockers.append("adapter verifier Program ID is still the fail-closed sentinel")
    elif qualification_program_id and frozen_verifier_program_id != qualification_program_id:
        blockers.append("adapter frozen verifier Program ID does not match revenue_qualification declare_id!")

    if frozen_qualification_adapter == SENTINEL:
        blockers.append("qualification adapter Program ID is still the fail-closed sentinel")
    elif frozen_qualification_adapter != adapter_program_id:
        blockers.append("qualification frozen adapter Program ID does not match adapter declare_id!")

    if frozen_evidence_program_id == SENTINEL:
        blockers.append("qualification evidence Program ID is still the fail-closed sentinel")
    elif evidence_program_id and frozen_evidence_program_id != evidence_program_id:
        blockers.append("qualification frozen evidence Program ID does not match revenue_evidence declare_id!")

    identities = [value for value in (core_program_id, adapter_program_id, qualification_program_id, evidence_program_id) if value]
    if len(identities) != len(set(identities)):
        blockers.append("core, adapter, qualification and evidence Program IDs must be distinct")

    if revenue_source == SENTINEL:
        blockers.append("qualified revenue source is still the fail-closed sentinel")
    elif revenue_source in set(identities + [treasury]):
        blockers.append("qualified revenue source must be the adapter RevenueAuthority PDA, not a program or treasury address")

    if registration_open <= 0:
        blockers.append("registration_open_at is not frozen to a positive UTC unix timestamp")
    elif registration_open <= int(time.time()):
        blockers.append("registration_open_at is not in the future")

    mainnet_match = re.search(r'\[programs\.mainnet\][\s\S]*?service_referral_protocol\s*=\s*"([1-9A-HJ-NP-Za-km-z]+)"', anchor)
    if not mainnet_match:
        blockers.append("Anchor.toml has no [programs.mainnet] Program ID")
    elif mainnet_match.group(1) != core_program_id:
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
        "commit_sha", "program_id", "revenue_adapter_program_id", "qualification_program_id",
        "evidence_program_id", "qualified_revenue_source", "registration_open_at", "service_treasury",
        "usdt_mint", "usdc_mint", "so_sha256", "adapter_so_sha256", "qualification_so_sha256",
        "evidence_so_sha256", "qualification_spec_sha256", "audit_report_sha256",
        "verified_build_run_url", "adapter_verified_build_run_url", "qualification_verified_build_run_url",
        "evidence_verified_build_run_url", "audit_status", "smoke_test_plan_approved",
    ]
    for key in required:
        if manifest.get(key) in (None, "", 0, False):
            blockers.append(f"release manifest field not frozen: {key}")

    expected_manifest = {
        "program_id": core_program_id,
        "revenue_adapter_program_id": adapter_program_id,
        "qualification_program_id": qualification_program_id,
        "evidence_program_id": evidence_program_id,
        "qualified_revenue_source": revenue_source,
        "registration_open_at": registration_open,
        **EXPECTED,
    }
    for key, expected in expected_manifest.items():
        if manifest.get(key) != expected:
            blockers.append(f"release manifest {key} does not match frozen source")

    if qualification_spec_hash and manifest.get("qualification_spec_sha256") != qualification_spec_hash:
        blockers.append("release manifest qualification_spec_sha256 does not match local FINAL specification")
    if source_sha and manifest.get("commit_sha") != source_sha:
        blockers.append("release manifest commit_sha does not match release source SHA")

    hashes = {
        "core": validate_sha_field(manifest, "so_sha256", blockers),
        "Revenue Adapter": validate_sha_field(manifest, "adapter_so_sha256", blockers),
        "Revenue Qualification": validate_sha_field(manifest, "qualification_so_sha256", blockers),
        "Revenue Evidence": validate_sha_field(manifest, "evidence_so_sha256", blockers),
    }
    validate_sha_field(manifest, "qualification_spec_sha256", blockers)
    validate_sha_field(manifest, "audit_report_sha256", blockers)
    for key in ("verified_build_run_url", "adapter_verified_build_run_url", "qualification_verified_build_run_url", "evidence_verified_build_run_url"):
        validate_action_url(manifest, key, blockers)

    if manifest.get("audit_status") != "passed":
        blockers.append("independent audit status is not 'passed'")
    if manifest.get("smoke_test_plan_approved") is not True:
        blockers.append("mainnet smoke-test plan is not approved")

    verify_local_artifact(CORE_ARTIFACT, hashes["core"], "core", blockers, notes)
    verify_local_artifact(ADAPTER_ARTIFACT, hashes["Revenue Adapter"], "Revenue Adapter", blockers, notes)
    verify_local_artifact(QUALIFICATION_ARTIFACT, hashes["Revenue Qualification"], "Revenue Qualification", blockers, notes)
    verify_local_artifact(EVIDENCE_ARTIFACT, hashes["Revenue Evidence"], "Revenue Evidence", blockers, notes)

    print("=== PRE-MAINNET GATE ===")
    print(f"Core Program ID:          {core_program_id}")
    print(f"Adapter Program ID:       {adapter_program_id}")
    print(f"Qualification Program ID: {qualification_program_id or '<missing>'}")
    print(f"Evidence Program ID:      {evidence_program_id or '<missing>'}")
    print(f"Treasury:                 {treasury}")
    print(f"USDT mint:                {usdt}")
    print(f"USDC mint:                {usdc}")
    print(f"RevenueAuthority:         {revenue_source}")
    print(f"Open UTC:                 {registration_open}")
    for note in notes:
        print(f"NOTE  {note}")

    if blockers:
        for item in blockers:
            print(f"BLOCK {item}")
        print(f"RESULT: BLOCKED ({len(blockers)} blocker(s))")
        return 1

    print("RESULT: READY FOR CONTROLLED MAINNET DEPLOYMENT")
    print(
        "WARNING: this does NOT authorize removal of upgrade authority. Finalization comes only after "
        "deployed-bytecode verification and limited mainnet smoke tests for the complete "
        "evidence -> qualification -> adapter -> referral chain."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
