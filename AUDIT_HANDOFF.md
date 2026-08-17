# Independent Audit Handoff — Service Referral Protocol

Status: **PRE-AUDIT / NOT YET A PRODUCTION SIGN-OFF**.

This handoff is intentionally prepared before the final Program ID and registration timestamp are frozen. The auditor must receive an exact immutable review package only after the remaining freeze fields below are completed.

## Review target

Repository: `PaulJourney/cartel1508`

Release branch / PR under hardening:

- branch: `agent/qualified-revenue-final-integration`
- PR: `#4`
- architecture: one Solana/Anchor program plus canonical SPL Token Program
- production economic event: `purchase_and_distribute`

The auditor must review the **exact final freeze commit**, not an earlier green development or Devnet commit.

## Frozen business/economic semantics

### Purchase and units

- 1 USDT/USDC = 1 logical unit.
- Purchases are unlimited subject to numeric/transaction limits.
- Each buyer-signed purchase transfers the full gross amount into the canonical vault and atomically performs all accounting.
- Purchase path is rentless: there is no persistent per-purchase `UnitBatch` PDA.

### SELF + nine uplines

- SELF / buyer: 50%.
- U1 / immutable direct sponsor: 15%.
- U2: 9%.
- U3: 6%.
- U4: 4%.
- U5: 2.5%.
- U6: 2%.
- U7: 1.5%.
- U8: 1%.
- U9: 2%.
- Pioneer pool: 2%.
- Service/platform: 5%.
- Total: 100% before deterministic integer rounding.
- No dynamic compression.
- No separate self-reentry genealogy position.

### Progressive activity / IC-A / weekly depth

- ACTIVE: 7 days.
- GRACE: 48 hours.
- Successful ACTIVE weeks 1–2 require 10 units each.
- Weeks 3–4 require 20 each.
- Weeks 5–6 require 30 each.
- Weeks 7–8 require 40 each.
- Week 9 onward requires 50, permanently capped at 50.
- Calendar inactivity does not advance the requirement; only successfully-started ACTIVE weeks do.
- Purchases while already ACTIVE increase `current_week_units` and current depth only; they never prequalify the next week.
- During GRACE/INACTIVE, partial purchases may accumulate toward the next ACTIVE requirement within one live seven-day qualification window.
- During an INACTIVE live partial qualification window, only buyer-own SELF is provisionally preserved; Pioneer and network receive no inactive partial-window exception.
- Claims are ACTIVE-only.
- INACTIVE scheduled/unclaimed value becomes treasury-destined under IC-A.
- An inactive upline's share is not reassigned to another ancestor.
- Stale value is settled before late reactivation.
- Weekly personal units unlock maximum monetizable depth prospectively: 10=>U3, 25=>U4, 50=>U5, 100=>U6, 200=>U7, 350=>U8, 500=>U9.
- ACTIVE/GRACE scheduled levels beyond unlocked depth route to Treasury/unallocated and are never recovered retroactively or compressed upward.
- GRACE retains the just-finished ACTIVE week's depth until reactivation/expiry settlement; the next ACTIVE week resets `current_week_units` to the new qualifying purchase volume.

## Frozen Pioneer position semantics

The old first-100-registration Pioneer model is retired.

- Exactly 100 Pioneer positions exist at most.
- Registration creates zero Pioneer positions.
- A single purchase creates `floor(units / 1000)` candidate positions.
- Purchases do not accumulate across transactions for Pioneer qualification.
- One wallet may own multiple positions.
- Actual assignment is capped by the remaining global capacity.
- 98/100 + a single 3,000-unit purchase creates exactly 2 positions.
- Once 100/100 is reached, every later purchase creates zero positions regardless of size.
- SELF rewards, network/downline rewards, Pioneer rewards, claims and wallet balances cannot directly create Pioneer positions.
- **Rule B:** the purchase that creates positions accounts for its own 2% before the new positions are assigned. New positions earn only from the next global purchase.
- Weighted per-wallet checkpoints/reward debt prevent retroactive rewards when one wallet acquires positions at different times.
- Every position is an equal virtual share of the fixed 2%/100 pool.
- Unassigned Pioneer shares go to Treasury and are never redistributed among existing positions.
- Positions themselves are permanent and never recycled.
- Pioneer economics are activity-gated: ACTIVE participates/claims, GRACE preserves eligible due, INACTIVE due expires to Treasury on settlement, and late reactivation cannot recover expired history.

## Primary source files

Audit at minimum:

- `programs/service_referral_protocol/src/lib.rs`
- `programs/service_referral_protocol/src/state.rs`
- `programs/service_referral_protocol/src/math.rs`
- `programs/service_referral_protocol/src/constants.rs`
- `programs/service_referral_protocol/Cargo.toml`
- root `Cargo.toml` / `Cargo.lock`
- `tests/reference-model.mjs`
- `integration-tests/**`
- `scripts/static-gates.py`
- `scripts/pre-mainnet-gate.py`
- `scripts/devnet-transaction-smoke.mjs`
- `offchain/rank-engine.mjs`
- `tests/rank-model.mjs`
- `release/mainnet-release.example.json`
- `.github/workflows/ci.yml`
- `.github/workflows/security-scan.yml`
- `.github/workflows/verifiable.yml`
- `.github/workflows/devnet-deploy-smoke.yml`
- `RANK_BADGE_SPEC.md`
- `PROGRAM_ID_CUSTODY_RUNBOOK.md`
- `MAINNET_FINALIZATION_RUNBOOK.md`
- the exact production transaction-builder/client source used to supply sponsor and upline accounts.

