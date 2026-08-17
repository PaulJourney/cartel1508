# External Audit Scope — SELF + 9-Upline Solana Protocol

Status: **pre-audit**. This document defines the intended independent review target. It is not an audit report or production approval.

## Production architecture in scope

The intended Mainnet release is a **single Solana program** plus the canonical SPL Token Program:

`buyer/claimant signer -> Service Referral Protocol -> canonical USDT/USDC accounts -> accounting -> claim / expiry settlement`

Buyer-signed `purchase_and_distribute` is the only production economic event that creates new liabilities. There is no production Revenue Evidence, Revenue Qualification, Revenue Adapter or Revenue Verifier program.

### Files in the review boundary

- `programs/service_referral_protocol/src/lib.rs`
- `programs/service_referral_protocol/src/state.rs`
- `programs/service_referral_protocol/src/math.rs`
- `programs/service_referral_protocol/src/constants.rs`
- `programs/service_referral_protocol/Cargo.toml`
- root `Cargo.toml` and `Cargo.lock`
- `tests/reference-model.mjs`
- `tests/rank-model.mjs` only to verify that rank metadata cannot alter payouts
- `integration-tests/**`
- `scripts/static-gates.py`
- `scripts/pre-mainnet-gate.py`
- `scripts/devnet-comprehensive-validation.mjs`
- `scripts/devnet-security-adversarial.mjs`
- `scripts/devnet-rpc-guard.mjs`
- `release/mainnet-release.example.json`
- `.github/workflows/ci.yml`
- `.github/workflows/security-scan.yml`
- `.github/workflows/verifiable.yml`
- `.github/workflows/local-pre-mainnet-validation.yml`
- `.github/workflows/devnet-deploy-smoke.yml`
- `README.md`
- `SECURITY.md`
- `PROGRAM_ID_CUSTODY_RUNBOOK.md`
- `MAINNET_FINALIZATION_RUNBOOK.md`
- the exact frontend/client transaction builder used for production transactions

Historical `scripts/migrate-*` utilities are migration history, not production runtime. Dead Revenue Adapter release tooling must not remain in the final release tree.

## Frozen economics

For every supported stablecoin purchase:

- **1 USDT/USDC = 1 logical unit**.
- Buyer is **SELF** and receives 50% subject to activity/expiry rules.
- Immutable direct sponsor is U1 and receives 15% subject to its own activity/depth.
- Eight additional ancestors complete exactly nine network uplines:
  - U1 15%
  - U2 9%
  - U3 6%
  - U4 4%
  - U5 2.5%
  - U6 2%
  - U7 1.5%
  - U8 1%
  - U9 2%
- Pioneer pool: 2%.
- Service/platform: 5%.
- Aggregate allocation: **50 + 43 + 2 + 5 = 100%** before deterministic integer handling.
- No self-reentry genealogy position exists.
- A tenth ancestor must never receive value from a target purchase.

## Progressive ACTIVE / GRACE / qualification

ACTIVE lasts 7 days; GRACE lasts 48 hours. Claims require ACTIVE status.

The next successfully-started ACTIVE week requires:

- weeks 1–2: 10 units;
- weeks 3–4: 20;
- weeks 5–6: 30;
- weeks 7–8: 40;
- week 9 onward: 50, permanently capped.

Required audit properties:

- the requirement advances only when a new ACTIVE week is successfully started;
- calendar inactivity alone never advances `active_weeks_started`;
- purchases while already ACTIVE add only to current-week personal units/depth and never prequalify the next week;
- GRACE/INACTIVE purchases may accumulate toward the **current next requirement** inside one seven-day partial-qualification window;
- stale partial progress resets without advancing the weekly ladder;
- during a live INACTIVE partial-qualification window, **only buyer SELF** is provisionally preserved for batching equivalence;
- network and Pioneer have no partial-window exception;
- if the partial window expires before the current next requirement is reached, the preserved SELF becomes Treasury-destined on the next settlement/touch;
- late reactivation cannot rescue value whose GRACE already expired because stale value is settled before reactivation.

## Weekly network depth

