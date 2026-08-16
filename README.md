# Service Referral Protocol — Solana V0.13

Public development repository for an ownerless Solana/Anchor protocol foundation.

## Confirmed economic model

- Solana + Anchor/Rust.
- USDT and USDC are separate accounting rails.
- **1 USDT/USDC = 1 globally unique logical unit.** Users may purchase any number of units, subject only to SPL-token numeric limits per transaction/batch.
- Every unit purchase is itself the sole production economic event that funds and creates referral accounting.
- Referral relationships are immutable after registration.
- The direct sponsor is genealogical **Level 1** and receives the **50% direct reward only**; it does not also receive a referral-depth percentage.
- Activity threshold: 10 units; ACTIVE 7 days; GRACE 48 hours.
- ACTIVE users may claim accrued rewards.
- During GRACE, unclaimed value is temporarily preserved; reactivation within GRACE preserves that value.
- Once INACTIVE, all whole-atomic unclaimed direct, network and Pioneer value becomes permanently treasury-destined. Late reactivation cannot rescue it.
- First 100 real registrations receive non-transferable Pioneer IDs.
- Claims are pull-based: users sign the claim and pay their own SOL transaction fee.

## Gas / transaction-fee model

The protocol does not send one transaction to every reward recipient.

- Registration: the registering wallet signs and pays SOL transaction/account-creation costs.
- Unit purchase: the buyer signs once and pays the Solana transaction fee. The same transaction transfers USDT/USDC into the protocol vault, creates the unit batch, updates activity and accounts for 50% direct + 43% network + 2% Pioneer + 5% service.
- Reward accrual: sponsor/uplines/Pioneers do not sign and pay no gas merely because an accrual is recorded for them.
- Claim: the ACTIVE beneficiary signs a separate claim transaction and pays its own SOL fee. Multiple accruals can be accumulated and withdrawn with one claim.
- Expiration settlement: once a user is INACTIVE, treasury-destined unclaimed value can be physically settled permissionlessly; whoever submits that settlement transaction pays its SOL fee. No permanent platform keeper is required.

## Frozen percentages

For each purchase amount:

- **Level 1 / direct sponsor: 50%**
- **Levels 2–10 network pool: 43%**
- **Pioneer pool: 2%**
- **Service/platform: 5%**

The final Levels 2–10 schedule is:

- L2: 15%
- L3: 9%
- L4: 6%
- L5: 4%
- L6: 2.5%
- L7: 2%
- L8: 1.5%
- L9: 1%
- L10: 2%

The nine network weights sum to exactly 43%. This is the minimal-delta remapping of the historical ten-weight table: the two deepest historical 1% buckets are consolidated into L10. There is no L11 economic level and L1 is never paid twice.

The complete allocation is exactly **50 + 43 + 2 + 5 = 100%**, before deterministic integer-rounding handling.

## Final production surface

The main economic path is:

`buyer -> purchase_and_distribute -> canonical stablecoin vault -> unit/activity + 50/43/2/5 accounting -> later claim or expiry settlement`

The on-chain instruction surface is limited to:

- `initialize`
- `register`
- `purchase_and_distribute`
- `settle_expired`
- `claim`

The previous qualified-revenue, Evidence, Qualification and Revenue Adapter programs/instructions have been removed from the final branch. They are not mainnet dependencies and are not part of the final audit/deployment surface.

## Mainnet identities under review

- Service treasury: `AepYo8xanmKuRiLVeYQuCTJoQr1nyKiTApoKwHMEg8fn`
- USDT: `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB`
- USDC: `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`

The currently declared Program ID is still a development identity. The final Program ID keypair must be generated offline; only its public key belongs in source control.

## Mainnet gate

Do **not** make the program immutable on mainnet until all of these pass:

1. Final core production build on the pinned toolchain.
2. Reference, Rust, LiteSVM integration and adversarial tests all green on the exact release commit.
3. Devnet smoke using the exact final instruction surface.
4. Independent audit of the exact final core and client transaction construction.
5. Public reproducible/verifiable build and SHA-256 of the final `.so`.
6. Final Program ID generated offline and frozen consistently in source/configuration.
7. Future registration-opening UTC frozen.
8. `release/mainnet-release.json` completed and the executable pre-mainnet gate fully green.
9. Controlled mainnet deployment and small smoke while upgrade authority is still retained.
10. Deployed bytecode verified against the audited artifact.
11. Only then permanently remove program upgrade authority.

## Important Solana property

Time does not execute transactions by itself. After the grace timestamp, unclaimed value is economically treasury-destined, but physical token movement occurs on the next instruction that settles or touches the relevant state. This does not require a platform-funded transaction per commission event.
