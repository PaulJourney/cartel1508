# Security model — V0.10

This is pre-audit software and is not yet approved for production deployment.

## Implemented and tested

- Service-unit purchase and qualified-revenue accounting are isolated funding paths.
- Referral relationships are immutable and ancestry is validated at runtime.
- Supported stablecoin mints are typed, distinct and require six decimals.
- Protocol vault, treasury and claim token accounts are constrained to canonical ATAs.
- Expired balances are permissionlessly settled and transferred to treasury.
- Claims require the beneficiary signature and ACTIVE status.
- Global service Unit IDs are monotonic `u128` ranges allocated in O(1) across all wallets.
- No owner/admin mutation, pause, treasury mutation, referral mutation or revenue-source mutation instruction exists.
- Static gates, Rust tests and LiteSVM integration tests pass.
- Dependency lockfiles are committed and CI is read-only.
- Anchor Docker `--verifiable` production build passes on Anchor 1.1.2.

## Remaining before irreversible mainnet

- Freeze final qualified-revenue source program/PDA.
- Freeze registration opening UTC.
- Establish final Program ID keypair custody outside the public repository.
- Complete devnet deployment and transaction smoke tests.
- Complete independent third-party security audit.
- Re-run locked CI and verifiable build after final immutable parameters are frozen.
- Perform limited mainnet smoke test while upgrade authority remains available.
- Remove upgrade authority permanently only after final verification.
