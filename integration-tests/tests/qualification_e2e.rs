use anchor_lang::{prelude::*, AccountDeserialize};
use anchor_litesvm::{AnchorLiteSVM, AssertionHelpers, TestHelpers};
use revenue_adapter::{RevenueReceipt, CONFIG_SEED, RECEIPT_SEED, REVENUE_AUTHORITY_SEED};
use service_referral_protocol::state::UserState;
use solana_signer::Signer;
use solana_transaction::Transaction;

const REFERRAL_BYTES: &[u8] = include_bytes!("../../target/deploy/service_referral_protocol.so");
const ADAPTER_BYTES: &[u8] = include_bytes!("../../target/deploy/revenue_adapter.so");
const QUALIFICATION_BYTES: &[u8] = include_bytes!("../../target/deploy/revenue_qualification.so");
const UNIT: u64 = 1_000_000;

fn derived_event_id(
    payer: Pubkey,
    beneficiary: Pubkey,
    mint: Pubkey,
    amount: u64,
    nonce: [u8; 32],
) -> [u8; 32] {
    let amount_bytes = amount.to_le_bytes();
    Pubkey::find_program_address(
        &[
            revenue_qualification::EVENT_DOMAIN,
            payer.as_ref(),
            beneficiary.as_ref(),
            mint.as_ref(),
            &amount_bytes,
            &nonce,
        ],
        &revenue_qualification::ID,
    )
    .0
    .to_bytes()
}

