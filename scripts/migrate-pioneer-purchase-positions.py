from pathlib import Path

ROOT = Path('.')

def replace_once(text, old, new, label):
    count = text.count(old)
    if count != 1:
        raise SystemExit(f'{label}: expected exactly once, found {count}')
    return text.replace(old, new, 1)

# Rename the old one-slot-per-user model everywhere relevant first.
for p in ROOT.rglob('*'):
    if not p.is_file() or any(part in {'.git', 'target', 'node_modules'} for part in p.parts):
        continue
    if p.suffix not in {'.rs', '.py', '.mjs', '.md'}:
        continue
    text = p.read_text(errors='ignore')
    new = (text
        .replace('pioneer_id', 'pioneer_positions')
        .replace('pioneerId', 'pioneerPositions')
        .replace('pioneer_count', 'pioneer_positions_assigned')
        .replace('pioneerCount', 'pioneerPositionsAssigned'))
    if new != text:
        p.write_text(new)

# Constants: positions are earned only from a single >=1000-unit purchase.
p = Path('programs/service_referral_protocol/src/constants.rs')
text = p.read_text()
text = replace_once(
    text,
    'pub const PIONEER_SLOTS: u16 = 100;\npub const PIONEER_SCALE: u128 = 1_000_000_000_000_000_000;\n',
    'pub const PIONEER_SLOTS: u16 = 100;\n/// One Pioneer position is earned for each complete 1,000 units in one purchase.\n/// Purchases never accumulate toward this threshold across transactions.\npub const PIONEER_POSITION_PURCHASE_UNITS: u64 = 1_000;\npub const PIONEER_SCALE: u128 = 1_000_000_000_000_000_000;\n',
    'pioneer threshold constant',
)
p.write_text(text)

# State comments: the u16 field size is unchanged, so account SPACE stays stable.
p = Path('programs/service_referral_protocol/src/state.rs')
text = p.read_text()
text = text.replace('    pub pioneer_positions_assigned: u16,', '    /// Number of the 100 Pioneer positions permanently assigned so far.\n    pub pioneer_positions_assigned: u16,')
text = text.replace('    pub pioneer_positions: u16,', '    /// Number of Pioneer positions owned by this wallet. One wallet may own many.\n    pub pioneer_positions: u16,')
p.write_text(text)

# Pure cap/threshold math so boundary behavior is independently unit tested.
p = Path('programs/service_referral_protocol/src/math.rs')
text = p.read_text()
insert_after = '''pub fn allocate_unit_range(next_unit_id: u128, units: u64) -> Result<(u128, u128, u128)> {
    if units == 0 {
        return err!(ProtocolError::ZeroUnits);
    }
    let first = next_unit_id;
    let next = first
        .checked_add(units as u128)
        .ok_or(ProtocolError::ArithmeticOverflow)?;
    let last = next
        .checked_sub(1)
        .ok_or(ProtocolError::ArithmeticUnderflow)?;
    Ok((first, last, next))
}
'''
addition = insert_after + '''
/// Pioneer positions created by this purchase only. There is no cross-purchase
/// accumulation. The global cap of 100 is absolute.
pub fn pioneer_positions_for_purchase(units: u64, already_assigned: u16) -> u16 {
    if already_assigned >= PIONEER_SLOTS {
        return 0;
    }
    let requested = units / PIONEER_POSITION_PURCHASE_UNITS;
    let remaining = (PIONEER_SLOTS - already_assigned) as u64;
    requested.min(remaining) as u16
}
'''
text = replace_once(text, insert_after, addition, 'pioneer position helper')
anchor = '''    #[test]
    fn huge_batch_payment_fits_u64() {
'''
new_tests = '''    #[test]
    fn pioneer_positions_require_single_thousand_unit_purchase_and_cap_at_100() {
        assert_eq!(pioneer_positions_for_purchase(0, 0), 0);
        assert_eq!(pioneer_positions_for_purchase(50, 0), 0);
        assert_eq!(pioneer_positions_for_purchase(500, 0), 0);
        assert_eq!(pioneer_positions_for_purchase(999, 0), 0);
        assert_eq!(pioneer_positions_for_purchase(1_000, 0), 1);
        assert_eq!(pioneer_positions_for_purchase(2_000, 0), 2);
        assert_eq!(pioneer_positions_for_purchase(3_750, 0), 3);
        // Separate 500-unit purchases each remain individually ineligible.
        assert_eq!(pioneer_positions_for_purchase(500, 0), 0);
        assert_eq!(pioneer_positions_for_purchase(500, 0), 0);
        // Only two slots remain: a 3,000-unit purchase receives exactly two.
        assert_eq!(pioneer_positions_for_purchase(3_000, 98), 2);
        // Once 100/100 is reached the pool can never create another position.
        assert_eq!(pioneer_positions_for_purchase(1_000, 100), 0);
        assert_eq!(pioneer_positions_for_purchase(u64::MAX, 100), 0);
    }

''' + anchor
text = replace_once(text, anchor, new_tests, 'pioneer math tests')
p.write_text(text)

