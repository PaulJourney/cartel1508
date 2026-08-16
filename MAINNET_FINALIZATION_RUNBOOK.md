# Mainnet finalization runbook

This runbook is intentionally conservative. Permanent immutability is the final step, never the deployment step.

## 1. Freeze the real qualification semantics

Before a production Revenue Evidence source exists, `QUALIFIED_REVENUE_QUALIFICATION_SPEC.md` must define the exact economic event, funding provenance, beneficiary/amount derivation, event-ID derivation, evidence/reference-hash derivation, refund/reversal rules, trust source and supported assets.

The file must contain exactly one status marker and it must be:

`QUALIFICATION_SPEC_STATUS: FINAL`

Service-unit/activity purchases remain excluded from qualified revenue.

## 2. Implement the production Evidence source

Implement `programs/revenue_evidence` from the FINAL specification. Its immutable or explicitly reviewed trust source must deterministically emit the canonical evidence accepted by Revenue Qualification.

The current `integration-tests/fixtures/evidence_stub` is test-only infrastructure and must never be substituted for this production program.

## 3. Freeze the four final program identities

Generate four distinct Program ID keypairs outside the public repository and CI:

- Revenue Evidence;
- Revenue Qualification;
- Revenue Adapter;
- Service Referral Protocol.

Record only public Program IDs in source control. Freeze Evidence and Adapter Program IDs into Revenue Qualification; freeze Revenue Qualification into the Adapter; derive the Adapter's `[b"revenue-authority"]` PDA from its final Program ID; freeze both Adapter Program ID and that exact PDA into the Referral core. Freeze the final Referral Program ID in `declare_id!` and `[programs.mainnet]`. Freeze registration opening UTC in the same reviewed release set.

## 4. Rebuild and test the complete chain from the final commit

- Run reference/static gates.
- Run locked Rust/property/LiteSVM core tests.
- Run Revenue Adapter static/unit/SBF gates.
- Run Revenue Qualification unit/SBF gates.
- Run production Revenue Evidence tests and SBF gates.
- Run full `evidence -> qualification -> adapter -> referral` cross-program adversarial tests.
- Run RustSec scans for all production dependency graphs.
- Produce production artifacts for all four programs.
- Produce verifiable-build evidence for all four.
- Record each `.so` SHA-256, IDL, dependency lockfile, repository source commit and pinned toolchain versions.

Any change after this point invalidates the affected build evidence and requires the gates to be repeated.

## 5. Independent audit

Give the auditor the exact FINAL qualification specification, exact source commit and all artifact hashes. The audit scope must include:

- Revenue Evidence qualification/trust semantics and any upstream dependency capable of issuing evidence;
- event identity, economic finality and anti-replay;
- beneficiary, mint and amount binding;
- Evidence-authority PDA -> Qualification boundary;
- Qualification verifier PDA -> Adapter authorization;
- Adapter RevenueAuthority canonical-ATA and prefunding checks;
- Adapter receipt creation;
- Adapter -> Referral CPI boundary;
- Referral ancestry/activity/accounting behavior;
- rollback atomicity across the complete transaction;
- absence of mutable admin paths after finalization.

Resolve accepted findings before deployment. If source, dependencies, constants, Program IDs or qualification semantics change, rebuild and re-verify the exact release submitted for deployment.

## 6. Freeze release evidence and run the executable gate

Create `release/mainnet-release.json` from the example and populate only verified public evidence:

- exact audited source commit;
- Evidence, Qualification, Adapter and Referral Program IDs;
- exact Adapter-derived RevenueAuthority PDA;
- registration UTC;
- service treasury and stablecoin mints;
- four production `.so` SHA-256 hashes;
- FINAL qualification-spec SHA-256;
- independent audit-report SHA-256;
- verified-build evidence URLs for all four programs;
- audit status and approved mainnet smoke plan.

Then run:

```text
python3 scripts/pre-mainnet-gate.py
```

The result must be exactly `READY FOR CONTROLLED MAINNET DEPLOYMENT`. `BLOCKED` is a hard stop. Do not bypass or weaken the gate for deployment convenience.

This authorizes only controlled deployment. It does not authorize removal of any upgrade authority.

## 7. Fund the temporary deployer only after final sizing

Measure the exact final artifacts and query current mainnet deployment requirements. Fund only the temporary deployment/authority wallet with the required SOL plus an explicit fee margin. Do not use the service treasury or RevenueAuthority as deployment funding wallets.

## 8. Controlled mainnet deployment

Deploy the exact audited artifacts with temporary upgrade authority retained:

1. Revenue Evidence;
2. Revenue Qualification;
3. Service Referral Protocol;
4. Revenue Adapter.

Record `solana program show` metadata, ProgramData addresses, deployment slots, transaction signatures and authorities for all four.

## 9. Verify all deployed bytecode before initialization

Dump each deployed program from mainnet and compare it against the corresponding audited artifact hash. Confirm final Program IDs, source commit, IDLs and build parameters all refer to the same release. Confirm that the final Adapter Program ID derives the exact RevenueAuthority PDA frozen in the Referral core.

Do not initialize if any comparison fails.

## 10. Initialize the frozen chain

Initialize only with the reviewed production configuration. Verify:

- Evidence Program ID is the exact one frozen in Revenue Qualification;
- Adapter Program ID is the exact one frozen in Revenue Qualification;
- Qualification Program ID is the exact one frozen in the Adapter;
- Adapter Program ID and RevenueAuthority PDA are the exact values frozen in the Referral core;
- registration UTC, treasury and USDT/USDC mints match the release manifest;
- canonical token accounts and all authority PDAs are derived as expected.

## 11. Limited mainnet smoke test

Exercise only the minimum transactions needed to confirm the complete deployed release behaves as audited:

- one independently qualified and separately funded event under the FINAL Evidence semantics;
- canonical Evidence creation and Evidence-authority CPI into Revenue Qualification;
- Qualification verifier-PDA CPI into the Adapter;
- creation of the unique immutable Adapter receipt;
- Adapter RevenueAuthority transfer into the Referral vault;
- expected 50/43/2/5 accounting;
- appropriate ACTIVE claim behavior;
- explicit replay rejection for the same event ID;
- no unexpected residual token imbalance.

Stop immediately if bytecode, Program ID, PDA, evidence, receipt, state, token balances or accounting differ from the audited baseline.

## 12. Permanent immutability

Only after all previous gates are signed off, remove upgrade authority from **each** production program:

```text
solana program set-upgrade-authority <FINAL_EVIDENCE_PROGRAM_ID> --final
solana program set-upgrade-authority <FINAL_QUALIFICATION_PROGRAM_ID> --final
solana program set-upgrade-authority <FINAL_ADAPTER_PROGRAM_ID> --final
solana program set-upgrade-authority <FINAL_REFERRAL_PROGRAM_ID> --final
```

Then run `solana program show` for all four and archive evidence proving no upgrade authority remains.

After finalization, these programs cannot be upgraded or closed. There is no rollback procedure.

## Secrets policy

Never commit or upload final Program ID keypair JSON files, deployment-wallet seeds/private keys, temporary upgrade-authority secrets, recovery phrases or encrypted secret backups. Public repository/audit/release evidence should contain only public keys, source, IDLs, hashes, transaction signatures and non-secret verification data.