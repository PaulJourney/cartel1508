use anchor_lang::prelude::*;
use anchor_spl::associated_token::get_associated_token_address_with_program_id;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use revenue_adapter::cpi;

declare_id!("AZQHbWahShE5oLKG3BXCrqmoMj6WWtiuxhnCcMp4YoE4");

pub const CONFIG_SEED: &[u8] = b"qualification-config";
pub const QUALIFIER_AUTHORITY_SEED: &[u8] = b"qualified-revenue-authority";

// Fail-closed production sentinel. Freeze this to the reviewed adapter Program ID
// only during the final mainnet release ceremony.
pub const MAINNET_ADAPTER_PROGRAM: Pubkey = pubkey!("11111111111111111111111111111111");
pub const MAINNET_USDT_MINT: Pubkey = pubkey!("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB");
pub const MAINNET_USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");

#[program]
pub mod revenue_qualification {
    use super::*;

    /// One-shot immutable configuration. The adapter must already be deployed and
    /// initialized; no update/admin instruction exists after this account is created.
    pub fn initialize(
        ctx: Context<Initialize>,
        usdt_mint: Pubkey,
        usdc_mint: Pubkey,
    ) -> Result<()> {
        require!(usdt_mint != usdc_mint, QualificationError::DuplicateMint);
        require_keys_neq!(ctx.accounts.adapter_program.key(), crate::ID, QualificationError::InvalidAdapterProgram);
        require!(ctx.accounts.adapter_program.executable, QualificationError::InvalidAdapterProgram);

        let adapter_program = ctx.accounts.adapter_program.key();
        let expected_adapter_config = Pubkey::find_program_address(
            &[revenue_adapter::CONFIG_SEED],
            &adapter_program,
        ).0;
        require_keys_eq!(
            ctx.accounts.adapter_config.key(),
            expected_adapter_config,
            QualificationError::InvalidAdapterConfig
        );
        require_keys_eq!(
            *ctx.accounts.adapter_config.owner,
            adapter_program,
            QualificationError::InvalidAdapterConfig
        );

        let revenue_authority = Pubkey::find_program_address(
            &[revenue_adapter::REVENUE_AUTHORITY_SEED],
            &adapter_program,
        ).0;

        #[cfg(feature = "production")]
        {
            require!(
                MAINNET_ADAPTER_PROGRAM != Pubkey::default(),
                QualificationError::ProductionNotFrozen
            );
            require_keys_eq!(
                adapter_program,
                MAINNET_ADAPTER_PROGRAM,
                QualificationError::ProductionIdentityMismatch
            );
            require_keys_eq!(
                usdt_mint,
                MAINNET_USDT_MINT,
                QualificationError::ProductionIdentityMismatch
            );
            require_keys_eq!(
                usdc_mint,
                MAINNET_USDC_MINT,
                QualificationError::ProductionIdentityMismatch
            );
        }

        let config = &mut ctx.accounts.config;
        config.bump = ctx.bumps.config;
        config.qualifier_authority_bump = ctx.bumps.qualification_authority;
        config.adapter_program = adapter_program;
        config.adapter_config = expected_adapter_config;
        config.revenue_authority = revenue_authority;
        config.usdt_mint = usdt_mint;
        config.usdc_mint = usdc_mint;
        config.initialized_at = Clock::get()?.unix_timestamp;
        Ok(())
    }

