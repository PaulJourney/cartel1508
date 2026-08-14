# Final Program ID and deployment-key custody runbook

Status: procedure defined; no final mainnet Program ID keypair has been generated or committed by this repository.

## Three identities that must remain separate

### 1. Final Program ID keypair

Purpose: fixes the on-chain address of the executable program during its initial deployment.

- Generate outside the public repository and outside GitHub Actions.
- Never fund this keypair merely because it is the Program ID.
- Never reuse it as service treasury, qualified-revenue source or user wallet.
- Record only its public key in source control.
- Keep the private key offline until initial deployment and bytecode verification are complete.

### 2. Temporary deployer / upgrade-authority keypair

Purpose: pays deployment rent/fees and temporarily retains the ability to correct a deployment during the tightly limited verification/smoke window.

- Generate separately from the Program ID keypair.
- Fund only after the final production artifact size and required mainnet balance have been measured.
- This key is not a permanent protocol administrator.
- After deployed-bytecode verification and controlled mainnet smoke tests, remove upgrade authority with `solana program set-upgrade-authority <PROGRAM_ID> --final`.
- After `--final`, no upgrade-authority private key can restore upgradeability.

### 3. Service Treasury

Permanent configured economic recipient:

`AepYo8xanmKuRiLVeYQuCTJoQr1nyKiTApoKwHMEg8fn`

- This wallet is not the Program ID.
- This wallet is not the final upgrade authority.
- This wallet must not be the production qualified-revenue source.
- Its private key/seed is never needed by the program build or deployment process.

## Offline Program ID generation

On a controlled machine with the pinned Solana CLI, generate the final program keypair to an explicitly chosen offline path. Example shape only:

```text
solana-keygen new --no-bip39-passphrase --outfile <OFFLINE_PATH>/service-referral-mainnet-program.json
solana-keygen pubkey <OFFLINE_PATH>/service-referral-mainnet-program.json
```

Do not paste the generated secret array, seed phrase or file contents into GitHub, ChatGPT, CI variables, issue comments or audit artifacts.

Record the resulting public Program ID in the reviewed final-freeze change set only.

## Final source freeze after public Program ID is known

In one reviewed commit:

1. replace the development `declare_id!` with the final Program ID;
2. add/update `[programs.mainnet]` in `Anchor.toml` with exactly the same Program ID;
3. freeze the independently audited qualified-revenue-source PDA;
4. freeze the exact registration opening UTC;
5. keep the already frozen treasury and mainnet USDT/USDC mints unchanged;
6. run all reference, static, Rust, property, LiteSVM, RustSec and verifiable-build gates again;
7. provide that exact commit and artifact hash to the independent auditor/final review;
8. complete `release/mainnet-release.json` only from verified public evidence.

Any source/configuration change after this freeze invalidates the previous artifact hash and requires a full rebuild/reverification.

## Funding rule

Do not pre-fund the temporary deployer based on an estimate from an earlier build. Immediately before mainnet deployment:

1. measure the exact final `.so` size;
2. query the current mainnet rent/fee requirements using the pinned Solana CLI/RPC;
3. fund only the temporary deployer with the required SOL plus a small explicit transaction-fee margin;
4. keep the treasury wallet independent from these deployment funds.

The successful devnet transaction smoke is useful evidence for workflow correctness, but it is not the authoritative mainnet funding quote.

## Deployment sequence

1. `python3 scripts/pre-mainnet-gate.py` must return `READY FOR CONTROLLED MAINNET DEPLOYMENT`.
2. Deploy the exact verified `.so` with the offline final Program ID keypair and temporary deployer/authority.
3. Record Program ID, ProgramData address, deployment slot, transaction signature and current upgrade authority.
4. Dump/verify deployed bytecode against the exact audited artifact hash.
5. Initialize only with the frozen production constants.
6. Perform the limited mainnet smoke plan.
7. Stop if any state, account address, accounting result or bytecode differs from the audited baseline.
8. Only after all checks pass, remove upgrade authority permanently with `--final`.
9. Verify `solana program show <PROGRAM_ID>` reports no upgrade authority and archive the public evidence.

## Secret-handling rule

No workflow in this repository should ever require the final Program ID private key or permanent wallet seed. CI builds and verifiable builds operate from public source only. The final secret material is used locally/offline only for the minimum signing operations that genuinely require it.
