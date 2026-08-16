use anchor_lang::prelude::*;
use anchor_spl::associated_token::get_associated_token_address_with_program_id;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

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

    pub fn initialize(ctx: Context<Initialize>, registration_open_at: i64) -> Result<()> {
        require!(constants::percentages_valid(), ProtocolError::InvalidPercentages);
        require!(
            ctx.accounts.usdt_mint.decimals == TOKEN_DECIMALS as u8,
            ProtocolError::InvalidTokenDecimals
        );
        require!(
            ctx.accounts.usdc_mint.decimals == TOKEN_DECIMALS as u8,
            ProtocolError::InvalidTokenDecimals
        );
        require!(
            ctx.accounts.usdt_mint.key() != ctx.accounts.usdc_mint.key(),
            ProtocolError::DuplicateStablecoinMint
        );

        validate_initialization_environment(
            ctx.accounts.service_treasury.key(),
            ctx.accounts.usdt_mint.key(),
            ctx.accounts.usdc_mint.key(),
            registration_open_at,
        )?;

        let now = Clock::get()?.unix_timestamp;
        require!(registration_open_at >= now, ProtocolError::RegistrationOpenInPast);

        let p = &mut ctx.accounts.protocol;
        p.bump = ctx.bumps.protocol;
        p.vault_authority_bump = ctx.bumps.vault_authority;
        p.initialized_at = now;
        p.registration_open_at = registration_open_at;
        p.service_treasury = ctx.accounts.service_treasury.key();
        p.usdt_mint = ctx.accounts.usdt_mint.key();
        p.usdc_mint = ctx.accounts.usdc_mint.key();
        p.pioneer_count = 0;
        p.real_user_count = 0;
        p.next_unit_id = 1;
        p.pioneer_index_usdt = 0;
        p.pioneer_index_usdc = 0;
        p.pioneer_unassigned_remainder_usdt_scaled = 0;
        p.pioneer_unassigned_remainder_usdc_scaled = 0;
        p.lifetime_service_fees_usdt = 0;
        p.lifetime_service_fees_usdc = 0;
        p.lifetime_unallocated_usdt = 0;
        p.lifetime_unallocated_usdc = 0;
        p.lifetime_expired_usdt = 0;
        p.lifetime_expired_usdc = 0;
        p.lifetime_rounding_usdt = 0;
        p.lifetime_rounding_usdc = 0;
        p.lifetime_pioneer_unassigned_usdt = 0;
        p.lifetime_pioneer_unassigned_usdc = 0;

        let root = &mut ctx.accounts.technical_root;
        root.bump = ctx.bumps.technical_root;
        root.wallet = Pubkey::default();
        root.referrer = Pubkey::default();
        root.registered_at = now;
        root.pioneer_id = 0;
        root.active_until = 0;
        root.grace_until = 0;
        root.qualification_progress_units = 0;
        root.qualification_window_started_at = 0;
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
        require!(
            ctx.accounts.referrer.wallet == ctx.accounts.referrer_wallet.key(),
            ProtocolError::ReferrerMismatch
        );
        require!(
            ctx.accounts.wallet.key() != ctx.accounts.referrer_wallet.key(),
            ProtocolError::SelfReferral
        );

        let pioneer_id = if p.pioneer_count < PIONEER_SLOTS {
            p.pioneer_count = p
                .pioneer_count
                .checked_add(1)
                .ok_or(ProtocolError::ArithmeticOverflow)?;
            p.pioneer_count
        } else {
            0
        };
        p.real_user_count = p
            .real_user_count
            .checked_add(1)
            .ok_or(ProtocolError::ArithmeticOverflow)?;

        let u = &mut ctx.accounts.user;
        u.bump = ctx.bumps.user;
        u.wallet = ctx.accounts.wallet.key();
        u.referrer = ctx.accounts.referrer_wallet.key();
        u.registered_at = now;
        u.pioneer_id = pioneer_id;
        u.active_until = 0;
        u.grace_until = 0;
        u.qualification_progress_units = 0;
        u.qualification_window_started_at = 0;
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

    /// Final production economic path.
    ///
    /// One buyer-signed transaction:
    /// - transfers exactly `units * 1 stablecoin` into the canonical vault;
    /// - allocates globally unique units and updates buyer activity;
    /// - credits eligible L1 direct sponsor with 50%;
    /// - credits eligible genealogical L2-L10 with the frozen 43% schedule;
    /// - accrues the 2% Pioneer pool;
    /// - routes the 5% service allocation plus unallocated/expired value to treasury.
    ///
    /// ACTIVE rewards are claimable. GRACE preserves unclaimed value temporarily.
    /// Once a user is INACTIVE, all previously unclaimed direct/network/Pioneer
    /// value is treasury-destined and cannot be rescued by late reactivation.
    pub fn purchase_and_distribute<'info>(
        ctx: Context<'info, PurchaseAndDistribute<'info>>,
        units: u64,
    ) -> Result<()> {
        require!(units > 0, ProtocolError::ZeroUnits);
        let payment_u128 = (units as u128)
            .checked_mul(TOKEN_SCALE as u128)
            .ok_or(ProtocolError::ArithmeticOverflow)?;
        let payment = u64::try_from(payment_u128).map_err(|_| ProtocolError::BatchTooLarge)?;
        let now = Clock::get()?.unix_timestamp;
        let pre_status = activity_status(&ctx.accounts.user, now);
        let mint = ctx.accounts.user_source.mint;

        validate_supported_source(&ctx.accounts.protocol, &ctx.accounts.user_source)?;
        require!(
            ctx.accounts.user_source.owner == ctx.accounts.wallet.key(),
            ProtocolError::WrongTokenAuthority
        );
        validate_vault_token_for_mint(
            &ctx.accounts.protocol,
            ctx.accounts.protocol.usdt_mint,
            &ctx.accounts.usdt_vault,
            ctx.accounts.vault_authority.key(),
        )?;
        validate_vault_token_for_mint(
            &ctx.accounts.protocol,
            ctx.accounts.protocol.usdc_mint,
            &ctx.accounts.usdc_vault,
            ctx.accounts.vault_authority.key(),
        )?;
        validate_treasury_token_for_mint(
            &ctx.accounts.protocol,
            ctx.accounts.protocol.usdt_mint,
            &ctx.accounts.service_treasury_usdt,
        )?;
        validate_treasury_token_for_mint(
            &ctx.accounts.protocol,
            ctx.accounts.protocol.usdc_mint,
            &ctx.accounts.service_treasury_usdc,
        )?;
        validate_user_pda(&ctx.accounts.user)?;
        validate_user_pda(&ctx.accounts.direct_referrer)?;
        require!(
            ctx.accounts.direct_referrer.wallet == ctx.accounts.user.referrer,
            ProtocolError::ReferrerMismatch
        );

        // Expire all of the buyer's treasury-destined value before a purchase can
        // reactivate them. This makes late reactivation unable to rescue commissions.
        settle_expired_all(
            &mut ctx.accounts.user,
            now,
            &mut ctx.accounts.protocol,
            &ctx.accounts.vault_authority,
            &ctx.accounts.usdt_vault,
            &ctx.accounts.usdc_vault,
            &ctx.accounts.service_treasury_usdt,
            &ctx.accounts.service_treasury_usdc,
            &ctx.accounts.token_program,
        )?;

        let payment_destination = if mint == ctx.accounts.protocol.usdt_mint {
            ctx.accounts.usdt_vault.to_account_info()
        } else {
            ctx.accounts.usdc_vault.to_account_info()
        };
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.key(),
                Transfer {
                    from: ctx.accounts.user_source.to_account_info(),
                    to: payment_destination,
                    authority: ctx.accounts.wallet.to_account_info(),
                },
            ),
            payment,
        )?;

        let (first_unit_id, last_unit_id, next_unit_id) =
            allocate_unit_range(ctx.accounts.protocol.next_unit_id, units)?;
        ctx.accounts.protocol.next_unit_id = next_unit_id;
        update_activity_after_purchase(&mut ctx.accounts.user, units, now, pre_status)?;
        let batch_index = ctx
            .accounts
            .user
            .next_batch_index
            .checked_sub(1)
            .ok_or(ProtocolError::ArithmeticUnderflow)?;

        let batch = &mut ctx.accounts.batch;
        batch.bump = ctx.bumps.batch;
        batch.owner = ctx.accounts.wallet.key();
        batch.batch_index = batch_index;
        batch.mint = mint;
        batch.units = units;
        batch.first_unit_id = first_unit_id;
        batch.last_unit_id = last_unit_id;
        batch.purchased_at = now;

        let (direct, levels, pioneer, service, rounding_remainder) =
            split_purchase_amount(payment)?;
        let p = &mut ctx.accounts.protocol;
        let mut unallocated: u64 = 0;
        let mut expired_flow: u64 = 0;

        if ctx.accounts.direct_referrer.is_technical_root() {
            unallocated = unallocated
                .checked_add(direct)
                .ok_or(ProtocolError::ArithmeticOverflow)?;
            for level in levels {
                unallocated = unallocated
                    .checked_add(level)
                    .ok_or(ProtocolError::ArithmeticOverflow)?;
            }
        } else {
            // Settle any prior unclaimed value if L1 is already inactive.
            let prior_expired = expire_unclaimed_for_mint(
                &mut ctx.accounts.direct_referrer,
                now,
                p,
                mint,
            )?;
            expired_flow = expired_flow
                .checked_add(prior_expired)
                .ok_or(ProtocolError::ArithmeticOverflow)?;

            // L1 is the direct sponsor and receives 50% only while ACTIVE/GRACE.
            match activity_status(&ctx.accounts.direct_referrer, now) {
                ActivityStatus::Active | ActivityStatus::Grace => {
                    add_direct(&mut ctx.accounts.direct_referrer, p, mint, direct)?;
                }
                ActivityStatus::Inactive => {
                    mark_user_expired(&mut ctx.accounts.direct_referrer, p, mint, direct)?;
                    expired_flow = expired_flow
                        .checked_add(direct)
                        .ok_or(ProtocolError::ArithmeticOverflow)?;
                }
            }

            let uplines = [
                ctx.accounts.upline_1.as_ref(),
                ctx.accounts.upline_2.as_ref(),
                ctx.accounts.upline_3.as_ref(),
                ctx.accounts.upline_4.as_ref(),
                ctx.accounts.upline_5.as_ref(),
                ctx.accounts.upline_6.as_ref(),
                ctx.accounts.upline_7.as_ref(),
                ctx.accounts.upline_8.as_ref(),
                ctx.accounts.upline_9.as_ref(),
            ];
            let mut expected_wallet = ctx.accounts.direct_referrer.referrer;

            for i in 0..9 {
                let ai = &uplines[i];
                let expected_key = Pubkey::find_program_address(
                    &[b"user", expected_wallet.as_ref()],
                    &crate::ID,
                )
                .0;
                require!(ai.key() == expected_key, ProtocolError::InvalidUpline);
                let mut upline: Account<UserState> = Account::try_from(ai)?;
                require!(
                    upline.wallet == expected_wallet,
                    ProtocolError::InvalidUpline
                );

                if upline.is_technical_root() {
                    for remaining in levels.iter().skip(i) {
                        unallocated = unallocated
                            .checked_add(*remaining)
                            .ok_or(ProtocolError::ArithmeticOverflow)?;
                    }
                    break;
                }

                let previously_expired = expire_unclaimed_for_mint(&mut upline, now, p, mint)?;
                expired_flow = expired_flow
                    .checked_add(previously_expired)
                    .ok_or(ProtocolError::ArithmeticOverflow)?;

                match activity_status(&upline, now) {
                    ActivityStatus::Active => {
                        add_network_claimable(&mut upline, p, mint, levels[i])?
                    }
                    ActivityStatus::Grace => {
                        add_network_pending(&mut upline, p, mint, levels[i])?
                    }
                    ActivityStatus::Inactive => {
                        mark_user_expired(&mut upline, p, mint, levels[i])?;
                        expired_flow = expired_flow
                            .checked_add(levels[i])
                            .ok_or(ProtocolError::ArithmeticOverflow)?;
                    }
                }
                expected_wallet = upline.referrer;
                upline.exit(&crate::ID)?;
            }
        }

        let pioneer_unassigned = accrue_pioneer(p, mint, pioneer)?;
        let treasury_now = service
            .checked_add(rounding_remainder)
            .ok_or(ProtocolError::ArithmeticOverflow)?
            .checked_add(unallocated)
            .ok_or(ProtocolError::ArithmeticOverflow)?
            .checked_add(expired_flow)
            .ok_or(ProtocolError::ArithmeticOverflow)?
            .checked_add(pioneer_unassigned)
            .ok_or(ProtocolError::ArithmeticOverflow)?;

        if treasury_now > 0 {
            if mint == p.usdt_mint {
                transfer_from_vault(
                    p,
                    &ctx.accounts.vault_authority,
                    &ctx.accounts.usdt_vault,
                    &ctx.accounts.service_treasury_usdt,
                    &ctx.accounts.token_program,
                    treasury_now,
                )?;
            } else {
                transfer_from_vault(
                    p,
                    &ctx.accounts.vault_authority,
                    &ctx.accounts.usdc_vault,
                    &ctx.accounts.service_treasury_usdc,
                    &ctx.accounts.token_program,
                    treasury_now,
                )?;
            }
        }
        add_protocol_treasury_metrics(
            p,
            mint,
            service,
            unallocated,
            rounding_remainder,
            pioneer_unassigned,
        )?;
        Ok(())
    }

    /// Permissionless physical settlement of value that became treasury-destined
    /// because the user is inactive. The settler pays the Solana transaction fee.
    pub fn settle_expired(ctx: Context<SettleExpired>) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        validate_user_pda(&ctx.accounts.user)?;
        let mint = ctx.accounts.vault_token.mint;
        validate_vault_token_for_mint(
            &ctx.accounts.protocol,
            mint,
            &ctx.accounts.vault_token,
            ctx.accounts.vault_authority.key(),
        )?;
        validate_treasury_token_for_mint(
            &ctx.accounts.protocol,
            mint,
            &ctx.accounts.service_treasury_token,
        )?;

        let amount = expire_unclaimed_for_mint(
            &mut ctx.accounts.user,
            now,
            &mut ctx.accounts.protocol,
            mint,
        )?;
        require!(amount > 0, ProtocolError::NothingToSettle);
        transfer_from_vault(
            &ctx.accounts.protocol,
            &ctx.accounts.vault_authority,
            &ctx.accounts.vault_token,
            &ctx.accounts.service_treasury_token,
            &ctx.accounts.token_program,
            amount,
        )?;
        Ok(())
    }

    pub fn claim(ctx: Context<Claim>) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        let p = &ctx.accounts.protocol;
        let user = &mut ctx.accounts.user;
        require!(
            activity_status(user, now) == ActivityStatus::Active,
            ProtocolError::NotActive
        );
        validate_user_pda(user)?;

        let mint = ctx.accounts.destination.mint;
        validate_vault_token_for_mint(
            p,
            mint,
            &ctx.accounts.vault_token,
            ctx.accounts.vault_authority.key(),
        )?;
        validate_user_destination(ctx.accounts.wallet.key(), mint, &ctx.accounts.destination)?;

        let pioneer_due = pioneer_due(user, p, mint)?;
        let (direct, network) = take_claimable(user, p, mint)?;
        let total = direct
            .checked_add(network)
            .ok_or(ProtocolError::ArithmeticOverflow)?
            .checked_add(pioneer_due)
            .ok_or(ProtocolError::ArithmeticOverflow)?;
        require!(total > 0, ProtocolError::NothingToClaim);

        checkpoint_pioneer_claimed(user, p, mint, pioneer_due)?;
        transfer_from_vault(
            p,
            &ctx.accounts.vault_authority,
            &ctx.accounts.vault_token,
            &ctx.accounts.destination,
            &ctx.accounts.token_program,
            total,
        )?;
        if mint == p.usdt_mint {
            user.lifetime_claimed_usdt = user
                .lifetime_claimed_usdt
                .checked_add(total as u128)
                .ok_or(ProtocolError::ArithmeticOverflow)?;
        } else {
            user.lifetime_claimed_usdc = user
                .lifetime_claimed_usdc
                .checked_add(total as u128)
                .ok_or(ProtocolError::ArithmeticOverflow)?;
        }
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub initializer: Signer<'info>,
    /// CHECK: immutable treasury address stored in ProtocolState and production-pinned.
    pub service_treasury: UncheckedAccount<'info>,
    pub usdt_mint: Box<Account<'info, Mint>>,
    pub usdc_mint: Box<Account<'info, Mint>>,
    #[account(
        init,
        payer = initializer,
        seeds = [b"protocol"],
        bump,
        space = ProtocolState::SPACE
    )]
    pub protocol: Box<Account<'info, ProtocolState>>,
    /// CHECK: PDA authority has no private key.
    #[account(seeds = [b"vault-authority"], bump)]
    pub vault_authority: UncheckedAccount<'info>,
    #[account(
        init,
        payer = initializer,
        seeds = [b"user", Pubkey::default().as_ref()],
        bump,
        space = UserState::SPACE
    )]
    pub technical_root: Box<Account<'info, UserState>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Register<'info> {
    #[account(mut)]
    pub wallet: Signer<'info>,
    #[account(mut, seeds = [b"protocol"], bump = protocol.bump)]
    pub protocol: Box<Account<'info, ProtocolState>>,
    /// CHECK: wallet key verified against immutable referrer state.
    pub referrer_wallet: UncheckedAccount<'info>,
    #[account(
        seeds = [b"user", referrer_wallet.key().as_ref()],
        bump = referrer.bump
    )]
    pub referrer: Box<Account<'info, UserState>>,
    #[account(
        init,
        payer = wallet,
        seeds = [b"user", wallet.key().as_ref()],
        bump,
        space = UserState::SPACE
    )]
    pub user: Box<Account<'info, UserState>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct PurchaseAndDistribute<'info> {
    #[account(mut)]
    pub wallet: Signer<'info>,
    #[account(mut, seeds = [b"protocol"], bump = protocol.bump)]
    pub protocol: Box<Account<'info, ProtocolState>>,
    #[account(mut, seeds = [b"user", wallet.key().as_ref()], bump = user.bump)]
    pub user: Box<Account<'info, UserState>>,
    #[account(mut)]
    pub user_source: Box<Account<'info, TokenAccount>>,
    /// CHECK: deterministic PDA authority validated by seeds and canonical vault checks.
    #[account(seeds = [b"vault-authority"], bump = protocol.vault_authority_bump)]
    pub vault_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub usdt_vault: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub usdc_vault: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub service_treasury_usdt: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub service_treasury_usdc: Box<Account<'info, TokenAccount>>,
    /// Direct sponsor = genealogical L1.
    #[account(mut)]
    pub direct_referrer: Box<Account<'info, UserState>>,
    /// CHECK: genealogical L2; verified dynamically against immutable ancestry.
    #[account(mut)]
    pub upline_1: UncheckedAccount<'info>,
    /// CHECK: genealogical L3.
    #[account(mut)]
    pub upline_2: UncheckedAccount<'info>,
    /// CHECK: genealogical L4.
    #[account(mut)]
    pub upline_3: UncheckedAccount<'info>,
    /// CHECK: genealogical L5.
    #[account(mut)]
    pub upline_4: UncheckedAccount<'info>,
    /// CHECK: genealogical L6.
    #[account(mut)]
    pub upline_5: UncheckedAccount<'info>,
    /// CHECK: genealogical L7.
    #[account(mut)]
    pub upline_6: UncheckedAccount<'info>,
    /// CHECK: genealogical L8.
    #[account(mut)]
    pub upline_7: UncheckedAccount<'info>,
    /// CHECK: genealogical L9.
    #[account(mut)]
    pub upline_8: UncheckedAccount<'info>,
    /// CHECK: genealogical L10.
    #[account(mut)]
    pub upline_9: UncheckedAccount<'info>,
    #[account(
        init,
        payer = wallet,
        seeds = [b"batch", wallet.key().as_ref(), &user.next_batch_index.to_le_bytes()],
        bump,
        space = UnitBatch::SPACE
    )]
    pub batch: Box<Account<'info, UnitBatch>>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SettleExpired<'info> {
    #[account(mut)]
    pub settler: Signer<'info>,
    #[account(mut, seeds = [b"protocol"], bump = protocol.bump)]
    pub protocol: Box<Account<'info, ProtocolState>>,
    #[account(mut)]
    pub user: Box<Account<'info, UserState>>,
    /// CHECK: deterministic PDA authority validated by seeds and canonical vault checks.
    #[account(seeds = [b"vault-authority"], bump = protocol.vault_authority_bump)]
    pub vault_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub vault_token: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub service_treasury_token: Box<Account<'info, TokenAccount>>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Claim<'info> {
    #[account(mut)]
    pub wallet: Signer<'info>,
    #[account(seeds = [b"protocol"], bump = protocol.bump)]
    pub protocol: Box<Account<'info, ProtocolState>>,
    #[account(mut, seeds = [b"user", wallet.key().as_ref()], bump = user.bump)]
    pub user: Box<Account<'info, UserState>>,
    /// CHECK: deterministic PDA authority validated by seeds and canonical vault checks.
    #[account(seeds = [b"vault-authority"], bump = protocol.vault_authority_bump)]
    pub vault_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub vault_token: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub destination: Box<Account<'info, TokenAccount>>,
    pub token_program: Program<'info, Token>,
}

fn require_supported_mint(p: &ProtocolState, mint: Pubkey) -> Result<()> {
    require!(
        mint == p.usdt_mint || mint == p.usdc_mint,
        ProtocolError::UnsupportedToken
    );
    Ok(())
}

fn canonical_ata(owner: Pubkey, mint: Pubkey) -> Pubkey {
    get_associated_token_address_with_program_id(&owner, &mint, &token::ID)
}

fn validate_supported_source(p: &ProtocolState, source: &Account<TokenAccount>) -> Result<()> {
    require_supported_mint(p, source.mint)
}

fn validate_vault_token_for_mint(
    p: &ProtocolState,
    mint: Pubkey,
    vault: &Account<TokenAccount>,
    vault_authority: Pubkey,
) -> Result<()> {
    require_supported_mint(p, mint)?;
    require!(vault.mint == mint, ProtocolError::MintMismatch);
    require!(
        vault.owner == vault_authority,
        ProtocolError::WrongVaultAuthority
    );
    require!(
        vault.key() == canonical_ata(vault_authority, mint),
        ProtocolError::NonCanonicalTokenAccount
    );
    Ok(())
}

fn validate_treasury_token_for_mint(
    p: &ProtocolState,
    mint: Pubkey,
    treasury: &Account<TokenAccount>,
) -> Result<()> {
    require_supported_mint(p, mint)?;
    require!(treasury.mint == mint, ProtocolError::MintMismatch);
    require!(treasury.owner == p.service_treasury, ProtocolError::WrongTreasury);
    require!(
        treasury.key() == canonical_ata(p.service_treasury, mint),
        ProtocolError::NonCanonicalTokenAccount
    );
    Ok(())
}

fn validate_user_destination(
    wallet: Pubkey,
    mint: Pubkey,
    destination: &Account<TokenAccount>,
) -> Result<()> {
    require!(destination.mint == mint, ProtocolError::MintMismatch);
    require!(
        destination.owner == wallet,
        ProtocolError::WrongTokenAuthority
    );
    require!(
        destination.key() == canonical_ata(wallet, mint),
        ProtocolError::NonCanonicalTokenAccount
    );
    Ok(())
}

fn validate_user_pda(user: &Account<UserState>) -> Result<()> {
    let expected = Pubkey::find_program_address(&[b"user", user.wallet.as_ref()], &crate::ID).0;
    require!(user.key() == expected, ProtocolError::InvalidUserPda);
    Ok(())
}

fn update_activity_after_purchase(
    user: &mut UserState,
    units: u64,
    now: i64,
    pre_status: ActivityStatus,
) -> Result<()> {
    user.lifetime_service_units = user
        .lifetime_service_units
        .checked_add(units as u128)
        .ok_or(ProtocolError::ArithmeticOverflow)?;
    user.next_batch_index = user
        .next_batch_index
        .checked_add(1)
        .ok_or(ProtocolError::ArithmeticOverflow)?;

    if user.qualification_progress_units > 0
        && user.qualification_window_started_at > 0
        && now
            > user
                .qualification_window_started_at
                .checked_add(ACTIVE_SECONDS)
                .ok_or(ProtocolError::ArithmeticOverflow)?
    {
        user.qualification_progress_units = 0;
        user.qualification_window_started_at = 0;
    }
    if user.qualification_progress_units == 0 {
        user.qualification_window_started_at = now;
    }
    user.qualification_progress_units = user
        .qualification_progress_units
        .checked_add(units)
        .ok_or(ProtocolError::ArithmeticOverflow)?;

    if user.qualification_progress_units >= ACTIVITY_THRESHOLD_UNITS {
        user.qualification_progress_units = 0;
        user.qualification_window_started_at = 0;
        user.active_until = now
            .checked_add(ACTIVE_SECONDS)
            .ok_or(ProtocolError::ArithmeticOverflow)?;
        user.grace_until = user
            .active_until
            .checked_add(GRACE_SECONDS)
            .ok_or(ProtocolError::ArithmeticOverflow)?;
        if pre_status == ActivityStatus::Grace {
            vest_pending(user)?;
        }
    }
    Ok(())
}

fn add_direct(
    user: &mut UserState,
    p: &ProtocolState,
    mint: Pubkey,
    amount: u64,
) -> Result<()> {
    if mint == p.usdt_mint {
        user.direct_accrued_usdt = user
            .direct_accrued_usdt
            .checked_add(amount)
            .ok_or(ProtocolError::ArithmeticOverflow)?;
    } else if mint == p.usdc_mint {
        user.direct_accrued_usdc = user
            .direct_accrued_usdc
            .checked_add(amount)
            .ok_or(ProtocolError::ArithmeticOverflow)?;
    } else {
        return err!(ProtocolError::UnsupportedToken);
    }
    Ok(())
}

fn add_network_claimable(
    user: &mut UserState,
    p: &ProtocolState,
    mint: Pubkey,
    amount: u64,
) -> Result<()> {
    if mint == p.usdt_mint {
        user.network_claimable_usdt = user
            .network_claimable_usdt
            .checked_add(amount)
            .ok_or(ProtocolError::ArithmeticOverflow)?;
    } else if mint == p.usdc_mint {
        user.network_claimable_usdc = user
            .network_claimable_usdc
            .checked_add(amount)
            .ok_or(ProtocolError::ArithmeticOverflow)?;
    } else {
        return err!(ProtocolError::UnsupportedToken);
    }
    Ok(())
}

fn add_network_pending(
    user: &mut UserState,
    p: &ProtocolState,
    mint: Pubkey,
    amount: u64,
) -> Result<()> {
    if mint == p.usdt_mint {
        user.network_pending_usdt = user
            .network_pending_usdt
            .checked_add(amount)
            .ok_or(ProtocolError::ArithmeticOverflow)?;
    } else if mint == p.usdc_mint {
        user.network_pending_usdc = user
            .network_pending_usdc
            .checked_add(amount)
            .ok_or(ProtocolError::ArithmeticOverflow)?;
    } else {
        return err!(ProtocolError::UnsupportedToken);
    }
    Ok(())
}

fn vest_pending(user: &mut UserState) -> Result<()> {
    user.network_claimable_usdt = user
        .network_claimable_usdt
        .checked_add(user.network_pending_usdt)
        .ok_or(ProtocolError::ArithmeticOverflow)?;
    user.network_claimable_usdc = user
        .network_claimable_usdc
        .checked_add(user.network_pending_usdc)
        .ok_or(ProtocolError::ArithmeticOverflow)?;
    user.network_pending_usdt = 0;
    user.network_pending_usdc = 0;
    Ok(())
}

fn mark_user_expired(
    user: &mut UserState,
    p: &mut ProtocolState,
    mint: Pubkey,
    amount: u64,
) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }
    if mint == p.usdt_mint {
        user.lifetime_expired_usdt = user
            .lifetime_expired_usdt
            .checked_add(amount as u128)
            .ok_or(ProtocolError::ArithmeticOverflow)?;
        p.lifetime_expired_usdt = p
            .lifetime_expired_usdt
            .checked_add(amount as u128)
            .ok_or(ProtocolError::ArithmeticOverflow)?;
    } else if mint == p.usdc_mint {
        user.lifetime_expired_usdc = user
            .lifetime_expired_usdc
            .checked_add(amount as u128)
            .ok_or(ProtocolError::ArithmeticOverflow)?;
        p.lifetime_expired_usdc = p
            .lifetime_expired_usdc
            .checked_add(amount as u128)
            .ok_or(ProtocolError::ArithmeticOverflow)?;
    } else {
        return err!(ProtocolError::UnsupportedToken);
    }
    Ok(())
}

