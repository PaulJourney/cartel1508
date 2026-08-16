from pathlib import Path

source = Path('programs/service_referral_protocol/src/lib.rs').read_text()
constants = Path('programs/service_referral_protocol/src/constants.rs').read_text()
state = Path('programs/service_referral_protocol/src/state.rs').read_text()


def section(start_marker: str, end_marker: str) -> str:
    start = source.index(start_marker)
    end = source.index(end_marker, start)
    return source[start:end]

legacy_purchase = section('    pub fn purchase_service_units', '    pub fn record_qualified_revenue')
legacy_revenue = section('    pub fn record_qualified_revenue', '    pub fn settle_expired')
final_purchase = section('    pub fn purchase_and_distribute', '    pub fn claim')
claim = section('    pub fn claim', '}\n\n#[derive(Accounts)]')
activity_helper = section('fn update_activity_after_purchase', 'fn add_direct')
production_accounts = section("pub struct PurchaseAndDistribute", "pub struct SettleExpired")

checks = {
    'no owner/admin mutation surface': all(x not in source for x in ['owner_admin', 'set_admin', 'transfer_admin', 'set_owner']),
    'no pause instruction': 'pub fn pause' not in source,
    'no treasury mutation instruction': 'set_treasury' not in source,
    'no referral mutation instruction': 'set_referrer' not in source,
    'no revenue-source mutation instruction': 'set_qualified_revenue_source' not in source,
    'final purchase is the reward event': all(x in final_purchase for x in ['split_purchase_amount(payment)', 'add_direct(', 'add_network_claimable(', 'accrue_pioneer(']),
    'final purchase sends buyer funds to canonical vault': 'to: payment_destination' in final_purchase and 'validate_vault_token_for_mint' in final_purchase,
    'sponsor is direct level one only': 'direct_referrer.wallet == ctx.accounts.user.referrer' in final_purchase and 'add_direct(&mut ctx.accounts.direct_referrer' in final_purchase,
    'network starts at genealogical level two': 'let mut expected_wallet = ctx.accounts.direct_referrer.referrer' in final_purchase,
    'production network stops at level ten': 'for i in 0..9' in final_purchase and all(f'upline_{i}' in production_accounts for i in range(1, 10)) and 'upline_10' not in production_accounts,
    'production network weights are nine levels and 43 percent': 'PURCHASE_NETWORK_LEVEL_BPS: [u64; 9]' in constants and '[1_500, 900, 600, 400, 250, 200, 150, 100, 200]' in constants,
    'legacy purchase disabled in production': '#[cfg(feature = "production")]' in legacy_purchase and 'LegacyRevenuePathDisabled' in legacy_purchase,
    'legacy qualified revenue disabled in production': '#[cfg(feature = "production")]' in legacy_revenue and 'LegacyRevenuePathDisabled' in legacy_revenue,
    'canonical ATA derivation exists': 'get_associated_token_address_with_program_id' in source and 'canonical_ata(' in source,
    'vault ATA must be canonical': 'vault.key() == canonical_ata(vault_authority, mint)' in source,
    'treasury ATA must be canonical': 'treasury.key() == canonical_ata(p.service_treasury, mint)' in source,
    'claim destination ATA must be canonical': 'destination.key() == canonical_ata(wallet, mint)' in source,
    'buyer and sponsor PDAs are validated': 'validate_user_pda(&ctx.accounts.user)' in final_purchase and 'validate_user_pda(&ctx.accounts.direct_referrer)' in final_purchase,
    'permissionless expired settlement exists': 'pub fn settle_expired' in source and 'pub settler: Signer' in source,
    'no in-memory-only expiry helper': 'settle_expired_in_memory' not in source,
    'expired flow transfers from vault': 'expire_pending_for_mint' in source and 'transfer_from_vault' in source,
    'activity partial window exists': 'qualification_window_started_at' in state and 'qualification_window_started_at' in activity_helper,
    'Pioneer high precision index exists': 'PIONEER_SCALE' in constants and '1_000_000_000_000_000_000' in constants,
    'Pioneer claim preserves fractional checkpoint': 'checkpoint_pioneer_claimed' in claim,
    'treasury accounting buckets separated': all(x in state for x in ['lifetime_service_fees_', 'lifetime_unallocated_', 'lifetime_expired_', 'lifetime_rounding_', 'lifetime_pioneer_unassigned_']),
    'pull claim exists and requires active': 'pub fn claim' in source and 'ProtocolError::NotActive' in claim,
    'claim requires claimant wallet signer': "pub wallet: Signer<'info>" in source and 'seeds = [b"user", wallet.key().as_ref()]' in source,
    'legacy token program is fixed': 'token::ID' in source,
    'mainnet treasury and stablecoin constants are frozen': all(x in constants for x in ['MAINNET_USDT_MINT', 'MAINNET_USDC_MINT', 'MAINNET_SERVICE_TREASURY']),
    'production launch time constant exists': 'MAINNET_REGISTRATION_OPEN_AT' in constants,
    'production initialization pins immutable mainnet inputs': all(x in source for x in ['validate_initialization_environment(', 'MAINNET_SERVICE_TREASURY', 'MAINNET_USDT_MINT', 'MAINNET_USDC_MINT', 'MAINNET_REGISTRATION_OPEN_AT', 'ProductionConfigNotFrozen', 'InvalidProductionConfig']),
    'initialization uses typed SPL Mint accounts': "pub usdt_mint: Box<Account<'info, Mint>>" in source and "pub usdc_mint: Box<Account<'info, Mint>>" in source,
    'stablecoin mints require six decimals': source.count('decimals == TOKEN_DECIMALS as u8') >= 2 and 'InvalidTokenDecimals' in source,
    'USDT and USDC mint accounts must differ': 'usdt_mint.key() != ctx.accounts.usdc_mint.key()' in source and 'DuplicateStablecoinMint' in source,
    'service units use global monotonic Unit IDs': all(x in state for x in ['next_unit_id', 'first_unit_id', 'last_unit_id']) and 'allocate_unit_range(ctx.accounts.protocol.next_unit_id, units)' in final_purchase and 'ctx.accounts.protocol.next_unit_id = next_unit_id' in final_purchase and 'first_local_unit_index' not in source and 'last_local_unit_index' not in source,
}

for name, ok in checks.items():
    print(('PASS' if ok else 'FAIL'), name)
if not all(checks.values()):
    raise SystemExit(1)
