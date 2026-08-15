from pathlib import Path

path = Path("programs/service_referral_protocol/src/lib.rs")
s = path.read_text()

if "use anchor_spl::associated_token::get_associated_token_address_with_program_id;" not in s:
    s = s.replace(
        "use anchor_lang::prelude::*;\n",
        "use anchor_lang::prelude::*;\nuse anchor_spl::associated_token::get_associated_token_address_with_program_id;\n",
        1,
    )

# Initialize newly separated treasury accounting buckets.
s = s.replace(
    "p.lifetime_expired_usdt = 0;\n        p.lifetime_expired_usdc = 0;",
    "p.lifetime_expired_usdt = 0;\n        p.lifetime_expired_usdc = 0;\n        p.lifetime_rounding_usdt = 0;\n        p.lifetime_rounding_usdc = 0;\n        p.lifetime_pioneer_unassigned_usdt = 0;\n        p.lifetime_pioneer_unassigned_usdc = 0;",
    1,
)

# Replace service-unit purchase with canonical ATA validation and physical expiry settlement.
start = s.index("    pub fn purchase_service_units(")
end = s.index("    pub fn record_qualified_revenue", start)
purchase = r'''    pub fn purchase_service_units(ctx: Context<PurchaseServiceUnits>, units: u64) -> Result<()> {
        require!(units > 0, ProtocolError::ZeroUnits);
        let payment_u128 = (units as u128).checked_mul(TOKEN_SCALE as u128).ok_or(ProtocolError::ArithmeticOverflow)?;
        let payment = u64::try_from(payment_u128).map_err(|_| ProtocolError::BatchTooLarge)?;
        let now = Clock::get()?.unix_timestamp;
        let pre_status = activity_status(&ctx.accounts.user, now);

        validate_supported_source(&ctx.accounts.protocol, &ctx.accounts.user_source)?;
        require!(ctx.accounts.user_source.owner == ctx.accounts.wallet.key(), ProtocolError::WrongTokenAuthority);
        validate_vault_token_for_mint(&ctx.accounts.protocol, ctx.accounts.protocol.usdt_mint, &ctx.accounts.usdt_vault, ctx.accounts.vault_authority.key())?;
        validate_vault_token_for_mint(&ctx.accounts.protocol, ctx.accounts.protocol.usdc_mint, &ctx.accounts.usdc_vault, ctx.accounts.vault_authority.key())?;
        validate_treasury_token_for_mint(&ctx.accounts.protocol, ctx.accounts.protocol.usdt_mint, &ctx.accounts.service_treasury_usdt)?;
        validate_treasury_token_for_mint(&ctx.accounts.protocol, ctx.accounts.protocol.usdc_mint, &ctx.accounts.service_treasury_usdc)?;

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

        let destination = if ctx.accounts.user_source.mint == ctx.accounts.protocol.usdt_mint {
            ctx.accounts.service_treasury_usdt.to_account_info()
        } else {
            ctx.accounts.service_treasury_usdc.to_account_info()
        };
        token::transfer(
            CpiContext::new(
                token::ID,
                Transfer {
                    from: ctx.accounts.user_source.to_account_info(),
                    to: destination,
                    authority: ctx.accounts.wallet.to_account_info(),
                },
            ),
            payment,
        )?;

        let user = &mut ctx.accounts.user;
        let first = user.lifetime_service_units.checked_add(1).ok_or(ProtocolError::ArithmeticOverflow)?;
        let last = user.lifetime_service_units.checked_add(units as u128).ok_or(ProtocolError::ArithmeticOverflow)?;
        user.lifetime_service_units = last;
        user.next_batch_index = user.next_batch_index.checked_add(1).ok_or(ProtocolError::ArithmeticOverflow)?;

        if user.qualification_progress_units > 0
            && user.qualification_window_started_at > 0
            && now > user.qualification_window_started_at.checked_add(ACTIVE_SECONDS).ok_or(ProtocolError::ArithmeticOverflow)?
        {
            user.qualification_progress_units = 0;
            user.qualification_window_started_at = 0;
        }
        if user.qualification_progress_units == 0 {
            user.qualification_window_started_at = now;
        }
        user.qualification_progress_units = user.qualification_progress_units.checked_add(units).ok_or(ProtocolError::ArithmeticOverflow)?;

        if user.qualification_progress_units >= ACTIVITY_THRESHOLD_UNITS {
            user.qualification_progress_units = 0;
            user.qualification_window_started_at = 0;
            user.active_until = now.checked_add(ACTIVE_SECONDS).ok_or(ProtocolError::ArithmeticOverflow)?;
            user.grace_until = user.active_until.checked_add(GRACE_SECONDS).ok_or(ProtocolError::ArithmeticOverflow)?;
            if pre_status == ActivityStatus::Grace { vest_pending(user)?; }
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

'''
s = s[:start] + purchase + s[end:]