# Core program migration.
p = Path('programs/service_referral_protocol/src/lib.rs')
text = p.read_text()
old_register = '''        let pioneer_positions = if p.pioneer_positions_assigned < PIONEER_SLOTS {
            p.pioneer_positions_assigned = p
                .pioneer_positions_assigned
                .checked_add(1)
                .ok_or(ProtocolError::ArithmeticOverflow)?;
            p.pioneer_positions_assigned
        } else {
            0
        };
        p.real_user_count = p
'''
new_register = '''        // Registration alone never consumes a Pioneer position. Positions are
        // created only after a qualifying single purchase of >=1,000 units.
        p.real_user_count = p
'''
text = replace_once(text, old_register, new_register, 'remove registration Pioneer assignment')
text = replace_once(text, '        u.pioneer_positions = pioneer_positions;\n', '        u.pioneer_positions = 0;\n', 'registered user starts with zero positions')
text = replace_once(text, '        u.pioneer_checkpoint_usdt = p.pioneer_index_usdt;\n        u.pioneer_checkpoint_usdc = p.pioneer_index_usdc;\n', '        u.pioneer_checkpoint_usdt = 0;\n        u.pioneer_checkpoint_usdc = 0;\n', 'zero position checkpoints at registration')

# Add positions only AFTER the current purchase Pioneer index and treasury flow are settled.
old_metrics_emit = '''        add_protocol_treasury_metrics(
            p,
            mint,
            service,
            unallocated,
            rounding_remainder,
            pioneer_unassigned,
        )?;

        emit!(UnitsPurchased {
            buyer: ctx.accounts.wallet.key(),
            mint,
            purchase_index,
            units,
            first_unit_id,
            last_unit_id,
            purchased_at: now,
        });
'''
new_metrics_emit = '''        add_protocol_treasury_metrics(
            p,
            mint,
            service,
            unallocated,
            rounding_remainder,
            pioneer_unassigned,
        )?;

        // Rule B: this purchase may earn new Pioneer positions, but those positions
        // start at the already-advanced index and therefore earn only from the next
        // global purchase onward. The global 100-position cap is absolute.
        let pioneer_positions_added =
            assign_pioneer_positions_after_purchase(&mut ctx.accounts.user, p, units)?;
        let pioneer_positions_total = p.pioneer_positions_assigned;

        emit!(UnitsPurchased {
            buyer: ctx.accounts.wallet.key(),
            mint,
            purchase_index,
            units,
            first_unit_id,
            last_unit_id,
            pioneer_positions_added,
            pioneer_positions_total,
            purchased_at: now,
        });
'''
text = replace_once(text, old_metrics_emit, new_metrics_emit, 'post-purchase Pioneer assignment')
text = replace_once(
    text,
    '    pub last_unit_id: u128,\n    pub purchased_at: i64,\n',
    '    pub last_unit_id: u128,\n    pub pioneer_positions_added: u16,\n    pub pioneer_positions_total: u16,\n    pub purchased_at: i64,\n',
    'purchase event Pioneer fields',
)

