use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use bevy::app::App;
use bevy::prelude::*;
use bevy::time::{Fixed, Time, TimeUpdateStrategy};
use bevy::MinimalPlugins;
use repro::{Command, Record, TimedCommand};
use serde::Deserialize;

use crate::logs;
use crate::systems::command_queue::CommandQueue;
use crate::systems::director::{
    DirectorPlugin, DirectorState, LegParameters, LegStatus, LogSettings,
};
use crate::systems::economy::types::{Pp, Weather};

const DEFAULT_FIXED_DT: f64 = 0.033_333_333_3;
const DEFAULT_MAX_TICKS: u32 = 200_000;
const MANIFEST_PATH: &str = "repro/records/manifest.toml";

#[derive(Debug, Clone, Copy)]
pub struct HeadlessConfig {
    pub dt: f64,
    pub logs_enabled: bool,
    pub max_ticks: u32,
}

impl Default for HeadlessConfig {
    fn default() -> Self {
        Self {
            dt: DEFAULT_FIXED_DT,
            logs_enabled: false,
            max_ticks: DEFAULT_MAX_TICKS,
        }
    }
}

pub fn record_leg(params: &LegParameters, cfg: &HeadlessConfig) -> Result<Record> {
    let mut app = build_headless_app(params, cfg);
    let mut record = Record::new(params.record_meta());
    run_headless_loop(&mut app, cfg.max_ticks, |tick, commands| {
        for command in commands {
            record.push_command(tick, command);
        }
        Ok(())
    })?;
    Ok(record)
}

pub fn replay_leg(
    params: &LegParameters,
    cfg: &HeadlessConfig,
    record: &Record,
    continue_after_mismatch: bool,
) -> Result<()> {
    let mut app = build_headless_app(params, cfg);
    let mut session = ReplaySession::replay(record, continue_after_mismatch, cfg.logs_enabled);
    run_headless_loop(&mut app, cfg.max_ticks, |tick, commands| {
        session.push_commands(tick, commands)
    })?;
    session.finalize()
}

fn build_headless_app(params: &LegParameters, cfg: &HeadlessConfig) -> App {
    let mut app = App::new();
    app.insert_resource(params.clone());
    app.insert_resource(LogSettings {
        enabled: cfg.logs_enabled || cfg!(feature = "m2_logs"),
    });
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        cfg.dt,
    )));
    app.insert_resource(Time::<Fixed>::from_seconds(cfg.dt));

    #[cfg(feature = "deterministic")]
    {
        use bevy::app::TaskPoolPlugin;
        use bevy::prelude::TaskPoolOptions;

        app.add_plugins(MinimalPlugins.set(TaskPoolPlugin {
            task_pool_options: TaskPoolOptions::with_num_threads(1),
        }));
    }
    #[cfg(not(feature = "deterministic"))]
    {
        app.add_plugins(MinimalPlugins);
    }
    app.add_plugins(DirectorPlugin);
    app
}

fn run_headless_loop<'a>(
    app: &mut App,
    max_ticks: u32,
    mut handler: impl FnMut(u32, Vec<Command>) -> Result<()> + 'a,
) -> Result<()> {
    let mut ticks = 0u32;
    loop {
        app.update();
        ticks = ticks.saturating_add(1);
        let (tick, status) = {
            let world = app.world();
            let state = world.resource::<DirectorState>();
            (state.leg_tick, state.status)
        };
        let commands = {
            let world = app.world_mut();
            let mut queue = world.resource_mut::<CommandQueue>();
            queue.drain().collect::<Vec<_>>()
        };
        handler(tick, commands)?;
        if matches!(status, LegStatus::Completed(_)) {
            break;
        }
        if ticks > max_ticks {
            return Err(anyhow!("headless simulation exceeded {max_ticks} ticks"));
        }
    }
    Ok(())
}

struct ReplaySession {
    expected: VecDeque<(u32, Vec<Command>)>,
    inflight: Option<InFlight>,
    continue_after_mismatch: bool,
    mismatched: bool,
    logs_enabled: bool,
}

struct InFlight {
    tick: u32,
    commands: Vec<Command>,
    consumed: usize,
}

