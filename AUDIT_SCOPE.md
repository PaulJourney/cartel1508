# External audit scope — Solana V0.10

Status: pre-audit. This document defines the review target; it does not represent an independent audit or production approval.

## Program in scope

- `programs/service_referral_protocol/src/lib.rs`
- `programs/service_referral_protocol/src/state.rs`
- `programs/service_referral_protocol/src/math.rs`
- `programs/service_referral_protocol/src/constants.rs`
- `programs/service_referral_protocol/Cargo.toml`
- root `Cargo.lock`
- `scripts/pre-mainnet-gate.py`
- `release/mainnet-release.example.json`

## Core properties to review

- No mutable owner/admin control surface.
- Immutable referral relationships and exact ancestry validation.
- Separation between service-unit purchases and qualified-revenue accounting.
- Stablecoin mint, token-program and canonical ATA validation.
- Vault collateral conservation and atomic rollback on failed instructions.
- ACTIVE / GRACE / INACTIVE time-boundary behavior and expired-balance settlement.
- Pioneer high-precision index, fractional carry and first-100 assignment semantics.
- Ten-level accounting, rounding and root/unallocated routing.
- Pull-based claim authorization and activity requirement.
- Global monotonic `u128` Unit ID ranges and overflow behavior.
- Production initialization remains fail-closed until final immutable source/time values are frozen.
- Pre-mainnet release gate fails closed unless Program ID, source/time constants, public artifact evidence and independent audit evidence are mutually consistent.
- Upgrade authority removal procedure after deployed-bytecode verification and limited mainnet smoke testing.

## Automated evidence already available

- Pinned toolchain: Solana 3.1.10 and Anchor 1.1.2.
- Read-only locked-dependency CI passes.
- Rust unit tests pass, including a 100,000,000,000-unit range case.
- Deterministic property tests cover 50,000 accounting amounts, including edge values through `u64::MAX`, and assert exact conservation of the gross amount after direct/network/Pioneer/service/rounding allocation.
- Deterministic Unit ID property tests exercise 25,000 variable-size batches and assert contiguous, non-overlapping monotonic ranges.
- LiteSVM integration suite passes for initialization, service-unit purchase, accounting, claim lifecycle, grace, expiry, ten-level routing, source authorization, ancestry substitution rejection and global Unit IDs across wallets.
- Normal locked production build and Anchor Docker verifiable production build produce the same SHA-256:
  `b257f3d588cec850b124a6b737e2d43032f0c292d8be06c4743722de76450194`
- Rebuilding after documentation-only commits produces the same verifiable SHA-256.
- Ephemeral Solana devnet deployment smoke test passed. Temporary devnet Program ID used for that proof: `6WZbsidaKgXcdvsJYuMS98BLsQzha67e6VyYLZFjSqEj`; its private key was not retained.
- RustSec `cargo-audit` reports 0 known vulnerabilities in the production lockfile. One informational warning remains: `bincode 1.3.3` is marked unmaintained by `RUSTSEC-2025-0141` and is transitively required by the current Anchor/Solana dependency graph.

## Explicit mainnet blockers

- Final qualified-revenue source program/PDA.
- Final registration opening UTC.
- Final Program ID keypair custody outside the public repository.
- Production-equivalent devnet transaction smoke beyond deployment-only proof.
- Independent third-party audit and disposition of findings.
- Final locked/verifiable build after all immutable values are frozen.
- Completed `release/mainnet-release.json` and a green `python3 scripts/pre-mainnet-gate.py` result.
- Limited mainnet smoke test before permanent removal of upgrade authority.

## Out of scope for the current audit baseline

- Frontend UI/UX and wallet presentation.
- Business/legal classification of the surrounding service.
- Any off-chain system that may ultimately qualify revenue; its trust model must be audited separately before it can become the immutable `qualified_revenue_source`.