# Weighted reward-debt model supports many positions acquired by one wallet at different times.
marker = 'fn pioneer_due(user: &UserState, p: &ProtocolState, mint: Pubkey) -> Result<u64> {'
if text.count(marker) != 1:
    raise SystemExit('pioneer_due marker missing')
assign_fn = '''fn assign_pioneer_positions_after_purchase(
    user: &mut UserState,
    p: &mut ProtocolState,
    units: u64,
) -> Result<u16> {
    let added = pioneer_positions_for_purchase(units, p.pioneer_positions_assigned);
    if added == 0 {
        return Ok(0);
    }
    let added_u128 = added as u128;
    // New positions enter at the current index, excluding every prior purchase and
    // the purchase that created the positions (Rule B).
    let usdt_debt = p
        .pioneer_index_usdt
        .checked_mul(added_u128)
        .ok_or(ProtocolError::ArithmeticOverflow)?;
    let usdc_debt = p
        .pioneer_index_usdc
        .checked_mul(added_u128)
        .ok_or(ProtocolError::ArithmeticOverflow)?;
    user.pioneer_checkpoint_usdt = user
        .pioneer_checkpoint_usdt
        .checked_add(usdt_debt)
        .ok_or(ProtocolError::ArithmeticOverflow)?;
    user.pioneer_checkpoint_usdc = user
        .pioneer_checkpoint_usdc
        .checked_add(usdc_debt)
        .ok_or(ProtocolError::ArithmeticOverflow)?;
    user.pioneer_positions = user
        .pioneer_positions
        .checked_add(added)
        .ok_or(ProtocolError::ArithmeticOverflow)?;
    p.pioneer_positions_assigned = p
        .pioneer_positions_assigned
        .checked_add(added)
        .ok_or(ProtocolError::ArithmeticOverflow)?;
    require!(p.pioneer_positions_assigned <= PIONEER_SLOTS, ProtocolError::ArithmeticOverflow);
    Ok(added)
}

'''
text = text.replace(marker, assign_fn + marker, 1)
old_due = '''    if user.pioneer_positions == 0 {
        return Ok(0);
    }
    let (index, checkpoint) = if mint == p.usdt_mint {
        (p.pioneer_index_usdt, user.pioneer_checkpoint_usdt)
    } else if mint == p.usdc_mint {
        (p.pioneer_index_usdc, user.pioneer_checkpoint_usdc)
    } else {
        return err!(ProtocolError::UnsupportedToken);
    };
    let diff_scaled = index
        .checked_sub(checkpoint)
        .ok_or(ProtocolError::ArithmeticUnderflow)?;
'''
new_due = '''    if user.pioneer_positions == 0 {
        return Ok(0);
    }
    let (index, checkpoint) = if mint == p.usdt_mint {
        (p.pioneer_index_usdt, user.pioneer_checkpoint_usdt)
    } else if mint == p.usdc_mint {
        (p.pioneer_index_usdc, user.pioneer_checkpoint_usdc)
    } else {
        return err!(ProtocolError::UnsupportedToken);
    };
    let entitlement_scaled = index
        .checked_mul(user.pioneer_positions as u128)
        .ok_or(ProtocolError::ArithmeticOverflow)?;
    let diff_scaled = entitlement_scaled
        .checked_sub(checkpoint)
        .ok_or(ProtocolError::ArithmeticUnderflow)?;
'''
text = replace_once(text, old_due, new_due, 'weighted pioneer due')

