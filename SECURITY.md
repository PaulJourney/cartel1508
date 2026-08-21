# Security model — V0.16

This is pre-audit software and is **not** approved for irreversible Mainnet deployment.

## Production trust boundary

The final production path is intentionally small:

`buyer/claimant signer -> Service Referral Protocol -> SPL Token Program`

There is no production Revenue Verifier, Revenue Adapter, external qualified-revenue source, owner/admin controller or keeper authority in the final economic path.

Buyer-signed `purchase_and_distribute` is the sole event that creates new economic liabilities.

## Immutable / ownerless surface

The final instruction surface is limited to:

- `initialize`
- `register`
- `purchase_and_distribute`
- `settle_expired`
- `claim`

The core exposes no owner/admin setter, pause/unpause, Treasury mutation, referral mutation, payout-table mutation or qualified-revenue-source mutation instruction.

Referral genealogy is immutable after registration.

## Identity and account integrity

The program does not trust client-provided account ordering or labels by themselves.

- `UserState` PDA is derived from the signer wallet.
- Registration referrer state is derived from the supplied immutable referrer wallet.
- Purchase ancestry is validated PDA-by-PDA against stored referrers.
- The buyer's token source must be owned by the buyer signer.
- Only the protocol's supported USDT/USDC mint can fund a purchase.
- Vault token accounts must be canonical SPL Token ATAs for the deterministic vault-authority PDA and expected mint.
- Service-Treasury token accounts must be canonical ATAs for the frozen Treasury and expected mint.
- Claim destination must be the claimant wallet's canonical ATA for the claimed mint.
- The SPL Token program is typed in Anchor rather than accepted as an arbitrary executable account.

A malicious or compromised transaction builder therefore cannot redirect SELF/network/Treasury/claim transfers to arbitrary token accounts without the transaction being rejected.

## Economic integrity

Frozen purchase allocation:

- SELF/buyer: 50%
- nine fixed network uplines: 43%
- Pioneer: 2%
- service: 5%

The network schedule is fixed at `15/9/6/4/2.5/2/1.5/1/2%`.

Network monetization depth is independently gated by each upline's own weekly personal volume. A locked or INACTIVE level is Treasury-destined; it is never compressed to another ancestor.

All critical arithmetic uses checked operations. Global logical Unit IDs are allocated monotonically in `u128`; SPL payment amounts are rejected before they exceed token `u64` capacity.

## Activity / expiry security

ACTIVE lasts seven days and GRACE lasts 48 hours.

The successfully-started weekly qualification ladder is:

`10, 10, 20, 20, 30, 30, 40, 40, 50, 50...`

Calendar inactivity alone cannot advance the ladder.

During a live INACTIVE partial-qualification window, **only buyer SELF** is provisionally preserved for batching equivalence. Network and Pioneer receive no partial-window exception.

Once a wallet becomes INACTIVE, previously unclaimed SELF/network/Pioneer value is permanently Treasury-destined. A late purchase settles stale value before reactivation, so expired history cannot be rescued.

Permissionless `settle_expired` physically moves Treasury-destined value; the settler only pays the transaction fee and receives no protocol authority.

## Pioneer security

- hard global cap: 100 positions;
- registration assigns zero;
- one purchase creates `floor(units / 1000)` candidates;
- separate purchases never accumulate toward the threshold;
- positions are permanent and may be multiple per wallet;
- assignment is capped by remaining slots;
- Rule B: positions created by a purchase do not earn from their creating purchase;
- weighted high-precision checkpoints prevent retroactive entitlement;
- INACTIVE unclaimed Pioneer due expires, while position ownership itself remains.

## Claims / replay

Claims require:

- claimant signature;
- claimant-bound `UserState` PDA;
- ACTIVE status;
- canonical vault;
- canonical claimant destination ATA;
- positive current entitlement.

A successful claim clears the consumed SELF/network accounting and advances the Pioneer checkpoint before the token transfer. Solana transaction atomicity ensures all of those mutations roll back if the transfer fails.

