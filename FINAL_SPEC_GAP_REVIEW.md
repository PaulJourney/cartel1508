# Final Specification Resolution / Gap Review

Status: **economic specification resolved; Mainnet remains fail-closed**.

The economic model is frozen and repeatedly validated in deterministic, LiteSVM and isolated Solana-runtime tests. A historical public-Devnet smoke has passed, but the **final public-Devnet comprehensive + runtime-security run remains a release blocker**. Final Program ID/timestamp freeze, production-runtime validation and independent audit also remain open.

## Frozen economics

- 1 USDT/USDC = 1 logical unit.
- Buyer-signed `purchase_and_distribute` is the sole production economic event.
- SELF / buyer: 50%.
- Nine immutable uplines: `15 / 9 / 6 / 4 / 2.5 / 2 / 1.5 / 1 / 2%` = 43%.
- Pioneer pool: 2%.
- Service: 5%.
- Total: **100%** before deterministic integer handling.
- No dynamic compression.
- No separate self-reentry genealogy position.
- Production release is one Solana/Anchor program plus the canonical SPL Token Program.

## Progressive ACTIVE-week rule — resolved

Requirement advances only when a wallet successfully starts a new ACTIVE week:

- weeks 1–2: 10 units;
- weeks 3–4: 20;
- weeks 5–6: 30;
- weeks 7–8: 40;
- week 9 onward: 50 cap.

ACTIVE lasts seven days; GRACE lasts 48 hours. Calendar inactivity never advances the ladder. Purchases while already ACTIVE raise only `current_week_units`/depth and do not prequalify another week.

During GRACE/INACTIVE, purchases may accumulate toward the current next requirement inside one live seven-day window. Stale partial progress resets. During a live INACTIVE partial-qualification window, **SELF only** is provisionally preserved; Pioneer/network receive no partial exception.

Once INACTIVE, whole-atomic unclaimed SELF/network/Pioneer value is Treasury-destined. Stale value is settled before late reactivation so expired history cannot be rescued.

## Weekly network depth — resolved

Each upline's own personal units in the current ACTIVE week unlock maximum monetizable depth:

- 10 → U1–U3
- 25 → U1–U4
- 50 → U1–U5
- 100 → U1–U6
- 200 → U1–U7
- 350 → U1–U8
- 500+ → U1–U9

Unlock is prospective only. ACTIVE/GRACE scheduled levels beyond depth go to Treasury/unallocated. INACTIVE scheduled levels go to Treasury/expired. No share is compressed to another ancestor.

## Pioneer positions — resolved

The old first-100-registration Pioneer model is retired.

- hard global cap: 100 positions;
- registration: zero positions;
- one purchase creates `floor(units / 1000)` candidates;
- separate purchases never accumulate toward the threshold;
- one wallet may own multiple/all positions;
- assignment is capped by remaining capacity;
- 98/100 + 3,000 units = exactly 2 final positions;
- 100/100 blocks every later position;
- only gross buyer-signed purchase units create positions;
- **Rule B:** creating purchase accrues Pioneer before its new positions are assigned;
- weighted checkpoints prevent retroactive entitlement;
- unassigned virtual-slot shares go to Treasury;
- positions are permanent, but INACTIVE unclaimed Pioneer due expires and cannot be recovered after reactivation.

## Rank / badge V1 — resolved as non-payout metadata

Rank remains off-chain. `UserRegistered` and `UnitsPurchased` events allow deterministic indexing of genealogy and purchase volume.

`RANK_BADGE_SPEC.md` defines QNV, qualified legs, balance caps, Current Rank, Highest Lifetime Rank and Pioneer badge display. **Ranks never alter the frozen 50/43/2/5 payout core in V1.**

## Self-reentry — retired

One wallet has one immutable genealogy node. Repeated purchases are additional units of the same user. The intended buyer economic participation is represented by the 50% SELF bucket, not by synthetic self-sponsored genealogy positions.

Multiple Pioneer positions are separate global-pool ownership and do not create genealogy nodes.

## Inactive compression — retired / IC-A frozen

- ACTIVE + sufficient depth: scheduled level claimable;
- GRACE + sufficient retained depth: scheduled level preserved;
- ACTIVE/GRACE + insufficient depth: Treasury/unallocated;
- INACTIVE: Treasury/expired;
- higher ancestors retain only their own fixed percentages.

## Validation state

### Deterministic / LiteSVM — PASS

Coverage includes:

