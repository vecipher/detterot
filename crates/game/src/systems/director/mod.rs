use std::collections::HashSet;

use bevy::prelude::*;

use crate::logs::m2;
use crate::scheduling::sets;
use crate::systems::command_queue::CommandQueue;
use crate::systems::economy::{Pp, RouteId, Weather};

pub mod config;
pub mod missions;
pub mod pause_wheel;
pub mod spawn;

use config::{load_director_cfg, DirectorCfg};
use missions::{MissionBank, MissionResult};
use pause_wheel::{apply_slowmo_time, PauseState, WheelState};
use spawn::{
    compute_spawn_budget, danger_diff_sign, danger_score, select_spawn_kind, spawn_position,
    wyhash64, DetRng, SpawnBudget,
};

pub const DIRECTOR_CFG_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/director/m2.toml");

#[derive(Resource, Debug, Clone)]
pub struct DirectorState {
    pub leg_tick: u32,
    pub status: LegStatus,
    pub link_id: RouteId,
    pub weather: Weather,
    pub prior_danger_score: i32,
    pub rng_salt: u64,
}

impl Default for DirectorState {
    fn default() -> Self {
        Self {
            leg_tick: 0,
            status: LegStatus::Loading,
            link_id: RouteId(1),
            weather: Weather::Clear,
            prior_danger_score: 0,
            rng_salt: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Success,
    Failure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegStatus {
    Loading,
    Running,
    Paused,
    Completed(Outcome),
}

#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct EconIntent {
    pub pending_pp_delta: i16,
    pub pending_basis_overlay_bp: i16,
}

#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectorRequests {
    pub queue_new_leg: bool,
}

#[derive(Resource, Debug, Clone)]
pub struct DirectorInputs {
    pub world_seed: u64,
    pub link_id: RouteId,
    pub day: u32,
    pub mission_minutes: u32,
    pub density_per_10k: u32,
    pub cadence_per_min: u32,
    pub player_rating: u8,
    pub pp: Pp,
}

impl Default for DirectorInputs {
    fn default() -> Self {
        Self {
            world_seed: 1,
            link_id: RouteId(1),
            day: 1,
            mission_minutes: 12,
            density_per_10k: 5,
            cadence_per_min: 3,
            player_rating: 50,
            pp: Pp(5000),
        }
    }
}

#[derive(Resource)]
struct DirectorRuntime {
    cfg: Option<DirectorCfg>,
    mission_bank: MissionBank,
    missions_ready: bool,
    active_missions: HashSet<String>,
    spawn_budget: Option<SpawnBudget>,
    prior_enemies: Option<u32>,
    last_link_id: Option<RouteId>,
    spawn_seed: u64,
    spawns_emitted: bool,
    missions_resolved: usize,
    missions_total: usize,
    any_failure: bool,
    current_danger_score: i32,
    needs_budget_refresh: bool,
    has_prior_danger: bool,
}

impl Default for DirectorRuntime {
    fn default() -> Self {
        Self {
            cfg: None,
            mission_bank: MissionBank::default(),
            missions_ready: false,
            active_missions: HashSet::new(),
            spawn_budget: None,
            prior_enemies: None,
            last_link_id: None,
            spawn_seed: 0,
            spawns_emitted: false,
            missions_resolved: 0,
            missions_total: 0,
            any_failure: false,
            current_danger_score: 0,
            needs_budget_refresh: true,
            has_prior_danger: false,
        }
    }
}

pub struct DirectorPlugin;

impl Plugin for DirectorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CommandQueue>();
        app.init_resource::<DirectorState>();
        app.init_resource::<DirectorInputs>();
        app.init_resource::<EconIntent>();
        app.init_resource::<DirectorRequests>();
        app.insert_resource(WheelState::default());
        app.insert_resource(PauseState::default());
        app.insert_resource(DirectorRuntime::default());
        app.add_systems(Startup, load_config_system);
        app.add_systems(Update, apply_slowmo_time);
        app.add_systems(
            FixedUpdate,
            (
                director_tick_system.in_set(sets::DETTEROT_Director),
                missions_system.in_set(sets::DETTEROT_Missions),
                spawn_system.in_set(sets::DETTEROT_Spawns),
                cleanup_system.in_set(sets::DETTEROT_Cleanup),
            ),
        );
    }
}

