use anchor_lang::prelude::*;
use anchor_spl::associated_token::get_associated_token_address_with_program_id;
use anchor_spl::token::{self, Token, TokenAccount};
use service_referral_protocol::state::{ProtocolState, UserState};

// Development-only identity. Replace with an offline-generated final Program ID
// during the reviewed mainnet freeze.
declare_id!("Gxf1ikEvwiusChxqj5jkQPvWhFhFw91nyYFNL6oT7ZLD");

pub const CONFIG_SEED: &[u8] = b"adapter-config";
pub const REVENUE_AUTHORITY_SEED: &[u8] = b"revenue-authority";
pub const RECEIPT_SEED: &[u8] = b"revenue-receipt";
pub const VERIFIER_AUTHORITY_SEED: &[u8] = b"qualified-revenue-verifier";

// Fail-closed production sentinel. Replace only when the independently audited
// verifier Program ID is final and immutable.
pub const MAINNET_VERIFIER_PROGRAM: Pubkey = pubkey!("11111111111111111111111111111111");

#[program]
pub mod revenue_adapter {
    use super::*;

    pub fn initialize_adapter(ctx: Context<InitializeAdapter>) -> Result<()> {
        validate_production_verifier(ctx.accounts.verifier_program.key())?;
        require!(
            ctx.accounts.verifier_program.key() != crate::ID,
            AdapterError::InvalidVerifierProgram
        );
        require!(
            ctx.accounts.verifier_program.key() != service_referral_protocol::ID,
            AdapterError::InvalidVerifierProgram
        );

        let now = Clock::get()?.unix_timestamp;
        let config = &mut ctx.accounts.config;
        config.bump = ctx.bumps.config;
        config.revenue_authority_bump = ctx.bumps.revenue_authority;
        config.initialized_at = now;
        config.verifier_program = ctx.accounts.verifier_program.key();
        config.referral_program = ctx.accounts.referral_program.key();
        Ok(())
    }

