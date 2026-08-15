use anchor_lang::prelude::*;
use anchor_spl::associated_token::get_associated_token_address_with_program_id;
use anchor_spl::token::{Token, TokenAccount};
use revenue_adapter::cpi as adapter_cpi;

declare_id!("AZQHbWahShE5oLKG3BXCrqmoMj6WWtiuxhnCcMp4YoE4");

pub const CONFIG_SEED: &[u8] = b"qualification-config";
pub const QUALIFIER_AUTHORITY_SEED: &[u8] = b"qualified-revenue-authority";
pub const EVIDENCE_AUTHORITY_SEED: &[u8] = b"revenue-evidence-authority";
pub const EVIDENCE_SEED: &[u8] = b"revenue-evidence";
pub const EVIDENCE_VERSION: u8 = 1;

// Fail-closed production sentinels. Replace only during the final reviewed freeze.
pub const MAINNET_ADAPTER_PROGRAM: Pubkey = pubkey!("11111111111111111111111111111111");
pub const MAINNET_EVIDENCE_PROGRAM: Pubkey = pubkey!("11111111111111111111111111111111");
pub const MAINNET_USDT_MINT: Pubkey = pubkey!("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB");
pub const MAINNET_USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");

#[program]
pub mod revenue_qualification {
    use super::*;

