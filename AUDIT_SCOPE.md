# External Audit Scope — SELF + 9-Upline Solana Protocol

Status: **pre-audit**. This document defines the intended independent review target. It is not an audit report or production approval.

## Production architecture in scope

The intended mainnet release is a **single immutable Solana program** plus the canonical SPL Token Program:

`User purchase -> Service Referral Protocol -> canonical USDT/USDC vault -> accounting -> pull claim / expiry settlement`

A successful unit purchase is the only production economic event that creates liabilities. There is no production Revenue Evidence, Revenue Qualification or Revenue Adapter program.

### Files in the production review boundary

- `programs/service_referral_protocol/src/lib.rs`
- `programs/service_referral_protocol/src/state.rs`
- `programs/service_referral_protocol/src/math.rs`
- `programs/service_referral_protocol/src/constants.rs`
- `programs/service_referral_protocol/Cargo.toml`
- root `Cargo.toml` and `Cargo.lock`
- `tests/reference-model.mjs`
- `integration-tests/**`
- `scripts/static-gates.py`
- `scripts/pre-mainnet-gate.py`
- `scripts/devnet-transaction-smoke.mjs`
- `release/mainnet-release.example.json`
- `.github/workflows/ci.yml`
- `.github/workflows/security-scan.yml`
- `.github/workflows/verifiable.yml`
- `.github/workflows/devnet-deploy-smoke.yml`
- `PROGRAM_ID_CUSTODY_RUNBOOK.md`
- `MAINNET_FINALIZATION_RUNBOOK.md`
- the exact frontend/client transaction builder used to construct production transactions

## Frozen economics to verify

For every supported stablecoin purchase:

- **1 USDT/USDC = 1 logical unit**.
- The buyer is **SELF** and receives the 50% SELF bucket subject to activity/expiry rules.
- The buyer's immutable direct sponsor is network U1 and receives 15% subject to activity/expiry rules.
- Eight additional ancestors complete exactly nine network uplines using:
  - U1 15%
  - U2 9%
  - U3 6%
  - U4 4%
  - U5 2.5%
  - U6 2%
  - U7 1.5%
  - U8 1%
  - U9 2%
- Pioneer pool: 2%, shared through the first-100 high-precision index.
- Service/platform: 5%.
- Aggregate allocation is exactly **50% + 43% + 2% + 5% = 100%** before deterministic integer-rounding handling.
- No separate self-reentry position exists; repeated purchases are additional units of the same immutable user.
- A tenth network ancestor must never receive value from a target purchase.

## Activity, qualification and IC-A properties

- Activity threshold is 10 units inside the qualification window.
- ACTIVE lasts 7 days; GRACE lasts 48 hours.
- Claims require ACTIVE status.
- GRACE preserves unclaimed value long enough for timely requalification.
- **IC-A is fixed-depth:** when a user is INACTIVE, that user's own scheduled/unclaimed value becomes treasury-destined; the next active ancestor does not inherit the percentage.
- Late reactivation cannot rescue value whose grace period has already ended because stale value is settled before reactivation.
- During a live partial qualification window, buyer-own SELF/Pioneer value is provisional so `10 x 1` unit purchases and `1 x 10` units have equivalent buyer-own economics when qualification is completed in time.
- Network-upline amounts are not protected by that partial-qualification exception and remain governed by IC-A.
- If the qualification window expires without reaching 10 units, provisional buyer-own value becomes treasury-destined on the next settlement/touch.

## Purchase-path properties to review

- `purchase_and_distribute` is the sole production economic entrypoint.
- Payment is exactly `units * 10^6` atomic units for six-decimal USDT/USDC.
- Unsupported mints are rejected.
- Source-token authority must be the buyer.
- Vault and treasury token accounts must be canonical ATAs for the expected owner/mint.
- Buyer funds enter the canonical vault before liabilities are released from it.
- Unit allocation, activity update, SELF/network/Pioneer accounting and treasury routing are atomic in one Solana transaction.
- Sponsor and eight ancestor account inputs must exactly match immutable ancestry.
- Technical-root termination deterministically routes the remaining network depth to treasury.
- Failed ancestry, token-account, arithmetic or CPI validation must roll back token movement and state.
- Global Unit IDs are monotonic, unique and overflow-safe.
- Large purchases cannot overflow the SPL Token `u64` payment amount.

