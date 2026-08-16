use anchor_lang::{prelude::*, AccountDeserialize};
use anchor_litesvm::{AnchorContext, AnchorLiteSVM, AssertionHelpers, TestHelpers};
use service_referral_protocol::{constants::PIONEER_SCALE, state::{ProtocolState, UserState}, ID};
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
fn pioneer_positions_are_single_purchase_only_weighted_rule_b_and_hard_capped() {
    let mut ctx = AnchorLiteSVM::build_with_program(ID, PROGRAM_BYTES);
    let initializer = ctx.svm.create_funded_account(80_000_000_000).expect("initializer");
    let treasury = ctx.svm.create_funded_account(20_000_000_000).expect("treasury");
    let a = ctx.svm.create_funded_account(10_000_000_000).expect("a");
    let b = ctx.svm.create_funded_account(10_000_000_000).expect("b");
    let c = ctx.svm.create_funded_account(10_000_000_000).expect("c");
    let usdt_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDT");
    let usdc_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDC");
    let (protocol, _) = Pubkey::find_program_address(&[b"protocol"], &ID);
    let (vault_authority, _) = Pubkey::find_program_address(&[b"vault-authority"], &ID);
    let (root, _) = Pubkey::find_program_address(&[b"user", Pubkey::default().as_ref()], &ID);
    let a_pda = Pubkey::find_program_address(&[b"user", a.pubkey().as_ref()], &ID).0;
    let b_pda = Pubkey::find_program_address(&[b"user", b.pubkey().as_ref()], &ID).0;
    let c_pda = Pubkey::find_program_address(&[b"user", c.pubkey().as_ref()], &ID).0;
    let registration_open_at = ctx.svm.get_sysvar::<Clock>().unix_timestamp;
    let init = ctx.program().accounts(service_referral_protocol::accounts::Initialize {
        initializer: initializer.pubkey(), service_treasury: treasury.pubkey(),
        usdt_mint: usdt_mint.pubkey(), usdc_mint: usdc_mint.pubkey(), protocol,
        vault_authority, technical_root: root, system_program: anchor_lang::system_program::ID,
    }).args(service_referral_protocol::instruction::Initialize { registration_open_at })
      .instruction().expect("init");
    ctx.execute_instruction(init, &[&initializer]).expect("init tx").assert_success();

    macro_rules! register {
        ($wallet:expr, $pda:expr) => {{
            ctx.svm.expire_blockhash();
            let ix = ctx.program().accounts(service_referral_protocol::accounts::Register {
                wallet: $wallet.pubkey(), protocol, referrer_wallet: Pubkey::default(),
                referrer: root, user: $pda, system_program: anchor_lang::system_program::ID,
            }).args(service_referral_protocol::instruction::Register {}).instruction().expect("register");
            ctx.execute_instruction(ix, &[&$wallet]).expect("register tx").assert_success();
        }};
    }
    register!(a, a_pda); register!(b, b_pda); register!(c, c_pda);
    assert_eq!(read_protocol(&ctx, protocol).pioneer_positions_assigned, 0, "registration must consume zero positions");
    assert_eq!(read_user(&ctx, a_pda).pioneer_positions, 0);

    let treasury_usdt = ctx.svm.create_associated_token_account(&usdt_mint.pubkey(), &treasury).expect("treasury USDT");
    let treasury_usdc = ctx.svm.create_associated_token_account(&usdc_mint.pubkey(), &treasury).expect("treasury USDC");
    let a_src = ctx.svm.create_associated_token_account(&usdc_mint.pubkey(), &a).expect("a ATA");
    let b_src = ctx.svm.create_associated_token_account(&usdc_mint.pubkey(), &b).expect("b ATA");
    let c_src = ctx.svm.create_associated_token_account(&usdc_mint.pubkey(), &c).expect("c ATA");
    let vault_usdt = spl_associated_token_account::get_associated_token_address_with_program_id(&vault_authority, &usdt_mint.pubkey(), &spl_token::id());
    let vault_usdc = spl_associated_token_account::get_associated_token_address_with_program_id(&vault_authority, &usdc_mint.pubkey(), &spl_token::id());
    let create_vaults = Transaction::new_signed_with_payer(&[
        spl_associated_token_account::instruction::create_associated_token_account(&initializer.pubkey(), &vault_authority, &usdt_mint.pubkey(), &spl_token::id()),
        spl_associated_token_account::instruction::create_associated_token_account(&initializer.pubkey(), &vault_authority, &usdc_mint.pubkey(), &spl_token::id()),
    ], Some(&initializer.pubkey()), &[&initializer], ctx.svm.latest_blockhash());
    ctx.svm.send_transaction(create_vaults).expect("vaults");
    ctx.svm.mint_to(&usdc_mint.pubkey(), &a_src, &initializer, 5_000 * UNIT).expect("fund a");
    ctx.svm.mint_to(&usdc_mint.pubkey(), &b_src, &initializer, 94_000 * UNIT).expect("fund b");
    ctx.svm.mint_to(&usdc_mint.pubkey(), &c_src, &initializer, 8_000 * UNIT).expect("fund c");

    macro_rules! buy_root {
        ($wallet:expr, $pda:expr, $src:expr, $units:expr) => {{
            ctx.svm.expire_blockhash();
            let ix = ctx.program().accounts(service_referral_protocol::accounts::PurchaseAndDistribute {
                wallet: $wallet.pubkey(), protocol, user: $pda, user_source: $src,
                vault_authority, usdt_vault: vault_usdt, usdc_vault: vault_usdc,
                service_treasury_usdt: treasury_usdt, service_treasury_usdc: treasury_usdc,
                direct_referrer: root, upline_1: root, upline_2: root, upline_3: root,
                upline_4: root, upline_5: root, upline_6: root, upline_7: root, upline_8: root,
                token_program: spl_token::id(),
            }).args(service_referral_protocol::instruction::PurchaseAndDistribute { units: $units })
              .instruction().expect("purchase");
            ctx.execute_instruction(ix, &[&$wallet]).expect("purchase tx").assert_success();
        }};
    }

    // Two separate 500 purchases never combine into one Pioneer position.
    buy_root!(a, a_pda, a_src, 500);
    buy_root!(a, a_pda, a_src, 500);
    assert_eq!(read_user(&ctx, a_pda).pioneer_positions, 0);
    assert_eq!(read_protocol(&ctx, protocol).pioneer_positions_assigned, 0);

    // A single 1,000 purchase creates exactly one position, but Rule B means that
    // position has zero due from the transaction that created it.
    buy_root!(a, a_pda, a_src, 1_000);
    let a_after_one = read_user(&ctx, a_pda);
    let p_after_one = read_protocol(&ctx, protocol);
    assert_eq!(a_after_one.pioneer_positions, 1);
    assert_eq!(p_after_one.pioneer_positions_assigned, 1);
    let due_after_creation = ((a_after_one.pioneer_positions as u128) * p_after_one.pioneer_index_usdc
        - a_after_one.pioneer_checkpoint_usdc) / PIONEER_SCALE;
    assert_eq!(due_after_creation, 0, "new position must not earn its creating purchase");

    // Same wallet can earn three more positions in one 3,000 purchase. Only its old
    // one position earns this purchase: 60 USDC pool / 100 = 0.6 USDC.
    buy_root!(a, a_pda, a_src, 3_000);
    let a_after_four = read_user(&ctx, a_pda);
    let p_after_four = read_protocol(&ctx, protocol);
    assert_eq!(a_after_four.pioneer_positions, 4);
    assert_eq!(p_after_four.pioneer_positions_assigned, 4);
    let due = ((a_after_four.pioneer_positions as u128) * p_after_four.pioneer_index_usdc
        - a_after_four.pioneer_checkpoint_usdc) / PIONEER_SCALE;
    assert_eq!(due, 600_000u128, "only the pre-existing position earns the 3,000 purchase");

    // Fill to exactly 98 positions with one independent 94,000 purchase.
    buy_root!(b, b_pda, b_src, 94_000);
    assert_eq!(read_user(&ctx, b_pda).pioneer_positions, 94);
    assert_eq!(read_protocol(&ctx, protocol).pioneer_positions_assigned, 98);

    // Only two remain. A 3,000 purchase requests three but receives exactly two.
    buy_root!(c, c_pda, c_src, 3_000);
    assert_eq!(read_user(&ctx, c_pda).pioneer_positions, 2);
    assert_eq!(read_protocol(&ctx, protocol).pioneer_positions_assigned, 100);

    // Permanent saturation: even another qualifying 5,000 purchase creates zero.
    buy_root!(c, c_pda, c_src, 5_000);
    assert_eq!(read_user(&ctx, c_pda).pioneer_positions, 2);
    assert_eq!(read_protocol(&ctx, protocol).pioneer_positions_assigned, 100);
}
