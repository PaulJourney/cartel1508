# Independent security audit handoff — pre-audit package

Status: **PRE-AUDIT / NOT A SECURITY CERTIFICATION / NOT A MAINNET APPROVAL**.

This document is the handoff index for an independent third-party security review of the complete Solana three-program trust chain. The auditor should review the exact final candidate commit supplied at engagement time, not rely on the development evidence IDs below as a substitute for reviewing the final frozen release.

## Repository / branch

- Repository: `PaulJourney/cartel1508`
- Development integration branch: `agent/revenue-adapter-v0.1`
- Pull request: PR #2, kept intentionally draft/pre-audit.
- Core base branch: `agent/solana-protocol-v0.8`

The final audit candidate commit will be recorded only after the production qualification evidence model, three final Program IDs, cross-program bindings and registration UTC are frozen.

## Programs to audit as one system

1. `service_referral_protocol`
2. `revenue_adapter`
3. `revenue_qualification`

Do not audit these as independent contracts only. The principal security properties cross CPI boundaries from qualification -> adapter -> referral and depend on deterministic PDA identities, token custody, event uniqueness and Solana transaction atomicity.

## Canonical scope documents

- `AUDIT_SCOPE.md` — detailed security questions and files in scope.
- `QUALIFIED_REVENUE_SOURCE_SPEC.md` — trust model and the unresolved economic-evidence boundary.
- `PRE_MAINNET_REVIEW.md` — readiness and blocker checklist.
- `PROGRAM_ID_CUSTODY_RUNBOOK.md` — final Program ID/private-key custody.
- `MAINNET_FINALIZATION_RUNBOOK.md` — controlled deploy / bytecode verification / authority removal.
- `MAINNET_SMOKE_PLAN.md` — draft limited live verification plan; explicitly not approved yet.
- `release/mainnet-release.example.json` — final public-evidence schema.
- `scripts/pre-mainnet-gate.py` — executable final release gate.

## Economic invariants

- Service-unit purchases never create referral liabilities.
- 1 stablecoin = 1 service unit.
- Qualified revenue is separately funded.
- Qualified gross revenue allocation is exactly 50% direct / 43% ten-level network / 2% Pioneer / 5% service allocation, subject to documented activity/root/Pioneer routing semantics.
- Liabilities are collateralized before accounting is committed.
- USDT and USDC remain separate rails.
- Network eligibility follows ACTIVE / GRACE / INACTIVE semantics.
- User claims are pull-based and require ACTIVE status.

## Three-program trust boundary

### Qualification gateway

The gateway requires a payer-signed SPL transfer, payer ownership of the source token account, supported mint, canonical Revenue Authority ATA and deterministic event identity. It re-derives the canonical AdapterConfig PDA, checks adapter/referral executable identities before transferring funds and signs the adapter CPI only through a deterministic Qualification Authority PDA.

### Revenue adapter

The adapter is initialize-once, binds referral + qualification identities, requires a deterministic Qualification Authority signer, accepts only the canonical Revenue Authority ATA, creates a deterministic receipt PDA for each event and signs the referral CPI only through the deterministic Revenue Authority PDA.

### Referral protocol

The core independently checks the immutable qualified-revenue source, transfers gross revenue into its vault before allocating liabilities, applies the economic split, immediately routes treasury-assigned portions and retains only outstanding user liabilities in the vault.

## Main unresolved production question

The current on-chain path proves **real payer-funded stablecoin movement and atomic accounting**. It does not prove that a payment is an independently legitimate external service/business revenue event.

A participant must not be able to manufacture a reward event simply by self-funding a payment and choosing a beneficiary. Before final audit sign-off and mainnet release, the actual product integration must define a concrete, non-self-generable evidence source/rule and the auditor must review its replay, uniqueness, payer/beneficiary/amount binding, refund/reversal and signer/oracle trust assumptions.

Until that evidence model is approved, production sentinels remain fail-closed and `qualification_evidence_status` must not be set to `approved`.

## Current automated evidence — development baseline

### Real Solana validator three-program smoke

Latest adversarial successful run:

- Actions run: `31901949687` — **SUCCESS**.
- Evidence artifact: `qualification-localnet-smoke-evidence`.
- Artifact ID: `9251370786`.
- Artifact digest: `sha256:5d27d17094deba2f136637986da4c3fd4e88ad52c5aee4e7860524185b95d6a2`.

It deploys all three programs to an isolated `solana-test-validator` with ephemeral Program IDs and proves:

- initialization and deterministic PDA linkage;
- Pioneer #1 registration;
- 10-unit activation and service-unit/reward-vault isolation;
- a separate payer-funded 100-USDC qualified event;
- qualification -> adapter -> referral CPI path;
- exact replay rejection with balances unchanged;
- **referral-executable substitution rejection before token movement, with customer/treasury/vault/Revenue Authority balances unchanged and no receipt created**;
- downstream ancestry failure with payer transfer + receipt creation rolled back;
- ACTIVE claim;
- exact conservation of all 310 test USDC.

The successful workflow itself is the runtime assertion: any failed balance, receipt, replay, substitution or conservation invariant exits non-zero and fails the Actions job.

