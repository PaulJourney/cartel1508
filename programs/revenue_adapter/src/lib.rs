use anchor_lang::prelude::*;
use anchor_spl::associated_token::get_associated_token_address_with_program_id;
use anchor_spl::token::{self, Token, TokenAccount};
use service_referral_protocol::cpi as referral_cpi;

declare_id!("EmGJDPvwSx6kU4KijWGh8uqRNj3BJXNjcWQyCmfKv7WL");

pub const QUALIFIER_AUTHORITY_SEED: &[u8] = b"qualified-revenue-authority";
pub const REVENUE_AUTHORITY_SEED: &[u8] = b"revenue-authority";
pub const CONFIG_SEED: &[u8] = b"adapter-config";
pub const RECEIPT_SEED: &[u8] = b"revenue-receipt";

// Production launch values are deliberately fail-closed until the final reviewed freeze.
pub const MAINNET_REFERRAL_PROGRAM: Pubkey = Pubkey::new_from_array([0u8; 32]);
pub const MAINNET_QUALIFICATION_PROGRAM: Pubkey = Pubkey::new_from_array([0u8; 32]);
pub const MAINNET_USDT_MINT: Pubkey = pubkey!("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB");
pub const MAINNET_USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");

#[program]
pub mod revenue_adapter {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        referral_program: Pubkey,
        qualification_program: Pubkey,
        usdt_mint: Pubkey,
        usdc_mint: Pubkey,
    ) -> Result<()> {
        require!(referral_program != Pubkey::default(), AdapterError::InvalidProgram);
        require!(qualification_program != Pubkey::default(), AdapterError::InvalidProgram);
        require!(referral_program != qualification_program, AdapterError::InvalidProgram);
        require!(usdt_mint != usdc_mint, AdapterError::DuplicateMint);
        validate_production_environment(referral_program, qualification_program, usdt_mint, usdc_mint)?;

        let expected_qualifier = Pubkey::find_program_address(
            &[QUALIFIER_AUTHORITY_SEED],
            &qualification_program,
        ).0;
        require_keys_eq!(
            ctx.accounts.qualification_authority.key(),
            expected_qualifier,
            AdapterError::InvalidQualificationAuthority
        );

        let config = &mut ctx.accounts.config;
        config.bump = ctx.bumps.config;
        config.revenue_authority_bump = ctx.bumps.revenue_authority;
        config.referral_program = referral_program;
        config.qualification_program = qualification_program;
        config.qualification_authority = expected_qualifier;
        config.revenue_authority = ctx.accounts.revenue_authority.key();
        config.usdt_mint = usdt_mint;
        config.usdc_mint = usdc_mint;
        config.initialized_at = Clock::get()?.unix_timestamp;
        Ok(())
    }

    pub fn submit_revenue_event(
        ctx: Context<SubmitRevenueEvent>,
        event_id: [u8; 32],
        amount: u64,
    ) -> Result<()> {
        require!(amount > 0, AdapterError::ZeroAmount);
        require!(event_id != [0u8; 32], AdapterError::InvalidEventId);

        let config = &ctx.accounts.config;
        require_keys_eq!(
            ctx.accounts.qualification_authority.key(),
            config.qualification_authority,
            AdapterError::InvalidQualificationAuthority
        );
        require_keys_eq!(
            ctx.accounts.referral_program.key(),
            config.referral_program,
            AdapterError::InvalidReferralProgram
        );
        require!(ctx.accounts.referral_program.executable, AdapterError::InvalidReferralProgram);

        let mint = ctx.accounts.source_token.mint;
        require!(mint == config.usdt_mint || mint == config.usdc_mint, AdapterError::UnsupportedMint);
        require_keys_eq!(
            ctx.accounts.source_token.owner,
            config.revenue_authority,
            AdapterError::WrongRevenueAuthority
        );
        let expected_source = get_associated_token_address_with_program_id(
            &config.revenue_authority,
            &mint,
            &token::ID,
        );
        require_keys_eq!(
            ctx.accounts.source_token.key(),
            expected_source,
            AdapterError::NonCanonicalSource
        );
        require!(ctx.accounts.source_token.amount >= amount, AdapterError::InsufficientFunding);

        let receipt = &mut ctx.accounts.receipt;
        receipt.bump = ctx.bumps.receipt;
        receipt.event_id = event_id;
        receipt.beneficiary = ctx.accounts.beneficiary.key();
        receipt.mint = mint;
        receipt.amount = amount;
        receipt.accepted_at = Clock::get()?.unix_timestamp;
        receipt.accepted_slot = Clock::get()?.slot;
        receipt.qualifier = ctx.accounts.qualification_authority.key();

        let signer_bump = [config.revenue_authority_bump];
        let signer_seeds: &[&[u8]] = &[REVENUE_AUTHORITY_SEED, &signer_bump];

        let cpi_accounts = referral_cpi::accounts::RecordQualifiedRevenue {
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

        referral_cpi::record_qualified_revenue(
            CpiContext::new_with_signer(
                ctx.accounts.referral_program.key(),
                cpi_accounts,
                &[signer_seeds],
            ),
            amount,
        )?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
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
    /// CHECK: deterministic PDA; no private key exists.
    #[account(seeds = [REVENUE_AUTHORITY_SEED], bump)]
    pub revenue_authority: UncheckedAccount<'info>,
    /// CHECK: must equal PDA derived under the frozen qualification program.
    pub qualification_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(event_id: [u8; 32])]
pub struct SubmitRevenueEvent<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub qualification_authority: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, AdapterConfig>,
    /// CHECK: adapter PDA signs only the downstream referral CPI.
    #[account(seeds = [REVENUE_AUTHORITY_SEED], bump = config.revenue_authority_bump)]
    pub revenue_authority: UncheckedAccount<'info>,
    #[account(
        init,
        payer = payer,
        seeds = [RECEIPT_SEED, event_id.as_ref()],
        bump,
        space = RevenueReceipt::SPACE
    )]
    pub receipt: Account<'info, RevenueReceipt>,
    #[account(mut)]
    pub source_token: Account<'info, TokenAccount>,

    /// CHECK: exact executable address is frozen in AdapterConfig.
    pub referral_program: UncheckedAccount<'info>,
    /// CHECK: validated by the downstream referral program.
    #[account(mut)]
    pub protocol: UncheckedAccount<'info>,
    /// CHECK: validated by the downstream referral program.
    pub vault_authority: UncheckedAccount<'info>,
    /// CHECK: validated by the downstream referral program.
    #[account(mut)]
    pub vault_token: UncheckedAccount<'info>,
    /// CHECK: validated by the downstream referral program.
    #[account(mut)]
    pub service_treasury_token: UncheckedAccount<'info>,
    /// CHECK: validated by the downstream referral program.
    #[account(mut)]
    pub beneficiary: UncheckedAccount<'info>,
    /// CHECK: validated by immutable ancestry in the downstream referral program.
    #[account(mut)] pub upline_1: UncheckedAccount<'info>,
    /// CHECK: validated by immutable ancestry in the downstream referral program.
    #[account(mut)] pub upline_2: UncheckedAccount<'info>,
    /// CHECK: validated by immutable ancestry in the downstream referral program.
    #[account(mut)] pub upline_3: UncheckedAccount<'info>,
    /// CHECK: validated by immutable ancestry in the downstream referral program.
    #[account(mut)] pub upline_4: UncheckedAccount<'info>,
    /// CHECK: validated by immutable ancestry in the downstream referral program.
    #[account(mut)] pub upline_5: UncheckedAccount<'info>,
    /// CHECK: validated by immutable ancestry in the downstream referral program.
    #[account(mut)] pub upline_6: UncheckedAccount<'info>,
    /// CHECK: validated by immutable ancestry in the downstream referral program.
    #[account(mut)] pub upline_7: UncheckedAccount<'info>,
    /// CHECK: validated by immutable ancestry in the downstream referral program.
    #[account(mut)] pub upline_8: UncheckedAccount<'info>,
    /// CHECK: validated by immutable ancestry in the downstream referral program.
    #[account(mut)] pub upline_9: UncheckedAccount<'info>,
    /// CHECK: validated by immutable ancestry in the downstream referral program.
    #[account(mut)] pub upline_10: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct AdapterConfig {
    pub bump: u8,
    pub revenue_authority_bump: u8,
    pub referral_program: Pubkey,
    pub qualification_program: Pubkey,
    pub qualification_authority: Pubkey,
    pub revenue_authority: Pubkey,
    pub usdt_mint: Pubkey,
    pub usdc_mint: Pubkey,
    pub initialized_at: i64,
}

