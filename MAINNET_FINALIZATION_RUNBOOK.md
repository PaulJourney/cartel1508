# Mainnet Finalization Runbook

This procedure is intentionally conservative. **Permanent immutability is the final step, never the deployment step.**

## 1. Freeze the final core surface

Before Mainnet:

- `purchase_and_distribute` is the sole economic entrypoint;
- obsolete Revenue Adapter / Evidence / Qualification production paths are absent;
- SELF + 9 uplines and 50/43/2/5 constants are frozen;
- progressive ACTIVE ladder is frozen to `10,10,20,20,30,30,40,40,50...`;
- weekly depth is frozen to 10→U3, 25→U4, 50→U5, 100→U6, 200→U7, 350→U8, 500→U9;
- depth remains prospective with no compression;
- Pioneer is frozen to 100 purchase-earned positions, 1,000-unit single-purchase threshold, Rule B, weighted checkpoints and ACTIVE-gated economics;
- registration assigns zero Pioneer positions;
- Service Treasury and canonical Mainnet USDT/USDC are frozen;
- no mutable owner/admin payout path remains;
- rank/badge remains off-chain and cannot affect payouts.

## 2. Close final public Devnet gate

Only after all local/CI hardening is green, create the final marker-only commit that starts the maintained public Devnet workflow.

The **same marker SHA** must receive:

- protocol CI PASS;
- RustSec PASS;
- verifiable-build PASS;
- isolated comprehensive + runtime-security PASS;
- public Devnet comprehensive + runtime-security PASS.

The public workflow must:

- record exact source SHA;
- generate disposable test identities only;
- substitute the temporary Program ID only in `Anchor.toml` and `lib.rs`;
- fail if another tracked source file changes during substitution;
- record temporary `.so` hash, Program ID, deploy output and public deployer address;
- execute `devnet:pre-mainnet`;
- execute `devnet:security`;
- publish no private key.

Record marker SHA, run URL and evidence-artifact SHA-256. A faucet/RPC failure is neither a protocol failure nor passing evidence; repeat only when a real execution can complete.

## 3. Generate/freeze final Program ID offline

Under `PROGRAM_ID_CUSTODY_RUNBOOK.md`:

1. generate final Service Referral Protocol Program ID keypair **offline**;
2. keep private key offline;
3. commit only the public Program ID in `declare_id!`;
4. add matching `[programs.mainnet]` entry in `Anchor.toml`.

Never paste the Program ID private key into GitHub, CI or chat.

## 4. Freeze future registration opening

Choose an explicit future UTC Unix timestamp and set `MAINNET_REGISTRATION_OPEN_AT`.

The final production initialization must pin:

- Service Treasury `AepYo8xanmKuRiLVeYQuCTJoQr1nyKiTApoKwHMEg8fn`;
- USDT `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB`;
- USDC `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`;
- exact frozen registration timestamp.

Any Program ID/timestamp/config/dependency change creates a new release SHA and invalidates prior final production evidence.

## 5. Build/test exact freeze commit

From a clean exact-head checkout:

- reference economics PASS;
- rank isolation model PASS;
- static gates PASS;
- locked Rust tests PASS;
- full LiteSVM time/adversarial suite PASS;
- RustSec PASS on exact head;
- production compile/build PASS;
- independent verifiable build PASS;
- production `.so` from both paths byte-identical;
- final `.so` SHA-256 recorded;
- final IDL, `Cargo.lock`, source SHA and toolchain recorded.

The suite must still include double-claim rejection and complete CPI rollback when payment fails after expiry settlement has begun.

## 6. Final production-runtime validation

The exact final `production` artifact must be executed in a production-compatible local/runtime environment **without adding a test bypass to the program**.

Purpose: prove the artifact whose production initialization pins final Treasury/mints/timestamp can actually initialize and execute the intended runtime path.

Required evidence:

- source SHA equals final release `commit_sha`;
- exact production `.so` hash matches release artifact;
- initialization uses frozen Treasury, canonical Mainnet mint addresses and frozen timestamp;
- runtime accounts/mints needed for local validation are prepared at the canonical public addresses without altering the program logic;
- canonical vault/Treasury ATA validation succeeds;
- controlled purchase/claim path succeeds;
- invalid canonical-account substitutions remain rejected;
- evidence artifact SHA-256 recorded.

Release manifest must contain:

- `production_runtime_source_sha` = final `commit_sha`;
- production-runtime Actions run URL;
- evidence SHA-256;
- `production_runtime_status: "passed"`.

## 7. Independent audit

Give the auditor:

- exact final source SHA;
- final public Program ID;
- frozen registration UTC;
- exact dependency lock;
- final IDL;
- exact production/verifiable `.so` SHA-256;
- exact protocol-CI, RustSec and verifiable evidence;
- final public-Devnet comprehensive/security evidence;
- final production-runtime evidence;
- `AUDIT_SCOPE.md`, `AUDIT_HANDOFF.md`, `FINAL_SPEC_GAP_REVIEW.md`, `PRE_MAINNET_DEFINITIVE_VALIDATION.md`, `SECURITY.md`;
- exact production transaction-builder/client source.

Audit at minimum:

- signer/PDA/ATA integrity;
- immutable ancestry and SELF + U1–U9 traversal;
- 50/43/2/5 conservation;
- progressive ACTIVE and partial-window behavior;
- weekly depth and Treasury routing;
- ACTIVE/GRACE/INACTIVE expiry;
- Pioneer acquisition/cap/Rule B/checkpoints/inactivity;
- claim authorization/replay;
- expiry settlement;
- Unit-ID overflow/continuity;
- failed-CPI atomic rollback;
- absence of admin mutation paths;
- production transaction-builder account construction;
- event integrity for rank/indexing without rank affecting payouts.

