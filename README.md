# Service Referral Protocol — Solana V0.16

Public development repository for an ownerless Solana/Anchor referral protocol foundation.

## Frozen economic model

- Solana + Anchor/Rust.
- USDT and USDC are separate six-decimal accounting rails.
- **1 USDT/USDC = 1 globally unique logical unit.**
- Buyer-signed `purchase_and_distribute` is the sole production economic event.
- Referral relationships are immutable after registration.
- The buyer is **SELF** and receives **50%** of every purchase.
- The immutable direct sponsor is U1 and receives the first fixed network share.
- The network contains exactly **9 uplines** total.
- No separate self-reentry genealogy position exists.
- ACTIVE lasts 7 days; GRACE lasts 48 hours.
- Claims are pull-based and require the beneficiary to be ACTIVE.
- **IC-A is frozen:** once INACTIVE, unclaimed SELF/network/Pioneer value is permanently treasury-destined. There is no dynamic compression.

## Frozen allocation

For every purchase:

- **SELF / buyer: 50%**
- **9 network uplines: 43%**
- **Pioneer pool: 2%**
- **Service/platform: 5%**

The nine fixed network weights are:

- U1 — direct sponsor: 15%
- U2: 9%
- U3: 6%
- U4: 4%
- U5: 2.5%
- U6: 2%
- U7: 1.5%
- U8: 1%
- U9: 2%

Total: **50 + 43 + 2 + 5 = 100%**, subject only to deterministic integer accounting.

A commercial representation may call this **10 economic levels including SELF**. Internally and for audit, use **SELF + 9 uplines**.

### Example — 100 USDC purchase

Assume Mario buys 100 units and, independently, each relevant upline is ACTIVE/GRACE and has unlocked enough weekly personal depth for its scheduled level:

- Mario / SELF: 50 USDC
- U1: 15 USDC
- U2–U9: 9, 6, 4, 2.5, 2, 1.5, 1 and 2 USDC
- Pioneer pool: 2 USDC to positions that existed before this purchase; unassigned Pioneer slot shares go to Treasury
- service: 5 USDC

If an upline is INACTIVE, its own scheduled share is treasury-destined. If an ACTIVE/GRACE upline exists but has not unlocked that level, the fixed share is unallocated to Treasury. In neither case is the percentage compressed upward.

## Progressive weekly ACTIVE requirement

A wallet advances its qualification requirement only when it successfully starts a new ACTIVE week. Calendar inactivity alone never advances the ladder:

- ACTIVE weeks 1–2: **10 units**
- weeks 3–4: **20**
- weeks 5–6: **30**
- weeks 7–8: **40**
- week 9 onward: **50**, permanently capped

During an already ACTIVE week, additional purchases increase only `current_week_units` and monetization depth; they never prequalify the next week.

During GRACE/INACTIVE, purchases accumulate toward the next requirement inside one live seven-day qualification window. Stale partial progress resets. During a live INACTIVE partial-qualification window, **only buyer SELF is provisionally preserved** for batching equivalence. Network and Pioneer receive no partial-window exception.

## Weekly network depth

Each upline's own personal units in its current ACTIVE week determine how many fixed network levels that wallet may monetize:

- 10 → U1–U3
- 25 → U1–U4
- 50 → U1–U5
- 100 → U1–U6
- 200 → U1–U7
- 350 → U1–U8
- 500+ → U1–U9

Depth unlocking is prospective only. Previously locked levels are never recovered retroactively. GRACE retains the depth of the just-finished ACTIVE week while preserved rewards remain pending.

## Pioneer 2% pool — purchase-earned positions

- Absolute global cap: **100 positions**.
- Registration earns **zero** positions.
- One individual purchase creates `floor(units / 1000)` candidate positions.
- Purchases never accumulate toward the threshold: separate `500 + 500` purchases create zero positions.
- One wallet may own multiple or all positions.
- Assignment is capped by remaining global capacity.
- At 98/100, a 3,000-unit purchase receives exactly 2 positions.
- At 100/100, every later purchase creates zero new positions.
- Positions are permanent and never recycled.
- **Rule B:** positions created by a purchase begin earning from the next global purchase, never from the purchase that created them.
- Each position is one equal virtual share of the fixed 2%/100 pool.
- Unassigned slot shares go to Treasury and are not redistributed among existing Pioneers.
- Weighted high-precision checkpoints prevent retroactive entitlement when a wallet adds positions later.
- Pioneer positions remain owned through inactivity, but unclaimed Pioneer due expires once the wallet becomes INACTIVE. Reactivation resumes participation prospectively only.

