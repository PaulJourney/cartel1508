# Final Specification Resolution

Status: **economic specification resolved for hardening**. Mainnet still remains fail-closed pending exact-head CI, devnet smoke, final Program ID, audit and release gates.

## Frozen economics

- 1 USDT/USDC = 1 logical unit.
- Users may purchase unlimited units subject only to on-chain numeric/transaction limits.
- Unit purchase is the sole production economic event.
- Aggregate split is **50% SELF + 43% network + 2% Pioneer + 5% service**.
- SELF is always the buyer of the units.
- The network pool is scheduled across exactly nine immutable uplines, beginning with the buyer's direct sponsor.
- Network schedule: `15 / 9 / 6 / 4 / 2.5 / 2 / 1.5 / 1 / 2` = 43%.
- ACTIVE lasts 7 days and GRACE lasts 48 hours.
- Claim is ACTIVE-only and claimant-paid.
- GRACE temporarily preserves eligible unclaimed value.
- Once INACTIVE, whole-atomic unclaimed SELF/network/Pioneer value is permanently treasury-destined under the rules below; late reactivation cannot recover expired value.
- The production release is a single core Solana program.

## Frozen progressive ACTIVE-week rule

The former permanent `10 units = ACTIVE` rule is retired. The requirement now progresses only when a wallet successfully starts an ACTIVE week:

- ACTIVE weeks 1–2: **10 units** each;
- ACTIVE weeks 3–4: **20 units** each;
- ACTIVE weeks 5–6: **30 units** each;
- ACTIVE weeks 7–8: **40 units** each;
- ACTIVE week 9 onward: **50 units**, permanently capped at 50.

The counter is based on successfully-started ACTIVE weeks, **not calendar time**. An inactive wallet that returns months later resumes from the same next-week requirement it had when it stopped.

A successful activation starts a personal seven-day ACTIVE cycle. Purchases made while already ACTIVE increase that cycle's personal-unit total and monetization depth only; they never pre-qualify a later week. During GRACE/INACTIVE, purchases accumulate toward the next ACTIVE-week requirement within one live seven-day qualification window. If that partial window expires before the requirement is reached, stale progress is discarded on the next touch.

During an INACTIVE partial qualification window, only the buyer's own SELF amount is provisionally preserved. Pioneer and network value receive no inactive partial-qualification exception.

## Frozen weekly network-depth rule

The 43% schedule remains fixed, but an upline monetizes only levels unlocked by that wallet's **personal units in its current ACTIVE week**:

- 10 units => **U1–U3**;
- 25 units => **U1–U4**;
- 50 units => **U1–U5**;
- 100 units => **U1–U6**;
- 200 units => **U1–U7**;
- 350 units => **U1–U8**;
- 500+ units => **U1–U9**.

Depth unlock is prospective only. Increasing personal units later in the same ACTIVE cycle never recovers U-level amounts from earlier purchases. When an ACTIVE/GRACE upline exists genealogically but the scheduled level is beyond that wallet's unlocked depth, that fixed share is treasury/unallocated. It is never compressed upward or reassigned.

GRACE retains the depth of the just-finished ACTIVE cycle while eligible network amounts remain pending. When the next ACTIVE week starts, `current_week_units` is replaced by the units that qualified the new week and depth is recalculated from that new total.

## Frozen Pioneer rule — 100 purchase-earned positions

The former rule assigning one Pioneer ID to each of the first 100 registrations is **retired**.

The final rule is:

- there are exactly **100 Pioneer positions maximum**;
- registration alone creates **zero** Pioneer positions;
- positions are created only by the gross units of a single buyer-signed `purchase_and_distribute` transaction;
- `candidate_positions = floor(units / 1000)`;
- purchases from separate transactions never accumulate toward the 1,000-unit threshold;
- one wallet may own multiple Pioneer positions;
- `positions_added = min(candidate_positions, 100 - positions_already_assigned)`;
- at 98/100, a 3,000-unit purchase creates exactly 2 positions, not 3;
- at 100/100, every later purchase creates exactly 0 additional positions regardless of size;
- SELF rewards, network/downline rewards, Pioneer rewards, claims, wallet balances and other receipts do not automatically create positions; only gross units in a buyer-signed purchase qualify.

### Pioneer Rule B

The purchase that creates a Pioneer position does **not** pay that new position from its own 2% Pioneer allocation.

The purchase first advances/distributes the current Pioneer pool using only positions that existed before the purchase. New positions are assigned afterward and enter at the then-current Pioneer index, so they begin earning from the **next global purchase**.

