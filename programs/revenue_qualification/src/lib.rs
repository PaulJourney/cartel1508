use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::hashv;
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

    /// Qualifies revenue only when the caller funds it with real SPL tokens.
    /// The transfer, adapter receipt creation and referral accounting are atomic.
    pub fn qualify_payment_and_route(
        ctx: Context<QualifyPaymentAndRoute>,
        client_nonce: [u8; 32],
        amount: u64,
    ) -> Result<()> {
        require!(amount > 0, QualificationError::ZeroAmount);
        require!(client_nonce != [0u8; 32], QualificationError::InvalidNonce);

        let mint = ctx.accounts.payer_source_token.mint;
        require!(
            mint == ctx.accounts.adapter_config.usdt_mint
                || mint == ctx.accounts.adapter_config.usdc_mint,
            QualificationError::UnsupportedMint
        );
        validate_production_environment(ctx.accounts.adapter_program.key(), mint)?;

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

        // Bind the replay key to the actual payer, beneficiary, mint, amount and nonce.
        let payer_key = ctx.accounts.payer.key();
        let beneficiary_key = ctx.accounts.beneficiary.key();
        let amount_bytes = amount.to_le_bytes();
        let event_id = hashv(&[
            EVENT_DOMAIN,
            payer_key.as_ref(),
            beneficiary_key.as_ref(),
            mint.as_ref(),
            &amount_bytes,
            &client_nonce,
        ])
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
        // If any later CPI fails, Solana rolls this transfer back atomically.
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
            amount,
        )?;

        emit!(RevenueQualified {
            event_id,
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

    /// CHECK: fixed by address and adapter configuration.
    #[account(address = revenue_adapter::ID)]
    pub adapter_program: UncheckedAccount<'info>,
    /// CHECK: adapter/referral programs validate the exact executable identity.
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
    #[msg("Client nonce must be non-zero")]
    InvalidNonce,
    #[msg("Unsupported stablecoin mint")]
    UnsupportedMint,
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
    #[msg("Adapter receipt PDA does not match derived event id")]
    InvalidAdapterReceipt,
    #[msg("Production adapter identity has not been frozen")]
    ProductionConfigNotFrozen,
    #[msg("Production environment does not match frozen identities")]
    InvalidProductionConfig,
}
