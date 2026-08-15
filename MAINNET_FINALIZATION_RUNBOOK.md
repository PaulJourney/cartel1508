# Mainnet finalization runbook — three-program release chain

This runbook is intentionally conservative. Permanent immutability is the final step, never the deployment step.

## 1. Freeze the final trust model and identities

- Obtain explicit approval of the concrete, non-self-generable qualified-revenue evidence model. A payer-signed token transfer alone is not sufficient evidence of legitimate service revenue.
- Generate three final Program ID keypairs offline/outside the public repository and CI: referral protocol, revenue adapter and revenue qualification gateway.
- Record only their public Program IDs in source control.
- Keep all Program ID keypairs and temporary upgrade-authority/deployer key material in secure, separate custody.
- Freeze adapter -> referral, adapter -> qualification and qualification -> adapter production bindings.
- Derive the final adapter `RevenueAuthority` PDA and freeze that exact public key as the referral protocol's qualified-revenue source.
- Derive the final qualification `qualified-revenue-authority` PDA from the final qualification Program ID; no private key exists for this authority.
- Freeze the exact registration opening UTC.
- Add all three exact Program IDs under `[programs.mainnet]` in `Anchor.toml`.
- Hash the approved `QUALIFIED_REVENUE_SOURCE_SPEC.md` and record that hash in the final release evidence.

## 2. Rebuild the exact final commit

- Run reference/static gates.
- Prove the pre-mainnet gate is fail-closed before the final manifest is completed.
- Run locked Rust, integration, property and security suites.
- Run the real-validator three-program smoke path.
- Run the RustSec dependency scan.
- Build the production profile for all three programs.
- Run the Docker verifiable build for all three programs.
- Record each `.so` SHA-256, all IDLs, Cargo.lock, repository commit and pinned toolchain versions.

Any source, dependency, Program ID, binding, evidence rule or immutable constant change after this point invalidates the corresponding final build evidence.

## 3. Independent audit

- Give the auditor the exact final commit, three Program IDs, cross-program binding model and all three artifact hashes.
- Include the qualification/evidence specification and the boundaries between qualification -> adapter -> referral.
- Require explicit review of anti-replay, beneficiary substitution, mint/source substitution, self-generated revenue risk, collateralization and authority-removal sequence.
- Resolve accepted findings before deployment.
- Rebuild/reverify after any source/dependency/configuration change.

## 4. Freeze release evidence and run the executable gate

Create `release/mainnet-release.json` from `release/mainnet-release.example.json` and freeze public evidence for:

- final commit;
- three Program IDs;
- final adapter Revenue Authority PDA / core qualified-revenue source;
- final Qualification Authority PDA;
- registration UTC;
- treasury and canonical mints;
- all three verified `.so` SHA-256 values;
- independent audit-report SHA-256;
- approved qualification-evidence-spec SHA-256;
- verified-build Actions run URL;
- audit status;
- qualification evidence status;
- approved limited mainnet smoke plan.

Then run:

```text
python3 scripts/pre-mainnet-gate.py
```

The gate independently derives both authority PDAs from the frozen final Program IDs, self-tests its PDA implementation against real Solana validator vectors and requires exact agreement with core/source/manifest values.

The result must be exactly `READY FOR CONTROLLED MAINNET DEPLOYMENT`. A `BLOCKED` result is a hard stop. Do not bypass or weaken the gate for deployment convenience.

This gate authorizes only controlled deployment. It does not authorize removal of any upgrade authority.

## 5. Mainnet deployment with temporary upgrade authority

- Measure exact final artifact sizes and current mainnet deployment requirements immediately before funding.
- Fund only the deployment/authority wallet with the SOL required for all three deployments plus the documented fee margin.
- Deploy the exact verified referral, adapter and qualification `.so` artifacts using their intended offline Program ID keypairs.
- Keep upgrade authority available during the tightly limited verification/smoke window.
- Record `solana program show` metadata, ProgramData address, deployment slot, transaction signature and authority for all three.

## 6. Verify all deployed bytecodes and identities

For each program:

- dump/verify deployed bytecode against the exact final verified artifact;
- confirm Program ID matches source, `Anchor.toml` and release manifest;
- confirm IDL/build evidence refers to the exact final release;
- stop if any byte differs.

Then independently derive and verify:

- adapter Revenue Authority PDA;
- qualification authority PDA;
- frozen adapter/referral/qualification cross-program identities.

## 7. Controlled initialization

Initialize only after all three deployed bytecodes are verified.

- Use only the frozen mainnet treasury, canonical USDT/USDC mints, registration UTC and cross-program identities.
- Verify core `qualified_revenue_source` equals the deterministic Revenue Authority PDA of the deployed final adapter.
- Verify adapter qualification authority equals the deterministic PDA under the final qualification program.
- Verify no mutable administrator or update route can redirect the advertised trust chain.

## 8. Limited mainnet smoke test

Exercise only the minimum pre-approved transactions needed to confirm the audited release:

- registration/qualification state where appropriate;
- service-unit isolation from reward funding;
- separately qualified revenue path;
- expected token conservation/accounting;
- anti-replay behavior;
- failure/rollback behavior if safely testable under the approved plan;
- claim behavior.

Stop immediately if deployed state, PDA derivation, token movement, accounting or bytecode differs from the audited baseline.

## 9. Permanent immutability

Only after all previous gates are signed off, remove upgrade authority from each program intended to be immutable:

```text
solana program set-upgrade-authority <FINAL_REFERRAL_PROGRAM_ID> --final
solana program set-upgrade-authority <FINAL_REVENUE_ADAPTER_PROGRAM_ID> --final
solana program set-upgrade-authority <FINAL_REVENUE_QUALIFICATION_PROGRAM_ID> --final
```

Run `solana program show` for all three again and archive proof of the resulting authority state.

After `--final`, that program cannot be upgraded or closed. There is no rollback procedure. Never finalize one program while the release plan still depends on changing another program's frozen cross-program identity.

## Secrets policy

Never commit or upload:

- any of the three final Program ID keypair JSON files;
- deployment wallet seed/private key;
- temporary upgrade-authority private key;
- treasury private key;
- encrypted secret backups or recovery phrases.

GitHub/audit artifacts should contain only public keys, source, IDLs, hashes, transaction signatures and non-secret verification evidence.
