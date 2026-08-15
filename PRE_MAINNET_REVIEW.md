# Pre-mainnet irreversible decisions — three-program release chain

## Technical baseline completed

- Pinned Anchor/Solana build passes.
- Locked dependency CI passes read-only.
- Rust and LiteSVM security/economic lifecycle tests pass for the established referral baseline.
- Global Unit ID allocation is tested across wallets.
- Deterministic property tests cover accounting conservation and Unit ID range invariants.
- The referral protocol, revenue adapter and revenue qualification gateway compile together under the pinned dependency graph.
- The revenue adapter uses an immutable config, deterministic Revenue Authority PDA and one deterministic receipt PDA per event.
- The qualification gateway requires an actual payer-signed SPL transfer before calling the adapter and uses a deterministic Qualification Authority PDA with no private key.
- Production features for the adapter and qualification gateway remain deliberately fail-closed until final Program IDs and cross-program bindings are frozen.
- A real isolated `solana-test-validator` smoke deploy of all three programs passes end-to-end.
- Successful three-program smoke evidence: GitHub Actions run `31899960367`, artifact `qualification-localnet-smoke-evidence` / ID `9250852374`.
- That smoke proved: initialization, Pioneer #1 registration, 10-unit activation, a real 100-USDC payer transfer, qualification -> adapter -> referral CPI accounting, exact replay rejection, downstream ancestry failure rollback, receipt rollback and final ACTIVE claim.
- The three-program smoke conserved all 310 test USDC exactly: 50.02 USDC final user balance, 200 USDC customer remainder, 59.98 USDC treasury, 0 reward vault and 0 adapter revenue ATA.
- The original referral-only production-equivalent devnet transaction smoke also remains valid baseline evidence for the core protocol.
- A fail-closed executable three-program pre-mainnet release gate and release-manifest template are present.

## What the successful payment gateway proves

The current qualification gateway proves that real USDT/USDC value is transferred by a signer and that payment, receipt creation and referral accounting are atomic. It also proves that replay and downstream failure do not leave partial token movements or receipts.

It does **not** yet prove that the payer-signed transfer corresponds to a legitimate external service/business revenue event. A participant could otherwise self-fund a payment and still satisfy the purely on-chain payment condition. This distinction remains a hard mainnet blocker rather than being hidden behind the successful smoke test.

## Blocking before deployment

- Freeze and externally review the exact non-self-generable qualification/evidence model for legitimate service revenue.
- Freeze three final mainnet Program IDs: referral protocol, revenue adapter and revenue qualification gateway.
- Derive the final adapter Revenue Authority PDA from the final adapter Program ID and freeze that PDA as the referral protocol's `MAINNET_QUALIFIED_REVENUE_SOURCE`.
- Freeze cross-program production bindings: adapter -> referral, adapter -> qualification and qualification -> adapter.
- Add all three exact identities under `[programs.mainnet]` in `Anchor.toml`.
- Freeze the registration opening UTC timestamp.
- Keep all three final Program ID private keypairs outside the public repository, ChatGPT and CI.
- Independent third-party audit of the exact final three-program trust chain and disposition of findings.
- Final locked/verifiable build after every immutable identity, evidence rule and timestamp is frozen.
- Complete `release/mainnet-release.json`, including three artifact hashes, audit hash and approved qualification-evidence-spec hash.
- `python3 scripts/pre-mainnet-gate.py` must return `READY FOR CONTROLLED MAINNET DEPLOYMENT`.

## Blocking before permanent immutability

- Controlled mainnet deployment of all three exact verified artifacts while temporary upgrade authority remains available.
- Confirm each deployed bytecode against its final verified artifact.
- Initialize only with the frozen cross-program configuration.
- Run the limited mainnet smoke plan against real canonical stablecoin mints and frozen addresses.
- Confirm the Revenue Authority PDA and Qualification Authority PDA match the reviewed derivations.
- Only then remove upgrade authority permanently from every program that is intended to be immutable.

## Explicit economic boundary

Service-unit purchases remain isolated from qualified-revenue accounting. They qualify activity and route to the service treasury; they must never be recycled into the qualified-revenue source path.

A real payer-signed stablecoin transfer is necessary but not, by itself, sufficient evidence of legitimate qualified service revenue. The production evidence source/rule must be concrete, non-self-generable, independently reviewed and hashed into the final release evidence before mainnet can be authorized.
