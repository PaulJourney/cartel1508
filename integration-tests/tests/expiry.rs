use anchor_lang::{prelude::*, AccountDeserialize};
use anchor_litesvm::{AnchorLiteSVM, AssertionHelpers, TestHelpers};
use service_referral_protocol::{state::{ProtocolState, UserState}, ID};
use solana_signer::Signer;
use solana_transaction::Transaction;

const PROGRAM_BYTES: &[u8] = include_bytes!("../../target/deploy/service_referral_protocol.so");
const UNIT: u64 = 1_000_000;

#[test]
fn pending_reward_expires_to_treasury_and_late_reactivation_cannot_rescue_it() {
    let mut ctx = AnchorLiteSVM::build_with_program(ID, PROGRAM_BYTES);
    let initializer = ctx.svm.create_funded_account(40_000_000_000).expect("initializer");
    let treasury = ctx.svm.create_funded_account(10_000_000_000).expect("treasury");
    let sponsor = ctx.svm.create_funded_account(10_000_000_000).expect("sponsor");
    let beneficiary = ctx.svm.create_funded_account(10_000_000_000).expect("beneficiary");
    let revenue_source = ctx.svm.create_funded_account(10_000_000_000).expect("revenue source");

    let usdt_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDT mint");
    let usdc_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDC mint");

    let (protocol_pda, _) = Pubkey::find_program_address(&[b"protocol"], &ID);
    let (vault_authority, _) = Pubkey::find_program_address(&[b"vault-authority"], &ID);
    let (technical_root, _) = Pubkey::find_program_address(&[b"user", Pubkey::default().as_ref()], &ID);
    let (sponsor_pda, _) = Pubkey::find_program_address(&[b"user", sponsor.pubkey().as_ref()], &ID);
    let (beneficiary_pda, _) = Pubkey::find_program_address(&[b"user", beneficiary.pubkey().as_ref()], &ID);
    let (sponsor_batch_0, _) = Pubkey::find_program_address(
        &[b"batch", sponsor.pubkey().as_ref(), &0u64.to_le_bytes()],
        &ID,
    );
    let (sponsor_batch_1, _) = Pubkey::find_program_address(
        &[b"batch", sponsor.pubkey().as_ref(), &1u64.to_le_bytes()],
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

    let register_sponsor_ix = ctx
        .program()
        .accounts(service_referral_protocol::accounts::Register {
            wallet: sponsor.pubkey(),
            protocol: protocol_pda,
            referrer_wallet: Pubkey::default(),
            referrer: technical_root,
            user: sponsor_pda,
            system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::Register {})
        .instruction()
        .expect("register sponsor ix");
    ctx.execute_instruction(register_sponsor_ix, &[&sponsor])
        .expect("register sponsor")
        .assert_success();

    let register_beneficiary_ix = ctx
        .program()
        .accounts(service_referral_protocol::accounts::Register {
            wallet: beneficiary.pubkey(),
            protocol: protocol_pda,
            referrer_wallet: sponsor.pubkey(),
            referrer: sponsor_pda,
            user: beneficiary_pda,
            system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::Register {})
        .instruction()
        .expect("register beneficiary ix");
    ctx.execute_instruction(register_beneficiary_ix, &[&beneficiary])
        .expect("register beneficiary")
        .assert_success();

    let treasury_usdt = ctx.svm.create_associated_token_account(&usdt_mint.pubkey(), &treasury).expect("treasury USDT ATA");
    let treasury_usdc = ctx.svm.create_associated_token_account(&usdc_mint.pubkey(), &treasury).expect("treasury USDC ATA");
    let sponsor_usdc = ctx.svm.create_associated_token_account(&usdc_mint.pubkey(), &sponsor).expect("sponsor USDC ATA");
    let revenue_usdc = ctx.svm.create_associated_token_account(&usdc_mint.pubkey(), &revenue_source).expect("revenue USDC ATA");

    let vault_usdt = spl_associated_token_account::get_associated_token_address_with_program_id(
        &vault_authority, &usdt_mint.pubkey(), &spl_token::id(),
    );
    let vault_usdc = spl_associated_token_account::get_associated_token_address_with_program_id(
        &vault_authority, &usdc_mint.pubkey(), &spl_token::id(),
    );
    let create_vault_usdt = spl_associated_token_account::instruction::create_associated_token_account(
        &initializer.pubkey(), &vault_authority, &usdt_mint.pubkey(), &spl_token::id(),
    );
    let create_vault_usdc = spl_associated_token_account::instruction::create_associated_token_account(
        &initializer.pubkey(), &vault_authority, &usdc_mint.pubkey(), &spl_token::id(),
    );
    let create_vaults_tx = Transaction::new_signed_with_payer(
        &[create_vault_usdt, create_vault_usdc],
        Some(&initializer.pubkey()),
        &[&initializer],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(create_vaults_tx).expect("create vault ATAs");

    ctx.svm.mint_to(&usdc_mint.pubkey(), &sponsor_usdc, &initializer, 20 * UNIT).expect("mint sponsor activation funds");
    ctx.svm.mint_to(&usdc_mint.pubkey(), &revenue_usdc, &initializer, 100 * UNIT).expect("mint revenue funds");

    let activate_ix = ctx
        .program()
        .accounts(service_referral_protocol::accounts::PurchaseServiceUnits {
            wallet: sponsor.pubkey(), protocol: protocol_pda, user: sponsor_pda,
            user_source: sponsor_usdc, vault_authority, usdt_vault: vault_usdt,
            usdc_vault: vault_usdc, service_treasury_usdt: treasury_usdt,
            service_treasury_usdc: treasury_usdc, batch: sponsor_batch_0,
            token_program: spl_token::id(), system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::PurchaseServiceUnits { units: 10 })
        .instruction().expect("activate sponsor ix");
    ctx.execute_instruction(activate_ix, &[&sponsor]).expect("activate sponsor").assert_success();

    let sponsor_account = ctx.svm.get_account(&sponsor_pda).expect("sponsor state");
    let mut sponsor_data = sponsor_account.data.as_slice();
    let sponsor_state = UserState::try_deserialize(&mut sponsor_data).expect("deserialize sponsor");
    let active_until = sponsor_state.active_until;
    let grace_until = sponsor_state.grace_until;

    let mut clock: Clock = ctx.svm.get_sysvar();
    clock.unix_timestamp = active_until + 1;
    ctx.svm.set_sysvar(&clock);
    ctx.svm.expire_blockhash();

    let record_ix = ctx
        .program()
        .accounts(service_referral_protocol::accounts::RecordQualifiedRevenue {
            revenue_source: revenue_source.pubkey(), protocol: protocol_pda,
            source_token: revenue_usdc, vault_authority, vault_token: vault_usdc,
            service_treasury_token: treasury_usdc, beneficiary: beneficiary_pda,
            upline_1: sponsor_pda, upline_2: technical_root, upline_3: technical_root,
            upline_4: technical_root, upline_5: technical_root, upline_6: technical_root,
            upline_7: technical_root, upline_8: technical_root, upline_9: technical_root,
            upline_10: technical_root, token_program: spl_token::id(),
        })
        .args(service_referral_protocol::instruction::RecordQualifiedRevenue { amount: 100 * UNIT })
        .instruction().expect("record revenue ix");
    ctx.execute_instruction(record_ix, &[&revenue_source]).expect("record revenue").assert_success();

    let sponsor_account = ctx.svm.get_account(&sponsor_pda).expect("sponsor after grace revenue");
    let mut sponsor_data = sponsor_account.data.as_slice();
    let sponsor_state = UserState::try_deserialize(&mut sponsor_data).expect("deserialize sponsor after grace revenue");
    assert_eq!(sponsor_state.network_pending_usdc, 15 * UNIT);
    assert_eq!(sponsor_state.network_claimable_usdc, 0);
    ctx.svm.assert_token_balance(&treasury_usdc, 10 * UNIT + 34_960_000);
    ctx.svm.assert_token_balance(&vault_usdc, 65_040_000);

    clock.unix_timestamp = grace_until + 1;
    ctx.svm.set_sysvar(&clock);
    ctx.svm.expire_blockhash();

    let settle_ix = ctx
        .program()
        .accounts(service_referral_protocol::accounts::SettleExpired {
            settler: initializer.pubkey(), protocol: protocol_pda, user: sponsor_pda,
            vault_authority, vault_token: vault_usdc,
            service_treasury_token: treasury_usdc, token_program: spl_token::id(),
        })
        .args(service_referral_protocol::instruction::SettleExpired {})
        .instruction().expect("settle expired ix");
    ctx.execute_instruction(settle_ix, &[&initializer]).expect("settle expired").assert_success();

    let sponsor_account = ctx.svm.get_account(&sponsor_pda).expect("sponsor after expiry");
    let mut sponsor_data = sponsor_account.data.as_slice();
    let sponsor_state = UserState::try_deserialize(&mut sponsor_data).expect("deserialize sponsor after expiry");
    assert_eq!(sponsor_state.network_pending_usdc, 0);
    assert_eq!(sponsor_state.network_claimable_usdc, 0);
    assert_eq!(sponsor_state.lifetime_expired_usdc, 15 * UNIT as u128);

    let protocol_account = ctx.svm.get_account(&protocol_pda).expect("protocol after expiry");
    let mut protocol_data = protocol_account.data.as_slice();
    let protocol_state = ProtocolState::try_deserialize(&mut protocol_data).expect("deserialize protocol after expiry");
    assert_eq!(protocol_state.lifetime_expired_usdc, 15 * UNIT as u128);
    ctx.svm.assert_token_balance(&treasury_usdc, 10 * UNIT + 49_960_000);
    ctx.svm.assert_token_balance(&vault_usdc, 50_040_000);

    let reactivate_ix = ctx
        .program()
        .accounts(service_referral_protocol::accounts::PurchaseServiceUnits {
            wallet: sponsor.pubkey(), protocol: protocol_pda, user: sponsor_pda,
            user_source: sponsor_usdc, vault_authority, usdt_vault: vault_usdt,
            usdc_vault: vault_usdc, service_treasury_usdt: treasury_usdt,
            service_treasury_usdc: treasury_usdc, batch: sponsor_batch_1,
            token_program: spl_token::id(), system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::PurchaseServiceUnits { units: 10 })
        .instruction().expect("late reactivate ix");
    ctx.execute_instruction(reactivate_ix, &[&sponsor]).expect("late reactivate").assert_success();

    let sponsor_account = ctx.svm.get_account(&sponsor_pda).expect("sponsor after late reactivation");
    let mut sponsor_data = sponsor_account.data.as_slice();
    let sponsor_state = UserState::try_deserialize(&mut sponsor_data).expect("deserialize sponsor after late reactivation");
    assert_eq!(sponsor_state.network_pending_usdc, 0);
    assert_eq!(sponsor_state.network_claimable_usdc, 0);
    assert_eq!(sponsor_state.lifetime_expired_usdc, 15 * UNIT as u128);
    assert_eq!(sponsor_state.lifetime_service_units, 20);
    assert!(sponsor_state.active_until > clock.unix_timestamp);

    ctx.svm.assert_token_balance(&sponsor_usdc, 0);
    ctx.svm.assert_token_balance(&treasury_usdc, 20 * UNIT + 49_960_000);
    ctx.svm.assert_token_balance(&vault_usdc, 50_040_000);
}
