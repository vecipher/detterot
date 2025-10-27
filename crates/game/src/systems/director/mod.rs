use std::{
    collections::VecDeque,
    hash::Hasher,
    path::{Path, PathBuf},
};

use anyhow::Result as AnyResult;
use bevy::prelude::*;
use blake3::Hasher as Blake3Hasher;
use log::warn;
use repro::{DetRng, RecordMeta};
use wyhash::WyHash;

use crate::{logs, scheduling, systems::command_queue::CommandQueue};

pub mod config;
pub mod econ_intent;
pub mod missions;
pub mod pause_wheel;
pub mod spawn;

use crate::systems::economy::types::{Pp, RouteId, Weather};
use config::{DirectorCfg, MissionCfg};
use econ_intent::EconIntent;
use missions::{MissionKind, MissionResult};
use pause_wheel::{bool_to_i32, PauseState, WheelState};
use spawn::{compute_spawn_budget, weather_key, SpawnBudget};

const DIRECTOR_CFG_PATH: &str = "assets/director/m2.toml";

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

#[derive(Resource, Debug)]
pub struct DirectorState {
    pub leg_tick: u32,
    pub status: LegStatus,
    pub link_id: RouteId,
    pub weather: Weather,
    pub prior_danger_score: i32,
    pub danger_diff: i32,
}

impl Default for DirectorState {
    fn default() -> Self {
        Self {
            leg_tick: 0,
            status: LegStatus::Loading,
            link_id: RouteId::default(),
            weather: Weather::default(),
            prior_danger_score: 0,
            danger_diff: 0,
        }
    }
}

#[derive(Resource, Clone)]
pub struct LegParameters {
    pub world_seed: String,
    pub link_label: String,
    pub weather: Weather,
    pub rulepack: String,
    pub rng_salt: String,
    pub pp: Pp,
    pub prior_enemies: Option<u32>,
    pub prior_danger_score: Option<i32>,
    pub mission_minutes: u32,
    pub density_per_10k: u32,
    pub cadence_per_min: u32,
    pub player_rating: u8,
    pub mission_sequence: Vec<String>,
}

impl Default for LegParameters {
    fn default() -> Self {
        Self {
            world_seed: "m2_default_seed".to_string(),
            link_label: "route_default".to_string(),
            weather: Weather::Clear,
            rulepack: "assets/rulepacks/day_001.toml".to_string(),
            rng_salt: "default".to_string(),
            pp: Pp(120),
            prior_enemies: None,
            prior_danger_score: None,
            mission_minutes: 12,
            density_per_10k: 90,
            cadence_per_min: 6,
            player_rating: 50,
            mission_sequence: vec![
                "rain_flag".to_string(),
                "sourvault".to_string(),
                "break_chain".to_string(),
                "wayleave".to_string(),
                "anchor_audit".to_string(),
            ],
        }
    }
}

impl LegParameters {
    pub fn record_meta(&self) -> RecordMeta {
        RecordMeta {
            schema: 1,
            world_seed: self.world_seed.clone(),
            link_id: self.link_label.clone(),
            rulepack: self.rulepack.clone(),
            weather: weather_to_str(self.weather).to_string(),
            rng_salt: self.rng_salt.clone(),
        }
    }

    pub fn from_record_meta(meta: &RecordMeta) -> Self {
        let mut params = LegParameters::default();
        params.apply_record_meta(meta);
        params
    }

    pub fn apply_record_meta(&mut self, meta: &RecordMeta) {
        self.world_seed = meta.world_seed.clone();
        self.link_label = meta.link_id.clone();
        self.rulepack = meta.rulepack.clone();
        if let Some(weather) = weather_from_str(&meta.weather) {
            self.weather = weather;
        }
        self.rng_salt = meta.rng_salt.clone();
    }
}

#[derive(Resource, Default, Clone)]
pub struct LogSettings {
    pub enabled: bool,
}

