use anchor_lang::{prelude::*, AccountDeserialize};
use anchor_litesvm::{AnchorLiteSVM, AssertionHelpers, TestHelpers};
use service_referral_protocol::{state::UserState, ID};
use solana_signer::Signer;
use solana_transaction::Transaction;

const PROGRAM_BYTES: &[u8] = include_bytes!("../../target/deploy/service_referral_protocol.so");
const UNIT: u64 = 1_000_000;

#[test]
fn supplied_upline_must_match_immutable_referral_ancestry() {
    let mut ctx = AnchorLiteSVM::build_with_program(ID, PROGRAM_BYTES);
    let initializer = ctx.svm.create_funded_account(40_000_000_000).expect("initializer");
    let treasury = ctx.svm.create_funded_account(10_000_000_000).expect("treasury");
    let revenue_source = ctx.svm.create_funded_account(10_000_000_000).expect("revenue source");
    let sponsor = ctx.svm.create_funded_account(10_000_000_000).expect("sponsor");
    let beneficiary = ctx.svm.create_funded_account(10_000_000_000).expect("beneficiary");

    let usdt_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDT mint");
    let usdc_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDC mint");
    let (protocol, _) = Pubkey::find_program_address(&[b"protocol"], &ID);
    let (vault_authority, _) = Pubkey::find_program_address(&[b"vault-authority"], &ID);
    let (technical_root, _) = Pubkey::find_program_address(&[b"user", Pubkey::default().as_ref()], &ID);
    let (sponsor_pda, _) = Pubkey::find_program_address(&[b"user", sponsor.pubkey().as_ref()], &ID);
    let (beneficiary_pda, _) = Pubkey::find_program_address(&[b"user", beneficiary.pubkey().as_ref()], &ID);

    let initialize_ix = ctx.program()
        .accounts(service_referral_protocol::accounts::Initialize {
            initializer: initializer.pubkey(), service_treasury: treasury.pubkey(),
            usdt_mint: usdt_mint.pubkey(), usdc_mint: usdc_mint.pubkey(), protocol,
            vault_authority, technical_root, system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::Initialize {
            registration_open_at: ctx.svm.get_sysvar::<Clock>().unix_timestamp,
            qualified_revenue_source: revenue_source.pubkey(),
        }).instruction().expect("initialize ix");
    ctx.execute_instruction(initialize_ix, &[&initializer]).expect("initialize tx").assert_success();

    let sponsor_register_ix = ctx.program()
        .accounts(service_referral_protocol::accounts::Register {
            wallet: sponsor.pubkey(), protocol, referrer_wallet: Pubkey::default(),
            referrer: technical_root, user: sponsor_pda, system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::Register {})
        .instruction().expect("register sponsor ix");
    ctx.execute_instruction(sponsor_register_ix, &[&sponsor]).expect("register sponsor").assert_success();

    let beneficiary_register_ix = ctx.program()
        .accounts(service_referral_protocol::accounts::Register {
            wallet: beneficiary.pubkey(), protocol, referrer_wallet: sponsor.pubkey(),
            referrer: sponsor_pda, user: beneficiary_pda, system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::Register {})
        .instruction().expect("register beneficiary ix");
    ctx.execute_instruction(beneficiary_register_ix, &[&beneficiary]).expect("register beneficiary").assert_success();

    let treasury_usdc = ctx.svm.create_associated_token_account(&usdc_mint.pubkey(), &treasury).expect("treasury USDC");
    let source_usdc = ctx.svm.create_associated_token_account(&usdc_mint.pubkey(), &revenue_source).expect("source USDC");
    let vault_usdc = spl_associated_token_account::get_associated_token_address_with_program_id(
        &vault_authority, &usdc_mint.pubkey(), &spl_token::id());
    let create_vault = spl_associated_token_account::instruction::create_associated_token_account(
        &initializer.pubkey(), &vault_authority, &usdc_mint.pubkey(), &spl_token::id());
    let create_vault_tx = Transaction::new_signed_with_payer(
        &[create_vault], Some(&initializer.pubkey()), &[&initializer], ctx.svm.latest_blockhash());
    ctx.svm.send_transaction(create_vault_tx).expect("create vault");

    let amount = 100 * UNIT;
    ctx.svm.mint_to(&usdc_mint.pubkey(), &source_usdc, &initializer, amount).expect("fund source");

    // The beneficiary's immutable referrer is sponsor, so upline_1 must be sponsor_pda.
    // A different valid UserState (technical_root) must not be accepted as a substitute.
    let record_ix = ctx.program()
        .accounts(service_referral_protocol::accounts::RecordQualifiedRevenue {
            revenue_source: revenue_source.pubkey(), protocol, source_token: source_usdc,
            vault_authority, vault_token: vault_usdc, service_treasury_token: treasury_usdc,
            beneficiary: beneficiary_pda,
            upline_1: technical_root, upline_2: technical_root, upline_3: technical_root,
            upline_4: technical_root, upline_5: technical_root, upline_6: technical_root,
            upline_7: technical_root, upline_8: technical_root, upline_9: technical_root,
            upline_10: technical_root, token_program: spl_token::id(),
        })
        .args(service_referral_protocol::instruction::RecordQualifiedRevenue { amount })
        .instruction().expect("record ix");

    let outcome = ctx.execute_instruction(record_ix, &[&revenue_source]).expect("program result");
    assert!(!outcome.is_success(), "upline substitution must fail immutable ancestry validation");

    // Even though the source->vault CPI occurs before the dynamic ancestry loop,
    // Solana transaction atomicity must roll it back together with any liability writes.
    ctx.svm.assert_token_balance(&source_usdc, amount);
    ctx.svm.assert_token_balance(&vault_usdc, 0);
    ctx.svm.assert_token_balance(&treasury_usdc, 0);

    let beneficiary_account = ctx.svm.get_account(&beneficiary_pda).expect("beneficiary state");
    let mut beneficiary_data = beneficiary_account.data.as_slice();
    let beneficiary_state = UserState::try_deserialize(&mut beneficiary_data).expect("deserialize beneficiary");
    assert_eq!(beneficiary_state.direct_accrued_usdc, 0);

    let sponsor_account = ctx.svm.get_account(&sponsor_pda).expect("sponsor state");
    let mut sponsor_data = sponsor_account.data.as_slice();
    let sponsor_state = UserState::try_deserialize(&mut sponsor_data).expect("deserialize sponsor");
    assert_eq!(sponsor_state.network_claimable_usdc, 0);
    assert_eq!(sponsor_state.network_pending_usdc, 0);
    assert_eq!(sponsor_state.lifetime_expired_usdc, 0);
}
