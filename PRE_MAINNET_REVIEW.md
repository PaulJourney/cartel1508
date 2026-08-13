# Pre-mainnet irreversible decisions — V0.10

## Technical baseline completed

- Pinned Anchor/Solana build passes.
- Locked dependency CI passes read-only.
- Rust and LiteSVM security/economic lifecycle tests pass.
- Global Unit ID allocation is tested across wallets.
- Anchor Docker verifiable production build passes.

## Blocking before deployment

- Final qualified service revenue source program ID + signer PDA.
- Registration opening UTC timestamp.
- Final Program ID/keypair custody procedure.
- Devnet deployment and smoke tests.
- Independent third-party audit.
- Final locked/verifiable build after all immutable values are frozen.

## Blocking before permanent immutability

- Limited mainnet deployment and smoke test while upgrade authority remains available.
- Confirm deployed bytecode against the final verified artifact.
- Only then remove upgrade authority permanently.

## Explicit boundary

Service-unit purchases remain isolated from the qualified-revenue accounting path. The final qualified-revenue source must be separately funded and independently reviewed before mainnet.
