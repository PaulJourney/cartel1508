use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

pub mod constants;
pub mod math;
pub mod state;

use constants::*;
use math::*;
use state::*;

declare_id!("4AuoBkj4vkH2K1jwUuECtVBqF6Q74efjGbaw7btuNjRV");

#[program]
pub mod service_referral_protocol {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, registration_open_at: i64, qualified_revenue_source: Pubkey) -> Result<()> {
        require!(constants::percentages_valid(), ProtocolError::InvalidPercentages);
        let now = Clock::get()?.unix_timestamp;
        require!(registration_open_at >= now, ProtocolError::RegistrationOpenInPast);
        require!(qualified_revenue_source != Pubkey::default(), ProtocolError::InvalidRevenueSource);

        let p = &mut ctx.accounts.protocol;
        p.bump = ctx.bumps.protocol;
        p.vault_authority_bump = ctx.bumps.vault_authority;
        p.initialized_at = now;
        p.registration_open_at = registration_open_at;
        p.service_treasury = ctx.accounts.service_treasury.key();
        p.qualified_revenue_source = qualified_revenue_source;
        p.usdt_mint = ctx.accounts.usdt_mint.key();
        p.usdc_mint = ctx.accounts.usdc_mint.key();
        p.pioneer_count = 0;
        p.real_user_count = 0;
        p.pioneer_index_usdt = 0;
        p.pioneer_index_usdc = 0;
        p.pioneer_reserve_usdt = 0;
        p.pioneer_reserve_usdc = 0;
        p.lifetime_service_fees_usdt = 0;
        p.lifetime_service_fees_usdc = 0;
        p.lifetime_unallocated_usdt = 0;
        p.lifetime_unallocated_usdc = 0;
        p.lifetime_expired_usdt = 0;
        p.lifetime_expired_usdc = 0;