See `AUDIT_SCOPE.md` for the detailed review properties.

## Automated evidence expected on the exact audit-freeze commit

The handoff is incomplete unless the exact final freeze source has successful evidence for:

- reference economic model;
- rank/badge model isolation from payouts;
- static security/economic gates;
- fail-closed pre-mainnet proof before final release fields are populated;
- production compile;
- production SBF build;
- Rust unit/property tests;
- LiteSVM integration/adversarial suite;
- RustSec scan;
- verifiable build;
- `.so` SHA-256;
- retained successful real Devnet deploy + transaction-smoke evidence for the final ABI/economic surface.

Pioneer test evidence must explicitly cover:

- registration = 0 positions;
- 500 + 500 separate purchases = 0 positions;
- 1,000 single purchase = 1 position;
- multiple positions per wallet;
- Rule B;
- weighted checkpoints;
- 98/100 + 3,000 = 2 positions;
- 100/100 saturation prevents position 101;
- inactivity expires eligible Pioneer economics without deleting the permanent position;
- later reactivation does not recover expired Pioneer history.

Progressive activity/depth evidence must explicitly cover:

- `10,10,20,20,30,30,40,40,50...` successful ACTIVE-week requirements;
- long calendar inactivity does not advance the requirement;
- partial requalification below threshold remains INACTIVE;
- purchases while ACTIVE raise current depth without starting another week;
- 10 personal weekly units monetize only U1–U3;
- 500 personal weekly units monetize U1–U9;
- unlocking is prospective and locked levels route Treasury/unallocated.

## Real Devnet evidence — PASSED

The maintained Devnet smoke successfully deployed and executed the final ABI/economic transaction path on source head `866e724e5ae57ec9eb20f641d9272ce508554a6a`.

- GitHub Actions run: `32039983574` — **PASS**;
- temporary Devnet Program ID: `9EWUPLXeyTJhW3idnWFUDP9xfBUAensW42kLYKiG55oM`;
- deployment transaction: `643FxLkJ1mtKtd8Kh87QRFu481EC5nsWF2h76ctHjE47GFsrgLw3fL8QxWWnYQFJ1qs8iCYdLFF4kLtSf2CBcBLP`;
- evidence artifact ID: `9291863971`;
- artifact digest: `sha256:1c63cecb2df7330d3ead7ec0ee872d828c595f138599c16ee9aeef9b524b7254`;
- smoke sentinel: `DEVNET TRANSACTION SMOKE: PASS`;
- Pioneer assignment reached exactly 100/100 and could not create position 101;
- final vault atomic balance was `0`;
- final sponsor atomic amount: `15920000000`;
- final buyer atomic amount: `53208800000`;
- final Treasury atomic amount: `36981200000`.

Subsequent repository cleanup only removed the temporary funding trigger and restored the normal manual-only Devnet workflow; protocol source was unchanged. The disposable Devnet Program ID and deployer must never be treated as production identities.

## Final freeze fields that must be supplied before audit sign-off

The final review package must record:

- exact final git commit SHA;
- final public Program ID generated offline;
- frozen future `MAINNET_REGISTRATION_OPEN_AT` UTC timestamp;
- exact `Cargo.lock`;
- final IDL;
- exact production/verifiable `.so` SHA-256;
- exact protocol-CI run URL;
- exact RustSec run URL;
- exact verified-build run URL;
- successful Devnet-smoke run URL and evidence digest;
- exact transaction-builder source/version.

No Program ID private key, deployer private key, seed phrase or recovery material belongs in this package.

## Audit output requirements

For every finding record:

- severity;
- affected file/function;
- exploit/precondition;
- economic/security impact;
- recommended remediation;
- project disposition: fixed / accepted / rejected with rationale;
- remediation commit SHA where applicable;
- re-test evidence.

The final audit report itself must be hashed with SHA-256 and that digest must be placed in `release/mainnet-release.json` with `audit_status: "passed"` only after all accepted findings are resolved.

## Immutability gate

Audit approval is not permission to immediately remove upgrade authority.

The sequence remains:

1. audit exact artifact;
2. complete release manifest and executable gate;
3. controlled mainnet deploy with temporary upgrade authority;
4. verify deployed bytecode against audited `.so`;
5. initialize with frozen inputs;
6. deliberately small mainnet smoke;
7. verify balances/state/bytecode again;
8. only then permanently remove upgrade authority.

After permanent authority removal there is no upgrade or rollback path.

## New frozen behavior for review

Review progressive ACTIVE-week qualification, prospective weekly U1-U9 depth, Treasury routing of locked levels, strict Pioneer ACTIVE/GRACE gating and event/indexer determinism. `RANK_BADGE_SPEC.md` is an indexer/UI specification only and must not be treated as an additional on-chain payout surface.
