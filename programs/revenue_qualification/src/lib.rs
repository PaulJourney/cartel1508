use anchor_lang::prelude::*;
use anchor_spl::associated_token::get_associated_token_address_with_program_id;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use revenue_adapter::cpi as adapter_cpi;

declare_id!("6WWnYWwsuYNPDruPJEsqJKekv9mN9NP2mCEU6XJ7qfWs");

pub const QUALIFIER_AUTHORITY_SEED: &[u8] = b"qualified-revenue-authority";
pub const EVENT_DOMAIN: &[u8] = b"qualified-revenue-v1";

// Final production identities remain deliberately fail-closed until release freeze.
pub const MAINNET_ADAPTER_PROGRAM: Pubkey = Pubkey::new_from_array([0u8; 32]);
pub const MAINNET_USDT_MINT: Pubkey = pubkey!("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB");
pub const MAINNET_USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");

#[program]
pub mod revenue_qualification {
    use super::*;

    /// Routes revenue only when the caller funds it with real SPL tokens and
    /// supplies a non-zero evidence hash from the reviewed qualification layer.
    /// Transfer, adapter receipt creation and referral accounting are atomic.
    pub fn qualify_payment_and_route(
        ctx: Context<QualifyPaymentAndRoute>,
        evidence_hash: [u8; 32],
        amount: u64,
    ) -> Result<()> {
        require!(amount > 0, QualificationError::ZeroAmount);
        require!(evidence_hash != [0u8; 32], QualificationError::InvalidEvidenceHash);

        let mint = ctx.accounts.payer_source_token.mint;
        require!(
            mint == ctx.accounts.adapter_config.usdt_mint
                || mint == ctx.accounts.adapter_config.usdc_mint,
            QualificationError::UnsupportedMint
        );
        validate_production_environment(ctx.accounts.adapter_program.key(), mint)?;

        // Re-derive the one canonical adapter config PDA so a substitute adapter-owned
        // config can never alter the frozen trust chain.
        let expected_adapter_config = Pubkey::find_program_address(
            &[revenue_adapter::CONFIG_SEED],
            &ctx.accounts.adapter_program.key(),
        )
        .0;
        require_keys_eq!(
            ctx.accounts.adapter_config.key(),
            expected_adapter_config,
            QualificationError::InvalidAdapterConfig
        );
        require!(
            ctx.accounts.adapter_program.executable,
            QualificationError::InvalidAdapterProgram
        );
        require_keys_eq!(
            ctx.accounts.adapter_config.referral_program,
            ctx.accounts.referral_program.key(),
            QualificationError::InvalidReferralProgram
        );
        require!(
            ctx.accounts.referral_program.executable,
            QualificationError::InvalidReferralProgram
        );
        require_keys_eq!(
            ctx.accounts.adapter_config.qualification_program,
            crate::ID,
            QualificationError::AdapterNotBoundToQualificationProgram
        );
        require_keys_eq!(
            ctx.accounts.adapter_config.qualification_authority,
            ctx.accounts.qualification_authority.key(),
            QualificationError::InvalidQualificationAuthority
        );
        require_keys_eq!(
            ctx.accounts.adapter_config.revenue_authority,
            ctx.accounts.revenue_authority.key(),
            QualificationError::WrongRevenueAuthority
        );
        require_keys_eq!(
            ctx.accounts.payer_source_token.owner,
            ctx.accounts.payer.key(),
            QualificationError::WrongPayerTokenOwner
        );
        require_keys_eq!(
            ctx.accounts.payer_source_token.mint,
            ctx.accounts.revenue_source_token.mint,
            QualificationError::MintMismatch
        );
        require_keys_eq!(
            ctx.accounts.revenue_source_token.owner,
            ctx.accounts.revenue_authority.key(),
            QualificationError::WrongRevenueAuthority
        );

        let expected_revenue_source = get_associated_token_address_with_program_id(
            &ctx.accounts.revenue_authority.key(),
            &mint,
            &token::ID,
        );
        require_keys_eq!(
            ctx.accounts.revenue_source_token.key(),
            expected_revenue_source,
            QualificationError::NonCanonicalRevenueSource
        );

        // Economic replay identity is derived from the evidence itself, not from a
        // freely variable nonce. The final evidence producer must guarantee that one
        // underlying economic event has one canonical evidence hash.
        let payer_key = ctx.accounts.payer.key();
        let beneficiary_key = ctx.accounts.beneficiary.key();
        let amount_bytes = amount.to_le_bytes();
        let event_id = Pubkey::find_program_address(
            &[
                EVENT_DOMAIN,
                evidence_hash.as_ref(),
                payer_key.as_ref(),
                beneficiary_key.as_ref(),
                mint.as_ref(),
                &amount_bytes,
            ],
            &crate::ID,
        )
        .0
        .to_bytes();

        let expected_receipt = Pubkey::find_program_address(
            &[revenue_adapter::RECEIPT_SEED, event_id.as_ref()],
            &ctx.accounts.adapter_program.key(),
        )
        .0;
        require_keys_eq!(
            ctx.accounts.adapter_receipt.key(),
            expected_receipt,
            QualificationError::InvalidAdapterReceipt
        );

        // First move real value into the adapter-owned revenue source ATA.
        // Any later CPI failure rolls this transfer back atomically.
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.key(),
                Transfer {
                    from: ctx.accounts.payer_source_token.to_account_info(),
                    to: ctx.accounts.revenue_source_token.to_account_info(),
                    authority: ctx.accounts.payer.to_account_info(),
                },
            ),
            amount,
        )?;

        let cpi_accounts = adapter_cpi::accounts::SubmitRevenueEvent {
            payer: ctx.accounts.payer.to_account_info(),
            qualification_authority: ctx.accounts.qualification_authority.to_account_info(),
            config: ctx.accounts.adapter_config.to_account_info(),
            revenue_authority: ctx.accounts.revenue_authority.to_account_info(),
            receipt: ctx.accounts.adapter_receipt.to_account_info(),
            source_token: ctx.accounts.revenue_source_token.to_account_info(),
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

        let authority_bump = [ctx.bumps.qualification_authority];
        let signer_seeds: &[&[u8]] = &[QUALIFIER_AUTHORITY_SEED, &authority_bump];
        adapter_cpi::submit_revenue_event(
            CpiContext::new_with_signer(
                ctx.accounts.adapter_program.key(),
                cpi_accounts,
                &[signer_seeds],
            ),
            event_id,
            evidence_hash,
            amount,
        )?;

        emit!(RevenueQualified {
            event_id,
            evidence_hash,
            payer: payer_key,
            beneficiary: beneficiary_key,
            mint,
            amount,
        });

        Ok(())
    }
}