fn load_config_system(mut runtime: ResMut<DirectorRuntime>) {
    if runtime.cfg.is_some() {
        return;
    }
    match load_director_cfg(DIRECTOR_CFG_PATH) {
        Ok(cfg) => runtime.cfg = Some(cfg),
        Err(err) => panic!("failed to load director config: {err}"),
    }
}

fn director_tick_system(
    mut state: ResMut<DirectorState>,
    inputs: Res<DirectorInputs>,
    mut runtime: ResMut<DirectorRuntime>,
    mut queue: ResMut<CommandQueue>,
    mut requests: ResMut<DirectorRequests>,
    pause: Res<PauseState>,
) {
    if pause.hard_paused_sp {
        return;
    }

    if runtime.cfg.is_none() {
        return;
    }

    if requests.queue_new_leg {
        if runtime.last_link_id != Some(inputs.link_id) {
            runtime.prior_enemies = None;
        }
        state.status = LegStatus::Loading;
        runtime.missions_ready = false;
        runtime.active_missions.clear();
        runtime.spawn_budget = None;
        runtime.spawns_emitted = false;
        runtime.missions_resolved = 0;
        runtime.missions_total = 0;
        runtime.any_failure = false;
        runtime.needs_budget_refresh = true;
        requests.queue_new_leg = false;
    }

    if !runtime.missions_ready
        && matches!(state.status, LegStatus::Loading)
        && !prepare_new_leg(&mut runtime, &mut state, &inputs)
    {
        return;
    }

    if !runtime.missions_ready {
        return;
    }

    queue.set_tick(state.leg_tick);
    if matches!(state.status, LegStatus::Loading) {
        state.status = LegStatus::Running;
    }

    if runtime.needs_budget_refresh {
        let cfg = runtime.cfg.as_ref().unwrap();
        let budget = compute_spawn_budget(inputs.pp, state.weather, runtime.prior_enemies, cfg);
        let danger = danger_score(
            &budget,
            inputs.mission_minutes,
            inputs.density_per_10k,
            inputs.cadence_per_min,
            inputs.player_rating,
        );
        queue.meter("danger_score", danger);
        if runtime.has_prior_danger {
            let diff = danger_diff_sign(danger, state.prior_danger_score);
            queue.meter("danger_diff", diff);
        }
        runtime.spawn_seed = wyhash64(inputs.world_seed, state.link_id, inputs.day, 0xBC_u64 << 32);
        runtime.current_danger_score = danger;
        state.rng_salt = runtime.spawn_seed;
        runtime.spawn_budget = Some(budget);
        runtime.prior_enemies = Some(budget.enemies);
        runtime.spawns_emitted = false;
        runtime.needs_budget_refresh = false;
    }
}

fn prepare_new_leg(
    runtime: &mut DirectorRuntime,
    state: &mut DirectorState,
    inputs: &DirectorInputs,
) -> bool {
    let cfg = match runtime.cfg.as_ref() {
        Some(cfg) => cfg,
        None => return false,
    };

    runtime.active_missions.clear();
    let base_seed = wyhash64(inputs.world_seed, inputs.link_id, inputs.day, 0xA5A5_0001);
    for (index, (name, mission)) in runtime.mission_bank.iter_mut().into_iter().enumerate() {
        if let Some(mission_cfg) = cfg.missions.get(name) {
            let seed = spawn::split_seed(base_seed, index as u64 + 1);
            mission.init(seed, mission_cfg);
            runtime.active_missions.insert(name.to_string());
        }
    }

    runtime.missions_total = runtime.active_missions.len();
    runtime.missions_resolved = 0;
    runtime.any_failure = false;
    runtime.missions_ready = true;
    runtime.spawn_budget = None;
    runtime.spawns_emitted = false;
    runtime.needs_budget_refresh = true;
    runtime.last_link_id = Some(inputs.link_id);
    state.link_id = inputs.link_id;
    state.leg_tick = 0;
    true
}

fn missions_system(
    mut runtime: ResMut<DirectorRuntime>,
    mut econ: ResMut<EconIntent>,
    mut queue: ResMut<CommandQueue>,
    state: Res<DirectorState>,
    pause: Res<PauseState>,
) {
    if pause.hard_paused_sp {
        return;
    }

    if runtime.cfg.is_none() {
        return;
    }

    let tick = state.leg_tick;
    queue.set_tick(tick.saturating_add(1));
    let active = runtime.active_missions.clone();
    let mut resolved = Vec::new();
    let mut saw_failure = false;
    for (name, mission) in runtime.mission_bank.iter_mut().into_iter() {
        if !active.contains(name) {
            continue;
        }
        if let Some(result) = mission.tick(1) {
            apply_mission_result(name, result, &mut econ, &mut queue, tick);
            resolved.push(name.to_string());
            if matches!(result, MissionResult::Fail { .. }) {
                saw_failure = true;
            }
        }
    }
    if saw_failure {
        runtime.any_failure = true;
    }
    runtime.missions_resolved = runtime.missions_resolved.saturating_add(resolved.len());
    for name in resolved {
        runtime.active_missions.remove(&name);
    }
}