# Replace qualified revenue processing with canonical account validation and separated treasury flows.
start = s.index("    pub fn record_qualified_revenue")
end = s.index("    pub fn claim(", start)
record_and_settle = r'''    pub fn record_qualified_revenue<'info>(ctx: Context<'info, RecordQualifiedRevenue<'info>>, amount: u64) -> Result<()> {
        require!(amount > 0, ProtocolError::ZeroAmount);
        let now = Clock::get()?.unix_timestamp;
        let p = &mut ctx.accounts.protocol;
        let mint = ctx.accounts.source_token.mint;
        require!(ctx.accounts.revenue_source.key() == p.qualified_revenue_source, ProtocolError::InvalidRevenueSource);
        require!(ctx.accounts.source_token.owner == ctx.accounts.revenue_source.key(), ProtocolError::WrongTokenAuthority);
        validate_supported_source(p, &ctx.accounts.source_token)?;
        validate_vault_token_for_mint(p, mint, &ctx.accounts.vault_token, ctx.accounts.vault_authority.key())?;
        validate_treasury_token_for_mint(p, mint, &ctx.accounts.service_treasury_token)?;
        validate_user_pda(&ctx.accounts.beneficiary)?;
        require!(!ctx.accounts.beneficiary.is_technical_root(), ProtocolError::InvalidBeneficiary);

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
        add_direct(&mut ctx.accounts.beneficiary, p, mint, direct)?;

        let uplines = [
            ctx.accounts.upline_1.as_ref(), ctx.accounts.upline_2.as_ref(),
            ctx.accounts.upline_3.as_ref(), ctx.accounts.upline_4.as_ref(),
            ctx.accounts.upline_5.as_ref(), ctx.accounts.upline_6.as_ref(),
            ctx.accounts.upline_7.as_ref(), ctx.accounts.upline_8.as_ref(),
            ctx.accounts.upline_9.as_ref(), ctx.accounts.upline_10.as_ref(),
        ];
        let mut expected_wallet = ctx.accounts.beneficiary.referrer;
        let mut unallocated_network: u64 = 0;
        let mut expired_flow: u64 = 0;

        for i in 0..10 {
            let ai = &uplines[i];
            let expected_key = Pubkey::find_program_address(&[b"user", expected_wallet.as_ref()], &crate::ID).0;
            require!(ai.key() == expected_key, ProtocolError::InvalidUpline);
            let mut upline: Account<UserState> = Account::try_from(ai)?;
            require!(upline.wallet == expected_wallet, ProtocolError::InvalidUpline);

            if upline.is_technical_root() {
                for remaining in levels.iter().skip(i) {
                    unallocated_network = unallocated_network.checked_add(*remaining).ok_or(ProtocolError::ArithmeticOverflow)?;
                }
                break;
            }

            let previously_expired = expire_pending_for_mint(&mut upline, now, p, mint)?;
            expired_flow = expired_flow.checked_add(previously_expired).ok_or(ProtocolError::ArithmeticOverflow)?;
            match activity_status(&upline, now) {
                ActivityStatus::Active => add_network_claimable(&mut upline, p, mint, levels[i])?,
                ActivityStatus::Grace => add_network_pending(&mut upline, p, mint, levels[i])?,
                ActivityStatus::Inactive => {
                    mark_user_expired(&mut upline, p, mint, levels[i])?;
                    expired_flow = expired_flow.checked_add(levels[i]).ok_or(ProtocolError::ArithmeticOverflow)?;
                }
            }
            expected_wallet = upline.referrer;
            upline.exit(&crate::ID)?;
        }

        let pioneer_unassigned = accrue_pioneer(p, mint, pioneer)?;
        let treasury_now = service
            .checked_add(rounding_remainder).ok_or(ProtocolError::ArithmeticOverflow)?
            .checked_add(unallocated_network).ok_or(ProtocolError::ArithmeticOverflow)?
            .checked_add(expired_flow).ok_or(ProtocolError::ArithmeticOverflow)?
            .checked_add(pioneer_unassigned).ok_or(ProtocolError::ArithmeticOverflow)?;

        if treasury_now > 0 {
            transfer_from_vault(
                p,
                &ctx.accounts.vault_authority,
                &ctx.accounts.vault_token,
                &ctx.accounts.service_treasury_token,
                &ctx.accounts.token_program,
                treasury_now,
            )?;
        }
        add_protocol_treasury_metrics(p, mint, service, unallocated_network, rounding_remainder, pioneer_unassigned)?;
        Ok(())
    }

    pub fn settle_expired(ctx: Context<SettleExpired>) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        validate_user_pda(&ctx.accounts.user)?;
        let mint = ctx.accounts.vault_token.mint;
        validate_vault_token_for_mint(&ctx.accounts.protocol, mint, &ctx.accounts.vault_token, ctx.accounts.vault_authority.key())?;
        validate_treasury_token_for_mint(&ctx.accounts.protocol, mint, &ctx.accounts.service_treasury_token)?;
        let amount = expire_pending_for_mint(&mut ctx.accounts.user, now, &mut ctx.accounts.protocol, mint)?;
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

'''
s = s[:start] + record_and_settle + s[end:]

