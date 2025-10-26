#![allow(dead_code)]

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::{
    basis::{update_basis, BasisDrivers},
    di::{step_di, DiState},
    interest::accrue_interest_per_leg,
    log,
    planting::apply_planting_pull,
    rot::convert_rot_to_debt,
    BasisBp, CommodityId, DetRng, EconomyDay, HubId, MoneyCents, Pp, Rulepack, Weather,
};

use super::planting::PendingPlanting;

const RNG_TAG_DI: u32 = 0;
const RNG_TAG_BASIS: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconState {
    pub day: EconomyDay,
    pub di_bp: HashMap<CommodityId, BasisBp>,
    pub di_overlay_bp: i32,
    pub basis_bp: HashMap<(HubId, CommodityId), BasisBp>,
    pub pp: Pp,
    pub rot_u16: u16,
    pub pending_planting: Vec<PendingPlanting>,
    pub debt_cents: MoneyCents,
}

impl Default for EconState {
    fn default() -> Self {
        Self {
            day: EconomyDay(0),
            di_bp: HashMap::new(),
            di_overlay_bp: 0,
            basis_bp: HashMap::new(),
            pp: Pp(0),
            rot_u16: 0,
            pending_planting: Vec::new(),
            debt_cents: MoneyCents::ZERO,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EconDelta {
    pub day: EconomyDay,
    pub hub: HubId,
    pub di: Vec<CommodityDelta>,
    pub basis: Vec<CommodityDelta>,
    pub pp_before: Pp,
    pub pp_after: Pp,
    pub rot_before: u16,
    pub rot_after: u16,
    pub debt_before: MoneyCents,
    pub interest_delta: MoneyCents,
    pub debt_after: MoneyCents,
    pub clamps_hit: Vec<String>,
    pub rng_cursors: Vec<RngCursor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommodityDelta {
    pub commodity: CommodityId,
    pub value: BasisBp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RngCursor {
    pub label: String,
    pub draws: u32,
}

impl EconDelta {
    fn new(day: EconomyDay, hub: HubId) -> Self {
        Self {
            day,
            hub,
            ..Default::default()
        }
    }
}

impl RngCursor {
    fn new(label: &'static str, draws: u32) -> Self {
        Self {
            label: label.to_string(),
            draws,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EconStepScope {
    /// Runs the global economy updates (DI, PP, debt, day counter) and then
    /// advances the basis for the provided hub.
    GlobalAndHub,
    /// Advances the basis for the provided hub without touching the global
    /// economy state. Use this for additional hubs within the same simulated
    /// day after [`GlobalAndHub`] has been executed once.
    HubOnly,
}

pub fn step_economy_day(
    rp: &Rulepack,
    world_seed: u64,
    econ_version: u32,
    hub: HubId,
    state: &mut EconState,

    scope: EconStepScope,
) -> EconDelta {
    let day = match scope {
        EconStepScope::GlobalAndHub => state.day,
        EconStepScope::HubOnly => EconomyDay(state.day.0.saturating_sub(1)),
    };
    let mut delta = EconDelta::new(day, hub);

    if matches!(scope, EconStepScope::GlobalAndHub) {
        // 1. DI step
        let mut di_state = DiState {
            per_com: state.di_bp.clone(),
            overlay_bp: state.di_overlay_bp,
        };
        let prev_di = di_state.per_com.clone();
        let mut rng_di =
            DetRng::from_seed_global(world_seed, econ_version, day, RNG_TAG_DI);
        step_di(day, &mut di_state, rp, &mut rng_di);
        state.di_bp = di_state.per_com;
        state.di_overlay_bp = di_state.overlay_bp;
        let mut di_entries: Vec<_> = state.di_bp.iter().collect();
        di_entries.sort_by_key(|(commodity, _)| commodity.0);
        for (commodity, value) in di_entries {
            delta.di.push(CommodityDelta {
                commodity: *commodity,
                value: *value,
            });
            if let Some(previous) = prev_di.get(commodity) {
                note_clamps(
                    &mut delta.clamps_hit,
                    "di",
                    *commodity,
                    previous,
                    value,
                    rp.di.per_day_clamp_bp,
                    rp.di.absolute_min_bp,
                    rp.di.absolute_max_bp,
                );
            }
        }
        delta
            .rng_cursors
            .push(RngCursor::new("di", rng_di.cursor()));

        // 2. Planting pull / PP decay
        delta.pp_before = state.pp;
        state.pp = apply_planting_pull(state.pp, state, &rp.pp);
        delta.pp_after = state.pp;

        // 3. Rot -> debt conversion
        delta.rot_before = state.rot_u16;
        let (rot_after, debt_delta) = convert_rot_to_debt(state.rot_u16, &rp.rot);
        state.rot_u16 = rot_after;
        delta.rot_after = rot_after;
        delta.debt_before = state.debt_cents;
        state.debt_cents = state.debt_cents.saturating_add(debt_delta);
        let (interest_delta, debt_with_interest) =
            accrue_interest_per_leg(state.debt_cents, &rp.interest);
        state.debt_cents = debt_with_interest;
        delta.interest_delta = interest_delta;
        delta.debt_after = state.debt_cents;

        // 4. Advance day
        state.day = EconomyDay(state.day.0.saturating_add(1));
    } else {
        delta.pp_before = state.pp;
        delta.pp_after = state.pp;
        delta.rot_before = state.rot_u16;
        delta.rot_after = state.rot_u16;
        delta.debt_before = state.debt_cents;
        delta.debt_after = state.debt_cents;
        delta.interest_delta = MoneyCents::ZERO;
    }

    // Basis updates for this hub
    let mut commodities: Vec<_> = state.di_bp.keys().copied().collect();
    commodities.sort_by_key(|c| c.0);
    let mut rng_basis = DetRng::from_seed(world_seed, econ_version, hub, day, RNG_TAG_BASIS);
    let drivers = BasisDrivers {
        pp: state.pp,
        weather: Weather::Clear,
        closed_routes: 0,
        stock_dev: 0,
    };
    for commodity in commodities {
        let key = (hub, commodity);
        let current = state.basis_bp.get(&key).copied().unwrap_or(BasisBp(0));
        let updated = update_basis(current, &drivers, rp, &mut rng_basis);
        note_clamps(
            &mut delta.clamps_hit,
            "basis",
            commodity,
            &current,
            &updated,
            rp.basis.per_day_clamp_bp,
            rp.basis.absolute_min_bp,
            rp.basis.absolute_max_bp,
        );
        state.basis_bp.insert(key, updated);
        delta.basis.push(CommodityDelta {
            commodity,
            value: updated,
        });
    }
    delta
        .rng_cursors
        .push(RngCursor::new("basis", rng_basis.cursor()));

    if matches!(scope, EconStepScope::GlobalAndHub) {
        log::log_econ_tick(&delta, &rp.pricing);
    }

    delta
}

#[allow(clippy::too_many_arguments)]
fn note_clamps(
    clamps: &mut Vec<String>,
    label: &'static str,
    commodity: CommodityId,
    previous: &BasisBp,
    updated: &BasisBp,
    per_day: i32,
    abs_min: i32,
    abs_max: i32,
) {
    if updated.0 == abs_min {
        clamps.push(format!("{label}:{}:abs_min", commodity.0));
    } else if updated.0 == abs_max {
        clamps.push(format!("{label}:{}:abs_max", commodity.0));
    }
    if per_day > 0 && (updated.0 - previous.0).abs() == per_day {
        clamps.push(format!("{label}:{}:per_day", commodity.0));
    }
}