fn apply_mission_result(
    name: &str,
    result: MissionResult,
    econ: &mut EconIntent,
    queue: &mut CommandQueue,
    tick: u32,
) {
    match result {
        MissionResult::Success {
            pp_delta,
            basis_bp_overlay,
        } => {
            econ.pending_pp_delta = econ.pending_pp_delta.saturating_add(pp_delta);
            econ.pending_basis_overlay_bp = econ
                .pending_basis_overlay_bp
                .saturating_add(basis_bp_overlay);
            queue.meter("pp_delta", pp_delta as i32);
            queue.meter("basis_bp_overlay", basis_bp_overlay as i32);
            m2::log_mission_result(name, "success", pp_delta, basis_bp_overlay, tick);
        }
        MissionResult::Fail {
            pp_delta,
            basis_bp_overlay,
        } => {
            econ.pending_pp_delta = econ.pending_pp_delta.saturating_add(pp_delta);
            econ.pending_basis_overlay_bp = econ
                .pending_basis_overlay_bp
                .saturating_add(basis_bp_overlay);
            queue.meter("pp_delta", pp_delta as i32);
            queue.meter("basis_bp_overlay", basis_bp_overlay as i32);
            m2::log_mission_result(name, "fail", pp_delta, basis_bp_overlay, tick);
        }
    }
}

fn spawn_system(
    mut runtime: ResMut<DirectorRuntime>,
    state: Res<DirectorState>,
    inputs: Res<DirectorInputs>,
    mut queue: ResMut<CommandQueue>,
    pause: Res<PauseState>,
) {
    if pause.hard_paused_sp {
        return;
    }

    if runtime.spawns_emitted {
        return;
    }
    let budget = match runtime.spawn_budget {
        Some(budget) => budget,
        None => return,
    };
    let cfg = match runtime.cfg.as_ref() {
        Some(cfg) => cfg,
        None => return,
    };

    queue.set_tick(state.leg_tick.saturating_add(1));
    let mut rng = DetRng::new(runtime.spawn_seed);
    for index in 0..budget.enemies {
        let kind =
            select_spawn_kind(cfg, state.weather, &mut rng).unwrap_or_else(|| "enemy".to_string());
        let seed = spawn::split_seed(runtime.spawn_seed, index as u64 + 1);
        let (x_mm, y_mm, z_mm) = spawn_position(seed, index);
        queue.spawn(&kind, x_mm, y_mm, z_mm);
    }
    runtime.spawns_emitted = true;
    m2::log_spawn_budget(
        state.leg_tick,
        state.link_id,
        inputs.pp,
        state.weather,
        budget,
    );
}

fn cleanup_system(
    mut state: ResMut<DirectorState>,
    mut runtime: ResMut<DirectorRuntime>,
    mut econ: ResMut<EconIntent>,
    pause: Res<PauseState>,
) {
    if pause.hard_paused_sp {
        return;
    }

    if runtime.missions_total > 0
        && runtime.missions_resolved >= runtime.missions_total
        && !matches!(state.status, LegStatus::Completed(_))
    {
        let outcome = if runtime.any_failure {
            Outcome::Failure
        } else {
            Outcome::Success
        };
        state.status = LegStatus::Completed(outcome);
        let danger_delta = runtime.current_danger_score - state.prior_danger_score;
        m2::log_post_leg_summary(
            state.leg_tick,
            danger_delta,
            econ.pending_pp_delta,
            econ.pending_basis_overlay_bp,
            0,
            0,
        );
        state.prior_danger_score = runtime.current_danger_score;
        runtime.has_prior_danger = true;
        *econ = EconIntent::default();
        runtime.missions_ready = false;
        runtime.active_missions.clear();
        runtime.spawn_budget = None;
        runtime.spawns_emitted = false;
        runtime.needs_budget_refresh = true;
    }
    state.leg_tick = state.leg_tick.saturating_add(1);
}
