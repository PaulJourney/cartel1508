#!/usr/bin/env python3
from pathlib import Path
import json
import re

ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def write(path: str, text: str) -> None:
    (ROOT / path).write_text(text)


def replace_once(path: str, old: str, new: str) -> None:
    text = read(path)
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one replacement, found {count}: {old[:100]!r}")
    write(path, text.replace(old, new, 1))


def regex_once(path: str, pattern: str, repl: str) -> None:
    text = read(path)
    new, count = re.subn(pattern, repl, text, count=1, flags=re.S)
    if count != 1:
        raise SystemExit(f"{path}: regex expected one replacement, found {count}: {pattern}")
    write(path, new)


# ---------------------------------------------------------------------------
# Frozen weekly activity + depth constants.
# ---------------------------------------------------------------------------
replace_once(
    "programs/service_referral_protocol/src/constants.rs",
    "pub const ACTIVITY_THRESHOLD_UNITS: u64 = 10;\npub const ACTIVE_SECONDS: i64 = 7 * 24 * 60 * 60;\npub const GRACE_SECONDS: i64 = 48 * 60 * 60;",
    """/// Compatibility alias for the initial weekly activity floor. The live requirement\n/// is progressive and must be derived with `active_requirement_for_week`.\npub const ACTIVITY_THRESHOLD_UNITS: u64 = 10;\npub const ACTIVE_SECONDS: i64 = 7 * 24 * 60 * 60;\npub const GRACE_SECONDS: i64 = 48 * 60 * 60;\n\n/// Two successful ACTIVE weeks per tier, capped permanently at 50 units/week:\n/// weeks 1-2=10, 3-4=20, 5-6=30, 7-8=40, 9+=50.\npub const ACTIVE_WEEK_REQUIREMENT_UNITS: [u64; 5] = [10, 20, 30, 40, 50];\npub const ACTIVE_WEEK_TIER_SPAN: u32 = 2;\n\n/// Weekly personal-unit thresholds for network monetization depth.\n/// 10=>U3, 25=>U4, 50=>U5, 100=>U6, 200=>U7, 350=>U8, 500=>U9.\npub const NETWORK_DEPTH_UNIT_THRESHOLDS: [u64; 7] = [10, 25, 50, 100, 200, 350, 500];""",
)

# ---------------------------------------------------------------------------
# User state: successful active-week count + personal units in current/last week.
# ---------------------------------------------------------------------------
replace_once(
    "programs/service_referral_protocol/src/state.rs",
    "    pub active_until: i64,\n    pub grace_until: i64,\n    pub qualification_progress_units: u64,",
    """    pub active_until: i64,\n    pub grace_until: i64,\n    /// Number of 7-day ACTIVE weeks successfully started by this wallet. Inactivity\n    /// does not advance this counter, so the progressive minimum never grows merely\n    /// because calendar time passed.\n    pub active_weeks_started: u32,\n    /// Personal units accumulated in the current ACTIVE week. During GRACE this is\n    /// intentionally retained so the just-finished week's depth remains valid while\n    /// rewards are pending. It is replaced only when the next ACTIVE week starts.\n    pub current_week_units: u64,\n    pub qualification_progress_units: u64,""",
)
replace_once(
    "programs/service_referral_protocol/src/state.rs",
    "    pub const SPACE: usize = 283;",
    "    pub const SPACE: usize = 295;",
)

# ---------------------------------------------------------------------------
# Pure math for progressive weekly activity and U1-U9 depth.
# ---------------------------------------------------------------------------
replace_once(
    "programs/service_referral_protocol/src/math.rs",
    "pub fn activity_status(user: &UserState, now: i64) -> ActivityStatus {",
    """/// Required personal units to start a given ACTIVE week. Week numbering starts\n/// at one. The requirement rises every two successfully-started ACTIVE weeks and\n/// caps permanently at 50 units from week 9 onward.\npub fn active_requirement_for_week(week_number: u32) -> u64 {\n    let normalized = week_number.max(1);\n    let tier = ((normalized - 1) / ACTIVE_WEEK_TIER_SPAN) as usize;\n    ACTIVE_WEEK_REQUIREMENT_UNITS[tier.min(ACTIVE_WEEK_REQUIREMENT_UNITS.len() - 1)]\n}\n\npub fn next_active_requirement(active_weeks_started: u32) -> u64 {\n    active_requirement_for_week(active_weeks_started.saturating_add(1))\n}\n\n/// Maximum network level monetizable from personal units in the current/last\n/// ACTIVE week. Values below 10 unlock no network depth.\npub fn network_depth_for_units(units: u64) -> u8 {\n    let mut depth = 0u8;\n    for (index, threshold) in NETWORK_DEPTH_UNIT_THRESHOLDS.iter().enumerate() {\n        if units < *threshold {\n            break;\n        }\n        depth = (index as u8) + 3;\n    }\n    depth\n}\n\npub fn activity_status(user: &UserState, now: i64) -> ActivityStatus {""",
)
replace_once(
    "programs/service_referral_protocol/src/math.rs",
    "    #[test]\n    fn huge_batch_payment_fits_u64() {",
    """    #[test]\n    fn progressive_active_requirement_caps_at_fifty() {\n        let expected = [\n            (1, 10), (2, 10), (3, 20), (4, 20), (5, 30), (6, 30),\n            (7, 40), (8, 40), (9, 50), (10, 50), (100, 50),\n        ];\n        for (week, units) in expected {\n            assert_eq!(active_requirement_for_week(week), units);\n        }\n        assert_eq!(next_active_requirement(0), 10);\n        assert_eq!(next_active_requirement(1), 10);\n        assert_eq!(next_active_requirement(2), 20);\n        assert_eq!(next_active_requirement(8), 50);\n        assert_eq!(next_active_requirement(u32::MAX), 50);\n    }\n\n    #[test]\n    fn weekly_units_unlock_exact_network_depth_boundaries() {\n        let expected = [\n            (0, 0), (9, 0), (10, 3), (24, 3), (25, 4), (49, 4),\n            (50, 5), (99, 5), (100, 6), (199, 6), (200, 7),\n            (349, 7), (350, 8), (499, 8), (500, 9), (10_000, 9),\n        ];\n        for (units, depth) in expected {\n            assert_eq!(network_depth_for_units(units), depth);\n        }\n    }\n\n    #[test]\n    fn huge_batch_payment_fits_u64() {""",
)

