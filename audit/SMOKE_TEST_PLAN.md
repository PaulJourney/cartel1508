# Mainnet Smoke Test Plan — v1

Status: **DRAFT — awaiting owner approval** (`smoke_test_plan_approved` stays `false` until ratified).

Per runbook §16.20: deliberately small, controlled-wallet only, and **must not consume Pioneer capacity** (every purchase far below 1,000 units). Registration must be open, so this runs on **1 September 2026 after 05:00 UTC (07:00 CEST)**, before permanent upgrade-authority removal.

## Preconditions

- Program deployed at `DA214e5LFbj1WARXzu89k295azhKCGMFcVicsm7CsfpL`; on-chain bytecode verified byte-identical to audited artifact `0c632ade…3369` (runbook §16.16).
- Frozen initialization completed **before 2026-09-01 05:00 UTC**; ProtocolState fields verified (§16.18).
- All four canonical vault/Treasury ATAs exist and verified (§16.19).
- Two fresh controlled wallets, funded: **A** (sponsor) and **B** (buyer), each with ~0.05 SOL fees; A: 10 USDC; B: 10 USDC.

## Steps and exact expectations (USDC path)

| # | Action | Expected |
|---|---|---|
| 1 | A `register` under technical root (zero pubkey) | success; `real_user_count = 1` |
| 2 | A `purchase_and_distribute` 10 units | success; A ACTIVE week 1; A SELF +5.000000; vault +5.000000; Treasury +5.000000 (4.3 unallocated + 0.5 service + 0.2 Pioneer-unassigned); `next_unit_id = 11`; `pioneer_positions_assigned = 0` |
| 3 | B `register` under A | success; `real_user_count = 2` |
| 4 | B `purchase_and_distribute` 10 units | success; B SELF +5; A `network_claimable` +1.5 (U1, depth U1–U3 unlocked by A's 10 units); Treasury +3.5 (2.8 unallocated U2–U9 + 0.5 service + 0.2 Pioneer); vault +6.5 |
| 5 | A `claim` (USDC) | success; A receives exactly 6.500000 (5 SELF + 1.5 network); vault −6.5 |
| 6 | A immediate second `claim` | **fails** `NothingToClaim` (6019); no balance change |
| 7 | B `claim` | success; B receives 5.000000; vault balance = 0 for USDC |
| 8 | `settle_expired` on A | **fails** `NothingToSettle` (6017) |

Total spend: 20 USDC (of which 11.5 returns to controlled wallets, 8.5 to Treasury) + ~0.01 SOL fees.

## Abort criteria

Any deviation — unexpected error code, any balance differing by even one atom, any state field mismatch — is a **hard stop**: do **not** remove upgrade authority; investigate with the program still upgradeable.

## After PASS

Re-verify bytecode + ProgramData authority (§16.21), then permanently remove upgrade authority (§16.22–23) and archive public evidence (signatures of every smoke transaction + final `solana program show`).
