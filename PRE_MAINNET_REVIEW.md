# Pre-Mainnet Irreversible Decisions — Core-Only Protocol

Status: **NOT READY FOR MAINNET** until every blocking release gate below is closed.

## Business / economics already frozen

- Solana / Anchor implementation.
- USDT and USDC only, six decimals.
- 1 USDT/USDC = 1 logical unit.
- Buyer-signed purchase is the sole production economic event.
- SELF / buyer: 50%.
- U1–U9: `15 / 9 / 6 / 4 / 2.5 / 2 / 1.5 / 1 / 2%` = 43%.
- Pioneer: 2%.
- Service: 5%.
- No separate self-reentry genealogy position.
- IC-A fixed routing/expiry; no dynamic compression.
- ACTIVE 7 days; GRACE 48 hours.
- Progressive ACTIVE requirement: `10,10,20,20,30,30,40,40,50...` capped at 50.
- Only successfully-started ACTIVE weeks advance the requirement.
- During a live INACTIVE partial-qualification window, **SELF only** is provisionally preserved.
- Claims are pull-based, claimant-signed and ACTIVE-only.
- Weekly personal-unit depth: 10→U3, 25→U4, 50→U5, 100→U6, 200→U7, 350→U8, 500→U9.
- Depth is prospective; locked or inactive fixed shares route to Treasury and are never compressed.
- Service Treasury: `AepYo8xanmKuRiLVeYQuCTJoQr1nyKiTApoKwHMEg8fn`.
- Mainnet USDT: `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB`.
- Mainnet USDC: `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`.

## Pioneer already frozen

- hard cap 100 positions;
- registration assigns zero;
- one purchase creates `floor(units / 1000)` candidates;
- purchases do not accumulate across transactions;
- one wallet may own multiple/all positions;
- 98/100 + 3,000 units = 2 final positions;
- 100/100 blocks position 101;
- only gross buyer-signed purchase units qualify;
- Rule B: new positions begin earning from the next purchase;
- weighted checkpoints prevent retroactive entitlement;
- unassigned virtual-slot shares go to Treasury;
- position ownership survives inactivity, while unclaimed Pioneer due expires once INACTIVE.

## Rank / badge V1 already frozen

Rank is indexer/application metadata only. It uses rolling QNV/qualified legs/balance rules from `RANK_BADGE_SPEC.md` and **cannot alter the 50/43/2/5 payout core**.

## Architecture already frozen

Production boundary:

`purchase_and_distribute -> canonical vault/accounting -> claim / expiry settlement`

The old Evidence / Qualification / Revenue Adapter / Verifier architecture is retired. The final purchase path is rentless and emits deterministic registration/purchase events for audit/indexing.

## Current engineering evidence

The repository now requires complementary evidence rather than one monolithic test:

1. reference economics and rank models;
2. static economic/security gates;
3. Rust unit/property tests;
4. LiteSVM time-boundary and adversarial tests;
5. isolated `solana-test-validator` comprehensive transactions;
6. isolated runtime-security adversarial transactions;
7. RustSec on every exact PR head;
8. reproducible/verifiable production build and byte/hash comparison;
9. final public Solana Devnet comprehensive + security run;
10. final production-runtime validation after Program ID/timestamp freeze;
11. independent audit of exact final core + production transaction builder.

Explicit adversarial coverage includes ancestry spoofing, wrong vault/mint/source authority, registration spoofing, re-registration, structural self-referral, Treasury ATA substitution, claim hijack, non-canonical claim destination, double claim/replay and atomic rollback when payment CPI fails after expiry settlement has already started.

The current local comprehensive/security framework has passed using real SPL accounts, PDAs, signatures and a deployed local validator. The comprehensive phase closes both token vaults to zero. The separate security phase deliberately adds another post-saturation purchase and independently asserts its resulting 0.2-USDC Pioneer liability.

## Public Devnet status

### Historical smoke — PASS / supporting history

A real public-Devnet smoke passed on source-equivalent core head `866e724e5ae57ec9eb20f641d9272ce508554a6a`:

- run `32039983574`;
- temporary Program ID `9EWUPLXeyTJhW3idnWFUDP9xfBUAensW42kLYKiG55oM`;
- deployment transaction `643FxLkJ1mtKtd8Kh87QRFu481EC5nsWF2h76ctHjE47GFsrgLw3fL8QxWWnYQFJ1qs8iCYdLFF4kLtSf2CBcBLP`;
- artifact `9291863971`;
- digest `sha256:1c63cecb2df7330d3ead7ec0ee872d828c595f138599c16ee9aeef9b524b7254`;
- Pioneer 100/100 and post-cap rejection;
- final smoke vault 0.

This proves real-network viability but **does not close the final Devnet gate** after the expanded comprehensive/security specification.

### Final public comprehensive + security — OPEN / blocking

After all free/local hardening is green, a single marker-only commit will start the final public workflow. The same marker SHA must also receive exact-head protocol CI, RustSec, verifiable build and local comprehensive/security evidence.

The public workflow:

- records exact source SHA;
- mutates only the temporary Program ID in `Anchor.toml` and `lib.rs` for the test deployment;
- fails if another tracked source file changes during that substitution;
- records temporary `.so` hash, Program ID and deployment output;
- runs both comprehensive and runtime-security suites;
- publishes no private key.

Until this public run is `passed`, Mainnet is blocked.

## Blocking before controlled Mainnet deployment

1. Final public Devnet comprehensive + security passes on its exact marker SHA.
2. Final Program ID keypair is generated offline; only its public key enters source/config.
3. Future registration-open UTC timestamp is frozen.
4. Final source/config includes canonical vault/Treasury ATA bootstrap and verification procedure.
5. Exact freeze commit receives green protocol CI and exact-head RustSec.
6. Final production/verifiable `.so` is reproduced and byte/hash matched.
7. Final **production-runtime validation** passes and is bound to the final release SHA.
8. Independent third-party audit of exact final core + production transaction builder completes and every finding is dispositioned/retested.
9. `release/mainnet-release.json` records exact commit, Program ID, timestamp, economics/activity/depth/Pioneer fingerprints, `.so` hash, audit hash, CI/RustSec/verifiable evidence, final Devnet comprehensive evidence and production-runtime evidence.
10. Mainnet smoke plan is explicitly approved.
11. `python3 scripts/pre-mainnet-gate.py` exits successfully with no blockers.
12. Current Mainnet deployment/rent/fee requirements are measured and only the temporary deployer is funded.

Until all conditions hold, deployment remains intentionally fail-closed.

## Blocking before permanent immutability

- Deploy with temporary upgrade authority retained.
- Verify deployed bytecode equals the exact audited `.so` **before initialization**.
- Initialize only frozen Treasury/mints/registration time.
- Create if absent and verify canonical USDT/USDC ATAs for vault-authority PDA and frozen Service Treasury.
- Execute a deliberately small Mainnet smoke; do not consume Pioneer positions merely to reproduce saturation tests.
- Verify Unit IDs, genealogy, activity/depth, balances, vault liabilities, service/Pioneer accounting and pull claim.
- Stop on any bytecode/PDA/ancestry/token/accounting mismatch.
- Reverify deployed bytecode/state.
- Only then permanently remove upgrade authority.

## Key custody

Final Program ID and temporary deployment/upgrade-authority private keys remain offline. No seed phrase, private key array or keypair JSON may be committed, uploaded to CI, pasted into PR/issues or sent in chat. Only public keys and non-secret hashes/evidence enter the release package.