impl AdapterConfig {
    pub const SPACE: usize = 8 + 1 + 1 + (32 * 6) + 8;
}

#[account]
pub struct RevenueReceipt {
    pub bump: u8,
    pub event_id: [u8; 32],
    pub beneficiary: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub accepted_at: i64,
    pub accepted_slot: u64,
    pub qualifier: Pubkey,
}

impl RevenueReceipt {
    pub const SPACE: usize = 8 + 1 + 32 + 32 + 32 + 8 + 8 + 8 + 32;
}

fn validate_production_environment(
    referral_program: Pubkey,
    qualification_program: Pubkey,
    usdt_mint: Pubkey,
    usdc_mint: Pubkey,
) -> Result<()> {
    #[cfg(feature = "production")]
    {
        require!(
            MAINNET_REFERRAL_PROGRAM != Pubkey::default()
                && MAINNET_QUALIFICATION_PROGRAM != Pubkey::default(),
            AdapterError::ProductionConfigNotFrozen
        );
        require_keys_eq!(referral_program, MAINNET_REFERRAL_PROGRAM, AdapterError::InvalidProductionConfig);
        require_keys_eq!(qualification_program, MAINNET_QUALIFICATION_PROGRAM, AdapterError::InvalidProductionConfig);
        require_keys_eq!(usdt_mint, MAINNET_USDT_MINT, AdapterError::InvalidProductionConfig);
        require_keys_eq!(usdc_mint, MAINNET_USDC_MINT, AdapterError::InvalidProductionConfig);
    }
    #[cfg(not(feature = "production"))]
    {
        let _ = (referral_program, qualification_program, usdt_mint, usdc_mint);
    }
    Ok(())
}

#[error_code]
pub enum AdapterError {
    #[msg("Invalid program identity")] InvalidProgram,
    #[msg("USDT and USDC mint identities must differ")] DuplicateMint,
    #[msg("Qualification authority is not the deterministic PDA of the frozen qualification program")] InvalidQualificationAuthority,
    #[msg("Referral program does not match the immutable adapter configuration")] InvalidReferralProgram,
    #[msg("Revenue event amount must be greater than zero")] ZeroAmount,
    #[msg("Revenue event ID must be non-zero")] InvalidEventId,
    #[msg("Unsupported stablecoin mint")] UnsupportedMint,
    #[msg("Revenue source token account is not owned by the adapter revenue authority PDA")] WrongRevenueAuthority,
    #[msg("Revenue source token account is not the canonical ATA")] NonCanonicalSource,
    #[msg("Revenue authority ATA is not sufficiently funded before accounting")] InsufficientFunding,
    #[msg("Production referral/qualification identities have not been frozen")] ProductionConfigNotFrozen,
    #[msg("Production initialization does not match frozen adapter configuration")] InvalidProductionConfig,
}
