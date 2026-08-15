# Qualified Revenue Qualification Specification

`QUALIFICATION_SPEC_STATUS: BLOCKED`

This file is a **hard release input**, not a placeholder that may be silently inferred by code or an operator. Mainnet must remain blocked until the status is intentionally changed to `FINAL` after the real product/business revenue source is defined and independently reviewed.

## Purpose

Define exactly what external/on-chain fact makes one payment a legitimate **qualified service revenue event** that is allowed to create referral rewards.

A payer-signed USDT/USDC transfer is necessary for collateralization but is **not sufficient qualification evidence** because a participant could self-fund an artificial event.

## Fields that must be frozen before `FINAL`

### 1. Economic event

- `event_name`: `<UNSET>`
- `business_description`: `<UNSET>`
- `when_revenue_is_economically_earned`: `<UNSET>`
- `what_is_explicitly_not_qualified_revenue`: service-unit/activity purchases plus `<UNSET>`

### 2. Authoritative evidence source

- `source_system_or_program`: `<UNSET>`
- `source_identity/program_id/domain`: `<UNSET>`
- `who_controls_that_source`: `<UNSET>`
- `why_an_arbitrary_participant_cannot_fabricate_the_evidence`: `<UNSET>`
- `whether_verification_is_fully_on_chain`: `<UNSET>`

If an oracle/attestation is required:

- `attestation_signer_model`: `<UNSET>`
- `key_rotation/recovery_model`: `<UNSET>`
- `how_permanent_unilateral_human_reward_control_is_prevented`: `<UNSET>`

### 3. Canonical evidence identity

Every underlying economic event must produce exactly one canonical 32-byte `evidence_hash`.

- `canonical_event_identifier`: `<UNSET>`
- `canonical_serialization`: `<UNSET>`
- `hash_algorithm/domain_separation`: `<UNSET>`
- `why_the_same_economic_event_cannot_legitimately_produce_two_different_evidence_hashes`: `<UNSET>`

The qualification gateway derives its on-chain event/receipt identity from:

- `evidence_hash`
- payer
- beneficiary
- stablecoin mint
- gross amount

There is deliberately no freely variable replay nonce in the economic identity.

### 4. Payer binding

- `how_payer_is_identified_in_the_authoritative_evidence`: `<UNSET>`
- `relationship_between_external_payer_and_Solana_signer`: `<UNSET>`
- `mismatch_policy`: `<UNSET>`

### 5. Beneficiary binding

- `how_the_reward_beneficiary_is_selected`: `<UNSET>`
- `why_the_payer_cannot_arbitrarily_redirect_an_unrelated_sale`: `<UNSET>`
- `beneficiary_change_policy_after_event_creation`: `<UNSET>`

### 6. Amount and stablecoin binding

- `gross_amount_definition`: `<UNSET>`
- `supported_payment_assets`: USDT / USDC, subject to final canonical mint verification
- `how_asset_and_amount_are_bound_to_evidence`: `<UNSET>`
- `fees/taxes/refunds treatment`: `<UNSET>`

### 7. Replay / duplicate prevention

- `what_makes_the_underlying_event_unique`: `<UNSET>`
- `cross-system_duplicate_policy`: `<UNSET>`
- `retry/idempotency_policy`: `<UNSET>`

Changing metadata, a nonce, an API request ID or an integration path must not allow the same underlying economic revenue to generate rewards twice.

### 8. Refund / reversal / dispute semantics

Because on-chain referral liabilities can become irreversible after claim/finalization, define before launch:

- `refund_policy`: `<UNSET>`
- `chargeback/reversal_policy`: `<UNSET>`
- `dispute_window_if_any`: `<UNSET>`
- `when_an_event_becomes_final_enough_to_create_rewards`: `<UNSET>`

### 9. Failure semantics

- evidence valid but token transfer fails → no receipt/reward
- token transfer succeeds but adapter/referral CPI fails → entire Solana transaction must roll back
- duplicate evidence → reject without token/accounting mutation
- unsupported mint/amount/payer/beneficiary mismatch → reject before reward creation
- any additional product-specific failure rules: `<UNSET>`

## Required independent review before `FINAL`

The reviewer/auditor must explicitly determine that:

- an arbitrary participant cannot fabricate qualified revenue;
- one underlying economic event cannot generate two canonical evidence hashes;
- payer, beneficiary, mint and gross amount cannot be substituted after qualification;
- refunds/reversals do not create an unacceptable mismatch with irreversible referral liabilities;
- no permanent human/admin key can unilaterally manufacture reward events contrary to the advertised trust model;
- the exact evidence semantics correspond to the code and final integration being deployed.

## Release rule

`QUALIFICATION_SPEC_STATUS` may change from `BLOCKED` to `FINAL` only in the exact final candidate commit reviewed by the independent auditor.

The SHA-256 of this exact `FINAL` file must be stored as `qualification_evidence_spec_sha256` in `release/mainnet-release.json` and verified by `scripts/pre-mainnet-gate.py`.
