# Service Referral Protocol — Solana V0.8

Public development repository for an ownerless Solana/Anchor protocol foundation.

## Scope implemented in this milestone

- Solana + Anchor/Rust.
- USDT and USDC supported as separate accounting rails.
- 1 USDT/USDC = 1 logical service unit; huge purchases use one batch PDA with a local unit-index range.
- Immutable referral relationship, maximum 10 economic levels.
- Activity threshold: 10 units; ACTIVE 7 days; GRACE 48 hours.
- Network rewards: ACTIVE -> claimable, GRACE -> pending, INACTIVE -> treasury allocation.
- Direct attributed rewards and Pioneer entitlement persist but can only be claimed while ACTIVE.
- First 100 real registrations receive non-transferable Pioneer IDs.
- Pull-based claims: users sign the claim and pay their own SOL transaction fee.
- Service-unit purchases **do not** create referral rewards.
- Referral liabilities can only be created by `record_qualified_revenue` after stablecoins are first transferred into the PDA vault by the frozen qualified-revenue source.

## Frozen economic percentages under test

Qualified service revenue accounting only:

- 50% direct attributed reward
- 43% referral depth: `15 / 9 / 6 / 4 / 2.5 / 2 / 1.5 / 1 / 1 / 1`
- 2% Pioneer allocation
- 5% service allocation

The purchase of service units used for activity qualification is deliberately isolated from this reward engine.

## Mainnet identities under review

- Service treasury: `AepYo8xanmKuRiLVeYQuCTJoQr1nyKiTApoKwHMEg8fn`
- USDT: `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB`
- USDC: `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`

The currently declared Program ID is a development identity and must not be treated as mainnet-final until a controlled program keypair is preserved outside the public repository and the verified build passes.

## Mainnet gate

Do **not** deploy immutable mainnet until all of these pass:

1. Anchor build on pinned toolchain.
2. Rust/reference tests and integration tests.
3. Devnet tests with mock and canonical-compatible SPL token accounts.
4. Independent audit.
5. Public reproducible/verified build.
6. Final qualified-revenue source program/PDA frozen and independently reviewed.
7. Registration opening UTC frozen.
8. Small mainnet smoke test while upgrade authority still exists.
9. Only then remove program upgrade authority permanently.

## Important Solana property

Time does not execute transactions by itself. A reward can become economically expired after the grace timestamp, but token movement to the treasury occurs on the next instruction that settles/touches that state. No platform keeper is required.
