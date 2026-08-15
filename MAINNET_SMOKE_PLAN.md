# Draft limited mainnet smoke plan

Status: **DRAFT / NOT APPROVED**. This document defines the smallest post-deployment verification sequence that may be considered after the exact audited three-program bytecodes are deployed and verified. It does not authorize deployment, spending, registration, qualified-revenue creation or upgrade-authority removal.

## Preconditions — hard stop unless all are true

- `python3 scripts/pre-mainnet-gate.py` returned `READY FOR CONTROLLED MAINNET DEPLOYMENT` for the exact deployment commit.
- The exact referral, revenue-adapter and revenue-qualification `.so` files named by the release manifest have been deployed.
- All three deployed bytecodes have already been verified against the exact audited/verifiable artifact SHA-256 values.
- All three Program IDs match source, `Anchor.toml` and `release/mainnet-release.json`.
- Adapter Revenue Authority PDA and Qualification Authority PDA independently derived from the deployed Program IDs match the release manifest and on-chain configuration.
- Treasury and canonical USDT/USDC mints have been independently re-verified immediately before execution.
- Registration opening UTC has been intentionally selected and checked.
- The concrete production qualified-revenue evidence model has independent approval and matches the hashed specification in the release manifest.
- Temporary upgrade authority remains available. No `--final` command has been executed.
- Operators have agreed exact transaction signers, stablecoin rail and maximum economic exposure before any transaction is sent.

If any precondition fails, stop. Do not “smoke through” a mismatch.

## Pioneer warning

The first 100 real registrations receive Pioneer IDs. A disposable smoke-test registration would permanently consume one of those scarce IDs.

Therefore either:

1. the registration portion of the mainnet smoke uses an **intended real production participant** that should legitimately receive the next Pioneer ID; or
2. registration is omitted from the live smoke because registration semantics were already established by the audited build and the production rollout intentionally preserves Pioneer slots.

This choice is a business/release decision and must be recorded before execution. Never create a throwaway mainnet wallet merely to prove `register()` works.

## Economic exposure limits

Keep all live amounts to the minimum needed to prove the audited path. Exact values must be written into the approved copy of this plan before execution.

Recommended principles:

- use only one stablecoin rail for the initial smoke;
- no large-value test transfers;
- do not fund accounts beyond the exact approved smoke requirement plus transaction-fee margin;
- do not use service-unit purchase funds as qualified-revenue funding;
- use a legitimate production evidence event, never a fabricated event solely to make the test pass;
- do not send a second qualified-revenue event unless it is explicitly part of the approved anti-replay/failure test and economically legitimate.

## Phase A — read-only deployment verification

Record before any state-changing protocol transaction:

- current slot/block time;
- all three Program IDs;
- all three ProgramData addresses;
- all three current upgrade authorities;
- deployed bytecode verification result for each program;
- Protocol PDA;
- AdapterConfig PDA;
- Revenue Authority PDA;
- Qualification Authority PDA;
- vault authority PDA;
- canonical stablecoin mint selected for smoke;
- canonical Revenue Authority ATA for that mint;
- canonical protocol vault ATA for that mint;
- canonical service-treasury ATA for that mint.

Every address must match the audited derivation. A mismatch is a hard stop.

## Phase B — controlled initialization

If the programs are not yet initialized, initialize only with the release-manifest values.

After initialization, read back and verify:

- service treasury;
- USDT and USDC mints;
- qualified revenue source == derived Revenue Authority PDA;
- registration opening timestamp;
- adapter referral Program ID;
- adapter qualification Program ID;
- adapter Qualification Authority PDA;
- adapter Revenue Authority PDA.

No value-transfer smoke begins until these reads match exactly.

## Phase C — service-unit isolation

Only if the selected production participant is intentionally allowed to become active during smoke:

- purchase exactly the approved minimum service units needed for the intended activity state;
- verify 1 stablecoin == 1 logical service unit;
- verify the unit batch records the expected globally unique Unit ID range;
- verify service-unit payment reaches the service treasury;
- verify the referral reward vault receives **zero** qualified-revenue funding from this purchase;
- verify expected ACTIVE/qualification timestamps.