old_checkpoint_guard_usdt = '''        require!(
            user.pioneer_checkpoint_usdt <= p.pioneer_index_usdt,
            ProtocolError::ArithmeticUnderflow
        );
'''
new_checkpoint_guard_usdt = '''        let max_checkpoint = p
            .pioneer_index_usdt
            .checked_mul(user.pioneer_positions as u128)
            .ok_or(ProtocolError::ArithmeticOverflow)?;
        require!(
            user.pioneer_checkpoint_usdt <= max_checkpoint,
            ProtocolError::ArithmeticUnderflow
        );
'''
text = replace_once(text, old_checkpoint_guard_usdt, new_checkpoint_guard_usdt, 'weighted USDT checkpoint guard')
old_checkpoint_guard_usdc = '''        require!(
            user.pioneer_checkpoint_usdc <= p.pioneer_index_usdc,
            ProtocolError::ArithmeticUnderflow
        );
'''
new_checkpoint_guard_usdc = '''        let max_checkpoint = p
            .pioneer_index_usdc
            .checked_mul(user.pioneer_positions as u128)
            .ok_or(ProtocolError::ArithmeticOverflow)?;
        require!(
            user.pioneer_checkpoint_usdc <= max_checkpoint,
            ProtocolError::ArithmeticUnderflow
        );
'''
text = replace_once(text, old_checkpoint_guard_usdc, new_checkpoint_guard_usdc, 'weighted USDC checkpoint guard')
p.write_text(text)

# Existing economics tests now have zero Pioneer positions for sub-$1,000 purchases.
p = Path('integration-tests/tests/purchase_distribution.rs')
text = p.read_text()
text = text.replace('4_998_000', '5_000_000').replace('5_002_000', '5_000_000')
old = '''    let pioneer_pool = 2 * UNIT;
    let pioneer_per_slot = pioneer_pool / 100;
    let pioneer_assigned = pioneer_per_slot * 2;
    let pioneer_unassigned = pioneer_pool - pioneer_assigned;
'''
new = '''    let pioneer_pool = 2 * UNIT;
    let pioneer_assigned = 0;
    let pioneer_unassigned = pioneer_pool;
'''
text = replace_once(text, old, new, 'purchase distribution unassigned Pioneer')
text = text.replace('assert_eq!(purchase_treasury_delta, 34_960_000);', 'assert_eq!(purchase_treasury_delta, 35_000_000);')
text = text.replace('assert_eq!(purchase_vault_liability, 65_040_000);', 'assert_eq!(purchase_vault_liability, 65_000_000);')
text = text.replace('assert_eq!(protocol.pioneer_positions_assigned, 2);', 'assert_eq!(protocol.pioneer_positions_assigned, 0);')
text = text.replace('198_000 + pioneer_unassigned as u128', '200_000 + pioneer_unassigned as u128')
text = text.replace('let sponsor_claim = 5 * UNIT + sponsor_network + 22_000;', 'let sponsor_claim = 5 * UNIT + sponsor_network;')
text = text.replace('ctx.svm.assert_token_balance(&vault_usdc, self_reward + pioneer_per_slot);', 'ctx.svm.assert_token_balance(&vault_usdc, self_reward);')
text = text.replace('ctx.svm.assert_token_balance(&buyer_usdc, self_reward + pioneer_per_slot);', 'ctx.svm.assert_token_balance(&buyer_usdc, self_reward);')
p.write_text(text)

p = Path('integration-tests/tests/claim_activity.rs')
text = p.read_text()
text = text.replace('15_020_000', '15_000_000')
text = text.replace('49_980_000', '50_000_000')
text = text.replace('50_020_000', '50_000_000')
text = text.replace("plus its current Pioneer\n    // entitlement are treasury-destined immediately under IC-A.", "is treasury-destined immediately under IC-A. No Pioneer position exists below a single 1,000-unit purchase.")
text = text.replace('// 15.020 expired sponsor/Pioneer + 28 unallocated upper network + 5 service\n    // + 1.960 unassigned Pioneer = 49.980 treasury. Buyer SELF + buyer Pioneer\n    // remain collateralized in the vault.', '// 15 expired sponsor + 28 unallocated upper network + 5 service + the full\n    // 2% unassigned Pioneer pool = 50 treasury. Only buyer SELF remains in vault.')
p.write_text(text)