# Replace claim with canonical destination/vault validation.
start = s.index("    pub fn claim(")
end = s.index("}\n\n#[derive(Accounts)]", start)
claim = r'''    pub fn claim(ctx: Context<Claim>) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        let p = &ctx.accounts.protocol;
        let user = &mut ctx.accounts.user;
        require!(activity_status(user, now) == ActivityStatus::Active, ProtocolError::NotActive);
        validate_user_pda(user)?;

        let mint = ctx.accounts.destination.mint;
        validate_vault_token_for_mint(p, mint, &ctx.accounts.vault_token, ctx.accounts.vault_authority.key())?;
        validate_user_destination(ctx.accounts.wallet.key(), mint, &ctx.accounts.destination)?;

        let pioneer_due = pioneer_due(user, p, mint)?;
        let (direct, network) = take_claimable(user, p, mint)?;
        let total = direct.checked_add(network).ok_or(ProtocolError::ArithmeticOverflow)?
            .checked_add(pioneer_due).ok_or(ProtocolError::ArithmeticOverflow)?;
        require!(total > 0, ProtocolError::NothingToClaim);

        checkpoint_pioneer_claimed(user, p, mint, pioneer_due)?;
        transfer_from_vault(p, &ctx.accounts.vault_authority, &ctx.accounts.vault_token, &ctx.accounts.destination, &ctx.accounts.token_program, total)?;
        if mint == p.usdt_mint {
            user.lifetime_claimed_usdt = user.lifetime_claimed_usdt.checked_add(total as u128).ok_or(ProtocolError::ArithmeticOverflow)?;
        } else {
            user.lifetime_claimed_usdc = user.lifetime_claimed_usdc.checked_add(total as u128).ok_or(ProtocolError::ArithmeticOverflow)?;
        }
        Ok(())
    }
'''
s = s[:start] + claim + s[end:]

# Replace PurchaseServiceUnits accounts and add permissionless settlement accounts.
start = s.index("#[derive(Accounts)]\npub struct PurchaseServiceUnits")
end = s.index("#[derive(Accounts)]\npub struct RecordQualifiedRevenue", start)
accounts = r'''#[derive(Accounts)]
pub struct PurchaseServiceUnits<'info> {
    #[account(mut)] pub wallet: Signer<'info>,
    #[account(mut, seeds = [b"protocol"], bump = protocol.bump)] pub protocol: Account<'info, ProtocolState>,
    #[account(mut, seeds = [b"user", wallet.key().as_ref()], bump = user.bump)] pub user: Account<'info, UserState>,
    #[account(mut)] pub user_source: Account<'info, TokenAccount>,
    /// CHECK: PDA authority validated by seeds and canonical vault checks.
    #[account(seeds = [b"vault-authority"], bump = protocol.vault_authority_bump)] pub vault_authority: UncheckedAccount<'info>,
    #[account(mut)] pub usdt_vault: Account<'info, TokenAccount>,
    #[account(mut)] pub usdc_vault: Account<'info, TokenAccount>,
    #[account(mut)] pub service_treasury_usdt: Account<'info, TokenAccount>,
    #[account(mut)] pub service_treasury_usdc: Account<'info, TokenAccount>,
    #[account(init, payer = wallet, seeds = [b"batch", wallet.key().as_ref(), &user.next_batch_index.to_le_bytes()], bump, space = UnitBatch::SPACE)]
    pub batch: Account<'info, UnitBatch>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SettleExpired<'info> {
    #[account(mut)] pub settler: Signer<'info>,
    #[account(mut, seeds = [b"protocol"], bump = protocol.bump)] pub protocol: Account<'info, ProtocolState>,
    #[account(mut)] pub user: Account<'info, UserState>,
    /// CHECK: PDA authority validated by seeds and canonical vault checks.
    #[account(seeds = [b"vault-authority"], bump = protocol.vault_authority_bump)] pub vault_authority: UncheckedAccount<'info>,
    #[account(mut)] pub vault_token: Account<'info, TokenAccount>,
    #[account(mut)] pub service_treasury_token: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

'''
s = s[:start] + accounts + s[end:]

