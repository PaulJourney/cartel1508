# Qualified Revenue Source — three-program final trust model

Status: the on-chain payment/adapter/referral transport is implemented and has passed a real-validator adversarial smoke test. The **business/economic evidence rule that makes a payment legitimate qualified service revenue is still intentionally unresolved and must remain a mainnet blocker** until it is concrete, non-self-generable, independently reviewed and frozen.

## Objective

`record_qualified_revenue` must never depend on a permanently privileged human wallet and must never create referral liabilities from service-unit/activity purchases.

The current architecture separates three responsibilities:

1. `revenue_qualification`: establishes whether an event may enter the qualified-revenue path and signs with a deterministic Qualification Authority PDA;
2. `revenue_adapter`: provides immutable anti-replay receipts and a deterministic Revenue Authority PDA that holds/funds the downstream referral CPI;
3. `service_referral_protocol`: independently validates its immutable qualified-revenue source, takes custody of the gross stablecoin amount and applies the 50/43/2/5 accounting.

The Revenue Authority PDA has no private key and is the intended final `MAINNET_QUALIFIED_REVENUE_SOURCE` after final Program IDs and evidence semantics are reviewed and frozen.

## Hard boundaries

1. Service-unit purchases and activity/registration payments are not qualified revenue and must never be forwarded into `record_qualified_revenue` as the source of referral rewards.
2. Every qualified event must be separately funded before referral liabilities are created.
3. Real token funding is necessary but **not sufficient** proof of legitimate service revenue: a participant must not be able to qualify an economic event merely by paying themselves/the system and selecting a beneficiary.
4. No permanently privileged human key may fabricate or approve arbitrary reward events after finalization.
5. Revenue Authority and Qualification Authority must be deterministic PDAs with no private keys.
6. Revenue Authority must not equal the service treasury, any Program ID, user wallet, deployer or upgrade-authority wallet.
7. USDT and USDC remain separate rails; no implicit swap/cross-token accounting substitution is permitted.
8. Every accepted economic event must be idempotent. Replaying the same event must fail on-chain.
9. Beneficiary, mint, amount and evidence identity must be cryptographically/deterministically bound so they cannot be substituted after qualification.
10. Any downstream failure must roll back the payment/receipt/accounting transaction atomically.

## Implemented on-chain structure

### Revenue qualification program

Current implementation:

- accepts a payer-signed SPL token transfer;
- supports only mints frozen in the adapter config;
- requires the payer token account to be owned by the payer;
- requires the destination to be the canonical stablecoin ATA of the adapter Revenue Authority PDA;
- derives an event identifier deterministically from payer, beneficiary, mint, amount and client nonce using Solana PDA primitives;
- signs the CPI into the adapter only through its deterministic `qualified-revenue-authority` PDA;
- is production fail-closed until the final adapter Program ID is frozen.

This proves that actual stablecoin value moves into the qualified-revenue path and that event identity cannot be freely replaced by the caller. It does **not** yet establish that the payment corresponds to an independently legitimate service-revenue event.

### Revenue adapter program

Implemented immutable accounts/boundaries:

- `AdapterConfig` PDA: initialize-once configuration binding referral Program ID, qualification Program ID, supported mints, Qualification Authority PDA and Revenue Authority PDA;
- `RevenueAuthority` PDA: no private key; exact intended source frozen into core at final release;
- canonical USDT and USDC ATAs owned by Revenue Authority;
- one deterministic `RevenueReceipt` PDA per event ID;
- no mutable admin/update instruction;
- production fail-closed until final referral/qualification Program IDs are frozen.

A successful receipt records event ID, beneficiary, mint, amount, acceptance time/slot and qualifier identity. A second initialization of the same deterministic receipt fails.

### Referral protocol

The referral protocol independently:

- checks that the qualified-revenue signer equals its immutable configured source;
- transfers the gross amount from the source ATA into the protocol vault before accounting;
- applies direct/network/Pioneer/service allocation only after collateralization;
- routes immediately treasury-assigned components to the configured service treasury;
- retains only claimable liabilities in the vault.

## Proven atomic flow