impl ReplaySession {
    fn replay(record: &Record, continue_after_mismatch: bool, logs_enabled: bool) -> Self {
        let mut grouped: BTreeMap<u32, Vec<Command>> = BTreeMap::new();
        for TimedCommand { tick, command } in &record.commands {
            grouped.entry(*tick).or_default().push(command.clone());
        }
        Self {
            expected: grouped.into_iter().collect(),
            inflight: None,
            continue_after_mismatch,
            mismatched: false,
            logs_enabled,
        }
    }

    fn push_commands(&mut self, tick: u32, commands: Vec<Command>) -> Result<()> {
        if let Some((mismatch_tick, expected_cmds, got_cmds)) =
            self.compute_mismatch(tick, &commands)
        {
            self.handle_mismatch(mismatch_tick, &expected_cmds, &got_cmds)?;
        }
        Ok(())
    }

    fn compute_mismatch(
        &mut self,
        tick: u32,
        commands: &[Command],
    ) -> Option<(u32, Vec<Command>, Vec<Command>)> {
        if let Some(mut current) = self.inflight.take() {
            if tick > current.tick && current.consumed < current.commands.len() {
                return Some((
                    current.tick,
                    current.commands[current.consumed..].to_vec(),
                    Vec::new(),
                ));
            }

            if tick < current.tick {
                if commands.is_empty() {
                    self.inflight = Some(current);
                    return None;
                }
                return Some((tick, Vec::new(), commands.to_vec()));
            }

            if tick == current.tick {
                let remaining = current.commands.len().saturating_sub(current.consumed);
                if commands.len() > remaining {
                    return Some((
                        current.tick,
                        current.commands[current.consumed..].to_vec(),
                        commands.to_vec(),
                    ));
                }
                let expected_slice =
                    &current.commands[current.consumed..current.consumed + commands.len()];
                if expected_slice != commands {
                    return Some((current.tick, current.commands.clone(), commands.to_vec()));
                }
                current.consumed += commands.len();
                if current.consumed < current.commands.len() {
                    self.inflight = Some(current);
                }
                return None;
            }

            // tick > current.tick but nothing left to consume
        }

        if let Some((expected_tick, _)) = self.expected.front() {
            if *expected_tick < tick {
                let (late_tick, exp) = self.expected.pop_front().unwrap();
                return Some((late_tick, exp, Vec::new()));
            }
            if *expected_tick == tick {
                let (tick, cmds) = self.expected.pop_front().unwrap();
                let mut current = InFlight {
                    tick,
                    commands: cmds,
                    consumed: 0,
                };
                if commands.len() > current.commands.len() {
                    return Some((current.tick, current.commands, commands.to_vec()));
                }
                let expected_slice = &current.commands[..commands.len()];
                if expected_slice != commands {
                    return Some((current.tick, current.commands, commands.to_vec()));
                }
                current.consumed = commands.len();
                if current.consumed < current.commands.len() {
                    self.inflight = Some(current);
                }
                return None;
            }
        }

        if commands.is_empty() {
            None
        } else {
            Some((tick, Vec::new(), commands.to_vec()))
        }
    }

    fn handle_mismatch(&mut self, tick: u32, expected: &[Command], got: &[Command]) -> Result<()> {
        self.mismatched = true;
        if self.logs_enabled {
            let expected_cmd = expected.first().cloned();
            let got_cmd = got.first().cloned();
            if let Err(err) = logs::m2::write_replay_mismatch(logs::m2::ReplayMismatchLog {
                tick,
                expected: expected_cmd,
                got: got_cmd,
            }) {
                eprintln!("failed to log replay mismatch: {err}");
            }
        }
        eprintln!(
            "replay mismatch at tick {tick}: expected {:?}, got {:?}",
            expected, got
        );
        if self.continue_after_mismatch {
            Ok(())
        } else {
            Err(anyhow!("replay mismatch at tick {tick}"))
        }
    }

    fn finalize(&mut self) -> Result<()> {
        if let Some(current) = self.inflight.take() {
            if current.consumed < current.commands.len() {
                let expected_remaining = current.commands[current.consumed..].to_vec();
                self.handle_mismatch(current.tick, &expected_remaining, &[])?;
            }
        }
        let leftovers: Vec<_> = self.expected.drain(..).collect();
        for (tick, cmds) in leftovers {
            self.handle_mismatch(tick, &cmds, &[])?;
        }
        if self.mismatched && !self.continue_after_mismatch {
            Err(anyhow!("replay mismatches encountered"))
        } else {
            Ok(())
        }
    }
}