# Replace token-account validators with exact canonical ATA validation.
start = s.index("fn validate_supported_token(")
end = s.index("fn add_direct(", start)
validators = r'''fn require_supported_mint(p: &ProtocolState, mint: Pubkey) -> Result<()> {
    require!(mint == p.usdt_mint || mint == p.usdc_mint, ProtocolError::UnsupportedToken);
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
    require!(vault.owner == vault_authority, ProtocolError::WrongVaultAuthority);
    require!(vault.key() == canonical_ata(vault_authority, mint), ProtocolError::NonCanonicalTokenAccount);
    Ok(())
}

fn validate_treasury_token_for_mint(p: &ProtocolState, mint: Pubkey, treasury: &Account<TokenAccount>) -> Result<()> {
    require_supported_mint(p, mint)?;
    require!(treasury.mint == mint, ProtocolError::MintMismatch);
    require!(treasury.owner == p.service_treasury, ProtocolError::WrongTreasury);
    require!(treasury.key() == canonical_ata(p.service_treasury, mint), ProtocolError::NonCanonicalTokenAccount);
    Ok(())
}

fn validate_user_destination(wallet: Pubkey, mint: Pubkey, destination: &Account<TokenAccount>) -> Result<()> {
    require!(destination.mint == mint, ProtocolError::MintMismatch);
    require!(destination.owner == wallet, ProtocolError::WrongTokenAuthority);
    require!(destination.key() == canonical_ata(wallet, mint), ProtocolError::NonCanonicalTokenAccount);
    Ok(())
}

fn validate_user_pda(user: &Account<UserState>) -> Result<()> {
    let expected = Pubkey::find_program_address(&[b"user", user.wallet.as_ref()], &crate::ID).0;
    require!(user.key() == expected, ProtocolError::InvalidUserPda);
    Ok(())
}

'''
s = s[:start] + validators + s[end:]