# ---------------------------------------------------------------------------
# Core state initialization and registration events.
# ---------------------------------------------------------------------------
for prefix in ("root", "u"):
    replace_once(
        "programs/service_referral_protocol/src/lib.rs",
        f"        {prefix}.active_until = 0;\n        {prefix}.grace_until = 0;\n        {prefix}.qualification_progress_units = 0;",
        f"        {prefix}.active_until = 0;\n        {prefix}.grace_until = 0;\n        {prefix}.active_weeks_started = 0;\n        {prefix}.current_week_units = 0;\n        {prefix}.qualification_progress_units = 0;",
    )

replace_once(
    "programs/service_referral_protocol/src/lib.rs",
    "        u.pioneer_checkpoint_usdt = 0;\n        u.pioneer_checkpoint_usdc = 0;\n        Ok(())\n    }\n\n    /// Final production economic path.",
    """        u.pioneer_checkpoint_usdt = 0;\n        u.pioneer_checkpoint_usdc = 0;\n\n        emit!(UserRegistered {\n            wallet: u.wallet,\n            referrer: u.referrer,\n            registered_at: now,\n        });\n        Ok(())\n    }\n\n    /// Final production economic path.""",
)

# Replace the activity updater with cycle-aware progressive qualification.
regex_once(
    "programs/service_referral_protocol/src/lib.rs",
    r"fn update_activity_after_purchase\(.*?\n\}\n\nfn add_self_reward",
    """fn update_activity_after_purchase(\n    user: &mut UserState,\n    units: u64,\n    now: i64,\n    pre_status: ActivityStatus,\n) -> Result<()> {\n    user.lifetime_service_units = user\n        .lifetime_service_units\n        .checked_add(units as u128)\n        .ok_or(ProtocolError::ArithmeticOverflow)?;\n    user.next_purchase_index = user\n        .next_purchase_index\n        .checked_add(1)\n        .ok_or(ProtocolError::ArithmeticOverflow)?;\n\n    // Purchases made during an already ACTIVE 7-day cycle increase only that\n    // cycle's monetization depth. They never pre-qualify the following week.\n    if pre_status == ActivityStatus::Active {\n        user.current_week_units = user\n            .current_week_units\n            .checked_add(units)\n            .ok_or(ProtocolError::ArithmeticOverflow)?;\n        return Ok(());\n    }\n\n    // GRACE/INACTIVE purchases accumulate toward the next successfully-started\n    // ACTIVE week. A partial qualification has at most one 7-day accumulation\n    // window; stale progress is discarded, while inactive calendar time alone\n    // never advances active_weeks_started.\n    if user.qualification_progress_units > 0\n        && now\n            > user\n                .qualification_window_started_at\n                .checked_add(ACTIVE_SECONDS)\n                .ok_or(ProtocolError::ArithmeticOverflow)?\n    {\n        user.qualification_progress_units = 0;\n        user.qualification_window_started_at = 0;\n    }\n    if user.qualification_progress_units == 0 {\n        user.qualification_window_started_at = now;\n    }\n    user.qualification_progress_units = user\n        .qualification_progress_units\n        .checked_add(units)\n        .ok_or(ProtocolError::ArithmeticOverflow)?;\n\n    let required = next_active_requirement(user.active_weeks_started);\n    if user.qualification_progress_units >= required {\n        let activating_units = user.qualification_progress_units;\n        user.qualification_progress_units = 0;\n        user.qualification_window_started_at = 0;\n        user.active_weeks_started = user\n            .active_weeks_started\n            .checked_add(1)\n            .ok_or(ProtocolError::ArithmeticOverflow)?;\n        user.current_week_units = activating_units;\n        user.active_until = now\n            .checked_add(ACTIVE_SECONDS)\n            .ok_or(ProtocolError::ArithmeticOverflow)?;\n        user.grace_until = user\n            .active_until\n            .checked_add(GRACE_SECONDS)\n            .ok_or(ProtocolError::ArithmeticOverflow)?;\n        if pre_status == ActivityStatus::Grace {\n            vest_pending(user)?;\n        }\n    }\n    Ok(())\n}\n\nfn add_self_reward""",
)

