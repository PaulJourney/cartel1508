use anchor_lang::prelude::*;
use service_referral_protocol::{math::activity_status, state::{ActivityStatus, UserState}};

fn user_with_deadlines(active_until: i64, grace_until: i64) -> UserState {
    UserState {
        bump: 0,
        wallet: Pubkey::new_unique(),
        referrer: Pubkey::default(),
        registered_at: 0,
        pioneer_positions: 0,
        active_until,
        grace_until,
        active_weeks_started: 1,
        current_week_units: 10,
        qualification_progress_units: 0,
        qualification_window_started_at: 0,
        lifetime_service_units: 10,
        next_purchase_index: 1,
        self_accrued_usdt: 0,
        self_accrued_usdc: 0,
        network_claimable_usdt: 0,
        network_claimable_usdc: 0,
        network_pending_usdt: 0,
        network_pending_usdc: 0,
        lifetime_claimed_usdt: 0,
        lifetime_claimed_usdc: 0,
        lifetime_expired_usdt: 0,
        lifetime_expired_usdc: 0,
        pioneer_checkpoint_usdt: 0,
        pioneer_checkpoint_usdc: 0,
    }
}

#[test]
fn active_grace_inactive_boundaries_are_exact_and_inclusive() {
    let user = user_with_deadlines(1_000, 1_200);

    assert_eq!(activity_status(&user, 999), ActivityStatus::Active);
    assert_eq!(activity_status(&user, 1_000), ActivityStatus::Active, "active_until itself must still be ACTIVE");
    assert_eq!(activity_status(&user, 1_001), ActivityStatus::Grace, "first second after active_until must be GRACE");
    assert_eq!(activity_status(&user, 1_200), ActivityStatus::Grace, "grace_until itself must still be GRACE");
    assert_eq!(activity_status(&user, 1_201), ActivityStatus::Inactive, "first second after grace_until must be INACTIVE");
}

#[test]
fn zero_deadlines_are_never_accidentally_active_or_grace() {
    let user = user_with_deadlines(0, 0);
    assert_eq!(activity_status(&user, -1), ActivityStatus::Inactive);
    assert_eq!(activity_status(&user, 0), ActivityStatus::Inactive);
    assert_eq!(activity_status(&user, 1), ActivityStatus::Inactive);
}
