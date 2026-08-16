# Final Specification Resolution

Status: **economic specification resolved for hardening**. Mainnet still remains fail-closed pending CI, devnet smoke, final Program ID, audit and release gates.

## Frozen economics

- 1 USDT/USDC = 1 logical unit.
- Users may purchase unlimited units subject only to on-chain numeric/transaction limits.
- Unit purchase is the sole production economic event.
- Aggregate split is **50% SELF + 43% network + 2% Pioneer + 5% service**.
- SELF is always the buyer of the units.
- The network pool is paid across exactly nine immutable uplines, beginning with the buyer's direct sponsor.
- Network schedule: `15 / 9 / 6 / 4 / 2.5 / 2 / 1.5 / 1 / 2` = 43%.
- First 100 real registrations receive Pioneer IDs.
- Activity threshold is 10 units; ACTIVE 7 days; GRACE 48 hours.
- Claim is ACTIVE-only and claimant-paid.
- GRACE temporarily preserves unclaimed value.
- Once INACTIVE, whole-atomic unclaimed SELF/network/Pioneer value is permanently treasury-destined; late reactivation cannot rescue it.
- The production release is a single core Solana program.

## Resolved historical rule — self-reentry

The old concept of creating a separate genealogy position "under oneself" is **retired**.

The final model achieves the intended economic effect without position multiplication:

- one `UserState` exists per wallet;
- the user's registered referrer remains immutable;
- every purchase treats the buyer as the **SELF economic level** and allocates 50% to that buyer subject to activity/expiry rules;
- the immutable sponsor is the first network upline and receives 15% subject to activity/expiry rules;
- repeated purchases are additional units of the same user, not synthetic self-sponsored positions.

This removes the need for sibling/chained re-entry positions and prevents one wallet from occupying multiple genealogy depths merely by buying repeatedly.

## Resolved historical rule — inactive compression

**IC-A is frozen.** The protocol uses fixed-depth percentages plus treasury expiry, not dynamic compression.

For each of the nine network-upline slots:

- ACTIVE: that slot's amount is claimable;
- GRACE: that slot's amount is pending/preserved for timely requalification;
- INACTIVE: that slot's amount is permanently treasury-destined;
- higher ancestors keep only their own fixed percentages;
- the inactive user's percentage is not reassigned to another ancestor.

The same final inactivity rule applies to the buyer's SELF reward and Pioneer entitlement.

## Economic-level terminology

For product/UI purposes the system can be described as **10 economic levels including the buyer**.

For smart-contract, audit and technical documentation use the unambiguous terminology:

**SELF + 9 uplines**

This avoids confusing the buyer's 50% SELF bucket with the direct sponsor's 15% network slot.

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