# Network depth enforcement helper. Active/Grace but out-of-depth is never earned;
# the scheduled share goes straight to the unallocated Treasury bucket (IC-A, no compression).
replace_once(
    "programs/service_referral_protocol/src/lib.rs",
    "fn vest_pending(user: &mut UserState) -> Result<()> {",
    """fn distribute_network_level(\n    user: &mut UserState,\n    p: &mut ProtocolState,\n    mint: Pubkey,\n    amount: u64,\n    level_number: u8,\n    now: i64,\n    unallocated: &mut u64,\n    expired_flow: &mut u64,\n) -> Result<()> {\n    let status = activity_status(user, now);\n    if matches!(status, ActivityStatus::Active | ActivityStatus::Grace)\n        && network_depth_for_units(user.current_week_units) < level_number\n    {\n        *unallocated = unallocated\n            .checked_add(amount)\n            .ok_or(ProtocolError::ArithmeticOverflow)?;\n        return Ok(());\n    }\n\n    match status {\n        ActivityStatus::Active => add_network_claimable(user, p, mint, amount)?,\n        ActivityStatus::Grace => add_network_pending(user, p, mint, amount)?,\n        ActivityStatus::Inactive => {\n            mark_user_expired(user, p, mint, amount)?;\n            *expired_flow = expired_flow\n                .checked_add(amount)\n                .ok_or(ProtocolError::ArithmeticOverflow)?;\n        }\n    }\n    Ok(())\n}\n\nfn vest_pending(user: &mut UserState) -> Result<()> {""",
)

replace_once(
    "programs/service_referral_protocol/src/lib.rs",
    """            match activity_status(&ctx.accounts.direct_referrer, now) {\n                ActivityStatus::Active => {\n                    add_network_claimable(&mut ctx.accounts.direct_referrer, p, mint, levels[0])?\n                }\n                ActivityStatus::Grace => {\n                    add_network_pending(&mut ctx.accounts.direct_referrer, p, mint, levels[0])?\n                }\n                ActivityStatus::Inactive => {\n                    mark_user_expired(&mut ctx.accounts.direct_referrer, p, mint, levels[0])?;\n                    expired_flow = expired_flow\n                        .checked_add(levels[0])\n                        .ok_or(ProtocolError::ArithmeticOverflow)?;\n                }\n            }""",
    """            distribute_network_level(\n                &mut ctx.accounts.direct_referrer,\n                p,\n                mint,\n                levels[0],\n                1,\n                now,\n                &mut unallocated,\n                &mut expired_flow,\n            )?;""",
)

replace_once(
    "programs/service_referral_protocol/src/lib.rs",
    """                match activity_status(&upline, now) {\n                    ActivityStatus::Active => {\n                        add_network_claimable(&mut upline, p, mint, levels[i])?\n                    }\n                    ActivityStatus::Grace => {\n                        add_network_pending(&mut upline, p, mint, levels[i])?\n                    }\n                    ActivityStatus::Inactive => {\n                        mark_user_expired(&mut upline, p, mint, levels[i])?;\n                        expired_flow = expired_flow\n                            .checked_add(levels[i])\n                            .ok_or(ProtocolError::ArithmeticOverflow)?;\n                    }\n                }""",
    """                distribute_network_level(\n                    &mut upline,\n                    p,\n                    mint,\n                    levels[i],\n                    (i + 1) as u8,\n                    now,\n                    &mut unallocated,\n                    &mut expired_flow,\n                )?;""",
)