#[derive(Resource, Default, Clone)]
struct DirectorRuntime {
    cfg: Option<DirectorCfg>,
    mission_names: Vec<String>,
    active_index: usize,
    active: Option<ActiveMission>,
    spawn_budget: Option<SpawnBudget>,
    prior_enemies: Option<u32>,
    spawn_emitted: bool,
    spawn_seed: u64,
    initialized: bool,
    post_logged: bool,
}

#[derive(Clone)]
struct ActiveMission {
    name: String,
    mission: MissionKind,
    cfg: MissionCfg,
}

#[derive(Resource, Default)]
struct WheelScript {
    events: VecDeque<WheelEvent>,
    seeded: bool,
    script_tick: u32,
}

struct WheelEvent {
    tick: u32,
    action: WheelAction,
}

enum WheelAction {
    EmitBaseline,
    ToggleTool,
    ToggleStance,
    ToggleOverwatch,
    ToggleMoveMode,
    ToggleSlowmo,
    Pause(bool),
}

pub struct DirectorPlugin;

impl Plugin for DirectorPlugin {
    fn build(&self, app: &mut App) {
        scheduling::configure_fixed_update(app);
        if !app.world_mut().contains_resource::<LegParameters>() {
            app.insert_resource(LegParameters::default());
        }
        app.init_resource::<DirectorState>();
        app.init_resource::<CommandQueue>();
        app.init_resource::<EconIntent>();
        app.init_resource::<WheelState>();
        app.init_resource::<PauseState>();
        app.init_resource::<DirectorRuntime>();
        app.init_resource::<WheelScript>();
        app.init_resource::<LogSettings>();

        use scheduling::sets;
        app.add_systems(
            FixedUpdate,
            (director_bootstrap, advance_leg_tick, drive_wheel_state)
                .chain()
                .in_set(sets::DETTEROT_Director),
        );
        app.add_systems(FixedUpdate, missions_tick.in_set(sets::DETTEROT_Missions));
        app.add_systems(
            FixedUpdate,
            emit_spawn_commands.in_set(sets::DETTEROT_Spawns),
        );
    }
}

pub fn danger_score(
    budget: &SpawnBudget,
    minutes: u32,
    density_per_10k: u32,
    cadence_per_min: u32,
    player_rating: u8,
) -> i32 {
    let base = 1000 * budget.enemies as i32
        + 400 * density_per_10k as i32
        + 300 * cadence_per_min as i32
        + 50 * minutes as i32;
    let rating = player_rating.clamp(0, 100) as i32;
    let numerator = 1000 + 4 * (rating - 50);
    let scaled = (base as i64) * (numerator as i64);
    ((scaled + 500) / 1000) as i32
}

pub fn danger_diff_sign(current: i32, prior: i32) -> i32 {
    match current.cmp(&prior) {
        std::cmp::Ordering::Greater => 1,
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
    }
}

pub fn apply_mission_result(
    queue: &mut CommandQueue,
    econ: &mut EconIntent,
    result: &MissionResult,
    cfg: &MissionCfg,
) {
    match result {
        MissionResult::Success {
            pp_delta,
            basis_bp_overlay,
        }
        | MissionResult::Fail {
            pp_delta,
            basis_bp_overlay,
        } => {
            econ.apply(*pp_delta, *basis_bp_overlay);
            queue.meter("pp_delta", *pp_delta as i32);
            queue.meter("basis_bp_overlay", *basis_bp_overlay as i32);
        }
    }
    let outcome_meter = match result {
        MissionResult::Success { .. } => 1,
        MissionResult::Fail { .. } => -1,
    };
    queue.meter("mission_outcome", outcome_meter);
    queue.meter("mission_pp_success", cfg.pp_success as i32);
    queue.meter("mission_pp_fail", cfg.pp_fail as i32);
}

