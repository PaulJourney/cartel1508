# Independent Audit Handoff — Service Referral Protocol

Status: **PRE-AUDIT / NOT A PRODUCTION SIGN-OFF**.

This handoff is prepared before the final Mainnet Program ID and registration timestamp are frozen. The independent auditor must review the **exact final freeze commit and exact final production artifact**, not an earlier green development/Devnet candidate.

## Review target

Repository: `PaulJourney/cartel1508`

- branch: `agent/qualified-revenue-final-integration`
- PR: `#4` (draft until release gates are complete)
- production architecture: one Solana/Anchor program plus canonical SPL Token Program
- sole production economic event: buyer-signed `purchase_and_distribute`

There is no production Revenue Verifier / Revenue Adapter / qualified-revenue chain in the final architecture.

## Frozen economics

- 1 USDT/USDC = 1 logical unit.
- SELF/buyer: 50%.
- U1 direct sponsor: 15%.
- U2: 9%.
- U3: 6%.
- U4: 4%.
- U5: 2.5%.
- U6: 2%.
- U7: 1.5%.
- U8: 1%.
- U9: 2%.
- Pioneer: 2%.
- Service: 5%.
- Total: 100% before deterministic integer handling.
- No dynamic compression.
- No separate self-reentry genealogy position.
- Purchases are rentless from the protocol perspective: no persistent per-purchase PDA.

## Progressive activity / depth / IC-A

- ACTIVE: 7 days.
- GRACE: 48 hours.
- successful ACTIVE weeks 1–2: 10 units each;
- weeks 3–4: 20;
- weeks 5–6: 30;
- weeks 7–8: 40;
- week 9 onward: 50 cap.
- Calendar inactivity does not advance the requirement.
- Purchases while ACTIVE increase `current_week_units`/depth only and never prequalify the next week.
- GRACE/INACTIVE partial purchases accumulate toward the current next requirement inside one seven-day window.
- During an INACTIVE live partial window, **SELF only** is provisional. Pioneer/network have no partial exception.
- Claims are ACTIVE-only.
- INACTIVE unclaimed SELF/network/Pioneer value becomes permanently Treasury-destined.
- Stale value is settled before late reactivation.

Each upline's own weekly personal units unlock its maximum monetizable level prospectively:

- 10 → U1–U3
- 25 → U1–U4
- 50 → U1–U5
- 100 → U1–U6
- 200 → U1–U7
- 350 → U1–U8
- 500+ → U1–U9

Locked ACTIVE/GRACE levels route to Treasury/unallocated. INACTIVE levels route to Treasury/expired. Neither is compressed or recovered retroactively.

## Pioneer semantics

- absolute cap: 100 positions;
- registration: zero positions;
- one purchase creates `floor(units / 1000)` candidates;
- separate purchases do not accumulate (`500 + 500 = 0`);
- one wallet may own multiple/all positions;
- assignment is capped by remaining capacity;
- 98/100 + one 3,000-unit purchase = exactly 2 positions;
- 100/100 permanently blocks position 101;
- **Rule B:** creating purchase accrues Pioneer using only pre-existing positions; new positions earn from the next purchase;
- weighted high-precision checkpoints prevent retroactive entitlement;
- unassigned virtual-slot value goes to Treasury;
- positions are permanent, but INACTIVE unclaimed due expires and cannot be rescued by reactivation.

## Primary audit source

Review at minimum:

- `programs/service_referral_protocol/src/{lib,state,math,constants}.rs`
- `programs/service_referral_protocol/Cargo.toml`
- root `Cargo.toml` / `Cargo.lock`
- `tests/reference-model.mjs`
- `tests/rank-model.mjs`
- `integration-tests/**`
- `scripts/static-gates.py`
- `scripts/pre-mainnet-gate.py`
- `scripts/devnet-comprehensive-validation.mjs`
- `scripts/devnet-security-adversarial.mjs`
- `scripts/devnet-rpc-guard.mjs`
- `offchain/rank-engine.mjs` only for rank/indexer isolation review
- `.github/workflows/ci.yml`
- `.github/workflows/security-scan.yml`
- `.github/workflows/verifiable.yml`
- `.github/workflows/local-pre-mainnet-validation.yml`
- `.github/workflows/devnet-deploy-smoke.yml`
- `release/mainnet-release.example.json`
- `README.md`
- `SECURITY.md`
- `AUDIT_SCOPE.md`
- `RANK_BADGE_SPEC.md`
- `PROGRAM_ID_CUSTODY_RUNBOOK.md`
- `MAINNET_FINALIZATION_RUNBOOK.md`
- exact production transaction-builder/client source.

Historical migration scripts are outside runtime. Dead Revenue Adapter release tooling is removed rather than treated as a dependency.

## Deterministic / LiteSVM evidence expected

The exact audited freeze must preserve green evidence for:

- reference conservation model;
- rank isolation from payouts;
- static security/economic gates;
- production compile;
- Rust unit/property tests;
- exact ACTIVE/GRACE/INACTIVE boundary tests;
- progressive `10,10,20,20,30,30,40,40,50...` ladder;
- no calendar progression;
- full U1–U9 depth thresholds and tenth-ancestor exclusion;
- prospective locked-level Treasury routing;
- batching equivalence;
- Pioneer zero-on-registration, non-cumulative threshold, Rule B, weighted checkpoints and hard 100 cap;
- Pioneer inactivity expiry without position deletion;
- false ancestry rollback;
- initialization duplicate/non-six-decimal mint rejection;
- immediate double-claim/replay rejection;
- failed payment CPI **after expiry settlement begins** rolls the entire transaction back and leaves the entitlement settleable exactly once.

## Isolated Solana-validator evidence