# Pioneer is strictly ACTIVE-only after GRACE. Only SELF retains provisional batching
# protection in an inactive partial qualification window.
replace_once(
    "programs/service_referral_protocol/src/lib.rs",
    """/// True while an INACTIVE user is still inside a live partial qualification window.\n/// SELF and Pioneer value created during this window is provisional so splitting the\n/// same qualifying purchase into multiple transactions cannot change its economics.""",
    """/// True while an INACTIVE user is still inside a live partial qualification window.\n/// Only SELF value created during this window is provisional so splitting a personal\n/// qualification into multiple transactions does not destroy the buyer's own SELF.\n/// Pioneer remains strictly ACTIVE/GRACE-gated and receives no inactive exception.""",
)
replace_once(
    "programs/service_referral_protocol/src/lib.rs",
    """/// A live partial qualification window is a narrow exception for the user's own\n/// SELF and Pioneer buckets: those remain provisional until the user either reaches\n/// the 10-unit threshold or the qualification window expires. Network amounts are\n/// never protected by this exception and continue to follow IC-A fixed-depth expiry.\n/// This makes 10 units bought as 10x1 economically equivalent to 10 units bought\n/// in one transaction for the buyer's own SELF/Pioneer entitlement.""",
    """/// A live partial qualification window is a narrow exception for the user's own\n/// SELF bucket only. Pioneer remains ACTIVE-only (GRACE preserves already-earned\n/// value); once INACTIVE, Pioneer due is treasury-destined even while the wallet is\n/// accumulating units toward reactivation. Network amounts likewise receive no\n/// partial-window protection and continue to follow IC-A fixed-depth expiry.""",
)
replace_once(
    "programs/service_referral_protocol/src/lib.rs",
    """    let preserve_self_and_pioneer = qualification_window_open(user, now);\n    let pioneer = if preserve_self_and_pioneer {\n        0\n    } else {\n        pioneer_due(user, p, mint)?\n    };""",
    """    let preserve_self = qualification_window_open(user, now);\n    let pioneer = pioneer_due(user, p, mint)?;""",
)
replace_once(
    "programs/service_referral_protocol/src/lib.rs",
    "if preserve_self_and_pioneer {",
    "if preserve_self {",
)
replace_once(
    "programs/service_referral_protocol/src/lib.rs",
    "if preserve_self_and_pioneer {",
    "if preserve_self {",
)

# Event surface for a deterministic off-chain rank/indexer layer and UI depth state.
replace_once(
    "programs/service_referral_protocol/src/lib.rs",
    """        let pioneer_positions_total = p.pioneer_positions_assigned;\n\n        emit!(UnitsPurchased {""",
    """        let pioneer_positions_total = p.pioneer_positions_assigned;\n        let buyer_network_depth = network_depth_for_units(ctx.accounts.user.current_week_units);\n        let next_active_requirement_units =\n            next_active_requirement(ctx.accounts.user.active_weeks_started);\n\n        emit!(UnitsPurchased {""",
)
replace_once(
    "programs/service_referral_protocol/src/lib.rs",
    """            pioneer_positions_added,\n            pioneer_positions_total,\n            purchased_at: now,""",
    """            pioneer_positions_added,\n            pioneer_positions_total,\n            active_weeks_started: ctx.accounts.user.active_weeks_started,\n            current_week_units: ctx.accounts.user.current_week_units,\n            qualification_progress_units: ctx.accounts.user.qualification_progress_units,\n            network_depth: buyer_network_depth,\n            next_active_requirement_units,\n            active_until: ctx.accounts.user.active_until,\n            grace_until: ctx.accounts.user.grace_until,\n            purchased_at: now,""",
)
replace_once(
    "programs/service_referral_protocol/src/lib.rs",
    """#[event]\npub struct UnitsPurchased {""",
    """#[event]\npub struct UserRegistered {\n    pub wallet: Pubkey,\n    pub referrer: Pubkey,\n    pub registered_at: i64,\n}\n\n#[event]\npub struct UnitsPurchased {""",
)
replace_once(
    "programs/service_referral_protocol/src/lib.rs",
    """    pub pioneer_positions_added: u16,\n    pub pioneer_positions_total: u16,\n    pub purchased_at: i64,""",
    """    pub pioneer_positions_added: u16,\n    pub pioneer_positions_total: u16,\n    pub active_weeks_started: u32,\n    pub current_week_units: u64,\n    pub qualification_progress_units: u64,\n    pub network_depth: u8,\n    pub next_active_requirement_units: u64,\n    pub active_until: i64,\n    pub grace_until: i64,\n    pub purchased_at: i64,""",
)

# Update top-level economic comments to describe dynamic depth rather than unconditional payout.
replace_once(
    "programs/service_referral_protocol/src/lib.rs",
    "/// - credits the immutable sponsor plus eight ancestors with the frozen 43% schedule;",
    "/// - schedules the frozen 43% across sponsor + eight ancestors, paying only levels unlocked by each upline's weekly personal units;",
)

# ---------------------------------------------------------------------------
# Deep LiteSVM test: prove 10 units pays only U1-U3, then 500 unlocks U1-U9
# prospectively in the same active cycle.
# ---------------------------------------------------------------------------
replace_once(
    "integration-tests/tests/deep_network.rs",
    "fn full_genealogy_pays_self_and_nine_uplines_but_never_tenth_upline() {",
    "fn weekly_depth_is_prospective_and_never_exceeds_nine_uplines() {",
)
replace_once(
    "integration-tests/tests/deep_network.rs",
    "// Activate exactly the nine payable uplines U9 through U1. U10/U11 stay inactive.",
    "// Activate the nine payable uplines with 10 personal units each. They are ACTIVE,\n    // but 10 weekly units unlock only U1-U3. U10/U11 stay inactive.",
)

