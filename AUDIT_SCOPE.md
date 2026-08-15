# External audit scope — Solana three-program release chain

Status: **pre-audit**. This document defines the complete independent review target. It is not itself an audit, security certification or production approval.

## Programs and release controls in scope

### Referral protocol

- `programs/service_referral_protocol/src/lib.rs`
- `programs/service_referral_protocol/src/state.rs`
- `programs/service_referral_protocol/src/math.rs`
- `programs/service_referral_protocol/src/constants.rs`
- `programs/service_referral_protocol/Cargo.toml`

### Revenue adapter

- `programs/revenue_adapter/src/lib.rs`
- `programs/revenue_adapter/Cargo.toml`

### Revenue qualification gateway

- `programs/revenue_qualification/src/lib.rs`
- `programs/revenue_qualification/Cargo.toml`

### Shared build, tests and release boundary

- root `Cargo.toml`
- root `Cargo.lock`
- `Anchor.toml`
- `integration-tests/Cargo.toml`
- `integration-tests/tests/**`
- `tests/reference-model.mjs`
- `scripts/static-gates.py`
- `scripts/pre-mainnet-gate.py`
- `scripts/qualification-localnet-smoke.mjs`
- `.github/workflows/ci.yml`
- `.github/workflows/revenue-adapter-ci.yml`
- `.github/workflows/qualification-localnet-smoke.yml`
- `.github/workflows/verifiable.yml`
- `.github/workflows/security-scan.yml`
- `release/mainnet-release.example.json`
- `QUALIFIED_REVENUE_SOURCE_SPEC.md`
- `PROGRAM_ID_CUSTODY_RUNBOOK.md`
- `MAINNET_FINALIZATION_RUNBOOK.md`
- `PRE_MAINNET_REVIEW.md`

The audit should treat the three programs as **one economic and authorization system**. A finding may cross program boundaries even when each individual instruction appears locally correct.

## Core referral properties to review

- No mutable owner/admin control surface.
- Immutable referral relationships and exact ancestry validation.
- Separation between service-unit/activity purchases and qualified-revenue accounting.
- Exact 50% direct / 43% ten-level / 2% Pioneer / 5% service allocation and conservation.
- Stablecoin mint, legacy SPL Token Program and canonical ATA validation.
- Vault collateral conservation and Solana transaction rollback assumptions.
- ACTIVE / GRACE / INACTIVE time-boundary behavior and expired-balance settlement.
- Pioneer high-precision index, fractional carry, first-100 assignment and non-retroactive semantics.
- Ten-level accounting, root/unallocated routing and rounding handling.
- Pull-based claim authorization and ACTIVE requirement.
- Global monotonic Unit ID allocation and overflow behavior.
- Production initialization pins immutable treasury, mints, qualified-revenue source and registration UTC.

## Revenue-adapter properties to review

- `AdapterConfig` is initialize-once and cannot later redirect referral or qualification identities.
- `RevenueAuthority` is a deterministic PDA with no private key.
- `qualification_authority` must be the deterministic PDA derived under the frozen qualification Program ID and must sign the submitted event.
- Only the configured referral executable may receive the CPI.
- Source token account must be the canonical USDT/USDC ATA of Revenue Authority.
- Source account ownership and available balance are checked before downstream liabilities are created.
- One deterministic `RevenueReceipt` PDA exists per event ID and exact replay cannot succeed.
- Receipt fields cannot be substituted after event acceptance.
- Adapter signer seeds cannot be abused to sign unrelated instructions or transfer arbitrary funds.
- No mutable admin/config instruction can change the trust boundary after initialization.
- Production feature cannot initialize while final referral/qualification identities remain sentinel/unfrozen.

## Revenue-qualification properties to review

