# Final Program ID and Deployment-Key Custody Runbook

Status: procedure defined; **no final mainnet secret keypair belongs in this repository**.

## Production identities

### 1. Service Referral Protocol Program ID keypair

Purpose: fixes the on-chain identity of the final immutable purchase/referral/accounting program.

- Generate offline, outside the public repository and outside GitHub Actions.
- Commit only its public key via `declare_id!` and the matching `[programs.mainnet]` entry.
- Never reuse this keypair as a user wallet, service treasury or deployment payer.
- Preserve the keypair offline even after immutability for historical custody/audit evidence; it will no longer provide upgrade power after upgrade authority is removed.

### 2. Temporary deployer / upgrade-authority keypair

Purpose: pays deployment costs and temporarily retains upgrade authority during the tightly limited verification/smoke window.

- Generate separately from the Program ID keypair and service treasury.
- Fund only after measuring the exact final artifact and current Solana mainnet requirements.
- Do not make it a protocol administrator; the final on-chain program exposes no permanent admin control surface.
- Permanently remove upgrade authority only after deployed bytecode and the limited mainnet smoke are verified.

### 3. Service treasury

Frozen recipient:

`AepYo8xanmKuRiLVeYQuCTJoQr1nyKiTApoKwHMEg8fn`

The treasury is not a Program ID and is not the deployment/upgrade authority. Its private key is not needed for program build or deployment.

## Offline Program ID generation

Example shape on a controlled machine with the pinned Solana CLI:

```text
solana-keygen new --no-bip39-passphrase --outfile <OFFLINE_PATH>/service-referral-mainnet-program.json
solana-keygen pubkey <OFFLINE_PATH>/service-referral-mainnet-program.json

solana-keygen new --no-bip39-passphrase --outfile <OFFLINE_PATH>/temporary-mainnet-deployer.json
solana-keygen pubkey <OFFLINE_PATH>/temporary-mainnet-deployer.json
```

The exact filenames/paths are operational examples only. Do not paste secret arrays, seed phrases or keypair-file contents into GitHub, ChatGPT, CI, issues, PRs, audit reports or release manifests.

## Public identity freeze sequence

Once the final Program ID public key is known:

1. replace the development `declare_id!` with the final public Program ID;
2. add the exact same public key under `[programs.mainnet]` in `Anchor.toml`;
3. keep the frozen service treasury unchanged;
4. keep the canonical mainnet USDT/USDC mints unchanged;
5. freeze the future registration-opening UTC timestamp;
6. run all static/reference/Rust/LiteSVM/security/verifiable-build gates again;
7. obtain/refresh independent audit approval for the exact resulting commit;
8. complete `release/mainnet-release.json` using only public identifiers and hashes.

Any code/configuration/dependency change after the audited freeze creates a new candidate release and invalidates the previous build hash.

## Funding rule

Do not choose deployment SOL from a historical estimate. Immediately before controlled mainnet deployment:

1. build the exact final audited `.so`;
2. measure its exact size;
3. query current mainnet deployment/rent/fee requirements with the pinned Solana CLI/RPC;
4. fund the temporary deployer with the required SOL plus a small explicit operating margin;
5. keep deployment SOL separate from USDT/USDC protocol economics.

Users pay their own Solana transaction fees for registration, purchases and claims after launch. The service treasury is not intended to subsidize routine user gas.

## Controlled deployment sequence

1. `python3 scripts/pre-mainnet-gate.py` must exit successfully.
2. Confirm the local source commit equals `release/mainnet-release.json.commit_sha`.
3. Confirm the local production `.so` SHA-256 equals the manifest hash.
4. Deploy that exact `.so` using the offline Program ID keypair and temporary deployer/upgrade authority.
5. Record Program ID, ProgramData address, deployment slot, deployment transaction and current upgrade authority.
6. Fetch/dump deployed bytecode and verify it against the exact audited artifact/hash.
7. Initialize only with frozen treasury, canonical mints and frozen registration opening time.
8. Run the limited mainnet smoke plan.
9. Stop on any Program ID, PDA, bytecode, token-balance, genealogy, unit-ID or accounting mismatch.
10. After successful smoke and verification, permanently remove upgrade authority.
11. Verify `solana program show <PROGRAM_ID>` reports no upgrade authority and archive that public evidence.

## Permanent immutability command shape

Use the exact pinned Solana CLI syntax reviewed at release time. The intended final operation is equivalent to:

```text
solana program set-upgrade-authority <PROGRAM_ID> --final
```

Do not execute the final authority removal before bytecode verification and smoke testing because it is irreversible.

## Secret-handling rule

CI and verifiable builds operate from public source only. No workflow requires the final Program ID private key or temporary deployer private key. Secret material is used only on the controlled deployment machine for signing operations that genuinely require it.
