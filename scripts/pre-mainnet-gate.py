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
LIB = ROOT / "programs/service_referral_protocol/src/lib.rs"
ADAPTER_LIB = ROOT / "programs/revenue_adapter/src/lib.rs"
VERIFIER_LIB = ROOT / "programs/revenue_verifier/src/lib.rs"
QUALIFICATION_SPEC = ROOT / "QUALIFIED_REVENUE_QUALIFICATION_SPEC.md"
ANCHOR = ROOT / "Anchor.toml"
MANIFEST = ROOT / "release/mainnet-release.json"
CORE_ARTIFACT = ROOT / "target/deploy/service_referral_protocol.so"
ADAPTER_ARTIFACT = ROOT / "programs/revenue_adapter/target/deploy/revenue_adapter.so"
VERIFIER_ARTIFACT = ROOT / "programs/revenue_verifier/target/deploy/revenue_verifier.so"

EXPECTED = {
    "service_treasury": "AepYo8xanmKuRiLVeYQuCTJoQr1nyKiTApoKwHMEg8fn",
    "usdt_mint": "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB",
    "usdc_mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
}
SENTINEL = "11111111111111111111111111111111"
DEVELOPMENT_PROGRAM_ID = "4AuoBkj4vkH2K1jwUuECtVBqF6Q74efjGbaw7btuNjRV"
DEVELOPMENT_ADAPTER_PROGRAM_ID = "Gxf1ikEvwiusChxqj5jkQPvWhFhFw91nyYFNL6oT7ZLD"
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
ACTIONS_RUN_RE = re.compile(r"^https://github\.com/[^/]+/[^/]+/actions/runs/[0-9]+(?:/.*)?$")


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


def git_output(*args: str) -> str | None:
    try:
        return subprocess.check_output(
            ["git", *args], cwd=ROOT, text=True, stderr=subprocess.DEVNULL
        ).strip()
    except Exception:
        return None


def release_source_sha() -> str | None:
    # On pull_request workflows GitHub checks out a synthetic merge commit.
    # If the workflow supplies the audited head SHA, prefer it; otherwise use HEAD.
    explicit = os.environ.get("RELEASE_SOURCE_SHA", "").strip()
    return explicit or git_output("rev-parse", "HEAD")


def tracked_secret_candidates() -> list[str]:
    out = git_output("ls-files")
    if out is None:
        return []
    bad = []
    for name in out.splitlines():
        lowered = name.lower()
        if (
            lowered.endswith("keypair.json")
            or "seed" in lowered
            or "private-key" in lowered
            or "private_key" in lowered
        ):
            bad.append(name)
    return bad