A wallet may acquire positions at multiple different times. Weighted reward debt/checkpoints preserve the correct entry index for every added position and prevent retroactive Pioneer rewards.

Each of the 100 positions represents one equal virtual share of the 2% pool. When fewer than 100 positions exist, the share corresponding to unassigned positions is treasury-destined; it is not redistributed among existing Pioneer holders.

### Pioneer ACTIVE requirement

Pioneer ownership is permanent but Pioneer economics are activity-gated:

- ACTIVE: the wallet's owned Pioneer positions participate and due is claimable under the normal pull-claim path;
- GRACE: already-eligible Pioneer entitlement remains preserved for timely reactivation;
- INACTIVE: unclaimed Pioneer due is treasury-destined when settled/touched;
- inactivity never deletes, transfers or recycles the underlying Pioneer positions;
- reactivation cannot recover Pioneer value that expired while INACTIVE;
- after reactivation, the permanent positions participate prospectively again.

If an INACTIVE Pioneer wallet makes a purchase that itself successfully reactivates the wallet, stale Pioneer due is settled first; the wallet is then ACTIVE before the current purchase's Pioneer accrual, so its pre-existing permanent positions may participate prospectively in that reactivation purchase. New positions created by that same purchase still obey Rule B.

## Rank and badge layer — non-payout V1

Rank is intentionally outside the payout core. `UserRegistered` and `UnitsPurchased` events expose enough deterministic data for an indexer to reconstruct immutable direct-referral legs, rolling purchase volume and qualification state.

V1 rank rules are defined in `RANK_BADGE_SPEC.md`:

- Current Rank is based on rolling 30-day Qualified Network Volume (QNV), qualified legs and balance rules;
- personal purchases are excluded from the user's own QNV rank calculation;
- claimed earnings are excluded from rank;
- Current Rank may rise or fall;
- Highest Lifetime Rank is permanent historical status;
- V1 rank/badges **do not modify the frozen 50/43/2/5 payout percentages**;
- Pioneer is a separate permanent badge displayed as `PIONEER ×N`, not a rank rung.

## Resolved historical rule — self-reentry

The old concept of creating a separate genealogy position "under oneself" is **retired**.

The final model achieves the intended economic effect without genealogy-position multiplication:

- one `UserState` exists per wallet;
- the user's registered referrer remains immutable;
- every purchase treats the buyer as the **SELF economic level** and allocates 50% to that buyer subject to activity/expiry rules;
- the immutable sponsor is the first network upline and receives its fixed 15% only when the sponsor is activity/depth eligible;
- repeated purchases are additional units of the same user, not synthetic self-sponsored genealogy positions.

A wallet may nevertheless own multiple **Pioneer pool positions** under the separate purchase-earned Pioneer rule above; those positions affect only the 2% Pioneer pool and do not create additional genealogy nodes or network depths.

## Resolved historical rule — inactive compression

**IC-A is frozen.** The protocol uses fixed-depth percentages plus treasury routing/expiry, not dynamic compression.

For each of the nine network-upline slots:

- ACTIVE + sufficient weekly depth: that slot's amount is claimable;
- GRACE + sufficient retained weekly depth: that slot's amount is pending/preserved for timely requalification;
- ACTIVE/GRACE but insufficient depth: that slot's amount is treasury/unallocated;
- INACTIVE: that slot's amount is permanently treasury-destined/expired;
- higher ancestors keep only their own fixed percentages;
- a locked or inactive user's percentage is not reassigned to another ancestor.

## Economic-level terminology

For product/UI purposes the system can be described as **10 economic levels including the buyer**.

For smart-contract, audit and technical documentation use the unambiguous terminology:

**SELF + 9 uplines**

This avoids confusing the buyer's 50% SELF bucket with the direct sponsor's 15% network slot. Pioneer positions are a separate global 2% pool and are not genealogy levels.

## Remaining non-economic mainnet gates

The economics above no longer block the specification freeze. Mainnet remains blocked until:

1. exact final core CI is green;
2. final devnet transaction smoke is green;
3. final Program ID is generated offline and frozen;
4. a future registration-open UTC timestamp is frozen;
5. reproducible/verifiable final artifact and SHA-256 are produced;
6. independent audit is completed and findings are dispositioned;
7. the executable pre-mainnet release manifest/gate is fully green;
8. a deliberately small mainnet smoke succeeds with upgrade authority retained;
9. deployed bytecode is verified against the audited artifact;
10. upgrade authority is permanently removed only after all previous gates pass.