/// Converts every whole-token-atom liability for an INACTIVE user into treasury-
/// destined expired value for one mint. ACTIVE and GRACE users are untouched.
/// Pioneer fractional dust remains in the checkpoint difference until it becomes a
/// whole atomic unit; this preserves exact long-run conservation.
fn expire_unclaimed_for_mint(
    user: &mut UserState,
    now: i64,
    p: &mut ProtocolState,
    mint: Pubkey,
) -> Result<u64> {
    if activity_status(user, now) != ActivityStatus::Inactive {
        return Ok(0);
    }

    let pioneer = pioneer_due(user, p, mint)?;
    let (direct, network_claimable, network_pending) = if mint == p.usdt_mint {
        let values = (
            user.direct_accrued_usdt,
            user.network_claimable_usdt,
            user.network_pending_usdt,
        );
        user.direct_accrued_usdt = 0;
        user.network_claimable_usdt = 0;
        user.network_pending_usdt = 0;
        values
    } else if mint == p.usdc_mint {
        let values = (
            user.direct_accrued_usdc,
            user.network_claimable_usdc,
            user.network_pending_usdc,
        );
        user.direct_accrued_usdc = 0;
        user.network_claimable_usdc = 0;
        user.network_pending_usdc = 0;
        values
    } else {
        return err!(ProtocolError::UnsupportedToken);
    };

    let amount = direct
        .checked_add(network_claimable)
        .ok_or(ProtocolError::ArithmeticOverflow)?
        .checked_add(network_pending)
        .ok_or(ProtocolError::ArithmeticOverflow)?
        .checked_add(pioneer)
        .ok_or(ProtocolError::ArithmeticOverflow)?;

    if pioneer > 0 {
        checkpoint_pioneer_claimed(user, p, mint, pioneer)?;
    }
    mark_user_expired(user, p, mint, amount)?;
    Ok(amount)
}