old_assertions = """    // U1 sponsor gets 15%, followed by U2..U9 across indices 9 down to 2.\n    assert_eq!(\n        after[10].network_claimable_usdc - before[10].network_claimable_usdc,\n        15 * UNIT\n    );\n    let expected = [\n        9 * UNIT,\n        6 * UNIT,\n        4 * UNIT,\n        2_500_000,\n        2 * UNIT,\n        1_500_000,\n        1 * UNIT,\n        2 * UNIT,\n    ];\n    for (offset, amount) in expected.iter().enumerate() {\n        let index = 9 - offset;\n        assert_eq!(\n            after[index].network_claimable_usdc - before[index].network_claimable_usdc,\n            *amount,\n            \"unexpected network delta at upline {}\",\n            offset + 2\n        );\n    }\n\n    // U10 and U11 exist in the real ancestry but are outside the nine-upline cap.\n    for index in [0usize, 1usize] {\n        assert_eq!(after[index].network_claimable_usdc, before[index].network_claimable_usdc);\n        assert_eq!(after[index].network_pending_usdc, before[index].network_pending_usdc);\n        assert_eq!(after[index].lifetime_expired_usdc, before[index].lifetime_expired_usdc);\n    }"""

new_assertions = """    // With only 10 personal weekly units, each active upline is qualified only\n    // through U3. U1/U2/U3 receive 15/9/6%; U4-U9 are not earned and route to\n    // Treasury as unallocated value without compression.\n    assert_eq!(\n        after[10].network_claimable_usdc - before[10].network_claimable_usdc,\n        15 * UNIT\n    );\n    assert_eq!(\n        after[9].network_claimable_usdc - before[9].network_claimable_usdc,\n        9 * UNIT\n    );\n    assert_eq!(\n        after[8].network_claimable_usdc - before[8].network_claimable_usdc,\n        6 * UNIT\n    );\n    for index in 2usize..=7usize {\n        assert_eq!(\n            after[index].network_claimable_usdc,\n            before[index].network_claimable_usdc,\n            \"U4-U9 must be locked at 10 weekly units\"\n        );\n    }\n\n    // U10 and U11 exist in the real ancestry but are outside the nine-upline cap.\n    for index in [0usize, 1usize] {\n        assert_eq!(after[index].network_claimable_usdc, before[index].network_claimable_usdc);\n        assert_eq!(after[index].network_pending_usdc, before[index].network_pending_usdc);\n        assert_eq!(after[index].lifetime_expired_usdc, before[index].lifetime_expired_usdc);\n    }\n\n    // Add 490 units to every payable upline during the same ACTIVE cycle. Their\n    // weekly personal total becomes 500, unlocking U1-U9 prospectively. No prior\n    // locked commission is recovered.\n    for i in 2usize..=10usize {\n        let source = spl_associated_token_account::get_associated_token_address_with_program_id(\n            &wallets[i].pubkey(),\n            &usdc_mint.pubkey(),\n            &spl_token::id(),\n        );\n        ctx.svm\n            .mint_to(&usdc_mint.pubkey(), &source, &initializer, 490 * UNIT)\n            .expect(\"fund depth upgrade\");\n\n        let mut uplines = [technical_root; 8];\n        let mut ancestor = i as isize - 2;\n        for slot in 0..8 {\n            if ancestor >= 0 {\n                uplines[slot] = user_pdas[ancestor as usize];\n                ancestor -= 1;\n            }\n        }\n        ctx.svm.expire_blockhash();\n        let upgrade_ix = ctx\n            .program()\n            .accounts(service_referral_protocol::accounts::PurchaseAndDistribute {\n                wallet: wallets[i].pubkey(),\n                protocol,\n                user: user_pdas[i],\n                user_source: source,\n                vault_authority,\n                usdt_vault: vault_usdt,\n                usdc_vault: vault_usdc,\n                service_treasury_usdt: treasury_usdt,\n                service_treasury_usdc: treasury_usdc,\n                direct_referrer: user_pdas[i - 1],\n                upline_1: uplines[0],\n                upline_2: uplines[1],\n                upline_3: uplines[2],\n                upline_4: uplines[3],\n                upline_5: uplines[4],\n                upline_6: uplines[5],\n                upline_7: uplines[6],\n                upline_8: uplines[7],\n                token_program: spl_token::id(),\n            })\n            .args(service_referral_protocol::instruction::PurchaseAndDistribute { units: 490 })\n            .instruction()\n            .expect(\"depth upgrade ix\");\n        ctx.execute_instruction(upgrade_ix, &[&wallets[i]])\n            .expect(\"depth upgrade tx\")\n            .assert_success();\n        assert_eq!(read_user(&ctx, user_pdas[i]).current_week_units, 500);\n    }\n\n    let before_full: Vec<UserState> = user_pdas\n        .iter()\n        .map(|pda| read_user(&ctx, *pda))\n        .collect();\n    ctx.svm\n        .mint_to(&usdc_mint.pubkey(), &buyer_source, &initializer, 100 * UNIT)\n        .expect(\"fund second buyer purchase\");\n    ctx.svm.expire_blockhash();\n    let full_depth_ix = ctx\n        .program()\n        .accounts(service_referral_protocol::accounts::PurchaseAndDistribute {\n            wallet: wallets[buyer_index].pubkey(),\n            protocol,\n            user: user_pdas[buyer_index],\n            user_source: buyer_source,\n            vault_authority,\n            usdt_vault: vault_usdt,\n            usdc_vault: vault_usdc,\n            service_treasury_usdt: treasury_usdt,\n            service_treasury_usdc: treasury_usdc,\n            direct_referrer: user_pdas[10],\n            upline_1: user_pdas[9],\n            upline_2: user_pdas[8],\n            upline_3: user_pdas[7],\n            upline_4: user_pdas[6],\n            upline_5: user_pdas[5],\n            upline_6: user_pdas[4],\n            upline_7: user_pdas[3],\n            upline_8: user_pdas[2],\n            token_program: spl_token::id(),\n        })\n        .args(service_referral_protocol::instruction::PurchaseAndDistribute { units: 100 })\n        .instruction()\n        .expect(\"full-depth target ix\");\n    ctx.execute_instruction(full_depth_ix, &[&wallets[buyer_index]])\n        .expect(\"full-depth target tx\")\n        .assert_success();\n\n    let after_full: Vec<UserState> = user_pdas\n        .iter()\n        .map(|pda| read_user(&ctx, *pda))\n        .collect();\n    let expected_full = [\n        15 * UNIT, 9 * UNIT, 6 * UNIT, 4 * UNIT, 2_500_000,\n        2 * UNIT, 1_500_000, 1 * UNIT, 2 * UNIT,\n    ];\n    for (level_offset, amount) in expected_full.iter().enumerate() {\n        let index = 10 - level_offset;\n        assert_eq!(\n            after_full[index].network_claimable_usdc - before_full[index].network_claimable_usdc,\n            *amount,\n            \"unexpected full-depth delta at U{}\",\n            level_offset + 1\n        );\n    }\n    for index in [0usize, 1usize] {\n        assert_eq!(after_full[index].network_claimable_usdc, before_full[index].network_claimable_usdc);\n        assert_eq!(after_full[index].network_pending_usdc, before_full[index].network_pending_usdc);\n    }"""
replace_once("integration-tests/tests/deep_network.rs", old_assertions, new_assertions)