fn director_bootstrap(
    mut runtime: ResMut<DirectorRuntime>,
    mut state: ResMut<DirectorState>,
    params: Res<LegParameters>,
    mut queue: ResMut<CommandQueue>,
    log_settings: Res<LogSettings>,
) {
    if runtime.initialized {
        return;
    }

    let cfg = match load_cfg(runtime.as_mut()) {
        Ok(cfg) => cfg.clone(),
        Err(err) => {
            warn!("failed to load director cfg: {err}");
            return;
        }
    };

    runtime.mission_names = mission_sequence(&cfg, &params);
    runtime.spawn_seed = spawn_seed(&params);
    runtime.spawn_budget = Some(compute_spawn_budget(
        params.pp,
        params.weather,
        params.prior_enemies,
        &cfg,
    ));
    runtime.prior_enemies = runtime.spawn_budget.map(|b| b.enemies);
    runtime.spawn_emitted = false;
    runtime.active_index = 0;
    runtime.active = prepare_mission(runtime.active_index, &runtime.mission_names, &cfg, &params);
    runtime.post_logged = false;
    runtime.initialized = true;

    state.status = LegStatus::Running;
    state.weather = params.weather;
    state.link_id = route_id_from_label(&params.link_label);

    let prior_score = params.prior_danger_score.unwrap_or(0);
    state.prior_danger_score = prior_score;

    if let Some(budget) = runtime.spawn_budget {
        let score = danger_score(
            &budget,
            params.mission_minutes,
            params.density_per_10k,
            params.cadence_per_min,
            params.player_rating,
        );
        let diff = danger_diff_sign(score, prior_score);
        queue.meter("danger_score", score);
        queue.meter("danger_diff", diff);
        state.danger_diff = diff;
        state.prior_danger_score = score;
        log_spawn_budget(&log_settings, state.leg_tick, &params, &budget);
    }
}

fn advance_leg_tick(mut state: ResMut<DirectorState>) {
    if matches!(state.status, LegStatus::Running) {
        state.leg_tick = state.leg_tick.saturating_add(1);
    }
}

fn missions_tick(
    mut runtime: ResMut<DirectorRuntime>,
    mut state: ResMut<DirectorState>,
    params: Res<LegParameters>,
    mut queue: ResMut<CommandQueue>,
    mut econ: ResMut<EconIntent>,
    log_settings: Res<LogSettings>,
) {
    if !matches!(state.status, LegStatus::Running) {
        return;
    }

    let mut resolved_outcome = None;
    if let Some(active) = runtime.active.as_mut() {
        if let Some(result) = active.mission.tick(1) {
            apply_mission_result(&mut queue, &mut econ, &result, &active.cfg);
            log_mission_result(&log_settings, active.name.as_str(), &result);
            resolved_outcome = Some(match result {
                MissionResult::Success { .. } => Outcome::Success,
                MissionResult::Fail { .. } => Outcome::Failure,
            });
        }
    }

    if let Some(outcome) = resolved_outcome {
        runtime.active_index += 1;
        if let Some(cfg) = runtime.cfg.as_ref() {
            runtime.active =
                prepare_mission(runtime.active_index, &runtime.mission_names, cfg, &params);
        } else {
            runtime.active = None;
        }
        if runtime.active.is_none() {
            state.status = LegStatus::Completed(outcome);
        }
    }

    if matches!(state.status, LegStatus::Completed(_)) && !runtime.post_logged {
        log_post_leg(&log_settings, &state, &econ);
        runtime.post_logged = true;
    }
}

fn emit_spawn_commands(
    mut runtime: ResMut<DirectorRuntime>,
    params: Res<LegParameters>,
    mut queue: ResMut<CommandQueue>,
) {
    if !runtime.initialized || runtime.spawn_emitted {
        return;
    }
    let Some(cfg) = runtime.cfg.as_ref() else {
        return;
    };
    let Some(budget) = runtime.spawn_budget else {
        return;
    };
    let base_seed = runtime.spawn_seed;
    for index in 0..budget.enemies {
        let spawn_seed = splitmix64(base_seed ^ index as u64);
        let mut rng = DetRng::from_seed(spawn_seed);
        let kind = select_spawn_kind(cfg, params.weather, &mut rng);
        let (x, y) = spawn_position(index);
        queue.spawn(&kind, x, y, 0);
    }
    runtime.spawn_emitted = true;
}

