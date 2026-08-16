use anchor_lang::prelude::*;

pub const TOKEN_DECIMALS: u32 = 6;
pub const TOKEN_SCALE: u64 = 1_000_000;

pub const ACTIVITY_THRESHOLD_UNITS: u64 = 10;
pub const ACTIVE_SECONDS: i64 = 7 * 24 * 60 * 60;
pub const GRACE_SECONDS: i64 = 48 * 60 * 60;

pub const PIONEER_SLOTS: u16 = 100;
pub const PIONEER_SCALE: u128 = 1_000_000_000_000_000_000;

pub const BPS_DENOMINATOR: u64 = 10_000;
pub const DIRECT_BPS: u64 = 5_000;
pub const NETWORK_BPS: u64 = 4_300;
pub const PIONEER_BPS: u64 = 200;
pub const SERVICE_BPS: u64 = 500;

// Frozen production genealogy:
// L1 = direct sponsor and receives DIRECT_BPS only.
// The 43% network pool is distributed over genealogical L2-L10.
pub const PURCHASE_NETWORK_LEVEL_BPS: [u64; 9] =
    [1_500, 900, 600, 400, 250, 200, 150, 100, 200];

pub const LEGACY_TOKEN_PROGRAM: Pubkey = anchor_spl::token::ID;

pub const MAINNET_USDT_MINT: Pubkey =
    pubkey!("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB");
pub const MAINNET_USDC_MINT: Pubkey =
    pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
pub const MAINNET_SERVICE_TREASURY: Pubkey =
    pubkey!("AepYo8xanmKuRiLVeYQuCTJoQr1nyKiTApoKwHMEg8fn");

// Fail-closed launch sentinel. Replace only during the final reviewed mainnet freeze.
pub const MAINNET_REGISTRATION_OPEN_AT: i64 = 0;

pub fn percentages_valid() -> bool {
    DIRECT_BPS + NETWORK_BPS + PIONEER_BPS + SERVICE_BPS == BPS_DENOMINATOR
        && PURCHASE_NETWORK_LEVEL_BPS.iter().copied().sum::<u64>() == NETWORK_BPS
}