### Three-program compile / production graph

- Hardened-source `revenue-adapter-ci`: run `31900961243` — **SUCCESS**.
- Covers locked graph, Rust tests, all three Anchor builds, production-feature graph and artifact hashing.

### Full protocol CI / integration dependency lock

- Final development-phase protocol CI: run `31901867138` — **SUCCESS**.
- Artifact: `anchor-build-three-program-locked-production`.
- Artifact ID: `9251365430`.
- Artifact digest: `sha256:43b72fa42bc27222f019de4804593113c62077c9efe1e1e484d106c4074a4826`.

This run passed:

- reference-model tests;
- extended static gates across all three programs;
- explicit Solana PDA derivation self-test;
- expected fail-closed pre-mainnet gate behavior;
- locked production dependency graph;
- all three Anchor builds;
- Rust tests;
- compilation of the complete LiteSVM integration target set with the synchronized lockfile;
- the permitted LiteSVM runtime suite;
- production build and three artifact hashes;
- release-gate development mode;
- artifact publication.

`integration-tests/Cargo.lock` is maintained separately from the production root lock. The qualification dependency was synchronized using Solana `3.1.10` / Anchor `1.1.2`; sync run `31901550667` compiled all integration targets with `--locked` before committing the lockfile and removing its temporary workflow.

The full qualification end-to-end runtime path is authoritative on real `solana-test-validator`; the known LiteSVM multi-program SPL-mint fixture anomaly is not used as the full-runtime release gate, although that target remains compiled as a diagnostic.

### Docker verifiable build

Most recent completed development-evidence run after release-gate PDA derivation:

- Actions run: `31901171318` — **SUCCESS**.
- Artifact: `anchor-verifiable-three-program-production`.
- Artifact ID: `9251212906`.
- Artifact digest: `sha256:572dfa7139ffa300fa7f0a980926704570eb43d70a8bfc6369a3063dbeaa5824`.

Development build hashes from that run:

- referral: `b257f3d588cec850b124a6b737e2d43032f0c292d8be06c4743722de76450194`
- adapter: `cfe900e16f1b114c00d9121505f4625392380f97c84bcf362982055e8843dba8`
- qualification: `aaa3a0a27f81c100109e2140e9f07b26e12aee2a61269e79bea84d6e33ad68cc`

These hashes are **not mainnet-final** because final Program IDs, evidence semantics and registration UTC are intentionally not frozen yet.

### Dependency security

- RustSec run: `31900619980` — **SUCCESS**.
- Evidence artifact ID: `9251007681`.
- 0 known vulnerabilities in the scanned production dependency graph.
- Informational warning only: transitively used `bincode 1.3.3` is marked unmaintained by RustSec; this should be considered in the toolchain/dependency review but is not reported as a known vulnerability by the scan.

### PDA release-gate self-test

`scripts/pre-mainnet-gate.py` includes a stdlib-only Solana PDA derivation implementation and self-tests it against public vectors generated by a real-validator smoke before trusting it for a release decision.

The final gate independently derives:

- Revenue Authority PDA from the final adapter Program ID;
- Qualification Authority PDA from the final qualification Program ID.

It requires exact agreement between derived PDAs, source constants and release-manifest values. Protocol CI requires the self-test to pass explicitly even while the development release remains intentionally `BLOCKED` for its mainnet sentinels.

## Evidence required from the final candidate

The development evidence above establishes the testing framework. It must **not** be reused as final audit/release evidence after immutable production values change.

For the exact final candidate, provide the auditor:

- exact git commit SHA;
- all three final public Program IDs;
- final Revenue Authority PDA and Qualification Authority PDA;
- exact `[programs.mainnet]` entries;
- final registration UTC;
- exact approved qualification/evidence specification + SHA-256;
- root and integration Cargo.lock files;
- final IDLs;
- final three Docker-verifiable `.so` artifacts + SHA-256 values;
- final Actions run URLs/artifact digests for protocol CI, real-validator smoke, RustSec and verifiable build;
- final release-manifest candidate;
- findings disposition for every audit finding.

Any code, dependency, Program ID, immutable binding, evidence rule or registration-time change after the auditor receives the candidate requires impact assessment and, where security-relevant, re-review/rebuild.

## Required auditor conclusions before `audit_status = passed`

The auditor should explicitly conclude whether:

- arbitrary/self-funded users can fabricate qualified revenue;
- the same underlying economic event can be counted more than once;
- payer, beneficiary, mint, amount, adapter, referral program, protocol state or ancestry can be substituted;
- token movements/receipts/liabilities fully roll back on downstream CPI failure;
- every liability is fully collateralized;
- service-unit funds are isolated from qualified-revenue funding;
- any mutable admin, key, config or upgrade mechanism remains that contradicts the advertised final trust model;
- all three production identities and authority PDAs are correctly frozen;
- the proposed deployment, bytecode-verification, smoke and upgrade-authority-removal sequence is safe for the exact final release.

Only an independent third party can supply the audit conclusion. This repository's tests, scripts and this handoff document are supporting evidence, not substitutes for that review.