#[allow(clippy::too_many_arguments)]
fn settle_expired_all<'info>(
    user: &mut UserState,
    now: i64,
    p: &mut ProtocolState,
    vault_authority: &UncheckedAccount<'info>,
    usdt_vault: &Box<Account<'info, TokenAccount>>,
    usdc_vault: &Box<Account<'info, TokenAccount>>,
    treasury_usdt: &Box<Account<'info, TokenAccount>>,
    treasury_usdc: &Box<Account<'info, TokenAccount>>,
    token_program: &Program<'info, Token>,
) -> Result<()> {
    let usdt_mint = p.usdt_mint;
    let usdc_mint = p.usdc_mint;
    let usdt = expire_unclaimed_for_mint(user, now, p, usdt_mint)?;
    let usdc = expire_unclaimed_for_mint(user, now, p, usdc_mint)?;
    if usdt > 0 {
        transfer_from_vault(
            p,
            vault_authority,
            usdt_vault,
            treasury_usdt,
            token_program,
            usdt,
        )?;
    }
    if usdc > 0 {
        transfer_from_vault(
            p,
            vault_authority,
            usdc_vault,
            treasury_usdc,
            token_program,
            usdc,
        )?;
    }
    Ok(())
}

fn accrue_pioneer(p: &mut ProtocolState, mint: Pubkey, amount: u64) -> Result<u64> {
    let per_share_scaled = (amount as u128)
        .checked_mul(PIONEER_SCALE)
        .ok_or(ProtocolError::ArithmeticOverflow)?
        .checked_div(PIONEER_SLOTS as u128)
        .ok_or(ProtocolError::ArithmeticUnderflow)?;
    let unassigned = (PIONEER_SLOTS - p.pioneer_count) as u128;
    let unassigned_scaled = per_share_scaled
        .checked_mul(unassigned)
        .ok_or(ProtocolError::ArithmeticOverflow)?;

    let remainder_scaled = if mint == p.usdt_mint {
        p.pioneer_index_usdt = p
            .pioneer_index_usdt
            .checked_add(per_share_scaled)
            .ok_or(ProtocolError::ArithmeticOverflow)?;
        p.pioneer_unassigned_remainder_usdt_scaled
            .checked_add(unassigned_scaled)
            .ok_or(ProtocolError::ArithmeticOverflow)?
    } else if mint == p.usdc_mint {
        p.pioneer_index_usdc = p
            .pioneer_index_usdc
            .checked_add(per_share_scaled)
            .ok_or(ProtocolError::ArithmeticOverflow)?;
        p.pioneer_unassigned_remainder_usdc_scaled
            .checked_add(unassigned_scaled)
            .ok_or(ProtocolError::ArithmeticOverflow)?
    } else {
        return err!(ProtocolError::UnsupportedToken);
    };

    let whole_atomic = remainder_scaled / PIONEER_SCALE;
    let fractional_scaled = remainder_scaled % PIONEER_SCALE;
    if mint == p.usdt_mint {
        p.pioneer_unassigned_remainder_usdt_scaled = fractional_scaled;
    } else {
        p.pioneer_unassigned_remainder_usdc_scaled = fractional_scaled;
    }
    u64::try_from(whole_atomic).map_err(|_| ProtocolError::ArithmeticOverflow.into())
}