        let root = &mut ctx.accounts.technical_root;
        root.bump = ctx.bumps.technical_root;
        root.wallet = Pubkey::default();
        root.referrer = Pubkey::default();
        root.registered_at = now;
        root.pioneer_id = 0;
        root.active_until = 0;
        root.grace_until = 0;
        root.qualification_progress_units = 0;
        root.lifetime_service_units = 0;
        root.next_batch_index = 0;
        root.direct_accrued_usdt = 0;
        root.direct_accrued_usdc = 0;
        root.network_claimable_usdt = 0;
        root.network_claimable_usdc = 0;
        root.network_pending_usdt = 0;
        root.network_pending_usdc = 0;
        root.lifetime_claimed_usdt = 0;
        root.lifetime_claimed_usdc = 0;
        root.lifetime_expired_usdt = 0;
        root.lifetime_expired_usdc = 0;
        root.pioneer_checkpoint_usdt = 0;
        root.pioneer_checkpoint_usdc = 0;
        Ok(())
    }

    pub fn register(ctx: Context<Register>) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        let p = &mut ctx.accounts.protocol;
        require!(now >= p.registration_open_at, ProtocolError::RegistrationNotOpen);
        require!(ctx.accounts.referrer.wallet == ctx.accounts.referrer_wallet.key(), ProtocolError::ReferrerMismatch);
        require!(ctx.accounts.wallet.key() != ctx.accounts.referrer_wallet.key(), ProtocolError::SelfReferral);

        let pioneer_id = if p.pioneer_count < PIONEER_SLOTS {
            p.pioneer_count = p.pioneer_count.checked_add(1).ok_or(ProtocolError::ArithmeticOverflow)?;
            p.pioneer_count
        } else { 0 };
        p.real_user_count = p.real_user_count.checked_add(1).ok_or(ProtocolError::ArithmeticOverflow)?;

        let u = &mut ctx.accounts.user;
        u.bump = ctx.bumps.user;
        u.wallet = ctx.accounts.wallet.key();
        u.referrer = ctx.accounts.referrer_wallet.key();
        u.registered_at = now;
        u.pioneer_id = pioneer_id;
        u.active_until = 0;
        u.grace_until = 0;
        u.qualification_progress_units = 0;
        u.lifetime_service_units = 0;
        u.next_batch_index = 0;
        u.direct_accrued_usdt = 0;
        u.direct_accrued_usdc = 0;
        u.network_claimable_usdt = 0;
        u.network_claimable_usdc = 0;
        u.network_pending_usdt = 0;
        u.network_pending_usdc = 0;
        u.lifetime_claimed_usdt = 0;
        u.lifetime_claimed_usdc = 0;
        u.lifetime_expired_usdt = 0;
        u.lifetime_expired_usdc = 0;
        u.pioneer_checkpoint_usdt = p.pioneer_index_usdt;
        u.pioneer_checkpoint_usdc = p.pioneer_index_usdc;
        Ok(())
    }

    pub fn purchase_service_units(ctx: Context<PurchaseServiceUnits>, units: u64) -> Result<()> {
        require!(units > 0, ProtocolError::ZeroUnits);
        let payment_u128 = (units as u128).checked_mul(TOKEN_SCALE as u128).ok_or(ProtocolError::ArithmeticOverflow)?;
        let payment = u64::try_from(payment_u128).map_err(|_| ProtocolError::BatchTooLarge)?;
        let now = Clock::get()?.unix_timestamp;
        validate_supported_token(&ctx.accounts.protocol, &ctx.accounts.user_source, &ctx.accounts.service_treasury_token)?;
        require!(ctx.accounts.user_source.owner == ctx.accounts.wallet.key(), ProtocolError::WrongTokenAuthority);

        token::transfer(
            CpiContext::new(
                token::ID,
                Transfer {
                    from: ctx.accounts.user_source.to_account_info(),
                    to: ctx.accounts.service_treasury_token.to_account_info(),
                    authority: ctx.accounts.wallet.to_account_info(),
                },
            ),
            payment,
        )?;

        let user = &mut ctx.accounts.user;
        settle_expired_in_memory(user, now, &mut ctx.accounts.protocol)?;

        let first = user.lifetime_service_units.checked_add(1).ok_or(ProtocolError::ArithmeticOverflow)?;
        let last = user.lifetime_service_units.checked_add(units as u128).ok_or(ProtocolError::ArithmeticOverflow)?;
        user.lifetime_service_units = last;
        user.next_batch_index = user.next_batch_index.checked_add(1).ok_or(ProtocolError::ArithmeticOverflow)?;
        user.qualification_progress_units = user.qualification_progress_units.checked_add(units).ok_or(ProtocolError::ArithmeticOverflow)?;

        if user.qualification_progress_units >= ACTIVITY_THRESHOLD_UNITS {
            let was_grace = activity_status(user, now) == ActivityStatus::Grace;
            user.qualification_progress_units = 0;
            user.active_until = now.checked_add(ACTIVE_SECONDS).ok_or(ProtocolError::ArithmeticOverflow)?;
            user.grace_until = user.active_until.checked_add(GRACE_SECONDS).ok_or(ProtocolError::ArithmeticOverflow)?;
            if was_grace { vest_pending(user)?; }
        }

        let batch = &mut ctx.accounts.batch;
        batch.bump = ctx.bumps.batch;
        batch.owner = ctx.accounts.wallet.key();
        batch.batch_index = user.next_batch_index - 1;
        batch.mint = ctx.accounts.user_source.mint;
        batch.units = units;
        batch.first_local_unit_index = first;
        batch.last_local_unit_index = last;
        batch.purchased_at = now;
        Ok(())
    }

    pub fn record_qualified_revenue<'info>(ctx: Context<'info, RecordQualifiedRevenue<'info>>, amount: u64) -> Result<()> {
        require!(amount > 0, ProtocolError::ZeroAmount);
        let now = Clock::get()?.unix_timestamp;
        let p = &mut ctx.accounts.protocol;
        require!(ctx.accounts.revenue_source.key() == p.qualified_revenue_source, ProtocolError::InvalidRevenueSource);
        require!(ctx.accounts.source_token.owner == ctx.accounts.revenue_source.key(), ProtocolError::WrongTokenAuthority);
        validate_vault_token(p, &ctx.accounts.source_token, &ctx.accounts.vault_token, ctx.accounts.vault_authority.key())?;
        validate_treasury_token(p, &ctx.accounts.source_token, &ctx.accounts.service_treasury_token)?;

        token::transfer(
            CpiContext::new(
                token::ID,
                Transfer {
                    from: ctx.accounts.source_token.to_account_info(),
                    to: ctx.accounts.vault_token.to_account_info(),
                    authority: ctx.accounts.revenue_source.to_account_info(),
                },
            ),
            amount,
        )?;

        let (direct, levels, pioneer, service, rounding_remainder) = split_amount(amount)?;
        add_direct(&mut ctx.accounts.beneficiary, p, ctx.accounts.source_token.mint, direct)?;

        let uplines = [
            ctx.accounts.upline_1.as_ref(), ctx.accounts.upline_2.as_ref(),
            ctx.accounts.upline_3.as_ref(), ctx.accounts.upline_4.as_ref(),
            ctx.accounts.upline_5.as_ref(), ctx.accounts.upline_6.as_ref(),
            ctx.accounts.upline_7.as_ref(), ctx.accounts.upline_8.as_ref(),
            ctx.accounts.upline_9.as_ref(), ctx.accounts.upline_10.as_ref(),
        ];
        let mut expected_wallet = ctx.accounts.beneficiary.referrer;
        let mut treasury_now = service.checked_add(rounding_remainder).ok_or(ProtocolError::ArithmeticOverflow)?;

        for i in 0..10 {
            let ai = &uplines[i];
            let expected_key = Pubkey::find_program_address(&[b"user", expected_wallet.as_ref()], &crate::ID).0;
            require!(ai.key() == expected_key, ProtocolError::InvalidUpline);
            let mut upline: Account<UserState> = Account::try_from(ai)?;
            require!(upline.wallet == expected_wallet, ProtocolError::InvalidUpline);

            if upline.is_technical_root() {
                for remaining in levels.iter().skip(i) {
                    treasury_now = treasury_now.checked_add(*remaining).ok_or(ProtocolError::ArithmeticOverflow)?;
                }
                break;
            }

            settle_expired_in_memory(&mut upline, now, p)?;
            match activity_status(&upline, now) {
                ActivityStatus::Active => add_network_claimable(&mut upline, p, ctx.accounts.source_token.mint, levels[i])?,
                ActivityStatus::Grace => add_network_pending(&mut upline, p, ctx.accounts.source_token.mint, levels[i])?,
                ActivityStatus::Inactive => {
                    treasury_now = treasury_now.checked_add(levels[i]).ok_or(ProtocolError::ArithmeticOverflow)?;
                    add_protocol_expired(p, ctx.accounts.source_token.mint, levels[i])?;
                }
            }
            expected_wallet = upline.referrer;
            upline.exit(&crate::ID)?;
        }

        accrue_pioneer(p, ctx.accounts.source_token.mint, pioneer)?;

        if treasury_now > 0 {
            transfer_from_vault(
                p,
                &ctx.accounts.vault_authority,
                &ctx.accounts.vault_token,
                &ctx.accounts.service_treasury_token,
                &ctx.accounts.token_program,
                treasury_now,
            )?;
            add_protocol_service_flow(p, ctx.accounts.source_token.mint, service, treasury_now.saturating_sub(service))?;
        }
        Ok(())
    }

    pub fn claim(ctx: Context<Claim>) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        let p = &ctx.accounts.protocol;
        let user = &mut ctx.accounts.user;
        require!(activity_status(user, now) == ActivityStatus::Active, ProtocolError::NotActive);
        require!(ctx.accounts.destination.owner == ctx.accounts.wallet.key(), ProtocolError::WrongTokenAuthority);
        validate_vault_token(p, &ctx.accounts.destination, &ctx.accounts.vault_token, ctx.accounts.vault_authority.key())?;

        let mint = ctx.accounts.destination.mint;
        let pioneer_due = pioneer_due(user, p, mint)?;
        let (direct, network) = take_claimable(user, p, mint)?;
        let total = direct.checked_add(network).ok_or(ProtocolError::ArithmeticOverflow)?
            .checked_add(pioneer_due).ok_or(ProtocolError::ArithmeticOverflow)?;
        require!(total > 0, ProtocolError::NothingToClaim);

        checkpoint_pioneer(user, p, mint)?;
        transfer_from_vault(p, &ctx.accounts.vault_authority, &ctx.accounts.vault_token, &ctx.accounts.destination, &ctx.accounts.token_program, total)?;
        if mint == p.usdt_mint {
            user.lifetime_claimed_usdt = user.lifetime_claimed_usdt.checked_add(total as u128).ok_or(ProtocolError::ArithmeticOverflow)?;
        } else {
            user.lifetime_claimed_usdc = user.lifetime_claimed_usdc.checked_add(total as u128).ok_or(ProtocolError::ArithmeticOverflow)?;
        }
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)] pub initializer: Signer<'info>,
    /// CHECK: immutable treasury address stored in ProtocolState.
    pub service_treasury: UncheckedAccount<'info>,
    /// CHECK: mint key stored in ProtocolState.
    pub usdt_mint: UncheckedAccount<'info>,
    /// CHECK: mint key stored in ProtocolState.
    pub usdc_mint: UncheckedAccount<'info>,
    #[account(init, payer = initializer, seeds = [b"protocol"], bump, space = ProtocolState::SPACE)]
    pub protocol: Account<'info, ProtocolState>,
    /// CHECK: PDA authority has no private key.
    #[account(seeds = [b"vault-authority"], bump)] pub vault_authority: UncheckedAccount<'info>,
    #[account(init, payer = initializer, seeds = [b"user", Pubkey::default().as_ref()], bump, space = UserState::SPACE)]
    pub technical_root: Account<'info, UserState>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Register<'info> {
    #[account(mut)] pub wallet: Signer<'info>,
    #[account(mut, seeds = [b"protocol"], bump = protocol.bump)] pub protocol: Account<'info, ProtocolState>,
    /// CHECK: wallet key verified against referrer state.
    pub referrer_wallet: UncheckedAccount<'info>,
    #[account(seeds = [b"user", referrer_wallet.key().as_ref()], bump = referrer.bump)] pub referrer: Account<'info, UserState>,
    #[account(init, payer = wallet, seeds = [b"user", wallet.key().as_ref()], bump, space = UserState::SPACE)] pub user: Account<'info, UserState>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct PurchaseServiceUnits<'info> {
    #[account(mut)] pub wallet: Signer<'info>,
    #[account(mut, seeds = [b"protocol"], bump = protocol.bump)] pub protocol: Account<'info, ProtocolState>,
    #[account(mut, seeds = [b"user", wallet.key().as_ref()], bump = user.bump)] pub user: Account<'info, UserState>,
    #[account(mut)] pub user_source: Account<'info, TokenAccount>,
    #[account(mut)] pub service_treasury_token: Account<'info, TokenAccount>,
    #[account(init, payer = wallet, seeds = [b"batch", wallet.key().as_ref(), &user.next_batch_index.to_le_bytes()], bump, space = UnitBatch::SPACE)]
    pub batch: Account<'info, UnitBatch>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RecordQualifiedRevenue<'info> {
    pub revenue_source: Signer<'info>,
    #[account(mut, seeds = [b"protocol"], bump = protocol.bump)] pub protocol: Account<'info, ProtocolState>,
    #[account(mut)] pub source_token: Account<'info, TokenAccount>,
    /// CHECK: PDA authority validated by seeds.
    #[account(seeds = [b"vault-authority"], bump = protocol.vault_authority_bump)] pub vault_authority: UncheckedAccount<'info>,
    #[account(mut)] pub vault_token: Account<'info, TokenAccount>,
    #[account(mut)] pub service_treasury_token: Account<'info, TokenAccount>,
    #[account(mut)] pub beneficiary: Account<'info, UserState>,
    /// CHECK: verified dynamically against immutable ancestry.
    #[account(mut)] pub upline_1: UncheckedAccount<'info>,
    /// CHECK: verified dynamically against immutable ancestry.
    #[account(mut)] pub upline_2: UncheckedAccount<'info>,
    /// CHECK: verified dynamically against immutable ancestry.
    #[account(mut)] pub upline_3: UncheckedAccount<'info>,
    /// CHECK: verified dynamically against immutable ancestry.
    #[account(mut)] pub upline_4: UncheckedAccount<'info>,
    /// CHECK: verified dynamically against immutable ancestry.
    #[account(mut)] pub upline_5: UncheckedAccount<'info>,
    /// CHECK: verified dynamically against immutable ancestry.
    #[account(mut)] pub upline_6: UncheckedAccount<'info>,
    /// CHECK: verified dynamically against immutable ancestry.
    #[account(mut)] pub upline_7: UncheckedAccount<'info>,
    /// CHECK: verified dynamically against immutable ancestry.
    #[account(mut)] pub upline_8: UncheckedAccount<'info>,
    /// CHECK: verified dynamically against immutable ancestry.
    #[account(mut)] pub upline_9: UncheckedAccount<'info>,
    /// CHECK: verified dynamically against immutable ancestry.
    #[account(mut)] pub upline_10: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Claim<'info> {
    #[account(mut)] pub wallet: Signer<'info>,
    #[account(seeds = [b"protocol"], bump = protocol.bump)] pub protocol: Account<'info, ProtocolState>,
    #[account(mut, seeds = [b"user", wallet.key().as_ref()], bump = user.bump)] pub user: Account<'info, UserState>,
    /// CHECK: PDA authority validated by seeds.
    #[account(seeds = [b"vault-authority"], bump = protocol.vault_authority_bump)] pub vault_authority: UncheckedAccount<'info>,
    #[account(mut)] pub vault_token: Account<'info, TokenAccount>,
    #[account(mut)] pub destination: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

fn validate_supported_token(p: &ProtocolState, source: &TokenAccount, treasury: &TokenAccount) -> Result<()> {
    require!(source.mint == p.usdt_mint || source.mint == p.usdc_mint, ProtocolError::UnsupportedToken);
    require!(treasury.mint == source.mint, ProtocolError::MintMismatch);
    require!(treasury.owner == p.service_treasury, ProtocolError::WrongTreasury);
    Ok(())
}

fn validate_vault_token(p: &ProtocolState, reference: &TokenAccount, vault: &TokenAccount, vault_authority: Pubkey) -> Result<()> {
    require!(reference.mint == p.usdt_mint || reference.mint == p.usdc_mint, ProtocolError::UnsupportedToken);
    require!(vault.mint == reference.mint, ProtocolError::MintMismatch);
    require!(vault.owner == vault_authority, ProtocolError::WrongVaultAuthority);
    Ok(())
}

fn validate_treasury_token(p: &ProtocolState, reference: &TokenAccount, treasury: &TokenAccount) -> Result<()> {
    require!(treasury.mint == reference.mint, ProtocolError::MintMismatch);
    require!(treasury.owner == p.service_treasury, ProtocolError::WrongTreasury);
    Ok(())
}

fn add_direct(user: &mut UserState, p: &ProtocolState, mint: Pubkey, amount: u64) -> Result<()> {
    if mint == p.usdt_mint {
        user.direct_accrued_usdt = user.direct_accrued_usdt.checked_add(amount).ok_or(ProtocolError::ArithmeticOverflow)?;
    } else if mint == p.usdc_mint {
        user.direct_accrued_usdc = user.direct_accrued_usdc.checked_add(amount).ok_or(ProtocolError::ArithmeticOverflow)?;
    } else { return err!(ProtocolError::UnsupportedToken); }
    Ok(())
}

fn add_network_claimable(user: &mut UserState, p: &ProtocolState, mint: Pubkey, amount: u64) -> Result<()> {
    if mint == p.usdt_mint {
        user.network_claimable_usdt = user.network_claimable_usdt.checked_add(amount).ok_or(ProtocolError::ArithmeticOverflow)?;
    } else if mint == p.usdc_mint {
        user.network_claimable_usdc = user.network_claimable_usdc.checked_add(amount).ok_or(ProtocolError::ArithmeticOverflow)?;
    } else { return err!(ProtocolError::UnsupportedToken); }
    Ok(())
}

fn add_network_pending(user: &mut UserState, p: &ProtocolState, mint: Pubkey, amount: u64) -> Result<()> {
    if mint == p.usdt_mint {
        user.network_pending_usdt = user.network_pending_usdt.checked_add(amount).ok_or(ProtocolError::ArithmeticOverflow)?;
    } else if mint == p.usdc_mint {
        user.network_pending_usdc = user.network_pending_usdc.checked_add(amount).ok_or(ProtocolError::ArithmeticOverflow)?;
    } else { return err!(ProtocolError::UnsupportedToken); }
    Ok(())
}

fn vest_pending(user: &mut UserState) -> Result<()> {
    user.network_claimable_usdt = user.network_claimable_usdt.checked_add(user.network_pending_usdt).ok_or(ProtocolError::ArithmeticOverflow)?;
    user.network_claimable_usdc = user.network_claimable_usdc.checked_add(user.network_pending_usdc).ok_or(ProtocolError::ArithmeticOverflow)?;
    user.network_pending_usdt = 0;
    user.network_pending_usdc = 0;
    Ok(())
}

fn settle_expired_in_memory(user: &mut UserState, now: i64, p: &mut ProtocolState) -> Result<()> {
    if user.grace_until > 0 && now > user.grace_until {
        if user.network_pending_usdt > 0 {
            let a = user.network_pending_usdt;
            user.network_pending_usdt = 0;
            user.lifetime_expired_usdt = user.lifetime_expired_usdt.checked_add(a as u128).ok_or(ProtocolError::ArithmeticOverflow)?;
            p.lifetime_expired_usdt = p.lifetime_expired_usdt.checked_add(a as u128).ok_or(ProtocolError::ArithmeticOverflow)?;
        }
        if user.network_pending_usdc > 0 {
            let a = user.network_pending_usdc;
            user.network_pending_usdc = 0;
            user.lifetime_expired_usdc = user.lifetime_expired_usdc.checked_add(a as u128).ok_or(ProtocolError::ArithmeticOverflow)?;
            p.lifetime_expired_usdc = p.lifetime_expired_usdc.checked_add(a as u128).ok_or(ProtocolError::ArithmeticOverflow)?;
        }
    }
    Ok(())
}

fn accrue_pioneer(p: &mut ProtocolState, mint: Pubkey, amount: u64) -> Result<()> {
    let per_share = amount / (PIONEER_SLOTS as u64);
    let remainder = amount % (PIONEER_SLOTS as u64);
    let unassigned = (PIONEER_SLOTS - p.pioneer_count) as u64;
    let reserve_add = per_share.checked_mul(unassigned).ok_or(ProtocolError::ArithmeticOverflow)?.checked_add(remainder).ok_or(ProtocolError::ArithmeticOverflow)?;
    if mint == p.usdt_mint {
        p.pioneer_index_usdt = p.pioneer_index_usdt.checked_add(per_share as u128).ok_or(ProtocolError::ArithmeticOverflow)?;
        p.pioneer_reserve_usdt = p.pioneer_reserve_usdt.checked_add(reserve_add).ok_or(ProtocolError::ArithmeticOverflow)?;
    } else if mint == p.usdc_mint {
        p.pioneer_index_usdc = p.pioneer_index_usdc.checked_add(per_share as u128).ok_or(ProtocolError::ArithmeticOverflow)?;
        p.pioneer_reserve_usdc = p.pioneer_reserve_usdc.checked_add(reserve_add).ok_or(ProtocolError::ArithmeticOverflow)?;
    } else { return err!(ProtocolError::UnsupportedToken); }
    Ok(())
}

fn pioneer_due(user: &UserState, p: &ProtocolState, mint: Pubkey) -> Result<u64> {
    if user.pioneer_id == 0 { return Ok(0); }
    let (idx, checkpoint) = if mint == p.usdt_mint {
        (p.pioneer_index_usdt, user.pioneer_checkpoint_usdt)
    } else if mint == p.usdc_mint {
        (p.pioneer_index_usdc, user.pioneer_checkpoint_usdc)
    } else { return err!(ProtocolError::UnsupportedToken); };
    let diff = idx.checked_sub(checkpoint).ok_or(ProtocolError::ArithmeticUnderflow)?;
    u64::try_from(diff).map_err(|_| ProtocolError::ArithmeticOverflow.into())
}

fn checkpoint_pioneer(user: &mut UserState, p: &ProtocolState, mint: Pubkey) -> Result<()> {
    if mint == p.usdt_mint { user.pioneer_checkpoint_usdt = p.pioneer_index_usdt; }
    else if mint == p.usdc_mint { user.pioneer_checkpoint_usdc = p.pioneer_index_usdc; }
    else { return err!(ProtocolError::UnsupportedToken); }
    Ok(())
}

fn take_claimable(user: &mut UserState, p: &ProtocolState, mint: Pubkey) -> Result<(u64, u64)> {
    if mint == p.usdt_mint {
        let d = user.direct_accrued_usdt;
        let n = user.network_claimable_usdt;
        user.direct_accrued_usdt = 0;
        user.network_claimable_usdt = 0;
        Ok((d, n))
    } else if mint == p.usdc_mint {
        let d = user.direct_accrued_usdc;
        let n = user.network_claimable_usdc;
        user.direct_accrued_usdc = 0;
        user.network_claimable_usdc = 0;
        Ok((d, n))
    } else { err!(ProtocolError::UnsupportedToken) }
}

fn add_protocol_expired(p: &mut ProtocolState, mint: Pubkey, amount: u64) -> Result<()> {
    if mint == p.usdt_mint {
        p.lifetime_expired_usdt = p.lifetime_expired_usdt.checked_add(amount as u128).ok_or(ProtocolError::ArithmeticOverflow)?;
    } else if mint == p.usdc_mint {
        p.lifetime_expired_usdc = p.lifetime_expired_usdc.checked_add(amount as u128).ok_or(ProtocolError::ArithmeticOverflow)?;
    } else { return err!(ProtocolError::UnsupportedToken); }
    Ok(())
}

fn add_protocol_service_flow(p: &mut ProtocolState, mint: Pubkey, service: u64, other: u64) -> Result<()> {
    if mint == p.usdt_mint {
        p.lifetime_service_fees_usdt = p.lifetime_service_fees_usdt.checked_add(service as u128).ok_or(ProtocolError::ArithmeticOverflow)?;
        p.lifetime_unallocated_usdt = p.lifetime_unallocated_usdt.checked_add(other as u128).ok_or(ProtocolError::ArithmeticOverflow)?;
    } else if mint == p.usdc_mint {
        p.lifetime_service_fees_usdc = p.lifetime_service_fees_usdc.checked_add(service as u128).ok_or(ProtocolError::ArithmeticOverflow)?;
        p.lifetime_unallocated_usdc = p.lifetime_unallocated_usdc.checked_add(other as u128).ok_or(ProtocolError::ArithmeticOverflow)?;
    } else { return err!(ProtocolError::UnsupportedToken); }
    Ok(())
}

fn transfer_from_vault<'info>(
    p: &ProtocolState,
    vault_authority: &UncheckedAccount<'info>,
    vault: &Account<'info, TokenAccount>,
    destination: &Account<'info, TokenAccount>,
    token_program: &Program<'info, Token>,
    amount: u64,
) -> Result<()> {
    let bump = [p.vault_authority_bump];
    let signer_seeds: &[&[u8]] = &[b"vault-authority", &bump];
    token::transfer(
        CpiContext::new_with_signer(
            token::ID,
            Transfer {
                from: vault.to_account_info(),
                to: destination.to_account_info(),
                authority: vault_authority.to_account_info(),
            },
            &[signer_seeds],
        ),
        amount,
    )
}

#[error_code]
pub enum ProtocolError {
    #[msg("Arithmetic overflow")] ArithmeticOverflow,
    #[msg("Arithmetic underflow")] ArithmeticUnderflow,
    #[msg("Percentage constants invalid")] InvalidPercentages,
    #[msg("Registration timestamp must be in the future")] RegistrationOpenInPast,
    #[msg("Registration is not open")] RegistrationNotOpen,
    #[msg("Invalid qualified revenue source")] InvalidRevenueSource,
    #[msg("Referrer state does not match supplied wallet")] ReferrerMismatch,
    #[msg("Self referral is forbidden")] SelfReferral,
    #[msg("Units must be greater than zero")] ZeroUnits,
    #[msg("Batch amount exceeds SPL token u64 capacity")] BatchTooLarge,
    #[msg("Amount must be greater than zero")] ZeroAmount,
    #[msg("Unsupported token")] UnsupportedToken,
    #[msg("Token mint mismatch")] MintMismatch,
    #[msg("Wrong token authority")] WrongTokenAuthority,
    #[msg("Wrong service treasury token account")] WrongTreasury,
    #[msg("Wrong vault authority")] WrongVaultAuthority,
    #[msg("Invalid upline account or ancestry")] InvalidUpline,
    #[msg("User is not active")] NotActive,
    #[msg("Nothing to claim")] NothingToClaim,
}
