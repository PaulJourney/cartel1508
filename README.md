# Service Referral Protocol — Solana V0.11

Public development repository for an ownerless Solana/Anchor protocol foundation.

## Confirmed economic model

- Solana + Anchor/Rust.
- USDT and USDC are separate accounting rails.
- **1 USDT/USDC = 1 globally unique logical unit.** Users may purchase any number of units, subject only to SPL-token numeric limits per transaction/batch.
- Every unit purchase is itself the economic event that funds and creates the referral accounting. There is no second revenue event in the production model.
- Immutable referral relationship.
- The direct sponsor is genealogical **Level 1** and receives the **50% direct reward only**; it does not also receive a referral-depth percentage.
- Activity threshold: 10 units; ACTIVE 7 days; GRACE 48 hours.
- Network rewards: ACTIVE -> claimable, GRACE -> pending, INACTIVE -> treasury allocation.
- Direct attributed rewards and Pioneer entitlement persist but can only be claimed while ACTIVE.
- First 100 real registrations receive non-transferable Pioneer IDs.
- Pull-based claims: users sign the claim and pay their own SOL transaction fee.

## Gas / transaction-fee model

The protocol does not send one transaction to every reward recipient.

- Registration: the registering wallet signs and pays SOL transaction/account-creation costs.
- Unit purchase: the buyer signs once and pays the Solana transaction fee. The same transaction transfers USDT/USDC into the protocol vault, creates the unit batch, updates activity and accounts for 50% direct + 43% network + 2% Pioneer + 5% service.
- Reward accrual: sponsor/uplines/Pioneers do not sign and pay no gas merely because an accrual was recorded for them.
- Claim: the beneficiary signs a separate claim transaction and pays its own SOL fee. Multiple accruals can therefore be accumulated and withdrawn with one claim.
- Expired pending amounts can be settled permissionlessly; no permanent platform keeper is required.

## Frozen aggregate percentages under test

For each purchase amount:

- 50% direct reward to the buyer's registered sponsor (Level 1)
- 43% referral-depth pool
- 2% Pioneer allocation
- 5% service allocation

These four buckets conserve exactly 100% before integer rounding handling.

### Referral-depth numbering freeze still required

The historical codebase contains ten referral-depth weights totaling 43%:

`15 / 9 / 6 / 4 / 2.5 / 2 / 1.5 / 1 / 1 / 1`

The business rule is now explicit that the sponsor is genealogical Level 1 and receives only the 50% direct reward. A ten-level genealogy therefore has only Levels 2–10 available for the 43% pool. The exact nine-weight remapping of the 43% must be frozen before mainnet; the implementation must not silently create an eleventh genealogical level or double-pay the sponsor.

## Production path

The final production entrypoint is `purchase_and_distribute`:

`buyer -> stablecoin vault -> unit/activity update + 50/43/2/5 accounting -> later pull claims`

The prior `purchase_service_units` and `record_qualified_revenue` paths are retained temporarily for development/regression comparison but are explicitly disabled when the core program is compiled with the `production` feature. The old Evidence/Qualification/Adapter chain is therefore not part of the intended final production economics and must not be treated as a mainnet dependency.

## Mainnet identities under review

- Service treasury: `AepYo8xanmKuRiLVeYQuCTJoQr1nyKiTApoKwHMEg8fn`
- USDT: `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB`
- USDC: `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`

The currently declared Program ID is a development identity and must not be treated as mainnet-final until a controlled program keypair is preserved outside the public repository and the verified build passes.

## Mainnet gate

Do **not** deploy immutable mainnet until all of these pass:

1. Final Levels 2–10 distribution weights are frozen and sum to exactly 43%.
2. Anchor production build on the pinned toolchain.
3. Rust/reference/integration/adversarial tests for purchase-triggered economics and pull claims.
4. Devnet tests with canonical-compatible SPL token accounts.
5. Independent audit of the final production core and client transaction construction.
6. Public reproducible/verified build.
7. Registration opening UTC frozen.
8. Final Program ID generated and preserved outside the public repository.
9. Small mainnet smoke test while upgrade authority still exists.
10. Only after bytecode verification and smoke success, remove program upgrade authority permanently.

## Important Solana property

Time does not execute transactions by itself. A reward can become economically expired after the grace timestamp, but token movement to the treasury occurs on the next instruction that settles/touches that state. No platform-funded transaction is required merely for time to pass.
