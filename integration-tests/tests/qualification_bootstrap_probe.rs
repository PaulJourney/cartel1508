use anchor_lang::prelude::*;
use anchor_litesvm::{AnchorLiteSVM, AssertionHelpers, TestHelpers};
use service_referral_protocol::ID;
use solana_signer::Signer;

const PROGRAM_BYTES: &[u8] = include_bytes!("../../target/deploy/service_referral_protocol.so");
const ADAPTER_PROGRAM_ID: Pubkey = pubkey!("EmGJDPvwSx6kU4KijWGh8uqRNj3BJXNjcWQyCmfKv7WL");
const REVENUE_AUTHORITY_SEED: &[u8] = b"revenue-authority";

#[test]
fn referral_bootstrap_accepts_adapter_pda_as_frozen_revenue_source() {
    let mut ctx = AnchorLiteSVM::build_with_program(ID, PROGRAM_BYTES);
    let initializer = ctx
        .svm
        .create_funded_account(30_000_000_000)
        .expect("initializer");
    let treasury = ctx
        .svm
        .create_funded_account(10_000_000_000)
        .expect("treasury");
    let user = ctx
        .svm
        .create_funded_account(10_000_000_000)
        .expect("user");
    let _bootstrap_source = ctx
        .svm
        .create_funded_account(10_000_000_000)
        .expect("bootstrap source");

    let usdt_mint = ctx
        .svm
        .create_token_mint(&initializer, 6)
        .expect("USDT mint");
    let usdc_mint = ctx
        .svm
        .create_token_mint(&initializer, 6)
        .expect("USDC mint");

    let (protocol_pda, _) = Pubkey::find_program_address(&[b"protocol"], &ID);
    let (vault_authority, _) = Pubkey::find_program_address(&[b"vault-authority"], &ID);
    let (technical_root, _) =
        Pubkey::find_program_address(&[b"user", Pubkey::default().as_ref()], &ID);
    let (revenue_authority, _) =
        Pubkey::find_program_address(&[REVENUE_AUTHORITY_SEED], &ADAPTER_PROGRAM_ID);

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
            qualified_revenue_source: revenue_authority,
        })
        .instruction()
        .expect("initialize ix");

    ctx.execute_instruction(initialize_ix, &[&initializer])
        .expect("initialize tx")
        .assert_success();

    assert!(ctx.svm.get_account(&protocol_pda).is_some());
    assert!(ctx.svm.get_account(&technical_root).is_some());
    let _ = user;
}
