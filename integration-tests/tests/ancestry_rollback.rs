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

fn read_user(ctx: &AnchorLiteSVM, pda: Pubkey) -> UserState {
    let account = ctx.svm.get_account(&pda).expect("user state");
    let mut data = account.data.as_slice();
    UserState::try_deserialize(&mut data).expect("deserialize user")
}

fn read_protocol(ctx: &AnchorLiteSVM, pda: Pubkey) -> ProtocolState {
    let account = ctx.svm.get_account(&pda).expect("protocol state");
    let mut data = account.data.as_slice();
    ProtocolState::try_deserialize(&mut data).expect("deserialize protocol")
}

#[test]
fn false_upline_cannot_move_tokens_create_units_or_leave_partial_rewards() {
    let mut ctx = AnchorLiteSVM::build_with_program(ID, PROGRAM_BYTES);
    let initializer = ctx
        .svm
        .create_funded_account(40_000_000_000)
        .expect("initializer");
    let treasury = ctx
        .svm
        .create_funded_account(10_000_000_000)
        .expect("treasury");
    let l2 = ctx.svm.create_funded_account(10_000_000_000).expect("L2");
    let l1 = ctx.svm.create_funded_account(10_000_000_000).expect("L1");
    let buyer = ctx
        .svm
        .create_funded_account(10_000_000_000)
        .expect("buyer");

    let usdt_mint = ctx
        .svm
        .create_token_mint(&initializer, 6)
        .expect("USDT mint");
    let usdc_mint = ctx
        .svm
        .create_token_mint(&initializer, 6)
        .expect("USDC mint");

    let (protocol, _) = Pubkey::find_program_address(&[b"protocol"], &ID);
    let (vault_authority, _) = Pubkey::find_program_address(&[b"vault-authority"], &ID);
    let (technical_root, _) =
        Pubkey::find_program_address(&[b"user", Pubkey::default().as_ref()], &ID);
    let (l2_pda, _) = Pubkey::find_program_address(&[b"user", l2.pubkey().as_ref()], &ID);
    let (l1_pda, _) = Pubkey::find_program_address(&[b"user", l1.pubkey().as_ref()], &ID);
    let (buyer_pda, _) = Pubkey::find_program_address(&[b"user", buyer.pubkey().as_ref()], &ID);
    let (buyer_batch, _) = Pubkey::find_program_address(
        &[b"batch", buyer.pubkey().as_ref(), &0u64.to_le_bytes()],
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
            protocol,
            vault_authority,
            technical_root,
            system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::Initialize {
            registration_open_at,
        })
        .instruction()
        .expect("initialize ix");
    ctx.execute_instruction(initialize_ix, &[&initializer])
        .expect("initialize tx")
        .assert_success();

    let register_l2 = ctx
        .program()
        .accounts(service_referral_protocol::accounts::Register {
            wallet: l2.pubkey(),
            protocol,
            referrer_wallet: Pubkey::default(),
            referrer: technical_root,
            user: l2_pda,
            system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::Register {})
        .instruction()
        .expect("register L2");
    ctx.execute_instruction(register_l2, &[&l2])
        .expect("register L2 tx")
        .assert_success();

    let register_l1 = ctx
        .program()
        .accounts(service_referral_protocol::accounts::Register {
            wallet: l1.pubkey(),
            protocol,
            referrer_wallet: l2.pubkey(),
            referrer: l2_pda,
            user: l1_pda,
            system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::Register {})
        .instruction()
        .expect("register L1");
    ctx.execute_instruction(register_l1, &[&l1])
        .expect("register L1 tx")
        .assert_success();

    let register_buyer = ctx
        .program()
        .accounts(service_referral_protocol::accounts::Register {
            wallet: buyer.pubkey(),
            protocol,
            referrer_wallet: l1.pubkey(),
            referrer: l1_pda,
            user: buyer_pda,
            system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::Register {})
        .instruction()
        .expect("register buyer");
    ctx.execute_instruction(register_buyer, &[&buyer])
        .expect("register buyer tx")
        .assert_success();

    let treasury_usdt = ctx
        .svm
        .create_associated_token_account(&usdt_mint.pubkey(), &treasury)
        .expect("treasury USDT");
    let treasury_usdc = ctx
        .svm
        .create_associated_token_account(&usdc_mint.pubkey(), &treasury)
        .expect("treasury USDC");
    let buyer_usdc = ctx
        .svm
        .create_associated_token_account(&usdc_mint.pubkey(), &buyer)
        .expect("buyer USDC");

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
    let create_vaults = Transaction::new_signed_with_payer(
        &[create_vault_usdt, create_vault_usdc],
        Some(&initializer.pubkey()),
        &[&initializer],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(create_vaults).expect("create vaults");

    ctx.svm
        .mint_to(&usdc_mint.pubkey(), &buyer_usdc, &initializer, 100 * UNIT)
        .expect("fund buyer");

    let buyer_before = read_user(&ctx, buyer_pda);
    let l1_before = read_user(&ctx, l1_pda);
    let protocol_before = read_protocol(&ctx, protocol);

    // The buyer's immutable ancestry is buyer -> L1 -> L2. We deliberately pass
    // technical_root as the first network account instead of the canonical L2 PDA.
    // The handler has already transferred payment and allocated a batch by the time
    // it reaches dynamic ancestry validation, so Solana atomic rollback is essential.
    let invalid_purchase = ctx
        .program()
        .accounts(service_referral_protocol::accounts::PurchaseAndDistribute {
            wallet: buyer.pubkey(),
            protocol,
            user: buyer_pda,
            user_source: buyer_usdc,
            vault_authority,
            usdt_vault: vault_usdt,
            usdc_vault: vault_usdc,
            service_treasury_usdt: treasury_usdt,
            service_treasury_usdc: treasury_usdc,
            direct_referrer: l1_pda,
            upline_1: technical_root, // WRONG: must be l2_pda.
            upline_2: technical_root,
            upline_3: technical_root,
            upline_4: technical_root,
            upline_5: technical_root,
            upline_6: technical_root,
            upline_7: technical_root,
            upline_8: technical_root,
            upline_9: technical_root,
            batch: buyer_batch,
            token_program: spl_token::id(),
            system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::PurchaseAndDistribute { units: 100 })
        .instruction()
        .expect("invalid purchase ix");

    let outcome = ctx
        .execute_instruction(invalid_purchase, &[&buyer])
        .expect("invalid purchase program result");
    assert!(
        !outcome.is_success(),
        "substituting a false upline must fail"
    );

    ctx.svm.assert_token_balance(&buyer_usdc, 100 * UNIT);
    ctx.svm.assert_token_balance(&vault_usdc, 0);
    ctx.svm.assert_token_balance(&treasury_usdc, 0);
    assert!(
        ctx.svm.get_account(&buyer_batch).is_none(),
        "failed purchase must not leave a UnitBatch"
    );

    let buyer_after = read_user(&ctx, buyer_pda);
    let l1_after = read_user(&ctx, l1_pda);
    let protocol_after = read_protocol(&ctx, protocol);

    assert_eq!(
        buyer_after.lifetime_service_units,
        buyer_before.lifetime_service_units
    );
    assert_eq!(buyer_after.next_batch_index, buyer_before.next_batch_index);
    assert_eq!(buyer_after.active_until, buyer_before.active_until);
    assert_eq!(
        l1_after.direct_accrued_usdc,
        l1_before.direct_accrued_usdc
    );
    assert_eq!(protocol_after.next_unit_id, protocol_before.next_unit_id);
    assert_eq!(
        protocol_after.pioneer_index_usdc,
        protocol_before.pioneer_index_usdc
    );
}