## Rentless purchase audit trail

Purchases **must not create a persistent per-purchase PDA**.

- `UserState.next_purchase_index` provides a monotonic per-user purchase index.
- `ProtocolState.next_unit_id` provides global logical Unit IDs.
- Each successful purchase emits `UnitsPurchased` containing buyer, mint, purchase index, units, first Unit ID, last Unit ID and timestamp.
- `PurchaseAndDistribute` must not require `SystemProgram` or a payer-created `UnitBatch` account.
- The event-based history must not weaken the economic/state invariants; a failed transaction must not be treated by clients/indexers as a successful purchase.

## Vault and accounting properties

- Vaults remain fully collateralized for all outstanding claim liabilities.
- Treasury movements equal service + explicitly unallocated + expired + rounding + Pioneer-unassigned value.
- USDT and USDC accounting rails cannot contaminate one another.
- Integer rounding cannot create value, underflow or orphan liabilities.
- Pioneer fractional carry is conserved at configured precision.
- No claimant can clear another user's accounting or redirect another user's claim.
- Failed claims preserve accrued state atomically.
- Expired value cannot be physically settled twice.

## Gas / signer model

- Registration: registering user signs and pays user-state creation costs.
- Purchase: buyer signs and pays one Solana transaction fee; **no per-purchase account rent is created**.
- Accrual: SELF/sponsor/uplines/Pioneers require no separate transaction merely to accrue value.
- Claim: claimant signs and pays the claim transaction fee.
- Expiry settlement: submitting settler signs/pays; settlement cannot redirect funds away from the frozen treasury.
- No platform-funded transaction is required per commission event.

## Authority and immutability properties

- No owner/admin instruction can mutate percentages, treasury, referral relationships, supported mints or accounting rules after initialization.
- Mainnet service treasury is frozen to `AepYo8xanmKuRiLVeYQuCTJoQr1nyKiTApoKwHMEg8fn`.
- Mainnet USDT mint is frozen to `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB`.
- Mainnet USDC mint is frozen to `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`.
- Production initialization remains fail-closed until final Program ID and registration-open UTC are frozen.
- Program keypair/deploy secrets must never be committed.
- Upgrade authority remains during controlled deploy/verification/smoke and is permanently removed only after deployed bytecode matches the reviewed artifact.

## Automated evidence required before sign-off

The exact release commit must have green evidence for at least:

- pinned production compile;
- reference-model conservation tests;
- static economic/security gates;
- Rust unit/property tests;
- LiteSVM purchase/claim/expiry tests;
- exact SELF 50% and U1–U9 schedule tests;
- tenth-ancestor non-payment test;
- ancestry/account-substitution atomic rollback;
- ACTIVE/GRACE/INACTIVE and IC-A boundary tests;
- split-purchase (`10 x 1`) qualification equivalence;
- unsupported mint / wrong authority / noncanonical ATA rejection;
- Unit-ID continuity/overflow tests;
- absence of per-purchase rent accounts and presence of `UnitsPurchased` audit event;
- RustSec dependency scan;
- reproducible/verifiable production build + SHA-256;
- devnet smoke using the exact final ABI.

Historical qualified-revenue/adapter evidence is not production-release evidence.

## Explicit blockers before mainnet approval

- All exact-final-commit CI green.
- Final Program ID generated under the custody runbook and frozen consistently.
- Future registration-open UTC frozen.
- Exact dependency lockfile frozen.
- Independent audit completed against exact final commit/artifact with every finding dispositioned.
- `release/mainnet-release.json` completed with exact commit, Program ID, artifact hash, audit hash and verified-build evidence.
- Executable pre-mainnet gate fully green.
- Controlled mainnet deploy and deliberately limited smoke completed while upgrade authority is retained.
- Deployed bytecode verified against the audited artifact.
- Upgrade authority removed only after every preceding gate passes.

## Out of scope unless separately commissioned

- Legal/regulatory classification of the commercial/referral model.
- Frontend visual design.
- Marketing/business claims.

The transaction builder remains security-relevant because it supplies the sponsor/upline account sequence; the on-chain program must independently verify that ancestry, but the client must still be audited for deterministic, correct account construction.