- split conservation and large numeric boundaries;
- exact ACTIVE/GRACE/INACTIVE time boundaries;
- progressive ACTIVE ladder and no calendar progression;
- full U1–U9 depth boundaries and prospective unlocking;
- batching equivalence;
- false ancestry rollback;
- Pioneer Rule B, weighted checkpoints, inactivity and 100/100 cap;
- double-claim rejection;
- failed payment CPI after expiry settlement begins rolls the entire transaction back atomically.

### Isolated Solana runtime — PASS

The program is built/deployed to `solana-test-validator` and exercised using signed transactions, real SPL Token accounts and PDAs.

The comprehensive phase has repeatedly proven:

- 20 users;
- 108,595 logical units;
- complete U1–U9 scenarios;
- USDT and USDC separation;
- Pioneer 100/100;
- final `nextUnitId = 108596`;
- comprehensive-phase USDT vault = 0;
- comprehensive-phase USDC vault = 0.

A dedicated runtime-security add-on has also passed:

- re-registration rejection;
- spoofed referrer rejection;
- structural self-referral rejection;
- non-canonical Treasury ATA rejection with atomic rollback;
- claim hijack rejection;
- non-canonical destination rejection;
- valid claim;
- double-claim/replay rejection;
- fake Treasury receives zero.

The security add-on intentionally creates one additional post-saturation purchase and therefore leaves an independently asserted 0.2-USDC Pioneer liability after the comprehensive phase has already closed both vaults to zero.

## Public Solana Devnet

### Historical transaction smoke — PASS

A real public-Devnet smoke passed on source-equivalent core head `866e724e5ae57ec9eb20f641d9272ce508554a6a`:

- run `32039983574`;
- temporary Program ID `9EWUPLXeyTJhW3idnWFUDP9xfBUAensW42kLYKiG55oM`;
- evidence artifact `9291863971`;
- digest `sha256:1c63cecb2df7330d3ead7ec0ee872d828c595f138599c16ee9aeef9b524b7254`;
- Pioneer reached 100/100;
- post-cap purchase created zero positions;
- final smoke vault = 0.

This is supporting history only.

### Final public comprehensive + security — PENDING

The final release candidate must run the maintained public workflow once after all free/local hardening is green. The same exact marker SHA must receive:

- protocol CI;
- RustSec;
- verifiable build;
- local comprehensive + security;
- public Devnet comprehensive + security.

The public workflow records exact source SHA, limits runtime mutation to temporary Program-ID substitution, records temporary `.so` hash/deployment evidence and publishes no private key.

Until this run is `passed`, **public Devnet remains a Mainnet release blocker**.

## Final production-runtime validation — PENDING

After the offline Mainnet Program ID and future registration timestamp are frozen, the exact production artifact must be exercised in a production-compatible runtime environment. The evidence must be bound to the final release SHA and recorded in `release/mainnet-release.json`.

This is separate from development-equivalent local/Devnet evidence because the production feature pins Treasury, stablecoin mints and registration timestamp at initialization.

## Documentation / repository gaps resolved during red-team hardening

- README corrected from fixed 10-unit activity to progressive weekly qualification.
- README/Audit Scope corrected so INACTIVE partial qualification preserves **SELF only**, not Pioneer.
- `SECURITY.md` rewritten from obsolete Verifier/Adapter architecture to the final single-program threat model.
- dead `scripts/revenue-adapter-static-gates.py` removed.
- RustSec workflow changed to run on every PR head so final marker-only commits receive exact-head security evidence.
- release manifest/gate strengthened to require separate public-Devnet comprehensive and final production-runtime evidence.
- runtime security suite added for registration/Treasury/claim/replay attacks.

## Remaining Mainnet gates

Mainnet remains intentionally blocked until all of the following are complete:

1. final public Devnet comprehensive + security run passes on the exact marker SHA;
2. final Program ID keypair is generated offline and only its public key is frozen;
3. future registration-open UTC timestamp is frozen;
4. canonical vault/Treasury ATA bootstrap and verification procedure is included;
5. final production artifact is rebuilt, reproduced and hash-matched;
6. final production-runtime validation passes on the final release SHA;
7. independent audit of exact frozen core + production transaction builder completes and all findings are dispositioned;
8. `release/mainnet-release.json` is complete and executable pre-mainnet gate is fully green;
9. Mainnet deploy occurs with temporary upgrade authority;
10. deployed bytecode is verified against the audited artifact before initialization;
11. frozen state is initialized and canonical ATAs are verified;
12. deliberately small Mainnet smoke passes without consuming Pioneer positions;
13. bytecode/state are reverified;
14. upgrade authority is removed permanently **last**.