fn pioneer_due(user: &UserState, p: &ProtocolState, mint: Pubkey) -> Result<u64> {
    if user.pioneer_id == 0 {
        return Ok(0);
    }
    let (index, checkpoint) = if mint == p.usdt_mint {
        (p.pioneer_index_usdt, user.pioneer_checkpoint_usdt)
    } else if mint == p.usdc_mint {
        (p.pioneer_index_usdc, user.pioneer_checkpoint_usdc)
    } else {
        return err!(ProtocolError::UnsupportedToken);
    };
    let diff_scaled = index
        .checked_sub(checkpoint)
        .ok_or(ProtocolError::ArithmeticUnderflow)?;
    let due = diff_scaled / PIONEER_SCALE;
    u64::try_from(due).map_err(|_| ProtocolError::ArithmeticOverflow.into())
}

fn checkpoint_pioneer_claimed(
    user: &mut UserState,
    p: &ProtocolState,
    mint: Pubkey,
    claimed: u64,
) -> Result<()> {
    if user.pioneer_id == 0 || claimed == 0 {
        return Ok(());
    }
    let advance_scaled = (claimed as u128)
        .checked_mul(PIONEER_SCALE)
        .ok_or(ProtocolError::ArithmeticOverflow)?;
    if mint == p.usdt_mint {
        user.pioneer_checkpoint_usdt = user
            .pioneer_checkpoint_usdt
            .checked_add(advance_scaled)
            .ok_or(ProtocolError::ArithmeticOverflow)?;
        require!(
            user.pioneer_checkpoint_usdt <= p.pioneer_index_usdt,
            ProtocolError::ArithmeticUnderflow
        );
    } else if mint == p.usdc_mint {
        user.pioneer_checkpoint_usdc = user
            .pioneer_checkpoint_usdc
            .checked_add(advance_scaled)
            .ok_or(ProtocolError::ArithmeticOverflow)?;
        require!(
            user.pioneer_checkpoint_usdc <= p.pioneer_index_usdc,
            ProtocolError::ArithmeticUnderflow
        );
    } else {
        return err!(ProtocolError::UnsupportedToken);
    }
    Ok(())
}

