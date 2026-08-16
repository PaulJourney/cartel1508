# External audit scope — Solana V0.12

Status: pre-audit. This document defines the intended independent review target; it is not an audit report or production approval.

## Production architecture in scope

The final reviewed system is a four-program chain:

`Revenue Evidence -> Revenue Qualification -> Revenue Adapter -> Service Referral Protocol -> SPL Token Program`

Revenue Evidence implements the FINAL real-world/on-chain qualification semantics and emits canonical evidence. Revenue Qualification accepts evidence only from the frozen evidence program, binds it to the frozen adapter and canonical RevenueAuthority funding, and CPI-signs as the adapter's verifier PDA. Revenue Adapter enforces verifier-PDA authorization, canonical prefunded RevenueAuthority token accounts, immutable receipts and atomic forwarding. Service Referral Protocol performs the 50/43/2/5 accounting and user lifecycle logic.

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

### Revenue Qualification

- `programs/revenue_qualification/src/lib.rs`
- `programs/revenue_qualification/Cargo.toml`
- its exact dependency lockfile
- `QUALIFIED_REVENUE_QUALIFICATION_SPEC.md` in FINAL status

### Revenue Evidence

- `programs/revenue_evidence/**` once the production implementation exists
- its exact dependency lockfile
- every upstream oracle, attestation, payment-settlement or signer dependency capable of causing a qualified evidence event
- `QUALIFIED_REVENUE_QUALIFICATION_SPEC.md` in FINAL status

### Cross-program/release controls

- `adapter-integration-tests/**`
- `integration-tests/tests/qualified_revenue_hardened_e2e.rs`
- `integration-tests/fixtures/evidence_stub/**` as test-fixture evidence only, never production code
- `test-programs/test_revenue_verifier/**` as legacy/test authorization evidence only, never production code
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
- Adapter config binds one Revenue Qualification Program ID and the exact referral Program ID.
- Verifier authorization requires the deterministic qualification-program PDA signer; an ordinary wallet signer cannot substitute for it.
- `RevenueAuthority` is a deterministic adapter PDA with no private key.
- Qualified funds must already exist in RevenueAuthority's canonical ATA before referral liabilities are created.
- Each event creates a deterministic receipt PDA keyed by the unique 32-byte event ID.
- Receipt binds event ID, evidence hash, beneficiary, mint, gross amount, accepted timestamp and slot.
- Replay of a successful event cannot create a second liability.
- Failed downstream CPI consumes no receipt and creates no persistent token/accounting mutation.
- Adapter cannot redirect to an arbitrary referral program.

## Revenue-Qualification properties to review

- Immutable config binds exactly one executable Revenue Evidence program, its deterministic evidence-authority PDA, one Revenue Adapter program/config and the frozen USDT/USDC rails.
- Only the evidence-authority PDA of the frozen evidence program can authorize qualification.
- Evidence account identity is deterministic from the event ID and must be owned by the frozen evidence program.
- Evidence binds event ID, qualification Program ID, adapter Program ID, RevenueAuthority, beneficiary, mint, amount, settlement timestamp and non-zero reference hash.
- Requested amount cannot diverge from evidence amount.
- Beneficiary cannot be substituted after evidence issuance.
- Only the canonical RevenueAuthority ATA is accepted and it must be sufficiently prefunded before CPI.
- Revenue Qualification signs the exact adapter verifier PDA and cannot redirect to another adapter.
- Production constants fail closed until final Program IDs are frozen.
- Failed qualification/adapter/referral CPI leaves no partial accounting mutation.

## Revenue-Evidence properties to review once implemented

The production evidence source must be audited against the exact FINAL `QUALIFIED_REVENUE_QUALIFICATION_SPEC.md`. Review at minimum:

- exact event that qualifies as service revenue;
- explicit exclusion of service-unit/activity purchases;
- funding provenance and economic finality;
- deterministic beneficiary binding;
- deterministic amount derivation;
- canonical unique-event-ID derivation;
- canonical evidence/reference-hash derivation;
- refund, cancellation, chargeback and reversal treatment;
- immutable or explicitly governed trust source/oracle/attestation model;
- absence of a generic permanent admin ability to fabricate arbitrary funded events;
- supported stablecoin/mint constraints;
- replay and failure semantics.

