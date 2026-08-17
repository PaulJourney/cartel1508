# Service Referral Protocol — Solana V0.16

Public development repository for an ownerless Solana/Anchor protocol foundation.

## Frozen economic model

- Solana + Anchor/Rust.
- USDT and USDC are separate accounting rails.
- **1 USDT/USDC = 1 globally unique logical unit.** Users may purchase any number of units, subject only to SPL-token numeric/transaction limits.
- Every unit purchase is the sole production economic event that funds and creates referral accounting.
- Referral relationships are immutable after registration.
- Every purchase treats the **buyer as SELF**, the first economic beneficiary, receiving the **50% SELF reward**.
- The buyer's immutable direct sponsor is the **first network upline** and receives the first 43%-pool slot: **15%**.
- Eight additional ancestors complete a total of **9 network uplines**.
- There is no separate self-reentry genealogy position: repeated purchases are additional units of the same user, and SELF is already economically represented by the 50% bucket on every purchase.
- Activity threshold: 10 units; ACTIVE 7 days; GRACE 48 hours.
- ACTIVE users may claim accrued rewards.
- During GRACE, unclaimed value is temporarily preserved; reactivation within GRACE preserves/vests it.
- **IC-A is frozen:** once INACTIVE, that user's scheduled/unclaimed SELF, network and Pioneer value becomes permanently treasury-destined. Higher uplines keep only their own fixed percentages; there is no dynamic compression.
- Pioneer positions are **purchase-earned**, not registration-earned; registration alone receives zero positions.
- Claims are pull-based: users sign the claim and pay their own SOL transaction fee.

## Frozen allocation

For each purchase amount:

- **SELF / buyer: 50%**
- **9 network uplines: 43%**
- **Pioneer pool: 2%**
- **Service/platform: 5%**

The nine network-upline weights are:

- Upline 1 — direct sponsor: 15%
- Upline 2: 9%
- Upline 3: 6%
- Upline 4: 4%
- Upline 5: 2.5%
- Upline 6: 2%
- Upline 7: 1.5%
- Upline 8: 1%
- Upline 9: 2%

The network weights sum to exactly 43%, so the complete allocation is exactly **50 + 43 + 2 + 5 = 100%** before deterministic integer-rounding handling.

A useful commercial representation is **10 economic levels including SELF**. Internally and in audit documentation the safer terminology is **SELF + 9 uplines**, which avoids confusing the buyer with the sponsor. Pioneer positions are a separate global pool and are not genealogy levels.

### Example — 100 USDC purchase

If Mario buys 100 USDC of units and is ACTIVE after the purchase:

- Mario / SELF: 50 USDC
- Mario's sponsor: 15 USDC
- next uplines: 9, 6, 4, 2.5, 2, 1.5, 1 and 2 USDC
- Pioneer pool: 2 USDC, split across Pioneer positions that already existed before this purchase; shares belonging to unassigned positions go to treasury
- service: 5 USDC

If any specific upline is INACTIVE, **IC-A** sends only that upline's scheduled share to treasury; the next active ancestor does not inherit that percentage.

## Pioneer 2% pool — purchase-earned positions

- The pool has an absolute maximum of **100 positions**.
- Registration alone earns **zero** positions.
- A single purchase earns `floor(units / 1000)` candidate positions.
- Purchases below 1,000 never accumulate across transactions: `500 + 500` in two purchases creates zero positions.
- One wallet may own multiple positions.
- Assignment is `min(candidate_positions, 100 - positions_already_assigned)`.
- At 98/100, a 3,000-unit purchase receives exactly 2 positions.
- At 100/100 the pool is permanently saturated and no later purchase can create another position, regardless of size.
- **Rule B:** positions created by a purchase begin earning only from the next global purchase; they never earn from the transaction that created them.
- Network/SELF/Pioneer earnings, claims and wallet balances never create Pioneer positions; only the gross units of the individual buyer-signed purchase do.
- Each position is one equal share of the fixed 2%/100 pool. Unassigned shares go to treasury and are never redistributed among existing Pioneer holders.
- Weighted checkpoints allow the same wallet to acquire positions at different times without receiving retroactive Pioneer value.

## Activity and expiry

The purchase updates the buyer's activity before the new SELF reward is evaluated:

- a purchase that brings the buyer to the 10-unit qualification threshold makes the buyer ACTIVE and allows the new 50% SELF reward to remain claimable;
- a buyer still in GRACE retains the SELF reward temporarily and must reactivate before claiming;
- a buyer that remains INACTIVE after the purchase has the new SELF reward treasury-destined under IC-A;
- stale unclaimed value is settled before late reactivation, so value whose grace period already ended cannot be rescued;
- during a live partial qualification window, the buyer's SELF and any already-owned Pioneer entitlement are provisional rather than immediately expired. Reaching 10 units inside the window makes `10×1` purchases economically equivalent to one 10-unit purchase for those own-user buckets; failure to qualify before the window closes sends the provisional value to treasury on settlement. Network-upline rewards remain governed by IC-A throughout;
- activity-unit accumulation does **not** imply Pioneer accumulation: Pioneer positions still require a single purchase of at least 1,000 units.

The same ACTIVE / GRACE / INACTIVE logic applies to fixed network-upline shares. Pioneer entitlement is also subject to final inactivity expiry.

## Gas / transaction-fee model

The protocol does not send one transaction to every reward recipient.

- Registration: the registering wallet signs and pays SOL transaction/account-creation costs.
- Unit purchase: the buyer signs once and pays the Solana transaction fee. The same transaction transfers USDT/USDC into the protocol vault, updates activity and performs 50/43/2/5 accounting. It creates no per-purchase rent account.
- Reward accrual: sponsor/uplines/Pioneers do not sign and pay no gas merely because accounting credit is created.
- Claim: an ACTIVE beneficiary signs a separate claim transaction and pays its own SOL fee. Multiple accruals can be accumulated and withdrawn together.
- Expiration settlement: treasury-destined value can be physically settled permissionlessly; whoever submits that transaction pays its SOL fee. No permanent platform keeper is required.
- Purchase history: global logical Unit IDs, purchase index, mint, unit count, Pioneer positions added/total and timestamp are emitted in the `UnitsPurchased` event. No `UnitBatch` PDA is created, so repeated/unlimited purchases do not accumulate per-purchase account rent.

## Final production surface

The main economic path is:

`buyer -> purchase_and_distribute -> canonical stablecoin vault -> activity + SELF/network/Pioneer/service accounting -> optional Pioneer-position assignment -> later claim or expiry settlement`

The on-chain instruction surface is limited to:

- `initialize`
- `register`
- `purchase_and_distribute`
- `settle_expired`
- `claim`

The previous qualified-revenue, Evidence, Qualification and Revenue Adapter programs/instructions have been removed from the final production surface.

## Mainnet identities under review

- Service treasury: `AepYo8xanmKuRiLVeYQuCTJoQr1nyKiTApoKwHMEg8fn`
- USDT: `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB`
- USDC: `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`

The currently declared Program ID is still a development identity. The final Program ID keypair must be generated offline; only its public key belongs in source control.

## Mainnet gate

Do **not** make the program immutable on mainnet until all of these pass. CI evidence must be generated on the exact current release-candidate commit; a workflow that requires approval or never creates jobs is not considered passing evidence.

1. Final core production build on the pinned toolchain.
2. Reference, Rust, LiteSVM integration and adversarial tests all green on the exact release commit.
3. Devnet smoke using the exact final instruction surface, SELF + 9-upline economics and purchase-earned Pioneer rules.
4. Independent audit of the exact final core and client transaction construction.
5. Public reproducible/verifiable build and SHA-256 of the final `.so`.
6. Final Program ID generated offline and frozen consistently in source/configuration.
7. Future registration-opening UTC frozen.
8. `release/mainnet-release.json` completed and the executable pre-mainnet gate fully green.
9. Controlled mainnet deployment and deliberately small smoke while upgrade authority is retained.
10. Deployed bytecode verified against the audited artifact.
11. Only then permanently remove program upgrade authority.

## Important Solana property

Time does not execute transactions by itself. After the grace timestamp, unclaimed value is economically treasury-destined, but physical token movement occurs on the next instruction that settles or touches the relevant state. This does not require a platform-funded transaction per commission event.

## Progressive weekly activity and monetization depth

The core now separates being ACTIVE from how deep an upline can monetize. Successful
ACTIVE weeks require 10/10/20/20/30/30/40/40/50 units, capped at 50 from week 9. Only
successfully-started ACTIVE weeks advance the requirement. During an ACTIVE week,
personal purchases accumulate toward depth: 10=>U3, 25=>U4, 50=>U5, 100=>U6,
200=>U7, 350=>U8, 500=>U9. Unlocking is prospective only; previously unqualified
levels are never recovered or compressed and route to Treasury. Pioneer remains a
separate 1,000-units-per-single-purchase rule and is strictly ACTIVE/GRACE-gated.
Ranks/badges are deterministic indexer-layer metadata; see `RANK_BADGE_SPEC.md`.
