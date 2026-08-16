# Pre-Mainnet Irreversible Decisions — Core-Only Protocol

Status: **NOT READY FOR MAINNET** until every blocking item below is closed.

## Business/economic decisions already frozen

- Solana / Anchor implementation.
- USDT and USDC only, each with 6 decimals.
- 1 USDT/USDC = 1 logical unit.
- A unit purchase is the sole production event that creates referral accounting.
- **SELF is the buyer and receives 50%.**
- The immutable direct sponsor is U1 and receives 15%.
- U2–U9 receive `9 / 6 / 4 / 2.5 / 2 / 1.5 / 1 / 2`, completing the 43% nine-upline network pool.
- Pioneer pool: 2%.
- Service/platform: 5%.
- No separate self-reentry genealogy position.
- IC-A fixed-depth expiry; no dynamic compression.
- Activity threshold: 10 units; ACTIVE 7 days; GRACE 48 hours.
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
- Rewards/claims/wallet balances do not create positions; only gross units in a buyer-signed purchase qualify.
- **Rule B:** newly created positions begin earning from the next global purchase, never from the purchase that created them.
- Weighted checkpoints prevent retroactive rewards when one wallet acquires positions at different times.
- Unassigned Pioneer shares are treasury-destined rather than redistributed.

## Architecture already frozen

The intended production boundary is one Solana program:

`purchase_and_distribute -> canonical vault/accounting -> claim / expiry settlement`

The previous Evidence / Qualification / Revenue Adapter design is superseded and is not part of the mainnet release.

The purchase path is rentless: no per-purchase `UnitBatch` PDA is created. Global Unit IDs, purchase indexes and Pioneer position additions/totals are emitted through `UnitsPurchased`.

## Engineering baseline already established

- Core workspace is pinned to the reviewed Solana/Anchor toolchain.
- Reference-model accounting checks enforce 50/43/2/5 conservation and the nine-upline weights.
- Static gates enforce SELF 50%, immutable sponsor-first network traversal, IC-A, canonical token accounts, rentless purchase and Pioneer threshold/cap/Rule B invariants.
- Core CI builds development and production artifacts from the locked dependency graph.
- RustSec scan is scoped to the final core dependency graph.
- Verifiable-build workflow is scoped to the final core artifact.
- LiteSVM testing covers purchase-triggered accounting, ancestry rollback, activity/expiry, batching equivalence and final Pioneer position rules including 98->100 saturation.
- Mainnet release manifest and executable gate are scoped to one production artifact and now require exact CI, RustSec, verified-build and successful devnet-smoke evidence.
- `AUDIT_SCOPE.md` and `AUDIT_HANDOFF.md` define the independent review package.

These are engineering evidences only. They do not replace the independent audit of the final frozen commit.

## Blocking before controlled mainnet deployment

1. Keep the exact-final-commit protocol CI, RustSec and verifiable build fully green.
2. Complete a **real devnet deploy and transaction smoke** using the final instruction/Pioneer surface. Faucet/rate-limit failure before deployment is not passing evidence.
3. Generate the final Program ID keypair offline. The secret keypair must never enter GitHub, CI or chat.
4. Replace the development `declare_id!` and add matching `[programs.mainnet]` configuration using only the public Program ID.
5. Freeze a future registration-opening UTC timestamp.
6. Rebuild/retest the exact Program-ID/timestamp freeze commit and produce the final verifiable `.so` and SHA-256.
7. Obtain an independent third-party audit of the exact final core plus client transaction builder, and disposition every finding.
8. Complete `release/mainnet-release.json` with the exact commit/economics/Pioneer fingerprint, Program ID, registration timestamp, treasury/mints, `.so` hash, audit hash, exact CI evidence and successful devnet-smoke evidence.
9. Approve the deliberately limited mainnet smoke plan.
10. `python3 scripts/pre-mainnet-gate.py` must exit successfully with no blockers.
11. Measure current mainnet deployment/rent/fee requirements and fund only the temporary deployer needed for controlled deployment.

Until all eleven conditions hold, deployment remains intentionally fail-closed.

## Blocking before permanent immutability

- Deploy the final core with temporary upgrade authority retained.
- Verify the deployed Program ID and bytecode against the exact audited `.so` hash before initialization.
- Initialize with only the frozen treasury, canonical mints and frozen registration opening time.
- Execute a deliberately small mainnet smoke transaction set; do not consume Pioneer positions merely to reproduce the devnet saturation test.
- Verify unit IDs, genealogy, stablecoin balances, vault liabilities, service allocation, Pioneer accounting and pull claim.
- Stop immediately on any bytecode, PDA, ancestry, token-balance or accounting mismatch.
- Only after the smoke is successful and deployed bytecode is reverified may upgrade authority be permanently removed.

## Key custody rule

GitHub contains public source and public Program IDs only. Final deployment/update keypairs remain offline under the custody runbook. No seed phrase, secret key array, JSON keypair or private key may be pasted into issues, PRs, CI variables, chat transcripts or repository files.
