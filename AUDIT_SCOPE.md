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
- `integration-tests/Cargo.lock`
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
- `MAINNET_SMOKE_PLAN.md`
- `PRE_MAINNET_REVIEW.md`
- `AUDIT_HANDOFF.md`

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
- Canonical `AdapterConfig` PDA is independently re-derived, not merely accepted by owner/discriminator.
- Adapter executable identity is fixed and executable.
- Referral executable identity must match the referral Program ID frozen in AdapterConfig before token movement.
- Real SPL value transfer occurs before the adapter CPI and is rolled back if any later CPI fails.
- Qualification Authority is a deterministic PDA with no private key and has a narrowly scoped signing purpose.
- Event identity deterministically binds payer, beneficiary, mint, amount and client nonce.
- The expected adapter receipt PDA is recomputed from the derived event identity.
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

Review that all state/token mutations roll back or are rejected before value movement for at least:

- exact event replay;
- wrong Qualification Authority;
- non-canonical AdapterConfig;
- wrong adapter executable;
- executable referral substitution;
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
- Final Qualification Authority PDA must be derived from the final qualification Program ID and recorded in the release manifest.
- `[programs.mainnet]` must contain all three exact identities.
- Release manifest must match the exact git commit and frozen source constants.
- Release manifest must contain SHA-256 values for all three exact production `.so` artifacts.
- `scripts/pre-mainnet-gate.py` must independently derive both authority PDAs, self-test that derivation against real-validator vectors, prefer Docker/verifiable artifacts and fail closed when any identity, artifact, audit evidence, qualification-evidence hash or smoke approval is missing/mismatched.
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

### Three-program hardened runtime evidence

- `revenue-adapter-ci` run `31900961243` — **SUCCESS** for locked graph, Rust tests, three Anchor builds, production graph and artifact hashing.
- Real isolated `solana-test-validator` Actions run `31901949687` — **SUCCESS**.
- Evidence artifact: `qualification-localnet-smoke-evidence`, artifact ID `9251370786`, digest `sha256:5d27d17094deba2f136637986da4c3fd4e88ad52c5aee4e7860524185b95d6a2`.
- The validator workflow proves payer-funded qualification, exact replay rejection, **referral executable substitution rejection before token movement**, downstream ancestry-failure rollback including receipt rollback, ACTIVE claim and exact 310-USDC conservation.
- The runtime workflow asserts that substitution leaves customer, treasury, reward vault and Revenue Authority balances unchanged and creates no receipt.

### Full protocol CI and separate integration lock

- Protocol CI run `31901867138` — **SUCCESS**.
- Artifact `anchor-build-three-program-locked-production`, ID `9251365430`, digest `sha256:43b72fa42bc27222f019de4804593113c62077c9efe1e1e484d106c4074a4826`.
- It passed the reference model, extended static gates, explicit PDA derivation self-test, expected fail-closed release gate, production graph, all three builds, Rust tests, complete LiteSVM target compilation with the synchronized integration lock, permitted LiteSVM runtime tests, production build/hashing and artifact upload.
- `integration-tests/Cargo.lock` was regenerated with pinned Solana `3.1.10` / Anchor `1.1.2`; sync run `31901550667` compiled all integration targets under `--locked` before committing the lockfile.
- Full qualification runtime e2e remains authoritative on real `solana-test-validator`; the known LiteSVM multi-program SPL-mint fixture anomaly remains a compiled diagnostic but is excluded from authoritative full-runtime gating.

### Docker verifiable evidence

- Docker/verifiable run `31901171318` — **SUCCESS**.
- Artifact `anchor-verifiable-three-program-production`, ID `9251212906`, digest `sha256:572dfa7139ffa300fa7f0a980926704570eb43d70a8bfc6369a3063dbeaa5824`.
- Development artifact hashes:
  - referral `b257f3d588cec850b124a6b737e2d43032f0c292d8be06c4743722de76450194`
  - adapter `cfe900e16f1b114c00d9121505f4625392380f97c84bcf362982055e8843dba8`
  - qualification `aaa3a0a27f81c100109e2140e9f07b26e12aee2a61269e79bea84d6e33ad68cc`
- These are development evidence only and must be rebuilt after final Program IDs/evidence/timestamp are frozen.

### Dependency security

- RustSec run `31900619980` — **SUCCESS**.
- Evidence artifact ID `9251007681`.
- 0 known vulnerabilities in the production dependency graph.
- Informational only: transitive `bincode 1.3.3` is marked unmaintained; it is not reported as a known vulnerability by this scan.

## Explicit mainnet blockers

- Concrete, non-self-generable production qualification/evidence model approved by independent review.
- Three final Program IDs and secure offline key custody.
- Final Revenue Authority and Qualification Authority PDA derivations.
- Frozen cross-program production bindings.
- Final registration opening UTC.
- Exact `[programs.mainnet]` entries.
- Independent third-party audit of the exact final three-program commit and disposition of findings.
- Final locked + real-validator + RustSec + Docker-verifiable evidence after every immutable value/evidence rule is frozen.
- Completed `release/mainnet-release.json` with three artifact hashes, both authority PDAs, audit hash and qualification-evidence-spec hash.
- `python3 scripts/pre-mainnet-gate.py` returns `READY FOR CONTROLLED MAINNET DEPLOYMENT`.
- Controlled mainnet deployment and bytecode verification for all three programs.
- Limited approved mainnet smoke before permanent authority removal.

## Out of scope unless explicitly added by the final engagement

- Frontend/UI and wallet presentation.
- Legal/regulatory/business classification of the surrounding commercial model.
- General security of external business systems not used as the production qualification evidence source.

However, **the trust assumptions, signer/attestation model, event uniqueness, replay semantics and interface of any external system used to qualify production revenue are in scope**, because those directly determine whether referral rewards can be fabricated or duplicated.