#[derive(Accounts)]
pub struct QualifyPaymentAndRoute<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        mut,
        constraint = payer_source_token.owner == payer.key() @ QualificationError::WrongPayerTokenOwner
    )]
    pub payer_source_token: Account<'info, TokenAccount>,

    /// CHECK: PDA owned by this program; it has no private key and signs only the adapter CPI.
    #[account(seeds = [QUALIFIER_AUTHORITY_SEED], bump)]
    pub qualification_authority: UncheckedAccount<'info>,

    #[account(owner = revenue_adapter::ID)]
    pub adapter_config: Account<'info, revenue_adapter::AdapterConfig>,
    /// CHECK: validated against adapter_config and canonical ATA derivation.
    pub revenue_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub revenue_source_token: Account<'info, TokenAccount>,
    /// CHECK: deterministic receipt PDA is recomputed from the internally derived event id.
    #[account(mut)]
    pub adapter_receipt: UncheckedAccount<'info>,

    /// CHECK: fixed by address and validated executable before any token transfer.
    #[account(address = revenue_adapter::ID)]
    pub adapter_program: UncheckedAccount<'info>,
    /// CHECK: bound to adapter_config and validated executable before any token transfer.
    pub referral_program: UncheckedAccount<'info>,
    /// CHECK: validated by referral protocol.
    #[account(mut)]
    pub protocol: UncheckedAccount<'info>,
    /// CHECK: validated by referral protocol.
    pub vault_authority: UncheckedAccount<'info>,
    /// CHECK: validated by referral protocol.
    #[account(mut)]
    pub vault_token: UncheckedAccount<'info>,
    /// CHECK: validated by referral protocol.
    #[account(mut)]
    pub service_treasury_token: UncheckedAccount<'info>,
    /// CHECK: validated by referral protocol user state.
    #[account(mut)]
    pub beneficiary: UncheckedAccount<'info>,
    /// CHECK: ancestry validated downstream.
    #[account(mut)] pub upline_1: UncheckedAccount<'info>,
    /// CHECK: ancestry validated downstream.
    #[account(mut)] pub upline_2: UncheckedAccount<'info>,
    /// CHECK: ancestry validated downstream.
    #[account(mut)] pub upline_3: UncheckedAccount<'info>,
    /// CHECK: ancestry validated downstream.
    #[account(mut)] pub upline_4: UncheckedAccount<'info>,
    /// CHECK: ancestry validated downstream.
    #[account(mut)] pub upline_5: UncheckedAccount<'info>,
    /// CHECK: ancestry validated downstream.
    #[account(mut)] pub upline_6: UncheckedAccount<'info>,
    /// CHECK: ancestry validated downstream.
    #[account(mut)] pub upline_7: UncheckedAccount<'info>,
    /// CHECK: ancestry validated downstream.
    #[account(mut)] pub upline_8: UncheckedAccount<'info>,
    /// CHECK: ancestry validated downstream.
    #[account(mut)] pub upline_9: UncheckedAccount<'info>,
    /// CHECK: ancestry validated downstream.
    #[account(mut)] pub upline_10: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[event]
