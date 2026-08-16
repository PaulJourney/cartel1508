# Pre-Mainnet Irreversible Decisions — Core-Only Protocol

Status: **NOT READY FOR MAINNET** until every blocking item below is closed.

## Business/economic decisions already frozen

- Solana / Anchor implementation.
- USDT and USDC only, each with 6 decimals.
- 1 USDT/USDC = 1 logical unit.
- A unit purchase is the sole production event that creates referral accounting.
- Level 1 is the immutable direct sponsor and receives 50% direct only.
- Levels 2–10 receive the 43% network pool: `15 / 9 / 6 / 4 / 2.5 / 2 / 1.5 / 1 / 2`.
- Pioneer pool: 2%, first 100 real registrations.
- Service/platform: 5%.
- Activity threshold: 10 units; ACTIVE 7 days; GRACE 48 hours.
- Claims are pull-based and require the claimant wallet signature and ACTIVE status.
- Buyer pays gas for purchase; beneficiary pays gas only when claiming; receiving an accrual requires no beneficiary transaction.
- Service treasury: `AepYo8xanmKuRiLVeYQuCTJoQr1nyKiTApoKwHMEg8fn`.
- Mainnet USDT: `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB`.
- Mainnet USDC: `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`.

## Architecture already frozen

The intended production boundary is one Solana program:

`purchase_and_distribute -> vault/accounting -> claim`

The previous Evidence / Qualification / Revenue Adapter design is superseded and is not part of the mainnet release.

## Engineering baseline already established

- Core workspace is pinned to the reviewed Solana/Anchor toolchain.
- Reference-model accounting checks enforce 50/43/2/5 conservation and the nine L2–L10 weights.
- Static gates require the purchase-triggered production path, canonical token accounts, pull claims and the L10 traversal cap.
- Core CI builds development and production artifacts from the locked dependency graph.
- RustSec scan is scoped to the final core dependency graph.
- Verifiable-build workflow is scoped to the final core artifact.
- Dedicated LiteSVM testing covers purchase-triggered accounting and separate claimant-signed withdrawal.
- Mainnet release manifest and executable gate have been reduced to one production artifact.

These are engineering evidences only. They do not replace the independent audit of the final frozen commit.

## Blocking before controlled mainnet deployment

1. Remove all remaining dead qualified-revenue instruction/state/configuration from the core itself so the audited interface exposes only the intended production model.
2. Migrate or replace historical tests that depend on the superseded instruction surface.
3. Make the complete final core CI green after that cleanup.
4. Run a devnet smoke using the final instruction surface: initialize, register genealogy, purchase, activity transition, 50/43/2/5 accounting and pull claim.
5. Generate the final Program ID keypair offline. The secret keypair must never enter GitHub or this repository.
6. Replace the development `declare_id!` and add matching `[programs.mainnet]` configuration.
7. Freeze a future registration-opening UTC timestamp.
8. Produce the exact final verifiable `.so` and SHA-256 from the audited source commit.
9. Obtain an independent third-party audit of the final core plus the client transaction builder, and disposition every finding.
10. Complete `release/mainnet-release.json` with the exact commit SHA, Program ID, registration timestamp, treasury/mints, `.so` hash, audit-report hash and verified-build evidence.
11. `python3 scripts/pre-mainnet-gate.py` must exit successfully with no blockers.

Until all eleven conditions hold, deployment remains intentionally fail-closed.

## Blocking before permanent immutability

- Deploy the final core with temporary upgrade authority retained.
- Verify the deployed Program ID and bytecode against the exact audited `.so` hash.
- Initialize with only the frozen treasury, canonical mints and frozen registration opening time.
- Execute a deliberately small mainnet smoke transaction set.
- Verify unit IDs, genealogy, stablecoin balances, vault liabilities, service allocation, Pioneer accounting and pull claim.
- Stop immediately on any bytecode, PDA, ancestry, token-balance or accounting mismatch.
- Only after the smoke is successful and independently reviewed may the upgrade authority be permanently removed.

## Key custody rule

GitHub contains public source and public Program IDs only. Final deployment/update keypairs remain offline under the custody runbook. No seed phrase, secret key array, JSON keypair or private key may be pasted into issues, PRs, CI variables, chat transcripts or repository files.
