use anchor_lang::{prelude::*, AccountDeserialize, InstructionData, ToAccountMetas};
use anchor_litesvm::{AnchorLiteSVM, AssertionHelpers, TestHelpers};
use revenue_adapter::RevenueReceipt;
use solana_instruction::Instruction;
use solana_signer::Signer;
use solana_transaction::Transaction;

const CORE_BYTES: &[u8] = include_bytes!("../artifacts/service_referral_protocol.dev.so");
const ADAPTER_BYTES: &[u8] = include_bytes!("../artifacts/revenue_adapter.dev.so");
const VERIFIER_BYTES: &[u8] = include_bytes!("../artifacts/test_revenue_verifier.so");
const UNIT: u64 = 1_000_000;

fn anchor_ix<A: ToAccountMetas, D: InstructionData>(
    program_id: Pubkey,
    accounts: A,
    args: D,
) -> Instruction {
    Instruction {
        program_id,
        accounts: accounts.to_account_metas(None),
        data: args.data(),
    }
}

fn adapter_accounts(
    relayer: Pubkey,
    config: Pubkey,
    verifier_authority: Pubkey,
    revenue_authority: Pubkey,
    source_token: Pubkey,
    receipt: Pubkey,
    protocol: Pubkey,
    vault_authority: Pubkey,
    vault_token: Pubkey,
    treasury_token: Pubkey,
    beneficiary: Pubkey,
    technical_root: Pubkey,
) -> revenue_adapter::accounts::ForwardQualifiedRevenue {
    revenue_adapter::accounts::ForwardQualifiedRevenue {
        relayer,
        config,
        verifier_authority,
        revenue_authority,
        source_token,
        receipt,
        protocol,
        vault_authority,
        vault_token,
        service_treasury_token: treasury_token,
        beneficiary,
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
        referral_program: service_referral_protocol::ID,
        token_program: spl_token::id(),
        system_program: anchor_lang::system_program::ID,
    }
}

fn verifier_accounts(
    relayer: Pubkey,
    config: Pubkey,
    verifier_authority: Pubkey,
    revenue_authority: Pubkey,
    source_token: Pubkey,
    receipt: Pubkey,
    protocol: Pubkey,
    vault_authority: Pubkey,
    vault_token: Pubkey,
    treasury_token: Pubkey,
    beneficiary: Pubkey,
    technical_root: Pubkey,
) -> test_revenue_verifier::accounts::SubmitForTest {
    test_revenue_verifier::accounts::SubmitForTest {
        relayer,
        config,
        verifier_authority,
        revenue_authority,
        source_token,
        receipt,
        protocol,
        vault_authority,
        vault_token,
        service_treasury_token: treasury_token,
        beneficiary,
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
        adapter_program: revenue_adapter::ID,
        referral_program: service_referral_protocol::ID,
        token_program: spl_token::id(),
        system_program: anchor_lang::system_program::ID,
    }
}