def qualification_status(text: str) -> str | None:
    matches = re.findall(
        r"^QUALIFICATION_SPEC_STATUS:\s*([A-Z_]+)\s*$",
        text,
        flags=re.MULTILINE,
    )
    if len(matches) != 1:
        return None
    return matches[0]


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
    lib = LIB.read_text()
    adapter_lib = ADAPTER_LIB.read_text()
    anchor = ANCHOR.read_text()

    source_program_id = extract(
        r'declare_id!\("([1-9A-HJ-NP-Za-km-z]+)"\)', lib, "referral declare_id"
    )
    adapter_program_id = extract(
        r'declare_id!\("([1-9A-HJ-NP-Za-km-z]+)"\)', adapter_lib, "adapter declare_id"
    )
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
    treasury = extract(
        r'MAINNET_SERVICE_TREASURY: Pubkey = pubkey!\("([1-9A-HJ-NP-Za-km-z]+)"\)',
        constants,
        "mainnet treasury",
    )
    usdt = extract(
        r'MAINNET_USDT_MINT: Pubkey = pubkey!\("([1-9A-HJ-NP-Za-km-z]+)"\)',
        constants,
        "USDT mint",
    )
    usdc = extract(
        r'MAINNET_USDC_MINT: Pubkey = pubkey!\("([1-9A-HJ-NP-Za-km-z]+)"\)',
        constants,
        "USDC mint",
    )
    revenue_source = extract(
        r'MAINNET_QUALIFIED_REVENUE_SOURCE: Pubkey = pubkey!\("([1-9A-HJ-NP-Za-km-z]+)"\)',
        constants,
        "qualified revenue source",
    )
    registration_open = int(
        extract(
            r'MAINNET_REGISTRATION_OPEN_AT: i64 = (-?\d+)',
            constants,
            "registration open timestamp",
        )
    )

    verifier_program_id: str | None = None
    if not VERIFIER_LIB.exists():
        blockers.append("production revenue verifier source is missing")
    else:
        verifier_lib = VERIFIER_LIB.read_text()
        try:
            verifier_program_id = extract(
                r'declare_id!\("([1-9A-HJ-NP-Za-km-z]+)"\)',
                verifier_lib,
                "verifier declare_id",
            )
        except RuntimeError as exc:
            blockers.append(str(exc))

    qualification_spec_hash: str | None = None
    if not QUALIFICATION_SPEC.exists():
        blockers.append("qualified-revenue product qualification specification is missing")
    else:
        qualification_text = QUALIFICATION_SPEC.read_text()
        status = qualification_status(qualification_text)
        if status is None:
            blockers.append("qualified-revenue product qualification specification has invalid or duplicate status marker")
        elif status != "FINAL":
            blockers.append("qualified-revenue product qualification specification is not FINAL")
        qualification_spec_hash = sha256_file(QUALIFICATION_SPEC)
        notes.append(f"qualification specification SHA-256: {qualification_spec_hash}")

    if treasury != EXPECTED["service_treasury"]:
        blockers.append(f"treasury mismatch: {treasury}")
    if usdt != EXPECTED["usdt_mint"]:
        blockers.append(f"USDT mint mismatch: {usdt}")
    if usdc != EXPECTED["usdc_mint"]:
        blockers.append(f"USDC mint mismatch: {usdc}")

    if source_program_id == DEVELOPMENT_PROGRAM_ID:
        blockers.append("Program ID is still the development identity")
    if adapter_program_id == DEVELOPMENT_ADAPTER_PROGRAM_ID:
        blockers.append("Revenue Adapter Program ID is still the development identity")
    if frozen_adapter_program_id == SENTINEL:
        blockers.append("revenue adapter Program ID is still the fail-closed sentinel")
    elif frozen_adapter_program_id != adapter_program_id:
        blockers.append("core frozen Revenue Adapter Program ID does not match adapter declare_id!")

    if frozen_verifier_program_id == SENTINEL:
        blockers.append("revenue verifier Program ID is still the fail-closed sentinel")
    elif verifier_program_id and frozen_verifier_program_id != verifier_program_id:
        blockers.append("adapter frozen verifier Program ID does not match verifier declare_id!")

    identities = [source_program_id, adapter_program_id]
    if verifier_program_id:
        identities.append(verifier_program_id)
    if len(identities) != len(set(identities)):
        blockers.append("referral, adapter and verifier Program IDs must be distinct")

    if revenue_source == SENTINEL:
        blockers.append("qualified revenue source is still the fail-closed sentinel")
    else:
        forbidden_sources = {
            treasury: "service treasury",
            source_program_id: "referral Program ID",
            adapter_program_id: "Revenue Adapter Program ID",
        }
        if verifier_program_id:
            forbidden_sources[verifier_program_id] = "verifier Program ID"
        if revenue_source in forbidden_sources:
            blockers.append(
                f"qualified revenue source must not equal the {forbidden_sources[revenue_source]}"
            )

    if registration_open <= 0:
        blockers.append("registration_open_at is not frozen to a positive UTC unix timestamp")
    elif registration_open <= int(time.time()):
        blockers.append("registration_open_at is not in the future")

    mainnet_match = re.search(
        r'\[programs\.mainnet\][\s\S]*?service_referral_protocol\s*=\s*"([1-9A-HJ-NP-Za-km-z]+)"',
        anchor,
    )
    if not mainnet_match:
        blockers.append("Anchor.toml has no [programs.mainnet] Program ID")
    elif mainnet_match.group(1) != source_program_id:
        blockers.append("Anchor.toml mainnet Program ID does not match declare_id!")

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
    else:
        try:
            manifest = json.loads(MANIFEST.read_text())
        except Exception as exc:
            blockers.append(f"release manifest is invalid JSON: {exc}")
            manifest = {}

        required = [
            "commit_sha",
            "program_id",
            "revenue_adapter_program_id",
            "verifier_program_id",
            "qualified_revenue_source",
            "registration_open_at",
            "service_treasury",
            "usdt_mint",
            "usdc_mint",
            "so_sha256",
            "adapter_so_sha256",
            "verifier_so_sha256",
            "qualification_spec_sha256",
            "audit_report_sha256",
            "verified_build_run_url",
            "adapter_verified_build_run_url",
            "verifier_verified_build_run_url",
            "audit_status",
            "smoke_test_plan_approved",
        ]
        for key in required:
            value = manifest.get(key)
            if value in (None, "", 0, False):
                blockers.append(f"release manifest field not frozen: {key}")

        expected_manifest = {
            "program_id": source_program_id,
            "revenue_adapter_program_id": adapter_program_id,
            "verifier_program_id": verifier_program_id,
            "qualified_revenue_source": revenue_source,
            "registration_open_at": registration_open,
            **EXPECTED,
        }
        for key, expected in expected_manifest.items():
            if manifest.get(key) != expected:
                blockers.append(f"release manifest {key} does not match frozen source")

        if qualification_spec_hash and manifest.get("qualification_spec_sha256") != qualification_spec_hash:
            blockers.append("release manifest qualification_spec_sha256 does not match local FINAL specification")

        manifest_commit = manifest.get("commit_sha")
        if source_sha and manifest_commit != source_sha:
            blockers.append("release manifest commit_sha does not match release source SHA")

        core_hash = validate_sha_field(manifest, "so_sha256", blockers)
        adapter_hash = validate_sha_field(manifest, "adapter_so_sha256", blockers)
        verifier_hash = validate_sha_field(manifest, "verifier_so_sha256", blockers)
        validate_sha_field(manifest, "qualification_spec_sha256", blockers)
        validate_sha_field(manifest, "audit_report_sha256", blockers)

        validate_action_url(manifest, "verified_build_run_url", blockers)
        validate_action_url(manifest, "adapter_verified_build_run_url", blockers)
        validate_action_url(manifest, "verifier_verified_build_run_url", blockers)

        if manifest.get("audit_status") != "passed":
            blockers.append("independent audit status is not 'passed'")
        if manifest.get("smoke_test_plan_approved") is not True:
            blockers.append("mainnet smoke-test plan is not approved")

        verify_local_artifact(CORE_ARTIFACT, core_hash, "referral", blockers, notes)
        verify_local_artifact(ADAPTER_ARTIFACT, adapter_hash, "Revenue Adapter", blockers, notes)
        verify_local_artifact(VERIFIER_ARTIFACT, verifier_hash, "revenue verifier", blockers, notes)

    print("=== PRE-MAINNET GATE ===")
    print(f"Referral Program ID: {source_program_id}")
    print(f"Adapter Program ID:  {adapter_program_id}")
    print(f"Verifier Program ID: {verifier_program_id or '<missing>'}")
    print(f"Treasury:            {treasury}")
    print(f"USDT mint:           {usdt}")
    print(f"USDC mint:           {usdc}")
    print(f"RevenueAuthority:    {revenue_source}")
    print(f"Open UTC:            {registration_open}")
    for note in notes:
        print(f"NOTE  {note}")

    if blockers:
        for item in blockers:
            print(f"BLOCK {item}")
        print(f"RESULT: BLOCKED ({len(blockers)} blocker(s))")
        return 1

    print("RESULT: READY FOR CONTROLLED MAINNET DEPLOYMENT")
    print(
        "WARNING: this does NOT authorize removal of any upgrade authority. "
        "Finalization comes only after deployed-bytecode verification and limited mainnet smoke tests for the complete verifier -> adapter -> referral chain."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