    /// A revenue event is qualified only by an actual stablecoin payment made in
    /// this same atomic transaction. If any downstream adapter/referral check fails,
    /// Solana rolls back both the payment and all receipt/accounting mutations.
    pub fn pay_and_qualify<'info>(
        ctx: Context<'_, '_, '_, 'info, PayAndQualify<'info>>,
        event_id: [u8; 32],
        amount: u64,
    ) -> Result<()> {
        require!(amount > 0, QualificationError::ZeroAmount);
        require!(event_id != [0u8; 32], QualificationError::InvalidEventId);

        let config = &ctx.accounts.config;
        require_keys_eq!(
            ctx.accounts.adapter_program.key(),
            config.adapter_program,
            QualificationError::InvalidAdapterProgram
        );
        require!(ctx.accounts.adapter_program.executable, QualificationError::InvalidAdapterProgram);
        require_keys_eq!(
            ctx.accounts.adapter_config.key(),
            config.adapter_config,
            QualificationError::InvalidAdapterConfig
        );
        require_keys_eq!(
            ctx.accounts.adapter_revenue_authority.key(),
            config.revenue_authority,
            QualificationError::InvalidRevenueAuthority
        );

        let mint = ctx.accounts.payer_token.mint;
        require!(
            mint == config.usdt_mint || mint == config.usdc_mint,
            QualificationError::UnsupportedMint
        );
        require_keys_eq!(
            ctx.accounts.payer_token.owner,
            ctx.accounts.payer.key(),
            QualificationError::WrongPayerAuthority
        );
        require_keys_eq!(
            ctx.accounts.revenue_token.owner,
            config.revenue_authority,
            QualificationError::InvalidRevenueAuthority
        );
        require_keys_eq!(
            ctx.accounts.revenue_token.mint,
            mint,
            QualificationError::MintMismatch
        );

        let expected_payer_ata = get_associated_token_address_with_program_id(
            &ctx.accounts.payer.key(),
            &mint,
            &token::ID,
        );
        require_keys_eq!(
            ctx.accounts.payer_token.key(),
            expected_payer_ata,
            QualificationError::NonCanonicalPayerSource
        );
        let expected_revenue_ata = get_associated_token_address_with_program_id(
            &config.revenue_authority,
            &mint,
            &token::ID,
        );
        require_keys_eq!(
            ctx.accounts.revenue_token.key(),
            expected_revenue_ata,
            QualificationError::NonCanonicalRevenueDestination
        );
        require!(
            ctx.accounts.payer_token.amount >= amount,
            QualificationError::InsufficientPaymentBalance
        );

        token::transfer(
            CpiContext::new(
                token::ID,
                Transfer {
                    from: ctx.accounts.payer_token.to_account_info(),
                    to: ctx.accounts.revenue_token.to_account_info(),
                    authority: ctx.accounts.payer.to_account_info(),
                },
            ),
            amount,
        )?;

        let cpi_accounts = cpi::accounts::SubmitRevenueEvent {
            payer: ctx.accounts.payer.to_account_info(),
            qualification_authority: ctx.accounts.qualification_authority.to_account_info(),
            config: ctx.accounts.adapter_config.to_account_info(),
            revenue_authority: ctx.accounts.adapter_revenue_authority.to_account_info(),
            receipt: ctx.accounts.adapter_receipt.to_account_info(),
            source_token: ctx.accounts.revenue_token.to_account_info(),
            referral_program: ctx.accounts.referral_program.to_account_info(),
            protocol: ctx.accounts.protocol.to_account_info(),
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
            system_program: ctx.accounts.system_program.to_account_info(),
        };

        let bump = [config.qualifier_authority_bump];
        let signer_seeds: &[&[u8]] = &[QUALIFIER_AUTHORITY_SEED, &bump];
        cpi::submit_revenue_event(
            CpiContext::new_with_signer(
                ctx.accounts.adapter_program.key(),
                cpi_accounts,
                &[signer_seeds],
            ),
            event_id,
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
        space = QualificationConfig::SPACE
    )]
    pub config: Account<'info, QualificationConfig>,
    /// CHECK: deterministic program signer with no private key.
    #[account(seeds = [QUALIFIER_AUTHORITY_SEED], bump)]
    pub qualification_authority: UncheckedAccount<'info>,
    /// CHECK: checked executable and frozen into config.
    pub adapter_program: UncheckedAccount<'info>,
    /// CHECK: canonical PDA and owner validated in initialize.
    pub adapter_config: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(event_id: [u8; 32])]
