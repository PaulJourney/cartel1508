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
- Pioneer pool: 2%, represented by exactly 100 equal virtual positions under the purchase-earned rule below.
- Service/platform: 5%.
- Aggregate allocation is exactly **50% + 43% + 2% + 5% = 100%** before deterministic integer-rounding handling.
- No separate self-reentry genealogy position exists; repeated purchases are additional units of the same immutable user.
- A tenth network ancestor must never receive value from a target purchase.

## Frozen Pioneer position properties

The auditor must treat the Pioneer rule as a separate global-pool mechanism, not as part of genealogy.

- Total Pioneer supply is capped absolutely at **100 positions**.
- Registration alone assigns zero Pioneer positions.
- A single purchase creates `floor(units / 1000)` candidate positions.
- Purchases are non-cumulative for Pioneer qualification: e.g. `500 + 500` in separate transactions creates zero positions.
- One wallet may own multiple Pioneer positions.
- Actual assignment is `min(candidate_positions, 100 - positions_already_assigned)`.
- At 98/100, a 3,000-unit purchase must create exactly 2 positions.
- Once the protocol reaches 100/100, no future purchase can ever create another Pioneer position.
- Only gross units in `purchase_and_distribute` qualify. SELF/network/Pioneer rewards, claims, wallet balances or other token receipts cannot create positions.
- **Rule B is mandatory:** the purchase that creates positions must first account for its own 2% using only positions existing before that purchase. Newly created positions begin earning from the next global purchase.
- A wallet acquiring positions at different times must not receive retroactive Pioneer value; weighted reward-debt/checkpoint arithmetic must preserve each entry point.
- Every position is one equal share of the fixed 2%/100 schedule.
- Value corresponding to unassigned positions is treasury-destined and must not be redistributed among existing Pioneer holders.

## Activity, qualification and IC-A properties

- Activity threshold is 10 units inside the qualification window.
- ACTIVE lasts 7 days; GRACE lasts 48 hours.
- Claims require ACTIVE status.
- GRACE preserves unclaimed value long enough for timely requalification.
- **IC-A is fixed-depth:** when a user is INACTIVE, that user's own scheduled/unclaimed value becomes treasury-destined; the next active ancestor does not inherit the percentage.
- Late reactivation cannot rescue value whose grace period has already ended because stale value is settled before reactivation.
- During a live partial qualification window, buyer-own SELF and any already-owned Pioneer entitlement are provisional so `10 x 1` unit purchases and `1 x 10` units have equivalent buyer-own economics when qualification is completed in time.
- Small purchases that preserve activity economics must **not** accumulate toward a Pioneer position; Pioneer qualification remains per-transaction at 1,000 units.
- Network-upline amounts are not protected by the partial-qualification exception and remain governed by IC-A.
- If the qualification window expires without reaching 10 units, provisional buyer-own value becomes treasury-destined on the next settlement/touch.

## Purchase-path properties to review

- `purchase_and_distribute` is the sole production economic entrypoint.
- Payment is exactly `units * 10^6` atomic units for six-decimal USDT/USDC.
- Unsupported mints are rejected.
- Source-token authority must be the buyer.
- Vault and treasury token accounts must be canonical ATAs for the expected owner/mint.
- Buyer funds enter the canonical vault before liabilities are released from it.
- Unit allocation, activity update, SELF/network/Pioneer accounting, Pioneer-position assignment and treasury routing are atomic in one Solana transaction.
- Pioneer pool accrual for the current event must execute before Pioneer positions from that event are assigned.
- Sponsor and eight ancestor account inputs must exactly match immutable ancestry.
- Technical-root termination deterministically routes the remaining network depth to treasury.
- Failed ancestry, token-account, arithmetic or CPI validation must roll back token movement and state.
- Global Unit IDs are monotonic, unique and overflow-safe.
- Large purchases cannot overflow the SPL Token `u64` payment amount.

## Rentless purchase audit trail

Purchases **must not create a persistent per-purchase PDA**.

- `UserState.next_purchase_index` provides a monotonic per-user purchase index.
- `ProtocolState.next_unit_id` provides global logical Unit IDs.
- Each successful purchase emits `UnitsPurchased` containing buyer, mint, purchase index, units, first Unit ID, last Unit ID, `pioneer_positions_added`, total assigned Pioneer positions and timestamp.
- `PurchaseAndDistribute` must not require `SystemProgram` or a payer-created `UnitBatch` account.
- The event-based history must not weaken the economic/state invariants; a failed transaction must not be treated by clients/indexers as a successful purchase.

## Vault and accounting properties

- Vaults remain fully collateralized for all outstanding claim liabilities.
- Treasury movements equal service + explicitly unallocated + expired + rounding + Pioneer-unassigned value.
- USDT and USDC accounting rails cannot contaminate one another.
- Integer rounding cannot create value, underflow or orphan liabilities.
- Pioneer fractional carry is conserved at configured precision.
- Weighted Pioneer checkpoints remain bounded by `global_index * wallet_positions` and cannot create retroactive or duplicated value.
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

- No owner/admin instruction can mutate percentages, treasury, referral relationships, supported mints, Pioneer threshold/cap/Rule B or accounting rules after initialization.
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
- Pioneer registration consumes zero positions;
- separate `500 + 500` purchases create zero Pioneer positions;
- one 1,000-unit purchase creates one position;
- one wallet can hold multiple Pioneer positions;
- Rule B excludes newly created positions from the creating transaction;
- 98/100 + 3,000 units creates exactly two final positions;
- 100/100 permanently blocks additional positions;
- weighted Pioneer accounting prevents retroactive rewards when positions are acquired at different times;
- unsupported mint / wrong authority / noncanonical ATA rejection;
- Unit-ID continuity/overflow tests;
- absence of per-purchase rent accounts and presence of `UnitsPurchased` audit event;
- RustSec dependency scan;
- reproducible/verifiable production build + SHA-256;
- devnet smoke using the exact final ABI and Pioneer economics.

Historical qualified-revenue/adapter evidence is not production-release evidence.

## Explicit blockers before mainnet approval

- All exact-final-commit CI green.
- Final devnet smoke green. A faucet/rate-limit failure before deployment is not protocol evidence and must remain a blocker until a real devnet execution succeeds.
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