The following path has been executed successfully on an isolated real `solana-test-validator` with all three programs built and deployed:

1. initialize referral with Revenue Authority PDA as qualified-revenue source;
2. initialize adapter bound to referral + qualification identities;
3. register Pioneer #1;
4. purchase 10 service units and prove those funds route to treasury, not reward vault;
5. payer signs a separate 100-USDC payment into Revenue Authority ATA;
6. qualification PDA signs CPI into adapter;
7. adapter creates the deterministic receipt and signs CPI into referral with Revenue Authority PDA;
8. referral transfers/collateralizes the 100 USDC and applies accounting;
9. exact replay is rejected and all token balances remain unchanged;
10. a fresh event with deliberately invalid downstream ancestry is rejected and both the payer transfer and newly-created receipt are rolled back;
11. ACTIVE beneficiary claims direct + Pioneer liability;
12. all minted test USDC are conserved exactly.

Evidence baseline: GitHub Actions run `31899960367`, artifact ID `9250852374`. Final test balances were 50.02 USDC to the test user/Pioneer, 200 USDC remaining with the payer, 59.98 USDC treasury, 0 protocol vault and 0 Revenue Authority ATA.

This smoke is strong evidence for **transport, collateralization, anti-replay and transaction atomicity**. It is not evidence that an external business event is economically genuine.

## Missing production evidence layer — hard mainnet blocker

Before `qualification_evidence_status` may be set to `approved`, the actual product/service integration must define a source of truth that an arbitrary participant cannot self-generate.

The final specification must answer, concretely:

- What external/on-chain event constitutes a legitimate service revenue event?
- Who/what creates that event, and why can the beneficiary/payer not fabricate it?
- What immutable identifier makes the economic event unique?
- How are payer, beneficiary, stablecoin mint and gross amount bound to that evidence?
- Can the evidence be verified entirely on-chain, or does it require a reviewed oracle/attestation mechanism?
- If an attestation mechanism exists, what prevents one human/operator from remaining a permanent unilateral reward administrator?
- What happens when the external source reverses, refunds or disputes a payment?
- Which event is considered authoritative if multiple external systems represent the same payment?

A generic instruction of the form “any wallet may pay X tokens and nominate beneficiary Y” must **not** be approved as the final evidence model, even though it is fully collateralized, because it permits self-generated reward events.

## Required final release binding

After the evidence layer is concrete and independently approved:

1. generate final Program IDs offline;
2. freeze `revenue_qualification::MAINNET_ADAPTER_PROGRAM` to final adapter ID;
3. freeze adapter `MAINNET_REFERRAL_PROGRAM` to final referral ID;
4. freeze adapter `MAINNET_QUALIFICATION_PROGRAM` to final qualification ID;
5. derive the final adapter Revenue Authority PDA;
6. freeze that exact PDA into core `MAINNET_QUALIFIED_REVENUE_SOURCE`;
7. freeze all three `[programs.mainnet]` identities;
8. hash this approved specification into `release/mainnet-release.json` as `qualification_evidence_spec_sha256`;
9. rebuild all three exact production artifacts and independently audit that exact final commit/binding;
10. keep `qualification_evidence_status` other than `approved` until the reviewer/auditor accepts the actual evidence semantics.

## Independent audit questions

The final auditor should specifically answer:

- Can the same economic revenue event be counted twice using different nonces/receipts/evidence representations?
- Can an attacker substitute payer, beneficiary, mint, amount, source token account, qualification program, adapter or referral program?
- Can a participant self-generate the evidence needed to create rewards?
- Can a human/admin fabricate or approve events after advertised finalization?
- Can unit-purchase/activity funds reach this source path?
- Does a failed qualification/adapter/referral CPI leave any payment or receipt mutation behind?
- Is every liability fully collateralized before accounting?
- Can any of the three program configs or cross-program identities be redirected after initialization/finalization?
- Are refund/reversal/dispute semantics compatible with immutable reward creation?

Until the concrete evidence model has reviewed answers to these questions, the referral protocol's production source sentinel and cross-program production sentinels must remain fail-closed.
