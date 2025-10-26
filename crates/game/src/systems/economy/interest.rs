#![allow(dead_code)]

use super::{bankers_round_cents, round_down_to_cents, InterestCfg, MoneyCents};

const ONE_Q16: u64 = 1 << 16;

pub fn accrue_interest_per_leg(debt: MoneyCents, cfg: &InterestCfg) -> (MoneyCents, MoneyCents) {
    if debt.as_i64() <= 0 {
        return (MoneyCents::ZERO, debt);
    }
    let bp = leg_basis_points(debt, cfg);
    if bp <= 0 {
        return (MoneyCents::ZERO, debt);
    }
    let delta = apply_basis_points(debt, bp);
    let next = debt.saturating_add(delta);
    (delta, next)
}

fn leg_basis_points(debt: MoneyCents, cfg: &InterestCfg) -> i32 {
    let mut total = cfg.base_leg_bp as i64;
    total = total.saturating_add(linear_component(debt, cfg));
    total = total.saturating_add(convex_component(debt, cfg));
    total = total.min(cfg.per_leg_cap_bp as i64);
    total.max(0) as i32
}

fn linear_component(debt: MoneyCents, cfg: &InterestCfg) -> i64 {
    if cfg.linear_scale_cents <= 0 {
        return 0;
    }
    let ratio = (debt.as_i64().max(0) as i128) / cfg.linear_scale_cents as i128;
    (ratio as i64) * cfg.linear_leg_bp as i64
}

fn convex_component(debt: MoneyCents, cfg: &InterestCfg) -> i64 {
    if cfg.convex_leg_bp <= 0 || cfg.linear_scale_cents <= 0 {
        return 0;
    }
    let ratio = ratio_q16(debt, cfg);
    if ratio == 0 {
        return 0;
    }
    let pow = pow_q16(ratio, cfg.convex_gamma_q16);
    ((cfg.convex_leg_bp as i64) * (pow as i64) / ONE_Q16 as i64).max(0)
}

fn ratio_q16(debt: MoneyCents, cfg: &InterestCfg) -> u64 {
    if cfg.linear_scale_cents <= 0 {
        return 0;
    }
    let numerator = (debt.as_i64().max(0) as i128) << 16;
    let denom = cfg.linear_scale_cents as i128;
    if denom == 0 {
        return 0;
    }
    let ratio = numerator / denom;
    ratio.clamp(0, u64::MAX as i128) as u64
}

fn apply_basis_points(amount: MoneyCents, bp: i32) -> MoneyCents {
    if bp <= 0 {
        return MoneyCents::ZERO;
    }
    let base = i128::from(amount.as_i64());
    let multiplier = i128::from(bp);
    let intermediate = base.saturating_mul(multiplier).saturating_mul(10);
    let milli = intermediate.div_euclid(10_000);
    let rounded = bankers_round_cents(milli);
    round_down_to_cents(i128::from(rounded.as_i64()) * 10)
}

fn pow_q16(base: u64, exponent: u32) -> u64 {
    if base == 0 {
        return 0;
    }
    let mut result = ONE_Q16;
    let integer = exponent >> 16;
    for _ in 0..integer {
        result = mul_q16(result, base);
    }

    let fraction = exponent & 0xFFFF;
    if fraction == 0 {
        return result;
    }

    let mut factor = sqrt_q16(base);
    let mut bit = 1u32 << 15;
    while bit > 0 {
        if (fraction & bit) != 0 {
            result = mul_q16(result, factor);
        }
        factor = sqrt_q16(factor);
        bit >>= 1;
    }

    result
}

fn mul_q16(a: u64, b: u64) -> u64 {
    let prod = (a as u128) * (b as u128);
    let shifted = prod >> 16;
    shifted.min(u64::MAX as u128) as u64
}

fn sqrt_q16(x: u64) -> u64 {
    if x == 0 {
        return 0;
    }
    let value = (x as u128) << 16;
    integer_sqrt(value) as u64
}

fn integer_sqrt(mut n: u128) -> u128 {
    let mut result = 0u128;
    let mut bit = 1u128 << 126;
    while bit > n {
        bit >>= 2;
    }
    while bit != 0 {
        if n >= result + bit {
            n -= result + bit;
            result = (result >> 1) + bit;
        } else {
            result >>= 1;
        }
        bit >>= 2;
    }
    result
}
