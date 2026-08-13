use anchor_lang::{prelude::*, AccountDeserialize};
use anchor_litesvm::{AnchorLiteSVM, AssertionHelpers, TestHelpers};
use service_referral_protocol::{state::{ProtocolState, UserState}, ID};
use solana_signer::Signer;
use solana_transaction::Transaction;

const PROGRAM_BYTES: &[u8] = include_bytes!("../../target/deploy/service_referral_protocol.so");
const UNIT: u64 = 1_000_000;

#[test]
fn qualified_revenue_exact_split_and_claim_are_fully_collateralized() {
    let mut ctx = AnchorLiteSVM::build_with_program(ID, PROGRAM_BYTES);
    let initializer = ctx.svm.create_funded_account(30_000_000_000).expect("initializer");
    let treasury = ctx.svm.create_funded_account(10_000_000_000).expect("treasury");
    let user = ctx.svm.create_funded_account(10_000_000_000).expect("user");
    let revenue_source = ctx.svm.create_funded_account(10_000_000_000).expect("revenue source");

    let usdt_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDT mint");
    let usdc_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDC mint");

    let (protocol_pda, _) = Pubkey::find_program_address(&[b"protocol"], &ID);
    let (vault_authority, _) = Pubkey::find_program_address(&[b"vault-authority"], &ID);
    let (technical_root, _) = Pubkey::find_program_address(&[b"user", Pubkey::default().as_ref()], &ID);
    let (user_pda, _) = Pubkey::find_program_address(&[b"user", user.pubkey().as_ref()], &ID);
    let (batch_pda, _) = Pubkey::find_program_address(
        &[b"batch", user.pubkey().as_ref(), &0u64.to_le_bytes()],
        &ID,
    );

    let registration_open_at = ctx.svm.get_sysvar::<Clock>().unix_timestamp;
    let initialize_ix = ctx
        .program()
        .accounts(service_referral_protocol::accounts::Initialize {
            initializer: initializer.pubkey(),
            service_treasury: treasury.pubkey(),
            usdt_mint: usdt_mint.pubkey(),
            usdc_mint: usdc_mint.pubkey(),
            protocol: protocol_pda,
            vault_authority,
            technical_root,
            system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::Initialize {
            registration_open_at,
            qualified_revenue_source: revenue_source.pubkey(),
        })
        .instruction()
        .expect("initialize ix");
    ctx.execute_instruction(initialize_ix, &[&initializer])
        .expect("initialize tx")
        .assert_success();

    let register_ix = ctx
        .program()
        .accounts(service_referral_protocol::accounts::Register {
            wallet: user.pubkey(),
            protocol: protocol_pda,
            referrer_wallet: Pubkey::default(),
            referrer: technical_root,
            user: user_pda,
            system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::Register {})
        .instruction()
        .expect("register ix");
    ctx.execute_instruction(register_ix, &[&user])
        .expect("register tx")
        .assert_success();

    let treasury_usdt = ctx
        .svm
        .create_associated_token_account(&usdt_mint.pubkey(), &treasury)
        .expect("treasury USDT ATA");
    let treasury_usdc = ctx
        .svm
        .create_associated_token_account(&usdc_mint.pubkey(), &treasury)
        .expect("treasury USDC ATA");
    let user_usdc = ctx
        .svm
        .create_associated_token_account(&usdc_mint.pubkey(), &user)
        .expect("user USDC ATA");
    let revenue_usdc = ctx
        .svm
        .create_associated_token_account(&usdc_mint.pubkey(), &revenue_source)
        .expect("revenue source USDC ATA");

    let vault_usdt = spl_associated_token_account::get_associated_token_address_with_program_id(
        &vault_authority,
        &usdt_mint.pubkey(),
        &spl_token::id(),
    );
    let vault_usdc = spl_associated_token_account::get_associated_token_address_with_program_id(
        &vault_authority,
        &usdc_mint.pubkey(),
        &spl_token::id(),
    );
    let create_vault_usdt = spl_associated_token_account::instruction::create_associated_token_account(
        &initializer.pubkey(),
        &vault_authority,
        &usdt_mint.pubkey(),
        &spl_token::id(),
    );
    let create_vault_usdc = spl_associated_token_account::instruction::create_associated_token_account(
        &initializer.pubkey(),
        &vault_authority,
        &usdc_mint.pubkey(),
        &spl_token::id(),
    );
    let create_vaults_tx = Transaction::new_signed_with_payer(
        &[create_vault_usdt, create_vault_usdc],
        Some(&initializer.pubkey()),
        &[&initializer],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(create_vaults_tx).expect("create vault ATAs");

    ctx.svm
        .mint_to(&usdc_mint.pubkey(), &user_usdc, &initializer, 10 * UNIT)
        .expect("mint activation USDC");
    ctx.svm
        .mint_to(&usdc_mint.pubkey(), &revenue_usdc, &initializer, 100 * UNIT)
        .expect("mint qualified revenue USDC");

    let purchase_ix = ctx
        .program()
        .accounts(service_referral_protocol::accounts::PurchaseServiceUnits {
            wallet: user.pubkey(),
            protocol: protocol_pda,
            user: user_pda,
            user_source: user_usdc,
            vault_authority,
            usdt_vault: vault_usdt,
            usdc_vault: vault_usdc,
            service_treasury_usdt: treasury_usdt,
            service_treasury_usdc: treasury_usdc,
            batch: batch_pda,
            token_program: spl_token::id(),
            system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::PurchaseServiceUnits { units: 10 })
        .instruction()
        .expect("purchase ix");
    ctx.execute_instruction(purchase_ix, &[&user])
        .expect("purchase tx")
        .assert_success();

    ctx.svm.assert_token_balance(&user_usdc, 0);
    ctx.svm.assert_token_balance(&treasury_usdc, 10 * UNIT);
    ctx.svm.assert_token_balance(&vault_usdc, 0);

    let revenue_amount = 100 * UNIT;
    let direct = 50 * UNIT;
    let network_unallocated = 43 * UNIT;
    let pioneer_pool = 2 * UNIT;
    let service_fee = 5 * UNIT;
    let pioneer_assigned = pioneer_pool / 100;
    let pioneer_unassigned = pioneer_pool - pioneer_assigned;
    let treasury_delta = network_unallocated + service_fee + pioneer_unassigned;
    let user_liability = direct + pioneer_assigned;
    assert_eq!(direct + network_unallocated + pioneer_pool + service_fee, revenue_amount);
    assert_eq!(treasury_delta + user_liability, revenue_amount);

    let record_ix = ctx
        .program()
        .accounts(service_referral_protocol::accounts::RecordQualifiedRevenue {
            revenue_source: revenue_source.pubkey(),
            protocol: protocol_pda,
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
        .instruction()
        .expect("record qualified revenue ix");
    ctx.execute_instruction(record_ix, &[&revenue_source])
        .expect("record qualified revenue tx")
        .assert_success();

    ctx.svm.assert_token_balance(&revenue_usdc, 0);
    ctx.svm.assert_token_balance(&treasury_usdc, 10 * UNIT + treasury_delta);
    ctx.svm.assert_token_balance(&vault_usdc, user_liability);

    let user_account = ctx.svm.get_account(&user_pda).expect("user state");
    let mut user_data = user_account.data.as_slice();
    let user_state = UserState::try_deserialize(&mut user_data).expect("deserialize user state");
    assert_eq!(user_state.pioneer_id, 1);
    assert_eq!(user_state.direct_accrued_usdc, direct);
    assert_eq!(user_state.network_claimable_usdc, 0);
    assert_eq!(user_state.network_pending_usdc, 0);

    let protocol_account = ctx.svm.get_account(&protocol_pda).expect("protocol state");
    let mut protocol_data = protocol_account.data.as_slice();
    let protocol = ProtocolState::try_deserialize(&mut protocol_data).expect("deserialize protocol state");
    assert_eq!(protocol.lifetime_service_fees_usdc, service_fee as u128);
    assert_eq!(protocol.lifetime_unallocated_usdc, network_unallocated as u128);
    assert_eq!(protocol.lifetime_pioneer_unassigned_usdc, pioneer_unassigned as u128);
    assert_eq!(protocol.lifetime_expired_usdc, 0);

    let claim_ix = ctx
        .program()
        .accounts(service_referral_protocol::accounts::Claim {
            wallet: user.pubkey(),
            protocol: protocol_pda,
            user: user_pda,
            vault_authority,
            vault_token: vault_usdc,
            destination: user_usdc,
            token_program: spl_token::id(),
        })
        .args(service_referral_protocol::instruction::Claim {})
        .instruction()
        .expect("claim ix");
    ctx.execute_instruction(claim_ix, &[&user])
        .expect("claim tx")
        .assert_success();

    ctx.svm.assert_token_balance(&user_usdc, user_liability);
    ctx.svm.assert_token_balance(&vault_usdc, 0);
    ctx.svm.assert_token_balance(&treasury_usdc, 10 * UNIT + treasury_delta);

    let user_account = ctx.svm.get_account(&user_pda).expect("user state after claim");
    let mut user_data = user_account.data.as_slice();
    let user_state = UserState::try_deserialize(&mut user_data).expect("deserialize user state after claim");
    assert_eq!(user_state.direct_accrued_usdc, 0);
    assert_eq!(user_state.network_claimable_usdc, 0);
    assert_eq!(user_state.lifetime_claimed_usdc, user_liability as u128);
}
