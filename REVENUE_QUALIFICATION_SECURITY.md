# Revenue Qualification Gateway — Security Boundary

## Purpose

`revenue_qualification` is the only component allowed to sign as the deterministic `qualified-revenue-authority` PDA accepted by `revenue_adapter`.

It deliberately does **not** decide by itself that a human-funded transfer is genuine external revenue. Doing so would let an operator or attacker fabricate a commission-bearing event simply by supplying funds, which violates the frozen qualified-revenue trust model.

Instead, the gateway accepts an event only when an independently reviewed immutable **evidence program** invokes it through CPI and signs with that program's deterministic `revenue-evidence-authority` PDA.

## Frozen evidence interface

For event ID `E`, the evidence program must own the canonical PDA:

`["revenue-evidence", E]`

The account serializes `RevenueEvidenceV1` containing:

- evidence version;
- event ID;
- qualification Program ID;
- adapter Program ID;
- adapter revenue-authority PDA;
- beneficiary;
- stablecoin mint;
- gross amount;
- settled timestamp;
- non-zero external reference/hash.

The qualification gateway checks every field against the runtime accounts and its immutable configuration.

## Authorization chain

1. The separately audited evidence program determines whether a real external revenue event satisfies the product-specific rules.
2. That evidence program owns the deterministic evidence PDA and is the only program capable of signing as its `revenue-evidence-authority` PDA.
3. Through CPI, the evidence program calls `revenue_qualification::qualify_verified_evidence`.
4. The qualification gateway validates the evidence PDA, owner, version, event fields, mint, beneficiary, amount and binding to the exact qualification/adapter/revenue-authority identities.
5. It verifies that the adapter revenue-authority canonical USDC/USDT ATA is already funded for at least the evidenced amount.
6. The qualification gateway signs a CPI with its own `qualified-revenue-authority` PDA into `revenue_adapter`.
7. The adapter creates the deterministic event receipt and signs as its `revenue-authority` PDA into `service_referral_protocol`.
8. The referral protocol transfers the gross stablecoin amount into its vault before applying the immutable 50/43/2/5 accounting.

If the evidence program funds/creates evidence and invokes this chain in one transaction, any downstream failure rolls the entire transaction back atomically.

## Trust properties

- A normal wallet cannot sign as the evidence-authority PDA.
- A normal wallet cannot sign as the qualification-authority PDA.
- A normal wallet cannot sign as the adapter revenue-authority PDA.
- The rent payer used for the adapter receipt has no authority to qualify an event.
- There is no admin/update instruction in the qualification gateway.
- The evidence Program ID, adapter Program ID, their canonical PDAs and supported stablecoin identities are frozen at initialize-once configuration.
- The downstream adapter retains deterministic anti-replay using one `RevenueReceipt` PDA per event ID.
- Service-unit/activity purchases remain outside this path.

## Remaining product-specific dependency

This gateway intentionally does not invent the external evidence semantics. The final evidence program must correspond to the real source of qualified service revenue and must be independently reviewed. Examples of questions that belong there include what business event constitutes earned revenue, how a beneficiary is attributed and what makes an external settlement final.

Until that concrete evidence program exists and is reviewed, `MAINNET_EVIDENCE_PROGRAM` remains the default sentinel and a production build cannot initialize.

## Mainnet freeze requirements

Before production initialization:

- finalize and audit the evidence program and its qualification semantics;
- freeze `MAINNET_EVIDENCE_PROGRAM` to that exact Program ID;
- freeze `MAINNET_ADAPTER_PROGRAM` to the reviewed adapter Program ID;
- verify USDT and USDC mint identities;
- rebuild all components reproducibly from the frozen commit;
- verify artifact hashes and independent audit evidence;
- initialize in dependency order and run controlled mainnet smoke tests;
- remove upgrade authorities only after deployed bytecode and runtime behavior are verified.