pub struct RevenueQualified {
    pub event_id: [u8; 32],
    pub evidence_hash: [u8; 32],
    pub payer: Pubkey,
    pub beneficiary: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
}

fn validate_production_environment(adapter_program: Pubkey, mint: Pubkey) -> Result<()> {
    #[cfg(feature = "production")]
    {
        require!(
            MAINNET_ADAPTER_PROGRAM != Pubkey::default(),
            QualificationError::ProductionConfigNotFrozen
        );
        require_keys_eq!(
            adapter_program,
            MAINNET_ADAPTER_PROGRAM,
            QualificationError::InvalidProductionConfig
        );
        require!(
            mint == MAINNET_USDT_MINT || mint == MAINNET_USDC_MINT,
            QualificationError::InvalidProductionConfig
        );
    }
    #[cfg(not(feature = "production"))]
    {
        let _ = (adapter_program, mint);
    }
    Ok(())
}

#[error_code]
pub enum QualificationError {
    #[msg("Revenue amount must be greater than zero")]
    ZeroAmount,
    #[msg("Qualification evidence hash must be non-zero")]
    InvalidEvidenceHash,
    #[msg("Unsupported stablecoin mint")]
    UnsupportedMint,
    #[msg("Adapter config is not the canonical adapter-config PDA")]
    InvalidAdapterConfig,
    #[msg("Adapter program is not executable")]
    InvalidAdapterProgram,
    #[msg("Referral program does not match immutable adapter configuration or is not executable")]
    InvalidReferralProgram,
    #[msg("Adapter is not bound to this qualification program")]
    AdapterNotBoundToQualificationProgram,
    #[msg("Qualification authority mismatch")]
    InvalidQualificationAuthority,
    #[msg("Revenue authority mismatch")]
    WrongRevenueAuthority,
    #[msg("Payer token account is not owned by payer")]
    WrongPayerTokenOwner,
    #[msg("Source and destination token mints differ")]
    MintMismatch,
    #[msg("Revenue destination is not the canonical adapter revenue ATA")]
    NonCanonicalRevenueSource,
    #[msg("Adapter receipt PDA does not match evidence-bound event id")]
    InvalidAdapterReceipt,
    #[msg("Production adapter identity has not been frozen")]
    ProductionConfigNotFrozen,
    #[msg("Production environment does not match frozen identities")]
    InvalidProductionConfig,
}
