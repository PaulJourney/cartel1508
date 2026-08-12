use anchor_lang::prelude::*;

pub const TOKEN_DECIMALS: u32 = 6;
pub const TOKEN_SCALE: u64 = 1_000_000;
pub const ACTIVITY_THRESHOLD_UNITS: u64 = 10;
pub const ACTIVE_SECONDS: i64 = 7 * 24 * 60 * 60;
pub const GRACE_SECONDS: i64 = 48 * 60 * 60;
pub const PIONEER_SLOTS: u16 = 100;
pub const BPS_DENOMINATOR: u64 = 10_000;
pub const DIRECT_BPS: u64 = 5_000;
pub const NETWORK_BPS: u64 = 4_300;
pub const PIONEER_BPS: u64 = 200;
pub const SERVICE_BPS: u64 = 500;
pub const NETWORK_LEVEL_BPS: [u64; 10] = [1_500, 900, 600, 400, 250, 200, 150, 100, 100, 100];

pub const LEGACY_TOKEN_PROGRAM: Pubkey = anchor_spl::token::ID;
pub const MAINNET_USDT_MINT: Pubkey = pubkey!("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB");
pub const MAINNET_USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
pub const MAINNET_SERVICE_TREASURY: Pubkey = pubkey!("AepYo8xanmKuRiLVeYQuCTJoQr1nyKiTApoKwHMEg8fn");

pub fn assert_percentages() -> Result<()> {
    require!(DIRECT_BPS + NETWORK_BPS + PIONEER_BPS + SERVICE_BPS == BPS_DENOMINATOR, ErrorCode::InvalidPercentages);
    let sum: u64 = NETWORK_LEVEL_BPS.iter().copied().sum();
    require!(sum == NETWORK_BPS, ErrorCode::InvalidPercentages);
    Ok(())
}

#[error_code]
pub enum ErrorCode {
    #[msg("Protocol percentage constants are inconsistent")]
    InvalidPercentages,
}
