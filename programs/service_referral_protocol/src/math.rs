use anchor_lang::prelude::*;
use crate::constants::*;
use crate::state::{ActivityStatus, UserState};
use crate::ProtocolError;

pub fn mul_bps(amount: u64, bps: u64) -> Result<u64> {
    let v = (amount as u128)
        .checked_mul(bps as u128)
        .ok_or(ProtocolError::ArithmeticOverflow)?
        / (BPS_DENOMINATOR as u128);
    u64::try_from(v).map_err(|_| ProtocolError::ArithmeticOverflow.into())
}

pub fn split_amount(amount: u64) -> Result<(u64, [u64; 10], u64, u64, u64)> {
    let direct = mul_bps(amount, DIRECT_BPS)?;
    let pioneer = mul_bps(amount, PIONEER_BPS)?;
    let service = mul_bps(amount, SERVICE_BPS)?;
    let mut levels = [0u64; 10];
    let mut network_sum = 0u64;
    for (i, bps) in NETWORK_LEVEL_BPS.iter().enumerate() {
        levels[i] = mul_bps(amount, *bps)?;
        network_sum = network_sum.checked_add(levels[i]).ok_or(ProtocolError::ArithmeticOverflow)?;
    }
    let allocated = direct
        .checked_add(network_sum).ok_or(ProtocolError::ArithmeticOverflow)?
        .checked_add(pioneer).ok_or(ProtocolError::ArithmeticOverflow)?
        .checked_add(service).ok_or(ProtocolError::ArithmeticOverflow)?;
    let rounding_remainder = amount.checked_sub(allocated).ok_or(ProtocolError::ArithmeticUnderflow)?;
    Ok((direct, levels, pioneer, service, rounding_remainder))
}

pub fn activity_status(user: &UserState, now: i64) -> ActivityStatus {
    if user.active_until > 0 && now <= user.active_until {
        ActivityStatus::Active
    } else if user.grace_until > 0 && now <= user.grace_until {
        ActivityStatus::Grace
    } else {
        ActivityStatus::Inactive
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentages_conserve_one_usdc() {
        let amount = 1_000_000u64;
        let (direct, levels, pioneer, service, remainder) = split_amount(amount).unwrap();
        assert_eq!(direct, 500_000);
        assert_eq!(levels.iter().sum::<u64>(), 430_000);
        assert_eq!(pioneer, 20_000);
        assert_eq!(service, 50_000);
        assert_eq!(remainder, 0);
    }

    #[test]
    fn huge_batch_payment_fits_u64() {
        let units = 100_000_000_000u64;
        let payment = (units as u128) * (TOKEN_SCALE as u128);
        assert!(payment <= u64::MAX as u128);
    }
}
