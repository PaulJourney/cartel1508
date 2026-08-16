use anchor_lang::prelude::*;

use crate::constants::*;
use crate::state::{ActivityStatus, UserState};
use crate::ProtocolError;

pub fn mul_bps(amount: u64, bps: u64) -> Result<u64> {
    let value = (amount as u128)
        .checked_mul(bps as u128)
        .ok_or(ProtocolError::ArithmeticOverflow)?
        / (BPS_DENOMINATOR as u128);
    u64::try_from(value).map_err(|_| ProtocolError::ArithmeticOverflow.into())
}

/// Final production split for purchase-triggered economics.
/// The buyer receives SELF_BPS. `levels[0]` is the immutable sponsor and
/// `levels[8]` is the ninth network upline.
pub fn split_purchase_amount(amount: u64) -> Result<(u64, [u64; 9], u64, u64, u64)> {
    let self_reward = mul_bps(amount, SELF_BPS)?;
    let pioneer = mul_bps(amount, PIONEER_BPS)?;
    let service = mul_bps(amount, SERVICE_BPS)?;

    let mut levels = [0u64; 9];
    let mut network_sum = 0u64;
    for (i, bps) in PURCHASE_NETWORK_LEVEL_BPS.iter().enumerate() {
        levels[i] = mul_bps(amount, *bps)?;
        network_sum = network_sum
            .checked_add(levels[i])
            .ok_or(ProtocolError::ArithmeticOverflow)?;
    }

    let allocated = self_reward
        .checked_add(network_sum)
        .ok_or(ProtocolError::ArithmeticOverflow)?
        .checked_add(pioneer)
        .ok_or(ProtocolError::ArithmeticOverflow)?
        .checked_add(service)
        .ok_or(ProtocolError::ArithmeticOverflow)?;
    let rounding_remainder = amount
        .checked_sub(allocated)
        .ok_or(ProtocolError::ArithmeticUnderflow)?;

    Ok((self_reward, levels, pioneer, service, rounding_remainder))
}

pub fn allocate_unit_range(next_unit_id: u128, units: u64) -> Result<(u128, u128, u128)> {
    if units == 0 {
        return err!(ProtocolError::ZeroUnits);
    }
    let first = next_unit_id;
    let next = first
        .checked_add(units as u128)
        .ok_or(ProtocolError::ArithmeticOverflow)?;
    let last = next
        .checked_sub(1)
        .ok_or(ProtocolError::ArithmeticUnderflow)?;
    Ok((first, last, next))
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
    fn purchase_percentages_conserve_one_usdc() {
        let amount = 1_000_000u64;
        let (self_reward, levels, pioneer, service, remainder) =
            split_purchase_amount(amount).unwrap();
        assert_eq!(self_reward, 500_000);
        assert_eq!(
            levels,
            [150_000, 90_000, 60_000, 40_000, 25_000, 20_000, 15_000, 10_000, 20_000]
        );
        assert_eq!(levels.iter().sum::<u64>(), 430_000);
        assert_eq!(pioneer, 20_000);
        assert_eq!(service, 50_000);
        assert_eq!(remainder, 0);
    }

    #[test]
    fn global_unit_ranges_are_contiguous_unique_and_support_huge_batches() {
        let (first_a, last_a, next_a) = allocate_unit_range(1, 10).unwrap();
        assert_eq!((first_a, last_a, next_a), (1, 10, 11));
        let (first_b, last_b, next_b) =
            allocate_unit_range(next_a, 100_000_000_000).unwrap();
        assert_eq!(first_b, 11);
        assert_eq!(last_b, 100_000_000_010);
        assert_eq!(next_b, 100_000_000_011);
        assert!(last_a < first_b);
    }

    #[test]
    fn huge_batch_payment_fits_u64() {
        let units = 100_000_000_000u64;
        let payment = (units as u128) * (TOKEN_SCALE as u128);
        assert!(payment <= u64::MAX as u128);
    }

    #[test]
    fn split_conserves_every_sample_including_u64_boundaries() {
        let fixed = [
            1u64,
            2,
            3,
            99,
            100,
            9_999,
            10_000,
            999_999,
            1_000_000,
            100_000_000_000_000,
            u64::MAX / 2,
            u64::MAX - 1,
            u64::MAX,
        ];

        for amount in fixed {
            assert_split_conservation(amount);
        }

        let mut x = 0x9E37_79B9_7F4A_7C15u64;
        for _ in 0..50_000 {
            x = x
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            assert_split_conservation(x.max(1));
        }
    }

    #[test]
    fn random_unit_ranges_remain_contiguous_and_non_overlapping() {
        let mut next = 1u128;
        let mut previous_last = 0u128;
        let mut x = 0xD1B5_4A32_D192_ED03u64;

        for _ in 0..25_000 {
            x = x
                .wrapping_mul(2_862_933_555_777_941_757)
                .wrapping_add(3_037_000_493);
            let units = (x % 1_000_000_000_000).max(1);
            let (first, last, new_next) = allocate_unit_range(next, units).unwrap();

            assert_eq!(first, next);
            assert_eq!(last - first + 1, units as u128);
            assert_eq!(new_next, last + 1);
            assert!(first > previous_last);

            previous_last = last;
            next = new_next;
        }
    }

    fn assert_split_conservation(amount: u64) {
        let (self_reward, levels, pioneer, service, remainder) =
            split_purchase_amount(amount).unwrap();
        let network = levels
            .iter()
            .fold(0u128, |acc, value| acc + (*value as u128));
        let total = (self_reward as u128)
            + network
            + (pioneer as u128)
            + (service as u128)
            + (remainder as u128);

        assert_eq!(total, amount as u128);
        assert!(self_reward <= amount);
        assert!(pioneer <= amount);
        assert!(service <= amount);
        assert!(levels.iter().all(|value| *value <= amount));
    }
}
