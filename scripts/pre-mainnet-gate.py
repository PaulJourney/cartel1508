#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import re
import subprocess
import sys
import time
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CORE_CONSTANTS = ROOT / "programs/service_referral_protocol/src/constants.rs"
CORE_LIB = ROOT / "programs/service_referral_protocol/src/lib.rs"
ADAPTER_LIB = ROOT / "programs/revenue_adapter/src/lib.rs"
QUALIFICATION_LIB = ROOT / "programs/revenue_qualification/src/lib.rs"
ANCHOR = ROOT / "Anchor.toml"
QUALIFICATION_SPEC = ROOT / "QUALIFIED_REVENUE_SOURCE_SPEC.md"
MANIFEST = ROOT / "release/mainnet-release.json"
ARTIFACT_CANDIDATES = {
    "referral_so_sha256": [
        ROOT / "target/verifiable/service_referral_protocol.so",
        ROOT / "target/deploy/service_referral_protocol.so",
    ],
    "revenue_adapter_so_sha256": [
        ROOT / "target/verifiable/revenue_adapter.so",
        ROOT / "target/deploy/revenue_adapter.so",
    ],
    "revenue_qualification_so_sha256": [
        ROOT / "target/verifiable/revenue_qualification.so",
        ROOT / "target/deploy/revenue_qualification.so",
    ],
}

EXPECTED = {
    "service_treasury": "AepYo8xanmKuRiLVeYQuCTJoQr1nyKiTApoKwHMEg8fn",
    "usdt_mint": "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB",
    "usdc_mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
}
SENTINEL_PUBKEY = "11111111111111111111111111111111"
DEVELOPMENT_IDS = {
    "referral": "4AuoBkj4vkH2K1jwUuECtVBqF6Q74efjGbaw7btuNjRV",
    "adapter": "EmGJDPvwSx6kU4KijWGh8uqRNj3BJXNjcWQyCmfKv7WL",
    "qualification": "6WWnYWwsuYNPDruPJEsqJKekv9mN9NP2mCEU6XJ7qfWs",
}
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
ACTIONS_RUN_RE = re.compile(r"^https://github\.com/[^/]+/[^/]+/actions/runs/[0-9]+(?:/.*)?$")
BASE58_RE = r"[1-9A-HJ-NP-Za-km-z]+"


def extract(pattern: str, text: str, label: str) -> str:
    m = re.search(pattern, text)
    if not m:
        raise RuntimeError(f"cannot parse {label}")
    return m.group(1)


def extract_declare_id(text: str, label: str) -> str:
    return extract(rf'declare_id!\("({BASE58_RE})"\)', text, label)


def extract_pubkey_macro(name: str, text: str, label: str) -> str:
    return extract(
        rf'{re.escape(name)}: Pubkey = pubkey!\("({BASE58_RE})"\)',
        text,
        label,
    )


def extract_frozen_binding(name: str, text: str, label: str) -> str | None:
    m = re.search(
        rf'{re.escape(name)}: Pubkey = pubkey!\("({BASE58_RE})"\)',
        text,
    )
    if m:
        return m.group(1)
    zero = re.search(
        rf'{re.escape(name)}: Pubkey = Pubkey::new_from_array\(\[0u8; 32\]\)',
        text,
    )
    if zero:
        return None
    raise RuntimeError(f"cannot parse {label}")


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def select_artifact(candidates: list[Path]) -> Path | None:
    for candidate in candidates:
        if candidate.exists():
            return candidate
    return None


def git_output(*args: str) -> str | None:
    try:
        return subprocess.check_output(
            ["git", *args], cwd=ROOT, text=True, stderr=subprocess.DEVNULL
        ).strip()
    except Exception:
        return None


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


def require_sha256(blockers: list[str], manifest: dict, key: str) -> str | None:
    value = manifest.get(key)
    if not value:
        blockers.append(f"release manifest field not frozen: {key}")
        return None
    value = str(value)
    if not SHA256_RE.fullmatch(value):
        blockers.append(f"release manifest {key} is not a lowercase SHA-256 digest")
        return None
    return value


