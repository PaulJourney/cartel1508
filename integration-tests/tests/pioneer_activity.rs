use anchor_lang::{prelude::*, AccountDeserialize};
use anchor_litesvm::{AnchorContext, AnchorLiteSVM, AssertionHelpers, TestHelpers};
use service_referral_protocol::{
    constants::PIONEER_SCALE,
    state::{ProtocolState, UserState},
    ID,
};
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

fn pioneer_due(user: &UserState, protocol: &ProtocolState) -> u128 {
    ((user.pioneer_positions as u128) * protocol.pioneer_index_usdc
        - user.pioneer_checkpoint_usdc)
        / PIONEER_SCALE
}

#[test]
fn pioneer_position_is_permanent_but_inactive_due_expires_before_reactivation() {
    let mut ctx = AnchorLiteSVM::build_with_program(ID, PROGRAM_BYTES);
    let initializer = ctx
        .svm
        .create_funded_account(50_000_000_000)
        .expect("initializer");
    let treasury = ctx
        .svm
        .create_funded_account(10_000_000_000)
        .expect("treasury");
    let pioneer = ctx
        .svm
        .create_funded_account(10_000_000_000)
        .expect("pioneer");
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
    let (root, _) = Pubkey::find_program_address(&[b"user", Pubkey::default().as_ref()], &ID);
    let pioneer_pda =
        Pubkey::find_program_address(&[b"user", pioneer.pubkey().as_ref()], &ID).0;
    let buyer_pda = Pubkey::find_program_address(&[b"user", buyer.pubkey().as_ref()], &ID).0;

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
            technical_root: root,
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

    macro_rules! register_root {
        ($wallet:expr, $pda:expr) => {{
            ctx.svm.expire_blockhash();
            let ix = ctx
                .program()
                .accounts(service_referral_protocol::accounts::Register {
                    wallet: $wallet.pubkey(),
                    protocol,
                    referrer_wallet: Pubkey::default(),
                    referrer: root,
                    user: $pda,
                    system_program: anchor_lang::system_program::ID,
                })
                .args(service_referral_protocol::instruction::Register {})
                .instruction()
                .expect("register ix");
            ctx.execute_instruction(ix, &[&$wallet])
                .expect("register tx")
                .assert_success();
        }};
    }
    register_root!(pioneer, pioneer_pda);
    register_root!(buyer, buyer_pda);

    let treasury_usdt = ctx
        .svm
        .create_associated_token_account(&usdt_mint.pubkey(), &treasury)
        .expect("treasury USDT");
    let treasury_usdc = ctx
        .svm
        .create_associated_token_account(&usdc_mint.pubkey(), &treasury)
        .expect("treasury USDC");
    let pioneer_src = ctx
        .svm
        .create_associated_token_account(&usdc_mint.pubkey(), &pioneer)
        .expect("pioneer ATA");
    let buyer_src = ctx
        .svm
        .create_associated_token_account(&usdc_mint.pubkey(), &buyer)
        .expect("buyer ATA");

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
    let create_vaults = Transaction::new_signed_with_payer(
        &[
            spl_associated_token_account::instruction::create_associated_token_account(
                &initializer.pubkey(),
                &vault_authority,
                &usdt_mint.pubkey(),
                &spl_token::id(),
            ),
            spl_associated_token_account::instruction::create_associated_token_account(
                &initializer.pubkey(),
                &vault_authority,
                &usdc_mint.pubkey(),
                &spl_token::id(),
            ),
        ],
        Some(&initializer.pubkey()),
        &[&initializer],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm
        .send_transaction(create_vaults)
        .expect("create vaults");

    ctx.svm
        .mint_to(
            &usdc_mint.pubkey(),
            &pioneer_src,
            &initializer,
            1_010 * UNIT,
        )
        .expect("fund pioneer");
    ctx.svm
        .mint_to(&usdc_mint.pubkey(), &buyer_src, &initializer, 2_000 * UNIT)
        .expect("fund buyer");

    macro_rules! buy_root {
        ($wallet:expr, $pda:expr, $src:expr, $units:expr) => {{
            ctx.svm.expire_blockhash();
            let ix = ctx
                .program()
                .accounts(service_referral_protocol::accounts::PurchaseAndDistribute {
                    wallet: $wallet.pubkey(),
                    protocol,
                    user: $pda,
                    user_source: $src,
                    vault_authority,
                    usdt_vault: vault_usdt,
                    usdc_vault: vault_usdc,
                    service_treasury_usdt: treasury_usdt,
                    service_treasury_usdc: treasury_usdc,
                    direct_referrer: root,
                    upline_1: root,
                    upline_2: root,
                    upline_3: root,
                    upline_4: root,
                    upline_5: root,
                    upline_6: root,
                    upline_7: root,
                    upline_8: root,
                    token_program: spl_token::id(),
                })
                .args(service_referral_protocol::instruction::PurchaseAndDistribute {
                    units: $units,
                })
                .instruction()
                .expect("purchase ix");
            ctx.execute_instruction(ix, &[&$wallet])
                .expect("purchase tx")
                .assert_success();
        }};
    }

    // Pioneer acquires one permanent position. Rule B excludes the creating purchase.
    buy_root!(pioneer, pioneer_pda, pioneer_src, 1_000);
    let after_creation = read_user(&ctx, pioneer_pda);
    assert_eq!(after_creation.pioneer_positions, 1);
    assert_eq!(pioneer_due(&after_creation, &read_protocol(&ctx, protocol)), 0);

    // A later global purchase while Pioneer is ACTIVE creates exactly 0.2 USDC due
    // for its one position: 2% of 1,000 / 100 = 0.2.
    buy_root!(buyer, buyer_pda, buyer_src, 1_000);
    let active_pioneer = read_user(&ctx, pioneer_pda);
    assert_eq!(
        pioneer_due(&active_pioneer, &read_protocol(&ctx, protocol)),
        200_000
    );

    // Let Pioneer pass both ACTIVE and GRACE. Inactivity does not recycle the slot.
    let mut clock: Clock = ctx.svm.get_sysvar();
    clock.unix_timestamp = active_pioneer.grace_until + 1;
    ctx.svm.set_sysvar(&clock);
    assert_eq!(read_user(&ctx, pioneer_pda).pioneer_positions, 1);

    // Another global purchase occurs while Pioneer is INACTIVE. Its fixed virtual
    // Pioneer share still advances in the global index, but it must be destined for
    // Treasury rather than become recoverable by a later reactivation.
    buy_root!(buyer, buyer_pda, buyer_src, 1_000);
    let inactive_before_settle = read_user(&ctx, pioneer_pda);
    let protocol_before_settle = read_protocol(&ctx, protocol);
    assert_eq!(inactive_before_settle.pioneer_positions, 1);
    assert_eq!(
        pioneer_due(&inactive_before_settle, &protocol_before_settle),
        400_000,
        "0.2 active-era + 0.2 inactive-era Pioneer due is currently unsettled"
    );

    // Permissionless expiry checkpoints all unclaimed Pioneer due to Treasury. The
    // position itself survives permanently, but no historical Pioneer amount remains.
    ctx.svm.expire_blockhash();
    let settle_ix = ctx
        .program()
        .accounts(service_referral_protocol::accounts::SettleExpired {
            settler: buyer.pubkey(),
            protocol,
            user: pioneer_pda,
            vault_authority,
            vault_token: vault_usdc,
            service_treasury_token: treasury_usdc,
            token_program: spl_token::id(),
        })
        .args(service_referral_protocol::instruction::SettleExpired {})
        .instruction()
        .expect("settle ix");
    ctx.execute_instruction(settle_ix, &[&buyer])
        .expect("settle tx")
        .assert_success();

    let settled = read_user(&ctx, pioneer_pda);
    let protocol_after_settle = read_protocol(&ctx, protocol);
    assert_eq!(settled.pioneer_positions, 1, "Pioneer position never recycles");
    assert_eq!(pioneer_due(&settled, &protocol_after_settle), 0);
    assert!(settled.lifetime_expired_usdc >= 400_000u128);

    // Reactivation starts week 2 (still 10 units). Historical inactive-era value is
    // not recovered; only the reactivation purchase itself accrues fresh Pioneer due
    // because the wallet becomes ACTIVE before the current purchase's Pioneer accrual.
    buy_root!(pioneer, pioneer_pda, pioneer_src, 10);
    let reactivated = read_user(&ctx, pioneer_pda);
    let protocol_after_reactivation = read_protocol(&ctx, protocol);
    assert_eq!(reactivated.active_weeks_started, 2);
    assert_eq!(reactivated.pioneer_positions, 1);
    assert_eq!(
        pioneer_due(&reactivated, &protocol_after_reactivation),
        2_000,
        "fresh ACTIVE purchase earns 1/100 of its 2% Pioneer pool"
    );
}
