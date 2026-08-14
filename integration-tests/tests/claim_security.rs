use anchor_lang::{prelude::*, AccountDeserialize};
use anchor_litesvm::{AnchorLiteSVM, AssertionHelpers, TestHelpers};
use service_referral_protocol::{state::UserState, ID};
use solana_signer::Signer;
use solana_transaction::Transaction;

const PROGRAM_BYTES: &[u8] = include_bytes!("../../target/deploy/service_referral_protocol.so");
const UNIT: u64 = 1_000_000;

#[test]
fn inactive_claim_is_rejected_atomically_without_erasing_accrued_value() {
    let mut ctx = AnchorLiteSVM::build_with_program(ID, PROGRAM_BYTES);
    let initializer = ctx.svm.create_funded_account(30_000_000_000).expect("initializer");
    let treasury = ctx.svm.create_funded_account(10_000_000_000).expect("treasury");
    let user = ctx.svm.create_funded_account(10_000_000_000).expect("user");
    let revenue_source = ctx.svm.create_funded_account(10_000_000_000).expect("revenue source");

    let usdt_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDT mint");
    let usdc_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDC mint");

    let (protocol, _) = Pubkey::find_program_address(&[b"protocol"], &ID);
    let (vault_authority, _) = Pubkey::find_program_address(&[b"vault-authority"], &ID);
    let (technical_root, _) = Pubkey::find_program_address(&[b"user", Pubkey::default().as_ref()], &ID);
    let (user_pda, _) = Pubkey::find_program_address(&[b"user", user.pubkey().as_ref()], &ID);

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
        .args(service_referral_protocol::instruction::Initialize {
            registration_open_at: ctx.svm.get_sysvar::<Clock>().unix_timestamp,
            qualified_revenue_source: revenue_source.pubkey(),
        })
        .instruction().expect("initialize ix");
    ctx.execute_instruction(initialize_ix, &[&initializer])
        .expect("initialize tx").assert_success();

    let register_ix = ctx.program()
        .accounts(service_referral_protocol::accounts::Register {
            wallet: user.pubkey(),
            protocol,
            referrer_wallet: Pubkey::default(),
            referrer: technical_root,
            user: user_pda,
            system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::Register {})
        .instruction().expect("register ix");
    ctx.execute_instruction(register_ix, &[&user])
        .expect("register tx").assert_success();

    // Registration alone must not make the user ACTIVE.
    let user_account = ctx.svm.get_account(&user_pda).expect("user state after registration");
    let mut user_data = user_account.data.as_slice();
    let registered = UserState::try_deserialize(&mut user_data).expect("deserialize registered user");
    assert_eq!(registered.active_until, 0);
    assert_eq!(registered.grace_until, 0);

    let treasury_usdc = ctx.svm.create_associated_token_account(&usdc_mint.pubkey(), &treasury)
        .expect("treasury USDC ATA");
    let user_usdc = ctx.svm.create_associated_token_account(&usdc_mint.pubkey(), &user)
        .expect("user USDC ATA");
    let revenue_usdc = ctx.svm.create_associated_token_account(&usdc_mint.pubkey(), &revenue_source)
        .expect("revenue source USDC ATA");

    let vault_usdc = spl_associated_token_account::get_associated_token_address_with_program_id(
        &vault_authority, &usdc_mint.pubkey(), &spl_token::id());
    let create_vault = spl_associated_token_account::instruction::create_associated_token_account(
        &initializer.pubkey(), &vault_authority, &usdc_mint.pubkey(), &spl_token::id());
    let create_vault_tx = Transaction::new_signed_with_payer(
        &[create_vault], Some(&initializer.pubkey()), &[&initializer], ctx.svm.latest_blockhash());
    ctx.svm.send_transaction(create_vault_tx).expect("create vault ATA");

    let revenue_amount = 100 * UNIT;
    ctx.svm.mint_to(&usdc_mint.pubkey(), &revenue_usdc, &initializer, revenue_amount)
        .expect("fund qualified revenue source");

    // Direct reward accrues to the beneficiary even while inactive. Network levels
    // route to the technical root/treasury here; Pioneer #1 retains 1/100 of 2%.
    let record_ix = ctx.program()
        .accounts(service_referral_protocol::accounts::RecordQualifiedRevenue {
            revenue_source: revenue_source.pubkey(),
            protocol,
            source_token: revenue_usdc,
            vault_authority,
            vault_token: vault_usdc,
            service_treasury_token: treasury_usdc,
            beneficiary: user_pda,
            upline_1: technical_root,
            upline_2: technical_root,
            upline_3: technical_root,
            upline_4: technical_root,
            upline_5: technical_root,
            upline_6: technical_root,
            upline_7: technical_root,
            upline_8: technical_root,
            upline_9: technical_root,
            upline_10: technical_root,
            token_program: spl_token::id(),
        })
        .args(service_referral_protocol::instruction::RecordQualifiedRevenue { amount: revenue_amount })
        .instruction().expect("record revenue ix");
    ctx.execute_instruction(record_ix, &[&revenue_source])
        .expect("record revenue tx").assert_success();

    let expected_direct = 50 * UNIT;
    let expected_pioneer = 20_000u64; // 1/100 of the 2 USDC Pioneer pool.
    let expected_vault = expected_direct + expected_pioneer;
    ctx.svm.assert_token_balance(&vault_usdc, expected_vault);
    ctx.svm.assert_token_balance(&user_usdc, 0);

    let before_account = ctx.svm.get_account(&user_pda).expect("user state before failed claim");
    let mut before_data = before_account.data.as_slice();
    let before = UserState::try_deserialize(&mut before_data).expect("deserialize before failed claim");
    assert_eq!(before.direct_accrued_usdc, expected_direct);
    assert_eq!(before.lifetime_claimed_usdc, 0);

    ctx.svm.expire_blockhash();
    let claim_ix = ctx.program()
        .accounts(service_referral_protocol::accounts::Claim {
            wallet: user.pubkey(),
            protocol,
            user: user_pda,
            vault_authority,
            vault_token: vault_usdc,
            destination: user_usdc,
            token_program: spl_token::id(),
        })
        .args(service_referral_protocol::instruction::Claim {})
        .instruction().expect("claim ix");

    let outcome = ctx.execute_instruction(claim_ix, &[&user]).expect("claim program result");
    assert!(!outcome.is_success(), "an INACTIVE user must not be able to claim");

    // Failed claim must be atomic: no stablecoin movement, no bucket clearing, no
    // Pioneer checkpoint advancement and no lifetime-claimed mutation.
    ctx.svm.assert_token_balance(&vault_usdc, expected_vault);
    ctx.svm.assert_token_balance(&user_usdc, 0);
    let after_account = ctx.svm.get_account(&user_pda).expect("user state after failed claim");
    let mut after_data = after_account.data.as_slice();
    let after = UserState::try_deserialize(&mut after_data).expect("deserialize after failed claim");
    assert_eq!(after.direct_accrued_usdc, before.direct_accrued_usdc);
    assert_eq!(after.network_claimable_usdc, before.network_claimable_usdc);
    assert_eq!(after.pioneer_checkpoint_usdc, before.pioneer_checkpoint_usdc);
    assert_eq!(after.lifetime_claimed_usdc, 0);
}
