# Mainnet finalization runbook

This runbook is intentionally conservative. Permanent immutability is the final step, never the deployment step.

## 1. Freeze the final identities

- Generate the final program keypair outside the public repository and outside CI artifacts.
- Record only its public Program ID in source control.
- Keep the program keypair and temporary upgrade-authority key material in separate secure custody.
- Freeze the qualified-revenue source program/PDA and registration opening UTC.
- Replace all development/sentinel Program ID and production constants in one reviewed change set.
- Add a `[programs.mainnet]` entry in `Anchor.toml` that exactly matches `declare_id!`.

## 2. Rebuild from the final commit

- Run reference/static gates.
- Run the locked Rust and LiteSVM suite.
- Run the RustSec dependency scan.
- Build the production profile.
- Run the Docker verifiable build.
- Record the final `.so` SHA-256, IDL, Cargo.lock, repository commit and toolchain versions.

Any change after this point invalidates the final build evidence and requires the gates to be repeated.

## 3. Independent audit

- Give the auditor the exact final commit and artifact hash.
- Resolve accepted findings before deployment.
- If source, dependencies, constants or Program ID change, rebuild and re-verify the artifact submitted for deployment.

## 4. Freeze release evidence and run the executable gate

Create `release/mainnet-release.json` from `release/mainnet-release.example.json` and freeze the final public evidence: commit, Program ID, qualified-revenue source, registration UTC, treasury/mints, verified `.so` SHA-256, independent audit-report SHA-256 and verified-build evidence URL.

Then run:

```text
python3 scripts/pre-mainnet-gate.py
```

The result must be exactly `READY FOR CONTROLLED MAINNET DEPLOYMENT`. A `BLOCKED` result is a hard stop. Do not bypass or weaken the gate for deployment convenience.

This gate authorizes only the controlled deployment phase. It does not authorize removal of upgrade authority.

## 5. Mainnet deployment with temporary upgrade authority

- Fund only the deployment/authority wallet with the SOL required for rent and transaction fees.
- Deploy the exact final verified `.so` using the intended Program ID keypair.
- Keep upgrade authority available during the limited smoke-test window.
- Record `solana program show` metadata, slot, ProgramData address and authority.

## 6. Verify deployed bytecode

- Dump the deployed program from mainnet.
- Compare the dumped bytecode against the final verified artifact.
- Publish/update verified-build metadata while the upgrade authority is still available.
- Confirm Program ID, source commit, IDL and build parameters all refer to the same final release.

## 7. Limited mainnet smoke test

- Initialize only with the frozen production configuration.
- Verify canonical vault/treasury addresses and protocol state.
- Exercise only the minimum transactions needed to confirm the deployed release behaves as audited.
- Stop immediately if deployed state, bytecode or accounting differs from the audited baseline.

## 8. Permanent immutability

Only after all previous gates are signed off:

```text
solana program set-upgrade-authority <FINAL_PROGRAM_ID> --final
```

Then run `solana program show <FINAL_PROGRAM_ID>` again and archive the result proving that no upgrade authority remains.

After this operation the program cannot be upgraded or closed. There is no rollback procedure.

## Secrets policy

Never commit or upload:

- final program keypair JSON
- deployment wallet seed/private key
- temporary upgrade-authority private key
- encrypted backups or recovery phrases

GitHub and public audit artifacts should contain only public keys, source, IDL, hashes, transaction signatures and non-secret verification evidence.
