use anchor_lang::{prelude::*, AccountDeserialize, AnchorDeserialize};
use anchor_litesvm::{AnchorLiteSVM, AssertionHelpers, Program, Signer, TestHelpers};
use revenue_adapter::RevenueReceipt;
use revenue_qualification::RevenueEvidenceV1;
use service_referral_protocol::state::{ProtocolState, UserState};
use solana_transaction::Transaction;

const CORE_BYTES: &[u8] = include_bytes!("../../target/deploy/service_referral_protocol.so");
const ADAPTER_BYTES: &[u8] = include_bytes!("../../target/deploy/revenue_adapter.so");
const QUALIFICATION_BYTES: &[u8] = include_bytes!("../../target/deploy/revenue_qualification.so");
const EVIDENCE_BYTES: &[u8] = include_bytes!("../../target/deploy/test_revenue_evidence_stub.so");
const UNIT: u64 = 1_000_000;

#[test]
fn hardened_evidence_chain_is_collateralized_atomic_and_replay_safe() {
    let mut ctx = AnchorLiteSVM::build_with_programs(&[
        (service_referral_protocol::ID, CORE_BYTES),
        (revenue_adapter::ID, ADAPTER_BYTES),
        (revenue_qualification::ID, QUALIFICATION_BYTES),
        (test_revenue_evidence_stub::ID, EVIDENCE_BYTES),
    ]);
    let mut test_clock = ctx.svm.get_sysvar::<Clock>();
    test_clock.unix_timestamp = 1_700_000_000;
    ctx.svm.set_sysvar(&test_clock);

    let core = Program::new(service_referral_protocol::ID);
    let adapter = Program::new(revenue_adapter::ID);
    let qualification = Program::new(revenue_qualification::ID);
    let evidence = Program::new(test_revenue_evidence_stub::ID);

    let initializer = ctx.svm.create_funded_account(50_000_000_000).expect("initializer");
    let treasury = ctx.svm.create_funded_account(10_000_000_000).expect("treasury");
    let user = ctx.svm.create_funded_account(20_000_000_000).expect("user");
    let usdt_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDT mint");
    let usdc_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDC mint");

    let (protocol_pda, _) = Pubkey::find_program_address(&[b"protocol"], &service_referral_protocol::ID);
    let (vault_authority, _) = Pubkey::find_program_address(&[b"vault-authority"], &service_referral_protocol::ID);
    let (technical_root, _) = Pubkey::find_program_address(&[b"user", Pubkey::default().as_ref()], &service_referral_protocol::ID);
    let (user_pda, _) = Pubkey::find_program_address(&[b"user", user.pubkey().as_ref()], &service_referral_protocol::ID);
    let (batch_pda, _) = Pubkey::find_program_address(&[b"batch", user.pubkey().as_ref(), &0u64.to_le_bytes()], &service_referral_protocol::ID);
    let (adapter_config, _) = Pubkey::find_program_address(&[revenue_adapter::CONFIG_SEED], &revenue_adapter::ID);
    let (revenue_authority, _) = Pubkey::find_program_address(&[revenue_adapter::REVENUE_AUTHORITY_SEED], &revenue_adapter::ID);
    let (qualification_config, _) = Pubkey::find_program_address(&[revenue_qualification::CONFIG_SEED], &revenue_qualification::ID);
    let (verifier_authority, _) = Pubkey::find_program_address(&[revenue_adapter::VERIFIER_AUTHORITY_SEED], &revenue_qualification::ID);
    let (evidence_authority, _) = Pubkey::find_program_address(&[revenue_qualification::EVIDENCE_AUTHORITY_SEED], &test_revenue_evidence_stub::ID);

    let registration_open_at = ctx.svm.get_sysvar::<Clock>().unix_timestamp;
    let core_init = core
        .accounts(service_referral_protocol::accounts::Initialize {
            initializer: initializer.pubkey(), service_treasury: treasury.pubkey(),
            usdt_mint: usdt_mint.pubkey(), usdc_mint: usdc_mint.pubkey(),
            protocol: protocol_pda, vault_authority, technical_root,
            system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::Initialize {
            registration_open_at, qualified_revenue_source: revenue_authority,
        }).instruction().expect("core init ix");
    ctx.execute_instruction(core_init, &[&initializer]).expect("core init").assert_success();

    let adapter_init = adapter
        .accounts(revenue_adapter::accounts::InitializeAdapter {
            initializer: initializer.pubkey(), config: adapter_config,
            revenue_authority, verifier_program: revenue_qualification::ID,
            referral_program: service_referral_protocol::ID,
            system_program: anchor_lang::system_program::ID,
        })
        .args(revenue_adapter::instruction::InitializeAdapter {})
        .instruction().expect("adapter init ix");
    ctx.execute_instruction(adapter_init, &[&initializer]).expect("adapter init").assert_success();

    let qualification_init = qualification
        .accounts(revenue_qualification::accounts::Initialize {
            initializer: initializer.pubkey(), config: qualification_config,
            verifier_authority, evidence_program: test_revenue_evidence_stub::ID,
            evidence_authority, adapter_program: revenue_adapter::ID,
            adapter_config, system_program: anchor_lang::system_program::ID,
        })
        .args(revenue_qualification::instruction::Initialize {
            usdt_mint: usdt_mint.pubkey(), usdc_mint: usdc_mint.pubkey(),
        }).instruction().expect("qualification init ix");
    ctx.execute_instruction(qualification_init, &[&initializer]).expect("qualification init").assert_success();

    let register = core
        .accounts(service_referral_protocol::accounts::Register {
            wallet: user.pubkey(), protocol: protocol_pda,
            referrer_wallet: Pubkey::default(), referrer: technical_root,
            user: user_pda, system_program: anchor_lang::system_program::ID,
        }).args(service_referral_protocol::instruction::Register {})
        .instruction().expect("register ix");
    ctx.execute_instruction(register, &[&user]).expect("register").assert_success();

    let treasury_usdt = ctx.svm.create_associated_token_account(&usdt_mint.pubkey(), &treasury).expect("treasury USDT");
    let treasury_usdc = ctx.svm.create_associated_token_account(&usdc_mint.pubkey(), &treasury).expect("treasury USDC");
    let user_usdc = ctx.svm.create_associated_token_account(&usdc_mint.pubkey(), &user).expect("user USDC");
    let vault_usdt = spl_associated_token_account::get_associated_token_address_with_program_id(&vault_authority, &usdt_mint.pubkey(), &spl_token::id());
    let vault_usdc = spl_associated_token_account::get_associated_token_address_with_program_id(&vault_authority, &usdc_mint.pubkey(), &spl_token::id());
    let revenue_usdc = spl_associated_token_account::get_associated_token_address_with_program_id(&revenue_authority, &usdc_mint.pubkey(), &spl_token::id());

    let create_atas = Transaction::new_signed_with_payer(&[
        spl_associated_token_account::instruction::create_associated_token_account(&initializer.pubkey(), &vault_authority, &usdt_mint.pubkey(), &spl_token::id()),
        spl_associated_token_account::instruction::create_associated_token_account(&initializer.pubkey(), &vault_authority, &usdc_mint.pubkey(), &spl_token::id()),
        spl_associated_token_account::instruction::create_associated_token_account(&initializer.pubkey(), &revenue_authority, &usdc_mint.pubkey(), &spl_token::id()),
    ], Some(&initializer.pubkey()), &[&initializer], ctx.svm.latest_blockhash());
    ctx.svm.send_transaction(create_atas).expect("create ATAs");

    ctx.svm.mint_to(&usdc_mint.pubkey(), &user_usdc, &initializer, 10 * UNIT).expect("fund activation");
    let purchase = core
        .accounts(service_referral_protocol::accounts::PurchaseServiceUnits {
            wallet: user.pubkey(), protocol: protocol_pda, user: user_pda,
            user_source: user_usdc, vault_authority, usdt_vault: vault_usdt,
            usdc_vault: vault_usdc, service_treasury_usdt: treasury_usdt,
            service_treasury_usdc: treasury_usdc, batch: batch_pda,
            token_program: spl_token::id(), system_program: anchor_lang::system_program::ID,
        }).args(service_referral_protocol::instruction::PurchaseServiceUnits { units: 10 })
        .instruction().expect("purchase ix");
    ctx.execute_instruction(purchase, &[&user]).expect("purchase").assert_success();
    ctx.svm.assert_token_balance(&treasury_usdc, 10 * UNIT);
    ctx.svm.assert_token_balance(&vault_usdc, 0);

    let revenue_amount = 100 * UNIT;
    ctx.svm.mint_to(&usdc_mint.pubkey(), &revenue_usdc, &initializer, revenue_amount).expect("fund revenue");
    let event_id = [7u8; 32];
    let reference_hash = [9u8; 32];
    let (evidence_pda, _) = Pubkey::find_program_address(&[revenue_qualification::EVIDENCE_SEED, event_id.as_ref()], &test_revenue_evidence_stub::ID);
    let (receipt_pda, _) = Pubkey::find_program_address(&[revenue_adapter::RECEIPT_SEED, event_id.as_ref()], &revenue_adapter::ID);

    let publish = evidence
        .accounts(test_revenue_evidence_stub::accounts::PublishAndQualify {
            rent_payer: initializer.pubkey(), evidence_authority, evidence: evidence_pda,
            qualification_program: revenue_qualification::ID, qualification_config,
            verifier_authority, adapter_program: revenue_adapter::ID, adapter_config,
            revenue_authority, adapter_receipt: receipt_pda, source_token: revenue_usdc,
            referral_program: service_referral_protocol::ID, protocol: protocol_pda,
            vault_authority, vault_token: vault_usdc, service_treasury_token: treasury_usdc,
            beneficiary: user_pda,
            upline_1: technical_root, upline_2: technical_root, upline_3: technical_root,
            upline_4: technical_root, upline_5: technical_root, upline_6: technical_root,
            upline_7: technical_root, upline_8: technical_root, upline_9: technical_root,
            upline_10: technical_root, token_program: spl_token::id(),
            system_program: anchor_lang::system_program::ID,
        })
        .args(test_revenue_evidence_stub::instruction::PublishAndQualify {
            event_id, evidence_amount: revenue_amount, requested_amount: revenue_amount, reference_hash,
        }).instruction().expect("publish ix");
    ctx.execute_instruction(publish, &[&initializer]).expect("publish").assert_success();

    let direct = 50 * UNIT;
    let network_unallocated = 43 * UNIT;
    let pioneer_pool = 2 * UNIT;
    let service_fee = 5 * UNIT;
    let pioneer_assigned = pioneer_pool / 100;
    let pioneer_unassigned = pioneer_pool - pioneer_assigned;
    let treasury_delta = network_unallocated + service_fee + pioneer_unassigned;
    let user_liability = direct + pioneer_assigned;
    assert_eq!(direct + network_unallocated + pioneer_pool + service_fee, revenue_amount);

    ctx.svm.assert_token_balance(&revenue_usdc, 0);
    ctx.svm.assert_token_balance(&treasury_usdc, 10 * UNIT + treasury_delta);
    ctx.svm.assert_token_balance(&vault_usdc, user_liability);

    let evidence_account = ctx.svm.get_account(&evidence_pda).expect("evidence");
    let mut evidence_data: &[u8] = evidence_account.data.as_slice();
    let stored_evidence = RevenueEvidenceV1::deserialize(&mut evidence_data).expect("evidence deserialize");
    assert_eq!(stored_evidence.event_id, event_id);
    assert_eq!(stored_evidence.amount, revenue_amount);
    assert_eq!(stored_evidence.beneficiary, user_pda);
    assert_eq!(stored_evidence.reference_hash, reference_hash);

    let receipt_account = ctx.svm.get_account(&receipt_pda).expect("receipt");
    let mut receipt_data = receipt_account.data.as_slice();
    let receipt = RevenueReceipt::try_deserialize(&mut receipt_data).expect("receipt deserialize");
    assert_eq!(receipt.event_id, event_id);
    assert_eq!(receipt.evidence_hash, reference_hash);
    assert_eq!(receipt.amount, revenue_amount);
    assert_eq!(receipt.beneficiary, user.pubkey());
    assert_eq!(receipt.mint, usdc_mint.pubkey());

    let user_account = ctx.svm.get_account(&user_pda).expect("user state");
    let mut user_data = user_account.data.as_slice();
    let user_state = UserState::try_deserialize(&mut user_data).expect("user deserialize");
    assert_eq!(user_state.pioneer_id, 1);
    assert_eq!(user_state.direct_accrued_usdc, direct);
    assert_eq!(user_state.network_claimable_usdc, 0);
    assert_eq!(user_state.network_pending_usdc, 0);

    let protocol_account = ctx.svm.get_account(&protocol_pda).expect("protocol state");
    let mut protocol_data = protocol_account.data.as_slice();
    let protocol = ProtocolState::try_deserialize(&mut protocol_data).expect("protocol deserialize");
    assert_eq!(protocol.lifetime_service_fees_usdc, service_fee as u128);
    assert_eq!(protocol.lifetime_unallocated_usdc, network_unallocated as u128);
    assert_eq!(protocol.lifetime_pioneer_unassigned_usdc, pioneer_unassigned as u128);

    ctx.svm.expire_blockhash();
    ctx.svm.mint_to(&usdc_mint.pubkey(), &revenue_usdc, &initializer, revenue_amount).expect("re-fund replay");
    let replay = evidence
        .accounts(test_revenue_evidence_stub::accounts::ReplayExisting {
            rent_payer: initializer.pubkey(), evidence_authority, evidence: evidence_pda,
            qualification_program: revenue_qualification::ID, qualification_config,
            verifier_authority, adapter_program: revenue_adapter::ID, adapter_config,
            revenue_authority, adapter_receipt: receipt_pda, source_token: revenue_usdc,
            referral_program: service_referral_protocol::ID, protocol: protocol_pda,
            vault_authority, vault_token: vault_usdc, service_treasury_token: treasury_usdc,
            beneficiary: user_pda,
            upline_1: technical_root, upline_2: technical_root, upline_3: technical_root,
            upline_4: technical_root, upline_5: technical_root, upline_6: technical_root,
            upline_7: technical_root, upline_8: technical_root, upline_9: technical_root,
            upline_10: technical_root, token_program: spl_token::id(),
            system_program: anchor_lang::system_program::ID,
        }).args(test_revenue_evidence_stub::instruction::ReplayExisting { event_id, amount: revenue_amount })
        .instruction().expect("replay ix");
    let replay_result = ctx.execute_instruction(replay, &[&initializer]).expect("replay tx");
    assert!(!replay_result.is_success(), "duplicate event must fail");
    ctx.svm.assert_token_balance(&revenue_usdc, revenue_amount);
    ctx.svm.assert_token_balance(&treasury_usdc, 10 * UNIT + treasury_delta);
    ctx.svm.assert_token_balance(&vault_usdc, user_liability);

    ctx.svm.expire_blockhash();
    let mismatch_event = [8u8; 32];
    let (mismatch_evidence, _) = Pubkey::find_program_address(&[revenue_qualification::EVIDENCE_SEED, mismatch_event.as_ref()], &test_revenue_evidence_stub::ID);
    let (mismatch_receipt, _) = Pubkey::find_program_address(&[revenue_adapter::RECEIPT_SEED, mismatch_event.as_ref()], &revenue_adapter::ID);
    let mismatch = evidence
        .accounts(test_revenue_evidence_stub::accounts::PublishAndQualify {
            rent_payer: initializer.pubkey(), evidence_authority, evidence: mismatch_evidence,
            qualification_program: revenue_qualification::ID, qualification_config,
            verifier_authority, adapter_program: revenue_adapter::ID, adapter_config,
            revenue_authority, adapter_receipt: mismatch_receipt, source_token: revenue_usdc,
            referral_program: service_referral_protocol::ID, protocol: protocol_pda,
            vault_authority, vault_token: vault_usdc, service_treasury_token: treasury_usdc,
            beneficiary: user_pda,
            upline_1: technical_root, upline_2: technical_root, upline_3: technical_root,
            upline_4: technical_root, upline_5: technical_root, upline_6: technical_root,
            upline_7: technical_root, upline_8: technical_root, upline_9: technical_root,
            upline_10: technical_root, token_program: spl_token::id(),
            system_program: anchor_lang::system_program::ID,
        }).args(test_revenue_evidence_stub::instruction::PublishAndQualify {
            event_id: mismatch_event, evidence_amount: revenue_amount,
            requested_amount: revenue_amount - UNIT, reference_hash: [10u8; 32],
        }).instruction().expect("mismatch ix");
    let mismatch_result = ctx.execute_instruction(mismatch, &[&initializer]).expect("mismatch tx");
    assert!(!mismatch_result.is_success(), "evidence/request mismatch must fail");
    assert!(ctx.svm.get_account(&mismatch_evidence).is_none());
    assert!(ctx.svm.get_account(&mismatch_receipt).is_none());
    ctx.svm.assert_token_balance(&revenue_usdc, revenue_amount);
    ctx.svm.assert_token_balance(&treasury_usdc, 10 * UNIT + treasury_delta);
    ctx.svm.assert_token_balance(&vault_usdc, user_liability);
}
