use anchor_lang::prelude::*;
use service_referral_protocol::constants::{
    MAINNET_QUALIFIED_REVENUE_SOURCE,
    MAINNET_REVENUE_ADAPTER_PROGRAM,
};

#[test]
fn frozen_release_binding_is_all_or_nothing_and_pda_exact() {
    let sentinel = Pubkey::default();
    let verifier = revenue_adapter::MAINNET_VERIFIER_PROGRAM;
    let adapter = MAINNET_REVENUE_ADAPTER_PROGRAM;
    let source = MAINNET_QUALIFIED_REVENUE_SOURCE;

    let any_frozen = verifier != sentinel || adapter != sentinel || source != sentinel;
    if !any_frozen {
        return;
    }

    assert_ne!(verifier, sentinel, "verifier Program ID must be frozen together with adapter binding");
    assert_ne!(adapter, sentinel, "Revenue Adapter Program ID must be frozen together with qualified source");
    assert_ne!(source, sentinel, "qualified revenue source PDA must be frozen together with adapter Program ID");

    assert_eq!(
        adapter,
        revenue_adapter::ID,
        "core MAINNET_REVENUE_ADAPTER_PROGRAM must equal adapter declare_id!"
    );

    let expected_source = Pubkey::find_program_address(
        &[revenue_adapter::REVENUE_AUTHORITY_SEED],
        &revenue_adapter::ID,
    )
    .0;
    assert_eq!(
        source,
        expected_source,
        "core MAINNET_QUALIFIED_REVENUE_SOURCE must equal adapter RevenueAuthority PDA"
    );

    assert_ne!(verifier, revenue_adapter::ID, "verifier and adapter Program IDs must differ");
    assert_ne!(verifier, service_referral_protocol::ID, "verifier and referral Program IDs must differ");
    assert_ne!(adapter, service_referral_protocol::ID, "adapter and referral Program IDs must differ");
}