fn take_claimable(
    user: &mut UserState,
    p: &ProtocolState,
    mint: Pubkey,
) -> Result<(u64, u64)> {
    if mint == p.usdt_mint {
        let direct = user.direct_accrued_usdt;
        let network = user.network_claimable_usdt;
        user.direct_accrued_usdt = 0;
        user.network_claimable_usdt = 0;
        Ok((direct, network))
    } else if mint == p.usdc_mint {
        let direct = user.direct_accrued_usdc;
        let network = user.network_claimable_usdc;
        user.direct_accrued_usdc = 0;
        user.network_claimable_usdc = 0;
        Ok((direct, network))
    } else {
        err!(ProtocolError::UnsupportedToken)
    }
}

fn add_protocol_treasury_metrics(
    p: &mut ProtocolState,
    mint: Pubkey,
    service: u64,
    unallocated: u64,
    rounding: u64,
    pioneer_unassigned: u64,
) -> Result<()> {
    if mint == p.usdt_mint {
        p.lifetime_service_fees_usdt = p
            .lifetime_service_fees_usdt
            .checked_add(service as u128)
            .ok_or(ProtocolError::ArithmeticOverflow)?;
        p.lifetime_unallocated_usdt = p
            .lifetime_unallocated_usdt
            .checked_add(unallocated as u128)
            .ok_or(ProtocolError::ArithmeticOverflow)?;
        p.lifetime_rounding_usdt = p
            .lifetime_rounding_usdt
            .checked_add(rounding as u128)
            .ok_or(ProtocolError::ArithmeticOverflow)?;
        p.lifetime_pioneer_unassigned_usdt = p
            .lifetime_pioneer_unassigned_usdt
            .checked_add(pioneer_unassigned as u128)
            .ok_or(ProtocolError::ArithmeticOverflow)?;
    } else if mint == p.usdc_mint {
        p.lifetime_service_fees_usdc = p
            .lifetime_service_fees_usdc
            .checked_add(service as u128)
            .ok_or(ProtocolError::ArithmeticOverflow)?;
        p.lifetime_unallocated_usdc = p
            .lifetime_unallocated_usdc
            .checked_add(unallocated as u128)
            .ok_or(ProtocolError::ArithmeticOverflow)?;
        p.lifetime_rounding_usdc = p
            .lifetime_rounding_usdc
            .checked_add(rounding as u128)
            .ok_or(ProtocolError::ArithmeticOverflow)?;
        p.lifetime_pioneer_unassigned_usdc = p
            .lifetime_pioneer_unassigned_usdc
            .checked_add(pioneer_unassigned as u128)
            .ok_or(ProtocolError::ArithmeticOverflow)?;
    } else {
        return err!(ProtocolError::UnsupportedToken);
    }
    Ok(())
}

