# Mainnet finalization runbook

This runbook is intentionally conservative. Permanent immutability is the final step, never the deployment step.

## 1. Freeze the real qualification semantics

Before a production verifier exists, `QUALIFIED_REVENUE_QUALIFICATION_SPEC.md` must define the exact economic event, funding provenance, beneficiary/amount derivation, event-ID derivation, evidence hash, refund/reversal rules, trust source and supported assets.

The file must contain exactly one status marker and it must be:

`QUALIFICATION_SPEC_STATUS: FINAL`

Service-unit/activity purchases remain excluded from qualified revenue.

## 2. Freeze the three final program identities

Generate three distinct Program ID keypairs outside the public repository and CI:

- revenue verifier;
- Revenue Adapter;
- Service Referral Protocol.

Record only public Program IDs in source control. Freeze the verifier Program ID into the adapter, derive the adapter's `[b"revenue-authority"]` PDA from its final Program ID, and freeze both the adapter Program ID and that exact PDA into the referral core. Freeze the final referral Program ID in `declare_id!` and `[programs.mainnet]`. Freeze registration opening UTC in the same reviewed release set.

## 3. Rebuild and test the complete chain from the final commit

- Run reference/static gates.
- Run locked Rust/property/LiteSVM core tests.
- Run Revenue Adapter static/unit/SBF gates.
- Run full verifier -> adapter -> referral cross-program adversarial tests.
- Run RustSec scans for all production dependency graphs.
- Produce production artifacts for verifier, adapter and referral.
- Produce verifiable-build evidence for all three.
- Record each `.so` SHA-256, IDL, dependency lockfile, repository source commit and pinned toolchain versions.

Any change after this point invalidates the affected build evidence and requires the gates to be repeated.

## 4. Independent audit

Give the auditor the exact FINAL qualification specification, exact source commit and all artifact hashes. The audit scope must include:

- verifier qualification/trust semantics;
- event identity and anti-replay;
- beneficiary, mint and amount binding;
- verifier PDA authorization;
- adapter RevenueAuthority canonical-ATA and prefunding checks;
- adapter receipt creation;
- adapter -> referral CPI boundary;
- referral ancestry/activity/accounting behavior;
- rollback atomicity across the complete transaction;
- absence of mutable admin paths after finalization.

Resolve accepted findings before deployment. If source, dependencies, constants, Program IDs or qualification semantics change, rebuild and re-verify the exact release submitted for deployment.

## 5. Freeze release evidence and run the executable gate

Create `release/mainnet-release.json` from the example and populate only verified public evidence:

- exact audited source commit;
- verifier, adapter and referral Program IDs;
- exact adapter-derived RevenueAuthority PDA;
- registration UTC;
- service treasury and stablecoin mints;
- three production `.so` SHA-256 hashes;
- FINAL qualification-spec SHA-256;
- independent audit-report SHA-256;
- verified-build evidence URLs for all three programs;
- audit status and approved mainnet smoke plan.

Then run:

```text
python3 scripts/pre-mainnet-gate.py
```

The result must be exactly `READY FOR CONTROLLED MAINNET DEPLOYMENT`. `BLOCKED` is a hard stop. Do not bypass or weaken the gate for deployment convenience.

This authorizes only controlled deployment. It does not authorize removal of any upgrade authority.

## 6. Fund the temporary deployer only after final sizing

Measure the exact final artifacts and query current mainnet deployment requirements. Fund only the temporary deployment/authority wallet with the required SOL plus an explicit fee margin. Do not use the service treasury or RevenueAuthority as deployment funding wallets.

## 7. Controlled mainnet deployment

Deploy the exact audited artifacts with temporary upgrade authority retained:

1. revenue verifier;
2. referral protocol;
3. Revenue Adapter.

Record `solana program show` metadata, ProgramData addresses, deployment slots, transaction signatures and authorities for all three. The order above ensures verifier and referral executables exist before adapter initialization.

## 8. Verify all deployed bytecode before initialization

Dump each deployed program from mainnet and compare it against the corresponding audited artifact hash. Confirm final Program IDs, source commit, IDLs and build parameters all refer to the same release. Confirm that the final adapter Program ID derives the exact RevenueAuthority PDA frozen in the referral core.

Do not initialize if any comparison fails.

## 9. Initialize the frozen chain

Initialize only with the reviewed production configuration. Verify:

- verifier Program ID is the exact one frozen in the adapter;
- adapter Program ID and RevenueAuthority PDA are the exact values frozen in the referral core;
- registration UTC, treasury and USDT/USDC mints match the release manifest;
- canonical token accounts are derived as expected.

## 10. Limited mainnet smoke test

Exercise only the minimum transactions needed to confirm the complete deployed release behaves as audited:

- one independently qualified and separately funded event under the FINAL verifier semantics;
- verifier PDA CPI into the adapter;
- creation of the unique immutable receipt;
- adapter RevenueAuthority transfer into the referral vault;
- expected 50/43/2/5 accounting;
- appropriate ACTIVE claim behavior;
- explicit replay rejection for the same event ID;
- no unexpected residual token imbalance.

Stop immediately if bytecode, Program ID, PDA, receipt, state, token balances or accounting differ from the audited baseline.

## 11. Permanent immutability

Only after all previous gates are signed off, remove upgrade authority from **each** production program:

```text
solana program set-upgrade-authority <FINAL_VERIFIER_PROGRAM_ID> --final
solana program set-upgrade-authority <FINAL_ADAPTER_PROGRAM_ID> --final
solana program set-upgrade-authority <FINAL_REFERRAL_PROGRAM_ID> --final
```

Then run `solana program show` for all three and archive evidence proving no upgrade authority remains.

After finalization, these programs cannot be upgraded or closed. There is no rollback procedure.

## Secrets policy

Never commit or upload final Program ID keypair JSON files, deployment-wallet seeds/private keys, temporary upgrade-authority secrets, recovery phrases or encrypted secret backups. Public repository/audit/release evidence should contain only public keys, source, IDLs, hashes, transaction signatures and non-secret verification data.
