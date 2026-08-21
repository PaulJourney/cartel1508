use anchor_lang::{prelude::*, AccountDeserialize};
use anchor_litesvm::{AnchorContext, AnchorLiteSVM, AssertionHelpers, TestHelpers};
use service_referral_protocol::{state::UserState, ID};
use solana_signer::Signer;
use solana_transaction::Transaction;

const PROGRAM_BYTES: &[u8] = include_bytes!("../../target/deploy/service_referral_protocol.so");
const UNIT: u64 = 1_000_000;

fn read_user(ctx: &AnchorContext, pda: Pubkey) -> UserState {
    let account = ctx.svm.get_account(&pda).expect("user state");
    let mut data = account.data.as_slice();
    UserState::try_deserialize(&mut data).expect("deserialize user")
}

#[test]
fn grace_preserves_self_and_network_temporarily_then_inactivity_expires_them() {
    let mut ctx = AnchorLiteSVM::build_with_program(ID, PROGRAM_BYTES);
    let initializer = ctx.svm.create_funded_account(50_000_000_000).expect("initializer");
    let treasury = ctx.svm.create_funded_account(10_000_000_000).expect("treasury");
    let sponsor = ctx.svm.create_funded_account(10_000_000_000).expect("sponsor");
    let buyer = ctx.svm.create_funded_account(10_000_000_000).expect("buyer");

    let usdt_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDT mint");
    let usdc_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDC mint");

    let (protocol, _) = Pubkey::find_program_address(&[b"protocol"], &ID);
    let (vault_authority, _) = Pubkey::find_program_address(&[b"vault-authority"], &ID);
    let (technical_root, _) = Pubkey::find_program_address(&[b"user", Pubkey::default().as_ref()], &ID);
    let (sponsor_pda, _) = Pubkey::find_program_address(&[b"user", sponsor.pubkey().as_ref()], &ID);
    let (buyer_pda, _) = Pubkey::find_program_address(&[b"user", buyer.pubkey().as_ref()], &ID);

    let registration_open_at = ctx.svm.get_sysvar::<Clock>().unix_timestamp;
    let initialize_ix = ctx.program()
        .accounts(service_referral_protocol::accounts::Initialize {
            initializer: initializer.pubkey(), service_treasury: treasury.pubkey(),
            usdt_mint: usdt_mint.pubkey(), usdc_mint: usdc_mint.pubkey(), protocol,
            vault_authority, technical_root, system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::Initialize { registration_open_at })
        .instruction().expect("initialize ix");
    ctx.execute_instruction(initialize_ix, &[&initializer]).expect("initialize tx").assert_success();

    let register_sponsor = ctx.program()
        .accounts(service_referral_protocol::accounts::Register {
            wallet: sponsor.pubkey(), protocol, referrer_wallet: Pubkey::default(),
            referrer: technical_root, user: sponsor_pda,
            system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::Register {})
        .instruction().expect("register sponsor");
    ctx.execute_instruction(register_sponsor, &[&sponsor]).expect("register sponsor tx").assert_success();

    let treasury_usdt = ctx.svm.create_associated_token_account(&usdt_mint.pubkey(), &treasury).expect("treasury USDT");
    let treasury_usdc = ctx.svm.create_associated_token_account(&usdc_mint.pubkey(), &treasury).expect("treasury USDC");
    let sponsor_usdc = ctx.svm.create_associated_token_account(&usdc_mint.pubkey(), &sponsor).expect("sponsor USDC");
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

    // Activate sponsor with 10 units. This is below the 1,000-unit Pioneer threshold.
    ctx.svm.mint_to(&usdc_mint.pubkey(), &sponsor_usdc, &initializer, 10 * UNIT).expect("fund sponsor");
    let sponsor_purchase = ctx.program()
        .accounts(service_referral_protocol::accounts::PurchaseAndDistribute {
            wallet: sponsor.pubkey(), protocol, user: sponsor_pda, user_source: sponsor_usdc,
            vault_authority, usdt_vault: vault_usdt, usdc_vault: vault_usdc,
            service_treasury_usdt: treasury_usdt, service_treasury_usdc: treasury_usdc,
            direct_referrer: technical_root,
            upline_1: technical_root, upline_2: technical_root, upline_3: technical_root,
            upline_4: technical_root, upline_5: technical_root, upline_6: technical_root,
            upline_7: technical_root, upline_8: technical_root,
            token_program: spl_token::id(),
        })
        .args(service_referral_protocol::instruction::PurchaseAndDistribute { units: 10 })
        .instruction().expect("sponsor purchase");
    ctx.execute_instruction(sponsor_purchase, &[&sponsor]).expect("sponsor purchase tx").assert_success();

    let sponsor_active = read_user(&ctx, sponsor_pda);
    assert!(sponsor_active.active_until > 0);

    let register_buyer = ctx.program()
        .accounts(service_referral_protocol::accounts::Register {
            wallet: buyer.pubkey(), protocol, referrer_wallet: sponsor.pubkey(),
            referrer: sponsor_pda, user: buyer_pda,
            system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::Register {})
        .instruction().expect("register buyer");
    ctx.execute_instruction(register_buyer, &[&buyer]).expect("register buyer tx").assert_success();

    // Move just into sponsor's 48-hour GRACE window.
    let mut clock: Clock = ctx.svm.get_sysvar();
    clock.unix_timestamp = sponsor_active.active_until + 1;
    ctx.svm.set_sysvar(&clock);
    ctx.svm.expire_blockhash();

    ctx.svm.mint_to(&usdc_mint.pubkey(), &buyer_usdc, &initializer, 100 * UNIT).expect("fund buyer");
    let buyer_purchase = ctx.program()
        .accounts(service_referral_protocol::accounts::PurchaseAndDistribute {
            wallet: buyer.pubkey(), protocol, user: buyer_pda, user_source: buyer_usdc,
            vault_authority, usdt_vault: vault_usdt, usdc_vault: vault_usdc,
            service_treasury_usdt: treasury_usdt, service_treasury_usdc: treasury_usdc,
            direct_referrer: sponsor_pda,
            upline_1: technical_root, upline_2: technical_root, upline_3: technical_root,
            upline_4: technical_root, upline_5: technical_root, upline_6: technical_root,
            upline_7: technical_root, upline_8: technical_root,
            token_program: spl_token::id(),
        })
        .args(service_referral_protocol::instruction::PurchaseAndDistribute { units: 100 })
        .instruction().expect("buyer purchase");
    ctx.execute_instruction(buyer_purchase, &[&buyer]).expect("buyer purchase tx").assert_success();

    let sponsor_grace = read_user(&ctx, sponsor_pda);
    assert_eq!(sponsor_grace.self_accrued_usdc, 5 * UNIT, "GRACE preserves sponsor own SELF reward");
    assert_eq!(sponsor_grace.network_pending_usdc, 15 * UNIT, "GRACE preserves sponsor U1 network reward as pending");
    assert_eq!(sponsor_grace.lifetime_expired_usdc, 0);

    // Claim is intentionally ACTIVE-only, so even during GRACE the sponsor cannot withdraw.
    let claim_during_grace = ctx.program()
        .accounts(service_referral_protocol::accounts::Claim {
            wallet: sponsor.pubkey(), protocol, user: sponsor_pda, vault_authority,
            vault_token: vault_usdc, destination: sponsor_usdc, token_program: spl_token::id(),
        })
        .args(service_referral_protocol::instruction::Claim {})
        .instruction().expect("grace claim");
    let grace_claim_result = ctx.execute_instruction(claim_during_grace, &[&sponsor]).expect("grace claim result");
    assert!(!grace_claim_result.is_success(), "GRACE user must reactivate before claiming");
    let still_grace = read_user(&ctx, sponsor_pda);
    assert_eq!(still_grace.self_accrued_usdc, 5 * UNIT);
    assert_eq!(still_grace.network_pending_usdc, 15 * UNIT);

    // Miss the grace deadline. Permissionless settlement must irreversibly clear the
    // 5 SELF + 15 pending U1 network. No Pioneer position was created.
    clock.unix_timestamp = sponsor_grace.grace_until + 1;
    ctx.svm.set_sysvar(&clock);
    ctx.svm.expire_blockhash();

    ctx.svm.assert_token_balance(&treasury_usdc, 40_000_000);
    ctx.svm.assert_token_balance(&vault_usdc, 70_000_000);

    let settle = ctx.program()
        .accounts(service_referral_protocol::accounts::SettleExpired {
            settler: buyer.pubkey(), protocol, user: sponsor_pda, vault_authority,
            vault_token: vault_usdc, service_treasury_token: treasury_usdc,
            token_program: spl_token::id(),
        })
        .args(service_referral_protocol::instruction::SettleExpired {})
        .instruction().expect("settle expired");
    ctx.execute_instruction(settle, &[&buyer]).expect("settle tx").assert_success();

    let sponsor_inactive = read_user(&ctx, sponsor_pda);
    assert_eq!(sponsor_inactive.self_accrued_usdc, 0);
    assert_eq!(sponsor_inactive.network_claimable_usdc, 0);
    assert_eq!(sponsor_inactive.network_pending_usdc, 0);
    assert_eq!(sponsor_inactive.lifetime_expired_usdc, 20_000_000u128);

    ctx.svm.assert_token_balance(&treasury_usdc, 60_000_000);
    ctx.svm.assert_token_balance(&vault_usdc, 50_000_000);

    // A second settlement must fail and cannot transfer the same expired value twice.
    ctx.svm.expire_blockhash();
    let settle_again = ctx.program()
        .accounts(service_referral_protocol::accounts::SettleExpired {
            settler: buyer.pubkey(), protocol, user: sponsor_pda, vault_authority,
            vault_token: vault_usdc, service_treasury_token: treasury_usdc,
            token_program: spl_token::id(),
        })
        .args(service_referral_protocol::instruction::SettleExpired {})
        .instruction().expect("second settle");
    let second = ctx.execute_instruction(settle_again, &[&buyer]).expect("second settle result");
    assert!(!second.is_success(), "expired value must not be settleable twice");
    ctx.svm.assert_token_balance(&treasury_usdc, 60_000_000);
    ctx.svm.assert_token_balance(&vault_usdc, 50_000_000);
}
