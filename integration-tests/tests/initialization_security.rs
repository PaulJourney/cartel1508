use anchor_lang::prelude::*;
use anchor_litesvm::{AnchorLiteSVM, TestHelpers};
use service_referral_protocol::ID;
use solana_signer::Signer;

const PROGRAM_BYTES: &[u8] = include_bytes!("../../target/deploy/service_referral_protocol.so");

fn protocol_addresses() -> (Pubkey, Pubkey, Pubkey) {
    let (protocol, _) = Pubkey::find_program_address(&[b"protocol"], &ID);
    let (vault_authority, _) = Pubkey::find_program_address(&[b"vault-authority"], &ID);
    let (technical_root, _) = Pubkey::find_program_address(&[b"user", Pubkey::default().as_ref()], &ID);
    (protocol, vault_authority, technical_root)
}

#[test]
fn initialize_rejects_non_six_decimal_stablecoin_mint_atomically() {
    let mut ctx = AnchorLiteSVM::build_with_program(ID, PROGRAM_BYTES);
    let initializer = ctx.svm.create_funded_account(20_000_000_000).expect("initializer");
    let treasury = ctx.svm.create_funded_account(5_000_000_000).expect("treasury");
    let revenue_source = ctx.svm.create_funded_account(5_000_000_000).expect("revenue source");

    let invalid_usdt = ctx.svm.create_token_mint(&initializer, 5).expect("5-decimal mint");
    let valid_usdc = ctx.svm.create_token_mint(&initializer, 6).expect("6-decimal mint");
    let (protocol, vault_authority, technical_root) = protocol_addresses();

    let registration_open_at = ctx.svm.get_sysvar::<Clock>().unix_timestamp;
    let initialize_ix = ctx
        .program()
        .accounts(service_referral_protocol::accounts::Initialize {
            initializer: initializer.pubkey(),
            service_treasury: treasury.pubkey(),
            usdt_mint: invalid_usdt.pubkey(),
            usdc_mint: valid_usdc.pubkey(),
            protocol,
            vault_authority,
            technical_root,
            system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::Initialize {
            registration_open_at,
            qualified_revenue_source: revenue_source.pubkey(),
        })
        .instruction()
        .expect("initialize ix");

    let result = ctx.execute_instruction(initialize_ix, &[&initializer]);
    assert!(result.is_err(), "5-decimal stablecoin mint must be rejected");
    assert!(ctx.svm.get_account(&protocol).is_none(), "failed initialize must not leave ProtocolState");
    assert!(ctx.svm.get_account(&technical_root).is_none(), "failed initialize must not leave technical root");
}

#[test]
fn initialize_rejects_same_mint_for_usdt_and_usdc_atomically() {
    let mut ctx = AnchorLiteSVM::build_with_program(ID, PROGRAM_BYTES);
    let initializer = ctx.svm.create_funded_account(20_000_000_000).expect("initializer");
    let treasury = ctx.svm.create_funded_account(5_000_000_000).expect("treasury");
    let revenue_source = ctx.svm.create_funded_account(5_000_000_000).expect("revenue source");

    let same_mint = ctx.svm.create_token_mint(&initializer, 6).expect("6-decimal mint");
    let (protocol, vault_authority, technical_root) = protocol_addresses();

    let registration_open_at = ctx.svm.get_sysvar::<Clock>().unix_timestamp;
    let initialize_ix = ctx
        .program()
        .accounts(service_referral_protocol::accounts::Initialize {
            initializer: initializer.pubkey(),
            service_treasury: treasury.pubkey(),
            usdt_mint: same_mint.pubkey(),
            usdc_mint: same_mint.pubkey(),
            protocol,
            vault_authority,
            technical_root,
            system_program: anchor_lang::system_program::ID,
        })
        .args(service_referral_protocol::instruction::Initialize {
            registration_open_at,
            qualified_revenue_source: revenue_source.pubkey(),
        })
        .instruction()
        .expect("initialize ix");

    let result = ctx.execute_instruction(initialize_ix, &[&initializer]);
    assert!(result.is_err(), "USDT and USDC must not resolve to the same mint");
    assert!(ctx.svm.get_account(&protocol).is_none(), "failed initialize must not leave ProtocolState");
    assert!(ctx.svm.get_account(&technical_root).is_none(), "failed initialize must not leave technical root");
}