fn transfer_from_vault<'info>(
    p: &ProtocolState,
    vault_authority: &UncheckedAccount<'info>,
    vault: &Box<Account<'info, TokenAccount>>,
    destination: &Box<Account<'info, TokenAccount>>,
    token_program: &Program<'info, Token>,
    amount: u64,
) -> Result<()> {
    let bump = [p.vault_authority_bump];
    let signer_seeds: &[&[u8]] = &[b"vault-authority", &bump];
    token::transfer(
        CpiContext::new_with_signer(
            token_program.key(),
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

fn validate_initialization_environment(
    service_treasury: Pubkey,
    usdt_mint: Pubkey,
    usdc_mint: Pubkey,
    registration_open_at: i64,
) -> Result<()> {
    #[cfg(feature = "production")]
    {
        require!(
            MAINNET_REGISTRATION_OPEN_AT > 0,
            ProtocolError::ProductionConfigNotFrozen
        );
        require_keys_eq!(
            service_treasury,
            MAINNET_SERVICE_TREASURY,
            ProtocolError::InvalidProductionConfig
        );
        require_keys_eq!(
            usdt_mint,
            MAINNET_USDT_MINT,
            ProtocolError::InvalidProductionConfig
        );
        require_keys_eq!(
            usdc_mint,
            MAINNET_USDC_MINT,
            ProtocolError::InvalidProductionConfig
        );
        require!(
            registration_open_at == MAINNET_REGISTRATION_OPEN_AT,
            ProtocolError::InvalidProductionConfig
        );
    }
    #[cfg(not(feature = "production"))]
    {
        let _ = (service_treasury, usdt_mint, usdc_mint, registration_open_at);
    }
    Ok(())
}

#[error_code]
pub enum ProtocolError {
    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,
    #[msg("Arithmetic underflow")]
    ArithmeticUnderflow,
    #[msg("Percentage constants invalid")]
    InvalidPercentages,
    #[msg("Registration timestamp must be in the future")]
    RegistrationOpenInPast,
    #[msg("Registration is not open")]
    RegistrationNotOpen,
    #[msg("Referrer state does not match supplied wallet")]
    ReferrerMismatch,
    #[msg("Self referral is forbidden")]
    SelfReferral,
    #[msg("Units must be greater than zero")]
    ZeroUnits,
    #[msg("Batch amount exceeds SPL token u64 capacity")]
    BatchTooLarge,
    #[msg("Unsupported token")]
    UnsupportedToken,
    #[msg("Token mint mismatch")]
    MintMismatch,
    #[msg("Wrong token authority")]
    WrongTokenAuthority,
    #[msg("Wrong service treasury token account")]
    WrongTreasury,
    #[msg("Wrong vault authority")]
    WrongVaultAuthority,
    #[msg("Invalid upline account or ancestry")]
    InvalidUpline,
    #[msg("User state is not at its canonical PDA")]
    InvalidUserPda,
    #[msg("Token account is not the canonical associated token account")]
    NonCanonicalTokenAccount,
    #[msg("No inactive unclaimed reward to settle")]
    NothingToSettle,
    #[msg("User is not active")]
    NotActive,
    #[msg("Nothing to claim")]
    NothingToClaim,
    #[msg("Production launch configuration has not been frozen")]
    ProductionConfigNotFrozen,
    #[msg("Production initialization does not match frozen mainnet configuration")]
    InvalidProductionConfig,
    #[msg("Supported stablecoin mint must use six decimals")]
    InvalidTokenDecimals,
    #[msg("USDT and USDC mint accounts must be distinct")]
    DuplicateStablecoinMint,
}
