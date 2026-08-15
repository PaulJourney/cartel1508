# Service Referral Protocol — Solana three-program V0.10 development baseline

Public development repository for an intended ownerless Solana/Anchor referral and qualified-revenue system.

## Economic model implemented

- Solana + Anchor/Rust.
- USDT and USDC are separate accounting rails.
- 1 USDT/USDC = 1 logical service unit.
- Large unit purchases use one batch PDA with a contiguous globally unique Unit ID range rather than one account per unit.
- Immutable referral relationship, maximum 10 economic levels.
- Activity threshold: 10 units; ACTIVE 7 days; GRACE 48 hours.
- Network rewards: ACTIVE -> claimable, GRACE -> pending, INACTIVE -> treasury allocation.
- Direct attributed rewards and Pioneer entitlement persist but can only be claimed while ACTIVE.
- First 100 real registrations receive non-transferable Pioneer IDs.
- Pull-based claims: users sign the claim and pay their own SOL transaction fee.
- Service-unit purchases **do not create referral rewards** and route directly to the service treasury.

## Qualified service-revenue percentages

The reward engine applies only to separately qualified, separately funded service revenue:

- 50% direct attributed reward
- 43% referral depth: `15 / 9 / 6 / 4 / 2.5 / 2 / 1.5 / 1 / 1 / 1`
- 2% Pioneer allocation
- 5% service allocation

Service-unit/activity payments are deliberately isolated from this reward engine.

## Three-program trust chain

### 1. `revenue_qualification`

Purpose: gate an event before it can enter the reward path.

Current development implementation:

- payer must sign a real SPL-token transfer;
- payer must own the payment source token account;
- only mints frozen by the adapter are accepted;
- destination is the canonical stablecoin ATA of the adapter Revenue Authority PDA;
- event identity deterministically binds payer, beneficiary, mint, amount and nonce;
- a deterministic Qualification Authority PDA with no private key signs the CPI into the adapter;
- production mode remains fail-closed until the final adapter Program ID is frozen.

### 2. `revenue_adapter`

Purpose: immutable anti-replay and collateralized transport boundary.

- initialize-once AdapterConfig binds referral + qualification identities and stablecoin rails;
- deterministic Revenue Authority PDA has no private key;
- one deterministic receipt PDA exists per event ID;
- source must be the canonical Revenue Authority ATA and must already contain the event amount;
- exact referral executable is frozen in config;
- Revenue Authority PDA signs CPI into `service_referral_protocol::record_qualified_revenue`;
- no mutable admin/update route is introduced;
- production mode remains fail-closed until final referral/qualification Program IDs are frozen.

### 3. `service_referral_protocol`

Purpose: independently authenticate the frozen revenue source, collateralize gross revenue and apply referral accounting.

- gross stablecoin value is transferred into the protocol vault before liabilities are allocated;
- direct/network/Pioneer/service accounting is then applied;
- treasury-assigned components leave the vault immediately;
- only outstanding user/Pioneer liabilities remain collateralized in the vault.

Solana transaction atomicity means a downstream failure rolls back the entire outer transaction, including the payer transfer and adapter receipt creation.

## What has been proven on a real validator

A permanent isolated `solana-test-validator` workflow builds and deploys all three programs with ephemeral test-only Program IDs, then executes real transactions.

Current passing evidence includes:

- referral + adapter initialization;
- Pioneer #1 registration;
- 10-unit activation and proof that unit-purchase funds do not enter the reward vault;
- separately funded 100-USDC qualification -> adapter -> referral accounting;
- exact event replay rejection with balances unchanged;
- deliberate downstream ancestry failure with payer transfer and new receipt fully rolled back;
- ACTIVE claim;
- exact conservation of all 310 test USDC.

Passing real-validator run: `31899960367`.

## Important unresolved production boundary

A real payer-signed stablecoin transfer proves that value moved. It does **not**, by itself, prove that the transfer corresponds to a legitimate external service/business revenue event. Without another non-self-generable evidence rule, a participant could self-fund an artificial event.

For that reason, mainnet remains intentionally fail-closed until the production qualification/evidence model is concrete, non-self-generable, independently reviewed and frozen into the final release evidence.

See `QUALIFIED_REVENUE_SOURCE_SPEC.md` and `AUDIT_SCOPE.md`.

## Public mainnet constants already identified

- Service treasury: `AepYo8xanmKuRiLVeYQuCTJoQr1nyKiTApoKwHMEg8fn`
- USDT: `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB`
- USDC: `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`

The three currently declared Program IDs are development identities and are **not** mainnet-final.

## Mainnet release gate

Do not fund or deploy the final mainnet release until all of the following are complete:

1. concrete qualified-revenue evidence model approved;
2. three final Program IDs generated/custodied offline and public IDs frozen in source;
3. final adapter Revenue Authority PDA derived and frozen as the core qualified-revenue source;
4. adapter -> referral, adapter -> qualification and qualification -> adapter production bindings frozen;
5. exact registration opening UTC frozen;
6. all three `[programs.mainnet]` identities frozen;
7. locked tests/static/security gates pass;
8. real-validator three-program smoke passes on the exact frozen release;
9. independent third-party audit of the exact final trust chain passes and findings are resolved;
10. Docker/verifiable build produces and hashes all three exact production artifacts;
11. `release/mainnet-release.json` matches source, commit, three artifact hashes, audit hash and qualification-evidence-spec hash;
12. `python3 scripts/pre-mainnet-gate.py` returns `READY FOR CONTROLLED MAINNET DEPLOYMENT`.

That READY result authorizes only a controlled mainnet deployment with temporary upgrade authority. After deployment:

- verify all three deployed bytecodes against the audited artifacts;
- verify all PDA derivations and cross-program bindings;
- run the limited approved mainnet smoke;
- only then remove upgrade authority from every program intended to be immutable.

Final Program ID private keys, deployer secrets and permanent-wallet seeds must never be committed to this repository, GitHub Actions or ChatGPT.

## Important Solana property

Time does not execute transactions by itself. A reward can become economically expired after the grace timestamp, but token movement to the treasury occurs on the next instruction that settles/touches that state. No platform keeper is required.
