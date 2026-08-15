# Final Program IDs and deployment-key custody runbook

Status: three-program custody procedure defined; no final mainnet Program ID keypair is committed to this repository.

## Production identities that must remain separate

### 1. Revenue Verifier Program ID keypair

Purpose: fixes the on-chain identity of the program that implements the FINAL qualified-revenue semantics.

- Generate outside the public repository and outside GitHub Actions.
- Record only the public Program ID in source control.
- Never reuse it as a wallet, treasury, RevenueAuthority or deployment payer.
- Its final public key is frozen into `revenue_adapter::MAINNET_VERIFIER_PROGRAM`.

### 2. Revenue Adapter Program ID keypair

Purpose: fixes the program that enforces verifier-PDA authorization, receipts and atomic forwarding into the referral protocol.

- Generate separately from the verifier and referral Program IDs.
- Record only the public Program ID in source control.
- The production `RevenueAuthority` is derived from this final Program ID with seed `[b"revenue-authority"]`.
- That derived PDA, not any human wallet, is frozen as the referral protocol's `MAINNET_QUALIFIED_REVENUE_SOURCE`.

### 3. Service Referral Protocol Program ID keypair

Purpose: fixes the on-chain identity of the immutable referral/accounting engine.

- Generate separately from verifier and adapter identities.
- Record only the public Program ID in source control and `[programs.mainnet]`.
- Never reuse it as service treasury, RevenueAuthority, verifier authority or user wallet.

### 4. Temporary deployer / upgrade-authority keypair

Purpose: pays deployment rent/fees and temporarily retains upgrade ability during the tightly limited bytecode-verification and smoke window.

- Keep separate from all three Program ID keypairs and from the service treasury.
- One controlled temporary deployer may deploy multiple programs if operationally appropriate, but it must not become a permanent protocol administrator.
- Fund it only after exact final artifact sizes and current mainnet requirements are measured.
- Remove upgrade authority from every production program only after the complete chain passes verification and smoke tests.

### 5. Service Treasury

Permanent configured economic recipient:

`AepYo8xanmKuRiLVeYQuCTJoQr1nyKiTApoKwHMEg8fn`

- It is not any Program ID.
- It is not RevenueAuthority.
- It is not the verifier authority.
- It is not deployment or upgrade authority.
- Its private key/seed is never needed by build or deployment workflows.

## Offline Program ID generation

On a controlled machine with the pinned Solana CLI, generate three distinct final program keypairs to explicit offline paths. Example shape only:

```text
solana-keygen new --no-bip39-passphrase --outfile <OFFLINE_PATH>/revenue-verifier-mainnet-program.json
solana-keygen new --no-bip39-passphrase --outfile <OFFLINE_PATH>/revenue-adapter-mainnet-program.json
solana-keygen new --no-bip39-passphrase --outfile <OFFLINE_PATH>/service-referral-mainnet-program.json

solana-keygen pubkey <OFFLINE_PATH>/revenue-verifier-mainnet-program.json
solana-keygen pubkey <OFFLINE_PATH>/revenue-adapter-mainnet-program.json
solana-keygen pubkey <OFFLINE_PATH>/service-referral-mainnet-program.json
```

Do not paste secret arrays, seed phrases or keypair-file contents into GitHub, ChatGPT, CI variables, issues, audit reports or release manifests.

## Identity dependency order

Once public keys are known:

1. freeze the final verifier `declare_id!`;
2. freeze that exact verifier Program ID in the adapter's `MAINNET_VERIFIER_PROGRAM`;
3. freeze the final adapter `declare_id!`;
4. derive the adapter `RevenueAuthority` PDA from `[b"revenue-authority"]` and the final adapter Program ID;
5. freeze the exact adapter Program ID in the referral core's `MAINNET_REVENUE_ADAPTER_PROGRAM`;
6. freeze the derived RevenueAuthority PDA as `MAINNET_QUALIFIED_REVENUE_SOURCE`;
7. freeze the final referral `declare_id!` and matching `[programs.mainnet]` entry;
8. freeze the exact registration opening UTC;
9. keep treasury and canonical mainnet USDT/USDC mints unchanged.

The repository release-binding tests must derive the same RevenueAuthority PDA. Do not type or guess that PDA manually as an independent configuration value.

## Final source freeze

The final reviewed commit must contain:

- FINAL `QUALIFIED_REVENUE_QUALIFICATION_SPEC.md`;
- production verifier implementation matching that specification;
- all three final public Program IDs;
- frozen verifier -> adapter and adapter -> referral bindings;
- frozen RevenueAuthority PDA;
- frozen registration opening UTC;
- exact dependency lockfiles used for the audited release;
- release manifest fields corresponding to the exact audited source.

After this freeze, run all reference, static, Rust, property, LiteSVM, cross-program adversarial, RustSec and verifiable-build gates again. Any source/config/dependency change invalidates previous artifact hashes and requires full rebuilding and, where material, renewed audit review.

## Funding rule

Do not pre-fund the temporary deployer from an estimate based on earlier artifacts. Immediately before controlled mainnet deployment:

1. measure the exact final `.so` sizes for verifier, adapter and referral;
2. query current mainnet rent/fee requirements using the pinned Solana CLI/RPC;
3. fund only the temporary deployer with required SOL plus a small explicit fee margin;
4. keep service treasury and RevenueAuthority economics independent from deployment SOL.

Devnet evidence proves workflow behavior, not the authoritative mainnet funding amount.

## Controlled deployment sequence

1. `python3 scripts/pre-mainnet-gate.py` must return `READY FOR CONTROLLED MAINNET DEPLOYMENT`.
2. Deploy the exact audited verifier `.so` with its offline Program ID keypair and temporary authority.
3. Deploy the exact audited referral `.so` with its offline Program ID keypair and temporary authority.
4. Deploy the exact audited adapter `.so` with its offline Program ID keypair and temporary authority.
5. Record Program IDs, ProgramData addresses, deployment slots, transaction signatures and current upgrade authorities for all three.
6. Dump/verify every deployed bytecode image against its exact audited hash before initialization.
7. Confirm the on-chain adapter Program ID derives the exact RevenueAuthority PDA frozen in the referral core.
8. Initialize only with the frozen production constants and executable dependencies.
9. Execute the limited full-chain mainnet smoke plan.
10. Stop on any bytecode, Program ID, PDA, state, token balance, receipt, authorization or accounting mismatch.
11. Only after all checks pass, remove upgrade authority permanently from verifier, adapter and referral.
12. Verify `solana program show <PROGRAM_ID>` for each program reports no upgrade authority and archive the public evidence.

Because adapter initialization requires the verifier and referral programs to be executable, both dependencies must be deployed before adapter initialization. Deployment itself and initialization are separate operations.

## Secret-handling rule

No repository workflow should require any final Program ID private key or permanent wallet seed. CI and verifiable builds operate from public source only. Final secret material is used locally/offline solely for signing operations that genuinely require it.
