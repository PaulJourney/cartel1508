#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CONSTANTS = ROOT / "programs/service_referral_protocol/src/constants.rs"
LIB = ROOT / "programs/service_referral_protocol/src/lib.rs"
ANCHOR = ROOT / "Anchor.toml"
MANIFEST = ROOT / "release/mainnet-release.json"
ARTIFACT = ROOT / "target/deploy/service_referral_protocol.so"

EXPECTED = {
    "service_treasury": "AepYo8xanmKuRiLVeYQuCTJoQr1nyKiTApoKwHMEg8fn",
    "usdt_mint": "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB",
    "usdc_mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
}
SENTINEL_SOURCE = "11111111111111111111111111111111"


def extract(pattern: str, text: str, label: str) -> str:
    m = re.search(pattern, text)
    if not m:
        raise RuntimeError(f"cannot parse {label}")
    return m.group(1)


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def tracked_secret_candidates() -> list[str]:
    try:
        out = subprocess.check_output(["git", "ls-files"], cwd=ROOT, text=True)
    except Exception:
        return []
    bad = []
    for name in out.splitlines():
        lowered = name.lower()
        if lowered.endswith("keypair.json") or "seed" in lowered or "private-key" in lowered or "private_key" in lowered:
            bad.append(name)
    return bad


def main() -> int:
    blockers: list[str] = []
    notes: list[str] = []

    constants = CONSTANTS.read_text()
    lib = LIB.read_text()
    anchor = ANCHOR.read_text()

    source_program_id = extract(r'declare_id!\("([1-9A-HJ-NP-Za-km-z]+)"\)', lib, "declare_id")
    treasury = extract(r'MAINNET_SERVICE_TREASURY: Pubkey = pubkey!\("([1-9A-HJ-NP-Za-km-z]+)"\)', constants, "mainnet treasury")
    usdt = extract(r'MAINNET_USDT_MINT: Pubkey = pubkey!\("([1-9A-HJ-NP-Za-km-z]+)"\)', constants, "USDT mint")
    usdc = extract(r'MAINNET_USDC_MINT: Pubkey = pubkey!\("([1-9A-HJ-NP-Za-km-z]+)"\)', constants, "USDC mint")
    revenue_source = extract(r'MAINNET_QUALIFIED_REVENUE_SOURCE: Pubkey = pubkey!\("([1-9A-HJ-NP-Za-km-z]+)"\)', constants, "qualified revenue source")
    registration_open = int(extract(r'MAINNET_REGISTRATION_OPEN_AT: i64 = (-?\d+)', constants, "registration open timestamp"))

    if treasury != EXPECTED["service_treasury"]:
        blockers.append(f"treasury mismatch: {treasury}")
    if usdt != EXPECTED["usdt_mint"]:
        blockers.append(f"USDT mint mismatch: {usdt}")
    if usdc != EXPECTED["usdc_mint"]:
        blockers.append(f"USDC mint mismatch: {usdc}")
    if revenue_source == SENTINEL_SOURCE:
        blockers.append("qualified revenue source is still the fail-closed sentinel")
    if registration_open <= 0:
        blockers.append("registration_open_at is not frozen to a positive UTC unix timestamp")

    mainnet_match = re.search(r'\[programs\.mainnet\][\s\S]*?service_referral_protocol\s*=\s*"([1-9A-HJ-NP-Za-km-z]+)"', anchor)
    if not mainnet_match:
        blockers.append("Anchor.toml has no [programs.mainnet] Program ID")
    elif mainnet_match.group(1) != source_program_id:
        blockers.append("Anchor.toml mainnet Program ID does not match declare_id!")

    tracked = tracked_secret_candidates()
    if tracked:
        blockers.append("secret-like deployment files are tracked: " + ", ".join(tracked))

    if not MANIFEST.exists():
        blockers.append("release/mainnet-release.json is missing")
    else:
        try:
            manifest = json.loads(MANIFEST.read_text())
        except Exception as exc:
            blockers.append(f"release manifest is invalid JSON: {exc}")
            manifest = {}

        required = [
            "commit_sha",
            "program_id",
            "qualified_revenue_source",
            "registration_open_at",
            "service_treasury",
            "usdt_mint",
            "usdc_mint",
            "so_sha256",
            "audit_report_sha256",
            "verified_build_run_url",
            "audit_status",
            "smoke_test_plan_approved",
        ]
        for key in required:
            value = manifest.get(key)
            if value in (None, "", 0, False):
                blockers.append(f"release manifest field not frozen: {key}")

        expected_manifest = {
            "program_id": source_program_id,
            "qualified_revenue_source": revenue_source,
            "registration_open_at": registration_open,
            **EXPECTED,
        }
        for key, expected in expected_manifest.items():
            if manifest.get(key) != expected:
                blockers.append(f"release manifest {key} does not match frozen source")

        if manifest.get("audit_status") != "passed":
            blockers.append("independent audit status is not 'passed'")
        if manifest.get("smoke_test_plan_approved") is not True:
            blockers.append("mainnet smoke-test plan is not approved")

        if ARTIFACT.exists() and manifest.get("so_sha256"):
            actual = sha256_file(ARTIFACT)
            if actual != manifest["so_sha256"]:
                blockers.append("local production .so SHA-256 does not match release manifest")
            else:
                notes.append(f"artifact SHA-256 verified: {actual}")
        else:
            notes.append("production .so not present locally; artifact hash comparison skipped")

    print("=== PRE-MAINNET GATE ===")
    print(f"Program ID: {source_program_id}")
    print(f"Treasury:   {treasury}")
    print(f"USDT mint:  {usdt}")
    print(f"USDC mint:  {usdc}")
    print(f"Revenue:    {revenue_source}")
    print(f"Open UTC:   {registration_open}")
    for note in notes:
        print(f"NOTE  {note}")

    if blockers:
        for item in blockers:
            print(f"BLOCK {item}")
        print(f"RESULT: BLOCKED ({len(blockers)} blocker(s))")
        return 1

    print("RESULT: READY FOR CONTROLLED MAINNET DEPLOYMENT")
    print("WARNING: this does NOT authorize removal of upgrade authority. Finalization comes only after bytecode verification and limited mainnet smoke tests.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
