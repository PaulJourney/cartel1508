from pathlib import Path

source = Path('programs/service_referral_protocol/src/lib.rs').read_text()
constants = Path('programs/service_referral_protocol/src/constants.rs').read_text()
state = Path('programs/service_referral_protocol/src/state.rs').read_text()
math = Path('programs/service_referral_protocol/src/math.rs').read_text()


def section(start_marker: str, end_marker: str) -> str:
    start = source.index(start_marker)
    end = source.index(end_marker, start)
    return source[start:end]


final_purchase = section('    pub fn purchase_and_distribute', '    pub fn settle_expired')
claim = section('    pub fn claim', '}\n\n#[derive(Accounts)]')
activity_helper = section('fn update_activity_after_purchase', 'fn add_self_reward')
qualification_window_helper = section('fn qualification_window_open', 'fn expire_unclaimed_for_mint')
network_distribution_helper = section('fn distribute_network_level', 'fn vest_pending')
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
    'final purchase is the sole reward event': all(x in final_purchase for x in ['split_purchase_amount(payment)', 'add_self_reward(', 'distribute_network_level(', 'accrue_pioneer(']) and 'add_network_claimable(' in network_distribution_helper,
    'final purchase sends buyer funds to canonical vault': 'to: payment_destination' in final_purchase and 'validate_vault_token_for_mint' in final_purchase,
    'purchase expires stale buyer value before reactivation': final_purchase.index('settle_expired_all(') < final_purchase.index('update_activity_after_purchase('),
    'buyer receives the 50 percent SELF bucket': 'add_self_reward(&mut ctx.accounts.user' in final_purchase and 'add_self_reward(&mut ctx.accounts.direct_referrer' not in final_purchase,
    'network starts at immutable sponsor': 'direct_referrer.wallet == ctx.accounts.user.referrer' in final_purchase and 'levels[0]' in final_purchase and 'distribute_network_level(' in final_purchase,
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
    'qualification window uses progress as existence sentinel': 'if user.qualification_progress_units == 0' in qualification_window_helper and 'qualification_window_started_at > 0' not in qualification_window_helper,
    'stale qualification reset is not gated by positive timestamp': 'if user.qualification_progress_units > 0' in activity_helper and 'qualification_window_started_at > 0' not in activity_helper,
    'inactive partial qualification preserves SELF but not Pioneer': 'qualification_window_open' in source and 'let preserve_self = qualification_window_open' in expiry_helper and 'let pioneer = pioneer_due(user, p, mint)?' in expiry_helper and 'preserve_self_and_pioneer' not in expiry_helper and 'network_claimable_usdt = 0' in expiry_helper and 'network_pending_usdt = 0' in expiry_helper,
    'progressive ACTIVE schedule is frozen': 'ACTIVE_WEEK_REQUIREMENT_UNITS: [u64; 5] = [10, 20, 30, 40, 50]' in constants and 'ACTIVE_WEEK_TIER_SPAN: u32 = 2' in constants and 'active_weeks_started' in state and 'current_week_units' in state and 'next_active_requirement(user.active_weeks_started)' in activity_helper,
    'weekly depth thresholds are frozen': 'NETWORK_DEPTH_UNIT_THRESHOLDS: [u64; 7] = [10, 25, 50, 100, 200, 350, 500]' in constants and 'network_depth_for_units(user.current_week_units)' in network_distribution_helper,
    'out-of-depth network share routes to Treasury without compression': 'level_number' in network_distribution_helper and 'unallocated' in network_distribution_helper and 'checked_add(amount)' in network_distribution_helper,
    'active purchases increase depth without prequalifying next week': 'pre_status == ActivityStatus::Active' in activity_helper and 'current_week_units' in activity_helper and 'return Ok(())' in activity_helper,
    'registration and purchase events support deterministic rank indexing': 'pub struct UserRegistered' in source and 'pub referrer: Pubkey' in source and all(x in source for x in ['active_weeks_started: u32', 'current_week_units: u64', 'network_depth: u8', 'next_active_requirement_units: u64']),
    'Pioneer positions require one 1000-unit purchase': 'PIONEER_POSITION_PURCHASE_UNITS: u64 = 1_000' in constants and 'units / PIONEER_POSITION_PURCHASE_UNITS' in math,
    'Pioneer registration consumes no position': 'u.pioneer_positions = 0' in source and 'Registration alone never consumes a Pioneer position' in source,
    'Pioneer global cap is absolute 100': 'pioneer_positions_for_purchase(units, p.pioneer_positions_assigned)' in source and 'p.pioneer_positions_assigned <= PIONEER_SLOTS' in source,
    'Pioneer Rule B assigns after current pool accrual': final_purchase.index('accrue_pioneer(') < final_purchase.index('assign_pioneer_positions_after_purchase('),
    'Pioneer supports multiple weighted positions per wallet': 'user.pioneer_positions as u128' in source and 'checked_mul(user.pioneer_positions as u128)' in source,
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
    'service units use global monotonic Unit IDs': 'next_unit_id' in state and all(x in source for x in ['pub struct UnitsPurchased', 'first_unit_id: u128', 'last_unit_id: u128', 'purchase_index: u64', 'emit!(UnitsPurchased', 'allocate_unit_range(ctx.accounts.protocol.next_unit_id, units)', 'ctx.accounts.protocol.next_unit_id = next_unit_id']) and 'first_local_unit_index' not in source and 'last_local_unit_index' not in source,
    'purchase has no per-purchase rent account': 'UnitBatch' not in state and 'UnitBatch' not in source and 'pub batch:' not in production_accounts and 'payer = wallet' not in production_accounts and 'pub system_program' not in production_accounts,
}

for name, ok in checks.items():
    print(('PASS' if ok else 'FAIL'), name)
if not all(checks.values()):
    raise SystemExit(1)
