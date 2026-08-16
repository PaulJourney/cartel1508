use anchor_lang::{prelude::*, AccountDeserialize};
use anchor_litesvm::{AnchorLiteSVM, AssertionHelpers, TestHelpers};
use service_referral_protocol::{
    state::{ProtocolState, UserState},
    ID,
};
use solana_signer::Signer;
use solana_transaction::Transaction;

const PROGRAM_BYTES: &[u8] = include_bytes!("../../target/deploy/service_referral_protocol.so");
const UNIT: u64 = 1_000_000;

#[test]
fn purchase_is_the_single_revenue_event_and_recipients_pull_claims() {
    let mut ctx = AnchorLiteSVM::build_with_program(ID, PROGRAM_BYTES);
    let initializer = ctx.svm.create_funded_account(40_000_000_000).expect("initializer");
    let treasury = ctx.svm.create_funded_account(10_000_000_000).expect("treasury");
    let sponsor = ctx.svm.create_funded_account(10_000_000_000).expect("sponsor");
    let buyer = ctx.svm.create_funded_account(10_000_000_000).expect("buyer");

    let usdt_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDT mint");
    let usdc_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDC mint");

    let (protocol_pda, _) = Pubkey::find_program_address(&[b"protocol"], &ID);
    let (vault_authority, _) = Pubkey::find_program_address(&[b"vault-authority"], &ID);
    let (technical_root, _) = Pubkey::find_program_address(&[b"user", Pubkey::default().as_ref()], &ID);
    let (sponsor_pda, _) = Pubkey::find_program_address(&[b"user", sponsor.pubkey().as_ref()], &ID);
    let (buyer_pda, _) = Pubkey::find_program_address(&[b"user", buyer.pubkey().as_ref()], &ID);

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
        .args(service_referral_protocol::instruction::Initialize { registration_open_at })
        .instruction()
        .expect("initialize ix");
    ctx.execute_instruction(initialize_ix, &[&initializer])
        .expect("initialize tx")
        .assert_success();

    let treasury_usdt = ctx
        .svm
        .create_associated_token_account(&usdt_mint.pubkey(), &treasury)
        .expect("treasury USDT ATA");
    let treasury_usdc = ctx
        .svm
        .create_associated_token_account(&usdc_mint.pubkey(), &treasury)
        .expect("treasury USDC ATA");
    let sponsor_usdc = ctx
        .svm
        .create_associated_token_account(&usdc_mint.pubkey(), &sponsor)
        .expect("sponsor USDC ATA");
    let buyer_usdc = ctx
        .svm
        .create_associated_token_account(&usdc_mint.pubkey(), &buyer)
        .expect("buyer USDC ATA");

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
        .expect("register sponsor tx")
        .assert_success();

    ctx.svm
        .mint_to(&usdc_mint.pubkey(), &sponsor_usdc, &initializer, 10 * UNIT)
        .expect("mint sponsor activation USDC");

    let sponsor_purchase_ix = ctx
        .program()
        .accounts(service_referral_protocol::accounts::PurchaseAndDistribute {
            wallet: sponsor.pubkey(),
            protocol: protocol_pda,
            user: sponsor_pda,
            user_source: sponsor_usdc,
            vault_authority,
            usdt_vault: vault_usdt,
            usdc_vault: vault_usdc,
            service_treasury_usdt: treasury_usdt,
            service_treasury_usdc: treasury_usdc,
            direct_referrer: technical_root,
            upline_1: technical_root,
            upline_2: technical_root,
            upline_3: technical_root,
            upline_4: technical_root,
            upline_5: technical_root,
            upline_6: technical_root,
            upline_7: technical_root,
            upline_8: technical_root,
            token_program: spl_token::id(),
        })
        .args(service_referral_protocol::instruction::PurchaseAndDistribute { units: 10 })
        .instruction()
        .expect("sponsor purchase ix");
    ctx.execute_instruction(sponsor_purchase_ix, &[&sponsor])
        .expect("sponsor purchase tx")
        .assert_success();

    ctx.svm.assert_token_balance(&sponsor_usdc, 0);
    ctx.svm.assert_token_balance(&treasury_usdc, 5_000_000);
    ctx.svm.assert_token_balance(&vault_usdc, 5_000_000);

    let register_buyer_ix = ctx
        .program()
        .accounts(service_referral_protocol::accounts::Register {
            wallet: buyer.pubkey(),
            protocol: protocol_pda,
            referrer_wallet: sponsor.pubkey(),
            referrer: sponsor_pda,
            user: buyer_pda,
            system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::Register {})
        .instruction()
        .expect("register buyer ix");
    ctx.execute_instruction(register_buyer_ix, &[&buyer])
        .expect("register buyer tx")
        .assert_success();

    ctx.svm
        .mint_to(&usdc_mint.pubkey(), &buyer_usdc, &initializer, 100 * UNIT)
        .expect("mint buyer purchase USDC");

    let buyer_purchase_ix = ctx
        .program()
        .accounts(service_referral_protocol::accounts::PurchaseAndDistribute {
            wallet: buyer.pubkey(),
            protocol: protocol_pda,
            user: buyer_pda,
            user_source: buyer_usdc,
            vault_authority,
            usdt_vault: vault_usdt,
            usdc_vault: vault_usdc,
            service_treasury_usdt: treasury_usdt,
            service_treasury_usdc: treasury_usdc,
            direct_referrer: sponsor_pda,
            upline_1: technical_root,
            upline_2: technical_root,
            upline_3: technical_root,
            upline_4: technical_root,
            upline_5: technical_root,
            upline_6: technical_root,
            upline_7: technical_root,
            upline_8: technical_root,
            token_program: spl_token::id(),
        })
        .args(service_referral_protocol::instruction::PurchaseAndDistribute { units: 100 })
        .instruction()
        .expect("buyer purchase ix");
    ctx.execute_instruction(buyer_purchase_ix, &[&buyer])
        .expect("buyer purchase tx")
        .assert_success();

    let self_reward = 50 * UNIT;
    let sponsor_network = 15 * UNIT;
    let network_unallocated = 28 * UNIT;
    let service_fee = 5 * UNIT;
    let pioneer_pool = 2 * UNIT;
    let pioneer_assigned = 0;
    let pioneer_unassigned = pioneer_pool;
    let purchase_treasury_delta = network_unallocated + service_fee + pioneer_unassigned;
    let purchase_vault_liability = self_reward + sponsor_network + pioneer_assigned;

    assert_eq!(purchase_treasury_delta, 35_000_000);
    assert_eq!(purchase_vault_liability, 65_000_000);
    assert_eq!(purchase_treasury_delta + purchase_vault_liability, 100 * UNIT);

    ctx.svm.assert_token_balance(&buyer_usdc, 0);
    ctx.svm.assert_token_balance(&treasury_usdc, 5_000_000 + purchase_treasury_delta);
    ctx.svm.assert_token_balance(&vault_usdc, 5_000_000 + purchase_vault_liability);

    let sponsor_account = ctx.svm.get_account(&sponsor_pda).expect("sponsor state");
    let mut sponsor_data = sponsor_account.data.as_slice();
    let sponsor_state = UserState::try_deserialize(&mut sponsor_data).expect("deserialize sponsor");
    assert_eq!(sponsor_state.self_accrued_usdc, 5 * UNIT);
    assert_eq!(sponsor_state.network_claimable_usdc, sponsor_network);
    assert!(sponsor_state.active_until > 0);

    let buyer_account = ctx.svm.get_account(&buyer_pda).expect("buyer state");
    let mut buyer_data = buyer_account.data.as_slice();
    let buyer_state = UserState::try_deserialize(&mut buyer_data).expect("deserialize buyer");
    assert_eq!(buyer_state.lifetime_service_units, 100);
    assert_eq!(buyer_state.self_accrued_usdc, self_reward);
    assert!(buyer_state.active_until > 0);

    let protocol_account = ctx.svm.get_account(&protocol_pda).expect("protocol state");
    let mut protocol_data = protocol_account.data.as_slice();
    let protocol = ProtocolState::try_deserialize(&mut protocol_data).expect("deserialize protocol");
    assert_eq!(protocol.pioneer_positions_assigned, 0);
    assert_eq!(protocol.lifetime_service_fees_usdc, 500_000 + service_fee as u128);
    assert_eq!(protocol.lifetime_unallocated_usdc, 4_300_000 + network_unallocated as u128);
    assert_eq!(protocol.lifetime_pioneer_unassigned_usdc, 200_000 + pioneer_unassigned as u128);

    let sponsor_claim = 5 * UNIT + sponsor_network;
    let sponsor_claim_ix = ctx
        .program()
        .accounts(service_referral_protocol::accounts::Claim {
            wallet: sponsor.pubkey(),
            protocol: protocol_pda,
            user: sponsor_pda,
            vault_authority,
            vault_token: vault_usdc,
            destination: sponsor_usdc,
            token_program: spl_token::id(),
        })
        .args(service_referral_protocol::instruction::Claim {})
        .instruction()
        .expect("sponsor claim ix");
    ctx.execute_instruction(sponsor_claim_ix, &[&sponsor])
        .expect("sponsor claim tx")
        .assert_success();

    ctx.svm.assert_token_balance(&sponsor_usdc, sponsor_claim);
    ctx.svm.assert_token_balance(&vault_usdc, self_reward);

    let buyer_claim_ix = ctx
        .program()
        .accounts(service_referral_protocol::accounts::Claim {
            wallet: buyer.pubkey(),
            protocol: protocol_pda,
            user: buyer_pda,
            vault_authority,
            vault_token: vault_usdc,
            destination: buyer_usdc,
            token_program: spl_token::id(),
        })
        .args(service_referral_protocol::instruction::Claim {})
        .instruction()
        .expect("buyer claim ix");
    ctx.execute_instruction(buyer_claim_ix, &[&buyer])
        .expect("buyer claim tx")
        .assert_success();

    ctx.svm.assert_token_balance(&buyer_usdc, self_reward);
    ctx.svm.assert_token_balance(&vault_usdc, 0);

    let sponsor_account = ctx.svm.get_account(&sponsor_pda).expect("sponsor after claim");
    let mut sponsor_data = sponsor_account.data.as_slice();
    let sponsor_state = UserState::try_deserialize(&mut sponsor_data).expect("deserialize sponsor after claim");
    assert_eq!(sponsor_state.self_accrued_usdc, 0);
    assert_eq!(sponsor_state.lifetime_claimed_usdc, sponsor_claim as u128);
}
