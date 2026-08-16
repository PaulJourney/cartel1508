from pathlib import Path

source = Path('programs/service_referral_protocol/src/lib.rs').read_text()
constants = Path('programs/service_referral_protocol/src/constants.rs').read_text()
state = Path('programs/service_referral_protocol/src/state.rs').read_text()


def section(start_marker: str, end_marker: str) -> str:
    start = source.index(start_marker)
    end = source.index(end_marker, start)
    return source[start:end]


final_purchase = section('    pub fn purchase_and_distribute', '    pub fn settle_expired')
claim = section('    pub fn claim', '}\n\n#[derive(Accounts)]')
activity_helper = section('fn update_activity_after_purchase', 'fn add_self_reward')
expiry_helper = section('fn expire_unclaimed_for_mint', '#[allow(clippy::too_many_arguments)]')
production_accounts = section('pub struct PurchaseAndDistribute', 'pub struct SettleExpired')
initialize = section('    pub fn initialize', '    pub fn register')

checks = {
    'no owner/admin mutation surface': all(x not in source for x in ['owner_admin', 'set_admin', 'transfer_admin', 'set_owner']),
    'no pause instruction': 'pub fn pause' not in source,
    'no treasury mutation instruction': 'set_treasury' not in source,
    'no referral mutation instruction': 'set_referrer' not in source,
    'legacy purchase instruction removed': 'pub fn purchase_service_units' not in source and 'PurchaseServiceUnits' not in source,
    'legacy qualified revenue instruction removed': 'pub fn record_qualified_revenue' not in source and 'RecordQualifiedRevenue' not in source,
    'legacy revenue source state removed': 'qualified_revenue_source' not in source and 'qualified_revenue_source' not in state,
    'legacy adapter/source constants removed': 'MAINNET_REVENUE_ADAPTER_PROGRAM' not in constants and 'MAINNET_QUALIFIED_REVENUE_SOURCE' not in constants,
    'final purchase is the sole reward event': all(x in final_purchase for x in ['split_purchase_amount(payment)', 'add_self_reward(', 'add_network_claimable(', 'accrue_pioneer(']),
    'final purchase sends buyer funds to canonical vault': 'to: payment_destination' in final_purchase and 'validate_vault_token_for_mint' in final_purchase,
    'purchase expires stale buyer value before reactivation': final_purchase.index('settle_expired_all(') < final_purchase.index('update_activity_after_purchase('),
    'buyer receives the 50 percent SELF bucket': 'add_self_reward(&mut ctx.accounts.user' in final_purchase and 'add_self_reward(&mut ctx.accounts.direct_referrer' not in final_purchase,
    'network starts at immutable sponsor': 'direct_referrer.wallet == ctx.accounts.user.referrer' in final_purchase and 'levels[0]' in final_purchase and 'add_network_claimable(&mut ctx.accounts.direct_referrer' in final_purchase,
    'network contains exactly nine uplines': 'for i in 1..9' in final_purchase and all(f'upline_{i}' in production_accounts for i in range(1, 9)) and 'upline_9' not in production_accounts,
    'production network weights are nine levels and 43 percent': 'PURCHASE_NETWORK_LEVEL_BPS: [u64; 9]' in constants and '[1_500, 900, 600, 400, 250, 200, 150, 100, 200]' in constants,
    'aggregate percentages are frozen': all(x in constants for x in ['SELF_BPS: u64 = 5_000', 'NETWORK_BPS: u64 = 4_300', 'PIONEER_BPS: u64 = 200', 'SERVICE_BPS: u64 = 500']),
    'canonical ATA derivation exists': 'get_associated_token_address_with_program_id' in source and 'canonical_ata(' in source,
    'vault ATA must be canonical': 'vault.key() == canonical_ata(vault_authority, mint)' in source,
    'treasury ATA must be canonical': 'treasury.key() == canonical_ata(p.service_treasury, mint)' in source,
    'claim destination ATA must be canonical': 'destination.key() == canonical_ata(wallet, mint)' in source,
    'buyer and sponsor PDAs are validated': 'validate_user_pda(&ctx.accounts.user)' in final_purchase and 'validate_user_pda(&ctx.accounts.direct_referrer)' in final_purchase,
    'permissionless expired settlement exists': 'pub fn settle_expired' in source and 'pub settler: Signer' in source,
    'all inactive unclaimed flow is covered': all(x in expiry_helper for x in ['ActivityStatus::Inactive', 'self_accrued_usdt', 'self_accrued_usdc', 'network_claimable_usdt', 'network_claimable_usdc', 'network_pending_usdt', 'network_pending_usdc', 'pioneer_due(', 'checkpoint_pioneer_claimed(', 'mark_user_expired(']),
    'inactive unclaimed flow transfers from vault': 'expire_unclaimed_for_mint' in source and 'transfer_from_vault' in source,
    'activity partial window exists': 'qualification_window_started_at' in state and 'qualification_window_started_at' in activity_helper,
    'Pioneer high precision index exists': 'PIONEER_SCALE' in constants and '1_000_000_000_000_000_000' in constants,
    'Pioneer claim preserves fractional checkpoint': 'checkpoint_pioneer_claimed' in claim,
    'treasury accounting buckets separated': all(x in state for x in ['lifetime_service_fees_', 'lifetime_unallocated_', 'lifetime_expired_', 'lifetime_rounding_', 'lifetime_pioneer_unassigned_']),
    'pull claim exists and requires active': 'ProtocolError::NotActive' in claim and 'take_claimable' in claim,
    'claim requires claimant wallet signer': "pub wallet: Signer<'info>" in source and 'seeds = [b"user", wallet.key().as_ref()]' in source,
    'SPL token CPI uses typed token program key': 'token_program.key()' in source and 'pub token_program: Program<' in source,
    'mainnet treasury and stablecoin constants are frozen': all(x in constants for x in ['MAINNET_USDT_MINT', 'MAINNET_USDC_MINT', 'MAINNET_SERVICE_TREASURY']),
    'production launch time remains fail closed': 'MAINNET_REGISTRATION_OPEN_AT: i64 = 0' in constants,
    'production initialization pins only final mainnet inputs': all(x in source for x in ['validate_initialization_environment(', 'MAINNET_SERVICE_TREASURY', 'MAINNET_USDT_MINT', 'MAINNET_USDC_MINT', 'MAINNET_REGISTRATION_OPEN_AT', 'ProductionConfigNotFrozen', 'InvalidProductionConfig']) and 'qualified_revenue' not in initialize,
    'initialization uses typed SPL Mint accounts': "pub usdt_mint: Box<Account<'info, Mint>>" in source and "pub usdc_mint: Box<Account<'info, Mint>>" in source,
    'stablecoin mints require six decimals': source.count('decimals == TOKEN_DECIMALS as u8') >= 2 and 'InvalidTokenDecimals' in source,
    'USDT and USDC mint accounts must differ': 'usdt_mint.key() != ctx.accounts.usdc_mint.key()' in source and 'DuplicateStablecoinMint' in source,
    'service units use global monotonic Unit IDs': all(x in state for x in ['next_unit_id', 'first_unit_id', 'last_unit_id']) and 'allocate_unit_range(ctx.accounts.protocol.next_unit_id, units)' in final_purchase and 'ctx.accounts.protocol.next_unit_id = next_unit_id' in final_purchase and 'first_local_unit_index' not in source and 'last_local_unit_index' not in source,
}

for name, ok in checks.items():
    print(('PASS' if ok else 'FAIL'), name)
if not all(checks.values()):
    raise SystemExit(1)