pub struct PayAndQualify<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, QualificationConfig>,
    /// CHECK: PDA signs only the adapter CPI.
    #[account(seeds = [QUALIFIER_AUTHORITY_SEED], bump = config.qualifier_authority_bump)]
    pub qualification_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub payer_token: Account<'info, TokenAccount>,
    #[account(mut)]
    pub revenue_token: Account<'info, TokenAccount>,

    /// CHECK: exact executable address frozen in QualificationConfig.
    pub adapter_program: UncheckedAccount<'info>,
    /// CHECK: exact PDA frozen in QualificationConfig and validated by adapter.
    pub adapter_config: UncheckedAccount<'info>,
    /// CHECK: exact adapter PDA frozen in QualificationConfig.
    pub adapter_revenue_authority: UncheckedAccount<'info>,
    /// CHECK: initialized atomically by adapter; event_id seeds are enforced there.
    #[account(mut)]
    pub adapter_receipt: UncheckedAccount<'info>,

    /// CHECK: validated and frozen by adapter/referral protocol.
    pub referral_program: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    #[account(mut)]
    pub protocol: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    pub vault_authority: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    #[account(mut)]
    pub vault_token: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    #[account(mut)]
    pub service_treasury_token: UncheckedAccount<'info>,
    /// CHECK: validated as a real UserState PDA by referral protocol.
    #[account(mut)]
    pub beneficiary: UncheckedAccount<'info>,
    /// CHECK: immutable ancestry validated downstream.
    #[account(mut)] pub upline_1: UncheckedAccount<'info>,
    /// CHECK: immutable ancestry validated downstream.
    #[account(mut)] pub upline_2: UncheckedAccount<'info>,
    /// CHECK: immutable ancestry validated downstream.
    #[account(mut)] pub upline_3: UncheckedAccount<'info>,
    /// CHECK: immutable ancestry validated downstream.
    #[account(mut)] pub upline_4: UncheckedAccount<'info>,
    /// CHECK: immutable ancestry validated downstream.
    #[account(mut)] pub upline_5: UncheckedAccount<'info>,
    /// CHECK: immutable ancestry validated downstream.
    #[account(mut)] pub upline_6: UncheckedAccount<'info>,
    /// CHECK: immutable ancestry validated downstream.
    #[account(mut)] pub upline_7: UncheckedAccount<'info>,
    /// CHECK: immutable ancestry validated downstream.
    #[account(mut)] pub upline_8: UncheckedAccount<'info>,
    /// CHECK: immutable ancestry validated downstream.
    #[account(mut)] pub upline_9: UncheckedAccount<'info>,
    /// CHECK: immutable ancestry validated downstream.
    #[account(mut)] pub upline_10: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct QualificationConfig {
    pub bump: u8,
    pub qualifier_authority_bump: u8,
    pub adapter_program: Pubkey,
    pub adapter_config: Pubkey,
    pub revenue_authority: Pubkey,
    pub usdt_mint: Pubkey,
    pub usdc_mint: Pubkey,
    pub initialized_at: i64,
}

impl QualificationConfig {
    pub const SPACE: usize = 8 + 1 + 1 + (32 * 5) + 8;
}

#[error_code]
pub enum QualificationError {
    #[msg("USDT and USDC mint identities must differ")]
    DuplicateMint,
    #[msg("Adapter program is invalid")]
    InvalidAdapterProgram,
    #[msg("Adapter config is not the canonical owned PDA")]
    InvalidAdapterConfig,
    #[msg("Adapter revenue authority is invalid")]
    InvalidRevenueAuthority,
    #[msg("Production identities have not been frozen")]
    ProductionNotFrozen,
    #[msg("Runtime identities do not match the reviewed production freeze")]
    ProductionIdentityMismatch,
    #[msg("Revenue event amount must be greater than zero")]
    ZeroAmount,
    #[msg("Revenue event ID must be non-zero")]
    InvalidEventId,
    #[msg("Unsupported stablecoin mint")]
    UnsupportedMint,
    #[msg("Payer does not own the payment token account")]
    WrongPayerAuthority,
    #[msg("Payment and revenue destination mints differ")]
    MintMismatch,
    #[msg("Payer source must be the canonical SPL ATA")]
    NonCanonicalPayerSource,
    #[msg("Revenue destination must be the canonical adapter authority ATA")]
    NonCanonicalRevenueDestination,
    #[msg("Payer has insufficient stablecoin balance")]
    InsufficientPaymentBalance,
}
