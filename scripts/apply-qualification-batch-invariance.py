from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

lib_path = ROOT / 'programs/service_referral_protocol/src/lib.rs'
lib = lib_path.read_text()
start = lib.index('/// Converts every whole-token-atom liability for an INACTIVE user into treasury-')
end = lib.index('#[allow(clippy::too_many_arguments)]', start)
new = r'''/// True while an INACTIVE user is still inside a live partial qualification window.
/// SELF and Pioneer value created during this window is provisional so splitting the
/// same qualifying purchase into multiple transactions cannot change its economics.
fn qualification_window_open(user: &UserState, now: i64) -> bool {
    if user.qualification_progress_units == 0 || user.qualification_window_started_at <= 0 {
        return false;
    }
    user.qualification_window_started_at
        .checked_add(ACTIVE_SECONDS)
        .map(|deadline| now <= deadline)
        .unwrap_or(false)
}

/// Converts whole-token-atom liabilities for an INACTIVE user into treasury-
/// destined expired value for one mint.
///
/// A live partial qualification window is a narrow exception for the user's own
/// SELF and Pioneer buckets: those remain provisional until the user either reaches
/// the 10-unit threshold or the qualification window expires. Network amounts are
/// never protected by this exception and continue to follow IC-A fixed-depth expiry.
/// This makes 10 units bought as 10x1 economically equivalent to 10 units bought
/// in one transaction for the buyer's own SELF/Pioneer entitlement.
fn expire_unclaimed_for_mint(
    user: &mut UserState,
    now: i64,
    p: &mut ProtocolState,
    mint: Pubkey,
) -> Result<u64> {
    if activity_status(user, now) != ActivityStatus::Inactive {
        return Ok(0);
    }

    let preserve_self_and_pioneer = qualification_window_open(user, now);
    let pioneer = if preserve_self_and_pioneer {
        0
    } else {
        pioneer_due(user, p, mint)?
    };

    let (self_reward, network_claimable, network_pending) = if mint == p.usdt_mint {
        let self_reward = if preserve_self_and_pioneer {
            0
        } else {
            let value = user.self_accrued_usdt;
            user.self_accrued_usdt = 0;
            value
        };
        let values = (
            self_reward,
            user.network_claimable_usdt,
            user.network_pending_usdt,
        );
        user.network_claimable_usdt = 0;
        user.network_pending_usdt = 0;
        values
    } else if mint == p.usdc_mint {
        let self_reward = if preserve_self_and_pioneer {
            0
        } else {
            let value = user.self_accrued_usdc;
            user.self_accrued_usdc = 0;
            value
        };
        let values = (
            self_reward,
            user.network_claimable_usdc,
            user.network_pending_usdc,
        );
        user.network_claimable_usdc = 0;
        user.network_pending_usdc = 0;
        values
    } else {
        return err!(ProtocolError::UnsupportedToken);
    };

    let amount = self_reward
        .checked_add(network_claimable)
        .ok_or(ProtocolError::ArithmeticOverflow)?
        .checked_add(network_pending)
        .ok_or(ProtocolError::ArithmeticOverflow)?
        .checked_add(pioneer)
        .ok_or(ProtocolError::ArithmeticOverflow)?;

    if pioneer > 0 {
        checkpoint_pioneer_claimed(user, p, mint, pioneer)?;
    }
    mark_user_expired(user, p, mint, amount)?;
    Ok(amount)
}

'''
lib = lib[:start] + new + lib[end:]
lib_path.write_text(lib)

# Strengthen static gates.
gate_path = ROOT / 'scripts/static-gates.py'
gate = gate_path.read_text()
needle = "    'activity partial window exists': 'qualification_window_started_at' in state and 'qualification_window_started_at' in activity_helper,\n"
replacement = needle + "    'partial qualification preserves SELF and Pioneer only': 'qualification_window_open' in source and 'preserve_self_and_pioneer' in expiry_helper and 'network_claimable_usdt = 0' in expiry_helper and 'network_pending_usdt = 0' in expiry_helper,\n"
if gate.count(needle) != 1:
    raise SystemExit(f'static gate insertion expected 1 match, got {gate.count(needle)}')
gate = gate.replace(needle, replacement, 1)
gate_path.write_text(gate)