def main() -> int:
    blockers: list[str] = []
    notes: list[str] = []

    core_constants = CORE_CONSTANTS.read_text()
    core_lib = CORE_LIB.read_text()
    adapter_lib = ADAPTER_LIB.read_text()
    qualification_lib = QUALIFICATION_LIB.read_text()
    anchor_text = ANCHOR.read_text()

    referral_program_id = extract_declare_id(core_lib, "referral declare_id")
    adapter_program_id = extract_declare_id(adapter_lib, "adapter declare_id")
    qualification_program_id = extract_declare_id(
        qualification_lib, "qualification declare_id"
    )

    treasury = extract_pubkey_macro(
        "MAINNET_SERVICE_TREASURY", core_constants, "mainnet treasury"
    )
    usdt = extract_pubkey_macro("MAINNET_USDT_MINT", core_constants, "USDT mint")
    usdc = extract_pubkey_macro("MAINNET_USDC_MINT", core_constants, "USDC mint")
    revenue_source = extract_pubkey_macro(
        "MAINNET_QUALIFIED_REVENUE_SOURCE",
        core_constants,
        "qualified revenue source",
    )
    registration_open = int(
        extract(
            r"MAINNET_REGISTRATION_OPEN_AT: i64 = (-?\d+)",
            core_constants,
            "registration open timestamp",
        )
    )

    adapter_referral_binding = extract_frozen_binding(
        "MAINNET_REFERRAL_PROGRAM",
        adapter_lib,
        "adapter referral production binding",
    )
    adapter_qualification_binding = extract_frozen_binding(
        "MAINNET_QUALIFICATION_PROGRAM",
        adapter_lib,
        "adapter qualification production binding",
    )
    adapter_usdt = extract_pubkey_macro(
        "MAINNET_USDT_MINT", adapter_lib, "adapter USDT mint"
    )
    adapter_usdc = extract_pubkey_macro(
        "MAINNET_USDC_MINT", adapter_lib, "adapter USDC mint"
    )
    qualification_adapter_binding = extract_frozen_binding(
        "MAINNET_ADAPTER_PROGRAM",
        qualification_lib,
        "qualification adapter production binding",
    )
    qualification_usdt = extract_pubkey_macro(
        "MAINNET_USDT_MINT", qualification_lib, "qualification USDT mint"
    )
    qualification_usdc = extract_pubkey_macro(
        "MAINNET_USDC_MINT", qualification_lib, "qualification USDC mint"
    )

    if treasury != EXPECTED["service_treasury"]:
        blockers.append(f"treasury mismatch: {treasury}")
    for label, value in [
        ("core USDT", usdt),
        ("adapter USDT", adapter_usdt),
        ("qualification USDT", qualification_usdt),
    ]:
        if value != EXPECTED["usdt_mint"]:
            blockers.append(f"{label} mint mismatch: {value}")
    for label, value in [
        ("core USDC", usdc),
        ("adapter USDC", adapter_usdc),
        ("qualification USDC", qualification_usdc),
    ]:
        if value != EXPECTED["usdc_mint"]:
            blockers.append(f"{label} mint mismatch: {value}")

    final_ids = {
        "referral": referral_program_id,
        "adapter": adapter_program_id,
        "qualification": qualification_program_id,
    }
    for name, program_id in final_ids.items():
        if program_id == DEVELOPMENT_IDS[name]:
            blockers.append(f"{name} Program ID is still the development identity")
    if len(set(final_ids.values())) != 3:
        blockers.append("referral, adapter and qualification Program IDs must be distinct")

    if adapter_referral_binding is None:
        blockers.append("adapter MAINNET_REFERRAL_PROGRAM is still fail-closed")
    elif adapter_referral_binding != referral_program_id:
        blockers.append("adapter MAINNET_REFERRAL_PROGRAM does not match referral Program ID")

    if adapter_qualification_binding is None:
        blockers.append("adapter MAINNET_QUALIFICATION_PROGRAM is still fail-closed")
    elif adapter_qualification_binding != qualification_program_id:
        blockers.append(
            "adapter MAINNET_QUALIFICATION_PROGRAM does not match qualification Program ID"
        )

    if qualification_adapter_binding is None:
        blockers.append("qualification MAINNET_ADAPTER_PROGRAM is still fail-closed")
    elif qualification_adapter_binding != adapter_program_id:
        blockers.append(
            "qualification MAINNET_ADAPTER_PROGRAM does not match adapter Program ID"
        )

    if revenue_source == SENTINEL_PUBKEY:
        blockers.append("qualified revenue source is still the fail-closed sentinel")
    else:
        forbidden_sources = {
            treasury,
            referral_program_id,
            adapter_program_id,
            qualification_program_id,
        }
        if revenue_source in forbidden_sources:
            blockers.append(
                "qualified revenue source must be the adapter Revenue Authority PDA, not a treasury or Program ID"
            )

    if registration_open <= 0:
        blockers.append("registration_open_at is not frozen to a positive UTC unix timestamp")
    elif registration_open <= int(time.time()):
        blockers.append("registration_open_at is not in the future")

    try:
        anchor = tomllib.loads(anchor_text)
        mainnet = anchor.get("programs", {}).get("mainnet", {})
    except Exception as exc:
        blockers.append(f"Anchor.toml cannot be parsed: {exc}")
        mainnet = {}

    anchor_expected = {
        "service_referral_protocol": referral_program_id,
        "revenue_adapter": adapter_program_id,
        "revenue_qualification": qualification_program_id,
    }
    for name, expected in anchor_expected.items():
        actual = mainnet.get(name)
        if not actual:
            blockers.append(f"Anchor.toml [programs.mainnet] missing {name}")
        elif actual != expected:
            blockers.append(
                f"Anchor.toml mainnet {name} does not match source declare_id!"
            )

    tracked = tracked_secret_candidates()
    if tracked:
        blockers.append("secret-like deployment files are tracked: " + ", ".join(tracked))

    git_head = git_output("rev-parse", "HEAD")
    if not git_head:
        blockers.append("cannot resolve current git HEAD")
    else:
        notes.append(f"git HEAD: {git_head}")

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

        required_scalar = [
            "commit_sha",
            "referral_program_id",
            "revenue_adapter_program_id",
            "revenue_qualification_program_id",
            "qualified_revenue_source",
            "registration_open_at",
            "service_treasury",
            "usdt_mint",
            "usdc_mint",
            "verified_build_run_url",
            "audit_status",
            "qualification_evidence_status",
            "smoke_test_plan_approved",
        ]
        for key in required_scalar:
            value = manifest.get(key)
            if value in (None, "", 0, False):
                blockers.append(f"release manifest field not frozen: {key}")

        expected_manifest = {
            "referral_program_id": referral_program_id,
            "revenue_adapter_program_id": adapter_program_id,
            "revenue_qualification_program_id": qualification_program_id,
            "qualified_revenue_source": revenue_source,
            "registration_open_at": registration_open,
            **EXPECTED,
        }
        for key, expected in expected_manifest.items():
            if manifest.get(key) != expected:
                blockers.append(f"release manifest {key} does not match frozen source")

        manifest_commit = manifest.get("commit_sha")
        if git_head and manifest_commit != git_head:
            blockers.append("release manifest commit_sha does not match current git HEAD")

        artifact_hashes: dict[str, str | None] = {}
        for hash_field, candidates in ARTIFACT_CANDIDATES.items():
            expected_hash = require_sha256(blockers, manifest, hash_field)
            artifact_hashes[hash_field] = expected_hash
            artifact = select_artifact(candidates)
            if artifact is None:
                locations = ", ".join(str(p.relative_to(ROOT)) for p in candidates)
                blockers.append(f"production artifact is missing; checked: {locations}")
            elif expected_hash:
                actual_hash = sha256_file(artifact)
                if actual_hash != expected_hash:
                    blockers.append(
                        f"{artifact.name} SHA-256 does not match release manifest"
                    )
                else:
                    notes.append(
                        f"artifact SHA-256 verified: {artifact.relative_to(ROOT)} {actual_hash}"
                    )

        audit_hash = require_sha256(blockers, manifest, "audit_report_sha256")
        qualification_spec_hash = require_sha256(
            blockers, manifest, "qualification_evidence_spec_sha256"
        )
        if qualification_spec_hash:
            if not QUALIFICATION_SPEC.exists():
                blockers.append("QUALIFIED_REVENUE_SOURCE_SPEC.md is missing")
            else:
                actual_spec_hash = sha256_file(QUALIFICATION_SPEC)
                if actual_spec_hash != qualification_spec_hash:
                    blockers.append(
                        "qualification evidence spec SHA-256 does not match repository spec"
                    )
                else:
                    notes.append(
                        f"qualification evidence spec SHA-256 verified: {actual_spec_hash}"
                    )

        verified_build_url = manifest.get("verified_build_run_url")
        if verified_build_url and not ACTIONS_RUN_RE.fullmatch(str(verified_build_url)):
            blockers.append("verified_build_run_url is not a GitHub Actions run URL")

        if manifest.get("audit_status") != "passed":
            blockers.append("independent audit status is not 'passed'")
        if manifest.get("qualification_evidence_status") != "approved":
            blockers.append("qualified revenue evidence model is not 'approved'")
        if manifest.get("smoke_test_plan_approved") is not True:
            blockers.append("mainnet smoke-test plan is not approved")

        _ = (artifact_hashes, audit_hash)

    print("=== PRE-MAINNET THREE-PROGRAM GATE ===")
    print(f"Referral ID:      {referral_program_id}")
    print(f"Adapter ID:       {adapter_program_id}")
    print(f"Qualification ID: {qualification_program_id}")
    print(f"Treasury:         {treasury}")
    print(f"USDT mint:        {usdt}")
    print(f"USDC mint:        {usdc}")
    print(f"Revenue source:   {revenue_source}")
    print(f"Open UTC:         {registration_open}")
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
        "Finalization comes only after all three deployed bytecodes are verified "
        "and the limited mainnet smoke plan passes."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
