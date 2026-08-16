use anchor_lang::{prelude::*, AccountDeserialize};
use anchor_litesvm::{AnchorContext, AnchorLiteSVM, AssertionHelpers, TestHelpers};
use service_referral_protocol::{state::{ProtocolState, UserState}, ID};
use solana_signer::Signer;
use solana_transaction::Transaction;

const PROGRAM_BYTES: &[u8] = include_bytes!("../../target/deploy/service_referral_protocol.so");
const UNIT: u64 = 1_000_000;

fn read_user(ctx: &AnchorContext, pda: Pubkey) -> UserState {
    let account = ctx.svm.get_account(&pda).expect("user state");
    let mut data = account.data.as_slice();
    UserState::try_deserialize(&mut data).expect("deserialize user")
}

fn read_protocol(ctx: &AnchorContext, pda: Pubkey) -> ProtocolState {
    let account = ctx.svm.get_account(&pda).expect("protocol state");
    let mut data = account.data.as_slice();
    ProtocolState::try_deserialize(&mut data).expect("deserialize protocol")
}

#[test]
fn inactive_sponsor_rewards_are_treasury_destined_and_cannot_be_claimed() {
    let mut ctx = AnchorLiteSVM::build_with_program(ID, PROGRAM_BYTES);
    let initializer = ctx.svm.create_funded_account(40_000_000_000).expect("initializer");
    let treasury = ctx.svm.create_funded_account(10_000_000_000).expect("treasury");
    let sponsor = ctx.svm.create_funded_account(10_000_000_000).expect("sponsor");
    let buyer = ctx.svm.create_funded_account(10_000_000_000).expect("buyer");

    let usdt_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDT mint");
    let usdc_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDC mint");

    let (protocol, _) = Pubkey::find_program_address(&[b"protocol"], &ID);
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
            protocol,
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

    let register_sponsor = ctx
        .program()
        .accounts(service_referral_protocol::accounts::Register {
            wallet: sponsor.pubkey(),
            protocol,
            referrer_wallet: Pubkey::default(),
            referrer: technical_root,
            user: sponsor_pda,
            system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::Register {})
        .instruction()
        .expect("register sponsor");
    ctx.execute_instruction(register_sponsor, &[&sponsor])
        .expect("register sponsor tx")
        .assert_success();

    let register_buyer = ctx
        .program()
        .accounts(service_referral_protocol::accounts::Register {
            wallet: buyer.pubkey(),
            protocol,
            referrer_wallet: sponsor.pubkey(),
            referrer: sponsor_pda,
            user: buyer_pda,
            system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::Register {})
        .instruction()
        .expect("register buyer");
    ctx.execute_instruction(register_buyer, &[&buyer])
        .expect("register buyer tx")
        .assert_success();

    let treasury_usdt = ctx.svm.create_associated_token_account(&usdt_mint.pubkey(), &treasury).expect("treasury USDT");
    let treasury_usdc = ctx.svm.create_associated_token_account(&usdc_mint.pubkey(), &treasury).expect("treasury USDC");
    let sponsor_usdc = ctx.svm.create_associated_token_account(&usdc_mint.pubkey(), &sponsor).expect("sponsor USDC");
    let buyer_usdc = ctx.svm.create_associated_token_account(&usdc_mint.pubkey(), &buyer).expect("buyer USDC");

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

    // Sponsor never buys 10 units and is INACTIVE. Buyer buys 100 units, becomes
    // ACTIVE, receives SELF 50%, while sponsor's U1 15% plus its current Pioneer
    // entitlement are treasury-destined immediately under IC-A.
    let purchase_ix = ctx
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
        .expect("purchase ix");
    ctx.execute_instruction(purchase_ix, &[&buyer])
        .expect("purchase tx")
        .assert_success();

    let sponsor_after_purchase = read_user(&ctx, sponsor_pda);
    let buyer_after_purchase = read_user(&ctx, buyer_pda);
    let protocol_after_purchase = read_protocol(&ctx, protocol);
    assert_eq!(sponsor_after_purchase.active_until, 0);
    assert_eq!(sponsor_after_purchase.network_claimable_usdc, 0);
    assert_eq!(sponsor_after_purchase.lifetime_expired_usdc, 15_020_000u128);
    assert_eq!(buyer_after_purchase.self_accrued_usdc, 50 * UNIT);
    assert!(buyer_after_purchase.active_until > 0);
    assert_eq!(protocol_after_purchase.lifetime_expired_usdc, 15_020_000u128);

    // 15.020 expired sponsor/Pioneer + 28 unallocated upper network + 5 service
    // + 1.960 unassigned Pioneer = 49.980 treasury. Buyer SELF + buyer Pioneer
    // remain collateralized in the vault.
    ctx.svm.assert_token_balance(&buyer_usdc, 0);
    ctx.svm.assert_token_balance(&treasury_usdc, 49_980_000);
    ctx.svm.assert_token_balance(&vault_usdc, 50_020_000);
    ctx.svm.assert_token_balance(&sponsor_usdc, 0);

    let claim_ix = ctx
        .program()
        .accounts(service_referral_protocol::accounts::Claim {
            wallet: sponsor.pubkey(),
            protocol,
            user: sponsor_pda,
            vault_authority,
            vault_token: vault_usdc,
            destination: sponsor_usdc,
            token_program: spl_token::id(),
        })
        .args(service_referral_protocol::instruction::Claim {})
        .instruction()
        .expect("claim ix");
    let claim_outcome = ctx
        .execute_instruction(claim_ix, &[&sponsor])
        .expect("inactive claim program result");
    assert!(!claim_outcome.is_success(), "inactive sponsor must not claim");

    // The purchase already settled every whole-atomic sponsor entitlement, so a
    // second permissionless settlement must fail rather than double-transfer value.
    ctx.svm.expire_blockhash();
    let settle_ix = ctx
        .program()
        .accounts(service_referral_protocol::accounts::SettleExpired {
            settler: buyer.pubkey(),
            protocol,
            user: sponsor_pda,
            vault_authority,
            vault_token: vault_usdc,
            service_treasury_token: treasury_usdc,
            token_program: spl_token::id(),
        })
        .args(service_referral_protocol::instruction::SettleExpired {})
        .instruction()
        .expect("settle ix");
    let settle_outcome = ctx
        .execute_instruction(settle_ix, &[&buyer])
        .expect("settle result");
    assert!(!settle_outcome.is_success(), "expired sponsor value must not settle twice");
    ctx.svm.assert_token_balance(&treasury_usdc, 49_980_000);
    ctx.svm.assert_token_balance(&vault_usdc, 50_020_000);

    let buyer_claim_ix = ctx
        .program()
        .accounts(service_referral_protocol::accounts::Claim {
            wallet: buyer.pubkey(),
            protocol,
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

    ctx.svm.assert_token_balance(&buyer_usdc, 50_020_000);
    ctx.svm.assert_token_balance(&treasury_usdc, 49_980_000);
    ctx.svm.assert_token_balance(&vault_usdc, 0);
    assert_eq!(50_020_000u64 + 49_980_000u64, 100 * UNIT);

}
