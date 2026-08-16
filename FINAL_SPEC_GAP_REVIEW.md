# Final Specification Resolution

Status: **economic specification resolved for hardening**. Mainnet still remains fail-closed pending exact-head CI, devnet smoke, final Program ID, audit and release gates.

## Frozen economics

- 1 USDT/USDC = 1 logical unit.
- Users may purchase unlimited units subject only to on-chain numeric/transaction limits.
- Unit purchase is the sole production economic event.
- Aggregate split is **50% SELF + 43% network + 2% Pioneer + 5% service**.
- SELF is always the buyer of the units.
- The network pool is paid across exactly nine immutable uplines, beginning with the buyer's direct sponsor.
- Network schedule: `15 / 9 / 6 / 4 / 2.5 / 2 / 1.5 / 1 / 2` = 43%.
- Activity threshold is 10 units; ACTIVE 7 days; GRACE 48 hours.
- Claim is ACTIVE-only and claimant-paid.
- GRACE temporarily preserves unclaimed value.
- Once INACTIVE, whole-atomic unclaimed SELF/network/Pioneer value is permanently treasury-destined; late reactivation cannot rescue it.
- The production release is a single core Solana program.

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
- SELF rewards, network/downline rewards, Pioneer rewards, claims, wallet balances and other receipts do not create positions.

### Pioneer Rule B

The purchase that creates a Pioneer position does **not** pay that new position from its own 2% Pioneer allocation.

The purchase first advances/distributes the current Pioneer pool using only positions that existed before the purchase. New positions are assigned afterward and enter at the then-current Pioneer index, so they begin earning from the **next global purchase**.

A wallet may acquire positions at multiple different times. Weighted reward debt/checkpoints preserve the correct entry index for every added position and prevent retroactive Pioneer rewards.

Each of the 100 positions represents one equal virtual share of the 2% pool. When fewer than 100 positions exist, the share corresponding to unassigned positions is treasury-destined; it is not redistributed among existing Pioneer holders.

## Resolved historical rule — self-reentry

The old concept of creating a separate genealogy position "under oneself" is **retired**.

The final model achieves the intended economic effect without genealogy-position multiplication:

- one `UserState` exists per wallet;
- the user's registered referrer remains immutable;
- every purchase treats the buyer as the **SELF economic level** and allocates 50% to that buyer subject to activity/expiry rules;
- the immutable sponsor is the first network upline and receives 15% subject to activity/expiry rules;
- repeated purchases are additional units of the same user, not synthetic self-sponsored genealogy positions.

A wallet may nevertheless own multiple **Pioneer pool positions** under the separate purchase-earned Pioneer rule above; those positions affect only the 2% Pioneer pool and do not create additional genealogy nodes or network depths.

## Resolved historical rule — inactive compression

**IC-A is frozen.** The protocol uses fixed-depth percentages plus treasury expiry, not dynamic compression.

For each of the nine network-upline slots:

- ACTIVE: that slot's amount is claimable;
- GRACE: that slot's amount is pending/preserved for timely requalification;
- INACTIVE: that slot's amount is permanently treasury-destined;
- higher ancestors keep only their own fixed percentages;
- the inactive user's percentage is not reassigned to another ancestor.

The same final inactivity rule applies to the buyer's SELF reward and any Pioneer entitlement already owned by that wallet.

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