    pub fn forward_qualified_revenue(
        ctx: Context<ForwardQualifiedRevenue>,
        event_id: [u8; 32],
        evidence_hash: [u8; 32],
        amount: u64,
    ) -> Result<()> {
        require!(amount > 0, AdapterError::ZeroAmount);
        require!(!is_zero_hash(&event_id), AdapterError::InvalidEventId);
        require!(!is_zero_hash(&evidence_hash), AdapterError::InvalidEvidenceHash);

        let config = &ctx.accounts.config;
        require_keys_eq!(
            config.referral_program,
            service_referral_protocol::ID,
            AdapterError::ReferralProgramMismatch
        );
        require_keys_eq!(
            ctx.accounts.referral_program.key(),
            config.referral_program,
            AdapterError::ReferralProgramMismatch
        );

        let expected_verifier = expected_verifier_authority(config.verifier_program);
        require_keys_eq!(
            ctx.accounts.verifier_authority.key(),
            expected_verifier,
            AdapterError::InvalidVerifierAuthority
        );

        let protocol_key = Pubkey::find_program_address(
            &[b"protocol"],
            &service_referral_protocol::ID,
        )
        .0;
        require_keys_eq!(
            ctx.accounts.protocol.key(),
            protocol_key,
            AdapterError::InvalidProtocolState
        );

        let revenue_authority = ctx.accounts.revenue_authority.key();
        let mint = ctx.accounts.source_token.mint;
        require!(
            mint == ctx.accounts.protocol.usdt_mint || mint == ctx.accounts.protocol.usdc_mint,
            AdapterError::UnsupportedToken
        );
        require_keys_eq!(
            ctx.accounts.source_token.owner,
            revenue_authority,
            AdapterError::WrongRevenueAuthority
        );
        let canonical_source = get_associated_token_address_with_program_id(
            &revenue_authority,
            &mint,
            &token::ID,
        );
        require_keys_eq!(
            ctx.accounts.source_token.key(),
            canonical_source,
            AdapterError::NonCanonicalRevenueAccount
        );
        require!(
            ctx.accounts.source_token.amount >= amount,
            AdapterError::InsufficientQualifiedFunds
        );

        let clock = Clock::get()?;
        let receipt = &mut ctx.accounts.receipt;
        receipt.bump = ctx.bumps.receipt;
        receipt.event_id = event_id;
        receipt.evidence_hash = evidence_hash;
        receipt.beneficiary = ctx.accounts.beneficiary.wallet;
        receipt.mint = mint;
        receipt.amount = amount;
        receipt.accepted_at = clock.unix_timestamp;
        receipt.accepted_slot = clock.slot;

        let authority_bump = [config.revenue_authority_bump];
        let authority_seeds: &[&[u8]] = &[REVENUE_AUTHORITY_SEED, &authority_bump];
        let signer_seeds: &[&[&[u8]]] = &[authority_seeds];

        let cpi_accounts = service_referral_protocol::cpi::accounts::RecordQualifiedRevenue {
            revenue_source: ctx.accounts.revenue_authority.to_account_info(),
            protocol: ctx.accounts.protocol.to_account_info(),
            source_token: ctx.accounts.source_token.to_account_info(),
            vault_authority: ctx.accounts.vault_authority.to_account_info(),
            vault_token: ctx.accounts.vault_token.to_account_info(),
            service_treasury_token: ctx.accounts.service_treasury_token.to_account_info(),
            beneficiary: ctx.accounts.beneficiary.to_account_info(),
            upline_1: ctx.accounts.upline_1.to_account_info(),
            upline_2: ctx.accounts.upline_2.to_account_info(),
            upline_3: ctx.accounts.upline_3.to_account_info(),
            upline_4: ctx.accounts.upline_4.to_account_info(),
            upline_5: ctx.accounts.upline_5.to_account_info(),
            upline_6: ctx.accounts.upline_6.to_account_info(),
            upline_7: ctx.accounts.upline_7.to_account_info(),
            upline_8: ctx.accounts.upline_8.to_account_info(),
            upline_9: ctx.accounts.upline_9.to_account_info(),
            upline_10: ctx.accounts.upline_10.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.referral_program.key(),
            cpi_accounts,
            signer_seeds,
        );
        service_referral_protocol::cpi::record_qualified_revenue(cpi_ctx, amount)?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeAdapter<'info> {
    #[account(mut)]
    pub initializer: Signer<'info>,
    #[account(
        init,
        payer = initializer,
        seeds = [CONFIG_SEED],
        bump,
        space = AdapterConfig::SPACE
    )]
    pub config: Account<'info, AdapterConfig>,
    /// CHECK: PDA authority only; no private key and no account data required.
    #[account(seeds = [REVENUE_AUTHORITY_SEED], bump)]
    pub revenue_authority: UncheckedAccount<'info>,
    /// CHECK: executable verifier is stored immutably in AdapterConfig.
    #[account(executable)]
    pub verifier_program: UncheckedAccount<'info>,
    pub referral_program: Program<'info, service_referral_protocol::program::ServiceReferralProtocol>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(event_id: [u8; 32])]
pub struct ForwardQualifiedRevenue<'info> {
    #[account(mut)]
    pub relayer: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, AdapterConfig>,
    pub verifier_authority: Signer<'info>,
    /// CHECK: PDA signer validated by adapter seeds.
    #[account(
        seeds = [REVENUE_AUTHORITY_SEED],
        bump = config.revenue_authority_bump
    )]
    pub revenue_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub source_token: Box<Account<'info, TokenAccount>>,
    #[account(
        init,
        payer = relayer,
        seeds = [RECEIPT_SEED, event_id.as_ref()],
        bump,
        space = RevenueReceipt::SPACE
    )]
    pub receipt: Account<'info, RevenueReceipt>,
    #[account(mut)]
    pub protocol: Box<Account<'info, ProtocolState>>,
    /// CHECK: referral program validates PDA seeds and bump.
    pub vault_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub vault_token: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub service_treasury_token: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub beneficiary: Box<Account<'info, UserState>>,
    /// CHECK: referral protocol verifies immutable ancestry dynamically.
    #[account(mut)]
    pub upline_1: UncheckedAccount<'info>,
    /// CHECK: referral protocol verifies immutable ancestry dynamically.
    #[account(mut)]
    pub upline_2: UncheckedAccount<'info>,
    /// CHECK: referral protocol verifies immutable ancestry dynamically.
    #[account(mut)]
    pub upline_3: UncheckedAccount<'info>,
    /// CHECK: referral protocol verifies immutable ancestry dynamically.
    #[account(mut)]
    pub upline_4: UncheckedAccount<'info>,
    /// CHECK: referral protocol verifies immutable ancestry dynamically.
    #[account(mut)]
    pub upline_5: UncheckedAccount<'info>,
    /// CHECK: referral protocol verifies immutable ancestry dynamically.
    #[account(mut)]
    pub upline_6: UncheckedAccount<'info>,
    /// CHECK: referral protocol verifies immutable ancestry dynamically.
    #[account(mut)]
    pub upline_7: UncheckedAccount<'info>,
    /// CHECK: referral protocol verifies immutable ancestry dynamically.
    #[account(mut)]
    pub upline_8: UncheckedAccount<'info>,
    /// CHECK: referral protocol verifies immutable ancestry dynamically.
    #[account(mut)]
    pub upline_9: UncheckedAccount<'info>,
    /// CHECK: referral protocol verifies immutable ancestry dynamically.
    #[account(mut)]
    pub upline_10: UncheckedAccount<'info>,
    pub referral_program: Program<'info, service_referral_protocol::program::ServiceReferralProtocol>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct AdapterConfig {
    pub bump: u8,
    pub revenue_authority_bump: u8,
    pub initialized_at: i64,
    pub verifier_program: Pubkey,
    pub referral_program: Pubkey,
}

