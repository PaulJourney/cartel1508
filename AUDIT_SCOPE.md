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
- Normal locked production build and Anchor Docker verifiable production build have produced the same SHA-256 baseline:
  `b257f3d588cec850b124a6b737e2d43032f0c292d8be06c4743722de76450194`.
- A deployment-only ephemeral Solana devnet proof passed previously with a temporary Program ID whose private key was not retained.
- A stronger production-equivalent devnet transaction smoke passed on 2026-08-14 using an ephemeral Program ID and mock six-decimal SPL stablecoins. The workflow executed deploy, initialize, Pioneer #1 registration, a 10-unit purchase, a separately funded 10-token qualified-revenue event, the 50/43/2/5 allocation path, and an ACTIVE user claim.
- The full devnet smoke ended with exact conservation of the 20 minted test tokens: `5,002,000` atomic units at the user, `14,998,000` at the test treasury, and `0` in the vault after claim.
- Devnet transaction evidence is retained by GitHub Actions for run `31785683081`, artifact `devnet-transaction-smoke-evidence` (artifact ID `9213483340`, archive digest `2517efadb5f6feaf85f03062e3909cffa688d72d21c97996c0de94763a8a3d9b`). The ephemeral Program ID was `BMrQVjrL9GYeJhFvURKuvYv8Ct63Dt5T4cuDcBvyA3pB`; no private deployment key is retained in the repository or evidence artifact.
- Transient public-devnet RPC `429 Too Many Requests` responses occurred during the smoke run, were retried by the client, and did not alter the successful on-chain result.
- RustSec `cargo-audit` has reported 0 known vulnerabilities in the production lockfile. One informational warning remains: `bincode 1.3.3` is marked unmaintained by `RUSTSEC-2025-0141` and is transitively required by the current Anchor/Solana dependency graph.

## Explicit mainnet blockers

- Final qualified-revenue source program/PDA.
- Final registration opening UTC.
- Final Program ID keypair custody outside the public repository.
- Independent third-party audit and disposition of findings.
- Final locked/verifiable build after all immutable values are frozen.
- Completed `release/mainnet-release.json` and a green `python3 scripts/pre-mainnet-gate.py` result.
- Controlled mainnet deployment and deployed-bytecode verification.
- Limited mainnet smoke test before permanent removal of upgrade authority.

## Out of scope for the current audit baseline

- Frontend UI/UX and wallet presentation.
- Business/legal classification of the surrounding service.
- Any off-chain system that may ultimately qualify revenue; its trust model must be audited separately before it can become the immutable `qualified_revenue_source`.