    /// One-shot immutable gateway configuration.
    ///
    /// The real revenue semantics live in a separately reviewed immutable evidence
    /// program. This gateway cannot be called directly by a human to fabricate an
    /// event because `evidence_authority` is a PDA that only that evidence program
    /// can sign for during CPI.
    pub fn initialize(
        ctx: Context<Initialize>,
        usdt_mint: Pubkey,
        usdc_mint: Pubkey,
    ) -> Result<()> {
        require!(usdt_mint != usdc_mint, QualificationError::DuplicateMint);

        let adapter_program = ctx.accounts.adapter_program.key();
        let evidence_program = ctx.accounts.evidence_program.key();

        require!(adapter_program != crate::ID, QualificationError::InvalidAdapterProgram);
        require!(ctx.accounts.adapter_program.executable, QualificationError::InvalidAdapterProgram);
        require!(evidence_program != Pubkey::default(), QualificationError::InvalidEvidenceProgram);
        require!(evidence_program != crate::ID, QualificationError::InvalidEvidenceProgram);
        require!(evidence_program != adapter_program, QualificationError::InvalidEvidenceProgram);
        require!(ctx.accounts.evidence_program.executable, QualificationError::InvalidEvidenceProgram);

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

        let expected_evidence_authority = Pubkey::find_program_address(
            &[EVIDENCE_AUTHORITY_SEED],
            &evidence_program,
        ).0;
        require_keys_eq!(
            ctx.accounts.evidence_authority.key(),
            expected_evidence_authority,
            QualificationError::InvalidEvidenceAuthority
        );

        let revenue_authority = Pubkey::find_program_address(
            &[revenue_adapter::REVENUE_AUTHORITY_SEED],
            &adapter_program,
        ).0;

        #[cfg(feature = "production")]
        {
            require!(
                MAINNET_ADAPTER_PROGRAM != Pubkey::default()
                    && MAINNET_EVIDENCE_PROGRAM != Pubkey::default(),
                QualificationError::ProductionNotFrozen
            );
            require_keys_eq!(
                adapter_program,
                MAINNET_ADAPTER_PROGRAM,
                QualificationError::ProductionIdentityMismatch
            );
            require_keys_eq!(
                evidence_program,
                MAINNET_EVIDENCE_PROGRAM,
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
        config.evidence_program = evidence_program;
        config.evidence_authority = expected_evidence_authority;
        config.usdt_mint = usdt_mint;
        config.usdc_mint = usdc_mint;
        config.initialized_at = Clock::get()?.unix_timestamp;
        Ok(())
    }

    /// Consume one immutable evidence PDA and forward the already-funded event to
    /// the adapter. Direct user calls cannot satisfy the evidence-authority signer.
    /// The adapter remains the anti-replay and collateralization boundary.
    pub fn qualify_verified_evidence(
        ctx: Context<QualifyVerifiedEvidence>,
        event_id: [u8; 32],
        amount: u64,
    ) -> Result<()> {
        require!(amount > 0, QualificationError::ZeroAmount);
        require!(event_id != [0u8; 32], QualificationError::InvalidEventId);

        let config = &ctx.accounts.config;
        require_keys_eq!(
            ctx.accounts.evidence_authority.key(),
            config.evidence_authority,
            QualificationError::InvalidEvidenceAuthority
        );
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

        let expected_evidence = Pubkey::find_program_address(
            &[EVIDENCE_SEED, event_id.as_ref()],
            &config.evidence_program,
        ).0;
        require_keys_eq!(
            ctx.accounts.evidence.key(),
            expected_evidence,
            QualificationError::InvalidEvidenceAccount
        );
        require_keys_eq!(
            *ctx.accounts.evidence.owner,
            config.evidence_program,
            QualificationError::InvalidEvidenceAccount
        );

        let evidence = {
            let data = ctx.accounts.evidence.try_borrow_data()?;
            let mut slice: &[u8] = &data;
            RevenueEvidenceV1::deserialize(&mut slice)
                .map_err(|_| error!(QualificationError::MalformedEvidence))?
        };

        require!(
            evidence.version == EVIDENCE_VERSION,
            QualificationError::UnsupportedEvidenceVersion
        );
        require!(evidence.event_id == event_id, QualificationError::EvidenceMismatch);
        require_keys_eq!(
            evidence.qualification_program,
            crate::ID,
            QualificationError::EvidenceMismatch
        );
        require_keys_eq!(
            evidence.adapter_program,
            config.adapter_program,
            QualificationError::EvidenceMismatch
        );
        require_keys_eq!(
            evidence.revenue_authority,
            config.revenue_authority,
            QualificationError::EvidenceMismatch
        );
        require_keys_eq!(
            evidence.beneficiary,
            ctx.accounts.beneficiary.key(),
            QualificationError::EvidenceMismatch
        );
        require!(evidence.amount == amount, QualificationError::EvidenceMismatch);
        require!(
            evidence.reference_hash != [0u8; 32],
            QualificationError::EvidenceMismatch
        );
        require!(evidence.settled_at > 0, QualificationError::EvidenceMismatch);
        require!(
            evidence.mint == config.usdt_mint || evidence.mint == config.usdc_mint,
            QualificationError::UnsupportedMint
        );

        require_keys_eq!(
            ctx.accounts.source_token.owner,
            config.revenue_authority,
            QualificationError::InvalidRevenueAuthority
        );
        require_keys_eq!(
            ctx.accounts.source_token.mint,
            evidence.mint,
            QualificationError::EvidenceMismatch
        );
        let expected_source = get_associated_token_address_with_program_id(
            &config.revenue_authority,
            &evidence.mint,
            &anchor_spl::token::ID,
        );
        require_keys_eq!(
            ctx.accounts.source_token.key(),
            expected_source,
            QualificationError::NonCanonicalRevenueSource
        );
        require!(
            ctx.accounts.source_token.amount >= amount,
            QualificationError::InsufficientFunding
        );

        let cpi_accounts = adapter_cpi::accounts::SubmitRevenueEvent {
            payer: ctx.accounts.rent_payer.to_account_info(),
            qualification_authority: ctx.accounts.qualification_authority.to_account_info(),
            config: ctx.accounts.adapter_config.to_account_info(),
            revenue_authority: ctx.accounts.adapter_revenue_authority.to_account_info(),
            receipt: ctx.accounts.adapter_receipt.to_account_info(),
            source_token: ctx.accounts.source_token.to_account_info(),
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
        adapter_cpi::submit_revenue_event(
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
    /// CHECK: deterministic gateway signer with no private key.
    #[account(seeds = [QUALIFIER_AUTHORITY_SEED], bump)]
    pub qualification_authority: UncheckedAccount<'info>,
    /// CHECK: must be executable, distinct and is frozen into config.
    pub evidence_program: UncheckedAccount<'info>,
    /// CHECK: exact PDA under the immutable evidence program, checked in initialize.
    pub evidence_authority: UncheckedAccount<'info>,
    /// CHECK: checked executable and frozen into config.
    pub adapter_program: UncheckedAccount<'info>,
    /// CHECK: canonical PDA and owner validated in initialize.
    pub adapter_config: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(event_id: [u8; 32])]
pub struct QualifyVerifiedEvidence<'info> {
    /// Rent payer for the adapter receipt only; this signer cannot qualify an event.
    #[account(mut)]
    pub rent_payer: Signer<'info>,
    /// Only the frozen evidence program can make this PDA a signer via CPI.
    pub evidence_authority: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, QualificationConfig>,
    /// CHECK: this program's PDA signs only the adapter CPI.
    #[account(seeds = [QUALIFIER_AUTHORITY_SEED], bump = config.qualifier_authority_bump)]
    pub qualification_authority: UncheckedAccount<'info>,
    /// CHECK: canonical evidence PDA and owner are verified, then fixed-format data is parsed.
    pub evidence: UncheckedAccount<'info>,

    /// CHECK: exact executable address frozen in QualificationConfig.
    pub adapter_program: UncheckedAccount<'info>,
    /// CHECK: exact PDA frozen in QualificationConfig and validated by adapter.
    pub adapter_config: UncheckedAccount<'info>,
    /// CHECK: exact adapter PDA frozen in QualificationConfig.
    pub adapter_revenue_authority: UncheckedAccount<'info>,
    /// CHECK: initialized atomically by adapter; event_id seeds are enforced there.
    #[account(mut)]
    pub adapter_receipt: UncheckedAccount<'info>,
    #[account(mut)]
    pub source_token: Account<'info, TokenAccount>,

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
    /// CHECK: key is bound into immutable external evidence and validated downstream.
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
    pub evidence_program: Pubkey,
    pub evidence_authority: Pubkey,
    pub usdt_mint: Pubkey,
    pub usdc_mint: Pubkey,
    pub initialized_at: i64,
}

impl QualificationConfig {
    pub const SPACE: usize = 8 + 1 + 1 + (32 * 7) + 8;
}

/// Canonical cross-program evidence interface. The immutable evidence program
/// owns the PDA `["revenue-evidence", event_id]` and serializes exactly this data.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct RevenueEvidenceV1 {
    pub version: u8,
    pub event_id: [u8; 32],
    pub qualification_program: Pubkey,
    pub adapter_program: Pubkey,
    pub revenue_authority: Pubkey,
    pub beneficiary: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub settled_at: i64,
    pub reference_hash: [u8; 32],
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
    #[msg("Evidence program is invalid or not executable")]
    InvalidEvidenceProgram,
    #[msg("Evidence authority is not the deterministic PDA of the frozen evidence program")]
    InvalidEvidenceAuthority,
    #[msg("Evidence account is not the canonical PDA owned by the frozen evidence program")]
    InvalidEvidenceAccount,
    #[msg("Evidence account data is malformed")]
    MalformedEvidence,
    #[msg("Evidence version is unsupported")]
    UnsupportedEvidenceVersion,
    #[msg("Evidence fields do not match the requested revenue event")]
    EvidenceMismatch,
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
    #[msg("Revenue source must be the canonical adapter authority ATA")]
    NonCanonicalRevenueSource,
    #[msg("Revenue source is not sufficiently funded before accounting")]
    InsufficientFunding,
}
