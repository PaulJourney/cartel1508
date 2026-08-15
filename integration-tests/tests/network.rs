use anchor_lang::{prelude::*, AccountDeserialize};
use anchor_litesvm::{AnchorLiteSVM, AssertionHelpers, TestHelpers};
use service_referral_protocol::{state::{ProtocolState, UserState}, ID};
use solana_signer::Signer;
use solana_transaction::Transaction;

const PROGRAM_BYTES: &[u8] = include_bytes!("../../target/deploy/service_referral_protocol.so");
const UNIT: u64 = 1_000_000;
const LEVELS: [u64; 10] = [
    15_000_000, 9_000_000, 6_000_000, 4_000_000, 2_500_000,
    2_000_000, 1_500_000, 1_000_000, 1_000_000, 1_000_000,
];

#[test]
fn ten_active_uplines_receive_exact_frozen_curve_with_zero_network_unallocated() {
    let mut ctx = AnchorLiteSVM::build_with_program(ID, PROGRAM_BYTES);
    let initializer = ctx.svm.create_funded_account(80_000_000_000).expect("initializer");
    let treasury = ctx.svm.create_funded_account(10_000_000_000).expect("treasury");
    let revenue_source = ctx.svm.create_funded_account(10_000_000_000).expect("revenue source");
    let beneficiary = ctx.svm.create_funded_account(10_000_000_000).expect("beneficiary");
    let mut uplines = Vec::with_capacity(10);
    for _ in 0..10 {
        uplines.push(ctx.svm.create_funded_account(10_000_000_000).expect("upline"));
    }

    let usdt_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDT mint");
    let usdc_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDC mint");

    let (protocol_pda, _) = Pubkey::find_program_address(&[b"protocol"], &ID);
    let (vault_authority, _) = Pubkey::find_program_address(&[b"vault-authority"], &ID);
    let (technical_root, _) = Pubkey::find_program_address(&[b"user", Pubkey::default().as_ref()], &ID);

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
        .instruction().expect("initialize ix");
    ctx.execute_instruction(initialize_ix, &[&initializer]).expect("initialize").assert_success();

    // uplines[0] is L1 (nearest beneficiary), uplines[9] is L10 (nearest root).
    // Register from L10 downward so every immutable referrer already exists.
    let mut upline_pdas = [Pubkey::default(); 10];
    for idx in (0..10).rev() {
        let wallet_key = uplines[idx].pubkey();
        let (user_pda, _) = Pubkey::find_program_address(&[b"user", wallet_key.as_ref()], &ID);
        upline_pdas[idx] = user_pda;
        let (referrer_wallet, referrer_pda) = if idx == 9 {
            (Pubkey::default(), technical_root)
        } else {
            (uplines[idx + 1].pubkey(), upline_pdas[idx + 1])
        };
        let register_ix = ctx
            .program()
            .accounts(service_referral_protocol::accounts::Register {
                wallet: wallet_key,
                protocol: protocol_pda,
                referrer_wallet,
                referrer: referrer_pda,
                user: user_pda,
                system_program: anchor_lang::system_program::ID,
            })
            .args(service_referral_protocol::instruction::Register {})
            .instruction().expect("register upline ix");
        ctx.execute_instruction(register_ix, &[&uplines[idx]])
            .expect("register upline").assert_success();
    }

    let (beneficiary_pda, _) = Pubkey::find_program_address(&[b"user", beneficiary.pubkey().as_ref()], &ID);
    let register_beneficiary_ix = ctx
        .program()
        .accounts(service_referral_protocol::accounts::Register {
            wallet: beneficiary.pubkey(),
            protocol: protocol_pda,
            referrer_wallet: uplines[0].pubkey(),
            referrer: upline_pdas[0],
            user: beneficiary_pda,
            system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::Register {})
        .instruction().expect("register beneficiary ix");
    ctx.execute_instruction(register_beneficiary_ix, &[&beneficiary])
        .expect("register beneficiary").assert_success();

    let treasury_usdt = ctx.svm.create_associated_token_account(&usdt_mint.pubkey(), &treasury).expect("treasury USDT ATA");
    let treasury_usdc = ctx.svm.create_associated_token_account(&usdc_mint.pubkey(), &treasury).expect("treasury USDC ATA");
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
        Some(&initializer.pubkey()), &[&initializer], ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(create_vaults_tx).expect("create vault ATAs");

    let mut upline_usdc = [Pubkey::default(); 10];
    for idx in 0..10 {
        let ata = ctx.svm.create_associated_token_account(&usdc_mint.pubkey(), &uplines[idx]).expect("upline USDC ATA");
        upline_usdc[idx] = ata;
        ctx.svm.mint_to(&usdc_mint.pubkey(), &ata, &initializer, 10 * UNIT).expect("mint activation funds");
        let (batch_pda, _) = Pubkey::find_program_address(
            &[b"batch", uplines[idx].pubkey().as_ref(), &0u64.to_le_bytes()], &ID,
        );
        let purchase_ix = ctx
            .program()
            .accounts(service_referral_protocol::accounts::PurchaseServiceUnits {
                wallet: uplines[idx].pubkey(), protocol: protocol_pda, user: upline_pdas[idx],
                user_source: ata, vault_authority, usdt_vault: vault_usdt,
                usdc_vault: vault_usdc, service_treasury_usdt: treasury_usdt,
                service_treasury_usdc: treasury_usdc, batch: batch_pda,
                token_program: spl_token::id(), system_program: anchor_lang::system_program::ID,
            })
            .args(service_referral_protocol::instruction::PurchaseServiceUnits { units: 10 })
            .instruction().expect("activate upline ix");
        ctx.execute_instruction(purchase_ix, &[&uplines[idx]])
            .expect("activate upline").assert_success();
    }

    ctx.svm.assert_token_balance(&treasury_usdc, 100 * UNIT);
    for ata in &upline_usdc {
        ctx.svm.assert_token_balance(ata, 0);
    }

    ctx.svm.mint_to(&usdc_mint.pubkey(), &revenue_usdc, &initializer, 100 * UNIT).expect("mint revenue funds");
    let record_ix = ctx
        .program()
        .accounts(service_referral_protocol::accounts::RecordQualifiedRevenue {
            revenue_source: revenue_source.pubkey(), protocol: protocol_pda,
            source_token: revenue_usdc, vault_authority, vault_token: vault_usdc,
            service_treasury_token: treasury_usdc, beneficiary: beneficiary_pda,
            upline_1: upline_pdas[0], upline_2: upline_pdas[1], upline_3: upline_pdas[2],
            upline_4: upline_pdas[3], upline_5: upline_pdas[4], upline_6: upline_pdas[5],
            upline_7: upline_pdas[6], upline_8: upline_pdas[7], upline_9: upline_pdas[8],
            upline_10: upline_pdas[9], token_program: spl_token::id(),
        })
        .args(service_referral_protocol::instruction::RecordQualifiedRevenue { amount: 100 * UNIT })
        .instruction().expect("record qualified revenue ix");
    ctx.execute_instruction(record_ix, &[&revenue_source])
        .expect("record qualified revenue").assert_success();

    assert_eq!(LEVELS.iter().copied().sum::<u64>(), 43 * UNIT);
    for idx in 0..10 {
        let account = ctx.svm.get_account(&upline_pdas[idx]).expect("upline state after revenue");
        let mut data = account.data.as_slice();
        let state = UserState::try_deserialize(&mut data).expect("deserialize upline state");
        assert_eq!(state.network_claimable_usdc, LEVELS[idx], "wrong level allocation at L{}", idx + 1);
        assert_eq!(state.network_pending_usdc, 0, "unexpected pending at L{}", idx + 1);
        assert_eq!(state.lifetime_expired_usdc, 0, "unexpected expiry at L{}", idx + 1);
    }

    let beneficiary_account = ctx.svm.get_account(&beneficiary_pda).expect("beneficiary state");
    let mut beneficiary_data = beneficiary_account.data.as_slice();
    let beneficiary_state = UserState::try_deserialize(&mut beneficiary_data).expect("deserialize beneficiary");
    assert_eq!(beneficiary_state.direct_accrued_usdc, 50 * UNIT);
    assert_eq!(beneficiary_state.network_claimable_usdc, 0);
    assert_eq!(beneficiary_state.pioneer_id, 11);

    let protocol_account = ctx.svm.get_account(&protocol_pda).expect("protocol state");
    let mut protocol_data = protocol_account.data.as_slice();
    let protocol = ProtocolState::try_deserialize(&mut protocol_data).expect("deserialize protocol");
    assert_eq!(protocol.real_user_count, 11);
    assert_eq!(protocol.pioneer_count, 11);
    assert_eq!(protocol.lifetime_service_fees_usdc, 5 * UNIT as u128);
    assert_eq!(protocol.lifetime_unallocated_usdc, 0);
    assert_eq!(protocol.lifetime_pioneer_unassigned_usdc, 1_780_000);
    assert_eq!(protocol.lifetime_expired_usdc, 0);

    // 100 qualified revenue = 50 direct + 43 network + 2 pioneer + 5 service.
    // With 11 Pioneers, 0.22 remains Pioneer liability and 1.78 is unassigned treasury flow.
    let expected_vault_liability = 50 * UNIT + 43 * UNIT + 220_000;
    let expected_revenue_treasury = 5 * UNIT + 1_780_000;
    assert_eq!(expected_vault_liability + expected_revenue_treasury, 100 * UNIT);
    ctx.svm.assert_token_balance(&vault_usdc, expected_vault_liability);
    ctx.svm.assert_token_balance(&treasury_usdc, 100 * UNIT + expected_revenue_treasury);
}
