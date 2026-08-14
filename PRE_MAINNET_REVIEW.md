# Pre-mainnet irreversible decisions — V0.10

## Technical baseline completed

- Pinned Anchor/Solana build passes.
- Locked dependency CI passes read-only.
- Rust and LiteSVM security/economic lifecycle tests pass.
- Global Unit ID allocation is tested across wallets.
- Deterministic property tests cover accounting conservation and Unit ID range invariants.
- Anchor Docker verifiable production build passes.
- Ephemeral Solana devnet deployment passes.
- Full production-equivalent devnet transaction smoke passes: initialize, Pioneer #1 registration, 10-unit purchase, separately funded qualified revenue, 50/43/2/5 accounting and ACTIVE claim.
- The 2026-08-14 transaction smoke conserved exactly 20 test tokens end-to-end: 5.002 to the test user, 14.998 to the test treasury and 0 remaining in the vault after claim.
- A fail-closed executable pre-mainnet release gate and release-manifest template are present.

## Blocking before deployment

- Final qualified service revenue source program ID + signer PDA.
- Registration opening UTC timestamp.
- Final Program ID/keypair custody procedure outside the public repository.
- Independent third-party audit and disposition of findings.
- Final locked/verifiable build after all immutable values are frozen.
- Complete `release/mainnet-release.json` and obtain `READY FOR CONTROLLED MAINNET DEPLOYMENT` from `python3 scripts/pre-mainnet-gate.py`.

## Blocking before permanent immutability

- Controlled mainnet deployment while upgrade authority remains available.
- Confirm deployed bytecode against the final verified artifact.
- Limited mainnet smoke test against frozen configuration.
- Only then remove upgrade authority permanently.

## Explicit boundary

Service-unit purchases remain isolated from the qualified-revenue accounting path. The final qualified-revenue source must be separately funded and independently reviewed before mainnet.
