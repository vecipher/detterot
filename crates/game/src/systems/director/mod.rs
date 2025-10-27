mod econ_intent;
pub mod missions;
pub mod pause_wheel;
pub mod spawn;

pub mod config;
pub mod rng;

use avian3d::prelude::{PhysicsSchedulePlugin, SubstepCount};
#[cfg(feature = "deterministic")]
use bevy::ecs::schedule::ExecutorKind;
use bevy::ecs::schedule::{Schedule, ScheduleLabel};
use bevy::prelude::*;
use bevy::time::Fixed;
use std::path::{Path, PathBuf};

use crate::logs::m2;
use crate::scheduling::sets;
use crate::systems::command_queue::CommandQueue;
use crate::systems::economy::{Pp, RouteId, Weather};

pub use econ_intent::EconIntent;
pub use missions::{MissionResult, MissionRuntime};
pub use pause_wheel::{PauseState, Stance, ToolSlot, WheelState};
pub use spawn::{
    choose_spawn_type, compute_spawn_budget, danger_diff_sign, danger_score, SpawnBudget,
    SpawnTypeTables,
};

use self::config::load_director_cfg;
use self::rng::{hash_mission_name, mission_seed};

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

#[derive(Resource, Debug, Clone)]
pub struct DirectorState {
    pub world_seed: u64,
    pub day: u32,
    pub leg_tick: u32,
    pub status: LegStatus,
    pub link_id: RouteId,
    pub weather: Weather,
    pub prior_danger_score: i32,
    pub current_danger_score: i32,
}

impl Default for DirectorState {
    fn default() -> Self {
        Self {
            world_seed: 0,
            day: 0,
            leg_tick: 0,
            status: LegStatus::Loading,
            link_id: RouteId::default(),
            weather: Weather::default(),
            prior_danger_score: 0,
            current_danger_score: 0,
        }
    }
}

#[derive(Resource, Clone)]
pub struct DirectorConfigResource(pub config::DirectorCfg);

#[derive(Resource, Default, Clone)]
pub struct MissionCatalog(pub Vec<(String, config::MissionCfg)>);

#[derive(Resource, Default, Clone, Copy)]
pub struct LegContext {
    pub world_seed: u64,
    pub link_id: RouteId,
    pub day: u32,
    pub weather: Weather,
    pub pp: Pp,
    pub density_per_10k: u32,
    pub cadence_per_min: u32,
    pub mission_minutes: u32,
    pub player_rating: u8,
}

#[derive(Resource, Default, Clone, Copy)]
pub struct SpawnMemory {
    pub prior_enemies: Option<u32>,
    pub last_budget: Option<SpawnBudget>,
    pub pending_budget: Option<SpawnBudget>,
    pub spawn_seed: u64,
    pub spawn_counter: u64,
    pub last_spawned_enemies: u32,
}

#[derive(ScheduleLabel, Debug, Hash, PartialEq, Eq, Clone)]
struct DirectorPhysicsSchedule;

pub struct DirectorPlugin;

impl Plugin for DirectorPlugin {
    fn build(&self, app: &mut App) {
        let cfg_path = director_cfg_path();
        let cfg = load_director_cfg(cfg_path.to_str().expect("cfg path"))
            .unwrap_or_else(|_| panic!("director config missing: {}", cfg_path.display()));
        let spawn_tables = SpawnTypeTables::from_cfg(&cfg);

        let mut missions: Vec<(String, config::MissionCfg)> = cfg
            .missions
            .iter()
            .map(|(name, cfg)| (name.clone(), cfg.clone()))
            .collect();
        missions.sort_by(|a, b| a.0.cmp(&b.0));
        let catalog = MissionCatalog(missions);

        app.add_schedule(Schedule::new(DirectorPhysicsSchedule));
        app.add_plugins(PhysicsSchedulePlugin::new(DirectorPhysicsSchedule));

        #[cfg(feature = "deterministic")]
        {
            app.edit_schedule(DirectorPhysicsSchedule, |schedule| {
                schedule.set_executor_kind(ExecutorKind::SingleThreaded);
            });
        }

        app.insert_resource(DirectorConfigResource(cfg))
            .insert_resource(catalog)
            .insert_resource(spawn_tables)
            .init_resource::<DirectorState>()
            .init_resource::<MissionRuntime>()
            .init_resource::<EconIntent>()
            .init_resource::<WheelState>()
            .init_resource::<PauseState>()
            .init_resource::<SpawnMemory>()
            .init_resource::<LegContext>()
            .add_systems(Startup, setup_director)
            .add_systems(
                FixedUpdate,
                (
                    drive_director.in_set(sets::DETTEROT_Director),
                    run_mission_runtime.in_set(sets::DETTEROT_Missions),
                    dispatch_spawns.in_set(sets::DETTEROT_Spawns),
                    physics_step.in_set(sets::DETTEROT_PhysicsStep),
                    finalize_leg.in_set(sets::DETTEROT_Cleanup),
                ),
            );
    }
}

