#![allow(dead_code)]

use super::{round_down_to_cents, bankers_round_cents, BasisBp, MoneyCents};

const BASIS_SCALE: i64 = 10_000;
const MILLI_CENT_SCALE: i128 = 10;

pub fn compute_price(base: MoneyCents, di: BasisBp, basis: BasisBp) -> MoneyCents {
    let multiplier = BASIS_SCALE
        .saturating_add(di.0 as i64)
        .saturating_add(basis.0 as i64);

    let intermediate = i128::from(base.as_i64())
        .saturating_mul(i128::from(multiplier))
        .saturating_mul(MILLI_CENT_SCALE);

    let milli_cents = intermediate.div_euclid(i128::from(BASIS_SCALE));
    let rounded = bankers_round_cents(milli_cents);

    // Final floor ensures we never carry residuals beyond a cent even if
    // future changes tweak the rounding scheme.
    round_down_to_cents(i128::from(rounded.as_i64()) * MILLI_CENT_SCALE)
}
