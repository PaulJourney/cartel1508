use anchor_lang::prelude::*;
use revenue_adapter::{
    MAINNET_QUALIFICATION_PROGRAM, MAINNET_REFERRAL_PROGRAM, REVENUE_AUTHORITY_SEED,
};
use service_referral_protocol::constants::{
    MAINNET_QUALIFIED_REVENUE_SOURCE, MAINNET_REVENUE_ADAPTER_PROGRAM,
};

#[test]
fn development_release_bindings_remain_fail_closed() {
    // This development branch must not accidentally contain a launchable mainnet binding.
    if MAINNET_REVENUE_ADAPTER_PROGRAM == Pubkey::default() {
        assert_eq!(MAINNET_QUALIFIED_REVENUE_SOURCE, pubkey!("11111111111111111111111111111111"));
    }
}

#[cfg(feature = "production")]
#[test]
fn production_release_binds_core_to_exact_adapter_and_revenue_authority() {
    assert_ne!(MAINNET_REVENUE_ADAPTER_PROGRAM, Pubkey::default());
    assert_eq!(MAINNET_REVENUE_ADAPTER_PROGRAM, revenue_adapter::ID);

    let (expected_source, _) =
        Pubkey::find_program_address(&[REVENUE_AUTHORITY_SEED], &revenue_adapter::ID);
    assert_eq!(MAINNET_QUALIFIED_REVENUE_SOURCE, expected_source);

    assert_ne!(MAINNET_REFERRAL_PROGRAM, Pubkey::default());
    assert_ne!(MAINNET_QUALIFICATION_PROGRAM, Pubkey::default());
    assert_eq!(MAINNET_REFERRAL_PROGRAM, service_referral_protocol::ID);
    assert_ne!(MAINNET_QUALIFICATION_PROGRAM, service_referral_protocol::ID);
}
