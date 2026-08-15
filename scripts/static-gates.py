from pathlib import Path

source = Path('programs/service_referral_protocol/src/lib.rs').read_text()
constants = Path('programs/service_referral_protocol/src/constants.rs').read_text()
state = Path('programs/service_referral_protocol/src/state.rs').read_text()
adapter = Path('programs/revenue_adapter/src/lib.rs').read_text()
qualification = Path('programs/revenue_qualification/src/lib.rs').read_text()


def section(start_marker: str, end_marker: str) -> str:
    start = source.index(start_marker)
    end = source.index(end_marker, start)
    return source[start:end]


def no_admin_surface(text: str) -> bool:
    forbidden = [
        'set_admin',
        'transfer_admin',
        'set_owner',
        'transfer_owner',
        'set_treasury',
        'set_referrer',
        'set_qualified_revenue_source',
        'update_config',
        'set_config',
        'pub fn pause',
    ]
    return all(item not in text for item in forbidden)


purchase = section('    pub fn purchase_service_units', '    pub fn record_qualified_revenue')
revenue = section('    pub fn record_qualified_revenue', '    pub fn settle_expired')
claim = section('    pub fn claim', '}\n\n#[derive(Accounts)]')

checks = {
    # Core protocol invariants.
    'core has no owner/admin mutation surface': no_admin_surface(source) and 'owner_admin' not in source,
    'core has no revenue-source mutation instruction': 'set_qualified_revenue_source' not in source,
    'service-unit purchase has no reward split': all(x not in purchase for x in ['split_amount(', 'add_direct(', 'add_network_claimable(', 'accrue_pioneer(']),
    'qualified revenue is separately authenticated': 'qualified_revenue_source' in revenue and 'InvalidRevenueSource' in revenue,
    '10 explicit upline accounts': all(f'upline_{i}' in source for i in range(1, 11)),
    'core canonical ATA derivation exists': 'get_associated_token_address_with_program_id' in source and 'canonical_ata(' in source,
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
    'mainnet treasury and stablecoin constants are frozen': all(x in constants for x in ['MAINNET_USDT_MINT', 'MAINNET_USDC_MINT', 'MAINNET_SERVICE_TREASURY']),
    'core production adapter binding constant exists': 'MAINNET_REVENUE_ADAPTER_PROGRAM' in constants,
    'production launch source/time constants exist': all(x in constants for x in ['MAINNET_QUALIFIED_REVENUE_SOURCE', 'MAINNET_REGISTRATION_OPEN_AT']),
    'production initialization pins immutable mainnet inputs': all(x in source for x in ['validate_initialization_environment(', 'MAINNET_SERVICE_TREASURY', 'MAINNET_USDT_MINT', 'MAINNET_USDC_MINT', 'MAINNET_QUALIFIED_REVENUE_SOURCE', 'MAINNET_REGISTRATION_OPEN_AT', 'ProductionConfigNotFrozen', 'InvalidProductionConfig']),
    'initialization uses typed SPL Mint accounts': "pub usdt_mint: Box<Account<'info, Mint>>" in source and "pub usdc_mint: Box<Account<'info, Mint>>" in source,
    'stablecoin mints require six decimals': source.count('decimals == TOKEN_DECIMALS as u8') >= 2 and 'InvalidTokenDecimals' in source,
    'USDT and USDC mint accounts must differ': 'usdt_mint.key() != ctx.accounts.usdc_mint.key()' in source and 'DuplicateStablecoinMint' in source,
    'service units use global monotonic Unit IDs': all(x in state for x in ['next_unit_id', 'first_unit_id', 'last_unit_id']) and 'allocate_unit_range(ctx.accounts.protocol.next_unit_id, units)' in purchase and 'ctx.accounts.protocol.next_unit_id = next_unit_id' in purchase and 'first_local_unit_index' not in source and 'last_local_unit_index' not in source,

    # Revenue adapter invariants.
    'adapter has no mutable admin/config surface': no_admin_surface(adapter),
    'adapter freezes referral and qualification identities for production': all(x in adapter for x in ['MAINNET_REFERRAL_PROGRAM', 'MAINNET_QUALIFICATION_PROGRAM', 'validate_production_environment']),
    'adapter derives deterministic qualification authority': 'QUALIFIER_AUTHORITY_SEED' in adapter and 'find_program_address' in adapter and 'InvalidQualificationAuthority' in adapter,
    'adapter requires qualification PDA signer on events': "pub qualification_authority: Signer<'info>" in adapter,
    'adapter derives private-keyless revenue authority PDA': 'REVENUE_AUTHORITY_SEED' in adapter and "seeds = [REVENUE_AUTHORITY_SEED]" in adapter,
    'adapter receipt is deterministic and initialize-once': 'RECEIPT_SEED' in adapter and 'seeds = [RECEIPT_SEED, event_id.as_ref()]' in adapter and 'init,' in adapter,
    'adapter binds exact referral executable': 'config.referral_program' in adapter and 'referral_program.executable' in adapter and 'InvalidReferralProgram' in adapter,
    'adapter requires canonical Revenue Authority ATA': 'get_associated_token_address_with_program_id' in adapter and 'NonCanonicalSource' in adapter,
    'adapter requires Revenue Authority ownership': 'config.revenue_authority' in adapter and 'WrongRevenueAuthority' in adapter,
    'adapter requires prefunding before liabilities': 'source_token.amount >= amount' in adapter and 'InsufficientFunding' in adapter,
    'adapter signs referral CPI only with Revenue Authority PDA': 'CpiContext::new_with_signer' in adapter and 'REVENUE_AUTHORITY_SEED' in adapter and 'referral_cpi::record_qualified_revenue' in adapter,
    'adapter supports only frozen USDT/USDC rails': all(x in adapter for x in ['MAINNET_USDT_MINT', 'MAINNET_USDC_MINT', 'UnsupportedMint']),

    # Qualification gateway invariants.
    'qualification gateway has no mutable admin/config surface': no_admin_surface(qualification),
    'qualification gateway requires payer signer': "pub payer: Signer<'info>" in qualification,
    'qualification gateway requires payer token ownership': 'payer_source_token.owner == payer.key()' in qualification and 'WrongPayerTokenOwner' in qualification,
    'qualification gateway performs real SPL transfer before adapter CPI': qualification.index('token::transfer(') < qualification.index('adapter_cpi::submit_revenue_event('),
    'qualification destination is canonical Revenue Authority ATA': 'get_associated_token_address_with_program_id' in qualification and 'NonCanonicalRevenueSource' in qualification,
    'qualification gateway binds adapter config owner': 'owner = revenue_adapter::ID' in qualification,
    'qualification gateway re-derives canonical adapter config PDA': 'expected_adapter_config' in qualification and 'revenue_adapter::CONFIG_SEED' in qualification and 'InvalidAdapterConfig' in qualification,
    'qualification gateway binds exact adapter executable': 'address = revenue_adapter::ID' in qualification and 'adapter_program.executable' in qualification and 'InvalidAdapterProgram' in qualification,
    'qualification gateway preflights immutable referral binding': 'adapter_config.referral_program' in qualification and 'referral_program.executable' in qualification and 'InvalidReferralProgram' in qualification,
    'qualification authority is deterministic PDA': 'QUALIFIER_AUTHORITY_SEED' in qualification and 'seeds = [QUALIFIER_AUTHORITY_SEED]' in qualification,
    'qualification event identity binds evidence payer beneficiary mint amount': all(x in qualification for x in ['evidence_hash.as_ref()', 'payer_key.as_ref()', 'beneficiary_key.as_ref()', 'mint.as_ref()', '&amount_bytes']),
    'qualification recomputes adapter receipt PDA': 'revenue_adapter::RECEIPT_SEED' in qualification and 'InvalidAdapterReceipt' in qualification,
    'qualification signs adapter CPI only with Qualification Authority PDA': 'CpiContext::new_with_signer' in qualification and 'QUALIFIER_AUTHORITY_SEED' in qualification and 'adapter_cpi::submit_revenue_event' in qualification,
    'qualification production adapter binding is fail-closed capable': 'MAINNET_ADAPTER_PROGRAM' in qualification and 'ProductionConfigNotFrozen' in qualification,
    'qualification supports only adapter frozen stablecoin rails': 'adapter_config.usdt_mint' in qualification and 'adapter_config.usdc_mint' in qualification and 'UnsupportedMint' in qualification,
}

for name, ok in checks.items():
    print(('PASS' if ok else 'FAIL'), name)
if not all(checks.values()):
    raise SystemExit(1)