fn director_cfg_path() -> PathBuf {
    let default = Path::new("assets/director/m2.toml");
    if default.exists() {
        return default.to_path_buf();
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/director/m2.toml")
}

fn setup_director(
    mut state: ResMut<DirectorState>,
    catalog: Res<MissionCatalog>,
    mut runtime: ResMut<MissionRuntime>,
    mut memory: ResMut<SpawnMemory>,
    context: Res<LegContext>,
) {
    state.status = LegStatus::Running;
    state.link_id = context.link_id;
    state.weather = context.weather;
    state.world_seed = context.world_seed;
    state.day = context.day;
    runtime.init_all(context.world_seed, context.link_id, context.day, &catalog.0);
    let spawn_id = hash_mission_name("spawn_types");
    memory.spawn_seed = mission_seed(context.world_seed, context.link_id, context.day, spawn_id);
    memory.spawn_counter = 0;
}

fn drive_director(
    mut state: ResMut<DirectorState>,
    cfg: Res<DirectorConfigResource>,
    mut memory: ResMut<SpawnMemory>,
    context: Res<LegContext>,
    mut queue: ResMut<CommandQueue>,
) {
    if !matches!(state.status, LegStatus::Running) {
        return;
    }

    let previous_budget = memory.last_budget;
    let budget = compute_spawn_budget(context.pp, state.weather, memory.prior_enemies, &cfg.0);
    let spawn_changed = previous_budget.map(|b| b != budget).unwrap_or(true);
    if spawn_changed {
        memory.pending_budget = Some(budget);
        let weather_string = format!("{:?}", state.weather);
        let _ = m2::log_spawn_budget(
            state.leg_tick,
            state.link_id.0,
            context.pp.0,
            &weather_string,
            &budget,
        );
    }
    memory.last_budget = Some(budget);

    let prior_danger = state.prior_danger_score;
    let previous_value = state.current_danger_score;
    let danger = danger_score(
        &budget,
        context.mission_minutes,
        context.density_per_10k,
        context.cadence_per_min,
        context.player_rating,
    );
    let diff = danger_diff_sign(danger, prior_danger);
    if state.leg_tick == 0 || danger != previous_value {
        queue.meter("danger_score", danger);
        queue.meter("danger_diff", diff);
    }

    state.current_danger_score = danger;
}

fn run_mission_runtime(
    mut runtime: ResMut<MissionRuntime>,
    mut queue: ResMut<CommandQueue>,
    mut econ: ResMut<EconIntent>,
    state: Res<DirectorState>,
) {
    if !matches!(state.status, LegStatus::Running) {
        return;
    }
    runtime.tick_all(state.leg_tick, 1, queue.as_mut(), econ.as_mut());
}

fn dispatch_spawns(
    mut memory: ResMut<SpawnMemory>,
    mut queue: ResMut<CommandQueue>,
    tables: Res<SpawnTypeTables>,
    state: Res<DirectorState>,
) {
    if !matches!(state.status, LegStatus::Running) {
        memory.pending_budget = None;
        return;
    }

    if let Some(budget) = memory.pending_budget.take() {
        queue.meter("spawn_count", budget.enemies as i32);
        let base_x = (state.leg_tick as i32) * 1000;
        let previous_spawned = memory.last_spawned_enemies;
        let desired_spawned = budget.enemies;
        let new_spawns = desired_spawned.saturating_sub(previous_spawned);
        for idx in 0..new_spawns {
            let offset_mm = (idx as i32) * 100;
            let kind = choose_spawn_type(
                &tables,
                state.weather,
                memory.spawn_seed,
                memory.spawn_counter,
            );
            memory.spawn_counter = memory.spawn_counter.saturating_add(1);
            queue.spawn(&kind, base_x + offset_mm, 0, 0);
        }
        memory.last_spawned_enemies = previous_spawned.max(desired_spawned);
        memory.prior_enemies = Some(memory.last_spawned_enemies);
    }
}

fn physics_step(world: &mut World) {
    if !matches!(world.resource::<DirectorState>().status, LegStatus::Running) {
        return;
    }

    let context = *world.resource::<LegContext>();
    if let Some(mut substeps) = world.get_resource_mut::<SubstepCount>() {
        let cadence = context.cadence_per_min;
        let desired = if cadence == 0 {
            1
        } else {
            ((cadence - 1) / 60).saturating_add(1)
        };
        let target = desired.clamp(1, 12);
        if substeps.0 != target {
            substeps.0 = target;
        }
    }

    let fixed_delta = world.resource::<Time<Fixed>>().timestep();
    world.resource_mut::<Time>().advance_by(fixed_delta);

    world.run_schedule(DirectorPhysicsSchedule);
}

fn finalize_leg(
    mut state: ResMut<DirectorState>,
    mut econ: ResMut<EconIntent>,
    mut queue: ResMut<CommandQueue>,
) {
    if !matches!(state.status, LegStatus::Running) {
        econ.clear();
        return;
    }
    if econ.pending_pp_delta != 0 {
        queue.meter("econ_pp_pending", econ.pending_pp_delta as i32);
    }
    if econ.pending_basis_overlay_bp != 0 {
        queue.meter("econ_basis_pending", econ.pending_basis_overlay_bp as i32);
    }
    let danger_delta = state.current_danger_score - state.prior_danger_score;
    let _ = m2::log_post_leg_summary(
        danger_delta,
        econ.pending_pp_delta,
        econ.pending_basis_overlay_bp,
        state.current_danger_score,
        econ.pending_basis_overlay_bp as i32,
    );
    state.prior_danger_score = state.current_danger_score;
    if state.leg_tick >= 600 {
        state.status = LegStatus::Completed(Outcome::Success);
    }
    state.leg_tick = state.leg_tick.saturating_add(1);
    econ.clear();
}
