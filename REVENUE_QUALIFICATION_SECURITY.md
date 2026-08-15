# Revenue Qualification Program — Security Boundary

## Purpose

`revenue_qualification` is the only component allowed to sign as the deterministic `qualified-revenue-authority` PDA accepted by `revenue_adapter`.

Its job is deliberately narrow: a revenue event becomes qualified only when a real supported stablecoin payment is transferred from the payer's canonical SPL token account to the adapter revenue-authority canonical token account in the same Solana transaction that records and distributes the event.

## Atomic flow

1. The payer signs the transaction.
2. The program validates a non-zero event ID and amount.
3. The payer source must be the canonical SPL ATA owned by the payer.
4. The mint must be one of the two immutable configured stablecoins.
5. The destination must be the canonical SPL ATA owned by the adapter `revenue-authority` PDA.
6. The stablecoin payment is transferred to that destination.
7. The qualification program signs a CPI with its `qualified-revenue-authority` PDA into `revenue_adapter`.
8. The adapter creates the deterministic event receipt and signs its own CPI into `service_referral_protocol`.
9. The referral protocol transfers the funded revenue into its vault and applies the immutable split/accounting rules.

All steps occur in one transaction. Failure at any downstream stage rolls back the payer transfer, adapter receipt and referral accounting.

## Trust properties

- No private key exists for the qualification-authority PDA.
- No private key exists for the adapter revenue-authority PDA.
- There is no admin instruction capable of minting, attesting or manually qualifying arbitrary revenue.
- Configuration is a deterministic initialize-once PDA.
- The adapter Program ID and adapter config PDA are frozen at initialization.
- The adapter config must be owned by the adapter program.
- Supported stablecoin identities are frozen at initialization.
- Production builds additionally require reviewed mainnet identities and fail closed while the adapter Program ID remains the sentinel.
- The downstream adapter retains deterministic receipt anti-replay by `event_id`.

## What this proves

The qualification program proves an on-chain fact: an exact amount of an allowed SPL stablecoin was authorized by the payer and moved into the protocol revenue pipeline in the same atomic execution.

It does **not** claim to prove off-chain facts such as delivery of a physical good, legal entitlement, identity, chargeback status on another payment rail or the truth of an external oracle. Those would require a separately reviewed trust model and must not be silently added to this program.

## Mainnet freeze requirements

Before a production build can initialize:

- replace `MAINNET_ADAPTER_PROGRAM` sentinel with the final reviewed adapter Program ID;
- verify USDT and USDC mint identities;
- build all three programs reproducibly from the same frozen commit;
- verify bytecode hashes and independent audit evidence;
- initialize in dependency order: referral protocol, revenue adapter, qualification program;
- run limited mainnet smoke tests before removing any upgrade authorities.
