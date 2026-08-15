# Pre-mainnet irreversible decisions — V0.11

## Technical baseline completed

- Pinned Anchor/Solana core build passes.
- Locked dependency CI passes for the referral core; Revenue Adapter dependency resolution is independently checked and RustSec-scanned.
- Rust and LiteSVM security/economic lifecycle tests pass.
- Global Unit ID allocation is tested across wallets.
- Deterministic property tests cover accounting conservation and Unit ID range invariants.
- Anchor Docker verifiable production build passes for the referral core.
- Ephemeral Solana devnet deployment passes for the referral core.
- Full production-equivalent referral devnet transaction smoke passes: initialize, Pioneer #1 registration, 10-unit purchase, separately funded qualified revenue, 50/43/2/5 accounting and ACTIVE claim.
- The 2026-08-14 referral smoke conserved exactly 20 test tokens end-to-end: 5.002 to the test user, 14.998 to the test treasury and 0 remaining in the vault after claim.
- Revenue Adapter V0.1 compiles, unit-tests and builds as an SBF production candidate.
- Cross-program LiteSVM tests prove the verifier-PDA authorization boundary: unsigned verifier PDA is rejected; an ordinary signed wallet cannot substitute for the verifier PDA; valid verifier-PDA CPI succeeds; replay of the same event ID fails; downstream failure rolls back the receipt, token movement and referral accounting atomically.
- A release-binding test derives the Adapter `RevenueAuthority` PDA and requires the core's frozen qualified-revenue source to match it exactly once final identities are introduced.
- The executable pre-mainnet gate now covers the complete verifier -> adapter -> referral release chain and remains intentionally fail-closed.

## Intentionally unresolved product boundary

`QUALIFIED_REVENUE_QUALIFICATION_SPEC.md` is currently marked `BLOCKED`.

This is deliberate. The repository must not invent which real service-revenue event qualifies, how beneficiary and amount are derived, how evidence is authenticated, or how refunds/reversals are treated. Those product/economic semantics must be concretely defined before a production verifier can exist.

Service-unit/activity purchases remain explicitly excluded from qualified revenue and must never be routed into referral reward creation.

## Blocking before controlled mainnet deployment

1. Complete `QUALIFIED_REVENUE_QUALIFICATION_SPEC.md` with concrete product/economic semantics and set exactly one status marker to `QUALIFICATION_SPEC_STATUS: FINAL`.
2. Implement the real production revenue verifier from that FINAL specification; cover its trust source, event identity, beneficiary/amount binding, evidence, funding and anti-replay semantics with adversarial tests.
3. Generate final offline Program ID keypairs for all three programs: revenue verifier, Revenue Adapter and Service Referral Protocol.
4. Freeze the final verifier Program ID into the adapter.
5. Freeze the final adapter Program ID into the referral core and derive/freeze the adapter's exact `[b"revenue-authority"]` PDA as `MAINNET_QUALIFIED_REVENUE_SOURCE`.
6. Freeze the exact registration opening UTC timestamp.
7. Obtain an independent third-party audit of the complete verifier -> adapter -> referral boundary and dispose of all accepted findings.
8. Produce final locked/verifiable builds of verifier, adapter and referral from the exact audited source commit; archive all three artifact hashes.
9. Complete `release/mainnet-release.json`, including the FINAL qualification-spec hash and all three verified-build evidences.
10. `python3 scripts/pre-mainnet-gate.py` must return exactly `READY FOR CONTROLLED MAINNET DEPLOYMENT`.

Until all ten conditions hold, do not fund a mainnet deployment wallet merely to accelerate release.

## Blocking before permanent immutability

- Deploy verifier, referral and adapter with temporary upgrade authority retained during the controlled verification window.
- Initialize only after all required executable Program IDs and frozen identities are present.
- Verify deployed bytecode for all three programs against the exact audited artifact hashes.
- Run a limited mainnet smoke test of the complete verifier -> adapter -> referral flow using the frozen production configuration.
- Stop immediately on any Program ID, PDA, bytecode, state, token balance or accounting mismatch.
- Only after the complete chain passes may upgrade authority be removed permanently from every production program.

## Explicit trust boundary

The referral core does not decide whether revenue is qualified. The Revenue Adapter does not decide whether revenue is qualified. The production verifier must implement only the FINAL, independently reviewed qualification specification. The adapter's role is limited to enforcing verifier-PDA authorization, canonical prefunded RevenueAuthority token accounts, immutable event receipts and atomic CPI into the referral core.