Any external oracle, signer set, upstream program, payment processor or attestation system whose compromise could issue false evidence is part of the security boundary and its key-rotation, finality and failure model must be explicitly dispositioned by the auditor.

## Cross-program properties to review

- Evidence-authority PDA -> Revenue Qualification signer propagation is valid only during the intended CPI frame.
- Revenue Qualification verifier PDA -> Revenue Adapter signer propagation is valid only during the intended CPI frame.
- Adapter RevenueAuthority PDA -> Referral signer propagation cannot be forged externally.
- Final Evidence, Qualification, Adapter and Referral Program IDs are distinct and frozen consistently.
- Qualification `MAINNET_EVIDENCE_PROGRAM` equals the exact final Revenue Evidence Program ID.
- Qualification `MAINNET_ADAPTER_PROGRAM` equals the exact final Revenue Adapter Program ID.
- Adapter `MAINNET_VERIFIER_PROGRAM` equals the exact final Revenue Qualification Program ID.
- Core `MAINNET_REVENUE_ADAPTER_PROGRAM` equals the exact final Revenue Adapter Program ID.
- Core `MAINNET_QUALIFIED_REVENUE_SOURCE` equals the exact `[b"revenue-authority"]` PDA derived from that final adapter Program ID.
- One transaction cannot partially persist evidence, receipt creation, token movement or referral accounting if any downstream step fails.
- The release manifest, FINAL qualification-spec hash, source commit and all four artifact hashes are mutually consistent.

## Automated evidence already available before the final audit freeze

- Pinned toolchain: Solana 3.1.10 and Anchor 1.1.2.
- Referral core reference/static/Rust/property/LiteSVM tests pass on the hardened integration branch.
- Revenue Adapter Rust unit tests and SBF candidate build pass on the hardened integration branch.
- Revenue Qualification Rust unit tests and SBF candidate build pass on the hardened integration branch.
- Test-only evidence fixture SBF build passes and its production feature is intentionally compile-blocked.
- Four-program LiteSVM end-to-end testing passes for:
  - canonical Evidence -> Qualification -> Adapter -> Referral CPI;
  - exact 50/43/2/5 allocation conservation;
  - canonical RevenueAuthority prefunding;
  - beneficiary/evidence/reference-hash binding;
  - duplicate event ID rejection;
  - evidence/request amount mismatch rejection;
  - rollback of nested evidence/receipt/accounting mutations on failure.
- Earlier adapter adversarial tests also cover unsigned verifier PDA rejection and ordinary-wallet signer substitution rejection.
- Referral-core production-equivalent devnet transaction smoke passed on 2026-08-14 with an ephemeral Program ID and mock six-decimal SPL stablecoins: initialize, Pioneer #1 registration, 10-unit activity purchase, separately funded qualified-revenue event, 50/43/2/5 allocation and ACTIVE claim.
- That referral smoke conserved all 20 minted test tokens: `5,002,000` atomic units at the user, `14,998,000` at the test treasury and `0` in the vault after claim.

These are engineering evidences, not substitutes for the independent audit. Final audit evidence must refer to the exact immutable-value freeze commit and the exact four final artifacts; historical development hashes must not be treated as final-release hashes.

## Explicit blockers before the auditor can sign off a final mainnet release

- Concrete FINAL qualified-revenue product/economic specification.
- Production Revenue Evidence implementation matching that specification.
- Final Evidence, Qualification, Adapter and Referral Program IDs generated under the custody runbook.
- Exact frozen evidence -> qualification -> adapter -> referral bindings, including RevenueAuthority PDA.
- Final registration opening UTC.
- Final dependency lockfiles and verifiable builds for all four production programs.
- Independent disposition of all audit findings against the exact final commit/artifacts.
- Completed `release/mainnet-release.json` and green executable pre-mainnet gate.
- Controlled mainnet deployment, bytecode verification and full-chain limited smoke before permanent immutability.

## Out of scope unless explicitly added by the auditor

- Frontend UI/UX and wallet presentation.
- Business/legal/regulatory classification of the surrounding commercial model.
- Operational security of third-party systems that cannot affect production evidence issuance.

Any system that supplies or can cause qualification evidence is not automatically out of scope: if compromise of that system can cause false qualified-revenue events, its trust assumptions must be documented and reviewed as part of the Revenue Evidence boundary.