p = Path('integration-tests/tests/batching_equivalence.rs')
text = p.read_text()
text = text.replace('ten_single_unit_purchases_preserve_same_self_and_pioneer_as_one_ten_unit_purchase', 'ten_single_unit_purchases_preserve_same_self_as_one_ten_unit_purchase')
text = text.replace('4_998_000', '5_000_000').replace('5_002_000', '5_000_000')
text = text.replace('// Exactly the same economics as a single 10-unit root purchase:\n    // SELF 5 + Pioneer #1 0.002 remain in vault; network 4.3 + service .5 +\n    // unassigned Pioneer .198 go to treasury.', '// Exactly the same economics as a single 10-unit root purchase: SELF 5 remains\n    // in vault; network 4.3 + service .5 + the full unassigned Pioneer .2 go treasury.\n    // Ten 1-unit purchases never combine into a 1,000-unit Pioneer position.')
p.write_text(text)

p = Path('integration-tests/tests/grace_expiry.rs')
text = p.read_text()
text = text.replace('// Activate sponsor with 10 units while it is the only Pioneer.', '// Activate sponsor with 10 units. This is below the 1,000-unit Pioneer threshold.')
text = text.replace('39_958_000', '40_000_000').replace('70_042_000', '70_000_000')
text = text.replace('20_022_000', '20_000_000').replace('59_980_000', '60_000_000').replace('50_020_000', '50_000_000')
text = text.replace("// 5 SELF + 15 pending U1 network + sponsor's 0.022 Pioneer due.", '// 5 SELF + 15 pending U1 network. No Pioneer position was created.')
p.write_text(text)

# Devnet smoke keeps its 10/100 scenario and now proves sub-threshold purchases create no Pioneer positions.
p = Path('scripts/devnet-transaction-smoke.mjs')
text = p.read_text()
text = text.replace('4_998_000n', '5_000_000n').replace('5_002_000n', '5_000_000n')
text = text.replace('39_958_000n', '40_000_000n').replace('70_042_000n', '70_000_000n')
text = text.replace('20_022_000n', '20_000_000n').replace('50_020_000n', '50_000_000n')
text = text.replace('2_158_000n', '2_200_000n')
text = text.replace('invariant(asBigInt(sponsorState.pioneerPositions) === 1n, "sponsor must be Pioneer #1");', 'invariant(asBigInt(sponsorState.pioneerPositions) === 0n, "10-unit sponsor purchase must not create Pioneer positions");')
text = text.replace('invariant(asBigInt(buyerState.pioneerPositions) === 2n, "buyer must be Pioneer #2");', 'invariant(asBigInt(buyerState.pioneerPositions) === 0n, "100-unit buyer purchase must not create Pioneer positions");')
text = text.replace('"sponsor claim must pay exactly 20.022 USDT"', '"sponsor claim must pay exactly 20 USDT"')
text = text.replace('"buyer claim must pay exactly 50.020 USDT"', '"buyer claim must pay exactly 50 USDT"')
p.write_text(text)