Explicit adversarial coverage requires an immediate second claim/replay to fail without changing `lifetime_claimed`, claimant balances or vault balances.

## Transaction atomicity

The purchase path deliberately settles already-expired buyer value before accepting the new payment. This order prevents late reactivation from rescuing expired history.

An explicit LiteSVM adversarial test then forces the later buyer payment CPI to fail after expiry settlement has started. The whole transaction must restore:

- old SELF entitlement;
- expiry counters;
- purchase index;
- ACTIVE-week/qualification state;
- global Unit ID;
- Vault and Treasury token balances.

The entitlement must remain settleable exactly once afterward.

## Runtime adversarial validation

The validation matrix includes, at minimum:

- false ancestry / upline substitution;
- wrong canonical vault;
- unsupported mint;
- source token owned by another wallet;
- non-canonical service-Treasury ATA;
- spoofed referrer wallet/PDA pair;
- re-registration of the same wallet;
- structural self-referral attempt;
- claim using another wallet's `UserState`;
- claim to a non-canonical destination;
- double-claim/replay;
- zero units / oversized purchase;
- initialization with duplicate or non-six-decimal mints;
- time-boundary ACTIVE/GRACE/INACTIVE behavior;
- Pioneer Rule B, weighted checkpoints and 100/100 saturation;
- full U1–U9 depth boundaries and out-of-depth Treasury routing.

These are exercised across Rust/property tests, LiteSVM, an isolated Solana validator and the final public Devnet run where technically possible.

## Production configuration / fail-closed launch

The current development source intentionally has `MAINNET_REGISTRATION_OPEN_AT = 0`. Under the `production` feature, initialization refuses to proceed until the launch configuration is frozen.

The final production initialization must match compile-time values for:

- Service Treasury;
- USDT mint;
- USDC mint;
- registration-open timestamp.

The final Mainnet Program ID is also frozen in `declare_id!` and `Anchor.toml` only after its keypair is generated offline.

## Key custody

Final Program ID and temporary deployment/upgrade-authority private keys must never enter:

- source control;
- GitHub Actions artifacts;
- issue/PR comments;
- chat;
- release documentation.

Only public keys and non-secret hashes/evidence belong in the repository. See `PROGRAM_ID_CUSTODY_RUNBOOK.md`.

## Release-integrity controls

Before controlled Mainnet deployment, the release process requires:

- exact-head protocol CI;
- exact-head RustSec scan;
- reproducible/verifiable `.so` build;
- byte/hash comparison of production artifacts;
- comprehensive isolated-runtime evidence;
- final public Devnet comprehensive + runtime-security evidence;
- final production-runtime validation bound to the final release SHA;
- independent third-party audit of the exact frozen core and production transaction builder;
- completed `release/mainnet-release.json`;
- fully green executable `scripts/pre-mainnet-gate.py`.

The gate also rejects secret-like tracked files and obsolete final-release Revenue Adapter tooling.

## Mainnet deployment safety

Deployment order is intentionally reversible until the final step:

1. generate Program ID / authority keypairs offline;
2. freeze final Program ID and future registration timestamp;
3. build/test/audit exact production artifact;
4. measure current deployment/rent/fee requirements;
5. deploy with temporary upgrade authority retained;
6. verify deployed bytecode equals the audited artifact **before initialization**;
7. initialize only the frozen configuration;
8. create if absent and verify canonical USDT/USDC ATAs for the vault-authority PDA and Service Treasury;
9. execute a deliberately small Mainnet smoke that does not consume Pioneer positions;
10. reverify bytecode and state;
11. only then permanently remove upgrade authority.

Once upgrade authority is removed permanently, there is no software rollback path. That step is therefore last.

## Historical tooling

Migration scripts may remain for historical state-transition documentation, but they are outside the final production/audit boundary. Dead Revenue Adapter release-gate tooling is removed so it cannot be mistaken for a current production dependency.