The comprehensive local runtime is not a mock-only model: it builds/deploys the program to `solana-test-validator` and uses signed transactions, real SPL Token accounts and PDAs.

A completed validation run on exact source `87affea4348c7d62017695684e388b32fedd9dba` proved the newly added runtime-security suite in addition to the comprehensive economics:

### Comprehensive phase

- **20 users**;
- **108,595 logical units**;
- U1–U9 depth boundary scenarios;
- 1×10 vs 10×1 batching;
- independent USDT/USDC paths;
- adversarial zero/overflow/ancestry/vault/mint/authority rejection;
- Pioneer non-cumulative acquisition, weighted checkpoints, Rule B and 98→100 cap;
- final `nextUnitId = 108596`;
- final Pioneer assigned = `100`;
- comprehensive-phase USDT vault = `0`;
- comprehensive-phase USDC vault = `0`.

### Runtime-security add-on

The same deployment then proved:

- same-wallet re-registration rejected;
- spoofed referrer wallet/PDA rejected;
- structural self-referral rejected;
- non-canonical Service-Treasury ATA rejected without moving value or advancing Unit/purchase indices;
- valid 10-unit purchase succeeds after the failed substitution;
- claim hijack using another wallet's `UserState` rejected;
- non-canonical claim destination rejected;
- valid buyer claim pays exactly 5 USDC SELF;
- immediate double-claim/replay rejected;
- fake Treasury receives `0`;
- only two valid security registrations changed `realUserCount` (`20 -> 22`);
- deliberate security add-on residual vault liability = exactly `200000` USDC atoms (0.2 USDC), independently expected as Pioneer liability from the added post-saturation purchase.

Run: `32053522367` — **PASS**.
Evidence artifact: `9295679641`.
Artifact package digest: `sha256:d4dfa650fb44b194e1349cf070edfbf1c77152768223b459a02b739afeaf3f7d`.

The 0.2-USDC final residual in this add-on is expected and asserted. It does not contradict the comprehensive-phase zero-vault invariant; it is liability created by a subsequent test purchase after the original Pioneer-owner keys have left the first Node process.

## Public Solana Devnet status

### Historical smoke — PASS, supporting evidence only

A real public-Devnet smoke on source-equivalent core head `866e724e5ae57ec9eb20f641d9272ce508554a6a` successfully deployed and exercised the core ABI/economics:

- run `32039983574` — PASS;
- temporary Program ID `9EWUPLXeyTJhW3idnWFUDP9xfBUAensW42kLYKiG55oM`;
- deployment transaction `643FxLkJ1mtKtd8Kh87QRFu481EC5nsWF2h76ctHjE47GFsrgLw3fL8QxWWnYQFJ1qs8iCYdLFF4kLtSf2CBcBLP`;
- evidence artifact `9291863971`;
- artifact digest `sha256:1c63cecb2df7330d3ead7ec0ee872d828c595f138599c16ee9aeef9b524b7254`;
- Pioneer reached 100/100 and position 101 was blocked;
- final smoke vault = 0.

This remains useful history but is **not the final public-Devnet release gate**.

### Final public comprehensive — PENDING / release blocker

The maintained public workflow must run once on the final marker SHA after all free/local hardening is green. That run must execute both:

1. `devnet:pre-mainnet` comprehensive validation;
2. `devnet:security` runtime adversarial validation.

The workflow records exact source SHA, allows only temporary Program-ID substitution in `Anchor.toml` and `lib.rs`, records the temporary `.so` hash, Program ID, deployment evidence and both validation outputs, and never publishes the private deployer key.

The final public Devnet source SHA, run URL and evidence digest must be recorded in the release manifest. A faucet/RPC failure before successful execution is not protocol evidence and does not satisfy the gate.

## Final production-runtime evidence — PENDING / release blocker

After the final Mainnet Program ID and future registration timestamp are frozen, the exact `production` artifact must receive a runtime validation bound to the **final release source SHA**. This is separate from development-equivalent local/Devnet evidence because production initialization pins the final Treasury/mints/timestamp.

The release manifest requires:

- `production_runtime_source_sha` equal to final `commit_sha`;
- successful run URL;
- evidence SHA-256;
- `production_runtime_status: "passed"`.

## Final freeze fields required before auditor sign-off

The final handoff package must record:

- exact final git SHA;
- final public Program ID generated offline;
- future `MAINNET_REGISTRATION_OPEN_AT` UTC timestamp;
- exact `Cargo.lock`;
- final IDL;
- final production/verifiable `.so` SHA-256;
- protocol-CI run URL;
- RustSec run URL;
- verified-build run URL;
- final public-Devnet comprehensive source SHA/run/evidence digest;
- final production-runtime source SHA/run/evidence digest;
- exact transaction-builder source/version.

No private key, seed phrase or recovery material belongs in this package.

## Audit output requirements

For every finding record:

- severity;
- affected file/function;
- exploit/precondition;
- economic/security impact;
- recommended remediation;
- disposition: fixed / accepted / rejected with rationale;
- remediation commit SHA;
- re-test evidence.

Hash the final independent audit report with SHA-256. Set `audit_status: "passed"` in the release manifest only after accepted findings are resolved/retested.

## Mainnet / immutability sequence

Audit approval is not permission to immediately make the program immutable.

Required order:

1. audit exact frozen artifact;
2. complete release manifest and executable gate;
3. deploy Mainnet with temporary upgrade authority;
4. verify deployed bytecode equals audited `.so` **before initialization**;
5. initialize only frozen values;
6. create if absent and verify canonical USDT/USDC ATAs for vault-authority PDA and Service Treasury;
7. deliberately small Mainnet smoke that does not consume Pioneer positions;
8. reverify balances/state/bytecode;
9. only then permanently remove upgrade authority.

After permanent authority removal, there is no software rollback path.
