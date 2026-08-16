use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use revenue_qualification::cpi as qualification_cpi;
use revenue_qualification::{RevenueEvidenceV1, EVIDENCE_AUTHORITY_SEED, EVIDENCE_SEED, EVIDENCE_VERSION};

#[cfg(feature = "production")]
compile_error!("test_revenue_evidence_stub is a TEST-ONLY fixture and must never be compiled for production");

declare_id!("GECbhfQHvCv7RBK19EnEzVnrRVTjnrY1h2dRNf65QHuG");

pub const EVIDENCE_SPACE: usize = 241;

#[program]
pub mod test_revenue_evidence_stub {
    use super::*;

    pub fn publish_and_qualify(
        ctx: Context<PublishAndQualify>,
        event_id: [u8; 32],
        evidence_amount: u64,
        requested_amount: u64,
        reference_hash: [u8; 32],
    ) -> Result<()> {
        require_keys_eq!(ctx.accounts.qualification_program.key(), revenue_qualification::ID, StubError::WrongQualificationProgram);
        require!(ctx.accounts.qualification_program.executable, StubError::WrongQualificationProgram);

        let evidence = RevenueEvidenceV1 {
            version: EVIDENCE_VERSION,
            event_id,
            qualification_program: ctx.accounts.qualification_program.key(),
            adapter_program: ctx.accounts.adapter_program.key(),
            revenue_authority: ctx.accounts.revenue_authority.key(),
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
            evidence.serialize(&mut writer).map_err(|_| error!(StubError::EvidenceSerializationFailed))?;
            require!(writer.is_empty(), StubError::BadEvidenceSpace);
        }
        forward(
            ctx.accounts.rent_payer.to_account_info(),
            ctx.accounts.evidence_authority.to_account_info(),
            ctx.accounts.qualification_config.to_account_info(),
            ctx.accounts.verifier_authority.to_account_info(),
            ctx.accounts.evidence.to_account_info(),
            ctx.accounts.adapter_program.to_account_info(),
            ctx.accounts.adapter_config.to_account_info(),
            ctx.accounts.revenue_authority.to_account_info(),
            ctx.accounts.adapter_receipt.to_account_info(),
            ctx.accounts.source_token.to_account_info(),
            ctx.accounts.referral_program.to_account_info(),
            ctx.accounts.protocol.to_account_info(),
            ctx.accounts.vault_authority.to_account_info(),
            ctx.accounts.vault_token.to_account_info(),
            ctx.accounts.service_treasury_token.to_account_info(),
            ctx.accounts.beneficiary.to_account_info(),
            [
                ctx.accounts.upline_1.to_account_info(), ctx.accounts.upline_2.to_account_info(),
                ctx.accounts.upline_3.to_account_info(), ctx.accounts.upline_4.to_account_info(),
                ctx.accounts.upline_5.to_account_info(), ctx.accounts.upline_6.to_account_info(),
                ctx.accounts.upline_7.to_account_info(), ctx.accounts.upline_8.to_account_info(),
                ctx.accounts.upline_9.to_account_info(), ctx.accounts.upline_10.to_account_info(),
            ],
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.bumps.evidence_authority,
            event_id,
            requested_amount,
        )
    }

