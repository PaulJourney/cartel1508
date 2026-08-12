from pathlib import Path

source = Path('programs/service_referral_protocol/src/lib.rs').read_text()
constants = Path('programs/service_referral_protocol/src/constants.rs').read_text()
state = Path('programs/service_referral_protocol/src/state.rs').read_text()


def section(start_marker: str, end_marker: str) -> str:
    start = source.index(start_marker)
    end = source.index(end_marker, start)
    return source[start:end]

purchase = section('    pub fn purchase_service_units', '    pub fn record_qualified_revenue')
revenue = section('    pub fn record_qualified_revenue', '    pub fn settle_expired')
claim = section('    pub fn claim', '}\n\n#[derive(Accounts)]')

checks = {
    'no owner/admin mutation surface': all(x not in source for x in ['owner_admin', 'set_admin', 'transfer_admin', 'set_owner']),
    'no pause instruction': 'pub fn pause' not in source,
    'no treasury mutation instruction': 'set_treasury' not in source,
    'no referral mutation instruction': 'set_referrer' not in source,
    'no revenue-source mutation instruction': 'set_qualified_revenue_source' not in source,
    'service-unit purchase has no reward split': all(x not in purchase for x in ['split_amount(', 'add_direct(', 'add_network_claimable(', 'accrue_pioneer(']),
    'qualified revenue is separately authenticated': 'qualified_revenue_source' in revenue and 'InvalidRevenueSource' in revenue,
    '10 explicit upline accounts': all(f'upline_{i}' in source for i in range(1, 11)),
    'canonical ATA derivation exists': 'get_associated_token_address_with_program_id' in source and 'canonical_ata(' in source,
    'vault ATA must be canonical': 'vault.key() == canonical_ata(vault_authority, mint)' in source,
    'treasury ATA must be canonical': 'treasury.key() == canonical_ata(p.service_treasury, mint)' in source,
    'claim destination ATA must be canonical': 'destination.key() == canonical_ata(wallet, mint)' in source,
    'beneficiary/user PDA validation exists': 'validate_user_pda(&ctx.accounts.beneficiary)' in revenue and 'InvalidUserPda' in source,
    'permissionless expired settlement exists': 'pub fn settle_expired' in source and 'pub settler: Signer' in source,
    'no in-memory-only expiry helper': 'settle_expired_in_memory' not in source,
    'expired flow transfers from vault': 'expire_pending_for_mint' in source and 'transfer_from_vault' in source,
    'activity partial window exists': 'qualification_window_started_at' in state and 'qualification_window_started_at' in purchase,
    'Pioneer high precision index exists': 'PIONEER_SCALE' in constants and '1_000_000_000_000_000_000' in constants,
    'Pioneer claim preserves fractional checkpoint': 'checkpoint_pioneer_claimed' in claim,
    'treasury accounting buckets separated': all(x in state for x in ['lifetime_service_fees_', 'lifetime_unallocated_', 'lifetime_expired_', 'lifetime_rounding_', 'lifetime_pioneer_unassigned_']),
    'pull claim exists and requires active': 'pub fn claim' in source and 'ProtocolError::NotActive' in claim,
    'legacy token program is fixed': 'token::ID' in source,
}

for name, ok in checks.items():
    print(('PASS' if ok else 'FAIL'), name)
if not all(checks.values()):
    raise SystemExit(1)
