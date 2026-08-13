from pathlib import Path

constants_path = Path('programs/service_referral_protocol/src/constants.rs')
lib_path = Path('programs/service_referral_protocol/src/lib.rs')
gates_path = Path('scripts/static-gates.py')

constants = constants_path.read_text()
if 'MAINNET_QUALIFIED_REVENUE_SOURCE' not in constants:
    anchor = 'pub const MAINNET_SERVICE_TREASURY: Pubkey = pubkey!("AepYo8xanmKuRiLVeYQuCTJoQr1nyKiTApoKwHMEg8fn");\n'
    if anchor not in constants:
        raise SystemExit('mainnet treasury constant anchor missing')
    constants = constants.replace(
        anchor,
        anchor
        + '// Fail-closed launch sentinels. Replace only during the final reviewed mainnet freeze.\n'
        + 'pub const MAINNET_QUALIFIED_REVENUE_SOURCE: Pubkey = pubkey!("11111111111111111111111111111111");\n'
        + 'pub const MAINNET_REGISTRATION_OPEN_AT: i64 = 0;\n',
        1,
    )
    constants_path.write_text(constants)

source = lib_path.read_text()
if 'validate_initialization_environment(' not in source:
    anchor = '        require!(constants::percentages_valid(), ProtocolError::InvalidPercentages);\n'
    if anchor not in source:
        raise SystemExit('initialize percentage guard anchor missing')
    source = source.replace(
        anchor,
        anchor
        + '        validate_initialization_environment(\n'
        + '            ctx.accounts.service_treasury.key(),\n'
        + '            ctx.accounts.usdt_mint.key(),\n'
        + '            ctx.accounts.usdc_mint.key(),\n'
        + '            qualified_revenue_source,\n'
        + '            registration_open_at,\n'
        + '        )?;\n',
        1,
    )

helper_marker = 'fn validate_initialization_environment('
if helper_marker not in source[source.find('}\n\n#[derive(Accounts)]') if '}\n\n#[derive(Accounts)]' in source else 0:]:
    helper = '''\nfn validate_initialization_environment(\n    service_treasury: Pubkey,\n    usdt_mint: Pubkey,\n    usdc_mint: Pubkey,\n    qualified_revenue_source: Pubkey,\n    registration_open_at: i64,\n) -> Result<()> {\n    #[cfg(feature = "production")]\n    {\n        require!(\n            MAINNET_QUALIFIED_REVENUE_SOURCE != Pubkey::default()\n                && MAINNET_REGISTRATION_OPEN_AT > 0,\n            ProtocolError::ProductionConfigNotFrozen\n        );\n        require_keys_eq!(service_treasury, MAINNET_SERVICE_TREASURY, ProtocolError::InvalidProductionConfig);\n        require_keys_eq!(usdt_mint, MAINNET_USDT_MINT, ProtocolError::InvalidProductionConfig);\n        require_keys_eq!(usdc_mint, MAINNET_USDC_MINT, ProtocolError::InvalidProductionConfig);\n        require_keys_eq!(\n            qualified_revenue_source,\n            MAINNET_QUALIFIED_REVENUE_SOURCE,\n            ProtocolError::InvalidProductionConfig\n        );\n        require!(\n            registration_open_at == MAINNET_REGISTRATION_OPEN_AT,\n            ProtocolError::InvalidProductionConfig\n        );\n    }\n    #[cfg(not(feature = "production"))]\n    {\n        let _ = (\n            service_treasury,\n            usdt_mint,\n            usdc_mint,\n            qualified_revenue_source,\n            registration_open_at,\n        );\n    }\n    Ok(())\n}\n'''
    error_anchor = '\n#[error_code]\npub enum ProtocolError {'
    if error_anchor not in source:
        raise SystemExit('ProtocolError anchor missing')
    source = source.replace(error_anchor, helper + error_anchor, 1)

if '    ProductionConfigNotFrozen,' not in source or '    InvalidProductionConfig,' not in source:
    error_anchor = '    #[msg("Nothing to claim")] NothingToClaim,\n'
    if error_anchor not in source:
        raise SystemExit('ProtocolError tail anchor missing')
    source = source.replace(
        error_anchor,
        error_anchor
        + '    #[msg("Production source/time configuration has not been frozen")] ProductionConfigNotFrozen,\n'
        + '    #[msg("Production initialization does not match frozen mainnet configuration")] InvalidProductionConfig,\n',
        1,
    )

lib_path.write_text(source)

gates = gates_path.read_text()
if "production initialization pins immutable mainnet inputs" not in gates:
    anchor = "    'legacy token program is fixed': 'token::ID' in source,\n"
    if anchor not in gates:
        raise SystemExit('static gate insertion anchor missing')
    gates = gates.replace(
        anchor,
        anchor
        + "    'mainnet treasury and stablecoin constants are frozen': all(x in constants for x in ['MAINNET_USDT_MINT', 'MAINNET_USDC_MINT', 'MAINNET_SERVICE_TREASURY']),\n"
        + "    'production launch source/time constants exist': all(x in constants for x in ['MAINNET_QUALIFIED_REVENUE_SOURCE', 'MAINNET_REGISTRATION_OPEN_AT']),\n"
        + "    'production initialization pins immutable mainnet inputs': all(x in source for x in ['validate_initialization_environment(', 'MAINNET_SERVICE_TREASURY', 'MAINNET_USDT_MINT', 'MAINNET_USDC_MINT', 'MAINNET_QUALIFIED_REVENUE_SOURCE', 'MAINNET_REGISTRATION_OPEN_AT', 'ProductionConfigNotFrozen', 'InvalidProductionConfig']),\n",
        1,
    )
    gates_path.write_text(gates)

print('production guard migration ready')