# Static gates: freeze purchase-only, non-cumulative, multi-position, capped and Rule-B semantics.
p = Path('scripts/static-gates.py')
text = p.read_text()
text = replace_once(
    text,
    "    'Pioneer high precision index exists': 'PIONEER_SCALE' in constants and '1_000_000_000_000_000_000' in constants,\n",
    "    'Pioneer positions require one 1000-unit purchase': 'PIONEER_POSITION_PURCHASE_UNITS: u64 = 1_000' in constants and 'units / PIONEER_POSITION_PURCHASE_UNITS' in source,\n    'Pioneer registration consumes no position': 'u.pioneer_positions = 0' in source and 'Registration alone never consumes a Pioneer position' in source,\n    'Pioneer global cap is absolute 100': 'pioneer_positions_for_purchase(units, p.pioneer_positions_assigned)' in source and 'p.pioneer_positions_assigned <= PIONEER_SLOTS' in source,\n    'Pioneer Rule B assigns after current pool accrual': final_purchase.index('accrue_pioneer(') < final_purchase.index('assign_pioneer_positions_after_purchase('),\n    'Pioneer supports multiple weighted positions per wallet': 'user.pioneer_positions as u128' in source and 'checked_mul(user.pioneer_positions as u128)' in source,\n    'Pioneer high precision index exists': 'PIONEER_SCALE' in constants and '1_000_000_000_000_000_000' in constants,\n",
    'Pioneer static gates',
)
p.write_text(text)