Each upline's own current ACTIVE-week personal units unlock the maximum fixed level that wallet may monetize:

- 10 → U1–U3
- 25 → U1–U4
- 50 → U1–U5
- 100 → U1–U6
- 200 → U1–U7
- 350 → U1–U8
- 500+ → U1–U9

Required audit properties:

- depth is prospective only;
- unlocking a deeper level cannot recover a previously locked share;
- an ACTIVE/GRACE upline lacking sufficient depth sends that scheduled amount to Treasury/unallocated;
- an INACTIVE upline sends its scheduled amount to Treasury/expired;
- neither case compresses the percentage to another ancestor;
- GRACE retains the depth of the just-finished ACTIVE week while preserved pending value remains eligible.

## Pioneer position properties

Pioneer is a separate global pool, not genealogy.

- absolute global cap: **100 positions**;
- registration assigns zero;
- one purchase creates `floor(units / 1000)` candidate positions;
- separate purchases do not accumulate: `500 + 500` creates zero;
- one wallet may own multiple/all positions;
- actual assignment is capped by remaining slots;
- at 98/100, a 3,000-unit purchase creates exactly two final positions;
- at 100/100, no later purchase creates another position;
- only gross buyer-signed purchase units qualify;
- **Rule B:** current purchase's Pioneer accrual occurs before positions earned by that purchase are assigned;
- weighted high-precision checkpoints prevent retroactive rewards when positions are acquired at different times;
- each assigned position is one equal share of the fixed 2%/100 schedule;
- unassigned virtual-slot value is Treasury-destined rather than redistributed;
- positions remain owned through inactivity, but unclaimed Pioneer due expires once INACTIVE and cannot be rescued by reactivation.

## Purchase-path integrity

The auditor must verify:

- `purchase_and_distribute` is the sole production economic entrypoint;
- payment equals exactly `units * 10^6` atomic units for six-decimal supported mints;
- zero units and purchases exceeding SPL Token `u64` payment capacity are rejected;
- source token authority must be buyer signer;
- unsupported mint is rejected;
- vault accounts are canonical SPL Token ATAs for vault-authority PDA + expected mint;
- Service-Treasury accounts are canonical ATAs for frozen Treasury + expected mint;
- buyer `UserState` is bound to buyer signer PDA;
- sponsor and U2–U9 account sequence is independently validated against immutable ancestry;
- technical-root termination routes missing ancestry deterministically to Treasury;
- buyer payment, Unit allocation, activity update, SELF/network/Pioneer accounting and Treasury routing are one atomic transaction;
- Pioneer current-event accrual happens before new-position assignment;
- failed ancestry/account/arithmetic/CPI validation rolls back every prior state/token mutation;
- global Unit IDs are monotonic, unique and overflow-safe.

## Registration security

Required adversarial review/test coverage:

- valid referrer wallet + referrer PDA must match;
- spoofed wallet/PDA pair is rejected atomically;
- same wallet cannot register twice because its canonical `UserState` already exists;
- structural self-referral cannot create a `UserState` or change `real_user_count`;
- failed registration cannot leave a partially initialized account or advance protocol counts.

## Claim / settlement security

- claim requires claimant signature and claimant-bound `UserState` PDA;
- claimant must be ACTIVE;
- claim vault must be canonical;
- destination must be claimant's canonical ATA;
- claimant cannot clear or redirect another user's accounting;
- successful claim consumes only current SELF/network/Pioneer entitlement;
- immediate repeated/double claim must fail and leave `lifetime_claimed`, balances and vault unchanged;
- failed claim must roll back any preliminary accounting/checkpoint mutation;
- expired value can be physically settled only once;
- permissionless settler cannot redirect value away from the frozen Treasury.

A dedicated adversarial case must also force a buyer payment CPI to fail **after** expiry settlement has begun. The entire transaction must restore old entitlement, expiry counters, purchase index, qualification/activity state, global Unit ID and token balances. The entitlement must remain settleable exactly once afterward.

## Vault and accounting properties

