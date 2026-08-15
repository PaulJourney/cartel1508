# Revenue Adapter V0.1 — staged trust boundary

Status: implementation candidate for CI and adversarial testing. Not mainnet-final.

## Purpose

The adapter is the only intended production path into `service_referral_protocol::record_qualified_revenue`. It does not decide whether an economic event is genuine. That decision belongs to a separately reviewed verifier program.

The call chain is:

`Verifier Program -> Revenue Adapter -> Service Referral Protocol -> SPL Token Program`

The verifier calls the adapter by CPI and signs with its deterministic `qualified-revenue-verifier` PDA. A normal wallet cannot directly satisfy that signer requirement because the PDA has no private key.

## Immutable adapter responsibilities

1. Store the verifier Program ID once at initialization; expose no mutation instruction.
2. Derive and validate the verifier-authority PDA from that Program ID.
3. Control a separate `revenue-authority` PDA with no private key.
4. Require qualified funds to already exist in the RevenueAuthority PDA's canonical SPL Token ATA.
5. Create exactly one deterministic receipt PDA per 32-byte event ID.
6. Bind each receipt to evidence hash, beneficiary, mint, amount, accepted timestamp and slot.
7. CPI into the frozen referral Program ID and sign only as RevenueAuthority PDA.
8. Rely on Solana transaction atomicity: if the downstream referral/token operation fails, receipt creation and all state changes roll back.

## Deliberately excluded

- no admin wallet;
- no setter for verifier Program ID;
- no setter for referral Program ID;
- no arbitrary treasury withdrawal;
- no arbitrary beneficiary override after a verifier CPI is composed;
- no conversion between USDT and USDC;
- no service-unit purchase path;
- no generic externally signed `record revenue` instruction.

## Anti-replay

`RevenueReceipt` is initialized at PDA seeds:

`[b"revenue-receipt", event_id]`

A second use of the same event ID attempts to initialize the same account and must fail. If the downstream CPI fails, the whole transaction rolls back, so a failed event does not consume the receipt ID.

## Mainnet freeze dependencies

The adapter remains fail-closed for production because `MAINNET_VERIFIER_PROGRAM` is still the system-program sentinel. Before mainnet:

1. implement and independently review the concrete verifier program/integration;
2. generate final adapter and referral Program IDs offline;
3. freeze the final verifier Program ID in adapter source;
4. derive the final RevenueAuthority PDA from the final adapter Program ID;
5. freeze that PDA as `MAINNET_QUALIFIED_REVENUE_SOURCE` in the referral program;
6. run cross-program adversarial tests and full accounting tests;
7. obtain independent third-party audit of verifier -> adapter -> referral boundary;
8. produce locked/verifiable builds and compare deployed bytecode before removing either upgrade authority.

## Current staging rule

The adapter is intentionally kept in an isolated Cargo workspace during the first CI cycle. CI generates and preserves its lockfile and SBF artifact. Once compile/test gates are green, the generated lockfile is reviewed and the adapter is folded into the root release workspace for the final freeze.
