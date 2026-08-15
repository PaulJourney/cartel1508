use anchor_lang::{prelude::*, AccountDeserialize};
use anchor_litesvm::{AnchorLiteSVM, AssertionHelpers, TestHelpers};
use revenue_adapter::{AdapterConfig, CONFIG_SEED, QUALIFIER_AUTHORITY_SEED, REVENUE_AUTHORITY_SEED};

const ADAPTER_BYTES: &[u8] = include_bytes!("../../target/deploy/revenue_adapter.so");
const REFERRAL_BYTES: &[u8] = include_bytes!("../../target/deploy/service_referral_protocol.so");

fn adapter_context() -> anchor_litesvm::AnchorContext {
    AnchorLiteSVM::build_with_programs(&[
        (revenue_adapter::ID, ADAPTER_BYTES),
        (service_referral_protocol::ID, REFERRAL_BYTES),
    ])
}

#[test]
fn initialize_rejects_wrong_qualification_authority() {
    let mut ctx = adapter_context();
    let initializer = ctx
        .svm
        .create_funded_account(10_000_000_000)
        .expect("fund initializer");

    let qualification_program = Pubkey::new_unique();
    let wrong_qualification_authority = Pubkey::new_unique();
    let usdt_mint = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();
    let (config, _) = Pubkey::find_program_address(&[CONFIG_SEED], &revenue_adapter::ID);
    let (revenue_authority, _) =
        Pubkey::find_program_address(&[REVENUE_AUTHORITY_SEED], &revenue_adapter::ID);

    let ix = ctx
        .program()
        .accounts(revenue_adapter::accounts::Initialize {
            initializer: initializer.pubkey(),
            config,
            revenue_authority,
            qualification_authority: wrong_qualification_authority,
            system_program: anchor_lang::system_program::ID,
        })
        .args(revenue_adapter::instruction::Initialize {
            referral_program: service_referral_protocol::ID,
            qualification_program,
            usdt_mint,
            usdc_mint,
        })
        .instruction()
        .expect("build adapter initialize");

    let result = ctx
        .execute_instruction(ix, &[&initializer])
        .expect("execute adapter initialize");
    assert!(!result.is_success(), "wrong qualifier PDA must be rejected");
    assert!(ctx.svm.get_account(&config).is_none(), "failed init must roll back config creation");
}

#[test]
fn initialize_freezes_config_once_with_deterministic_pdas() {
    let mut ctx = adapter_context();
    let initializer = ctx
        .svm
        .create_funded_account(20_000_000_000)
        .expect("fund initializer");

    let qualification_program = Pubkey::new_unique();
    let usdt_mint = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();
    let (qualification_authority, _) =
        Pubkey::find_program_address(&[QUALIFIER_AUTHORITY_SEED], &qualification_program);
    let (config, config_bump) =
        Pubkey::find_program_address(&[CONFIG_SEED], &revenue_adapter::ID);
    let (revenue_authority, revenue_authority_bump) =
        Pubkey::find_program_address(&[REVENUE_AUTHORITY_SEED], &revenue_adapter::ID);

    let build_initialize = || {
        ctx.program()
            .accounts(revenue_adapter::accounts::Initialize {
                initializer: initializer.pubkey(),
                config,
                revenue_authority,
                qualification_authority,
                system_program: anchor_lang::system_program::ID,
            })
            .args(revenue_adapter::instruction::Initialize {
                referral_program: service_referral_protocol::ID,
                qualification_program,
                usdt_mint,
                usdc_mint,
            })
            .instruction()
            .expect("build adapter initialize")
    };

    ctx.execute_instruction(build_initialize(), &[&initializer])
        .expect("execute first initialize")
        .assert_success();

    let config_account = ctx.svm.get_account(&config).expect("config exists");
    let mut data = config_account.data.as_slice();
    let frozen = AdapterConfig::try_deserialize(&mut data).expect("deserialize adapter config");

    assert_eq!(frozen.bump, config_bump);
    assert_eq!(frozen.revenue_authority_bump, revenue_authority_bump);
    assert_eq!(frozen.referral_program, service_referral_protocol::ID);
    assert_eq!(frozen.qualification_program, qualification_program);
    assert_eq!(frozen.qualification_authority, qualification_authority);
    assert_eq!(frozen.revenue_authority, revenue_authority);
    assert_eq!(frozen.usdt_mint, usdt_mint);
    assert_eq!(frozen.usdc_mint, usdc_mint);

    ctx.svm.expire_blockhash();
    let second = ctx
        .execute_instruction(build_initialize(), &[&initializer])
        .expect("execute second initialize");
    assert!(!second.is_success(), "deterministic config PDA must be initialize-once");

    let config_after = ctx.svm.get_account(&config).expect("config remains");
    let mut data_after = config_after.data.as_slice();
    let frozen_after = AdapterConfig::try_deserialize(&mut data_after).expect("deserialize config after replay");
    assert_eq!(frozen_after.referral_program, frozen.referral_program);
    assert_eq!(frozen_after.qualification_program, frozen.qualification_program);
    assert_eq!(frozen_after.qualification_authority, frozen.qualification_authority);
    assert_eq!(frozen_after.revenue_authority, frozen.revenue_authority);
}
