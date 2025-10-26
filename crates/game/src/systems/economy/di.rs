#![allow(dead_code)]

use std::collections::HashMap;

use super::{rulepack::DiCfg, BasisBp, CommodityId, DetRng, EconomyDay, Rulepack};

#[derive(Debug, Clone, Default)]
pub struct DiState {
    pub per_com: HashMap<CommodityId, BasisBp>,
    pub overlay_bp: i32,
}

impl DiState {
    pub fn new(per_com: HashMap<CommodityId, BasisBp>) -> Self {
        Self {
            per_com,
            overlay_bp: 0,
        }
    }
}

pub fn step_di(day: EconomyDay, state: &mut DiState, rp: &Rulepack, rng: &mut DetRng) {
    let _ = day;
    let cfg = &rp.di;
    let mut keys: Vec<CommodityId> = state.per_com.keys().copied().collect();
    keys.sort_by_key(|id| id.0);

    for commodity in keys {
        let current = state
            .per_com
            .get(&commodity)
            .copied()
            .unwrap_or(BasisBp(cfg.long_run_mean_bp));
        let next = advance_value(current, state.overlay_bp, cfg, rng);
        state.per_com.insert(commodity, BasisBp(next));
    }

    decay_overlay(state, cfg);
}

fn advance_value(current: BasisBp, overlay_bp: i32, cfg: &DiCfg, rng: &mut DetRng) -> i32 {
    let mean = cfg.long_run_mean_bp as i64;
    let retention = cfg.retention_bp as i64;
    let deviation = current.0 as i64 - mean;
    let retained = deviation * retention / 10_000;
    let noise = rng.norm_bounded_bp(0, cfg.noise_sigma_bp, cfg.noise_clamp_bp);

    let mut next = mean + retained + overlay_bp as i64 + noise.0 as i64;
    let per_day = cfg.per_day_clamp_bp as i64;
    let current_i64 = current.0 as i64;
    if per_day > 0 {
        let max_step = current_i64 + per_day;
        let min_step = current_i64 - per_day;
        if next > max_step {
            next = max_step;
        } else if next < min_step {
            next = min_step;
        }
    }

    let abs_min = cfg.absolute_min_bp as i64;
    let abs_max = cfg.absolute_max_bp as i64;
    next = next.clamp(abs_min, abs_max);
    next as i32
}

fn decay_overlay(state: &mut DiState, cfg: &DiCfg) {
    let decay = cfg.overlay_decay_bp.max(0);
    if state.overlay_bp > 0 {
        state.overlay_bp = (state.overlay_bp - decay).max(0);
    } else if state.overlay_bp < 0 {
        state.overlay_bp = (state.overlay_bp + decay).min(0);
    }

    state.overlay_bp = state
        .overlay_bp
        .clamp(cfg.overlay_min_bp, cfg.overlay_max_bp);
}