fn drive_wheel_state(
    mut script: ResMut<WheelScript>,
    mut wheel: ResMut<WheelState>,
    mut pause: ResMut<PauseState>,
    mut queue: ResMut<CommandQueue>,
    mut state: ResMut<DirectorState>,
) {
    if !matches!(state.status, LegStatus::Running | LegStatus::Paused) {
        return;
    }
    if !script.seeded {
        script.events = VecDeque::from(vec![
            WheelEvent {
                tick: 0,
                action: WheelAction::EmitBaseline,
            },
            WheelEvent {
                tick: 20,
                action: WheelAction::ToggleTool,
            },
            WheelEvent {
                tick: 40,
                action: WheelAction::ToggleStance,
            },
            WheelEvent {
                tick: 60,
                action: WheelAction::ToggleOverwatch,
            },
            WheelEvent {
                tick: 70,
                action: WheelAction::ToggleMoveMode,
            },
            WheelEvent {
                tick: 80,
                action: WheelAction::ToggleSlowmo,
            },
            WheelEvent {
                tick: 100,
                action: WheelAction::Pause(true),
            },
            WheelEvent {
                tick: 120,
                action: WheelAction::Pause(false),
            },
        ]);
        script.seeded = true;
    }

    if state.leg_tick > script.script_tick {
        script.script_tick = state.leg_tick;
    } else {
        script.script_tick = script.script_tick.saturating_add(1);
    }

    while let Some(event) = script.events.front() {
        if event.tick > script.script_tick {
            break;
        }
        let event = script.events.pop_front().unwrap();
        match event.action {
            WheelAction::EmitBaseline => wheel.emit_meters(&mut queue),
            WheelAction::ToggleTool => {
                wheel.tool = match wheel.tool {
                    pause_wheel::ToolSlot::A => pause_wheel::ToolSlot::B,
                    pause_wheel::ToolSlot::B => pause_wheel::ToolSlot::A,
                };
                wheel.emit_meters(&mut queue);
            }
            WheelAction::ToggleStance => {
                wheel.stance = match wheel.stance {
                    pause_wheel::Stance::Brace => pause_wheel::Stance::Vault,
                    pause_wheel::Stance::Vault => pause_wheel::Stance::Brace,
                };
                wheel.emit_meters(&mut queue);
            }
            WheelAction::ToggleOverwatch => {
                wheel.overwatch = !wheel.overwatch;
                wheel.emit_meters(&mut queue);
            }
            WheelAction::ToggleMoveMode => {
                wheel.move_mode = !wheel.move_mode;
                wheel.emit_meters(&mut queue);
            }
            WheelAction::ToggleSlowmo => {
                wheel.slowmo_enabled = !wheel.slowmo_enabled;
                wheel.emit_meters(&mut queue);
            }
            WheelAction::Pause(value) => {
                pause.hard_paused_sp = value;
                queue.meter("wheel_pause", bool_to_i32(pause.hard_paused_sp));
                state.status = if value {
                    LegStatus::Paused
                } else {
                    LegStatus::Running
                };
            }
        }
    }
}

fn prepare_mission(
    index: usize,
    mission_names: &[String],
    cfg: &DirectorCfg,
    params: &LegParameters,
) -> Option<ActiveMission> {
    let name = mission_names.get(index)?.clone();
    let mission_cfg = cfg.missions.get(&name)?.clone();
    let mut mission = MissionKind::from_name(&name)?;
    let seed = mission_seed(params, index, &name);
    mission.init(seed, &mission_cfg);
    Some(ActiveMission {
        name,
        mission,
        cfg: mission_cfg,
    })
}

fn load_cfg(runtime: &mut DirectorRuntime) -> AnyResult<&DirectorCfg> {
    if runtime.cfg.is_none() {
        let path = director_cfg_path();
        runtime.cfg = Some(config::load_director_cfg(&path)?);
    }
    Ok(runtime.cfg.as_ref().expect("cfg initialized"))
}

fn director_cfg_path() -> PathBuf {
    let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(DIRECTOR_CFG_PATH);
    if manifest_path.exists() {
        manifest_path
    } else {
        PathBuf::from(DIRECTOR_CFG_PATH)
    }
}

