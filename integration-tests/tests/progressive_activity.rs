use anchor_lang::{prelude::*, AccountDeserialize};
use anchor_litesvm::{AnchorContext, AnchorLiteSVM, AssertionHelpers, TestHelpers};
use service_referral_protocol::{math::next_active_requirement, state::UserState, ID};
use solana_signer::Signer;
use solana_transaction::Transaction;

const PROGRAM_BYTES: &[u8] = include_bytes!("../../target/deploy/service_referral_protocol.so");
const UNIT: u64 = 1_000_000;

fn read_user(ctx: &AnchorContext, pda: Pubkey) -> UserState {
    let account = ctx.svm.get_account(&pda).expect("user state");
    let mut data = account.data.as_slice();
    UserState::try_deserialize(&mut data).expect("deserialize user")
}

#[test]
fn active_week_requirement_progresses_only_on_successful_activation_and_caps_at_fifty() {
    let mut ctx = AnchorLiteSVM::build_with_program(ID, PROGRAM_BYTES);
    let initializer = ctx
        .svm
        .create_funded_account(50_000_000_000)
        .expect("initializer");
    let treasury = ctx
        .svm
        .create_funded_account(10_000_000_000)
        .expect("treasury");
    let user = ctx
        .svm
        .create_funded_account(10_000_000_000)
        .expect("user");

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
    let (user_pda, _) = Pubkey::find_program_address(&[b"user", user.pubkey().as_ref()], &ID);

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

    let register_ix = ctx
        .program()
        .accounts(service_referral_protocol::accounts::Register {
            wallet: user.pubkey(),
            protocol,
            referrer_wallet: Pubkey::default(),
            referrer: technical_root,
            user: user_pda,
            system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::Register {})
        .instruction()
        .expect("register ix");
    ctx.execute_instruction(register_ix, &[&user])
        .expect("register tx")
        .assert_success();

    let treasury_usdt = ctx
        .svm
        .create_associated_token_account(&usdt_mint.pubkey(), &treasury)
        .expect("treasury USDT");
    let treasury_usdc = ctx
        .svm
        .create_associated_token_account(&usdc_mint.pubkey(), &treasury)
        .expect("treasury USDC");
    let user_usdc = ctx
        .svm
        .create_associated_token_account(&usdc_mint.pubkey(), &user)
        .expect("user USDC");

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
        .mint_to(&usdc_mint.pubkey(), &user_usdc, &initializer, 1_000 * UNIT)
        .expect("fund user");

    macro_rules! buy {
        ($units:expr) => {{
            ctx.svm.expire_blockhash();
            let purchase_ix = ctx
                .program()
                .accounts(service_referral_protocol::accounts::PurchaseAndDistribute {
                    wallet: user.pubkey(),
                    protocol,
                    user: user_pda,
                    user_source: user_usdc,
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
                .args(service_referral_protocol::instruction::PurchaseAndDistribute {
                    units: $units,
                })
                .instruction()
                .expect("purchase ix");
            ctx.execute_instruction(purchase_ix, &[&user])
                .expect("purchase tx")
                .assert_success();
        }};
    }

    // Week 1 requires 10.
    buy!(10);
    let week1 = read_user(&ctx, user_pda);
    assert_eq!(week1.active_weeks_started, 1);
    assert_eq!(week1.current_week_units, 10);
    assert_eq!(next_active_requirement(week1.active_weeks_started), 10);

    // Week 2 also requires 10 and can be started from GRACE.
    let mut clock: Clock = ctx.svm.get_sysvar();
    clock.unix_timestamp = week1.active_until + 1;
    ctx.svm.set_sysvar(&clock);
    buy!(10);
    let week2 = read_user(&ctx, user_pda);
    assert_eq!(week2.active_weeks_started, 2);
    assert_eq!(week2.current_week_units, 10);
    assert_eq!(next_active_requirement(week2.active_weeks_started), 20);

    // Calendar inactivity must not advance the progressive requirement. Move far
    // beyond GRACE: the next successful week is still week 3 and still requires 20.
    clock.unix_timestamp = week2.grace_until + 30 * 24 * 60 * 60;
    ctx.svm.set_sysvar(&clock);
    assert_eq!(read_user(&ctx, user_pda).active_weeks_started, 2);

    buy!(19);
    let partial = read_user(&ctx, user_pda);
    assert_eq!(partial.active_weeks_started, 2);
    assert_eq!(partial.qualification_progress_units, 19);
    assert!(clock.unix_timestamp > partial.grace_until);

    buy!(1);
    let week3 = read_user(&ctx, user_pda);
    assert_eq!(week3.active_weeks_started, 3);
    assert_eq!(week3.current_week_units, 20);
    assert_eq!(week3.qualification_progress_units, 0);
    assert_eq!(next_active_requirement(week3.active_weeks_started), 20);

    // Continue from GRACE through the exact frozen requirement ladder.
    let remaining_requirements = [20u64, 30, 30, 40, 40, 50, 50];
    for (offset, required) in remaining_requirements.into_iter().enumerate() {
        let previous = read_user(&ctx, user_pda);
        clock.unix_timestamp = previous.active_until + 1;
        ctx.svm.set_sysvar(&clock);
        buy!(required);

        let current = read_user(&ctx, user_pda);
        let expected_week = 4 + offset as u32;
        assert_eq!(current.active_weeks_started, expected_week);
        assert_eq!(current.current_week_units, required);
        assert_eq!(current.qualification_progress_units, 0);
        assert!(current.active_until > clock.unix_timestamp);
    }

    let week10 = read_user(&ctx, user_pda);
    assert_eq!(week10.active_weeks_started, 10);
    assert_eq!(week10.current_week_units, 50);
    assert_eq!(next_active_requirement(week10.active_weeks_started), 50);

    // Extra units while already ACTIVE increase only current-week depth. They must
    // never prequalify or increment the next ACTIVE week.
    buy!(450);
    let upgraded = read_user(&ctx, user_pda);
    assert_eq!(upgraded.active_weeks_started, 10);
    assert_eq!(upgraded.current_week_units, 500);
    assert_eq!(upgraded.qualification_progress_units, 0);
    assert_eq!(next_active_requirement(upgraded.active_weeks_started), 50);
}