# Add a dedicated end-to-end LiteSVM test for non-cumulative threshold, Rule B,
# multi-position ownership, 98->100 truncation and permanent saturation.
p = Path('integration-tests/tests/pioneer_positions.rs')
p.write_text(r'''use anchor_lang::{prelude::*, AccountDeserialize};
use anchor_litesvm::{AnchorContext, AnchorLiteSVM, AssertionHelpers, TestHelpers};
use service_referral_protocol::{constants::PIONEER_SCALE, state::{ProtocolState, UserState}, ID};
use solana_signer::Signer;
use solana_transaction::Transaction;

const PROGRAM_BYTES: &[u8] = include_bytes!("../../target/deploy/service_referral_protocol.so");
const UNIT: u64 = 1_000_000;

fn read_user(ctx: &AnchorContext, pda: Pubkey) -> UserState {
    let account = ctx.svm.get_account(&pda).expect("user state");
    let mut data = account.data.as_slice();
    UserState::try_deserialize(&mut data).expect("deserialize user")
}
fn read_protocol(ctx: &AnchorContext, pda: Pubkey) -> ProtocolState {
    let account = ctx.svm.get_account(&pda).expect("protocol state");
    let mut data = account.data.as_slice();
    ProtocolState::try_deserialize(&mut data).expect("deserialize protocol")
}

#[test]
fn pioneer_positions_are_single_purchase_only_weighted_rule_b_and_hard_capped() {
    let mut ctx = AnchorLiteSVM::build_with_program(ID, PROGRAM_BYTES);
    let initializer = ctx.svm.create_funded_account(80_000_000_000).expect("initializer");
    let treasury = ctx.svm.create_funded_account(20_000_000_000).expect("treasury");
    let a = ctx.svm.create_funded_account(10_000_000_000).expect("a");
    let b = ctx.svm.create_funded_account(10_000_000_000).expect("b");
    let c = ctx.svm.create_funded_account(10_000_000_000).expect("c");
    let usdt_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDT");
    let usdc_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDC");
    let (protocol, _) = Pubkey::find_program_address(&[b"protocol"], &ID);
    let (vault_authority, _) = Pubkey::find_program_address(&[b"vault-authority"], &ID);
    let (root, _) = Pubkey::find_program_address(&[b"user", Pubkey::default().as_ref()], &ID);
    let a_pda = Pubkey::find_program_address(&[b"user", a.pubkey().as_ref()], &ID).0;
    let b_pda = Pubkey::find_program_address(&[b"user", b.pubkey().as_ref()], &ID).0;
    let c_pda = Pubkey::find_program_address(&[b"user", c.pubkey().as_ref()], &ID).0;
    let registration_open_at = ctx.svm.get_sysvar::<Clock>().unix_timestamp;
    let init = ctx.program().accounts(service_referral_protocol::accounts::Initialize {
        initializer: initializer.pubkey(), service_treasury: treasury.pubkey(),
        usdt_mint: usdt_mint.pubkey(), usdc_mint: usdc_mint.pubkey(), protocol,
        vault_authority, technical_root: root, system_program: anchor_lang::system_program::ID,
    }).args(service_referral_protocol::instruction::Initialize { registration_open_at })
      .instruction().expect("init");
    ctx.execute_instruction(init, &[&initializer]).expect("init tx").assert_success();

    macro_rules! register {
        ($wallet:expr, $pda:expr) => {{
            ctx.svm.expire_blockhash();
            let ix = ctx.program().accounts(service_referral_protocol::accounts::Register {
                wallet: $wallet.pubkey(), protocol, referrer_wallet: Pubkey::default(),
                referrer: root, user: $pda, system_program: anchor_lang::system_program::ID,
            }).args(service_referral_protocol::instruction::Register {}).instruction().expect("register");
            ctx.execute_instruction(ix, &[&$wallet]).expect("register tx").assert_success();
        }};
    }
    register!(a, a_pda); register!(b, b_pda); register!(c, c_pda);
    assert_eq!(read_protocol(&ctx, protocol).pioneer_positions_assigned, 0, "registration must consume zero positions");
    assert_eq!(read_user(&ctx, a_pda).pioneer_positions, 0);

    let treasury_usdt = ctx.svm.create_associated_token_account(&usdt_mint.pubkey(), &treasury).expect("treasury USDT");
    let treasury_usdc = ctx.svm.create_associated_token_account(&usdc_mint.pubkey(), &treasury).expect("treasury USDC");
    let a_src = ctx.svm.create_associated_token_account(&usdc_mint.pubkey(), &a).expect("a ATA");
    let b_src = ctx.svm.create_associated_token_account(&usdc_mint.pubkey(), &b).expect("b ATA");
    let c_src = ctx.svm.create_associated_token_account(&usdc_mint.pubkey(), &c).expect("c ATA");
    let vault_usdt = spl_associated_token_account::get_associated_token_address_with_program_id(&vault_authority, &usdt_mint.pubkey(), &spl_token::id());
    let vault_usdc = spl_associated_token_account::get_associated_token_address_with_program_id(&vault_authority, &usdc_mint.pubkey(), &spl_token::id());
    let create_vaults = Transaction::new_signed_with_payer(&[
        spl_associated_token_account::instruction::create_associated_token_account(&initializer.pubkey(), &vault_authority, &usdt_mint.pubkey(), &spl_token::id()),
        spl_associated_token_account::instruction::create_associated_token_account(&initializer.pubkey(), &vault_authority, &usdc_mint.pubkey(), &spl_token::id()),
    ], Some(&initializer.pubkey()), &[&initializer], ctx.svm.latest_blockhash());
    ctx.svm.send_transaction(create_vaults).expect("vaults");
    ctx.svm.mint_to(&usdc_mint.pubkey(), &a_src, &initializer, 5_000 * UNIT).expect("fund a");
    ctx.svm.mint_to(&usdc_mint.pubkey(), &b_src, &initializer, 94_000 * UNIT).expect("fund b");
    ctx.svm.mint_to(&usdc_mint.pubkey(), &c_src, &initializer, 8_000 * UNIT).expect("fund c");

    macro_rules! buy_root {
        ($wallet:expr, $pda:expr, $src:expr, $units:expr) => {{
            ctx.svm.expire_blockhash();
            let ix = ctx.program().accounts(service_referral_protocol::accounts::PurchaseAndDistribute {
                wallet: $wallet.pubkey(), protocol, user: $pda, user_source: $src,
                vault_authority, usdt_vault: vault_usdt, usdc_vault: vault_usdc,
                service_treasury_usdt: treasury_usdt, service_treasury_usdc: treasury_usdc,
                direct_referrer: root, upline_1: root, upline_2: root, upline_3: root,
                upline_4: root, upline_5: root, upline_6: root, upline_7: root, upline_8: root,
                token_program: spl_token::id(),
            }).args(service_referral_protocol::instruction::PurchaseAndDistribute { units: $units })
              .instruction().expect("purchase");
            ctx.execute_instruction(ix, &[&$wallet]).expect("purchase tx").assert_success();
        }};
    }

    // Two separate 500 purchases never combine into one Pioneer position.
    buy_root!(a, a_pda, a_src, 500);
    buy_root!(a, a_pda, a_src, 500);
    assert_eq!(read_user(&ctx, a_pda).pioneer_positions, 0);
    assert_eq!(read_protocol(&ctx, protocol).pioneer_positions_assigned, 0);

    // A single 1,000 purchase creates exactly one position, but Rule B means that
    // position has zero due from the transaction that created it.
    buy_root!(a, a_pda, a_src, 1_000);
    let a_after_one = read_user(&ctx, a_pda);
    let p_after_one = read_protocol(&ctx, protocol);
    assert_eq!(a_after_one.pioneer_positions, 1);
    assert_eq!(p_after_one.pioneer_positions_assigned, 1);
    let due_after_creation = ((a_after_one.pioneer_positions as u128) * p_after_one.pioneer_index_usdc
        - a_after_one.pioneer_checkpoint_usdc) / PIONEER_SCALE;
    assert_eq!(due_after_creation, 0, "new position must not earn its creating purchase");

    // Same wallet can earn three more positions in one 3,000 purchase. Only its old
    // one position earns this purchase: 60 USDC pool / 100 = 0.6 USDC.
    buy_root!(a, a_pda, a_src, 3_000);
    let a_after_four = read_user(&ctx, a_pda);
    let p_after_four = read_protocol(&ctx, protocol);
    assert_eq!(a_after_four.pioneer_positions, 4);
    assert_eq!(p_after_four.pioneer_positions_assigned, 4);
    let due = ((a_after_four.pioneer_positions as u128) * p_after_four.pioneer_index_usdc
        - a_after_four.pioneer_checkpoint_usdc) / PIONEER_SCALE;
    assert_eq!(due, 600_000u128, "only the pre-existing position earns the 3,000 purchase");

    // Fill to exactly 98 positions with one independent 94,000 purchase.
    buy_root!(b, b_pda, b_src, 94_000);
    assert_eq!(read_user(&ctx, b_pda).pioneer_positions, 94);
    assert_eq!(read_protocol(&ctx, protocol).pioneer_positions_assigned, 98);

    // Only two remain. A 3,000 purchase requests three but receives exactly two.
    buy_root!(c, c_pda, c_src, 3_000);
    assert_eq!(read_user(&ctx, c_pda).pioneer_positions, 2);
    assert_eq!(read_protocol(&ctx, protocol).pioneer_positions_assigned, 100);

    // Permanent saturation: even another qualifying 5,000 purchase creates zero.
    buy_root!(c, c_pda, c_src, 5_000);
    assert_eq!(read_user(&ctx, c_pda).pioneer_positions, 2);
    assert_eq!(read_protocol(&ctx, protocol).pioneer_positions_assigned, 100);
}
''')

# Update README wording if old registration model is present; otherwise append a precise section.
p = Path('README.md')
text = p.read_text()
section = '''\n### Pioneer 2% pool — purchase-earned positions\n\n- The pool has an absolute maximum of **100 positions**.\n- Registration alone earns **zero** positions.\n- A single purchase earns `floor(units / 1000)` positions; purchases below 1,000 never accumulate across transactions.\n- One wallet may own multiple positions.\n- Assignment is capped by remaining capacity: at 98/100, a 3,000-unit purchase receives exactly 2 positions.\n- At 100/100 the pool is permanently saturated and no later purchase can create another position.\n- **Rule B:** positions created by a purchase begin earning only from the next global purchase.\n- Network/SELF/Pioneer earnings and claims never create Pioneer positions; only the gross units of the individual buyer-signed purchase do.\n'''
if '### Pioneer 2% pool — purchase-earned positions' not in text:
    text += section
p.write_text(text)

print('Pioneer purchase-position migration applied')