fn mission_sequence(cfg: &DirectorCfg, params: &LegParameters) -> Vec<String> {
    if !params.mission_sequence.is_empty() {
        return params.mission_sequence.clone();
    }
    let mut keys: Vec<_> = cfg.missions.keys().cloned().collect();
    keys.sort();
    keys
}

fn mission_seed(params: &LegParameters, index: usize, mission: &str) -> u64 {
    let mut hasher = WyHash::with_seed(0);
    hasher.write(params.world_seed.as_bytes());
    hasher.write(params.link_label.as_bytes());
    hasher.write(params.rng_salt.as_bytes());
    hasher.write(mission.as_bytes());
    hasher.write(&index.to_le_bytes());
    hasher.finish()
}

fn spawn_seed(params: &LegParameters) -> u64 {
    let mut hasher = WyHash::with_seed(0);
    hasher.write(params.world_seed.as_bytes());
    hasher.write(params.link_label.as_bytes());
    hasher.write(params.rng_salt.as_bytes());
    hasher.write(b"spawn_budget");
    hasher.finish()
}

fn select_spawn_kind(cfg: &DirectorCfg, weather: Weather, rng: &mut DetRng) -> String {
    let weights = gather_weights(cfg, weather);
    let total: u64 = weights.iter().map(|(_, weight)| *weight).sum();
    if total == 0 {
        return "bandit".to_string();
    }
    let roll = (rng.next_u32() as u64) % total;
    let mut cursor = 0u64;
    for (kind, weight) in weights {
        cursor = cursor.saturating_add(weight);
        if roll < cursor {
            return kind;
        }
    }
    "bandit".to_string()
}

fn gather_weights(cfg: &DirectorCfg, weather: Weather) -> Vec<(String, u64)> {
    if let Some(weather_types) = cfg.weather_types.as_ref() {
        if let Some(map) = weather_types.get(weather_key(weather)) {
            let mut entries: Vec<_> = map
                .iter()
                .map(|(k, v)| (k.clone(), weight_from_float(*v)))
                .collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            return entries;
        }
    }
    if let Some(global) = cfg.types.as_ref() {
        let mut entries: Vec<_> = global
            .iter()
            .map(|(k, v)| (k.clone(), weight_from_float(*v)))
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        return entries;
    }
    vec![("bandit".to_string(), 1)]
}

fn weight_from_float(value: f32) -> u64 {
    let bits = value.to_bits() as u64 & 0x7FFF_FFFF;
    let scaled = bits >> 7;
    if scaled == 0 {
        1
    } else {
        scaled
    }
}

fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn spawn_position(index: u32) -> (i32, i32) {
    let offset = index as i32;
    let x = (offset * 173) % 3200 - 1600;
    let y = (offset * 97) % 2800 - 1400;
    (x, y)
}

