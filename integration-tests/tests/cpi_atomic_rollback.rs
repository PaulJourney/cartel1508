use anchor_lang::{prelude::*, AccountDeserialize};
use anchor_litesvm::{AnchorContext, AnchorLiteSVM, AssertionHelpers, TestHelpers};
use service_referral_protocol::{state::{ProtocolState, UserState}, ID};
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
fn failed_payment_cpi_rolls_back_prior_expiry_settlement_and_state_mutations() {
    let mut ctx = AnchorLiteSVM::build_with_program(ID, PROGRAM_BYTES);
    let initializer = ctx.svm.create_funded_account(40_000_000_000).expect("initializer");
    let treasury = ctx.svm.create_funded_account(10_000_000_000).expect("treasury");
    let buyer = ctx.svm.create_funded_account(10_000_000_000).expect("buyer");

    let usdt_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDT mint");
    let usdc_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDC mint");

    let (protocol, _) = Pubkey::find_program_address(&[b"protocol"], &ID);
    let (vault_authority, _) = Pubkey::find_program_address(&[b"vault-authority"], &ID);
    let (technical_root, _) = Pubkey::find_program_address(&[b"user", Pubkey::default().as_ref()], &ID);
    let (buyer_pda, _) = Pubkey::find_program_address(&[b"user", buyer.pubkey().as_ref()], &ID);

    let registration_open_at = ctx.svm.get_sysvar::<Clock>().unix_timestamp;
    let initialize_ix = ctx.program()
        .accounts(service_referral_protocol::accounts::Initialize {
            initializer: initializer.pubkey(),
            service_treasury: treasury.pubkey(),
            usdt_mint: usdt_mint.pubkey(),
            usdc_mint: usdc_mint.pubkey(),
            protocol,
            vault_authority,
            technical_root,
            system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::Initialize { registration_open_at })
        .instruction().expect("initialize ix");
    ctx.execute_instruction(initialize_ix, &[&initializer]).expect("initialize tx").assert_success();

    let register_buyer = ctx.program()
        .accounts(service_referral_protocol::accounts::Register {
            wallet: buyer.pubkey(),
            protocol,
            referrer_wallet: Pubkey::default(),
            referrer: technical_root,
            user: buyer_pda,
            system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::Register {})
        .instruction().expect("register buyer");
    ctx.execute_instruction(register_buyer, &[&buyer]).expect("register buyer tx").assert_success();

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
        Some(&initializer.pubkey()),
        &[&initializer],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(create_vaults).expect("create vaults");

    ctx.svm.mint_to(&usdc_mint.pubkey(), &buyer_usdc, &initializer, 10 * UNIT).expect("fund first purchase");
    let first_purchase = ctx.program()
        .accounts(service_referral_protocol::accounts::PurchaseAndDistribute {
            wallet: buyer.pubkey(), protocol, user: buyer_pda, user_source: buyer_usdc,
            vault_authority, usdt_vault: vault_usdt, usdc_vault: vault_usdc,
            service_treasury_usdt: treasury_usdt, service_treasury_usdc: treasury_usdc,
            direct_referrer: technical_root,
            upline_1: technical_root, upline_2: technical_root, upline_3: technical_root,
            upline_4: technical_root, upline_5: technical_root, upline_6: technical_root,
            upline_7: technical_root, upline_8: technical_root,
            token_program: spl_token::id(),
        })
        .args(service_referral_protocol::instruction::PurchaseAndDistribute { units: 10 })
        .instruction().expect("first purchase");
    ctx.execute_instruction(first_purchase, &[&buyer]).expect("first purchase tx").assert_success();

    let active = read_user(&ctx, buyer_pda);
    let protocol_before_failed_tx = read_protocol(&ctx, protocol);
    assert_eq!(active.self_accrued_usdc, 5 * UNIT);
    assert_eq!(active.next_purchase_index, 1);
    assert_eq!(active.active_weeks_started, 1);
    assert_eq!(protocol_before_failed_tx.next_unit_id, 11);
    assert_eq!(protocol_before_failed_tx.lifetime_expired_usdc, 0);
    ctx.svm.assert_token_balance(&buyer_usdc, 0);
    ctx.svm.assert_token_balance(&treasury_usdc, 5 * UNIT);
    ctx.svm.assert_token_balance(&vault_usdc, 5 * UNIT);

    let mut clock: Clock = ctx.svm.get_sysvar();
    clock.unix_timestamp = active.grace_until + 1;
    ctx.svm.set_sysvar(&clock);
    ctx.svm.expire_blockhash();

    let failing_purchase = ctx.program()
        .accounts(service_referral_protocol::accounts::PurchaseAndDistribute {
            wallet: buyer.pubkey(), protocol, user: buyer_pda, user_source: buyer_usdc,
            vault_authority, usdt_vault: vault_usdt, usdc_vault: vault_usdc,
            service_treasury_usdt: treasury_usdt, service_treasury_usdc: treasury_usdc,
            direct_referrer: technical_root,
            upline_1: technical_root, upline_2: technical_root, upline_3: technical_root,
            upline_4: technical_root, upline_5: technical_root, upline_6: technical_root,
            upline_7: technical_root, upline_8: technical_root,
            token_program: spl_token::id(),
        })
        .args(service_referral_protocol::instruction::PurchaseAndDistribute { units: 10 })
        .instruction().expect("failing purchase");
    let failed = ctx.execute_instruction(failing_purchase, &[&buyer]).expect("failed purchase result");
    assert!(!failed.is_success(), "insufficient source balance must fail purchase");

    let after_failed_tx = read_user(&ctx, buyer_pda);
    let protocol_after_failed_tx = read_protocol(&ctx, protocol);
    assert_eq!(after_failed_tx.self_accrued_usdc, 5 * UNIT, "failed tx must restore pre-settlement SELF");
    assert_eq!(after_failed_tx.lifetime_expired_usdc, 0, "failed tx must not record expiry");
    assert_eq!(after_failed_tx.next_purchase_index, 1, "failed purchase must not consume purchase index");
    assert_eq!(after_failed_tx.active_weeks_started, 1, "failed purchase must not advance ACTIVE weeks");
    assert_eq!(after_failed_tx.qualification_progress_units, 0, "failed purchase must not leave qualification progress");
    assert_eq!(protocol_after_failed_tx.next_unit_id, 11, "failed purchase must not allocate logical units");
    assert_eq!(protocol_after_failed_tx.lifetime_expired_usdc, 0, "failed tx must restore protocol expiry metrics");
    ctx.svm.assert_token_balance(&buyer_usdc, 0);
    ctx.svm.assert_token_balance(&treasury_usdc, 5 * UNIT);
    ctx.svm.assert_token_balance(&vault_usdc, 5 * UNIT);

    ctx.svm.expire_blockhash();
    let settle = ctx.program()
        .accounts(service_referral_protocol::accounts::SettleExpired {
            settler: initializer.pubkey(), protocol, user: buyer_pda, vault_authority,
            vault_token: vault_usdc, service_treasury_token: treasury_usdc,
            token_program: spl_token::id(),
        })
        .args(service_referral_protocol::instruction::SettleExpired {})
        .instruction().expect("settle after failed purchase");
    ctx.execute_instruction(settle, &[&initializer]).expect("settle tx").assert_success();
    let settled = read_user(&ctx, buyer_pda);
    assert_eq!(settled.self_accrued_usdc, 0);
    assert_eq!(settled.lifetime_expired_usdc, (5 * UNIT) as u128);
    ctx.svm.assert_token_balance(&treasury_usdc, 10 * UNIT);
    ctx.svm.assert_token_balance(&vault_usdc, 0);
}
