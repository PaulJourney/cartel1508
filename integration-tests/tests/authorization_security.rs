use anchor_lang::{prelude::*, AccountDeserialize};
use anchor_litesvm::{AnchorLiteSVM, AssertionHelpers, TestHelpers};
use service_referral_protocol::{state::UserState, ID};
use solana_signer::Signer;
use solana_transaction::Transaction;

const PROGRAM_BYTES: &[u8] = include_bytes!("../../target/deploy/service_referral_protocol.so");
const UNIT: u64 = 1_000_000;

#[test]
fn only_configured_revenue_source_can_record_qualified_revenue() {
    let mut ctx = AnchorLiteSVM::build_with_program(ID, PROGRAM_BYTES);
    let initializer = ctx.svm.create_funded_account(30_000_000_000).expect("initializer");
    let treasury = ctx.svm.create_funded_account(10_000_000_000).expect("treasury");
    let configured_source = ctx.svm.create_funded_account(10_000_000_000).expect("configured source");
    let other_source = ctx.svm.create_funded_account(10_000_000_000).expect("other source");
    let beneficiary = ctx.svm.create_funded_account(10_000_000_000).expect("beneficiary");

    let usdt_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDT mint");
    let usdc_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDC mint");
    let (protocol, _) = Pubkey::find_program_address(&[b"protocol"], &ID);
    let (vault_authority, _) = Pubkey::find_program_address(&[b"vault-authority"], &ID);
    let (technical_root, _) = Pubkey::find_program_address(&[b"user", Pubkey::default().as_ref()], &ID);
    let (beneficiary_pda, _) = Pubkey::find_program_address(&[b"user", beneficiary.pubkey().as_ref()], &ID);

    let initialize_ix = ctx.program()
        .accounts(service_referral_protocol::accounts::Initialize {
            initializer: initializer.pubkey(), service_treasury: treasury.pubkey(),
            usdt_mint: usdt_mint.pubkey(), usdc_mint: usdc_mint.pubkey(), protocol,
            vault_authority, technical_root, system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::Initialize {
            registration_open_at: ctx.svm.get_sysvar::<Clock>().unix_timestamp,
            qualified_revenue_source: configured_source.pubkey(),
        }).instruction().expect("initialize ix");
    ctx.execute_instruction(initialize_ix, &[&initializer]).expect("initialize tx").assert_success();

    let register_ix = ctx.program()
        .accounts(service_referral_protocol::accounts::Register {
            wallet: beneficiary.pubkey(), protocol, referrer_wallet: Pubkey::default(),
            referrer: technical_root, user: beneficiary_pda, system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::Register {})
        .instruction().expect("register ix");
    ctx.execute_instruction(register_ix, &[&beneficiary]).expect("register tx").assert_success();

    let treasury_usdc = ctx.svm.create_associated_token_account(&usdc_mint.pubkey(), &treasury).expect("treasury USDC");
    let other_source_usdc = ctx.svm.create_associated_token_account(&usdc_mint.pubkey(), &other_source).expect("other source USDC");
    let vault_usdc = spl_associated_token_account::get_associated_token_address_with_program_id(
        &vault_authority, &usdc_mint.pubkey(), &spl_token::id());
    let create_vault = spl_associated_token_account::instruction::create_associated_token_account(
        &initializer.pubkey(), &vault_authority, &usdc_mint.pubkey(), &spl_token::id());
    let create_vault_tx = Transaction::new_signed_with_payer(
        &[create_vault], Some(&initializer.pubkey()), &[&initializer], ctx.svm.latest_blockhash());
    ctx.svm.send_transaction(create_vault_tx).expect("create vault");

    let amount = 100 * UNIT;
    ctx.svm.mint_to(&usdc_mint.pubkey(), &other_source_usdc, &initializer, amount).expect("fund other source");

    let record_ix = ctx.program()
        .accounts(service_referral_protocol::accounts::RecordQualifiedRevenue {
            revenue_source: other_source.pubkey(), protocol, source_token: other_source_usdc,
            vault_authority, vault_token: vault_usdc, service_treasury_token: treasury_usdc,
            beneficiary: beneficiary_pda,
            upline_1: technical_root, upline_2: technical_root, upline_3: technical_root,
            upline_4: technical_root, upline_5: technical_root, upline_6: technical_root,
            upline_7: technical_root, upline_8: technical_root, upline_9: technical_root,
            upline_10: technical_root, token_program: spl_token::id(),
        })
        .args(service_referral_protocol::instruction::RecordQualifiedRevenue { amount })
        .instruction().expect("record ix");

    let outcome = ctx.execute_instruction(record_ix, &[&other_source]).expect("program result");
    assert!(!outcome.is_success(), "only the immutable configured source may record revenue");
    ctx.svm.assert_token_balance(&other_source_usdc, amount);
    ctx.svm.assert_token_balance(&vault_usdc, 0);
    ctx.svm.assert_token_balance(&treasury_usdc, 0);

    let account = ctx.svm.get_account(&beneficiary_pda).expect("beneficiary");
    let mut data = account.data.as_slice();
    let user = UserState::try_deserialize(&mut data).expect("deserialize beneficiary");
    assert_eq!(user.direct_accrued_usdc, 0);
    assert_eq!(user.network_claimable_usdc, 0);
    assert_eq!(user.network_pending_usdc, 0);
}
