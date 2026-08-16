# Pre-mainnet irreversible decisions — V0.12

## Technical baseline completed

- Pinned Anchor/Solana core build passes.
- Locked dependency CI passes for the referral core; isolated dependency resolution is checked for the Revenue Adapter and Revenue Qualification programs.
- Rust and LiteSVM security/economic lifecycle tests pass.
- Global Unit ID allocation is tested across wallets.
- Deterministic property tests cover accounting conservation and Unit ID range invariants.
- Anchor development SBF build passes for the referral core.
- Previous production-equivalent referral devnet transaction smoke passed: initialize, Pioneer #1 registration, 10-unit purchase, separately funded qualified revenue, 50/43/2/5 accounting and ACTIVE claim.
- The 2026-08-14 referral smoke conserved exactly 20 test tokens end-to-end: 5.002 to the test user, 14.998 to the test treasury and 0 remaining in the vault after claim.
- Revenue Adapter compiles, unit-tests and builds as an SBF candidate while retaining fail-closed production identities.
- Revenue Qualification compiles, unit-tests and builds as an SBF candidate while retaining fail-closed production identities.
- The test-only Revenue Evidence fixture compiles for testing and is explicitly prevented from compiling with the `production` feature.
- The hardened four-program LiteSVM chain proves `evidence -> qualification -> adapter -> referral` behavior end-to-end.
- The full-chain test proves exact 50/43/2/5 conservation, canonical prefunding, beneficiary/evidence binding, replay rejection, mismatch rollback and atomic downstream accounting.
- The Revenue Adapter still enforces verifier-PDA authorization: an ordinary wallet cannot substitute for the qualification verifier PDA.
- A release-binding test derives the Adapter `RevenueAuthority` PDA and requires the core's frozen qualified-revenue source to match it exactly once final identities are introduced.
- The executable pre-mainnet gate has been aligned to the four-program production architecture and remains intentionally fail-closed.

## Intentionally unresolved product boundary

`QUALIFIED_REVENUE_QUALIFICATION_SPEC.md` is currently marked `BLOCKED`.

This is deliberate. The repository must not invent which real service-revenue event qualifies, how beneficiary and amount are derived, how evidence is authenticated, how economic finality is established, or how refunds/reversals are treated. Those product/economic semantics must be concretely defined before a production Revenue Evidence source can exist.

Service-unit/activity purchases remain explicitly excluded from qualified revenue and must never be routed into referral reward creation.

The current `integration-tests/fixtures/evidence_stub` is test infrastructure only. It demonstrates the security and atomicity of the downstream chain; it is not a production source of truth and is compile-blocked for production.

## Blocking before controlled mainnet deployment

1. Complete `QUALIFIED_REVENUE_QUALIFICATION_SPEC.md` with concrete product/economic semantics and set exactly one status marker to `QUALIFICATION_SPEC_STATUS: FINAL`.
2. Implement the real production `programs/revenue_evidence` source from that FINAL specification, including its trust source, event identity, beneficiary/amount binding, evidence, funding provenance and economic-finality rules.
3. Complete adversarial tests for the production evidence source against Revenue Qualification, Revenue Adapter and the referral core.
4. Generate final offline Program ID keypairs for all four production programs: Revenue Evidence, Revenue Qualification, Revenue Adapter and Service Referral Protocol.
5. Freeze the final Revenue Evidence Program ID and Revenue Adapter Program ID into Revenue Qualification.
6. Freeze the final Revenue Qualification Program ID into the Revenue Adapter.
7. Freeze the final Revenue Adapter Program ID into the referral core and derive/freeze the adapter's exact `[b"revenue-authority"]` PDA as `MAINNET_QUALIFIED_REVENUE_SOURCE`.
8. Freeze the exact registration opening UTC timestamp.
9. Obtain an independent third-party audit of the complete `evidence -> qualification -> adapter -> referral` boundary and dispose of all accepted findings.
10. Produce final locked/verifiable builds of all four programs from the exact audited source commit; archive all four artifact hashes and verified-build evidence.
11. Complete `release/mainnet-release.json`, including the FINAL qualification-spec hash and all four verified-build evidences.
12. `python3 scripts/pre-mainnet-gate.py` must return exactly `READY FOR CONTROLLED MAINNET DEPLOYMENT`.

Until all twelve conditions hold, mainnet deployment remains intentionally blocked.

## Blocking before permanent immutability

- Deploy Revenue Evidence, Revenue Qualification, Revenue Adapter and referral core with temporary upgrade authority retained during the controlled verification window.
- Initialize only after all required executable Program IDs and frozen identities are present and mutually consistent.
- Verify deployed bytecode for all four programs against the exact audited artifact hashes.
- Run a limited mainnet smoke test of the complete `evidence -> qualification -> adapter -> referral` flow using the frozen production configuration.
- Stop immediately on any Program ID, PDA, bytecode, state, token balance, evidence binding or accounting mismatch.
- Only after the complete chain passes may upgrade authority be removed permanently from every production program.

## Explicit trust boundary

The referral core does not decide whether revenue is qualified. The Revenue Adapter does not decide whether revenue is qualified. Revenue Qualification does not invent an economic event; it validates canonical evidence emitted by the one frozen production Revenue Evidence program and forwards it through the hardened adapter boundary.

The production Revenue Evidence program must implement only the FINAL, independently reviewed qualification specification. Its trust source and any external system capable of causing evidence issuance are part of the audited security boundary.

The Adapter's role is limited to enforcing the Revenue Qualification verifier-PDA authorization, canonical prefunded RevenueAuthority token accounts, immutable event receipts and atomic CPI into the referral core.