from pathlib import Path

lib_path = Path('programs/service_referral_protocol/src/lib.rs')
gates_path = Path('scripts/static-gates.py')

source = lib_path.read_text()

old_import = 'use anchor_spl::token::{self, Token, TokenAccount, Transfer};'
new_import = 'use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};'
if old_import in source:
    source = source.replace(old_import, new_import, 1)
elif new_import not in source:
    raise SystemExit('anchor_spl token import anchor missing')

check_marker = '        require!(constants::percentages_valid(), ProtocolError::InvalidPercentages);\n'
checks = (
    '        require!(ctx.accounts.usdt_mint.decimals == TOKEN_DECIMALS as u8, ProtocolError::InvalidTokenDecimals);\n'
    '        require!(ctx.accounts.usdc_mint.decimals == TOKEN_DECIMALS as u8, ProtocolError::InvalidTokenDecimals);\n'
    '        require!(ctx.accounts.usdt_mint.key() != ctx.accounts.usdc_mint.key(), ProtocolError::DuplicateStablecoinMint);\n'
)
if 'ProtocolError::InvalidTokenDecimals' not in source[source.find('pub fn initialize'):source.find('pub fn register')]:
    if check_marker not in source:
        raise SystemExit('initialize validation anchor missing')
    source = source.replace(check_marker, check_marker + checks, 1)

old_accounts = (
    '    /// CHECK: mint key stored in ProtocolState.\n'
    "    pub usdt_mint: UncheckedAccount<'info>,\n"
    '    /// CHECK: mint key stored in ProtocolState.\n'
    "    pub usdc_mint: UncheckedAccount<'info>,"
)
new_accounts = (
    '    /// Canonical SPL mint; initialization also enforces six decimals.\n'
    "    pub usdt_mint: Box<Account<'info, Mint>>,\n"
    '    /// Canonical SPL mint; initialization also enforces six decimals.\n'
    "    pub usdc_mint: Box<Account<'info, Mint>>," 
)
if old_accounts in source:
    source = source.replace(old_accounts, new_accounts, 1)
elif "pub usdt_mint: Box<Account<'info, Mint>>" not in source or "pub usdc_mint: Box<Account<'info, Mint>>" not in source:
    raise SystemExit('Initialize mint account anchors missing')

if '    InvalidTokenDecimals,' not in source or '    DuplicateStablecoinMint,' not in source:
    error_anchor = '    #[msg("Production initialization does not match frozen mainnet configuration")] InvalidProductionConfig,\n'
    if error_anchor not in source:
        raise SystemExit('ProtocolError production tail anchor missing')
    source = source.replace(
        error_anchor,
        error_anchor
        + '    #[msg("Supported stablecoin mint must use six decimals")] InvalidTokenDecimals,\n'
        + '    #[msg("USDT and USDC mint accounts must be distinct")] DuplicateStablecoinMint,\n',
        1,
    )

lib_path.write_text(source)

gates = gates_path.read_text()
if "initialization uses typed SPL Mint accounts" not in gates:
    anchor = "    'production initialization pins immutable mainnet inputs': all(x in source for x in ['validate_initialization_environment(', 'MAINNET_SERVICE_TREASURY', 'MAINNET_USDT_MINT', 'MAINNET_USDC_MINT', 'MAINNET_QUALIFIED_REVENUE_SOURCE', 'MAINNET_REGISTRATION_OPEN_AT', 'ProductionConfigNotFrozen', 'InvalidProductionConfig']),\n"
    if anchor not in gates:
        raise SystemExit('static-gate production anchor missing')
    gates = gates.replace(
        anchor,
        anchor
        + "    'initialization uses typed SPL Mint accounts': \"pub usdt_mint: Box<Account<'info, Mint>>\" in source and \"pub usdc_mint: Box<Account<'info, Mint>>\" in source,\n"
        + "    'stablecoin mints require six decimals': source.count('decimals == TOKEN_DECIMALS as u8') >= 2 and 'InvalidTokenDecimals' in source,\n"
        + "    'USDT and USDC mint accounts must differ': 'usdt_mint.key() != ctx.accounts.usdc_mint.key()' in source and 'DuplicateStablecoinMint' in source,\n",
        1,
    )
    gates_path.write_text(gates)

print('SPL mint initialization hardening migration ready')
