#![allow(dead_code)]

use super::{BasisBp, DetRng, Pp, Rulepack, Weather};

#[derive(Debug, Clone, Copy)]
pub struct BasisDrivers {
    pub pp: Pp,
    pub weather: Weather,
    pub closed_routes: u8,
    pub stock_dev: i32,
}

pub fn update_basis(
    current: BasisBp,
    drivers: &BasisDrivers,
    rp: &Rulepack,
    rng: &mut DetRng,
) -> BasisBp {
    let cfg = &rp.basis;

    let pp_term = compute_pp_term(drivers.pp, rp);
    let weather_term = weather_impact(cfg, drivers.weather);
    let routes_term = i32::from(drivers.closed_routes) * cfg.beta_routes_bp;
    let stock_term = drivers.stock_dev * cfg.beta_stock_bp;
    let noise = rng
        .norm_bounded_bp(0, cfg.noise_sigma_bp, cfg.noise_clamp_bp)
        .0;

    let mut next =
        current.0 as i64 + (pp_term + weather_term + routes_term + stock_term + noise) as i64;

    let delta_cap = cfg.per_day_clamp_bp as i64;
    if delta_cap > 0 {
        let max_step = current.0 as i64 + delta_cap;
        let min_step = current.0 as i64 - delta_cap;
        if next > max_step {
            next = max_step;
        } else if next < min_step {
            next = min_step;
        }
    }

    let abs_min = cfg.absolute_min_bp as i64;
    let abs_max = cfg.absolute_max_bp as i64;
    next = next.clamp(abs_min, abs_max);

    BasisBp(next as i32)
}

fn compute_pp_term(pp: Pp, rp: &Rulepack) -> i32 {
    let neutral = rp.pp.neutral_pp as i32;
    let delta = pp.0 as i32 - neutral;
    // Scale PP deviation so that 100 points roughly equate to one coefficient step.
    let scaled = delta / 100;
    scaled * rp.basis.beta_pp_bp
}

fn weather_impact(cfg: &super::rulepack::BasisCfg, weather: Weather) -> i32 {
    match weather {
        Weather::Clear => cfg.weather.clear_bp,
        Weather::Rains => cfg.weather.rains_bp,
        Weather::Fog => cfg.weather.fog_bp,
        Weather::Windy => cfg.weather.windy_bp,
    }
}