## Gas / transaction-fee model

- Registration: registering wallet signs and pays SOL account/transaction costs.
- Purchase: buyer signs once and pays the Solana transaction fee; there is no per-purchase rent PDA.
- Reward accrual: beneficiaries do not sign merely to receive accounting credit.
- Claim: ACTIVE beneficiary signs and pays its own SOL fee.
- Expiry settlement: permissionless; the submitter pays the transaction fee.
- Purchase history is event-based through globally monotonic Unit IDs and purchase indices.

## Final production surface

Economic path:

`buyer -> purchase_and_distribute -> canonical stablecoin vault -> SELF/network/Pioneer/service accounting -> later claim or expiry settlement`

On-chain instructions:

- `initialize`
- `register`
- `purchase_and_distribute`
- `settle_expired`
- `claim`

The previous qualified-revenue / Revenue Adapter / Evidence / Qualification production architecture has been retired. Historical migration tooling is outside the production and audit boundary.

## Security properties

The program independently validates the transaction builder's inputs:

- user PDA is bound to the signer wallet;
- immutable sponsor/ancestry is reconstructed PDA-by-PDA;
- source token account must be owned by the buyer;
- mint must be a supported protocol mint;
- vault accounts must be canonical SPL Token ATAs owned by the vault-authority PDA;
- Treasury accounts must be canonical ATAs of the frozen service Treasury;
- claim destination must be the claimant's canonical ATA;
- economic arithmetic uses checked operations;
- failed transactions roll back all preceding state/token mutations atomically.

Runtime adversarial coverage includes false ancestry, wrong vault, unsupported mint, wrong source authority, Treasury substitution, registration spoofing, claim hijacking, non-canonical claim destination, double-claim/replay and a failed payment CPI after expiry settlement has already begun.

## Validation layers

Before Mainnet, the candidate is exercised in complementary environments:

1. deterministic reference economics and rank models;
2. Rust unit/property tests;
3. LiteSVM integration/adversarial tests with clock warping for ACTIVE/GRACE/INACTIVE boundaries;
4. isolated `solana-test-validator` comprehensive transactions with real SPL accounts, PDAs, signatures and deployment;
5. final public Solana Devnet comprehensive + security-adversarial run;
6. reproducible/verifiable production build with byte/hash comparison;
7. final production-runtime validation after Program ID/timestamp freeze;
8. independent external audit of the exact frozen release candidate.

No single layer is treated as proof by itself.

## Mainnet identities

- Service Treasury: `AepYo8xanmKuRiLVeYQuCTJoQr1nyKiTApoKwHMEg8fn`
- USDT: `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB`
- USDC: `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`

The currently declared Program ID remains a development identity. The final Mainnet Program ID keypair must be generated offline; only its public key belongs in source control.

## Mainnet release gate

Do **not** deploy/initialize or make the program immutable on Mainnet until all required gates are green on the appropriate frozen source:

1. final economics/activity/depth/Pioneer specification frozen;
2. exact-head protocol CI and RustSec green;
3. comprehensive isolated-runtime validation green;
4. final public Devnet comprehensive + security-adversarial evidence green;
5. final Mainnet Program ID generated offline and public key frozen in source/config;
6. future registration-open UTC timestamp frozen;
7. final production `.so` built reproducibly and SHA-256 verified;
8. final production-runtime validation bound to the final release SHA;
9. independent audit completed and all findings dispositioned;
10. `release/mainnet-release.json` complete and `scripts/pre-mainnet-gate.py` fully green;
11. deployed Mainnet bytecode verified against the audited artifact before initialization;
12. canonical USDT/USDC vault-authority and service-Treasury ATAs created if absent and verified;
13. deliberately small Mainnet smoke while upgrade authority is retained;
14. deployed bytecode/state reverified;
15. only then permanently remove upgrade authority.

## Solana time property

Time does not execute transactions automatically. Once a grace deadline passes, value is economically Treasury-destined, but physical token movement occurs on the next instruction that settles/touches the relevant state. No platform-funded transaction is required for every commission event.

## Rank / badge V1

Ranks are deterministic off-chain metadata and do not change the frozen payout core. See `RANK_BADGE_SPEC.md`.