# ---------------------------------------------------------------------------
# Static gates: bind the new incentives to CI.
# ---------------------------------------------------------------------------
replace_once(
    "scripts/static-gates.py",
    "qualification_window_helper = section('fn qualification_window_open', 'fn expire_unclaimed_for_mint')\nexpiry_helper = section('fn expire_unclaimed_for_mint', '#[allow(clippy::too_many_arguments)]')",
    "qualification_window_helper = section('fn qualification_window_open', 'fn expire_unclaimed_for_mint')\nnetwork_distribution_helper = section('fn distribute_network_level', 'fn vest_pending')\nexpiry_helper = section('fn expire_unclaimed_for_mint', '#[allow(clippy::too_many_arguments)]')",
)
replace_once(
    "scripts/static-gates.py",
    "'final purchase is the sole reward event': all(x in final_purchase for x in ['split_purchase_amount(payment)', 'add_self_reward(', 'add_network_claimable(', 'accrue_pioneer(']),",
    "'final purchase is the sole reward event': all(x in final_purchase for x in ['split_purchase_amount(payment)', 'add_self_reward(', 'distribute_network_level(', 'accrue_pioneer(']) and 'add_network_claimable(' in network_distribution_helper,",
)
replace_once(
    "scripts/static-gates.py",
    "'network starts at immutable sponsor': 'direct_referrer.wallet == ctx.accounts.user.referrer' in final_purchase and 'levels[0]' in final_purchase and 'add_network_claimable(&mut ctx.accounts.direct_referrer' in final_purchase,",
    "'network starts at immutable sponsor': 'direct_referrer.wallet == ctx.accounts.user.referrer' in final_purchase and 'levels[0]' in final_purchase and 'distribute_network_level(' in final_purchase,",
)
replace_once(
    "scripts/static-gates.py",
    "'partial qualification preserves SELF and Pioneer only': 'qualification_window_open' in source and 'preserve_self_and_pioneer' in expiry_helper and 'network_claimable_usdt = 0' in expiry_helper and 'network_pending_usdt = 0' in expiry_helper,",
    "'inactive partial qualification preserves SELF but not Pioneer': 'qualification_window_open' in source and 'let preserve_self = qualification_window_open' in expiry_helper and 'let pioneer = pioneer_due(user, p, mint)?' in expiry_helper and 'preserve_self_and_pioneer' not in expiry_helper and 'network_claimable_usdt = 0' in expiry_helper and 'network_pending_usdt = 0' in expiry_helper,",
)
replace_once(
    "scripts/static-gates.py",
    "'Pioneer positions require one 1000-unit purchase':",
    "'progressive ACTIVE schedule is frozen': 'ACTIVE_WEEK_REQUIREMENT_UNITS: [u64; 5] = [10, 20, 30, 40, 50]' in constants and 'ACTIVE_WEEK_TIER_SPAN: u32 = 2' in constants and 'active_weeks_started' in state and 'current_week_units' in state and 'next_active_requirement(user.active_weeks_started)' in activity_helper,\n    'weekly depth thresholds are frozen': 'NETWORK_DEPTH_UNIT_THRESHOLDS: [u64; 7] = [10, 25, 50, 100, 200, 350, 500]' in constants and 'network_depth_for_units(user.current_week_units)' in network_distribution_helper,\n    'out-of-depth network share routes to Treasury without compression': 'level_number' in network_distribution_helper and 'unallocated' in network_distribution_helper and 'checked_add(amount)' in network_distribution_helper,\n    'active purchases increase depth without prequalifying next week': 'pre_status == ActivityStatus::Active' in activity_helper and 'current_week_units' in activity_helper and 'return Ok(())' in activity_helper,\n    'registration and purchase events support deterministic rank indexing': 'pub struct UserRegistered' in source and 'pub referrer: Pubkey' in source and all(x in source for x in ['active_weeks_started: u32', 'current_week_units: u64', 'network_depth: u8', 'next_active_requirement_units: u64']),\n    'Pioneer positions require one 1000-unit purchase':",
)

