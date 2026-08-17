# Pre-Mainnet Irreversible Decisions — Core-Only Protocol

Status: **NOT READY FOR MAINNET** until every remaining blocking item below is closed.

## Business/economic decisions already frozen

- Solana / Anchor implementation.
- USDT and USDC only, each with 6 decimals.
- 1 USDT/USDC = 1 logical unit.
- A unit purchase is the sole production event that creates referral accounting.
- **SELF is the buyer and receives 50%.**
- The immutable direct sponsor is U1; U1–U9 use the fixed schedule `15 / 9 / 6 / 4 / 2.5 / 2 / 1.5 / 1 / 2`, completing the 43% network pool.
- Pioneer pool: 2%.
- Service/platform: 5%.
- No separate self-reentry genealogy position.
- IC-A fixed-depth routing/expiry; no dynamic compression.
- ACTIVE lasts 7 days; GRACE lasts 48 hours.
- Progressive ACTIVE-week minimum: weeks 1–2=10, 3–4=20, 5–6=30, 7–8=40, week 9+=50 units, capped permanently at 50.
- Only successfully-started ACTIVE weeks advance that minimum; calendar inactivity does not.
- Weekly personal-unit depth: 10=>U3, 25=>U4, 50=>U5, 100=>U6, 200=>U7, 350=>U8, 500=>U9.
- Depth unlock is prospective. A scheduled level beyond an ACTIVE/GRACE wallet's unlocked depth routes to Treasury/unallocated and is never recovered or compressed.
- Claims are pull-based and require the claimant wallet signature and ACTIVE status.
- Buyer pays gas for purchase; beneficiary pays gas only when claiming; receiving an accrual requires no beneficiary transaction.
- Service treasury: `AepYo8xanmKuRiLVeYQuCTJoQr1nyKiTApoKwHMEg8fn`.
- Mainnet USDT: `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB`.
- Mainnet USDC: `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`.

## Pioneer decisions already frozen

- Exactly 100 Pioneer positions maximum.
- Registration consumes zero positions.
- A single purchase creates `floor(units / 1000)` candidate positions.
- Purchases do not accumulate across transactions for Pioneer qualification.
- One wallet may own multiple positions.
- Assignment is capped by remaining capacity.
- At 98/100, a 3,000-unit purchase creates exactly two positions.
- At 100/100 no later purchase can create another position.
- Rewards/claims/wallet balances do not automatically create positions; only gross units in a buyer-signed purchase qualify.
- **Rule B:** newly created positions begin earning from the next global purchase, never from the purchase that created them.
- Weighted checkpoints prevent retroactive rewards when one wallet acquires positions at different times.
- Unassigned Pioneer shares are treasury-destined rather than redistributed.
- Pioneer positions are permanent and never recycled, but Pioneer economics are ACTIVE-gated: GRACE preserves eligible due; INACTIVE due expires to Treasury and cannot be recovered on later reactivation.

## Rank/badge decision already frozen for V1

- Rank is an indexer/application-layer feature, not an additional payout path.
- Current Rank uses rolling 30-day QNV, qualified legs and balance rules as specified in `RANK_BADGE_SPEC.md`.
- Personal purchase volume and claimed earnings do not increase the user's own QNV rank.
- Highest Lifetime Rank is retained separately from dynamic Current Rank.
- V1 rank/badges do not modify the 50/43/2/5 percentages.
- Pioneer is a separate badge displayed as `PIONEER ×N`.

## Architecture already frozen

The intended production boundary is one Solana program:

`purchase_and_distribute -> canonical vault/accounting -> claim / expiry settlement`

The previous Evidence / Qualification / Revenue Adapter design is superseded and is not part of the mainnet release.

The purchase path is rentless: no per-purchase `UnitBatch` PDA is created. Global Unit IDs, purchase indexes, Pioneer position additions/totals and current qualification/depth data are emitted through `UnitsPurchased`; immutable referral registration is emitted through `UserRegistered` for deterministic off-chain rank indexing.

## Engineering baseline already established

