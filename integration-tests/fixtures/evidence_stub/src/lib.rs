use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use revenue_qualification::cpi as qualification_cpi;
use revenue_qualification::{
    RevenueEvidenceV1, EVIDENCE_AUTHORITY_SEED, EVIDENCE_SEED, EVIDENCE_VERSION,
};

declare_id!("GECbhfQHvCv7RBK19EnEzVnrRVTjnrY1h2dRNf65QHuG");

// Fixed Borsh size of RevenueEvidenceV1:
// version(1) + event_id(32) + 5 Pubkeys(160) + amount(8) + settled_at(8)
// + reference_hash(32) = 241 bytes. There is deliberately no Anchor account
// discriminator: the production gateway validates owner + canonical PDA + version.
pub const EVIDENCE_SPACE: usize = 241;

#[program]
pub mod test_revenue_evidence_stub {
    use super::*;

    /// TEST ONLY: publish canonical evidence and immediately exercise the complete
    /// nested CPI chain. `evidence_amount` may intentionally differ from
    /// `requested_amount` so tests can prove mismatch rollback.
    pub fn publish_and_qualify(
        ctx: Context<PublishAndQualify>,
        event_id: [u8; 32],
        evidence_amount: u64,
        requested_amount: u64,
        reference_hash: [u8; 32],
    ) -> Result<()> {
        require_keys_eq!(
            ctx.accounts.qualification_program.key(),
            revenue_qualification::ID,
            StubError::WrongQualificationProgram
        );
        require!(
            ctx.accounts.qualification_program.executable,
            StubError::WrongQualificationProgram
        );

        let evidence = RevenueEvidenceV1 {
            version: EVIDENCE_VERSION,
            event_id,
            qualification_program: ctx.accounts.qualification_program.key(),
            adapter_program: ctx.accounts.adapter_program.key(),
            revenue_authority: ctx.accounts.adapter_revenue_authority.key(),
            beneficiary: ctx.accounts.beneficiary.key(),
            mint: ctx.accounts.source_token.mint,
            amount: evidence_amount,
            settled_at: Clock::get()?.unix_timestamp,
            reference_hash,
        };

        {
            let mut data = ctx.accounts.evidence.try_borrow_mut_data()?;
            require!(data.len() == EVIDENCE_SPACE, StubError::BadEvidenceSpace);
            let mut writer: &mut [u8] = &mut data;
            evidence
                .serialize(&mut writer)
                .map_err(|_| error!(StubError::EvidenceSerializationFailed))?;
            require!(writer.is_empty(), StubError::BadEvidenceSpace);
        }

        forward_to_qualification(
            &ctx.accounts,
            ctx.bumps.evidence_authority,
            event_id,
            requested_amount,
        )
    }

    /// TEST ONLY: forward an already-existing evidence PDA again. This reaches the
    /// adapter's own receipt boundary and proves duplicate event IDs cannot double
    /// account revenue even when the upstream evidence PDA already exists.
    pub fn replay_existing(
        ctx: Context<ReplayExisting>,
        event_id: [u8; 32],
        amount: u64,
    ) -> Result<()> {
        require_keys_eq!(
            ctx.accounts.qualification_program.key(),
            revenue_qualification::ID,
            StubError::WrongQualificationProgram
        );
        require!(
            ctx.accounts.qualification_program.executable,
            StubError::WrongQualificationProgram
        );

        forward_existing_to_qualification(
            &ctx.accounts,
            ctx.bumps.evidence_authority,
            event_id,
            amount,
        )
    }
}