If a live unit purchase is not operationally appropriate, omit this phase rather than inventing a fake participant.

## Phase D — one legitimate qualified-revenue event

Use exactly one small, legitimate production revenue event under the independently approved evidence model.

Before submission record:

- immutable external/on-chain evidence identifier;
- payer;
- beneficiary;
- stablecoin mint;
- gross amount;
- expected deterministic event/receipt identity;
- expected pre-transaction balances.

Execute the production qualification -> adapter -> referral path.

Immediately verify:

- payer was charged exactly the approved gross amount;
- adapter Revenue Authority ATA does not retain an unexplained balance after downstream accounting;
- adapter receipt exists at exactly the expected PDA;
- referral accounting conserves the gross amount exactly;
- direct/network/Pioneer/service allocations match the audited 50/43/2/5 rules and current activity state;
- treasury change equals the expected immediate treasury allocation;
- protocol vault balance equals outstanding user liabilities only.

Any unexplained atomic-unit difference is a hard stop.

## Phase E — anti-replay

Only replay the **same already-legitimate evidence event** if the approved evidence and product semantics permit a harmless duplicate-submission test.

Expected result:

- transaction rejected;
- payer balance unchanged;
- treasury unchanged;
- reward vault unchanged;
- Revenue Authority ATA unchanged;
- no second receipt/accounting mutation.

Do not create a new nonce/evidence identifier and call that an anti-replay test. The production evidence model must make the underlying economic event itself idempotent.

If duplicate live submission is operationally unsafe, prove anti-replay from read-only receipt/evidence state and omit the transaction.

## Phase F — failure/rollback check

A deliberately invalid live transaction should be attempted only if it can be constructed without risking a legitimate payment, Pioneer slot or irreversible external business state.

If approved, use a failure that occurs inside the audited on-chain chain and verify:

- payer/token movement is rolled back;
- no receipt survives;
- treasury and reward vault are unchanged;
- no user liability is created.

Otherwise omit the live negative test and rely on exact audited real-validator evidence for this property.

## Phase G — claim

Only if the intended production participant is ACTIVE and the smoke legitimately created claimable value:

- record claimable amounts before claim;
- submit one user-signed claim;
- verify destination is the canonical ATA;
- verify exact payout;
- verify vault decreases by exactly the payout;
- verify no unrelated accounting state changes.

Do not manufacture extra revenue merely to make a claim possible.

## Phase H — reconciliation and sign-off

Before any authority-removal command, record a reconciliation containing:

- every smoke transaction signature;
- pre/post token balances;
- every created PDA/receipt/batch;
- exact expected vs observed atomic-unit accounting;
- Program IDs and deployed-bytecode hashes;
- current upgrade authorities;
- any omitted smoke phase and the reason it was omitted;
- explicit statement that the observed production state matches the audited baseline.

If any discrepancy remains unresolved, stop and retain upgrade authority.

## Permanent authority removal — separate irreversible action

This smoke plan does **not** itself authorize `--final`.

Only after deployment verification + approved smoke + reconciliation are all signed off may the release owner execute the separately controlled authority-removal procedure in `MAINNET_FINALIZATION_RUNBOOK.md`.

Remove authority only from the exact programs intended to become immutable, verify each resulting authority state, and archive the evidence. Never finalize one program while another program still needs a cross-program binding or code correction.

## Approval fields — intentionally blank

Before mainnet execution, the approved copy must record:

- release commit: `<UNSET>`
- referral Program ID: `<UNSET>`
- adapter Program ID: `<UNSET>`
- qualification Program ID: `<UNSET>`
- selected stablecoin rail: `<UNSET>`
- maximum stablecoin exposure: `<UNSET>`
- registration/Pioneer decision: `<UNSET>`
- exact legitimate qualification evidence source: `<UNSET>`
- smoke executor(s): `<UNSET>`
- smoke approver(s): `<UNSET>`
- approval date/time UTC: `<UNSET>`

Until these fields are intentionally completed and approved, `smoke_test_plan_approved` in the final release manifest must remain `false`.