- Payer must sign and own the payment source token account.
- Supported mint must match the immutable adapter configuration.
- Destination must be the canonical Revenue Authority ATA.
- Real SPL value transfer occurs before the adapter CPI and is rolled back if any later CPI fails.
- Qualification Authority is a deterministic PDA with no private key and has a narrowly scoped signing purpose.
- Event identity deterministically binds payer, beneficiary, mint, amount and client nonce.
- The expected adapter receipt PDA is recomputed from the derived event identity.
- Adapter account ownership and executable identity cannot be substituted.
- A malicious caller cannot substitute referral program, protocol state, beneficiary or ancestry to redirect accounting.
- Production feature cannot initialize/use an unfrozen adapter identity.
- No mutable admin/config/approval function provides a hidden permanent human reward authority.

## Cross-program properties to review

The auditor should explicitly trace one successful and multiple failing transactions across all CPI boundaries.

### Successful qualified-revenue path

1. A legitimate evidence source qualifies an economic service-revenue event.
2. Payer-funded USDT/USDC moves into the adapter Revenue Authority ATA.
3. Qualification Authority PDA signs CPI into the adapter.
4. Adapter verifies event uniqueness and creates its deterministic receipt.
5. Revenue Authority PDA signs CPI into the referral protocol.
6. Referral protocol independently validates the immutable qualified-revenue source.
7. Gross value is transferred into the protocol vault before liabilities are allocated.
8. 50/43/2/5 accounting is applied and immediately treasury-assigned amounts leave the vault.
9. Only outstanding user/Pioneer liabilities remain collateralized in the reward vault.

### Failure paths

Review that all state/token mutations roll back for at least:

- exact event replay;
- wrong Qualification Authority;
- wrong adapter/referral executable;
- wrong Revenue Authority or non-canonical ATA;
- unsupported mint or mint mismatch;
- insufficient payer/source funding;
- substituted beneficiary;
- substituted ancestry/upline accounts;
- invalid protocol/vault/treasury accounts;
- downstream referral rejection after payer transfer and receipt initialization have already begun.

## Critical economic-evidence question

The current on-chain gateway proves that real stablecoin value is payer-funded and routed atomically. **That alone is not sufficient evidence that the payment represents legitimate external service revenue.** A participant could otherwise self-fund a transaction and manufacture an economically artificial reward event.

The final production evidence integration must therefore be reviewed as part of the trust model before deployment. The auditor must answer:

- What exact event constitutes legitimate service revenue?
- Can an arbitrary participant create that evidence themselves?
- What immutable external/on-chain identifier prevents the same economic event from being represented twice with different nonces?
- How are payer, beneficiary, mint and gross amount bound to that evidence?
- If an oracle/attestation exists, who can sign, how is unilateral permanent human control prevented, and how is key compromise handled?
- Can refunds, chargebacks, reversals or disputes create an irreversible mismatch between business revenue and referral liabilities?
- Can two integrations or two external records refer to the same underlying payment and create duplicate rewards?

A generic rule of “any wallet may pay X stablecoins and nominate beneficiary Y” must not be accepted as the final qualification rule merely because it is fully collateralized.

## Release-engineering properties to review

- Three final Program IDs must be distinct and replace all development identities.
- Adapter production bindings must exactly equal final referral + qualification Program IDs.
- Qualification production binding must exactly equal final adapter Program ID.
- Final adapter `RevenueAuthority` PDA must exactly equal core `MAINNET_QUALIFIED_REVENUE_SOURCE`.
- `[programs.mainnet]` must contain all three exact identities.
- Release manifest must match the exact git commit and frozen source constants.
- Release manifest must contain SHA-256 values for all three exact production `.so` artifacts.
- `scripts/pre-mainnet-gate.py` should prefer Docker/verifiable artifacts and fail closed when any identity, artifact, audit evidence, qualification-evidence hash or smoke approval is missing/mismatched.
- `qualification_evidence_spec_sha256` must match the reviewed repository specification.
- `audit_report_sha256` must identify the exact independent report covering this exact release chain.
- Secret-like Program ID/deployer material must never be committed.
- Any source/config change after audit/build evidence invalidates the corresponding release evidence.

