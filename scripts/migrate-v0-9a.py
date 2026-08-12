from pathlib import Path

path = Path("programs/service_referral_protocol/src/lib.rs")
s = path.read_text()

# Protocol/User state initialization additions and renamed Pioneer remainder fields.
s = s.replace("p.pioneer_reserve_usdt = 0;", "p.pioneer_unassigned_remainder_usdt_scaled = 0;")
s = s.replace("p.pioneer_reserve_usdc = 0;", "p.pioneer_unassigned_remainder_usdc_scaled = 0;")
s = s.replace(
    "root.qualification_progress_units = 0;\n        root.lifetime_service_units = 0;",
    "root.qualification_progress_units = 0;\n        root.qualification_window_started_at = 0;\n        root.lifetime_service_units = 0;",
)
s = s.replace(
    "u.qualification_progress_units = 0;\n        u.lifetime_service_units = 0;",
    "u.qualification_progress_units = 0;\n        u.qualification_window_started_at = 0;\n        u.lifetime_service_units = 0;",
)

# Partial qualification units expire seven days after the current accumulation window starts.
old_purchase = """        user.lifetime_service_units = last;
        user.next_batch_index = user.next_batch_index.checked_add(1).ok_or(ProtocolError::ArithmeticOverflow)?;
        user.qualification_progress_units = user.qualification_progress_units.checked_add(units).ok_or(ProtocolError::ArithmeticOverflow)?;

        if user.qualification_progress_units >= ACTIVITY_THRESHOLD_UNITS {
            let was_grace = activity_status(user, now) == ActivityStatus::Grace;
            user.qualification_progress_units = 0;
            user.active_until = now.checked_add(ACTIVE_SECONDS).ok_or(ProtocolError::ArithmeticOverflow)?;
            user.grace_until = user.active_until.checked_add(GRACE_SECONDS).ok_or(ProtocolError::ArithmeticOverflow)?;
            if was_grace { vest_pending(user)?; }
        }
"""
new_purchase = """        user.lifetime_service_units = last;
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
            let was_grace = activity_status(user, now) == ActivityStatus::Grace;
            user.qualification_progress_units = 0;
            user.qualification_window_started_at = 0;
            user.active_until = now.checked_add(ACTIVE_SECONDS).ok_or(ProtocolError::ArithmeticOverflow)?;
            user.grace_until = user.active_until.checked_add(GRACE_SECONDS).ok_or(ProtocolError::ArithmeticOverflow)?;
            if was_grace { vest_pending(user)?; }
        }
"""
if old_purchase not in s:
    raise SystemExit("purchase activity block not found")
s = s.replace(old_purchase, new_purchase)

# Pioneer accrual now uses a high precision per-share index and immediately routes
# whole atomic units belonging to unassigned virtual shares to treasury.
old_accrue_call = """        accrue_pioneer(p, ctx.accounts.source_token.mint, pioneer)?;

        if treasury_now > 0 {
"""
new_accrue_call = """        let pioneer_unassigned_now = accrue_pioneer(p, ctx.accounts.source_token.mint, pioneer)?;
        treasury_now = treasury_now.checked_add(pioneer_unassigned_now).ok_or(ProtocolError::ArithmeticOverflow)?;

        if treasury_now > 0 {
"""
if old_accrue_call not in s:
    raise SystemExit("pioneer accrue call not found")
s = s.replace(old_accrue_call, new_accrue_call)

start = s.index("fn accrue_pioneer(")
end = s.index("fn take_claimable(")
replacement = r'''fn accrue_pioneer(p: &mut ProtocolState, mint: Pubkey, amount: u64) -> Result<u64> {
    let per_share_scaled = (amount as u128)
        .checked_mul(PIONEER_SCALE).ok_or(ProtocolError::ArithmeticOverflow)?
        .checked_div(PIONEER_SLOTS as u128).ok_or(ProtocolError::ArithmeticUnderflow)?;
    let unassigned = (PIONEER_SLOTS - p.pioneer_count) as u128;
    let unassigned_scaled = per_share_scaled.checked_mul(unassigned).ok_or(ProtocolError::ArithmeticOverflow)?;

    let remainder_scaled = if mint == p.usdt_mint {
        p.pioneer_index_usdt = p.pioneer_index_usdt.checked_add(per_share_scaled).ok_or(ProtocolError::ArithmeticOverflow)?;
        p.pioneer_unassigned_remainder_usdt_scaled
            .checked_add(unassigned_scaled).ok_or(ProtocolError::ArithmeticOverflow)?
    } else if mint == p.usdc_mint {
        p.pioneer_index_usdc = p.pioneer_index_usdc.checked_add(per_share_scaled).ok_or(ProtocolError::ArithmeticOverflow)?;
        p.pioneer_unassigned_remainder_usdc_scaled
            .checked_add(unassigned_scaled).ok_or(ProtocolError::ArithmeticOverflow)?
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
    if user.pioneer_id == 0 { return Ok(0); }
    let (idx, checkpoint) = if mint == p.usdt_mint {
        (p.pioneer_index_usdt, user.pioneer_checkpoint_usdt)
    } else if mint == p.usdc_mint {
        (p.pioneer_index_usdc, user.pioneer_checkpoint_usdc)
    } else { return err!(ProtocolError::UnsupportedToken); };
    let diff_scaled = idx.checked_sub(checkpoint).ok_or(ProtocolError::ArithmeticUnderflow)?;
    let due = diff_scaled / PIONEER_SCALE;
    u64::try_from(due).map_err(|_| ProtocolError::ArithmeticOverflow.into())
}

fn checkpoint_pioneer_claimed(user: &mut UserState, p: &ProtocolState, mint: Pubkey, claimed: u64) -> Result<()> {
    if user.pioneer_id == 0 || claimed == 0 { return Ok(()); }
    let advance_scaled = (claimed as u128).checked_mul(PIONEER_SCALE).ok_or(ProtocolError::ArithmeticOverflow)?;
    if mint == p.usdt_mint {
        user.pioneer_checkpoint_usdt = user.pioneer_checkpoint_usdt.checked_add(advance_scaled).ok_or(ProtocolError::ArithmeticOverflow)?;
        require!(user.pioneer_checkpoint_usdt <= p.pioneer_index_usdt, ProtocolError::ArithmeticOverflow);
    } else if mint == p.usdc_mint {
        user.pioneer_checkpoint_usdc = user.pioneer_checkpoint_usdc.checked_add(advance_scaled).ok_or(ProtocolError::ArithmeticOverflow)?;
        require!(user.pioneer_checkpoint_usdc <= p.pioneer_index_usdc, ProtocolError::ArithmeticOverflow);
    } else { return err!(ProtocolError::UnsupportedToken); }
    Ok(())
}

'''
s = s[:start] + replacement + s[end:]
s = s.replace("checkpoint_pioneer(user, p, mint)?;", "checkpoint_pioneer_claimed(user, p, mint, pioneer_due)?;")

path.write_text(s)