# ---------------------------------------------------------------------------
# Pre-mainnet manifest/source gate binds progressive activity/depth too.
# ---------------------------------------------------------------------------
replace_once(
    "scripts/pre-mainnet-gate.py",
    '    "economics_profile": "SELF50_NETWORK43_PIONEER2_SERVICE5",\n    "pioneer_position_cap": 100,',
    '    "economics_profile": "SELF50_NETWORK43_PIONEER2_SERVICE5",\n    "activity_profile": "ACTIVE_WEEKS_10_10_20_20_30_30_40_40_50_CAP",\n    "depth_profile": "WEEKLY_DEPTH_10U3_25U4_50U5_100U6_200U7_350U8_500U9",\n    "pioneer_position_cap": 100,',
)
replace_once(
    "scripts/pre-mainnet-gate.py",
    """    if 'for i in 1..9' not in core_lib or 'pub upline_8:' not in core_lib or 'pub upline_9:' in core_lib:\n        blockers.append(\"production referral traversal is not frozen to sponsor plus eight ancestors\")\n\n    # Pioneer release invariants are mainnet gates, not documentation-only rules.""",
    """    if 'for i in 1..9' not in core_lib or 'pub upline_8:' not in core_lib or 'pub upline_9:' in core_lib:\n        blockers.append(\"production referral traversal is not frozen to sponsor plus eight ancestors\")\n    if 'ACTIVE_WEEK_REQUIREMENT_UNITS: [u64; 5] = [10, 20, 30, 40, 50]' not in constants or 'ACTIVE_WEEK_TIER_SPAN: u32 = 2' not in constants:\n        blockers.append(\"progressive ACTIVE weekly schedule is not frozen\")\n    if 'NETWORK_DEPTH_UNIT_THRESHOLDS: [u64; 7] = [10, 25, 50, 100, 200, 350, 500]' not in constants:\n        blockers.append(\"weekly U1-U9 depth schedule is not frozen\")\n    if 'pub active_weeks_started: u32' not in state or 'pub current_week_units: u64' not in state:\n        blockers.append(\"weekly activity/depth state is missing\")\n    if 'distribute_network_level(' not in core_lib or 'network_depth_for_units(user.current_week_units)' not in core_lib:\n        blockers.append(\"network payouts are not gated by weekly personal-unit depth\")\n    if 'let pioneer = pioneer_due(user, p, mint)?' not in core_lib or 'preserve_self_and_pioneer' in core_lib:\n        blockers.append(\"Pioneer is not strictly ACTIVE/GRACE-gated after inactivity\")\n\n    # Pioneer release invariants are mainnet gates, not documentation-only rules.""",
)
replace_once(
    "scripts/pre-mainnet-gate.py",
    '        "usdt_mint", "usdc_mint", "economics_profile", "pioneer_position_cap",',
    '        "usdt_mint", "usdc_mint", "economics_profile", "activity_profile", "depth_profile", "pioneer_position_cap",',
)

manifest_path = ROOT / "release/mainnet-release.example.json"
manifest = json.loads(manifest_path.read_text())
manifest["activity_profile"] = "ACTIVE_WEEKS_10_10_20_20_30_30_40_40_50_CAP"
manifest["depth_profile"] = "WEEKLY_DEPTH_10U3_25U4_50U5_100U6_200U7_350U8_500U9"
manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")

