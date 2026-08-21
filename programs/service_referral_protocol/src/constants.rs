use anchor_lang::prelude::*;

pub const TOKEN_DECIMALS: u32 = 6;
pub const TOKEN_SCALE: u64 = 1_000_000;

/// Compatibility alias for the initial weekly activity floor. The live requirement
/// is progressive and must be derived with `active_requirement_for_week`.
pub const ACTIVITY_THRESHOLD_UNITS: u64 = 10;
pub const ACTIVE_SECONDS: i64 = 7 * 24 * 60 * 60;
pub const GRACE_SECONDS: i64 = 48 * 60 * 60;

/// Two successful ACTIVE weeks per tier, capped permanently at 50 units/week:
/// weeks 1-2=10, 3-4=20, 5-6=30, 7-8=40, 9+=50.
pub const ACTIVE_WEEK_REQUIREMENT_UNITS: [u64; 5] = [10, 20, 30, 40, 50];
pub const ACTIVE_WEEK_TIER_SPAN: u32 = 2;

/// Weekly personal-unit thresholds for network monetization depth.
/// 10=>U3, 25=>U4, 50=>U5, 100=>U6, 200=>U7, 350=>U8, 500=>U9.
pub const NETWORK_DEPTH_UNIT_THRESHOLDS: [u64; 7] = [10, 25, 50, 100, 200, 350, 500];

pub const PIONEER_SLOTS: u16 = 100;
/// One Pioneer position is earned for each complete 1,000 units in one purchase.
/// Purchases never accumulate toward this threshold across transactions.
pub const PIONEER_POSITION_PURCHASE_UNITS: u64 = 1_000;
pub const PIONEER_SCALE: u128 = 1_000_000_000_000_000_000;

pub const BPS_DENOMINATOR: u64 = 10_000;
pub const SELF_BPS: u64 = 5_000;
pub const NETWORK_BPS: u64 = 4_300;
pub const PIONEER_BPS: u64 = 200;
pub const SERVICE_BPS: u64 = 500;

// Frozen production economics:
// SELF = buyer and receives SELF_BPS.
// The 43% network pool is distributed over exactly nine uplines, starting with sponsor.
pub const PURCHASE_NETWORK_LEVEL_BPS: [u64; 9] = [1_500, 900, 600, 400, 250, 200, 150, 100, 200];

pub const LEGACY_TOKEN_PROGRAM: Pubkey = anchor_spl::token::ID;

pub const MAINNET_USDT_MINT: Pubkey = pubkey!("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB");
pub const MAINNET_USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
pub const MAINNET_SERVICE_TREASURY: Pubkey =
    pubkey!("AepYo8xanmKuRiLVeYQuCTJoQr1nyKiTApoKwHMEg8fn");

// Safety-gated production launch: 2026-09-01 07:00 CEST = 2026-09-01 05:00 UTC.
// This remains deliberately future-dated until production runtime evidence and
// the independent audit are complete and bound to the exact release head.
pub const MAINNET_REGISTRATION_OPEN_AT: i64 = 1_788_238_800;

pub fn percentages_valid() -> bool {
    SELF_BPS + NETWORK_BPS + PIONEER_BPS + SERVICE_BPS == BPS_DENOMINATOR
        && PURCHASE_NETWORK_LEVEL_BPS.iter().copied().sum::<u64>() == NETWORK_BPS
}
