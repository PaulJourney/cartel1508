use anchor_lang::{prelude::*, AccountDeserialize};
use anchor_litesvm::{AnchorLiteSVM, AssertionHelpers, TestHelpers};
use service_referral_protocol::{state::{ProtocolState, UnitBatch}, ID};
use solana_signer::Signer;
use solana_transaction::Transaction;

const PROGRAM_BYTES: &[u8] = include_bytes!("../../target/deploy/service_referral_protocol.so");
const UNIT: u64 = 1_000_000;

#[test]
fn unit_ranges_are_global_across_different_wallets() {
    let mut ctx = AnchorLiteSVM::build_with_program(ID, PROGRAM_BYTES);
    let initializer = ctx.svm.create_funded_account(40_000_000_000).expect("initializer");
    let treasury = ctx.svm.create_funded_account(10_000_000_000).expect("treasury");
    let revenue_source = ctx.svm.create_funded_account(10_000_000_000).expect("revenue source");
    let user_a = ctx.svm.create_funded_account(10_000_000_000).expect("user a");
    let user_b = ctx.svm.create_funded_account(10_000_000_000).expect("user b");

    let usdt_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDT mint");
    let usdc_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDC mint");

    let (protocol, _) = Pubkey::find_program_address(&[b"protocol"], &ID);
    let (vault_authority, _) = Pubkey::find_program_address(&[b"vault-authority"], &ID);
    let (technical_root, _) = Pubkey::find_program_address(&[b"user", Pubkey::default().as_ref()], &ID);
    let (user_a_pda, _) = Pubkey::find_program_address(&[b"user", user_a.pubkey().as_ref()], &ID);
    let (user_b_pda, _) = Pubkey::find_program_address(&[b"user", user_b.pubkey().as_ref()], &ID);
    let (batch_a, _) = Pubkey::find_program_address(&[b"batch", user_a.pubkey().as_ref(), &0u64.to_le_bytes()], &ID);
    let (batch_b, _) = Pubkey::find_program_address(&[b"batch", user_b.pubkey().as_ref(), &0u64.to_le_bytes()], &ID);

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
    ctx.execute_instruction(initialize_ix, &[&initializer]).expect("initialize tx").assert_success();

    for (wallet, user_pda) in [(&user_a, user_a_pda), (&user_b, user_b_pda)] {
        let register_ix = ctx.program()
            .accounts(service_referral_protocol::accounts::Register {
                wallet: wallet.pubkey(),
                protocol,
                referrer_wallet: Pubkey::default(),
                referrer: technical_root,
                user: user_pda,
                system_program: anchor_lang::system_program::ID,
            })
            .args(service_referral_protocol::instruction::Register {})
            .instruction().expect("register ix");
        ctx.execute_instruction(register_ix, &[wallet]).expect("register tx").assert_success();
    }

    let treasury_usdt = ctx.svm.create_associated_token_account(&usdt_mint.pubkey(), &treasury).expect("treasury USDT");
    let treasury_usdc = ctx.svm.create_associated_token_account(&usdc_mint.pubkey(), &treasury).expect("treasury USDC");
    let user_a_usdc = ctx.svm.create_associated_token_account(&usdc_mint.pubkey(), &user_a).expect("user a USDC");
    let user_b_usdc = ctx.svm.create_associated_token_account(&usdc_mint.pubkey(), &user_b).expect("user b USDC");

    let vault_usdt = spl_associated_token_account::get_associated_token_address_with_program_id(
        &vault_authority, &usdt_mint.pubkey(), &spl_token::id());
    let vault_usdc = spl_associated_token_account::get_associated_token_address_with_program_id(
        &vault_authority, &usdc_mint.pubkey(), &spl_token::id());
    let create_vault_usdt = spl_associated_token_account::instruction::create_associated_token_account(
        &initializer.pubkey(), &vault_authority, &usdt_mint.pubkey(), &spl_token::id());
    let create_vault_usdc = spl_associated_token_account::instruction::create_associated_token_account(
        &initializer.pubkey(), &vault_authority, &usdc_mint.pubkey(), &spl_token::id());
    let create_vaults_tx = Transaction::new_signed_with_payer(
        &[create_vault_usdt, create_vault_usdc], Some(&initializer.pubkey()), &[&initializer], ctx.svm.latest_blockhash());
    ctx.svm.send_transaction(create_vaults_tx).expect("create vaults");

    ctx.svm.mint_to(&usdc_mint.pubkey(), &user_a_usdc, &initializer, 3 * UNIT).expect("fund a");
    ctx.svm.mint_to(&usdc_mint.pubkey(), &user_b_usdc, &initializer, 7 * UNIT).expect("fund b");

    let purchase_a = ctx.program()
        .accounts(service_referral_protocol::accounts::PurchaseServiceUnits {
            wallet: user_a.pubkey(), protocol, user: user_a_pda, user_source: user_a_usdc,
            vault_authority, usdt_vault: vault_usdt, usdc_vault: vault_usdc,
            service_treasury_usdt: treasury_usdt, service_treasury_usdc: treasury_usdc,
            batch: batch_a, token_program: spl_token::id(), system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::PurchaseServiceUnits { units: 3 })
        .instruction().expect("purchase a");
    ctx.execute_instruction(purchase_a, &[&user_a]).expect("purchase a tx").assert_success();

    let purchase_b = ctx.program()
        .accounts(service_referral_protocol::accounts::PurchaseServiceUnits {
            wallet: user_b.pubkey(), protocol, user: user_b_pda, user_source: user_b_usdc,
            vault_authority, usdt_vault: vault_usdt, usdc_vault: vault_usdc,
            service_treasury_usdt: treasury_usdt, service_treasury_usdc: treasury_usdc,
            batch: batch_b, token_program: spl_token::id(), system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::PurchaseServiceUnits { units: 7 })
        .instruction().expect("purchase b");
    ctx.execute_instruction(purchase_b, &[&user_b]).expect("purchase b tx").assert_success();

    let a_account = ctx.svm.get_account(&batch_a).expect("batch a");
    let mut a_data = a_account.data.as_slice();
    let a = UnitBatch::try_deserialize(&mut a_data).expect("deserialize a");
    let b_account = ctx.svm.get_account(&batch_b).expect("batch b");
    let mut b_data = b_account.data.as_slice();
    let b = UnitBatch::try_deserialize(&mut b_data).expect("deserialize b");

    assert_eq!((a.first_unit_id, a.last_unit_id), (1, 3));
    assert_eq!((b.first_unit_id, b.last_unit_id), (4, 10));
    assert!(a.last_unit_id < b.first_unit_id);

    let protocol_account = ctx.svm.get_account(&protocol).expect("protocol");
    let mut protocol_data = protocol_account.data.as_slice();
    let protocol_state = ProtocolState::try_deserialize(&mut protocol_data).expect("deserialize protocol");
    assert_eq!(protocol_state.next_unit_id, 11);

    ctx.svm.assert_token_balance(&treasury_usdc, 10 * UNIT);
    ctx.svm.assert_token_balance(&vault_usdc, 0);
}
