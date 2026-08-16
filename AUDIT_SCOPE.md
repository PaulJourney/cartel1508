# External Audit Scope — Purchase-Triggered Solana Protocol

Status: **pre-audit**. This document defines the intended independent review target. It is not an audit report or a production approval.

## Production architecture in scope

The intended mainnet release is a **single immutable Solana program** plus the canonical SPL Token Program:

`User purchase -> Service Referral Protocol -> canonical USDT/USDC vault -> accounting -> pull claim`

There is no production Revenue Evidence, Revenue Qualification or Revenue Adapter program. A successful unit purchase is the only economic event that creates referral liabilities.

### Files in the production review boundary

- `programs/service_referral_protocol/src/lib.rs`
- `programs/service_referral_protocol/src/state.rs`
- `programs/service_referral_protocol/src/math.rs`
- `programs/service_referral_protocol/src/constants.rs`
- `programs/service_referral_protocol/Cargo.toml`
- root `Cargo.toml` and `Cargo.lock`
- `tests/reference-model.mjs`
- `integration-tests/**`
- `scripts/static-gates.py`
- `scripts/pre-mainnet-gate.py`
- `release/mainnet-release.example.json`
- `.github/workflows/ci.yml`
- `.github/workflows/security-scan.yml`
- `.github/workflows/verifiable.yml`
- `.github/workflows/devnet-deploy-smoke.yml`
- `PROGRAM_ID_CUSTODY_RUNBOOK.md`
- `MAINNET_FINALIZATION_RUNBOOK.md`
- the exact frontend/client transaction builder used to call the production instructions

## Frozen economics to verify

For every stablecoin unit purchase:

- 1 USDT/USDC = 1 logical unit.
- Level 1 is the buyer's immutable direct sponsor and receives 50% direct only.
- Levels 2–10 receive the 43% network pool using the frozen schedule:
  - L2 15%
  - L3 9%
  - L4 6%
  - L5 4%
  - L6 2.5%
  - L7 2%
  - L8 1.5%
  - L9 1%
  - L10 2%
- Pioneer pool: 2%, shared by the first 100 real registrations according to the high-precision index implementation.
- Service/platform allocation: 5%.
- Aggregate allocation is exactly 50% + 43% + 2% + 5% = 100% before deterministic integer-rounding handling.
- There is no L11 economic level and Level 1 must never receive a duplicate network-depth percentage.

## User lifecycle and claim properties to review

- Registration relation is immutable after creation.
- Activity threshold is 10 units within the qualification window.
- ACTIVE lasts 7 days and GRACE lasts 48 hours.
- Network reward behavior at ACTIVE / GRACE / INACTIVE boundaries is deterministic and cannot be bypassed by timestamp/account substitution.
- Pull claims require the beneficiary wallet signature and ACTIVE status.
- Reward accrual itself requires no beneficiary signature and therefore no beneficiary-paid gas.
- A claimant can aggregate multiple accruals and withdraw them in one later claim.
- Permissionless expiry settlement cannot redirect funds away from the frozen service treasury.
- Time passing by itself does not execute a transaction; settlement happens when state is touched by an instruction.

## Purchase-path properties to review

- `purchase_and_distribute` is the sole production economic entrypoint.
- Payment amount is exactly `units * 10^6` atomic units for six-decimal USDT/USDC.
- Unsupported mints are rejected.
- User source token account authority is the buyer.
- Vault and treasury token accounts are canonical ATAs for the expected owner and mint.
- Buyer funds are transferred into the canonical vault before liabilities are released from it.
- Unit allocation, activity update, referral accounting, Pioneer accounting and treasury routing are atomic in one Solana transaction.
- Supplied sponsor and L2–L10 accounts must exactly match immutable ancestry.
- Technical-root termination routes the remaining unallocated depth deterministically to treasury.
- Failed ancestry, token-account, arithmetic or CPI validation must roll back token movement and state changes.
- Global Unit IDs are monotonic, unique and overflow-safe.
- Large purchases cannot overflow SPL Token `u64` payment amounts.

## Vault and accounting properties to review

- The vault remains fully collateralized for all outstanding claim liabilities.
- Treasury movements are exactly service allocation plus explicitly unallocated/expired/rounding/Pioneer-unassigned value.
- USDT and USDC accounting rails cannot contaminate each other.
- Integer rounding cannot create value, underflow, or orphan liabilities.
- Pioneer fractional carry is conserved at the configured high precision.
- No claimant can clear another user's accounting buckets or redirect another user's claim.
- A failed claim is atomic and preserves the user's accrued state.

## Authority and immutability properties to review

- No owner/admin instruction can mutate percentages, treasury, referral relationships, supported mints or accounting rules after initialization.
- Mainnet service treasury is frozen to `AepYo8xanmKuRiLVeYQuCTJoQr1nyKiTApoKwHMEg8fn`.
- Mainnet USDT mint is frozen to `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB`.
- Mainnet USDC mint is frozen to `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`.
- Production initialization must fail closed until the final Program ID and registration-open timestamp are frozen.
- The final Program ID keypair and deploy authority secret must never be committed to the repository.
- Upgrade authority must remain during controlled deployment/verification/smoke and be permanently removed only after the deployed bytecode matches the reviewed artifact.

## Gas model to verify

- Registration: registering user is the payer/signature authority for user-state creation.
- Purchase: buyer signs and pays one Solana transaction fee for purchase plus accounting.
- Accrual: sponsor/uplines/Pioneers do not sign merely to receive accounting credit.
- Claim: claimant signs and pays the Solana transaction fee.
- The protocol does not require a platform-funded transaction per commission event.

## Automated evidence required before final audit sign-off

At minimum the exact release commit must have green evidence for:

- pinned Solana/Anchor production compile;
- reference-model conservation tests;
- static security/economic gates;
- Rust unit/property tests;
- LiteSVM purchase/distribution/claim integration tests;
- adversarial ancestry/account-substitution rollback tests;
- ACTIVE/GRACE/INACTIVE boundary and expiry tests;
- unsupported mint / wrong authority / noncanonical ATA rejection;
- unit-ID continuity and overflow tests;
- RustSec dependency scan;
- reproducible/verifiable production build and SHA-256;
- devnet smoke using the final instruction surface.

Historical qualified-revenue/adapter test evidence is not part of the final release evidence and must not be cited as proof of the production architecture.

## Explicit blockers before final mainnet approval

- Final core instruction/state cleanup complete with no obsolete qualified-revenue authority or production entrypoint.
- Final Program ID generated under the custody runbook and frozen consistently in source/configuration.
- Final registration opening UTC frozen.
- Exact dependency lockfile frozen.
- Independent audit completed against the exact final commit and artifact, with every finding dispositioned.
- `release/mainnet-release.json` completed with exact commit, Program ID, artifact hash, audit hash and verified-build evidence.
- Executable pre-mainnet gate fully green.
- Controlled mainnet deploy and limited smoke completed while upgrade authority is still retained.
- Deployed bytecode verified against the audited artifact.
- Upgrade authority permanently removed only after all preceding checks pass.

## Out of scope unless separately commissioned

- Legal/regulatory classification of the commercial/referral model.
- Frontend visual design.
- Marketing or business claims.

The client transaction builder remains security-relevant because it chooses the sponsor/upline accounts supplied to the program; it must therefore be reviewed for correct deterministic ancestry construction even though the on-chain program independently verifies those accounts.
