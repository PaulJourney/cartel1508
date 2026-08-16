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

// Final production genealogy:
// L1 = direct sponsor and receives DIRECT_BPS only.
// The 43% network pool is therefore distributed over L2-L10 (9 levels).
// Minimal-delta freeze from the historical ten-weight table: the two deepest
// 1% buckets are consolidated into L10, preserving the full 43% total.
pub const PURCHASE_NETWORK_LEVEL_BPS: [u64; 9] = [1_500, 900, 600, 400, 250, 200, 150, 100, 200];

// Legacy development-only qualified-revenue table retained temporarily so the
// historical adapter regression suite can still be compiled while that stack is
// removed from the final production release surface.
pub const NETWORK_LEVEL_BPS: [u64; 10] = [1_500, 900, 600, 400, 250, 200, 150, 100, 100, 100];

pub const LEGACY_TOKEN_PROGRAM: Pubkey = anchor_spl::token::ID;
pub const MAINNET_USDT_MINT: Pubkey = pubkey!("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB");
pub const MAINNET_USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
pub const MAINNET_SERVICE_TREASURY: Pubkey = pubkey!("AepYo8xanmKuRiLVeYQuCTJoQr1nyKiTApoKwHMEg8fn");
// Fail-closed launch sentinels. Replace only during the final reviewed mainnet freeze.
// The legacy adapter/source constants are retained temporarily for regression builds
// but are not part of the intended final purchase-triggered production architecture.
pub const MAINNET_REVENUE_ADAPTER_PROGRAM: Pubkey = pubkey!("11111111111111111111111111111111");
pub const MAINNET_QUALIFIED_REVENUE_SOURCE: Pubkey = pubkey!("11111111111111111111111111111111");
pub const MAINNET_REGISTRATION_OPEN_AT: i64 = 0;

pub fn percentages_valid() -> bool {
    DIRECT_BPS + NETWORK_BPS + PIONEER_BPS + SERVICE_BPS == BPS_DENOMINATOR
        && PURCHASE_NETWORK_LEVEL_BPS.iter().copied().sum::<u64>() == NETWORK_BPS
        && NETWORK_LEVEL_BPS.iter().copied().sum::<u64>() == NETWORK_BPS
}