## Upgrade-authority / immutability procedure to review

- Three final Program ID private keypairs are generated and kept outside repository/CI/ChatGPT.
- Temporary deployer/upgrade authority is separate from Program IDs, treasury and PDAs.
- All three exact bytecodes are verified after controlled mainnet deployment.
- Cross-program identities and PDA derivations are verified on-chain before finalization.
- Limited mainnet smoke is completed before authority removal.
- Upgrade authority is removed from each program intended to become immutable only after the whole coordinated release is verified.
- No program is finalized while another still requires a binding/configuration change.

## Automated evidence currently available

### Referral baseline

- Pinned toolchain: Solana `3.1.10`, Anchor `1.1.2`.
- Reference-model and deterministic accounting/property tests cover exact conservation and global Unit ID invariants.
- LiteSVM tests cover initialization, service-unit purchases, source authorization, ancestry validation, network routing, claim lifecycle, grace/expiry and Unit IDs.
- Previous production-equivalent devnet core smoke passed initialize, Pioneer #1 registration, 10-unit purchase, separately funded qualified revenue, 50/43/2/5 accounting and ACTIVE claim.
- Prior core devnet smoke conserved exactly 20 test tokens: `5.002` to user, `14.998` to treasury and `0` in reward vault after claim.

### Three-program trust-chain evidence

- All three programs compile together under the pinned locked workspace dependency graph.
- Permanent three-program CI run `31900218190` completed successfully after the qualification program was added to the workspace/build graph.
- Real isolated `solana-test-validator` end-to-end Actions run `31899960367` completed successfully.
- Evidence artifact: `qualification-localnet-smoke-evidence`, artifact ID `9250852374`, archive SHA-256 `4ed7d16bc13f036f5194195d48c7fe87fdc8ae0f0919a255ea27a1ecdf1f2f48`.
- That run generated ephemeral test-only identities, built/deployed all three `.so` artifacts and executed real transactions.
- It proved successful payer-funded qualification, exact replay rejection, downstream ancestry-failure rollback including receipt rollback, and final claim.
- It conserved exactly 310 test USDC: `50.02` final user/Pioneer, `200` customer remainder, `59.98` service treasury, `0` referral reward vault and `0` adapter Revenue Authority ATA.
- The full qualification runtime e2e gate is intentionally enforced on real `solana-test-validator`; a known LiteSVM multi-program SPL-mint fixture anomaly is retained as a compiled diagnostic test but excluded from authoritative runtime gating.

The final audit handoff must additionally include the most recent green RustSec, protocol-CI and Docker-verifiable run IDs/artifact hashes after the current release-engineering changes finish executing. Do not substitute an older green run for final release evidence after a relevant source/build change.

## Explicit mainnet blockers

- Concrete, non-self-generable production qualification/evidence model approved by independent review.
- Three final Program IDs and secure offline key custody.
- Final Revenue Authority and Qualification Authority PDA derivations.
- Frozen cross-program production bindings.
- Final registration opening UTC.
- Exact `[programs.mainnet]` entries.
- Independent third-party audit of the exact final three-program commit and disposition of findings.
- Final locked + Docker-verifiable build after every immutable value/evidence rule is frozen.
- Completed `release/mainnet-release.json` with three artifact hashes, audit hash and qualification-evidence-spec hash.
- `python3 scripts/pre-mainnet-gate.py` returns `READY FOR CONTROLLED MAINNET DEPLOYMENT`.
- Controlled mainnet deployment and bytecode verification for all three programs.
- Limited mainnet smoke before permanent authority removal.

## Out of scope unless explicitly added by the final engagement

- Frontend/UI and wallet presentation.
- Legal/regulatory/business classification of the surrounding commercial model.
- General security of external business systems not used as the production qualification evidence source.

However, **the trust assumptions, signer/attestation model, event uniqueness, replay semantics and interface of any external system used to qualify production revenue are in scope**, because those directly determine whether referral rewards can be fabricated or duplicated.