# Replace in-memory-only expiry with mint-specific accounting and physical settlement helper.
start = s.index("fn settle_expired_in_memory(")
end = s.index("fn accrue_pioneer(", start)
expiry_helpers = r'''fn mark_user_expired(user: &mut UserState, p: &mut ProtocolState, mint: Pubkey, amount: u64) -> Result<()> {
    if amount == 0 { return Ok(()); }
    if mint == p.usdt_mint {
        user.lifetime_expired_usdt = user.lifetime_expired_usdt.checked_add(amount as u128).ok_or(ProtocolError::ArithmeticOverflow)?;
        p.lifetime_expired_usdt = p.lifetime_expired_usdt.checked_add(amount as u128).ok_or(ProtocolError::ArithmeticOverflow)?;
    } else if mint == p.usdc_mint {
        user.lifetime_expired_usdc = user.lifetime_expired_usdc.checked_add(amount as u128).ok_or(ProtocolError::ArithmeticOverflow)?;
        p.lifetime_expired_usdc = p.lifetime_expired_usdc.checked_add(amount as u128).ok_or(ProtocolError::ArithmeticOverflow)?;
    } else { return err!(ProtocolError::UnsupportedToken); }
    Ok(())
}

fn expire_pending_for_mint(user: &mut UserState, now: i64, p: &mut ProtocolState, mint: Pubkey) -> Result<u64> {
    if user.grace_until == 0 || now <= user.grace_until { return Ok(0); }
    let amount = if mint == p.usdt_mint {
        let a = user.network_pending_usdt;
        user.network_pending_usdt = 0;
        a
    } else if mint == p.usdc_mint {
        let a = user.network_pending_usdc;
        user.network_pending_usdc = 0;
        a
    } else { return err!(ProtocolError::UnsupportedToken); };
    mark_user_expired(user, p, mint, amount)?;
    Ok(amount)
}

fn settle_expired_all<'info>(
    user: &mut UserState,
    now: i64,
    p: &mut ProtocolState,
    vault_authority: &UncheckedAccount<'info>,
    usdt_vault: &Account<'info, TokenAccount>,
    usdc_vault: &Account<'info, TokenAccount>,
    treasury_usdt: &Account<'info, TokenAccount>,
    treasury_usdc: &Account<'info, TokenAccount>,
    token_program: &Program<'info, Token>,
) -> Result<()> {
    if user.grace_until == 0 || now <= user.grace_until { return Ok(()); }
    let usdt_mint = p.usdt_mint;
    let usdc_mint = p.usdc_mint;
    let usdt = expire_pending_for_mint(user, now, p, usdt_mint)?;
    let usdc = expire_pending_for_mint(user, now, p, usdc_mint)?;
    if usdt > 0 { transfer_from_vault(p, vault_authority, usdt_vault, treasury_usdt, token_program, usdt)?; }
    if usdc > 0 { transfer_from_vault(p, vault_authority, usdc_vault, treasury_usdc, token_program, usdc)?; }
    Ok(())
}

'''
s = s[:start] + expiry_helpers + s[end:]

# Remove obsolete protocol-only expired helper and replace blended treasury metrics.
start = s.index("fn add_protocol_expired(")
end = s.index("fn transfer_from_vault", start)
metrics = r'''fn add_protocol_treasury_metrics(
    p: &mut ProtocolState,
    mint: Pubkey,
    service: u64,
    unallocated_network: u64,
    rounding: u64,
    pioneer_unassigned: u64,
) -> Result<()> {
    if mint == p.usdt_mint {
        p.lifetime_service_fees_usdt = p.lifetime_service_fees_usdt.checked_add(service as u128).ok_or(ProtocolError::ArithmeticOverflow)?;
        p.lifetime_unallocated_usdt = p.lifetime_unallocated_usdt.checked_add(unallocated_network as u128).ok_or(ProtocolError::ArithmeticOverflow)?;
        p.lifetime_rounding_usdt = p.lifetime_rounding_usdt.checked_add(rounding as u128).ok_or(ProtocolError::ArithmeticOverflow)?;
        p.lifetime_pioneer_unassigned_usdt = p.lifetime_pioneer_unassigned_usdt.checked_add(pioneer_unassigned as u128).ok_or(ProtocolError::ArithmeticOverflow)?;
    } else if mint == p.usdc_mint {
        p.lifetime_service_fees_usdc = p.lifetime_service_fees_usdc.checked_add(service as u128).ok_or(ProtocolError::ArithmeticOverflow)?;
        p.lifetime_unallocated_usdc = p.lifetime_unallocated_usdc.checked_add(unallocated_network as u128).ok_or(ProtocolError::ArithmeticOverflow)?;
        p.lifetime_rounding_usdc = p.lifetime_rounding_usdc.checked_add(rounding as u128).ok_or(ProtocolError::ArithmeticOverflow)?;
        p.lifetime_pioneer_unassigned_usdc = p.lifetime_pioneer_unassigned_usdc.checked_add(pioneer_unassigned as u128).ok_or(ProtocolError::ArithmeticOverflow)?;
    } else { return err!(ProtocolError::UnsupportedToken); }
    Ok(())
}

'''
s = s[:start] + metrics + s[end:]

# Add V0.9B security errors once.
needle = '    #[msg("Invalid upline account or ancestry")] InvalidUpline,\n'
addition = needle + '    #[msg("Invalid beneficiary user PDA")] InvalidBeneficiary,\n    #[msg("User state is not at its canonical PDA")] InvalidUserPda,\n    #[msg("Token account is not the canonical associated token account")] NonCanonicalTokenAccount,\n    #[msg("No expired pending reward to settle")] NothingToSettle,\n'
if "InvalidUserPda" not in s:
    if needle not in s:
        raise SystemExit("error insertion point not found")
    s = s.replace(needle, addition, 1)

path.write_text(s)
