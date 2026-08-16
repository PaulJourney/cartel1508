# Final Specification Gap Review

Status: **blocking final mainnet freeze** until the two historical rules below are reconciled with the current core.

This document exists to prevent a technically green build from being mistaken for a fully frozen economic specification.

## Already implemented and considered frozen

- 1 USDT/USDC = 1 unit.
- Unit quantities are not capped by product policy; only on-chain numeric/transaction constraints apply.
- Unit purchase is the sole production economic event.
- Aggregate split is 50% direct + 43% network + 2% Pioneer + 5% service.
- Standard genealogy is capped at ten economic levels.
- Standard L1 is the immutable direct sponsor and receives 50% direct only.
- Standard L2-L10 schedule is `15 / 9 / 6 / 4 / 2.5 / 2 / 1.5 / 1 / 2`.
- First 100 real registrations receive Pioneer IDs.
- Activity threshold is 10 units; ACTIVE 7 days; GRACE 48 hours.
- Claim is ACTIVE-only and claimant-paid.
- GRACE temporarily preserves unclaimed value.
- Once INACTIVE, unclaimed whole-atomic direct/network/Pioneer value is permanently treasury-destined; late reactivation cannot rescue it.
- Registration, purchase and claim gas are paid by the wallet initiating that transaction; reward accrual itself does not require beneficiary gas.
- The final intended release is a single core Solana program; the old Evidence/Qualification/Adapter architecture has been removed.

## Blocking historical rule 1 — self-reentry

Earlier product discussions explicitly allowed a user to **re-enter under themselves** while preserving the original referral relationship, with a discussed target of approximately `0.5 stablecoin direct per re-entry unit` under the 50% direct model.

The current core is wallet-based, not position-based:

- one `UserState` exists per wallet;
- its `referrer` is immutable;
- every current purchase treats that immutable referrer as L1;
- additional units do not create a second genealogy position under the same wallet.

Therefore the current core implements **unlimited additional units**, but it does **not** implement a distinct self-reentry position.

Before mainnet, the final specification must explicitly choose one of these economic meanings:

### SR-A — no separate position reentry

Repeated purchases are simply additional units on the same immutable user. The original sponsor remains L1 for every purchase and receives the 50% direct reward whenever eligible.

### SR-B — self-reentry is an economic position

A reentry unit/position is placed under the user's own main position. The user can therefore become L1 of the reentry and receive the 50% direct component on that reentry, while the original sponsor/referral ancestry remains above the user's main position.

If SR-B is required, the exact treatment of multiple reentries must also be frozen: whether every reentry is a sibling directly under the main position or whether reentries chain under prior reentries. These two models produce materially different multi-level payouts and cannot be inferred safely from the current wallet-only state.

**Current implementation corresponds to SR-A.** Do not freeze mainnet until that is confirmed or replaced.

## Blocking historical rule 2 — inactive “compression”

Earlier product notes also used the term **compression of inactive users**. The current core implements fixed genealogical depth with treasury expiry:

- each L2-L10 slot has a fixed percentage;
- if that specific upline is INACTIVE, its scheduled amount becomes expired/treasury-destined;
- higher ancestors still receive only their own fixed level percentages;
- the inactive user's percentage is not reassigned to the next ACTIVE ancestor.

That is **not classic dynamic compression**.

Before mainnet, the specification must explicitly freeze one of these meanings:

### IC-A — fixed depth + treasury expiry

Inactive user's own scheduled commission goes to service treasury. Higher active uplines keep only their normal fixed percentages. This matches the current core and the separate historical rule that unclaimed/inactive value goes to service.

### IC-B — dynamic compression

Inactive users are skipped for payout purposes, so the next ACTIVE ancestor takes the compressed level slot/percentage. This materially changes ancestry traversal, account requirements and economics and would require a new implementation and adversarial test suite.

**Current implementation corresponds to IC-A.** Do not freeze mainnet until that is confirmed or replaced.

## Engineering rule

No final Program ID, immutable production artifact, independent final audit or mainnet deployment should be treated as final until SR-A/SR-B and IC-A/IC-B are explicitly resolved. Everything else may continue to be tested and hardened in parallel.