- vault collateral must equal outstanding claim liabilities;
- Treasury movement must equal service + unallocated + expired + rounding + Pioneer-unassigned flows;
- USDT and USDC rails cannot contaminate each other;
- integer rounding cannot create value, underflow or orphan liabilities;
- Pioneer fractional carry is conserved at configured precision;
- Pioneer checkpoints cannot exceed `global_index * wallet_positions`;
- complete claim sweeps in the comprehensive scenario must close USDT and USDC vaults to zero before the separate security add-on begins;
- the security add-on may intentionally leave a precisely asserted residual Pioneer liability caused by its additional post-saturation purchase; that residual must equal the independently computed liability and is not evidence of an accounting leak.

## Rentless audit trail

Purchases must not create a persistent per-purchase PDA.

- `UserState.next_purchase_index` is per-user monotonic.
- `ProtocolState.next_unit_id` is globally monotonic.
- successful purchase emits `UnitsPurchased` with buyer, mint, purchase index, units, Unit range, Pioneer positions added/total and activity/depth fields.
- `PurchaseAndDistribute` must not require `SystemProgram` or a payer-created batch account.
- failed transactions must not be indexed as successful purchases.

## Authority / Mainnet configuration

- no owner/admin instruction may mutate payouts, Treasury, mints, genealogy, Pioneer rules or activity/depth rules;
- Mainnet Treasury is frozen to `AepYo8xanmKuRiLVeYQuCTJoQr1nyKiTApoKwHMEg8fn`;
- Mainnet USDT is frozen to `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB`;
- Mainnet USDC is frozen to `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`;
- production initialization must remain fail-closed until final Program ID and future registration-open UTC are frozen;
- Program ID / deployment private keys must never be committed or uploaded to CI;
- upgrade authority remains only through controlled deploy/verification/smoke and is permanently removed last.

## Automated evidence required before sign-off

The appropriate exact source commits must provide green evidence for:

- reference economics/conservation model;
- rank model proving rank cannot affect payouts;
- static economic/security gates;
- production compile;
- Rust unit/property tests;
- LiteSVM time-boundary and adversarial tests;
- false ancestry atomic rollback;
- progressive ACTIVE ladder and no calendar progression;
- full U1–U9 depth boundaries and tenth-ancestor exclusion;
- split-purchase batching equivalence;
- Pioneer zero-on-registration, non-cumulative threshold, Rule B, weighted positions and 100/100 cap;
- double-claim rejection;
- CPI rollback after expiry settlement begins;
- comprehensive isolated Solana-validator execution using real SPL accounts/PDAs/signatures;
- isolated runtime-security adversarial execution;
- RustSec scan bound to exact PR head;
- reproducible/verifiable production build and SHA-256 comparison;
- **final public Solana Devnet comprehensive + runtime-security execution** with exact source/evidence binding;
- **final production-runtime validation bound to the final Mainnet release SHA after Program ID/timestamp freeze**.

Historical public-Devnet smoke or historical qualified-revenue/adapter evidence is supporting history only; it is not a substitute for these final gates.

## Explicit blockers before Mainnet approval

- final public Devnet comprehensive + security evidence is not `passed`;
- final Program ID has not been generated offline and frozen;
- registration-open UTC is not frozen to a future timestamp;
- canonical Mainnet vault/Treasury ATA bootstrap procedure is not included and verified;
- final production artifact has not been reproduced/hash-matched;
- production-runtime validation is not `passed` or is not bound to the final release SHA;
- independent audit has not completed on the exact frozen core + production transaction builder, or findings remain unresolved;
- `release/mainnet-release.json` is incomplete;
- executable pre-mainnet gate is not fully green;
- Mainnet deploy bytecode has not been verified against the audited artifact before initialization;
- controlled small Mainnet smoke has not passed;
- deployed bytecode/state has not been reverified before permanent upgrade-authority removal.

## Out of scope unless separately commissioned

- legal/regulatory classification of the commercial/referral model;
- frontend visual design;
- marketing/business claims.

The production transaction builder remains security-relevant because it supplies the account sequence, even though the on-chain program must independently validate every security-critical relation.