fn forward_to_qualification<'info>(
    accounts: &PublishAndQualify<'info>,
    evidence_authority_bump: u8,
    event_id: [u8; 32],
    amount: u64,
) -> Result<()> {
    let cpi_accounts = qualification_cpi::accounts::QualifyVerifiedEvidence {
        rent_payer: accounts.rent_payer.to_account_info(),
        evidence_authority: accounts.evidence_authority.to_account_info(),
        config: accounts.qualification_config.to_account_info(),
        qualification_authority: accounts.qualification_authority.to_account_info(),
        evidence: accounts.evidence.to_account_info(),
        adapter_program: accounts.adapter_program.to_account_info(),
        adapter_config: accounts.adapter_config.to_account_info(),
        adapter_revenue_authority: accounts.adapter_revenue_authority.to_account_info(),
        adapter_receipt: accounts.adapter_receipt.to_account_info(),
        source_token: accounts.source_token.to_account_info(),
        referral_program: accounts.referral_program.to_account_info(),
        protocol: accounts.protocol.to_account_info(),
        vault_authority: accounts.vault_authority.to_account_info(),
        vault_token: accounts.vault_token.to_account_info(),
        service_treasury_token: accounts.service_treasury_token.to_account_info(),
        beneficiary: accounts.beneficiary.to_account_info(),
        upline_1: accounts.upline_1.to_account_info(),
        upline_2: accounts.upline_2.to_account_info(),
        upline_3: accounts.upline_3.to_account_info(),
        upline_4: accounts.upline_4.to_account_info(),
        upline_5: accounts.upline_5.to_account_info(),
        upline_6: accounts.upline_6.to_account_info(),
        upline_7: accounts.upline_7.to_account_info(),
        upline_8: accounts.upline_8.to_account_info(),
        upline_9: accounts.upline_9.to_account_info(),
        upline_10: accounts.upline_10.to_account_info(),
        token_program: accounts.token_program.to_account_info(),
        system_program: accounts.system_program.to_account_info(),
    };

    let bump = [evidence_authority_bump];
    let signer_seeds: &[&[u8]] = &[EVIDENCE_AUTHORITY_SEED, &bump];
    qualification_cpi::qualify_verified_evidence(
        CpiContext::new_with_signer(
            accounts.qualification_program.key(),
            cpi_accounts,
            &[signer_seeds],
        ),
        event_id,
        amount,
    )
}

fn forward_existing_to_qualification<'info>(
    accounts: &ReplayExisting<'info>,
    evidence_authority_bump: u8,
    event_id: [u8; 32],
    amount: u64,
) -> Result<()> {
    let cpi_accounts = qualification_cpi::accounts::QualifyVerifiedEvidence {
        rent_payer: accounts.rent_payer.to_account_info(),
        evidence_authority: accounts.evidence_authority.to_account_info(),
        config: accounts.qualification_config.to_account_info(),
        qualification_authority: accounts.qualification_authority.to_account_info(),
        evidence: accounts.evidence.to_account_info(),
        adapter_program: accounts.adapter_program.to_account_info(),
        adapter_config: accounts.adapter_config.to_account_info(),
        adapter_revenue_authority: accounts.adapter_revenue_authority.to_account_info(),
        adapter_receipt: accounts.adapter_receipt.to_account_info(),
        source_token: accounts.source_token.to_account_info(),
        referral_program: accounts.referral_program.to_account_info(),
        protocol: accounts.protocol.to_account_info(),
        vault_authority: accounts.vault_authority.to_account_info(),
        vault_token: accounts.vault_token.to_account_info(),
        service_treasury_token: accounts.service_treasury_token.to_account_info(),
        beneficiary: accounts.beneficiary.to_account_info(),
        upline_1: accounts.upline_1.to_account_info(),
        upline_2: accounts.upline_2.to_account_info(),
        upline_3: accounts.upline_3.to_account_info(),
        upline_4: accounts.upline_4.to_account_info(),
        upline_5: accounts.upline_5.to_account_info(),
        upline_6: accounts.upline_6.to_account_info(),
        upline_7: accounts.upline_7.to_account_info(),
        upline_8: accounts.upline_8.to_account_info(),
        upline_9: accounts.upline_9.to_account_info(),
        upline_10: accounts.upline_10.to_account_info(),
        token_program: accounts.token_program.to_account_info(),
        system_program: accounts.system_program.to_account_info(),
    };

    let bump = [evidence_authority_bump];
    let signer_seeds: &[&[u8]] = &[EVIDENCE_AUTHORITY_SEED, &bump];
    qualification_cpi::qualify_verified_evidence(
        CpiContext::new_with_signer(
            accounts.qualification_program.key(),
            cpi_accounts,
            &[signer_seeds],
        ),
        event_id,
        amount,
    )
}

