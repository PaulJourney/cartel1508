# Security model — V0.10 three-program trust chain

This repository is **pre-audit software and is not approved for production/mainnet deployment**.

The security model spans three Solana programs:

1. `revenue_qualification`
2. `revenue_adapter`
3. `service_referral_protocol`

They must be reviewed and released as one coordinated authorization and economic system.

## Implemented and tested

### Referral core

- Service-unit purchase and qualified-revenue accounting are isolated funding paths.
- Service-unit purchases route directly to treasury and do not create referral liabilities.
- Referral relationships are immutable and ancestry is validated at runtime.
- Supported stablecoin mints are typed, distinct and require six decimals.
- Protocol vault, treasury and claim token accounts are constrained to canonical ATAs.
- Gross qualified revenue is collateralized before referral liabilities are committed.
- Expired balances are permissionlessly settled and transferred to treasury.
- Claims require the beneficiary signature and ACTIVE status.
- Global service Unit IDs are monotonic `u128` ranges allocated in O(1) across all wallets.
- No owner/admin mutation, pause, treasury mutation, referral mutation or revenue-source mutation instruction exists.

### Revenue adapter

- Initialize-once AdapterConfig binds referral + qualification identities and supported stablecoin rails.
- Revenue Authority is a deterministic PDA with no private key.
- Qualification Authority must be the deterministic signer PDA of the bound qualification program.
- Source token account must be the canonical Revenue Authority ATA and sufficiently funded.
- One deterministic receipt PDA exists per event ID; exact receipt replay is rejected.
- Only the bound executable referral program may receive the downstream CPI.
- Revenue Authority PDA signs the referral CPI through deterministic signer seeds.
- No mutable admin/config update route is present.

### Revenue qualification gateway

- Payer signs the real SPL-token transfer and must own the payment source account.
- Supported mint must match the immutable adapter configuration.
- Destination must be the canonical Revenue Authority ATA.
- Canonical AdapterConfig PDA is independently re-derived before token movement.
- Adapter and referral accounts are checked as executable, and referral identity must match the immutable adapter configuration before token movement.
- Event identity deterministically binds payer, beneficiary, mint, amount and nonce.
- Qualification Authority is a deterministic PDA with no private key and signs only through program PDA seeds.
- Adapter receipt PDA is recomputed from the internally derived event identity.
- A downstream CPI failure rolls back the outer token transfer and receipt mutation atomically.

### Release and test controls

- Static gates cover core, adapter and qualification trust boundaries.
- Root production dependencies and the separate integration-test dependency graph are locked.
- Rust tests and targeted LiteSVM tests cover deterministic/state invariants.
- Full three-program runtime qualification is gated on a real isolated `solana-test-validator` rather than relying on the known LiteSVM multi-program SPL-mint fixture anomaly.
- Real-validator smoke proves successful payer-funded routing, exact replay rejection, downstream failure rollback, claim and exact token conservation.
- The validator smoke also exercises account/executable substitution defenses as they are added.
- RustSec scanning currently reports zero known vulnerabilities in the production dependency graph; transitive maintenance warnings remain reviewable evidence rather than being hidden.
- Docker `anchor build --verifiable` produces and hashes all three production-profile program artifacts.
- `scripts/pre-mainnet-gate.py` independently derives the Revenue Authority and Qualification Authority PDAs from final Program IDs and self-tests that PDA derivation against real-validator vectors.
- The final release gate prefers the Docker/verifiable artifacts and requires exact source/manifest/commit/hash agreement.

## Critical unresolved production boundary

A payer-signed real USDT/USDC transfer proves that value moved. **It does not by itself prove that the payment is legitimate external service/business revenue.**

Without an additional non-self-generable evidence rule, a participant could self-fund an artificial event and create economically artificial referral rewards even though every token is fully collateralized.

Therefore production/mainnet remains deliberately fail-closed until the actual product integration defines a concrete evidence source/rule that:

- an arbitrary participant cannot fabricate;
- uniquely identifies the underlying economic event;
- binds payer, beneficiary, stablecoin mint and gross amount;
- prevents the same economic event from being represented multiple times under different nonces/integrations;
- has reviewed refund/reversal/dispute semantics;
- does not leave one permanent human/admin key with unilateral reward-creation authority.

See `QUALIFIED_REVENUE_SOURCE_SPEC.md` and `AUDIT_SCOPE.md`.

## Remaining before controlled mainnet deployment

- Approve the concrete non-self-generable qualified-revenue evidence model through independent review.
- Generate and securely custody three final Program ID keypairs outside GitHub, CI and ChatGPT.
- Freeze all three public Program IDs and `[programs.mainnet]` entries.
- Freeze adapter -> referral, adapter -> qualification and qualification -> adapter production bindings.
- Derive/freeze the final Revenue Authority PDA as core `MAINNET_QUALIFIED_REVENUE_SOURCE`.
- Derive/record the final Qualification Authority PDA.
- Freeze the exact registration opening UTC.
- Complete independent third-party security audit of the exact final three-program commit and resolve findings.
- Re-run locked CI, real-validator smoke, RustSec and Docker-verifiable builds after every immutable production value/evidence rule is frozen.
- Complete `release/mainnet-release.json` with exact commit, identities, PDAs, three artifact hashes, audit hash and evidence-spec hash.
- `python3 scripts/pre-mainnet-gate.py` must return `READY FOR CONTROLLED MAINNET DEPLOYMENT`.
- Measure exact final artifact sizes and current mainnet deployment requirements before funding the temporary deployer.

## Remaining before permanent immutability

- Deploy the exact audited artifacts with temporary upgrade authority still available.
- Verify all three deployed bytecodes against their exact verified artifact hashes.
- Verify every on-chain cross-program binding and PDA derivation.
- Execute only the limited approved mainnet smoke plan in `MAINNET_SMOKE_PLAN.md`.
- Reconcile all token movements and protocol state.
- Remove upgrade authority permanently only after the coordinated three-program release has passed all verification.

`--final` is irreversible. A program must not be finalized while another program in the trust chain still requires a binding or code correction.
