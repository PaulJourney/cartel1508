# Qualified Revenue Source — final trust model

Status: architecture freeze candidate for external review. The production source address remains intentionally unset in `constants.rs` until this dependency is implemented, audited and frozen.

## Objective

`record_qualified_revenue` must never depend on a permanently privileged human wallet. For an ownerless final protocol, the recommended production `qualified_revenue_source` is a deterministic PDA controlled by a separate revenue-adapter program. That PDA has no private key; the adapter program authorizes it only during CPI using its signer seeds.

The adapter is a distinct trust boundary from the referral protocol and must be independently audited before its PDA can be frozen into the referral program.

## Hard boundaries

1. Service-unit purchases and activity/registration payments are not qualified revenue and must never be forwarded into `record_qualified_revenue` as the source of referral rewards.
2. A qualified event must be separately funded before the referral program creates liabilities.
3. The adapter source PDA must not equal the service treasury, a user wallet, the deployment wallet or any upgrade-authority wallet.
4. The adapter must not possess a generic instruction that lets an administrator fabricate arbitrary funded reward events for arbitrary beneficiaries after finalization.
5. USDT and USDC remain separate rails; the adapter must not perform an implicit swap or cross-token accounting substitution.
6. Every accepted revenue event must be idempotent. Replaying the same external event must fail on-chain.

## Recommended on-chain structure

### Revenue adapter program

The adapter receives or verifies independently qualified service revenue according to the final product integration. Its exact qualification rules are deliberately not invented here; they depend on the real external revenue source and must be reviewed as part of the final audit.

Recommended immutable accounts:

- `AdapterConfig` PDA: frozen integration identity and supported protocol Program ID.
- `RevenueAuthority` PDA: the exact public key frozen as `MAINNET_QUALIFIED_REVENUE_SOURCE` in the referral protocol.
- canonical USDT ATA owned by `RevenueAuthority`.
- canonical USDC ATA owned by `RevenueAuthority`.
- one `RevenueReceipt` PDA per unique external event identifier/hash to prevent replay.

### Revenue receipt

A successful event should create an immutable receipt containing at least:

- unique event ID or cryptographic hash;
- beneficiary wallet/PDA;
- stablecoin mint;
- gross funded amount;
- accepted-at slot/timestamp;
- immutable reference/hash to the external qualification evidence where applicable.

Creating a second receipt with the same deterministic event ID must fail.

## Atomic flow

1. Independently qualified service revenue becomes available to the adapter under the final reviewed integration rules.
2. The adapter validates the unique event and proves that no `RevenueReceipt` exists for it.
3. The exact stablecoin amount is available in the `RevenueAuthority` PDA's canonical token account before referral liabilities are created.
4. The adapter creates the immutable `RevenueReceipt`.
5. The adapter performs a CPI into `record_qualified_revenue`, passing the `RevenueAuthority` PDA as signer via PDA signer seeds.
6. The referral program independently validates that this signer equals its immutable `qualified_revenue_source`, transfers the gross amount into its vault, and only then applies the 50/43/2/5 accounting.
7. If any step fails, the Solana transaction rolls back atomically, including the receipt creation and token/accounting mutations.

## Final immutability requirements

Before freezing the adapter PDA into the referral protocol:

- adapter source code and exact Program ID are final;
- qualification logic and anti-replay semantics are covered by integration/adversarial tests;
- adapter program has undergone independent third-party audit together with the referral protocol integration boundary;
- adapter deployment bytecode is verified against the audited source;
- no mutable admin/config instruction can redirect the referral Program ID, beneficiary rules or qualified-revenue semantics after final freeze;
- if the adapter uses an upgradeable loader during smoke testing, its upgrade authority is removed only after deployed-bytecode verification and controlled mainnet smoke testing;
- the resulting `RevenueAuthority` PDA is frozen into `MAINNET_QUALIFIED_REVENUE_SOURCE`, after which the referral protocol is rebuilt and re-audited/re-verified as the final artifact.

## Explicitly rejected final configurations

Do not freeze any of the following as production `qualified_revenue_source`:

- the service treasury wallet;
- the user's treasury/deployer MetaMask address;
- a normal EOA/keypair controlled by one operator;
- the temporary deployment/upgrade-authority wallet;
- an adapter whose configuration can later be changed by an admin;
- any source whose funding is the service-unit/activity purchase flow itself.

## Audit questions

The independent auditor should specifically answer:

- Can the same economic revenue event be counted twice?
- Can an attacker substitute the beneficiary, mint, source token account or referral protocol?
- Can a human/admin fabricate an event after finalization even if they provide funding?
- Can unit-purchase funds reach this source path?
- Can adapter or referral state be reconfigured after the advertised immutability point?
- Is every liability created by `record_qualified_revenue` fully collateralized within the same atomic transaction?

Until these questions have reviewed answers and a concrete adapter/source exists, the referral protocol's production source sentinel must remain fail-closed.
