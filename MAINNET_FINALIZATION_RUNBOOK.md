# Mainnet Finalization Runbook

This procedure is intentionally conservative. **Permanent immutability is the final step, never the deployment step.**

## 1. Freeze the final core surface

Before any mainnet deployment:

- `purchase_and_distribute` must be the sole production economic entrypoint;
- all obsolete qualified-revenue source/adapter/evidence/qualification state and instructions must be removed from the final core;
- final **SELF + 9 uplines** and 50/43/2/5 constants must be frozen;
- progressive ACTIVE requirements must be frozen to weeks 1–2=10, 3–4=20, 5–6=30, 7–8=40 and week 9+=50 units, with inactivity never advancing the counter;
- weekly network depth must be frozen to 10=>U3, 25=>U4, 50=>U5, 100=>U6, 200=>U7, 350=>U8, 500=>U9, prospective only and no compression;
- Pioneer must be frozen to the purchase-earned model: 100 total positions, 1,000-unit single-purchase threshold, multiple positions per wallet, absolute cap, Rule B and ACTIVE-gated economics with permanent positions;
- registration must assign zero Pioneer positions;
- service treasury and canonical USDT/USDC mints must be frozen;
- registration-opening UTC must be frozen;
- no mutable owner/admin instruction may remain.

Rank/badge logic is explicitly outside the payout core in V1 and must not introduce any extra economic path or alter 50/43/2/5.

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

The final automated suite must include progressive activity/depth evidence for at least:

- exact ACTIVE-week requirements `10,10,20,20,30,30,40,40,50...`;
- long calendar inactivity does not increment `active_weeks_started`;
- partial requalification below the required units stays INACTIVE;
- purchases while already ACTIVE increase `current_week_units` but do not prequalify the next week;
- 10 personal weekly units unlock only U1–U3;
- 500 personal weekly units unlock U1–U9;
- depth increases are prospective and never recover earlier locked levels;
- locked network-level shares route to Treasury/unallocated without compression.

The final automated suite must also include Pioneer evidence for at least:

- registration = 0 positions;
- separate `500 + 500` purchases = 0 positions;
- one 1,000-unit purchase = 1 position;
- one wallet can hold multiple positions;
- Rule B excludes newly created positions from the creating purchase;
- 98/100 + 3,000 units = exactly 2 final positions;
- 100/100 permanently blocks additional positions;
- weighted checkpoints prevent retroactive Pioneer rewards;
- Pioneer position ownership survives inactivity;
- Pioneer due while INACTIVE expires to Treasury and is not recovered after reactivation.

Any source/configuration/dependency change after this point invalidates the artifact hash and requires the gates to be rerun.

## 4. Devnet smoke on the final interface

Before independent final sign-off, deploy a temporary/dev identity and exercise the same final instruction surface:

1. initialize;
2. register a controlled genealogy and verify registration itself creates zero Pioneer positions;
3. purchase units using USDC/USDT-compatible six-decimal test mints;
4. verify global Unit IDs, `active_weeks_started`, `current_week_units`, qualification progress and ACTIVE/GRACE state;
5. verify 50% SELF / 43% fixed network schedule / 2% Pioneer / 5% service conservation;
6. verify at least one locked-depth network share routes to Treasury/unallocated and a later prospectively-unlocked level is paid correctly;
7. exercise the purchase-earned Pioneer path, including Rule B and saturation behavior;
8. verify Pioneer inactive-expiry/reactivation behavior if the smoke environment can advance time safely; otherwise retain exact LiteSVM evidence for this time-dependent invariant;
9. verify canonical vault collateral;
10. claim from an ACTIVE beneficiary using a separate claimant-signed transaction;
11. verify final token conservation.

The maintained `scripts/devnet-transaction-smoke.mjs` includes the real-transaction Pioneer saturation sequence reaching 98 positions, capping a later 3,000-unit purchase to the final two positions, then verifying that a subsequent 5,000-unit purchase cannot create position 101. Before release, its successful run must correspond to the final ABI and activity/depth model.