# Add an on-chain path-independence regression test.
test_path = ROOT / 'integration-tests/tests/batching_equivalence.rs'
test_path.write_text(r'''use anchor_lang::{prelude::*, AccountDeserialize};
use anchor_litesvm::{AnchorLiteSVM, AssertionHelpers, TestHelpers};
use service_referral_protocol::{state::{ProtocolState, UserState}, ID};
use solana_signer::Signer;
use solana_transaction::Transaction;

const PROGRAM_BYTES: &[u8] = include_bytes!("../../target/deploy/service_referral_protocol.so");
const UNIT: u64 = 1_000_000;

fn read_user(ctx: &AnchorLiteSVM, pda: Pubkey) -> UserState {
    let account = ctx.svm.get_account(&pda).expect("user state");
    let mut data = account.data.as_slice();
    UserState::try_deserialize(&mut data).expect("deserialize user")
}

fn read_protocol(ctx: &AnchorLiteSVM, pda: Pubkey) -> ProtocolState {
    let account = ctx.svm.get_account(&pda).expect("protocol state");
    let mut data = account.data.as_slice();
    ProtocolState::try_deserialize(&mut data).expect("deserialize protocol")
}

#[test]
fn ten_single_unit_purchases_preserve_same_self_and_pioneer_as_one_ten_unit_purchase() {
    let mut ctx = AnchorLiteSVM::build_with_program(ID, PROGRAM_BYTES);
    let initializer = ctx.svm.create_funded_account(60_000_000_000).expect("initializer");
    let treasury = ctx.svm.create_funded_account(10_000_000_000).expect("treasury");
    let buyer = ctx.svm.create_funded_account(20_000_000_000).expect("buyer");

    let usdt_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDT mint");
    let usdc_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDC mint");
    let (protocol, _) = Pubkey::find_program_address(&[b"protocol"], &ID);
    let (vault_authority, _) = Pubkey::find_program_address(&[b"vault-authority"], &ID);
    let (technical_root, _) = Pubkey::find_program_address(&[b"user", Pubkey::default().as_ref()], &ID);
    let (buyer_pda, _) = Pubkey::find_program_address(&[b"user", buyer.pubkey().as_ref()], &ID);

    let registration_open_at = ctx.svm.get_sysvar::<Clock>().unix_timestamp;
    let initialize_ix = ctx.program()
        .accounts(service_referral_protocol::accounts::Initialize {
            initializer: initializer.pubkey(), service_treasury: treasury.pubkey(),
            usdt_mint: usdt_mint.pubkey(), usdc_mint: usdc_mint.pubkey(), protocol,
            vault_authority, technical_root, system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::Initialize { registration_open_at })
        .instruction().expect("initialize");
    ctx.execute_instruction(initialize_ix, &[&initializer]).expect("initialize tx").assert_success();

    let register_ix = ctx.program()
        .accounts(service_referral_protocol::accounts::Register {
            wallet: buyer.pubkey(), protocol, referrer_wallet: Pubkey::default(),
            referrer: technical_root, user: buyer_pda,
            system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::Register {})
        .instruction().expect("register");
    ctx.execute_instruction(register_ix, &[&buyer]).expect("register tx").assert_success();

    let treasury_usdt = ctx.svm.create_associated_token_account(&usdt_mint.pubkey(), &treasury).expect("treasury USDT");
    let treasury_usdc = ctx.svm.create_associated_token_account(&usdc_mint.pubkey(), &treasury).expect("treasury USDC");
    let buyer_usdc = ctx.svm.create_associated_token_account(&usdc_mint.pubkey(), &buyer).expect("buyer USDC");
    let vault_usdt = spl_associated_token_account::get_associated_token_address_with_program_id(
        &vault_authority, &usdt_mint.pubkey(), &spl_token::id(),
    );
    let vault_usdc = spl_associated_token_account::get_associated_token_address_with_program_id(
        &vault_authority, &usdc_mint.pubkey(), &spl_token::id(),
    );
    let create_vaults = Transaction::new_signed_with_payer(
        &[
            spl_associated_token_account::instruction::create_associated_token_account(
                &initializer.pubkey(), &vault_authority, &usdt_mint.pubkey(), &spl_token::id(),
            ),
            spl_associated_token_account::instruction::create_associated_token_account(
                &initializer.pubkey(), &vault_authority, &usdc_mint.pubkey(), &spl_token::id(),
            ),
        ],
        Some(&initializer.pubkey()), &[&initializer], ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(create_vaults).expect("create vaults");
    ctx.svm.mint_to(&usdc_mint.pubkey(), &buyer_usdc, &initializer, 10 * UNIT).expect("fund buyer");

    for batch_index in 0u64..10 {
        let (batch, _) = Pubkey::find_program_address(
            &[b"batch", buyer.pubkey().as_ref(), &batch_index.to_le_bytes()], &ID,
        );
        let ix = ctx.program()
            .accounts(service_referral_protocol::accounts::PurchaseAndDistribute {
                wallet: buyer.pubkey(), protocol, user: buyer_pda, user_source: buyer_usdc,
                vault_authority, usdt_vault: vault_usdt, usdc_vault: vault_usdc,
                service_treasury_usdt: treasury_usdt, service_treasury_usdc: treasury_usdc,
                direct_referrer: technical_root,
                upline_1: technical_root, upline_2: technical_root, upline_3: technical_root,
                upline_4: technical_root, upline_5: technical_root, upline_6: technical_root,
                upline_7: technical_root, upline_8: technical_root,
                batch, token_program: spl_token::id(), system_program: anchor_lang::system_program::ID,
            })
            .args(service_referral_protocol::instruction::PurchaseAndDistribute { units: 1 })
            .instruction().expect("single-unit purchase");
        ctx.execute_instruction(ix, &[&buyer]).expect("single-unit tx").assert_success();
    }

    let user = read_user(&ctx, buyer_pda);
    let state = read_protocol(&ctx, protocol);
    assert_eq!(user.lifetime_service_units, 10);
    assert!(user.active_until > 0, "tenth unit inside the window must activate buyer");
    assert_eq!(user.self_accrued_usdc, 5 * UNIT, "all ten provisional SELF rewards must survive qualification");
    assert_eq!(state.next_unit_id, 11);

    // Exactly the same economics as a single 10-unit root purchase:
    // SELF 5 + Pioneer #1 0.002 remain in vault; network 4.3 + service .5 +
    // unassigned Pioneer .198 go to treasury.
    ctx.svm.assert_token_balance(&buyer_usdc, 0);
    ctx.svm.assert_token_balance(&treasury_usdc, 4_998_000);
    ctx.svm.assert_token_balance(&vault_usdc, 5_002_000);

    let claim = ctx.program()
        .accounts(service_referral_protocol::accounts::Claim {
            wallet: buyer.pubkey(), protocol, user: buyer_pda, vault_authority,
            vault_token: vault_usdc, destination: buyer_usdc, token_program: spl_token::id(),
        })
        .args(service_referral_protocol::instruction::Claim {})
        .instruction().expect("claim");
    ctx.execute_instruction(claim, &[&buyer]).expect("claim tx").assert_success();

    ctx.svm.assert_token_balance(&buyer_usdc, 5_002_000);
    ctx.svm.assert_token_balance(&treasury_usdc, 4_998_000);
    ctx.svm.assert_token_balance(&vault_usdc, 0);
    let claimed = read_user(&ctx, buyer_pda);
    assert_eq!(claimed.lifetime_claimed_usdc, 5_002_000u128);
}
''')

# Document the invariant.
readme_path = ROOT / 'README.md'
readme = readme_path.read_text()
needle = '- stale unclaimed value is settled before late reactivation, so value whose grace period already ended cannot be rescued.\n'
addition = needle + '- During a live partial qualification window, the buyer\'s SELF and Pioneer entitlement is provisional rather than immediately expired. Reaching 10 units inside the window makes `10×1` purchases economically equivalent to one 10-unit purchase for those own-user buckets; failure to qualify before the window closes sends the provisional value to treasury on settlement. Network-upline rewards remain governed by IC-A throughout.\n'
if readme.count(needle) != 1:
    raise SystemExit(f'README insertion expected 1 match, got {readme.count(needle)}')
readme_path.write_text(readme.replace(needle, addition, 1))

print('qualification batch-invariance hardening applied')
