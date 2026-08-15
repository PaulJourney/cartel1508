use anchor_lang::prelude::*;

#[cfg(feature = "production")]
compile_error!("test_revenue_verifier is a TEST-ONLY fixture and must never be compiled for production");

declare_id!("2R4jhFt26TjJxRx6LG8DcN9q1VRoXjK3e5C1t6whEbEy");

pub const VERIFIER_AUTHORITY_SEED: &[u8] = b"qualified-revenue-verifier";

#[program]
pub mod test_revenue_verifier {
    use super::*;

    pub fn submit_for_test(
        ctx: Context<SubmitForTest>,
        event_id: [u8; 32],
        evidence_hash: [u8; 32],
        amount: u64,
    ) -> Result<()> {
        let bump = [ctx.bumps.verifier_authority];
        let verifier_seeds: &[&[u8]] = &[VERIFIER_AUTHORITY_SEED, &bump];
        let signer_seeds: &[&[&[u8]]] = &[verifier_seeds];

        let cpi_accounts = revenue_adapter::cpi::accounts::ForwardQualifiedRevenue {
            relayer: ctx.accounts.relayer.to_account_info(),
            config: ctx.accounts.config.to_account_info(),
            verifier_authority: ctx.accounts.verifier_authority.to_account_info(),
            revenue_authority: ctx.accounts.revenue_authority.to_account_info(),
            source_token: ctx.accounts.source_token.to_account_info(),
            receipt: ctx.accounts.receipt.to_account_info(),
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
            referral_program: ctx.accounts.referral_program.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.adapter_program.key(),
            cpi_accounts,
            signer_seeds,
        );
        revenue_adapter::cpi::forward_qualified_revenue(
            cpi_ctx,
            event_id,
            evidence_hash,
            amount,
        )
    }
}

#[derive(Accounts)]
pub struct SubmitForTest<'info> {
    #[account(mut)]
    pub relayer: Signer<'info>,
    /// CHECK: pass-through adapter config PDA.
    pub config: UncheckedAccount<'info>,
    /// CHECK: PDA signer exists only in this CPI frame.
    #[account(seeds = [VERIFIER_AUTHORITY_SEED], bump)]
    pub verifier_authority: UncheckedAccount<'info>,
    /// CHECK: pass-through adapter RevenueAuthority PDA.
    pub revenue_authority: UncheckedAccount<'info>,
    /// CHECK: adapter and referral program perform exact SPL validation.
    #[account(mut)]
    pub source_token: UncheckedAccount<'info>,
    /// CHECK: adapter creates and validates the deterministic receipt PDA.
    #[account(mut)]
    pub receipt: UncheckedAccount<'info>,
    /// CHECK: referral protocol validates its canonical state PDA.
    #[account(mut)]
    pub protocol: UncheckedAccount<'info>,
    /// CHECK: referral protocol validates the canonical vault authority PDA.
    pub vault_authority: UncheckedAccount<'info>,
    /// CHECK: referral protocol validates the canonical vault token account.
    #[account(mut)]
    pub vault_token: UncheckedAccount<'info>,
    /// CHECK: referral protocol validates the canonical treasury token account.
    #[account(mut)]
    pub service_treasury_token: UncheckedAccount<'info>,
    /// CHECK: referral protocol validates beneficiary PDA and data.
    #[account(mut)]
    pub beneficiary: UncheckedAccount<'info>,
    /// CHECK: referral protocol verifies immutable ancestry.
    #[account(mut)] pub upline_1: UncheckedAccount<'info>,
    /// CHECK: referral protocol verifies immutable ancestry.
    #[account(mut)] pub upline_2: UncheckedAccount<'info>,
    /// CHECK: referral protocol verifies immutable ancestry.
    #[account(mut)] pub upline_3: UncheckedAccount<'info>,
    /// CHECK: referral protocol verifies immutable ancestry.
    #[account(mut)] pub upline_4: UncheckedAccount<'info>,
    /// CHECK: referral protocol verifies immutable ancestry.
    #[account(mut)] pub upline_5: UncheckedAccount<'info>,
    /// CHECK: referral protocol verifies immutable ancestry.
    #[account(mut)] pub upline_6: UncheckedAccount<'info>,
    /// CHECK: referral protocol verifies immutable ancestry.
    #[account(mut)] pub upline_7: UncheckedAccount<'info>,
    /// CHECK: referral protocol verifies immutable ancestry.
    #[account(mut)] pub upline_8: UncheckedAccount<'info>,
    /// CHECK: referral protocol verifies immutable ancestry.
    #[account(mut)] pub upline_9: UncheckedAccount<'info>,
    /// CHECK: referral protocol verifies immutable ancestry.
    #[account(mut)] pub upline_10: UncheckedAccount<'info>,
    pub adapter_program: Program<'info, revenue_adapter::program::RevenueAdapter>,
    /// CHECK: adapter validates the exact Service Referral Program ID.
    #[account(executable)]
    pub referral_program: UncheckedAccount<'info>,
    /// CHECK: pass-through SPL Token executable.
    #[account(executable)]
    pub token_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifier_authority_seed_matches_adapter_contract() {
        assert_eq!(VERIFIER_AUTHORITY_SEED, revenue_adapter::VERIFIER_AUTHORITY_SEED);
    }
}