A faucet/rate-limit failure **before deployment is not a protocol smoke failure and is not passing evidence**. The release remains blocked until a real devnet deploy and transaction sequence completes successfully.

Record the successful GitHub Actions run URL and evidence-artifact digest in the final release manifest.

Devnet evidence is workflow evidence only; it is not a substitute for mainnet bytecode verification or independent audit.

## 5. Independent audit

Give the auditor:

- exact source commit;
- final public Program ID;
- final constants and registration UTC;
- exact dependency lockfile;
- final IDL;
- exact production/verifiable artifact hash;
- exact protocol-CI, RustSec, verifiable-build and successful devnet-smoke evidence;
- `FINAL_SPEC_GAP_REVIEW.md`, `RANK_BADGE_SPEC.md`, `AUDIT_SCOPE.md` and `AUDIT_HANDOFF.md`;
- client transaction-builder source used to derive sponsor/upline accounts.

Audit at minimum:

- purchase payment amount and token-account validation;
- immutable referral ancestry and SELF + nine-upline traversal;
- 50/43/2/5 conservation and rounding;
- progressive ACTIVE-week counter/requirements and stale partial qualification;
- weekly U1–U9 depth boundaries, prospective unlock and Treasury routing of locked levels;
- ACTIVE/GRACE/INACTIVE behavior;
- Pioneer 1,000-unit per-purchase qualification;
- Pioneer absolute 100-position cap and 98->100 boundary;
- Pioneer Rule B and weighted checkpoint/reward-debt arithmetic;
- Pioneer ACTIVE-only expiry/reactivation with permanent position ownership;
- Pioneer fractional carry and unassigned-share treasury routing;
- vault collateral and treasury routing;
- pull claim authorization;
- expiry settlement;
- unit-ID overflow/continuity;
- rollback atomicity;
- absence of mutable admin/control paths;
- client construction of all required accounts;
- event sufficiency/integrity for deterministic off-chain rank indexing, without rank affecting core payouts.

Resolve every accepted finding. Any material code/configuration/dependency change requires a new exact artifact and appropriate renewed audit review.

## 6. Complete release evidence

Create `release/mainnet-release.json` from the example and populate:

- exact audited source commit SHA;
- final Program ID;
- registration-opening UTC;
- service treasury;
- USDT mint;
- USDC mint;
- frozen economics, progressive activity, depth and Pioneer fingerprint fields;
- final production `.so` SHA-256;
- independent audit-report SHA-256;
- exact protocol-CI run URL;
- exact RustSec run URL;
- exact verified-build run URL;
- successful devnet-smoke run URL and evidence SHA-256;
- `devnet_smoke_status: "passed"`;
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
- service treasury and canonical stablecoin constants match the release manifest;
- frozen activity/depth/Pioneer constants and source shape match the audited release.

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
- one small canonical-stablecoin unit purchase;
- exact unit count / Unit ID assignment;
- buyer-paid purchase transaction;
- expected `active_weeks_started`, `current_week_units`, depth and qualification state;
- expected vault and treasury movement;
- expected SELF/network/Pioneer accounting for the current qualification and pre-existing Pioneer state;
- separate ACTIVE pull claim signed by the beneficiary;
- claimant-paid transaction fee;
- final token conservation;
- no unexpected state/account changes.

Do **not** use a large 1,000+ unit mainnet purchase merely to manufacture a Pioneer test if doing so would consume production Pioneer capacity. Pioneer threshold/cap/Rule B behavior is proven in the exact audited LiteSVM/devnet evidence; the mainnet smoke should minimize irreversible production-state impact.

Likewise, do not manufacture deep production genealogy merely to reproduce every U1–U9 depth boundary. Those boundaries must already be proven in exact audited reference/LiteSVM/devnet evidence.

Run an intentionally invalid ancestry/account test only if it can be done without risking production state; otherwise rely on the exact audited adversarial evidence.

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
