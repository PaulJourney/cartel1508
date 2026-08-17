# Pre-Mainnet Definitive Validation Matrix

Status: **Mainnet must remain blocked until every mandatory row below has passing evidence on source-equivalent final code, followed by exact-head CI/verifiable evidence after the Mainnet Program ID + registration timestamp freeze.**

## Validation philosophy

No test-only clock or privileged bypass is permitted in the production Solana program.

Therefore validation is deliberately split into two complementary layers:

1. **Real Solana Devnet** for actual signatures, transaction execution, SPL token accounts, PDA/account constraints, canonical vault/treasury routing, multi-wallet genealogy, USDT/USDC branches, rollback behavior, Pioneer checkpoints and end-to-end token conservation.
2. **Exact-program LiteSVM** for deterministic time travel across the real 7-day ACTIVE and 48-hour GRACE windows. Devnet cannot safely warp `Clock` without changing the program under test, so time-dependent behavior must be tested with the same compiled program bytes in LiteSVM rather than adding a test backdoor.

A passing Devnet run alone is not sufficient; a passing LiteSVM run alone is not sufficient. Both are mandatory.

## Real Devnet comprehensive suite

Executable: `scripts/devnet-comprehensive-validation.mjs`

Mandatory coverage:

- initialize a fresh temporary-ID deployment;
- create separate 6-decimal test USDT and USDC mints;
- create canonical PDA vaults and treasury ATAs;
- register a real 12-wallet genealogy: buyer + 11 ancestors;
- prove only U1-U9 can ever receive network value; U10/U11 remain outside the cap;
- set U4-U9 one unit below every depth boundary (24/49/99/199/349/499) and prove only U1-U3 receive the target purchase;
- add exactly one unit to cross all six boundaries and prove the next target purchase pays exact U1-U9 percentages;
- prove depth unlock is prospective and earlier locked value is not recovered;
- prove Treasury receives exactly service + locked/unallocated + unassigned Pioneer flow;
- prove ACTIVE purchases accumulate `current_week_units` without incrementing `active_weeks_started` again;
- prove `1 x 10` and `10 x 1` qualification produce equivalent SELF/ACTIVE state;
- prove USDT and USDC accrual/claim accounting is isolated per mint;
- reject zero-unit purchases;
- reject token-payment overflow / `PurchaseTooLarge` before transfer;
- reject tampered ancestry accounts and prove full transaction rollback;
- reject non-canonical vault accounts;
- reject unsupported mints;
- reject token accounts owned by the wrong wallet;
- prove failed purchases do not transfer funds, mutate network accruals, advance Unit IDs or increment purchase indexes;
- prove 500 + 500 separate purchases create zero Pioneer positions;
- prove one 1,000-unit purchase creates one Pioneer position only after its own Pioneer event (Rule B);
- prove a 2,000-unit purchase creates two positions;
- prove multiple wallets can own Pioneer positions simultaneously;
- prove weighted checkpoints when one wallet adds positions at different indexes;
- prove exact multi-wallet Pioneer claim amounts;
- prove 4 assigned positions -> 98 -> exactly 100 with a purchase that has more candidates than remaining slots;
- prove 100/100 is permanent and a later 5,000-unit purchase creates zero positions;
- prove fully assigned Pioneer pool creates no further unassigned Pioneer flow;
- perform complete active claim sweep;
- require both canonical stablecoin vaults to finish at zero liability after sweep;
- require `real_user_count` to equal actual registrations;
- require `next_unit_id = 1 + sum(all successful purchase units)`;
- require every `next_purchase_index` to equal the wallet's number of successful purchases;
- require end-to-end token conservation for test USDT, USDC and rejected unsupported mint.

The suite terminates successfully only when it prints:

`DEVNET PRE-MAINNET COMPREHENSIVE VALIDATION: PASS`

## Exact-time LiteSVM suite

Mandatory coverage in `integration-tests/**`:

- progressive ACTIVE ladder `10,10,20,20,30,30,40,40,50...`;
- week counter advances only on successfully-started ACTIVE cycles;
- long calendar inactivity alone never advances the requirement;
- purchases while ACTIVE increase current-week volume/depth and do not prequalify another week;
- partial qualification below requirement remains INACTIVE;
- stale partial qualification windows reset;
- ACTIVE lasts the real seven-day duration;
- GRACE lasts the real 48-hour duration;
- claim is ACTIVE-only;
- GRACE preserves eligible SELF/network due but does not permit claim;
- expired SELF/network value is permissionlessly settleable once and only once;
- late reactivation cannot rescue expired value;
- Pioneer positions remain permanently owned through inactivity;
- inactive Pioneer due expires to Treasury;
- late reactivation cannot recover historical Pioneer due;
- permanent Pioneer positions resume earning only prospectively after reactivation;
- exact status boundaries: `active_until` is still ACTIVE, `active_until + 1` is GRACE, `grace_until` is still GRACE, `grace_until + 1` is INACTIVE;
- zero/uninitialized deadlines never create accidental ACTIVE/GRACE status.

Dedicated exact-boundary test: `integration-tests/tests/activity_status_boundaries.rs`.

## Existing deterministic / adversarial suite

The final gate also requires all existing tests to remain green, including:

- `purchase_distribution.rs`;
- `deep_network.rs`;
- `progressive_activity.rs`;
- `grace_expiry.rs`;
- `claim_activity.rs`;
- `pioneer_positions.rs`;
- `pioneer_activity.rs`;
- `batching_equivalence.rs`;
- `ancestry_rollback.rs`;
- `initialization_security.rs`;
- reference economic model;
- rank/badge model;
- static security/economic gates;
- Rust unit/property tests including large randomized split-conservation and Unit-ID range checks;
- RustSec dependency scan;
- protocol production build;
- independent verifiable build;
- byte-identical `.so` comparison / SHA-256 evidence.

## Release decision

A comprehensive validation PASS means the implementation has passed the project's internal pre-mainnet engineering battery. It does **not** replace the independent external audit.

Mainnet remains blocked until:

1. comprehensive real Devnet PASS;
2. exact-time/adversarial LiteSVM PASS;
3. protocol CI + RustSec + verifiable build PASS;
4. final Mainnet Program ID generated offline and only public ID committed;
5. future registration-open UTC timestamp frozen;
6. exact freeze commit rebuilt/retested and `.so` hash frozen;
7. independent third-party audit completed and findings dispositioned;
8. executable release manifest/gate fully green;
9. deployed Mainnet bytecode verified before initialization;
10. deliberately small Mainnet smoke passes with temporary upgrade authority;
11. deployed state/bytecode reverified;
12. upgrade authority removed only as the final irreversible action.
