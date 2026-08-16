use anchor_lang::{prelude::*, AccountDeserialize};
use anchor_litesvm::{AnchorContext, AnchorLiteSVM, AssertionHelpers, TestHelpers};
use service_referral_protocol::{state::UserState, ID};
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
fn full_genealogy_pays_self_and_nine_uplines_but_never_tenth_upline() {
    let mut ctx = AnchorLiteSVM::build_with_program(ID, PROGRAM_BYTES);
    let initializer = ctx
        .svm
        .create_funded_account(80_000_000_000)
        .expect("initializer");
    let treasury = ctx
        .svm
        .create_funded_account(20_000_000_000)
        .expect("treasury");

    // Index mapping is deliberately top-down:
    // 0=U11, 1=U10, 2=U9, ... 10=U1/sponsor, 11=buyer/SELF.
    let mut wallets = Vec::new();
    for _ in 0..12 {
        wallets.push(
            ctx.svm
                .create_funded_account(10_000_000_000)
                .expect("fund genealogy wallet"),
        );
    }

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
    let user_pdas: Vec<Pubkey> = wallets
        .iter()
        .map(|wallet| Pubkey::find_program_address(&[b"user", wallet.pubkey().as_ref()], &ID).0)
        .collect();

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

    // Register a real ancestor chain ending at the buyer.
    for i in 0..wallets.len() {
        let (referrer_wallet, referrer_pda) = if i == 0 {
            (Pubkey::default(), technical_root)
        } else {
            (wallets[i - 1].pubkey(), user_pdas[i - 1])
        };
        let register_ix = ctx
            .program()
            .accounts(service_referral_protocol::accounts::Register {
                wallet: wallets[i].pubkey(),
                protocol,
                referrer_wallet,
                referrer: referrer_pda,
                user: user_pdas[i],
                system_program: anchor_lang::system_program::ID,
            })
            .args(service_referral_protocol::instruction::Register {})
            .instruction()
            .expect("register ix");
        ctx.execute_instruction(register_ix, &[&wallets[i]])
            .expect("register tx")
            .assert_success();
    }

    let treasury_usdt = ctx
        .svm
        .create_associated_token_account(&usdt_mint.pubkey(), &treasury)
        .expect("treasury USDT");
    let treasury_usdc = ctx
        .svm
        .create_associated_token_account(&usdc_mint.pubkey(), &treasury)
        .expect("treasury USDC");

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

    // Activate exactly the nine payable uplines U9 through U1. U10/U11 stay inactive.
    for i in 2..=10 {
        let source = ctx
            .svm
            .create_associated_token_account(&usdc_mint.pubkey(), &wallets[i])
            .expect("activation ATA");
        ctx.svm
            .mint_to(&usdc_mint.pubkey(), &source, &initializer, 10 * UNIT)
            .expect("mint activation funds");

        let mut uplines = [technical_root; 8];
        let mut ancestor = i as isize - 2;
        for slot in 0..8 {
            if ancestor >= 0 {
                uplines[slot] = user_pdas[ancestor as usize];
                ancestor -= 1;
            }
        }
        let (batch, _) = Pubkey::find_program_address(
            &[b"batch", wallets[i].pubkey().as_ref(), &0u64.to_le_bytes()],
            &ID,
        );

        let purchase_ix = ctx
            .program()
            .accounts(service_referral_protocol::accounts::PurchaseAndDistribute {
                wallet: wallets[i].pubkey(),
                protocol,
                user: user_pdas[i],
                user_source: source,
                vault_authority,
                usdt_vault: vault_usdt,
                usdc_vault: vault_usdc,
                service_treasury_usdt: treasury_usdt,
                service_treasury_usdc: treasury_usdc,
                direct_referrer: user_pdas[i - 1],
                upline_1: uplines[0],
                upline_2: uplines[1],
                upline_3: uplines[2],
                upline_4: uplines[3],
                upline_5: uplines[4],
                upline_6: uplines[5],
                upline_7: uplines[6],
                upline_8: uplines[7],
                batch,
                token_program: spl_token::id(),
                system_program: anchor_lang::system_program::ID,
            })
            .args(service_referral_protocol::instruction::PurchaseAndDistribute { units: 10 })
            .instruction()
            .expect("activation purchase ix");
        ctx.execute_instruction(purchase_ix, &[&wallets[i]])
            .expect("activation purchase tx")
            .assert_success();
    }

    let before: Vec<UserState> = user_pdas
        .iter()
        .map(|pda| read_user(&ctx, *pda))
        .collect();

    let buyer_index = 11usize;
    let buyer_source = ctx
        .svm
        .create_associated_token_account(&usdc_mint.pubkey(), &wallets[buyer_index])
        .expect("buyer USDC ATA");
    ctx.svm
        .mint_to(
            &usdc_mint.pubkey(),
            &buyer_source,
            &initializer,
            100 * UNIT,
        )
        .expect("mint buyer funds");
    let (buyer_batch, _) = Pubkey::find_program_address(
        &[
            b"batch",
            wallets[buyer_index].pubkey().as_ref(),
            &0u64.to_le_bytes(),
        ],
        &ID,
    );

    // buyer receives SELF 50%; sponsor index10 is U1, then indices9..2 are U2..U9.
    let target_ix = ctx
        .program()
        .accounts(service_referral_protocol::accounts::PurchaseAndDistribute {
            wallet: wallets[buyer_index].pubkey(),
            protocol,
            user: user_pdas[buyer_index],
            user_source: buyer_source,
            vault_authority,
            usdt_vault: vault_usdt,
            usdc_vault: vault_usdc,
            service_treasury_usdt: treasury_usdt,
            service_treasury_usdc: treasury_usdc,
            direct_referrer: user_pdas[10],
            upline_1: user_pdas[9],
            upline_2: user_pdas[8],
            upline_3: user_pdas[7],
            upline_4: user_pdas[6],
            upline_5: user_pdas[5],
            upline_6: user_pdas[4],
            upline_7: user_pdas[3],
            upline_8: user_pdas[2],
            batch: buyer_batch,
            token_program: spl_token::id(),
            system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::PurchaseAndDistribute { units: 100 })
        .instruction()
        .expect("target purchase ix");
    ctx.execute_instruction(target_ix, &[&wallets[buyer_index]])
        .expect("target purchase tx")
        .assert_success();

    ctx.svm.assert_token_balance(&buyer_source, 0);

    let after: Vec<UserState> = user_pdas
        .iter()
        .map(|pda| read_user(&ctx, *pda))
        .collect();

    // Buyer is SELF and receives exactly 50%; target purchase gives the sponsor no SELF delta.
    assert_eq!(
        after[11].self_accrued_usdc - before[11].self_accrued_usdc,
        50 * UNIT
    );
    assert_eq!(
        after[10].self_accrued_usdc - before[10].self_accrued_usdc,
        0
    );

    // U1 sponsor gets 15%, followed by U2..U9 across indices 9 down to 2.
    assert_eq!(
        after[10].network_claimable_usdc - before[10].network_claimable_usdc,
        15 * UNIT
    );
    let expected = [
        9 * UNIT,
        6 * UNIT,
        4 * UNIT,
        2_500_000,
        2 * UNIT,
        1_500_000,
        1 * UNIT,
        2 * UNIT,
    ];
    for (offset, amount) in expected.iter().enumerate() {
        let index = 9 - offset;
        assert_eq!(
            after[index].network_claimable_usdc - before[index].network_claimable_usdc,
            *amount,
            "unexpected network delta at upline {}",
            offset + 2
        );
    }

    // U10 and U11 exist in the real ancestry but are outside the nine-upline cap.
    for index in [0usize, 1usize] {
        assert_eq!(after[index].network_claimable_usdc, before[index].network_claimable_usdc);
        assert_eq!(after[index].network_pending_usdc, before[index].network_pending_usdc);
        assert_eq!(after[index].lifetime_expired_usdc, before[index].lifetime_expired_usdc);
    }
}
