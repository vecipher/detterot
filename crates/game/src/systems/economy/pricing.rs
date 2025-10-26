#![allow(dead_code)]

use super::{bankers_round_cents, round_down_to_cents, rulepack::PricingCfg, BasisBp, MoneyCents};

const BASIS_SCALE: i64 = 10_000;
const MILLI_CENT_SCALE: i128 = 10;

pub fn compute_price(
    base: MoneyCents,
    di: BasisBp,
    basis: BasisBp,
    pricing: &PricingCfg,
) -> MoneyCents {
    let drivers_bp = i64::from(di.0).saturating_add(i64::from(basis.0));
    let min_multiplier_bp = i64::from(pricing.min_multiplier_bp);
    let max_multiplier_bp = i64::from(pricing.max_multiplier_bp);
    let clamped_drivers_bp = drivers_bp.clamp(min_multiplier_bp, max_multiplier_bp);

    let multiplier = BASIS_SCALE.saturating_add(clamped_drivers_bp);

    let intermediate = i128::from(base.as_i64())
        .saturating_mul(i128::from(multiplier))
        .saturating_mul(MILLI_CENT_SCALE);

    let milli_cents = intermediate.div_euclid(i128::from(BASIS_SCALE));
    let rounded = bankers_round_cents(milli_cents);

    // Final floor ensures we never carry residuals beyond a cent even if
    // future changes tweak the rounding scheme.
    round_down_to_cents(i128::from(rounded.as_i64()) * MILLI_CENT_SCALE)
}