fn route_id_from_label(label: &str) -> RouteId {
    let mut hasher = Blake3Hasher::new();
    hasher.update(label.as_bytes());
    let digest = hasher.finalize();
    let bytes = &digest.as_bytes()[..2];
    RouteId(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn weather_to_str(weather: Weather) -> &'static str {
    match weather {
        Weather::Clear => "Clear",
        Weather::Rains => "Rains",
        Weather::Fog => "Fog",
        Weather::Windy => "Windy",
    }
}

pub fn weather_from_str(label: &str) -> Option<Weather> {
    match label {
        "Clear" => Some(Weather::Clear),
        "Rains" => Some(Weather::Rains),
        "Fog" => Some(Weather::Fog),
        "Windy" => Some(Weather::Windy),
        _ => None,
    }
}

fn log_spawn_budget(
    settings: &LogSettings,
    tick: u32,
    params: &LegParameters,
    budget: &SpawnBudget,
) {
    if !settings.enabled {
        return;
    }
    let entry = logs::m2::SpawnBudgetLog {
        tick,
        link_id: &params.link_label,
        pp: params.pp.0 as i32,
        weather: weather_to_str(params.weather),
        budget: logs::m2::Budget {
            enemies: budget.enemies,
            obstacles: budget.obstacles,
            note: None,
        },
    };
    if let Err(err) = logs::m2::write_spawn_budget(entry) {
        warn!("failed to write spawn budget log: {err}");
    }
}

fn log_mission_result(settings: &LogSettings, name: &str, result: &MissionResult) {
    if !settings.enabled {
        return;
    }
    let (outcome, pp, basis) = match result {
        MissionResult::Success {
            pp_delta,
            basis_bp_overlay,
        } => ("success", *pp_delta, *basis_bp_overlay),
        MissionResult::Fail {
            pp_delta,
            basis_bp_overlay,
        } => ("fail", *pp_delta, *basis_bp_overlay),
    };
    let payload = logs::m2::MissionResultLog {
        name,
        outcome,
        pp_delta: pp,
        basis_bp_overlay: basis,
    };
    if let Err(err) = logs::m2::write_mission_result(payload) {
        warn!("failed to write mission log: {err}");
    }
}

fn log_post_leg(settings: &LogSettings, state: &DirectorState, econ: &EconIntent) {
    if !settings.enabled {
        return;
    }
    let payload = logs::m2::PostLegSummaryLog {
        danger_delta: state.danger_diff,
        applied_pp_delta: econ.pending_pp_delta,
        applied_basis_overlay: econ.pending_basis_overlay_bp,
        di_bp_after: econ.pending_pp_delta,
        basis_bp_after: econ.pending_basis_overlay_bp,
    };
    if let Err(err) = logs::m2::write_post_leg_summary(payload) {
        warn!("failed to write post leg summary: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn danger_score_increases_with_enemies() {
        let budget_low = SpawnBudget {
            enemies: 10,
            obstacles: 0,
        };
        let budget_high = SpawnBudget {
            enemies: 20,
            obstacles: 0,
        };
        let low = danger_score(&budget_low, 10, 5, 3, 50);
        let high = danger_score(&budget_high, 10, 5, 3, 50);
        assert!(high > low);
    }

    #[test]
    fn danger_diff_matches_sign() {
        assert_eq!(danger_diff_sign(10, 5), 1);
        assert_eq!(danger_diff_sign(5, 10), -1);
        assert_eq!(danger_diff_sign(7, 7), 0);
    }

    #[test]
    fn pause_freezes_ticks_and_missions() {
        use bevy::time::{Fixed, Time, TimeUpdateStrategy};
        use bevy::MinimalPlugins;
        use std::time::Duration;

        let mut app = App::new();
        app.insert_resource(LegParameters::default());
        app.insert_resource(LogSettings { enabled: false });
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            0.033_333_333_3,
        )));
        app.insert_resource(Time::<Fixed>::from_seconds(0.033_333_333_3));
        app.add_plugins(MinimalPlugins);
        app.add_plugins(DirectorPlugin);

        let mut snapshot = None;

        for _ in 0..200 {
            app.update();
            let status = {
                let state = app.world().resource::<DirectorState>();
                state.status
            };
            if status == LegStatus::Paused {
                let (tick, elapsed) = {
                    let state = app.world().resource::<DirectorState>();
                    let runtime = app.world().resource::<DirectorRuntime>();
                    let elapsed = runtime
                        .active
                        .as_ref()
                        .map(|mission| mission.mission.debug_elapsed_ticks())
                        .unwrap_or(0);
                    (state.leg_tick, elapsed)
                };
                snapshot = Some((tick, elapsed));
                break;
            }
        }

        let (paused_tick, paused_elapsed) = snapshot.expect("pause event triggered");

        for _ in 0..5 {
            app.update();
            {
                let state = app.world().resource::<DirectorState>();
                assert_eq!(state.status, LegStatus::Paused);
                assert_eq!(state.leg_tick, paused_tick);
            }
            {
                let runtime = app.world().resource::<DirectorRuntime>();
                let elapsed = runtime
                    .active
                    .as_ref()
                    .map(|mission| mission.mission.debug_elapsed_ticks())
                    .unwrap_or(0);
                assert_eq!(elapsed, paused_elapsed);
            }
        }
    }
}