#[derive(Accounts)]
#[instruction(event_id: [u8; 32])]
pub struct PublishAndQualify<'info> {
    #[account(mut)]
    pub rent_payer: Signer<'info>,
    /// CHECK: deterministic PDA used only to sign the gateway CPI.
    #[account(seeds = [EVIDENCE_AUTHORITY_SEED], bump)]
    pub evidence_authority: UncheckedAccount<'info>,
    /// CHECK: raw fixed-format test evidence, owned by this fixture program.
    #[account(
        init,
        payer = rent_payer,
        seeds = [EVIDENCE_SEED, event_id.as_ref()],
        bump,
        space = EVIDENCE_SPACE
    )]
    pub evidence: UncheckedAccount<'info>,

    /// CHECK: exact dev/test qualification program checked in instruction.
    pub qualification_program: UncheckedAccount<'info>,
    /// CHECK: validated by qualification gateway.
    pub qualification_config: UncheckedAccount<'info>,
    /// CHECK: validated by qualification gateway.
    pub qualification_authority: UncheckedAccount<'info>,

    /// CHECK: validated by qualification gateway and adapter.
    pub adapter_program: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    pub adapter_config: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    pub adapter_revenue_authority: UncheckedAccount<'info>,
    /// CHECK: initialized by adapter.
    #[account(mut)]
    pub adapter_receipt: UncheckedAccount<'info>,
    #[account(mut)]
    pub source_token: Account<'info, TokenAccount>,

    /// CHECK: validated downstream.
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
    /// CHECK: validated downstream and bound into evidence.
    #[account(mut)]
    pub beneficiary: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    #[account(mut)] pub upline_1: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    #[account(mut)] pub upline_2: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    #[account(mut)] pub upline_3: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    #[account(mut)] pub upline_4: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    #[account(mut)] pub upline_5: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    #[account(mut)] pub upline_6: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    #[account(mut)] pub upline_7: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    #[account(mut)] pub upline_8: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    #[account(mut)] pub upline_9: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    #[account(mut)] pub upline_10: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(event_id: [u8; 32])]
pub struct ReplayExisting<'info> {
    #[account(mut)]
    pub rent_payer: Signer<'info>,
    /// CHECK: deterministic PDA used only to sign the gateway CPI.
    #[account(seeds = [EVIDENCE_AUTHORITY_SEED], bump)]
    pub evidence_authority: UncheckedAccount<'info>,
    /// CHECK: canonical evidence account created by this fixture.
    #[account(seeds = [EVIDENCE_SEED, event_id.as_ref()], bump)]
    pub evidence: UncheckedAccount<'info>,

    /// CHECK: exact dev/test qualification program checked in instruction.
    pub qualification_program: UncheckedAccount<'info>,
    /// CHECK: validated by qualification gateway.
    pub qualification_config: UncheckedAccount<'info>,
    /// CHECK: validated by qualification gateway.
    pub qualification_authority: UncheckedAccount<'info>,

    /// CHECK: validated by qualification gateway and adapter.
    pub adapter_program: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    pub adapter_config: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    pub adapter_revenue_authority: UncheckedAccount<'info>,
    /// CHECK: already-created adapter receipt; duplicate init must fail downstream.
    #[account(mut)]
    pub adapter_receipt: UncheckedAccount<'info>,
    #[account(mut)]
    pub source_token: Account<'info, TokenAccount>,

    /// CHECK: validated downstream.
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
    /// CHECK: validated downstream and bound into evidence.
    #[account(mut)]
    pub beneficiary: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    #[account(mut)] pub upline_1: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    #[account(mut)] pub upline_2: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    #[account(mut)] pub upline_3: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    #[account(mut)] pub upline_4: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    #[account(mut)] pub upline_5: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    #[account(mut)] pub upline_6: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    #[account(mut)] pub upline_7: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    #[account(mut)] pub upline_8: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    #[account(mut)] pub upline_9: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    #[account(mut)] pub upline_10: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[error_code]
pub enum StubError {
    #[msg("Wrong qualification program")]
    WrongQualificationProgram,
    #[msg("Unexpected evidence account size")]
    BadEvidenceSpace,
    #[msg("Could not serialize test evidence")]
    EvidenceSerializationFailed,
}