    pub fn replay_existing(ctx: Context<ReplayExisting>, event_id: [u8; 32], amount: u64) -> Result<()> {
        forward(
            ctx.accounts.rent_payer.to_account_info(),
            ctx.accounts.evidence_authority.to_account_info(),
            ctx.accounts.qualification_config.to_account_info(),
            ctx.accounts.verifier_authority.to_account_info(),
            ctx.accounts.evidence.to_account_info(),
            ctx.accounts.adapter_program.to_account_info(),
            ctx.accounts.adapter_config.to_account_info(),
            ctx.accounts.revenue_authority.to_account_info(),
            ctx.accounts.adapter_receipt.to_account_info(),
            ctx.accounts.source_token.to_account_info(),
            ctx.accounts.referral_program.to_account_info(),
            ctx.accounts.protocol.to_account_info(),
            ctx.accounts.vault_authority.to_account_info(),
            ctx.accounts.vault_token.to_account_info(),
            ctx.accounts.service_treasury_token.to_account_info(),
            ctx.accounts.beneficiary.to_account_info(),
            [
                ctx.accounts.upline_1.to_account_info(), ctx.accounts.upline_2.to_account_info(),
                ctx.accounts.upline_3.to_account_info(), ctx.accounts.upline_4.to_account_info(),
                ctx.accounts.upline_5.to_account_info(), ctx.accounts.upline_6.to_account_info(),
                ctx.accounts.upline_7.to_account_info(), ctx.accounts.upline_8.to_account_info(),
                ctx.accounts.upline_9.to_account_info(), ctx.accounts.upline_10.to_account_info(),
            ],
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.bumps.evidence_authority,
            event_id,
            amount,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn forward<'info>(
    rent_payer: AccountInfo<'info>, evidence_authority: AccountInfo<'info>, qualification_config: AccountInfo<'info>,
    verifier_authority: AccountInfo<'info>, evidence: AccountInfo<'info>, adapter_program: AccountInfo<'info>,
    adapter_config: AccountInfo<'info>, revenue_authority: AccountInfo<'info>, adapter_receipt: AccountInfo<'info>,
    source_token: AccountInfo<'info>, referral_program: AccountInfo<'info>, protocol: AccountInfo<'info>,
    vault_authority: AccountInfo<'info>, vault_token: AccountInfo<'info>, service_treasury_token: AccountInfo<'info>,
    beneficiary: AccountInfo<'info>, uplines: [AccountInfo<'info>; 10], token_program: AccountInfo<'info>,
    system_program: AccountInfo<'info>, evidence_authority_bump: u8, event_id: [u8; 32], amount: u64,
) -> Result<()> {
    let cpi_accounts = qualification_cpi::accounts::QualifyVerifiedEvidence {
        rent_payer,
        evidence_authority: evidence_authority.clone(),
        config: qualification_config,
        verifier_authority,
        evidence,
        adapter_program,
        adapter_config,
        revenue_authority,
        adapter_receipt,
        source_token,
        referral_program,
        protocol,
        vault_authority,
        vault_token,
        service_treasury_token,
        beneficiary,
        upline_1: uplines[0].clone(), upline_2: uplines[1].clone(), upline_3: uplines[2].clone(),
        upline_4: uplines[3].clone(), upline_5: uplines[4].clone(), upline_6: uplines[5].clone(),
        upline_7: uplines[6].clone(), upline_8: uplines[7].clone(), upline_9: uplines[8].clone(),
        upline_10: uplines[9].clone(), token_program, system_program,
    };
    let bump = [evidence_authority_bump];
    let signer_seeds: &[&[u8]] = &[EVIDENCE_AUTHORITY_SEED, &bump];
    qualification_cpi::qualify_verified_evidence(
        CpiContext::new_with_signer(revenue_qualification::ID, cpi_accounts, &[signer_seeds]),
        event_id,
        amount,
    )
}

#[derive(Accounts)]
#[instruction(event_id: [u8; 32])]
pub struct PublishAndQualify<'info> {
    #[account(mut)] pub rent_payer: Signer<'info>,
    /// CHECK: deterministic CPI signer.
    #[account(seeds = [EVIDENCE_AUTHORITY_SEED], bump)] pub evidence_authority: UncheckedAccount<'info>,
    /// CHECK: raw fixed-format test evidence.
    #[account(init, payer = rent_payer, seeds = [EVIDENCE_SEED, event_id.as_ref()], bump, space = EVIDENCE_SPACE)]
    pub evidence: UncheckedAccount<'info>,
    /// CHECK: checked in handler.
    #[account(executable)] pub qualification_program: UncheckedAccount<'info>,
    /// CHECK: validated by qualification gateway.
    pub qualification_config: UncheckedAccount<'info>,
    /// CHECK: qualification verifier PDA.
    pub verifier_authority: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    #[account(executable)] pub adapter_program: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    pub adapter_config: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    pub revenue_authority: UncheckedAccount<'info>,
    /// CHECK: created by adapter.
    #[account(mut)] pub adapter_receipt: UncheckedAccount<'info>,
    #[account(mut)] pub source_token: Account<'info, TokenAccount>,
    /// CHECK: validated downstream.
    #[account(executable)] pub referral_program: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    #[account(mut)] pub protocol: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    pub vault_authority: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    #[account(mut)] pub vault_token: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    #[account(mut)] pub service_treasury_token: UncheckedAccount<'info>,
    /// CHECK: validated downstream and bound to evidence.
    #[account(mut)] pub beneficiary: UncheckedAccount<'info>,
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
    #[account(mut)] pub rent_payer: Signer<'info>,
    /// CHECK: deterministic CPI signer.
    #[account(seeds = [EVIDENCE_AUTHORITY_SEED], bump)] pub evidence_authority: UncheckedAccount<'info>,
    /// CHECK: canonical evidence account from this fixture.
    #[account(seeds = [EVIDENCE_SEED, event_id.as_ref()], bump)] pub evidence: UncheckedAccount<'info>,
    /// CHECK: checked by destination program.
    #[account(executable)] pub qualification_program: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    pub qualification_config: UncheckedAccount<'info>,
    /// CHECK: qualification verifier PDA.
    pub verifier_authority: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    #[account(executable)] pub adapter_program: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    pub adapter_config: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    pub revenue_authority: UncheckedAccount<'info>,
    /// CHECK: existing receipt; duplicate init must fail downstream.
    #[account(mut)] pub adapter_receipt: UncheckedAccount<'info>,
    #[account(mut)] pub source_token: Account<'info, TokenAccount>,
    /// CHECK: validated downstream.
    #[account(executable)] pub referral_program: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    #[account(mut)] pub protocol: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    pub vault_authority: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    #[account(mut)] pub vault_token: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    #[account(mut)] pub service_treasury_token: UncheckedAccount<'info>,
    /// CHECK: validated downstream.
    #[account(mut)] pub beneficiary: UncheckedAccount<'info>,
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
    #[msg("Wrong qualification program")] WrongQualificationProgram,
    #[msg("Unexpected evidence account size")] BadEvidenceSpace,
    #[msg("Could not serialize test evidence")] EvidenceSerializationFailed,
}
