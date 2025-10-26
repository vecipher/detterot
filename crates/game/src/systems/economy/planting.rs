#![allow(dead_code)]

use serde::{Deserialize, Serialize};

use super::{state::EconState, Pp, PpCfg};
use super::types::HubId;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingPlanting {
    pub hub: HubId,
    pub size: u8,
    pub age_days: u16,
}

pub fn schedule_planting(mut planting: PendingPlanting, state: &mut EconState) {
    planting.age_days = 0;
    state.pending_planting.push(planting);
}

pub fn apply_planting_pull(pp: Pp, state: &mut EconState, cfg: &PpCfg) -> Pp {
    let mut total_pull: i64 = 0;
    for planting in state.pending_planting.iter_mut() {
        let base_pull = i64::from(cfg.planting_size_to_pp_bp) * i64::from(planting.size);
        let decay = i64::from(cfg.decay_per_day_bp) * i64::from(planting.age_days);
        let contribution = (base_pull - decay).max(0);
        total_pull += contribution;
        planting.age_days = planting.age_days.saturating_add(1);
    }

    state.pending_planting.retain(|p| {
        if p.age_days >= cfg.planting_max_age_days || p.size == 0 {
            return false;
        }
        let base_pull = i64::from(cfg.planting_size_to_pp_bp) * i64::from(p.size);
        let decay = i64::from(cfg.decay_per_day_bp) * i64::from(p.age_days);
        base_pull - decay > 0
    });

    let delta_pp = (total_pull * i64::from(cfg.pull_strength_bp)).max(0) / 10_000;
    let mut new_pp = pp.0 as i64;
    let decay = i64::from(cfg.pull_decay_bp).max(0);
    new_pp -= decay;
    new_pp += delta_pp;
    new_pp = new_pp.clamp(cfg.min_pp as i64, cfg.max_pp as i64);
    Pp(new_pp as u16)
}
