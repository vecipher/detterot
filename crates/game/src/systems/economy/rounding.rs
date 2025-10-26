#![allow(dead_code)]

use super::money::MoneyCents;

pub fn bankers_round_cents(milli_cents: i128) -> MoneyCents {
    let mut cents = milli_cents / 10;
    let remainder = milli_cents % 10;

    if remainder != 0 {
        let abs_rem = remainder.abs();
        let direction = remainder.signum();
        if abs_rem > 5 || (abs_rem == 5 && cents & 1 != 0) {
            cents = cents.saturating_add(direction);
        }
    }

    MoneyCents::from_i128_clamped(cents)
}

pub fn round_down_to_cents(milli_cents: i128) -> MoneyCents {
    let cents = milli_cents.div_euclid(10);
    MoneyCents::from_i128_clamped(cents)
}