# ---------------------------------------------------------------------------
# Rank/badge specification stays outside the payout core by design.
# ---------------------------------------------------------------------------
rank_spec = """# Rank & Badge Specification — V1\n\nThis specification is application/indexer-layer only. Rank never changes the frozen\n50/43/2/5 payout percentages in V1. All inputs are deterministically reconstructable\nfrom `UserRegistered` and `UnitsPurchased` on-chain events.\n\n## Personal qualification versus rank\n\n- Personal purchases control ACTIVE status and weekly network depth only.\n- Personal purchases are excluded from the user's own QNV rank calculation.\n- Claimed earnings are excluded from rank.\n- QNV is gross purchase units generated by all descendants, grouped by immutable\n  direct-referral leg, over a rolling 30-day window.\n- A Qualified Leg must contribute at least 100 QNV units in the rolling 30-day window.\n- Balance caps are applied to the QNV required for the rank, not to lifetime volume.\n\n| Current Rank | QNV / rolling 30d | Qualified Legs | Maximum contribution from one leg |\n|---|---:|---:|---:|\n| Bronze | 1,000 | 2 | none |\n| Silver | 5,000 | 2 | none |\n| Gold | 15,000 | 3 | 60% |\n| Platinum | 50,000 | 4 | 50% |\n| Emerald | 150,000 | 5 | 50% |\n| Diamond | 500,000 | 6 | 50% |\n| Double Diamond | 1,500,000 | 6 | 45% |\n| Crown | 5,000,000 | 8 | 40% |\n| Crown Ambassador | 15,000,000 | 10 | 40% |\n\nThe UI exposes both `Current Rank` (rolling/dynamic) and `Highest Lifetime Rank`\n(permanent historical achievement).\n\n## Pioneer badge\n\nPioneer is independent of rank and is displayed as `PIONEER ×N`, where N is the\nnumber of permanently-owned Pioneer positions. The global cap remains 100 positions,\nnot 100 wallets. Pioneer positions never recycle, but their economic rewards remain\nACTIVE-only under the core activity/GRACE/expiry rules.\n"""
write("RANK_BADGE_SPEC.md", rank_spec)

# ---------------------------------------------------------------------------
# Append concise frozen-model notes to docs without rewriting prior audit history.
# ---------------------------------------------------------------------------
append_blocks = {
    "README.md": """\n\n## Progressive weekly activity and monetization depth\n\nThe core now separates being ACTIVE from how deep an upline can monetize. Successful\nACTIVE weeks require 10/10/20/20/30/30/40/40/50 units, capped at 50 from week 9. Only\nsuccessfully-started ACTIVE weeks advance the requirement. During an ACTIVE week,\npersonal purchases accumulate toward depth: 10=>U3, 25=>U4, 50=>U5, 100=>U6,\n200=>U7, 350=>U8, 500=>U9. Unlocking is prospective only; previously unqualified\nlevels are never recovered or compressed and route to Treasury. Pioneer remains a\nseparate 1,000-units-per-single-purchase rule and is strictly ACTIVE/GRACE-gated.\nRanks/badges are deterministic indexer-layer metadata; see `RANK_BADGE_SPEC.md`.\n""",
    "AUDIT_SCOPE.md": """\n\n## Weekly activity/depth invariants added to audit scope\n\n- ACTIVE-week requirements: weeks 1-2=10, 3-4=20, 5-6=30, 7-8=40, week 9+=50.\n- Calendar inactivity must never increment the ACTIVE-week counter.\n- While ACTIVE, purchases increase current-week personal units/depth and never prequalify the next week.\n- Depth thresholds are exactly 10=>U3, 25=>U4, 50=>U5, 100=>U6, 200=>U7, 350=>U8, 500=>U9.\n- Out-of-depth scheduled network value is Treasury/unallocated, never compressed and never retroactively recoverable.\n- GRACE retains the previous ACTIVE week's depth while network value is pending.\n- Pioneer positions are permanent but Pioneer economic due is ACTIVE/GRACE-gated; INACTIVE partial qualification preserves SELF only, not Pioneer.\n- Rank/badge logic is explicitly outside the payout core and must not modify 50/43/2/5 economics.\n""",
    "AUDIT_HANDOFF.md": """\n\n## New frozen behavior for review\n\nReview progressive ACTIVE-week qualification, prospective weekly U1-U9 depth, Treasury routing of locked levels, and strict Pioneer ACTIVE/GRACE gating. `RANK_BADGE_SPEC.md` is an indexer/UI specification only and must not be treated as an additional on-chain payout surface.\n""",
}
for filename, block in append_blocks.items():
    text = read(filename)
    marker = block.strip().splitlines()[0]
    if marker not in text:
        write(filename, text.rstrip() + block + "\n")

# Clean up the one-shot migration machinery in the migration commit itself.
(ROOT / "scripts/apply-weekly-depth-rank-migration.py").unlink()
(ROOT / ".github/workflows/apply-weekly-depth-rank-migration.yml").unlink()
