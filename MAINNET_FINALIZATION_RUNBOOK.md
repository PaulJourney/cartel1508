# Mainnet Finalization Runbook

This procedure is intentionally conservative. **Permanent immutability is the final step, never the deployment step.**

## 1. Freeze the final core surface

Before any mainnet deployment:

- `purchase_and_distribute` must be the sole production economic entrypoint;
- all obsolete qualified-revenue source/adapter/evidence/qualification state and instructions must be removed from the final core;
- final L1–L10 genealogy and 50/43/2/5 constants must be frozen;
- service treasury and canonical USDT/USDC mints must be frozen;
- registration-opening UTC must be frozen;
- no mutable owner/admin instruction may remain.

## 2. Freeze the final Program ID

Generate one final Service Referral Protocol Program ID keypair offline under `PROGRAM_ID_CUSTODY_RUNBOOK.md`.

Commit only the public Program ID:

- exact `declare_id!` in the program;
- exact matching `[programs.mainnet]` entry in `Anchor.toml`.

Never commit or paste the secret keypair.

## 3. Build and test the exact final commit

From a clean checkout of the release commit:

- run `node tests/reference-model.mjs`;
- run `python3 scripts/static-gates.py`;
- run locked Rust tests;
- run all final LiteSVM integration/adversarial tests;
- run RustSec scan;
- build the production SBF artifact with the pinned toolchain;
- build the verifiable artifact;
- record `.so` SHA-256, IDL, `Cargo.lock`, source commit and toolchain versions.

Any source/configuration/dependency change after this point invalidates the artifact hash and requires the gates to be rerun.

## 4. Devnet smoke on the final interface

Before independent final sign-off, deploy a temporary/dev identity and exercise the same final instruction surface:

1. initialize;
2. register a controlled genealogy;
3. purchase units using USDC/USDT-compatible six-decimal test mints;
4. verify global unit IDs and activity state;
5. verify 50% direct / 43% L2–L10 / 2% Pioneer / 5% service accounting;
6. verify canonical vault collateral;
7. claim from an ACTIVE beneficiary using a separate claimant-signed transaction;
8. test an invalid ancestry/account substitution and verify complete rollback;
9. test grace/expiry behavior;
10. verify final token conservation.

Devnet evidence is workflow evidence only; it is not a substitute for mainnet bytecode verification or independent audit.

## 5. Independent audit

Give the auditor:

- exact source commit;
- final public Program ID;
- final constants and registration UTC;
- exact dependency lockfile;
- final IDL;
- exact production/verifiable artifact hash;
- automated test/build evidence;
- client transaction-builder source used to derive sponsor/upline accounts.

Audit at minimum:

- purchase payment amount and token-account validation;
- immutable referral ancestry and L2–L10 traversal;
- 50/43/2/5 conservation and rounding;
- ACTIVE/GRACE/INACTIVE behavior;
- Pioneer indexing/fractional carry;
- vault collateral and treasury routing;
- pull claim authorization;
- expiry settlement;
- unit-ID overflow/continuity;
- rollback atomicity;
- absence of mutable admin/control paths;
- client construction of all required accounts.

Resolve every accepted finding. Any material code/configuration/dependency change requires a new exact artifact and appropriate renewed audit review.

## 6. Complete release evidence

Create `release/mainnet-release.json` from the example and populate:

- exact audited source commit SHA;
- final Program ID;
- registration-opening UTC;
- service treasury;
- USDT mint;
- USDC mint;
- final production `.so` SHA-256;
- independent audit-report SHA-256;
- verified-build evidence URL;
- `audit_status: "passed"`;
- `smoke_test_plan_approved: true`.

Then run:

```text
python3 scripts/pre-mainnet-gate.py
```

A non-zero result is a hard stop. Do not weaken the gate for deployment convenience.

## 7. Fund the temporary deployer

Only after the final artifact exists:

1. measure exact `.so` size;
2. query current Solana mainnet program deployment/rent/fee requirements;
3. fund the temporary deployer with required SOL plus a small explicit margin;
4. keep deployment SOL separate from protocol USDT/USDC economics.

## 8. Controlled mainnet deployment

Deploy the exact audited `.so` using:

- the offline final Program ID keypair;
- a separate temporary deployer/upgrade-authority keypair.

Keep upgrade authority temporarily.

Record:

- Program ID;
- ProgramData address;
- deployment slot;
- deployment transaction signature;
- current upgrade authority;
- source commit and local artifact hash.

## 9. Verify deployed bytecode before initialization

Dump/read the deployed program and verify its bytecode against the exact audited artifact/hash.

Also verify:

- Program ID equals the frozen public ID;
- mainnet `Anchor.toml` mapping matches;
- IDL corresponds to the same source commit;
- service treasury and canonical stablecoin constants match the release manifest.

Do not initialize if any comparison fails.

## 10. Initialize the frozen program

Initialize only with:

- frozen service treasury `AepYo8xanmKuRiLVeYQuCTJoQr1nyKiTApoKwHMEg8fn`;
- canonical mainnet USDT `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB`;
- canonical mainnet USDC `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`;
- frozen future registration-opening timestamp.

Verify protocol and technical-root PDAs immediately after initialization.

## 11. Limited mainnet smoke

Use deliberately small amounts and controlled wallets. Verify at least:

- one registration under the intended referral relationship;
- one unit purchase using a canonical supported stablecoin account;
- exact unit count / Unit ID assignment;
- buyer-paid purchase transaction;
- expected vault and treasury movement;
- expected direct/network/Pioneer accruals;
- separate ACTIVE pull claim signed by the beneficiary;
- claimant-paid transaction fee;
- final token conservation;
- no unexpected state/account changes.

Run at least one intentionally invalid ancestry/account test if it can be done without risking production state; otherwise rely on the exact audited LiteSVM adversarial evidence.

Stop immediately on any bytecode, Program ID, PDA, genealogy, state, balance or accounting mismatch.

## 12. Permanent immutability

Only after deployment verification and smoke are signed off, remove upgrade authority permanently using the exact pinned Solana CLI syntax reviewed at release time, equivalent to:

```text
solana program set-upgrade-authority <FINAL_PROGRAM_ID> --final
```

Then verify:

```text
solana program show <FINAL_PROGRAM_ID>
```

Archive public evidence that no upgrade authority remains.

After this step the program cannot be upgraded or closed. There is no rollback procedure.

## Secrets policy

Never commit or upload Program ID keypair JSON, deployer private keys, seed phrases, recovery phrases or secret backups. Public release evidence contains only public keys, hashes, source, IDL, transaction signatures and non-secret verification data.
