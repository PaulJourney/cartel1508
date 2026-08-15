#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path

BASE58 = r"[1-9A-HJ-NP-Za-km-z]+"
STATUS_MARKER = "QUALIFICATION_SPEC_STATUS: FINAL"


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def _declare_id(text: str) -> str | None:
    match = re.search(rf'declare_id!\("({BASE58})"\)', text)
    return match.group(1) if match else None


def _core_adapter_binding(text: str) -> str | None:
    pubkey = re.search(
        rf'MAINNET_REVENUE_ADAPTER_PROGRAM: Pubkey = pubkey!\("({BASE58})"\)',
        text,
    )
    if pubkey:
        return pubkey.group(1)
    if re.search(
        r'MAINNET_REVENUE_ADAPTER_PROGRAM: Pubkey = Pubkey::new_from_array\(\[0u8; 32\]\)',
        text,
    ):
        return None
    raise RuntimeError("cannot parse MAINNET_REVENUE_ADAPTER_PROGRAM")


def evaluate(root: Path) -> tuple[list[str], list[str]]:
    blockers: list[str] = []
    notes: list[str] = []

    spec = root / "QUALIFIED_REVENUE_QUALIFICATION_SPEC.md"
    adapter_lib = root / "programs/revenue_adapter/src/lib.rs"
    constants = root / "programs/service_referral_protocol/src/constants.rs"
    manifest_path = root / "release/mainnet-release.json"

    if not spec.exists():
        blockers.append("qualified revenue qualification spec is missing")
        spec_hash = None
    else:
        spec_text = spec.read_text()
        spec_hash = sha256_file(spec)
        notes.append(f"qualification spec SHA-256: {spec_hash}")
        if STATUS_MARKER not in spec_text:
            blockers.append(
                "QUALIFIED_REVENUE_QUALIFICATION_SPEC.md is not FINAL; "
                "the real non-self-generable revenue evidence model is unresolved"
            )
        if "<UNSET>" in spec_text:
            blockers.append("qualification spec still contains <UNSET> fields")

    adapter_id = _declare_id(adapter_lib.read_text())
    if not adapter_id:
        blockers.append("cannot parse revenue adapter declare_id")
        adapter_binding = None
    else:
        adapter_binding = _core_adapter_binding(constants.read_text())
        if adapter_binding is None:
            blockers.append("core MAINNET_REVENUE_ADAPTER_PROGRAM is still fail-closed")
        elif adapter_binding != adapter_id:
            blockers.append(
                "core MAINNET_REVENUE_ADAPTER_PROGRAM does not equal the adapter declare_id"
            )
        else:
            notes.append(f"core adapter binding verified: {adapter_id}")

    if manifest_path.exists():
        try:
            manifest = json.loads(manifest_path.read_text())
        except Exception as exc:
            blockers.append(f"release manifest cannot be parsed for qualification gate: {exc}")
            manifest = {}

        if adapter_id and manifest.get("revenue_adapter_program_id") != adapter_id:
            blockers.append(
                "release manifest revenue_adapter_program_id does not match adapter declare_id"
            )
        if spec_hash and manifest.get("qualification_evidence_spec_sha256") != spec_hash:
            blockers.append(
                "release manifest qualification_evidence_spec_sha256 does not match FINAL qualification spec"
            )
        if manifest.get("qualification_evidence_status") != "approved":
            blockers.append("release manifest qualification_evidence_status is not 'approved'")

    return blockers, notes


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    blockers, notes = evaluate(root)
    print("=== QUALIFIED REVENUE EVIDENCE GATE ===")
    for note in notes:
        print(f"NOTE  {note}")
    if blockers:
        for blocker in blockers:
            print(f"BLOCK {blocker}")
        print(f"RESULT: BLOCKED ({len(blockers)} blocker(s))")
        return 1
    print("RESULT: QUALIFICATION EVIDENCE READY")
    return 0


if __name__ == "__main__":
    sys.exit(main())
