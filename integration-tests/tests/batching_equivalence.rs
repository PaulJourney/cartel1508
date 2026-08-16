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
