# Security model — V0.8

This is pre-audit software and must not receive production funds.

## Core invariants

1. Unit purchases and referral reward creation are separate instructions and separate funding paths.
2. `record_qualified_revenue` creates liabilities only after the stablecoin transfer to the protocol vault succeeds.
3. Referral is write-once.
4. The technical root receives no economic reward.
5. The reward engine inspects exactly ten immutable ancestry accounts.
6. Unsupported token mints are rejected.
7. Claims are pull-based and require the beneficiary wallet signature.
8. ACTIVE/GRACE/INACTIVE is derived from Solana Clock timestamps; there is no off-chain timer authority.
9. Pioneer status is non-transferable and has no retroactive accrual before assignment.
10. Final deployment must remove upgrade authority only after audit and verified build.

## Remaining hardening before audit

- Replace ownership-only treasury token checks with exact canonical ATA checks.
- Add physical sweep of previously pending expired balances during user/source touch so accounting and token balances move together.
- Add full integration tests for PDA ancestry mutation/serialization.
- Add exact vault ATA checks and canonical SPL Token Program validation.
- Freeze production initializer, qualified revenue source, launch timestamp and Program ID in a production build profile.
- Fuzz account-substitution and duplicate-account attacks.