#[test]
fn real_payment_is_atomic_replay_safe_and_rolls_back_on_downstream_failure() {
    let mut ctx = AnchorLiteSVM::build_with_programs(&[
        (service_referral_protocol::ID, REFERRAL_BYTES),
        (revenue_adapter::ID, ADAPTER_BYTES),
        (revenue_qualification::ID, QUALIFICATION_BYTES),
    ]);

    let initializer = ctx
        .svm
        .create_funded_account(40_000_000_000)
        .expect("initializer");
    let treasury = ctx
        .svm
        .create_funded_account(10_000_000_000)
        .expect("treasury");
    let user = ctx
        .svm
        .create_funded_account(10_000_000_000)
        .expect("registered beneficiary");
    let customer = ctx
        .svm
        .create_funded_account(10_000_000_000)
        .expect("external customer payer");

    let usdt_mint = ctx
        .svm
        .create_token_mint(&initializer, 6)
        .expect("USDT mint");
    let usdc_mint = ctx
        .svm
        .create_token_mint(&initializer, 6)
        .expect("USDC mint");

    let (protocol_pda, _) = Pubkey::find_program_address(
        &[b"protocol"],
        &service_referral_protocol::ID,
    );
    let (vault_authority, _) = Pubkey::find_program_address(
        &[b"vault-authority"],
        &service_referral_protocol::ID,
    );
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

    let (adapter_config, _) = Pubkey::find_program_address(&[CONFIG_SEED], &revenue_adapter::ID);
    let (revenue_authority, _) =
        Pubkey::find_program_address(&[REVENUE_AUTHORITY_SEED], &revenue_adapter::ID);
    let (qualification_authority, _) = Pubkey::find_program_address(
        &[revenue_qualification::QUALIFIER_AUTHORITY_SEED],
        &revenue_qualification::ID,
    );

    // The referral protocol trusts only the adapter-owned PDA as its qualified revenue source.
    ctx.program_id = service_referral_protocol::ID;
    let registration_open_at = ctx.svm.get_sysvar::<Clock>().unix_timestamp;
    let initialize_protocol_ix = ctx
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
        .expect("build protocol initialize");
    ctx.execute_instruction(initialize_protocol_ix, &[&initializer])
        .expect("execute protocol initialize")
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
        .expect("build register");
    ctx.execute_instruction(register_ix, &[&user])
        .expect("execute register")
        .assert_success();

    // Freeze the adapter to the exact referral + qualification program identities.
    ctx.program_id = revenue_adapter::ID;
    let initialize_adapter_ix = ctx
        .program()
        .accounts(revenue_adapter::accounts::Initialize {
            initializer: initializer.pubkey(),
            config: adapter_config,
            revenue_authority,
            qualification_authority,
            system_program: anchor_lang::system_program::ID,
        })
        .args(revenue_adapter::instruction::Initialize {
            referral_program: service_referral_protocol::ID,
            qualification_program: revenue_qualification::ID,
            usdt_mint: usdt_mint.pubkey(),
            usdc_mint: usdc_mint.pubkey(),
        })
        .instruction()
        .expect("build adapter initialize");
    ctx.execute_instruction(initialize_adapter_ix, &[&initializer])
        .expect("execute adapter initialize")
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
    let customer_usdc = ctx
        .svm
        .create_associated_token_account(&usdc_mint.pubkey(), &customer)
        .expect("customer USDC ATA");

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
    let create_revenue_usdc =
        spl_associated_token_account::instruction::create_associated_token_account(
            &initializer.pubkey(),
            &revenue_authority,
            &usdc_mint.pubkey(),
            &spl_token::id(),
        );
    let create_protocol_atas = Transaction::new_signed_with_payer(
        &[create_vault_usdt, create_vault_usdc, create_revenue_usdc],
        Some(&initializer.pubkey()),
        &[&initializer],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm
        .send_transaction(create_protocol_atas)
        .expect("create PDA-owned ATAs");

    // Activate the beneficiary with the required 10 service units.
    ctx.svm
        .mint_to(&usdc_mint.pubkey(), &user_usdc, &initializer, 10 * UNIT)
        .expect("mint activation funds");
    ctx.svm
        .mint_to(&usdc_mint.pubkey(), &customer_usdc, &initializer, 300 * UNIT)
        .expect("mint external customer payment funds");

    ctx.program_id = service_referral_protocol::ID;
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
        .expect("build activation purchase");
    ctx.execute_instruction(purchase_ix, &[&user])
        .expect("execute activation purchase")
        .assert_success();

    let revenue_amount = 100 * UNIT;
    let direct = 50 * UNIT;
    let network_unallocated = 43 * UNIT;
    let pioneer_pool = 2 * UNIT;
    let service_fee = 5 * UNIT;
    let pioneer_assigned = pioneer_pool / 100;
    let pioneer_unassigned = pioneer_pool - pioneer_assigned;
    let treasury_delta = network_unallocated + service_fee + pioneer_unassigned;
    let user_liability = direct + pioneer_assigned;

    ctx.svm.assert_token_balance(&customer_usdc, 300 * UNIT);
    ctx.svm.assert_token_balance(&revenue_usdc, 0);
    ctx.svm.assert_token_balance(&vault_usdc, 0);
    ctx.svm.assert_token_balance(&treasury_usdc, 10 * UNIT);

    ctx.program_id = revenue_qualification::ID;
    let nonce_1 = [1u8; 32];
    let event_1 = derived_event_id(
        customer.pubkey(),
        user_pda,
        usdc_mint.pubkey(),
        revenue_amount,
        nonce_1,
    );
    let receipt_1 = Pubkey::find_program_address(
        &[RECEIPT_SEED, event_1.as_ref()],
        &revenue_adapter::ID,
    )
    .0;

    let success_ix = ctx
        .program()
        .accounts(revenue_qualification::accounts::QualifyPaymentAndRoute {
            payer: customer.pubkey(),
            payer_source_token: customer_usdc,
            qualification_authority,
            adapter_config,
            revenue_authority,
            revenue_source_token: revenue_usdc,
            adapter_receipt: receipt_1,
            adapter_program: revenue_adapter::ID,
            referral_program: service_referral_protocol::ID,
            protocol: protocol_pda,
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
            system_program: anchor_lang::system_program::ID,
        })
        .args(revenue_qualification::instruction::QualifyPaymentAndRoute {
            client_nonce: nonce_1,
            amount: revenue_amount,
        })
        .instruction()
        .expect("build qualification success");
    ctx.execute_instruction(success_ix, &[&customer])
        .expect("execute qualification success")
        .assert_success();

    // Real customer value was consumed exactly once and the adapter source ATA is drained
    // atomically into the referral vault/treasury accounting.
    ctx.svm.assert_token_balance(&customer_usdc, 200 * UNIT);
    ctx.svm.assert_token_balance(&revenue_usdc, 0);
    ctx.svm.assert_token_balance(&vault_usdc, user_liability);
    ctx.svm
        .assert_token_balance(&treasury_usdc, 10 * UNIT + treasury_delta);

    let receipt_account = ctx.svm.get_account(&receipt_1).expect("receipt exists");
    let mut receipt_data = receipt_account.data.as_slice();
    let receipt = RevenueReceipt::try_deserialize(&mut receipt_data).expect("deserialize receipt");
    assert_eq!(receipt.event_id, event_1);
    assert_eq!(receipt.beneficiary, user_pda);
    assert_eq!(receipt.mint, usdc_mint.pubkey());
    assert_eq!(receipt.amount, revenue_amount);
    assert_eq!(receipt.qualifier, qualification_authority);

    let user_account = ctx.svm.get_account(&user_pda).expect("beneficiary state");
    let mut user_data = user_account.data.as_slice();
    let user_state = UserState::try_deserialize(&mut user_data).expect("deserialize beneficiary");
    assert_eq!(user_state.direct_accrued_usdc, direct);

    // Replay the exact same economic event. The adapter receipt already exists, so the CPI
    // must fail. The outer token transfer must roll back: customer and protocol balances
    // remain byte-for-byte economically unchanged.
    ctx.svm.expire_blockhash();
    let replay_ix = ctx
        .program()
        .accounts(revenue_qualification::accounts::QualifyPaymentAndRoute {
            payer: customer.pubkey(),
            payer_source_token: customer_usdc,
            qualification_authority,
            adapter_config,
            revenue_authority,
            revenue_source_token: revenue_usdc,
            adapter_receipt: receipt_1,
            adapter_program: revenue_adapter::ID,
            referral_program: service_referral_protocol::ID,
            protocol: protocol_pda,
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
            system_program: anchor_lang::system_program::ID,
        })
        .args(revenue_qualification::instruction::QualifyPaymentAndRoute {
            client_nonce: nonce_1,
            amount: revenue_amount,
        })
        .instruction()
        .expect("build replay");
    let replay = ctx
        .execute_instruction(replay_ix, &[&customer])
        .expect("execute replay");
    assert!(!replay.is_success(), "same event receipt must reject replay");
    ctx.svm.assert_token_balance(&customer_usdc, 200 * UNIT);
    ctx.svm.assert_token_balance(&revenue_usdc, 0);
    ctx.svm.assert_token_balance(&vault_usdc, user_liability);
    ctx.svm
        .assert_token_balance(&treasury_usdc, 10 * UNIT + treasury_delta);

    // A different event reaches the adapter, but an invalid ancestry account makes the
    // downstream referral CPI fail. Both the preceding customer transfer and newly-created
    // adapter receipt must disappear through transaction rollback.
    ctx.svm.expire_blockhash();
    let nonce_2 = [2u8; 32];
    let event_2 = derived_event_id(
        customer.pubkey(),
        user_pda,
        usdc_mint.pubkey(),
        revenue_amount,
        nonce_2,
    );
    let receipt_2 = Pubkey::find_program_address(
        &[RECEIPT_SEED, event_2.as_ref()],
        &revenue_adapter::ID,
    )
    .0;
    assert!(ctx.svm.get_account(&receipt_2).is_none());

    let downstream_failure_ix = ctx
        .program()
        .accounts(revenue_qualification::accounts::QualifyPaymentAndRoute {
            payer: customer.pubkey(),
            payer_source_token: customer_usdc,
            qualification_authority,
            adapter_config,
            revenue_authority,
            revenue_source_token: revenue_usdc,
            adapter_receipt: receipt_2,
            adapter_program: revenue_adapter::ID,
            referral_program: service_referral_protocol::ID,
            protocol: protocol_pda,
            vault_authority,
            vault_token: vault_usdc,
            service_treasury_token: treasury_usdc,
            beneficiary: user_pda,
            upline_1: user_pda, // deliberately wrong: expected technical root
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
            system_program: anchor_lang::system_program::ID,
        })
        .args(revenue_qualification::instruction::QualifyPaymentAndRoute {
            client_nonce: nonce_2,
            amount: revenue_amount,
        })
        .instruction()
        .expect("build downstream failure");
    let downstream_failure = ctx
        .execute_instruction(downstream_failure_ix, &[&customer])
        .expect("execute downstream failure");
    assert!(
        !downstream_failure.is_success(),
        "invalid ancestry must reject the routed revenue"
    );

    ctx.svm.assert_token_balance(&customer_usdc, 200 * UNIT);
    ctx.svm.assert_token_balance(&revenue_usdc, 0);
    ctx.svm.assert_token_balance(&vault_usdc, user_liability);
    ctx.svm
        .assert_token_balance(&treasury_usdc, 10 * UNIT + treasury_delta);
    assert!(
        ctx.svm.get_account(&receipt_2).is_none(),
        "failed downstream CPI must roll back receipt creation"
    );
}
