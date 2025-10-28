mod econ_intent;
pub mod input;
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
use std::time::Duration;

use crate::logs::m2;
use crate::scheduling::sets;
use crate::systems::command_queue::CommandQueue;
use crate::systems::economy::{Pp, RouteId, Weather};

pub use econ_intent::EconIntent;
pub use input::{apply_wheel_inputs, WheelInputAction, WheelInputQueue};
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
    pub multiplayer: bool,
    pub prior_danger_score: Option<i32>,
    pub basis_overlay_bp_total: i32,
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
            .init_resource::<WheelInputQueue>()
            .init_resource::<SpawnMemory>()
            .init_resource::<LegContext>()
            .add_systems(Startup, setup_director)
            .add_systems(
                FixedUpdate,
                (
                    apply_wheel_inputs.in_set(sets::DETTEROT_Input),
                    sync_pause_state.in_set(sets::DETTEROT_Director),
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
    state.prior_danger_score = context.prior_danger_score.unwrap_or_default();
    runtime.init_all(context.world_seed, context.link_id, context.day, &catalog.0);
    let spawn_id = hash_mission_name("spawn_types");
    memory.spawn_seed = mission_seed(context.world_seed, context.link_id, context.day, spawn_id);
    memory.spawn_counter = 0;
}

fn sync_pause_state(mut state: ResMut<DirectorState>, pause: Res<PauseState>) {
    match state.status {
        LegStatus::Running | LegStatus::Paused => {
            state.status = if pause.hard_paused_sp {
                LegStatus::Paused
            } else {
                LegStatus::Running
            };
        }
        _ => {}
    }
}

fn drive_director(
    mut state: ResMut<DirectorState>,
    cfg: Res<DirectorConfigResource>,
    mut memory: ResMut<SpawnMemory>,
    context: Res<LegContext>,
    mut queue: ResMut<CommandQueue>,
    pause: Res<PauseState>,
) {
    if !matches!(state.status, LegStatus::Running | LegStatus::Paused) {
        return;
    }
    if pause.hard_paused_sp {
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
    pause: Res<PauseState>,
) {
    if !matches!(state.status, LegStatus::Running | LegStatus::Paused) {
        return;
    }
    if pause.hard_paused_sp {
        return;
    }
    runtime.tick_all(state.leg_tick, 1, queue.as_mut(), econ.as_mut());
}

fn dispatch_spawns(
    mut memory: ResMut<SpawnMemory>,
    mut queue: ResMut<CommandQueue>,
    tables: Res<SpawnTypeTables>,
    state: Res<DirectorState>,
    pause: Res<PauseState>,
) {
    if !matches!(state.status, LegStatus::Running | LegStatus::Paused) {
        memory.pending_budget = None;
        return;
    }
    if pause.hard_paused_sp {
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

const SLOWMO_NUMERATOR: u32 = 4;
const SLOWMO_DENOMINATOR: u32 = 5;

fn scale_duration(duration: Duration, numerator: u32, denominator: u32) -> Duration {
    if numerator == denominator {
        return duration;
    }

    let total_nanos = duration.as_nanos();
    let scaled = total_nanos
        .saturating_mul(numerator as u128)
        .saturating_div(denominator as u128);

    Duration::new(
        (scaled / 1_000_000_000) as u64,
        (scaled % 1_000_000_000) as u32,
    )
}

fn physics_step(world: &mut World) {
    let paused = world.resource::<PauseState>().hard_paused_sp;
    let status = {
        let state = world.resource::<DirectorState>();
        state.status
    };
    if paused || !matches!(status, LegStatus::Running | LegStatus::Paused) {
        return;
    }

    let context = *world.resource::<LegContext>();
    let wheel = *world.resource::<WheelState>();
    let base_cadence = context.cadence_per_min;
    let effective_cadence = if wheel.slowmo_enabled {
        base_cadence.saturating_mul(SLOWMO_NUMERATOR) / SLOWMO_DENOMINATOR
    } else {
        base_cadence
    };

    let base_delta = world.resource::<Time<Fixed>>().timestep();
    let effective_delta = if wheel.slowmo_enabled {
        scale_duration(base_delta, SLOWMO_NUMERATOR, SLOWMO_DENOMINATOR)
    } else {
        base_delta
    };

    if wheel.slowmo_enabled {
        if let Some(mut queue) = world.get_resource_mut::<CommandQueue>() {
            let nanos = effective_delta.as_nanos().min(i32::MAX as u128) as i32;
            queue.meter("physics_fixed_dt_ns", nanos);
        }
    }

    if let Some(mut substeps) = world.get_resource_mut::<SubstepCount>() {
        let desired = if effective_cadence == 0 {
            1
        } else {
            ((effective_cadence - 1) / 60).saturating_add(1)
        };
        let target = desired.clamp(1, 12);
        if substeps.0 != target {
            substeps.0 = target;
        }
    }

    if wheel.slowmo_enabled {
        world
            .resource_mut::<Time<Fixed>>()
            .set_timestep(effective_delta);
    }

    world.resource_mut::<Time>().advance_by(effective_delta);

    world.run_schedule(DirectorPhysicsSchedule);

    if wheel.slowmo_enabled {
        world.resource_mut::<Time<Fixed>>().set_timestep(base_delta);
    }
}

fn finalize_leg(
    mut state: ResMut<DirectorState>,
    mut econ: ResMut<EconIntent>,
    mut queue: ResMut<CommandQueue>,
    mut context: ResMut<LegContext>,
    pause: Res<PauseState>,
) {
    if !matches!(state.status, LegStatus::Running | LegStatus::Paused) {
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
    let basis_delta = i32::from(econ.pending_basis_overlay_bp);
    let basis_total = context.basis_overlay_bp_total.saturating_add(basis_delta);
    let _ = m2::log_post_leg_summary(
        danger_delta,
        econ.pending_pp_delta,
        econ.pending_basis_overlay_bp,
        state.current_danger_score,
        basis_total,
    );
    state.prior_danger_score = state.current_danger_score;
    context.prior_danger_score = Some(state.current_danger_score);
    context.basis_overlay_bp_total = basis_total;
    const LEG_DURATION_TOLERANCE_TICKS: u32 = 60;
    let mission_minutes = context.mission_minutes;
    let target_tick = mission_minutes.saturating_mul(60);
    let clamp_tick = |queue: &mut CommandQueue,
                      mission_minutes: u32,
                      tolerance: u32,
                      target_tick: u32,
                      attempted_tick: u32|
     -> u32 {
        let max_tick = target_tick.saturating_add(tolerance);
        if attempted_tick > max_tick {
            let overflow = attempted_tick - max_tick;
            queue.meter("leg_tick_over_window", overflow as i32);
            let _ =
                m2::log_leg_duration_clamped(mission_minutes, tolerance, attempted_tick, max_tick);
            max_tick
        } else {
            attempted_tick
        }
    };

    state.leg_tick = clamp_tick(
        &mut *queue,
        mission_minutes,
        LEG_DURATION_TOLERANCE_TICKS,
        target_tick,
        state.leg_tick,
    );

    if state.leg_tick >= target_tick {
        state.status = LegStatus::Completed(Outcome::Success);
    }

    if !pause.hard_paused_sp && matches!(state.status, LegStatus::Running) {
        let next_tick = state.leg_tick.saturating_add(1);
        state.leg_tick = clamp_tick(
            &mut *queue,
            mission_minutes,
            LEG_DURATION_TOLERANCE_TICKS,
            target_tick,
            next_tick,
        );
    }
    econ.clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::IntoSystem;

    #[test]
    fn finalize_leg_accumulates_basis_overlay_total() {
        m2::set_enabled(false);

        let mut world = World::new();
        world.insert_resource(DirectorState {
            status: LegStatus::Running,
            current_danger_score: 120,
            prior_danger_score: 100,
            ..Default::default()
        });
        world.insert_resource(EconIntent {
            pending_pp_delta: 0,
            pending_basis_overlay_bp: 25,
        });
        let mut queue = CommandQueue::default();
        queue.begin_tick(0);
        world.insert_resource(queue);
        world.insert_resource(LegContext {
            basis_overlay_bp_total: 100,
            ..Default::default()
        });
        world.insert_resource(PauseState::default());

        let mut system = IntoSystem::into_system(finalize_leg);
        system.initialize(&mut world);
        let _ = system.run((), &mut world);
        system.apply_deferred(&mut world);

        {
            let context = world.resource::<LegContext>();
            assert_eq!(context.basis_overlay_bp_total, 125);
        }
        {
            let econ = world.resource::<EconIntent>();
            assert_eq!(econ.pending_basis_overlay_bp, 0);
        }

        {
            let mut econ = world.resource_mut::<EconIntent>();
            econ.pending_basis_overlay_bp = 10;
        }
        {
            let mut queue = world.resource_mut::<CommandQueue>();
            queue.begin_tick(1);
        }
        {
            let mut state = world.resource_mut::<DirectorState>();
            state.current_danger_score = 118;
        }

        let _ = system.run((), &mut world);
        system.apply_deferred(&mut world);

        let context = world.resource::<LegContext>();
        assert_eq!(context.basis_overlay_bp_total, 135);
        let queue = world.resource::<CommandQueue>();
        assert_eq!(queue.buf.len(), 2);
        assert!(queue
            .buf
            .iter()
            .any(|command| matches!(command.kind, repro::CommandKind::Meter(ref meter) if meter.value == 25)));
        assert!(queue
            .buf
            .iter()
            .any(|command| matches!(command.kind, repro::CommandKind::Meter(ref meter) if meter.value == 10)));
    }
}