pub mod goldens {
    use super::*;

    #[derive(Debug, Clone, Deserialize)]
    pub struct GoldenCase {
        pub file: String,
        pub world_seed: Option<String>,
        pub link_id: Option<String>,
        pub weather: Option<String>,
        pub rng_salt: Option<String>,
        pub rulepack: Option<String>,
        pub pp: Option<u16>,
        pub prior_enemies: Option<u32>,
        pub prior_danger_score: Option<i32>,
        pub mission_minutes: Option<u32>,
        pub density_per_10k: Option<u32>,
        pub cadence_per_min: Option<u32>,
        pub player_rating: Option<u8>,
        pub mission_sequence: Option<Vec<String>>,
        pub fixed_dt: Option<f64>,
        pub max_ticks: Option<u32>,
    }

    #[derive(Debug, Deserialize)]
    struct Manifest {
        cases: Vec<GoldenCase>,
    }

    pub fn load_manifest() -> Result<Vec<GoldenCase>> {
        let path = manifest_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let display = path.display().to_string();
        let data = fs::read_to_string(&path)
            .with_context(|| format!("unable to read manifest at {}", display))?;
        let manifest: Manifest =
            toml::from_str(&data).with_context(|| format!("invalid manifest at {}", display))?;
        Ok(manifest.cases)
    }

    pub fn case_for_path<'a>(cases: &'a [GoldenCase], path: &Path) -> Option<&'a GoldenCase> {
        let file = path.file_name()?.to_string_lossy();
        cases.iter().find(|case| case.file == file)
    }

    pub fn apply_case(params: &mut LegParameters, case: &GoldenCase) -> Result<()> {
        if let Some(world_seed) = &case.world_seed {
            params.world_seed = world_seed.clone();
        }
        if let Some(link_id) = &case.link_id {
            params.link_label = link_id.clone();
        }
        if let Some(rng_salt) = &case.rng_salt {
            params.rng_salt = rng_salt.clone();
        }
        if let Some(rulepack) = &case.rulepack {
            params.rulepack = rulepack.clone();
        }
        if let Some(weather) = &case.weather {
            params.weather = parse_weather(weather)?;
        }
        if let Some(pp) = case.pp {
            params.pp = Pp(pp);
        }
        params.prior_enemies = case.prior_enemies;
        params.prior_danger_score = case.prior_danger_score;
        if let Some(minutes) = case.mission_minutes {
            params.mission_minutes = minutes;
        }
        if let Some(density) = case.density_per_10k {
            params.density_per_10k = density;
        }
        if let Some(cadence) = case.cadence_per_min {
            params.cadence_per_min = cadence;
        }
        if let Some(rating) = case.player_rating {
            params.player_rating = rating;
        }
        if let Some(sequence) = &case.mission_sequence {
            params.mission_sequence = sequence.clone();
        }
        Ok(())
    }

    fn parse_weather(label: &str) -> Result<Weather> {
        match label {
            "Clear" => Ok(Weather::Clear),
            "Rains" => Ok(Weather::Rains),
            "Fog" => Ok(Weather::Fog),
            "Windy" => Ok(Weather::Windy),
            other => Err(anyhow!("unknown weather '{other}' in manifest")),
        }
    }
}

pub fn apply_manifest_for_path(path: &Path, params: &mut LegParameters) -> Result<HeadlessConfig> {
    let cases = goldens::load_manifest()?;
    if let Some(case) = goldens::case_for_path(&cases, path) {
        goldens::apply_case(params, case)?;
        let dt = case.fixed_dt.unwrap_or(DEFAULT_FIXED_DT);
        let max_ticks = case.max_ticks.unwrap_or(DEFAULT_MAX_TICKS);
        Ok(HeadlessConfig {
            dt,
            logs_enabled: false,
            max_ticks,
        })
    } else {
        Ok(HeadlessConfig::default())
    }
}

pub fn default_headless_config() -> HeadlessConfig {
    HeadlessConfig::default()
}

fn manifest_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(MANIFEST_PATH)
}
