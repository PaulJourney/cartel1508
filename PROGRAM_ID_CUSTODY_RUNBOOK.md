# Final Program IDs and deployment-key custody runbook

Status: procedure defined; no final mainnet Program ID private keypair for any production program is generated, stored or committed by this repository.

## Public identities in the final release

The mainnet system now contains three executable programs whose identities are mutually bound:

1. `service_referral_protocol`
2. `revenue_adapter`
3. `revenue_qualification`

Each requires its own final Program ID keypair. None of those private keypairs belongs in GitHub, CI, ChatGPT, release artifacts or ordinary operational wallets.

The adapter's deterministic `RevenueAuthority` PDA is derived from the **final revenue-adapter Program ID** and is frozen into the referral protocol as `MAINNET_QUALIFIED_REVENUE_SOURCE`. The qualification program's deterministic `qualified-revenue-authority` PDA is derived from the **final qualification Program ID** and is frozen into the adapter configuration/binding. These PDAs have no private keys.

## Identities and roles that must remain separate

### 1. Three final Program ID keypairs

Purpose: fix the on-chain address of each executable during initial deployment.

- Generate all three on a controlled offline/user-owned machine outside the public repository and GitHub Actions.
- Use a distinct keypair for referral, adapter and qualification.
- Never reuse any Program ID keypair as service treasury, Revenue Authority, Qualification Authority, deployer, user wallet or upgrade authority.
- Record only the three public Program IDs in the final reviewed source freeze.
- Keep the private Program ID keypairs offline through initial deployment and bytecode verification.

Example command shapes only:

```text
solana-keygen new --no-bip39-passphrase --outfile <OFFLINE_PATH>/referral-mainnet-program.json
solana-keygen new --no-bip39-passphrase --outfile <OFFLINE_PATH>/revenue-adapter-mainnet-program.json
solana-keygen new --no-bip39-passphrase --outfile <OFFLINE_PATH>/revenue-qualification-mainnet-program.json

solana-keygen pubkey <OFFLINE_PATH>/referral-mainnet-program.json
solana-keygen pubkey <OFFLINE_PATH>/revenue-adapter-mainnet-program.json
solana-keygen pubkey <OFFLINE_PATH>/revenue-qualification-mainnet-program.json
```

Do not paste secret arrays, seed phrases or files into GitHub, ChatGPT, CI variables, issues, PRs or audit artifacts.

### 2. Temporary deployer / upgrade authority

Purpose: pays deployment rent/fees and temporarily retains the ability to correct a deployment during the tightly limited verification/smoke window.

- Generate separately from all three Program ID keypairs.
- It may be used as the temporary upgrade authority for the three programs if the final deployment procedure chooses one shared temporary authority; that does not make it a protocol administrator.
- Fund only after exact final artifact sizes and current mainnet requirements have been measured.
- After all three deployed bytecodes and the cross-program initialization are verified and the limited mainnet smoke passes, remove upgrade authority from each program intended to become immutable.
- `--final` is irreversible for that program.

### 3. Service treasury

Permanent configured economic recipient:

`AepYo8xanmKuRiLVeYQuCTJoQr1nyKiTApoKwHMEg8fn`

- Not a Program ID.
- Not an upgrade authority.
- Not the qualified-revenue source.
- Its private key/seed is never required for build or deployment.

### 4. Revenue Authority PDA

- Derived deterministically under the final revenue-adapter Program ID using the reviewed seed.
- Has no private key.
- Its canonical USDT/USDC token accounts receive separately qualified revenue before downstream accounting.
- The exact PDA public key is frozen as `MAINNET_QUALIFIED_REVENUE_SOURCE` in the referral protocol.

### 5. Qualification Authority PDA

- Derived deterministically under the final revenue-qualification Program ID using the reviewed seed.
- Has no private key.
- It signs only through qualification-program PDA signer seeds during the CPI into the revenue adapter.

## Final source freeze after the three public Program IDs are known

In one reviewed change set:

1. replace all three development `declare_id!` values with final Program IDs;
2. add all three exact public IDs under `[programs.mainnet]` in `Anchor.toml`;
3. freeze adapter `MAINNET_REFERRAL_PROGRAM` to the final referral Program ID;
4. freeze adapter `MAINNET_QUALIFICATION_PROGRAM` to the final qualification Program ID;
5. freeze qualification `MAINNET_ADAPTER_PROGRAM` to the final adapter Program ID;
6. derive the final adapter Revenue Authority PDA and freeze it as core `MAINNET_QUALIFIED_REVENUE_SOURCE`;
7. freeze the exact registration opening UTC;
8. keep treasury and canonical USDT/USDC mints unchanged unless a separately reviewed release explicitly changes them;
9. freeze the approved qualification/evidence specification and record its SHA-256 in the release manifest;
10. rerun reference, static, Rust, property, security, real-validator smoke and verifiable-build gates;
11. submit the exact final commit and all three artifact hashes to the independent auditor/final review;
12. complete `release/mainnet-release.json` only from verified public evidence.

Any source/configuration change after this freeze invalidates prior artifact hashes and requires the affected review/build evidence to be repeated.

## Funding rule

Do not pre-fund the temporary deployer from estimates based on development artifacts. Immediately before mainnet deployment:

1. measure the exact three final `.so` sizes;
2. query current mainnet rent/fee requirements using the pinned Solana CLI/RPC;
3. calculate the full requirement for all three deploys plus the explicitly approved smoke transactions;
4. fund only the temporary deployer with the required SOL plus a small documented transaction-fee margin;
5. keep treasury and Program ID custody independent from deployment funds.

## Controlled deployment sequence

1. `python3 scripts/pre-mainnet-gate.py` must return `READY FOR CONTROLLED MAINNET DEPLOYMENT`.
2. Deploy the exact three verified `.so` artifacts using their respective offline Program ID keypairs and the temporary deployer/authority.
3. Record each Program ID, ProgramData address, deployment slot, transaction signature and upgrade authority.
4. Dump/verify each deployed bytecode against its exact audited artifact hash.
5. Verify the three deployed identities equal the frozen source/manifest/Anchor identities.
6. Initialize only with the frozen production cross-program bindings, Revenue Authority PDA, treasury/mints and registration UTC.
7. Verify the resulting on-chain Revenue Authority and Qualification Authority PDA derivations.
8. Perform the limited mainnet smoke plan.
9. Stop if any state, token movement, PDA, account address, accounting result or bytecode differs from the audited baseline.
10. Only after all checks pass, remove upgrade authority permanently from every program intended to be immutable.
11. Run `solana program show <PROGRAM_ID>` for all three and archive evidence that the expected upgrade authorities are absent.

## Secret-handling rule

No workflow in this repository should ever require a final Program ID private key or permanent wallet seed. CI and verifiable builds operate from public source only. Final secret material is used locally/offline only for the minimum signing operations that genuinely require it.
