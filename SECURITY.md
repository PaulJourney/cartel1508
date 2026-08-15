# Security model — V0.11

This is pre-audit software and is not yet approved for irreversible production deployment.

## Production trust chain

The intended production path is:

`Revenue Verifier -> Revenue Adapter -> Service Referral Protocol -> SPL Token Program`

No human wallet is intended to have permanent authority to fabricate referral-reward events after finalization.

## Service Referral Protocol — implemented and tested

- Service-unit/activity purchases and qualified-revenue accounting are isolated funding paths.
- Referral relationships are immutable and ancestry is validated at runtime.
- Supported stablecoin mints are typed, distinct, fixed to legacy SPL Token semantics and require six decimals.
- Protocol vault, treasury and claim token accounts are constrained to canonical ATAs.
- Expired network balances are permissionlessly settled and transferred to treasury.
- Claims require beneficiary signature and ACTIVE status.
- Global service Unit IDs are monotonic ranges allocated in O(1) across all wallets.
- No owner/admin mutation, pause, treasury mutation, referral mutation or qualified-revenue-source mutation instruction exists.
- Reference/static/Rust/property/LiteSVM gates and verifiable-build workflows have passed on release-candidate commits.

## Revenue Adapter — implemented and tested

- No admin setter can mutate verifier, referral target or RevenueAuthority after initialization.
- Verifier authorization is bound to a deterministic PDA under the configured verifier Program ID.
- An unsigned verifier PDA is rejected.
- An ordinary wallet that signs correctly but substitutes its own key for the verifier PDA is rejected.
- RevenueAuthority is an adapter PDA with no private key.
- Qualified funds must be prefunded in RevenueAuthority's canonical token ATA before liabilities are created.
- Event receipts are deterministic by event ID and bind evidence hash, beneficiary, mint, amount, timestamp and slot.
- Successful event replay is rejected.
- Downstream failure rolls back receipt creation, token movement and referral accounting atomically.
- Release-binding tests require the referral core's frozen qualified-revenue source to equal the exact RevenueAuthority PDA derived from the final adapter Program ID.
- Adapter RustSec dependency scanning and SBF production-candidate builds have passed on release-candidate commits.

## Revenue Verifier — intentionally not implemented yet

A production verifier must not be invented before the product/economic qualification semantics are frozen. `QUALIFIED_REVENUE_QUALIFICATION_SPEC.md` is intentionally `BLOCKED` until the exact qualified event, beneficiary and amount derivation, evidence, funding provenance, reversals and trust source are defined.

The mainnet gate treats missing verifier source and a non-FINAL qualification specification as hard blockers.

## Release-integrity controls

- The pre-mainnet gate checks referral, adapter and verifier identities as a single release chain.
- Final verifier, adapter and referral Program IDs must be distinct.
- Adapter's frozen verifier ID must match the verifier `declare_id!`.
- Referral core's frozen adapter ID must match the adapter `declare_id!`.
- Referral core's frozen qualified-revenue source must be the adapter's exact derived RevenueAuthority PDA.
- Final release evidence must bind the audited source SHA, FINAL qualification-spec SHA, independent audit hash and all three production artifact hashes.
- A PR workflow uses the actual PR head SHA rather than GitHub's synthetic merge-commit SHA for release-source binding.

## Remaining before controlled mainnet deployment

- Make the qualified-revenue specification concrete and FINAL.
- Implement and adversarially test the real production Revenue Verifier.
- Generate final offline Program IDs/keypairs for verifier, adapter and referral.
- Freeze all cross-program identities/PDA and registration opening UTC.
- Obtain an independent third-party audit of the complete verifier -> adapter -> referral boundary.
- Re-run all locked/security/verifiable gates against the exact final commit and artifacts.
- Complete the release manifest and obtain `READY FOR CONTROLLED MAINNET DEPLOYMENT` from the executable gate.
- Deploy with temporary upgrade authority, verify all three deployed bytecodes and run a limited full-chain mainnet smoke.

## Permanent immutability

Only after complete deployed-chain verification may upgrade authority be removed from verifier, adapter and referral. Once removed with `--final`, those programs cannot be upgraded or closed and there is no rollback path.