#[test]
fn verifier_pda_authorization_replay_and_downstream_rollback_hold_end_to_end() {
    let programs: &[(Pubkey, &[u8])] = &[
        (service_referral_protocol::ID, CORE_BYTES),
        (revenue_adapter::ID, ADAPTER_BYTES),
        (test_revenue_verifier::ID, VERIFIER_BYTES),
    ];
    let mut ctx = AnchorLiteSVM::build_with_programs(programs);

    let initializer = ctx.svm.create_funded_account(40_000_000_000).expect("initializer");
    let treasury = ctx.svm.create_funded_account(10_000_000_000).expect("treasury");
    let user = ctx.svm.create_funded_account(10_000_000_000).expect("user");
    let relayer = ctx.svm.create_funded_account(10_000_000_000).expect("relayer");
    let attacker = ctx.svm.create_funded_account(10_000_000_000).expect("attacker");

    let usdt_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDT mint");
    let usdc_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDC mint");

    let (protocol_pda, _) = Pubkey::find_program_address(&[b"protocol"], &service_referral_protocol::ID);
    let (vault_authority, _) = Pubkey::find_program_address(&[b"vault-authority"], &service_referral_protocol::ID);
    let (technical_root, _) = Pubkey::find_program_address(
        &[b"user", Pubkey::default().as_ref()],
        &service_referral_protocol::ID,
    );
    let (user_pda, _) = Pubkey::find_program_address(
        &[b"user", user.pubkey().as_ref()],
        &service_referral_protocol::ID,
    );
    let (batch_pda, _) = Pubkey::find_program_address(
        &[b"batch", user.pubkey().as_ref(), &0u64.to_le_bytes()],
        &service_referral_protocol::ID,
    );
    let (config_pda, _) = Pubkey::find_program_address(
        &[revenue_adapter::CONFIG_SEED],
        &revenue_adapter::ID,
    );
    let (revenue_authority, _) = Pubkey::find_program_address(
        &[revenue_adapter::REVENUE_AUTHORITY_SEED],
        &revenue_adapter::ID,
    );
    let (verifier_authority, _) = Pubkey::find_program_address(
        &[test_revenue_verifier::VERIFIER_AUTHORITY_SEED],
        &test_revenue_verifier::ID,
    );

    let registration_open_at = ctx.svm.get_sysvar::<Clock>().unix_timestamp;
    let initialize_core = ctx
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
            qualified_revenue_source: revenue_authority,
        })
        .instruction()
        .expect("core initialize ix");
    ctx.execute_instruction(initialize_core, &[&initializer])
        .expect("core initialize tx")
        .assert_success();

    let initialize_adapter = anchor_ix(
        revenue_adapter::ID,
        revenue_adapter::accounts::InitializeAdapter {
            initializer: initializer.pubkey(),
            config: config_pda,
            revenue_authority,
            verifier_program: test_revenue_verifier::ID,
            referral_program: service_referral_protocol::ID,
            system_program: anchor_lang::system_program::ID,
        },
        revenue_adapter::instruction::InitializeAdapter {},
    );
    ctx.execute_instruction(initialize_adapter, &[&initializer])
        .expect("adapter initialize tx")
        .assert_success();

    let register = ctx
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
    ctx.execute_instruction(register, &[&user])
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
    let revenue_usdc = spl_associated_token_account::get_associated_token_address_with_program_id(
        &revenue_authority,
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
    let create_revenue_usdc = spl_associated_token_account::instruction::create_associated_token_account(
        &initializer.pubkey(),
        &revenue_authority,
        &usdc_mint.pubkey(),
        &spl_token::id(),
    );
    let create_atas = Transaction::new_signed_with_payer(
        &[create_vault_usdt, create_vault_usdc, create_revenue_usdc],
        Some(&initializer.pubkey()),
        &[&initializer],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(create_atas).expect("create PDA ATAs");

    ctx.svm
        .mint_to(&usdc_mint.pubkey(), &user_usdc, &initializer, 10 * UNIT)
        .expect("mint activation units");
    ctx.svm
        .mint_to(&usdc_mint.pubkey(), &revenue_usdc, &initializer, 100 * UNIT)
        .expect("prefund qualified revenue");

    let purchase = ctx
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
    ctx.execute_instruction(purchase, &[&user])
        .expect("purchase tx")
        .assert_success();

    // A direct transaction cannot make the verifier PDA a signer. Force the
    // account meta to non-signer so the transaction is valid and Anchor itself
    // proves the authorization check fails inside the adapter.
    let unauthorized_event = [1u8; 32];
    let unauthorized_evidence = [11u8; 32];
    let (unauthorized_receipt, _) = Pubkey::find_program_address(
        &[revenue_adapter::RECEIPT_SEED, unauthorized_event.as_ref()],
        &revenue_adapter::ID,
    );
    let mut unauthorized_ix = anchor_ix(
        revenue_adapter::ID,
        adapter_accounts(
            relayer.pubkey(),
            config_pda,
            verifier_authority,
            revenue_authority,
            revenue_usdc,
            unauthorized_receipt,
            protocol_pda,
            vault_authority,
            vault_usdc,
            treasury_usdc,
            user_pda,
            technical_root,
        ),
        revenue_adapter::instruction::ForwardQualifiedRevenue {
            event_id: unauthorized_event,
            evidence_hash: unauthorized_evidence,
            amount: 100 * UNIT,
        },
    );
    let verifier_meta = unauthorized_ix
        .accounts
        .iter_mut()
        .find(|meta| meta.pubkey == verifier_authority)
        .expect("verifier authority meta");
    verifier_meta.is_signer = false;

    let unauthorized = ctx
        .execute_instruction(unauthorized_ix, &[&relayer])
        .expect("execute unauthorized adapter tx");
    assert!(!unauthorized.is_success(), "direct adapter call must fail without verifier PDA signature");
    assert!(ctx.svm.get_account(&unauthorized_receipt).is_none());
    ctx.svm.assert_token_balance(&revenue_usdc, 100 * UNIT);
    ctx.svm.assert_token_balance(&vault_usdc, 0);
    ctx.svm.assert_token_balance(&treasury_usdc, 10 * UNIT);

    // A normal wallet may be a valid transaction signer, but it still cannot
    // substitute itself for the verifier program's deterministic authority PDA.
    let wrong_signer_event = [4u8; 32];
    let wrong_signer_evidence = [44u8; 32];
    let (wrong_signer_receipt, _) = Pubkey::find_program_address(
        &[revenue_adapter::RECEIPT_SEED, wrong_signer_event.as_ref()],
        &revenue_adapter::ID,
    );
    let wrong_signer_ix = anchor_ix(
        revenue_adapter::ID,
        adapter_accounts(
            relayer.pubkey(),
            config_pda,
            attacker.pubkey(),
            revenue_authority,
            revenue_usdc,
            wrong_signer_receipt,
            protocol_pda,
            vault_authority,
            vault_usdc,
            treasury_usdc,
            user_pda,
            technical_root,
        ),
        revenue_adapter::instruction::ForwardQualifiedRevenue {
            event_id: wrong_signer_event,
            evidence_hash: wrong_signer_evidence,
            amount: 100 * UNIT,
        },
    );
    let wrong_signer = ctx
        .execute_instruction(wrong_signer_ix, &[&relayer, &attacker])
        .expect("execute wrong-signer adapter tx");
    assert!(!wrong_signer.is_success(), "ordinary signer must not substitute verifier PDA");
    assert!(ctx.svm.get_account(&wrong_signer_receipt).is_none());
    ctx.svm.assert_token_balance(&revenue_usdc, 100 * UNIT);
    ctx.svm.assert_token_balance(&vault_usdc, 0);
    ctx.svm.assert_token_balance(&treasury_usdc, 10 * UNIT);

    // The mock verifier can sign only its deterministic PDA during CPI. This
    // proves the intended verifier -> adapter -> referral chain end-to-end.
    let event_id = [2u8; 32];
    let evidence_hash = [22u8; 32];
    let (receipt_pda, _) = Pubkey::find_program_address(
        &[revenue_adapter::RECEIPT_SEED, event_id.as_ref()],
        &revenue_adapter::ID,
    );
    let submit = anchor_ix(
        test_revenue_verifier::ID,
        verifier_accounts(
            relayer.pubkey(),
            config_pda,
            verifier_authority,
            revenue_authority,
            revenue_usdc,
            receipt_pda,
            protocol_pda,
            vault_authority,
            vault_usdc,
            treasury_usdc,
            user_pda,
            technical_root,
        ),
        test_revenue_verifier::instruction::SubmitForTest {
            event_id,
            evidence_hash,
            amount: 100 * UNIT,
        },
    );
    ctx.execute_instruction(submit.clone(), &[&relayer])
        .expect("execute verifier CPI")
        .assert_success();

    let direct = 50 * UNIT;
    let network_unallocated = 43 * UNIT;
    let pioneer_pool = 2 * UNIT;
    let service_fee = 5 * UNIT;
    let pioneer_assigned = pioneer_pool / 100;
    let pioneer_unassigned = pioneer_pool - pioneer_assigned;
    let treasury_delta = network_unallocated + service_fee + pioneer_unassigned;
    let user_liability = direct + pioneer_assigned;

    ctx.svm.assert_token_balance(&revenue_usdc, 0);
    ctx.svm.assert_token_balance(&treasury_usdc, 10 * UNIT + treasury_delta);
    ctx.svm.assert_token_balance(&vault_usdc, user_liability);

    let receipt_account = ctx.svm.get_account(&receipt_pda).expect("receipt exists");
    let mut receipt_data = receipt_account.data.as_slice();
    let receipt = RevenueReceipt::try_deserialize(&mut receipt_data).expect("deserialize receipt");
    assert_eq!(receipt.event_id, event_id);
    assert_eq!(receipt.evidence_hash, evidence_hash);
    assert_eq!(receipt.beneficiary, user.pubkey());
    assert_eq!(receipt.mint, usdc_mint.pubkey());
    assert_eq!(receipt.amount, 100 * UNIT);

    // LiteSVM deduplicates identical transactions under the same recent
    // blockhash. Advance it before issuing the same mint_to shape again.
    ctx.svm.expire_blockhash();
    ctx.svm
        .mint_to(&usdc_mint.pubkey(), &revenue_usdc, &initializer, 100 * UNIT)
        .expect("refund source for replay test");
    let treasury_before_replay = 10 * UNIT + treasury_delta;
    let vault_before_replay = user_liability;
    let replay = ctx
        .execute_instruction(submit, &[&relayer])
        .expect("execute replay tx");
    assert!(!replay.is_success(), "same event ID must be rejected");
    ctx.svm.assert_token_balance(&revenue_usdc, 100 * UNIT);
    ctx.svm.assert_token_balance(&treasury_usdc, treasury_before_replay);
    ctx.svm.assert_token_balance(&vault_usdc, vault_before_replay);

    // Force a downstream ancestry failure after the adapter has entered its
    // handler. Solana atomicity must roll back both the pre-created receipt and
    // the core token transfer/accounting changes.
    let rollback_event = [3u8; 32];
    let rollback_evidence = [33u8; 32];
    let (rollback_receipt, _) = Pubkey::find_program_address(
        &[revenue_adapter::RECEIPT_SEED, rollback_event.as_ref()],
        &revenue_adapter::ID,
    );
    let mut invalid_accounts = verifier_accounts(
        relayer.pubkey(),
        config_pda,
        verifier_authority,
        revenue_authority,
        revenue_usdc,
        rollback_receipt,
        protocol_pda,
        vault_authority,
        vault_usdc,
        treasury_usdc,
        user_pda,
        technical_root,
    );
    invalid_accounts.upline_1 = user_pda;
    let rollback_ix = anchor_ix(
        test_revenue_verifier::ID,
        invalid_accounts,
        test_revenue_verifier::instruction::SubmitForTest {
            event_id: rollback_event,
            evidence_hash: rollback_evidence,
            amount: 10 * UNIT,
        },
    );
    let rollback = ctx
        .execute_instruction(rollback_ix, &[&relayer])
        .expect("execute rollback case");
    assert!(!rollback.is_success(), "invalid downstream ancestry must fail");
    assert!(ctx.svm.get_account(&rollback_receipt).is_none(), "failed CPI must not consume receipt");
    ctx.svm.assert_token_balance(&revenue_usdc, 100 * UNIT);
    ctx.svm.assert_token_balance(&treasury_usdc, treasury_before_replay);
    ctx.svm.assert_token_balance(&vault_usdc, vault_before_replay);
}