impl AdapterConfig {
    pub const SPACE: usize = 8 + 1 + 1 + 8 + 32 + 32;
}

#[account]
pub struct RevenueReceipt {
    pub bump: u8,
    pub event_id: [u8; 32],
    pub evidence_hash: [u8; 32],
    pub beneficiary: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub accepted_at: i64,
    pub accepted_slot: u64,
}

impl RevenueReceipt {
    pub const SPACE: usize = 8 + 1 + 32 + 32 + 32 + 32 + 8 + 8 + 8;
}

fn expected_verifier_authority(verifier_program: Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[VERIFIER_AUTHORITY_SEED], &verifier_program).0
}

fn is_zero_hash(value: &[u8; 32]) -> bool {
    value.iter().all(|byte| *byte == 0)
}

fn validate_production_verifier(verifier_program: Pubkey) -> Result<()> {
    #[cfg(feature = "production")]
    {
        require!(
            MAINNET_VERIFIER_PROGRAM != Pubkey::default(),
            AdapterError::ProductionVerifierNotFrozen
        );
        require_keys_eq!(
            verifier_program,
            MAINNET_VERIFIER_PROGRAM,
            AdapterError::InvalidProductionVerifier
        );
    }
    #[cfg(not(feature = "production"))]
    {
        let _ = verifier_program;
    }
    Ok(())
}

#[error_code]
pub enum AdapterError {
    #[msg("Amount must be greater than zero")]
    ZeroAmount,
    #[msg("Revenue event ID must be non-zero")]
    InvalidEventId,
    #[msg("Evidence hash must be non-zero")]
    InvalidEvidenceHash,
    #[msg("Verifier program is invalid")]
    InvalidVerifierProgram,
    #[msg("Verifier PDA signer is invalid")]
    InvalidVerifierAuthority,
    #[msg("Referral program does not match frozen adapter configuration")]
    ReferralProgramMismatch,
    #[msg("Referral protocol state PDA is invalid")]
    InvalidProtocolState,
    #[msg("Qualified revenue token is unsupported")]
    UnsupportedToken,
    #[msg("Qualified revenue token account is not owned by RevenueAuthority")]
    WrongRevenueAuthority,
    #[msg("Qualified revenue source token account is not the canonical ATA")]
    NonCanonicalRevenueAccount,
    #[msg("RevenueAuthority ATA is not sufficiently funded")]
    InsufficientQualifiedFunds,
    #[msg("Production verifier Program ID has not been frozen")]
    ProductionVerifierNotFrozen,
    #[msg("Production verifier does not match frozen Program ID")]
    InvalidProductionVerifier,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_hash_detection_is_exact() {
        assert!(is_zero_hash(&[0u8; 32]));
        let mut nonzero = [0u8; 32];
        nonzero[31] = 1;
        assert!(!is_zero_hash(&nonzero));
    }

    #[test]
    fn verifier_authority_is_deterministic_and_program_scoped() {
        let first = expected_verifier_authority(service_referral_protocol::ID);
        let second = expected_verifier_authority(service_referral_protocol::ID);
        let other = expected_verifier_authority(crate::ID);
        assert_eq!(first, second);
        assert_ne!(first, other);
    }

    #[test]
    fn account_sizes_include_anchor_discriminator() {
        assert_eq!(AdapterConfig::SPACE, 82);
        assert_eq!(RevenueReceipt::SPACE, 161);
    }
}
