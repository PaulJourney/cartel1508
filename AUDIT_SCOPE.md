# External audit scope — Solana V0.11

Status: pre-audit. This document defines the intended independent review target; it is not an audit report or production approval.

## Production architecture in scope

The final reviewed system is a three-program chain:

`Revenue Verifier -> Revenue Adapter -> Service Referral Protocol -> SPL Token Program`

The verifier determines whether an event satisfies the FINAL product/economic qualification specification. The adapter enforces verifier-PDA authorization, prefunded canonical RevenueAuthority token accounts, immutable receipts and atomic forwarding. The referral protocol performs the 50/43/2/5 accounting and user lifecycle logic.

### Service Referral Protocol

- `programs/service_referral_protocol/src/lib.rs`
- `programs/service_referral_protocol/src/state.rs`
- `programs/service_referral_protocol/src/math.rs`
- `programs/service_referral_protocol/src/constants.rs`
- `programs/service_referral_protocol/Cargo.toml`
- root `Cargo.lock`

### Revenue Adapter

- `programs/revenue_adapter/src/lib.rs`
- `programs/revenue_adapter/tests/release_binding.rs`
- `programs/revenue_adapter/Cargo.toml`
- the exact dependency lockfile used for the final audited build

### Revenue Verifier

- `programs/revenue_verifier/**` once the production implementation exists
- its exact dependency lockfile
- `QUALIFIED_REVENUE_QUALIFICATION_SPEC.md` in FINAL status

### Cross-program/release controls

- `adapter-integration-tests/**`
- `test-programs/test_revenue_verifier/**` as test-fixture evidence only, never production code
- `scripts/static-gates.py`
- `scripts/revenue-adapter-static-gates.py`
- `scripts/pre-mainnet-gate.py`
- `release/mainnet-release.example.json`
- `PROGRAM_ID_CUSTODY_RUNBOOK.md`
- `MAINNET_FINALIZATION_RUNBOOK.md`

## Referral-core properties to review

- No mutable owner/admin control surface.
- Immutable referral relationships and exact ancestry validation.
- Separation between service-unit/activity purchases and qualified-revenue accounting.
- Stablecoin mint, legacy SPL Token program and canonical ATA validation.
- Vault collateral conservation and atomic rollback on failed instructions.
- ACTIVE / GRACE / INACTIVE time-boundary behavior and expired-balance settlement.
- Pioneer high-precision index, fractional carry and first-100 assignment semantics.
- Ten-level accounting, rounding and technical-root/unallocated routing.
- Pull-based claim authorization and ACTIVE requirement.
- Global monotonic Unit ID ranges and overflow behavior.
- Production initialization fail-closed behavior.

## Revenue-Adapter properties to review

- No mutable admin/verifier/referral/treasury control surface after initialization.
- Adapter config binds one verifier Program ID and the exact referral Program ID.
- Verifier authorization requires the deterministic verifier-program PDA signer; an ordinary wallet signer cannot substitute for it.
- `RevenueAuthority` is a deterministic adapter PDA with no private key.
- Qualified funds must already exist in RevenueAuthority's canonical ATA before referral liabilities are created.
- Only frozen USDT/USDC rails can reach the referral core.
- Each event creates a deterministic receipt PDA keyed by the unique 32-byte event ID.
- Receipt binds event ID, evidence hash, beneficiary, mint, gross amount, accepted timestamp and slot.
- Replay of a successful event cannot create a second liability.
- Failed downstream CPI consumes no receipt and creates no persistent token/accounting mutation.
- Adapter cannot redirect to an arbitrary referral program.

## Revenue-Verifier properties to review once implemented

The verifier must be audited against the exact FINAL `QUALIFIED_REVENUE_QUALIFICATION_SPEC.md`. Review at minimum:

- exact event that qualifies as service revenue;
- explicit exclusion of service-unit/activity purchases;
- funding provenance and economic finality;
- deterministic beneficiary binding;
- deterministic amount derivation;
- canonical unique-event-ID derivation;
- canonical evidence payload/hash;
- refund, cancellation, chargeback and reversal treatment;
- immutable trust source/oracle/attestation model;
- absence of a generic permanent admin ability to fabricate arbitrary funded events;
- supported stablecoin/mint constraints;
- replay and failure semantics.

If the verifier depends on an external oracle, signer set, upstream program or attestation system, that dependency and its key-rotation/failure model are part of the security boundary and must be explicitly dispositioned by the auditor.

## Cross-program properties to review

- Verifier PDA -> Adapter signer propagation is valid only during the intended CPI frame.
- Adapter RevenueAuthority PDA -> Referral signer propagation cannot be forged externally.
- Final verifier, adapter and referral Program IDs are distinct and frozen consistently.
- Core `MAINNET_REVENUE_ADAPTER_PROGRAM` equals the exact final adapter Program ID.
- Core `MAINNET_QUALIFIED_REVENUE_SOURCE` equals the exact `[b"revenue-authority"]` PDA derived from that final adapter Program ID.
- Adapter `MAINNET_VERIFIER_PROGRAM` equals the exact final verifier Program ID.
- One transaction cannot partially persist receipt creation, token movement or referral accounting if any downstream step fails.
- The release manifest, FINAL qualification-spec hash, source commit and all three artifact hashes are mutually consistent.

## Automated evidence already available before the final audit freeze

- Pinned toolchain: Solana 3.1.10 and Anchor 1.1.2.
- Referral core reference/static/Rust/property/LiteSVM tests pass on previously green release-candidate commits.
- Revenue Adapter static gates, Rust unit tests and SBF development/production-candidate builds pass on previously green release-candidate commits.
- Cross-program LiteSVM adversarial tests have passed for:
  - correct verifier-PDA CPI;
  - exact verifier PDA supplied without signer privilege -> rejected;
  - ordinary wallet signer substituted for verifier PDA -> rejected;
  - duplicate event ID -> rejected;
  - downstream ancestry failure -> receipt/token/accounting rollback.
- RustSec scanning has passed for the core production lockfile and the adapter dependency graph on previously green release-candidate commits.
- Referral-core production-equivalent devnet transaction smoke passed on 2026-08-14 with an ephemeral Program ID and mock six-decimal SPL stablecoins: initialize, Pioneer #1 registration, 10-unit activity purchase, separately funded qualified-revenue event, 50/43/2/5 allocation and ACTIVE claim.
- That referral smoke conserved all 20 minted test tokens: `5,002,000` atomic units at the user, `14,998,000` at the test treasury and `0` in the vault after claim.

These are engineering evidences, not substitutes for the independent audit. Final audit evidence must refer to the exact immutable-value freeze commit and the exact three final artifacts; historical development hashes must not be treated as final-release hashes.

## Explicit blockers before the auditor can sign off a final mainnet release

- Concrete FINAL qualified-revenue product/economic specification.
- Production Revenue Verifier implementation matching that specification.
- Final verifier, adapter and referral Program IDs generated under the custody runbook.
- Exact frozen verifier -> adapter and adapter -> referral bindings, including RevenueAuthority PDA.
- Final registration opening UTC.
- Final dependency lockfiles and verifiable builds for all three production programs.
- Independent disposition of all audit findings against the exact final commit/artifacts.
- Completed `release/mainnet-release.json` and green executable pre-mainnet gate.
- Controlled mainnet deployment, bytecode verification and full-chain limited smoke before permanent immutability.

## Out of scope unless explicitly added by the auditor

- Frontend UI/UX and wallet presentation.
- Business/legal/regulatory classification of the surrounding commercial model.
- Operational security of third-party systems not used by the production verifier.

Any system that supplies qualification evidence to the production verifier is not automatically out of scope: if compromise of that system can cause false qualified-revenue events, its trust assumptions must be documented and reviewed as part of the verifier boundary.