- Core workspace is pinned to the reviewed Solana/Anchor toolchain.
- Reference-model accounting checks enforce 50/43/2/5 conservation, nine-upline weights, progressive ACTIVE requirements and exact weekly depth boundaries.
- Static gates enforce SELF 50%, immutable sponsor-first network traversal, IC-A, canonical token accounts, rentless purchase, progressive activity/depth and Pioneer threshold/cap/Rule B/ACTIVE invariants.
- Core CI builds development and production artifacts from the locked dependency graph.
- RustSec scan is scoped to the final core dependency graph.
- Verifiable-build workflow is scoped to the final core artifact.
- LiteSVM testing covers purchase-triggered accounting, ancestry rollback, progressive activity, inactivity not advancing the week counter, prospective U3→U9 depth, activity/expiry, batching equivalence, Pioneer ACTIVE-only expiry/reactivation and final Pioneer position rules including 98->100 saturation.
- Mainnet release manifest and executable gate are scoped to one production artifact and require exact economics, activity/depth fingerprints, CI, RustSec, verified-build and successful devnet-smoke evidence.
- `AUDIT_SCOPE.md`, `AUDIT_HANDOFF.md`, `FINAL_SPEC_GAP_REVIEW.md` and `RANK_BADGE_SPEC.md` define the review package.

These are engineering evidences only. They do not replace the independent audit of the final frozen commit.

## Devnet deployment / transaction gate — CLOSED

A real GitHub-hosted Devnet deploy and transaction smoke completed successfully on core source head `866e724e5ae57ec9eb20f641d9272ce508554a6a`.

Evidence:

- GitHub Actions run: `32039983574` — **PASS**;
- temporary Devnet Program ID: `9EWUPLXeyTJhW3idnWFUDP9xfBUAensW42kLYKiG55oM`;
- deployment transaction: `643FxLkJ1mtKtd8Kh87QRFu481EC5nsWF2h76ctHjE47GFsrgLw3fL8QxWWnYQFJ1qs8iCYdLFF4kLtSf2CBcBLP`;
- evidence artifact ID: `9291863971`;
- artifact digest: `sha256:1c63cecb2df7330d3ead7ec0ee872d828c595f138599c16ee9aeef9b524b7254`;
- smoke output: `DEVNET TRANSACTION SMOKE: PASS`;
- Pioneer saturation: exactly `100/100` assigned and buyer-owned positions;
- final tested vault atomic balance: `0`;
- initial deployer balance: `5 SOL` Devnet;
- final deployer balance: `2.62394996 SOL` Devnet.

Subsequent cleanup commits only restored the manual-only workflow and removed the temporary funding trigger; they did not modify the protocol source used by the passing Devnet run. The Devnet Program ID and disposable deployer are test-only identities and must never be reused as the Mainnet identity.

## Blocking before controlled mainnet deployment

1. Keep the exact-final-commit protocol CI, RustSec and verifiable build fully green.
2. Generate the final Program ID keypair offline. The secret keypair must never enter GitHub, CI or chat.
3. Replace the development `declare_id!` and add matching `[programs.mainnet]` configuration using only the public Program ID.
4. Freeze a future registration-opening UTC timestamp.
5. Rebuild/retest the exact Program-ID/timestamp freeze commit and produce the final verifiable `.so` and SHA-256.
6. Obtain an independent third-party audit of the exact final core plus client transaction builder, and disposition every finding.
7. Complete `release/mainnet-release.json` with the exact commit/economics/activity/depth/Pioneer fingerprint, Program ID, registration timestamp, treasury/mints, `.so` hash, audit hash, exact CI evidence and successful devnet-smoke evidence.
8. Approve the deliberately limited mainnet smoke plan.
9. `python3 scripts/pre-mainnet-gate.py` must exit successfully with no blockers.
10. Measure current mainnet deployment/rent/fee requirements and fund only the temporary deployer needed for controlled deployment.

Until all ten remaining conditions hold, deployment remains intentionally fail-closed.

## Blocking before permanent immutability

- Deploy the final core with temporary upgrade authority retained.
- Verify the deployed Program ID and bytecode against the exact audited `.so` hash before initialization.
- Initialize with only the frozen treasury, canonical mints and frozen registration opening time.
- Execute a deliberately small mainnet smoke transaction set; do not consume Pioneer positions merely to reproduce the devnet saturation test.
- Verify unit IDs, genealogy, progressive activity state, weekly depth, stablecoin balances, vault liabilities, service allocation, Pioneer accounting and pull claim.
- Stop immediately on any bytecode, PDA, ancestry, token-balance or accounting mismatch.
- Only after the smoke is successful and deployed bytecode is reverified may upgrade authority be permanently removed.

## Key custody rule

GitHub contains public source and public Program IDs only. Final deployment/update keypairs remain offline under the custody runbook. No seed phrase, secret key array, JSON keypair or private key may be pasted into issues, PRs, CI variables, chat transcripts or repository files.
