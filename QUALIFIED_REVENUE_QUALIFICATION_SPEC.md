# Qualified Revenue Qualification Specification

QUALIFICATION_SPEC_STATUS: BLOCKED

This file is the mandatory product/economic specification for the production revenue verifier. It is intentionally **not final** yet. Mainnet release must remain blocked until the real qualification semantics are defined, independently reviewed, implemented and frozen.

## What must be frozen before this status can become FINAL

### 1. Economic event

Define exactly which real-world or on-chain event constitutes qualified service revenue. Service-unit/activity purchases are explicitly excluded and must never qualify.

### 2. Funding provenance

Define how USDT/USDC reaches the Revenue Adapter's `RevenueAuthority` canonical ATA before liabilities are created. The verifier must not accept an unfunded event.

### 3. Beneficiary binding

Define how the qualified event deterministically identifies the beneficiary user. An operator must not be able to substitute an arbitrary beneficiary after the economic event occurred.

### 4. Amount derivation

Define how the exact qualified gross amount is derived and verified. The verifier must not permit a human/admin to choose an arbitrary reward amount.

### 5. Unique event identifier

Define the canonical 32-byte event ID or its deterministic derivation. The same economic event must always map to the same ID so the adapter receipt PDA prevents replay.

### 6. Evidence hash

Define the canonical evidence payload and hashing procedure. The evidence hash recorded in `RevenueReceipt` must bind the event, beneficiary, mint and amount to the reviewed qualification evidence.

### 7. Refunds, reversals and cancellations

Define whether an event is eligible only after it becomes economically final. The production design must not create irreversible referral liabilities from revenue that can later be freely reversed unless the reviewed economics explicitly account for that risk.

### 8. Trust source

Define the exact immutable authority or cryptographic proof accepted by the verifier. Any oracle, signer set, upstream program or attestation scheme must have a documented security and key-rotation model. A generic permanent admin signer is not acceptable.

### 9. Supported assets

Only the frozen legacy SPL USDT and USDC mints used by the referral protocol may qualify. No implicit swaps or cross-token substitutions are permitted.

### 10. Failure and replay semantics

The verifier must fail closed. A failed verifier -> adapter -> referral transaction must consume no receipt and create no token/accounting mutation. Reusing the same successful event ID must fail.

## Finalization rule

Change the marker above to exactly:

`QUALIFICATION_SPEC_STATUS: FINAL`

only after all sections are concrete, the production verifier implementation matches them, adversarial tests cover them, and the independent auditor has reviewed the complete verifier -> adapter -> referral boundary. The final release manifest must contain the SHA-256 of this exact FINAL file.