Every accepted finding must be resolved/retested. Material changes require new artifact evidence and appropriate renewed audit review.

## 8. Complete release manifest / executable gate

Create `release/mainnet-release.json` from the example and populate:

- final `commit_sha`;
- Program ID;
- registration-open timestamp;
- Treasury and mints;
- economics/activity/depth/Pioneer fingerprints;
- final `.so` SHA-256;
- audit-report SHA-256;
- protocol-CI URL;
- RustSec URL;
- verified-build URL;
- `devnet_comprehensive_source_sha`;
- final Devnet comprehensive run URL;
- Devnet evidence SHA-256;
- `devnet_comprehensive_status: "passed"`;
- `production_runtime_source_sha` equal to final commit;
- production-runtime run URL;
- production-runtime evidence SHA-256;
- `production_runtime_status: "passed"`;
- `audit_status: "passed"`;
- `smoke_test_plan_approved: true`.

Then run:

```text
python3 scripts/pre-mainnet-gate.py
```

Any non-zero exit is a **hard stop**. Never weaken the gate for deployment convenience.

## 9. Measure/fund temporary Mainnet deployer

Only after final artifact/audit/gate are complete:

1. measure exact `.so` size;
2. query current Solana Mainnet deployment/rent/fee requirements using current official tooling;
3. calculate required SOL plus a small explicit margin;
4. fund only the temporary deployer/upgrade authority needed for controlled deployment;
5. keep deployment SOL operationally separate from USDT/USDC protocol economics.

## 10. Controlled Mainnet deployment

Deploy the exact audited `.so` with:

- offline final Program ID keypair;
- separate temporary deployer/upgrade-authority keypair.

Keep upgrade authority temporarily.

Record Program ID, ProgramData address, deployment slot/signature, authority, source SHA and local artifact hash.

## 11. Verify deployed bytecode **before initialization**

Dump/read the deployed program using current pinned official tooling and compare against the exact audited production artifact/hash.

Verify:

- Program ID matches frozen public ID;
- ProgramData address is expected;
- `[programs.mainnet]` mapping matches;
- IDL matches final source;
- Treasury/mints/timestamp/economics/activity/depth/Pioneer source constants match release manifest.

If any byte or identity check fails: **do not initialize**.

## 12. Initialize frozen state

Initialize only with frozen values:

- Service Treasury;
- canonical Mainnet USDT;
- canonical Mainnet USDC;
- future registration-open timestamp.

Immediately verify:

- protocol PDA;
- vault-authority PDA;
- technical-root PDA;
- stored Treasury/mints/timestamp;
- Pioneer assigned count starts at zero;
- `real_user_count` starts at zero;
- `next_unit_id` starts at one.

## 13. Bootstrap and verify canonical token ATAs

**This step is mandatory before the first economic smoke.** The protocol validates canonical accounts but does not create these four ATAs inside `purchase_and_distribute`.

Create **if absent**, using standard permissionless Associated Token Account creation:

1. vault-authority PDA + USDT mint;
2. vault-authority PDA + USDC mint;
3. frozen Service Treasury + USDT mint;
4. frozen Service Treasury + USDC mint.

For each ATA verify on-chain:

- address equals canonical ATA derivation;
- token program is canonical SPL Token Program expected by the release;
- mint equals expected USDT/USDC;
- owner equals expected vault-authority PDA or frozen Treasury;
- initial token amount is recorded and understood before smoke.

Do not proceed if an existing account at a required address has unexpected program owner, mint, token-account owner or malformed data.

## 14. Deliberately small Mainnet smoke

Use controlled wallets and the smallest useful economic amounts.

Verify at minimum:

- intended registration relationship;
- one small supported-stablecoin purchase;
- exact Unit count / Unit ID;
- buyer-paid transaction;
- ACTIVE-week/qualification/depth state;
- exact vault/Treasury deltas;
- exact SELF/network/Pioneer accounting for the current state;
- separate ACTIVE pull claim;
- claimant-paid transaction fee;
- conservation and absence of unexpected accounts/state.

**Do not use 1,000+ units merely to retest Pioneer acquisition**, because that would irreversibly consume production Pioneer capacity. Do not build an artificial deep Mainnet genealogy just to repeat U1–U9 tests already proven in audited evidence.

Stop immediately on any Program ID, bytecode, PDA, ancestry, balance or accounting mismatch.

## 15. Reverify after smoke

Before immutability:

- dump/verify bytecode again against audited `.so`;
- verify ProgramData/upgrade authority;
- verify protocol/technical-root state;
- verify canonical ATA owner/mint/program data;
- verify smoke transaction signatures and expected balances;
- verify no unexpected state changes.

## 16. Permanent immutability — LAST

Only after written sign-off of deploy verification + initialization + ATA bootstrap + smoke + re-verification, remove upgrade authority using the **current official Solana CLI syntax verified at release time**.

Then independently verify on-chain that no upgrade authority remains and archive public evidence.

After this step the program cannot be upgraded or closed. There is no software rollback procedure.

## Secrets policy

Never commit/upload Program ID keypair JSON, deployer/upgrade-authority private keys, seed phrases, recovery phrases or secret backups. Public release evidence contains only public keys, hashes, source, IDL, transaction signatures and non-secret verification data.
