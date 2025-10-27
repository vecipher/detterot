use std::convert::TryFrom;
use std::fs;
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use thiserror::Error;

use crate::systems::director::{DirectorInputs, DirectorState};
use crate::{
    build_headless_app, collect_commands, configure_fixed_dt, director_inputs_mut, request_new_leg,
};
use crate::{logs::m2, systems::economy};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Play,
    Record,
    Replay,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub mode: Mode,
    pub io: Option<PathBuf>,
    pub fixed_dt: Option<f64>,
    pub headless: bool,
    pub continue_after_mismatch: bool,
    pub debug_logs: bool,
    pub world_seed: Option<u64>,
    pub link_id: Option<u16>,
    pub weather: Option<String>,
    pub pp: Option<u16>,
    pub day: Option<u32>,
    pub mission_minutes: Option<u32>,
    pub density_per_10k: Option<u32>,
    pub cadence_per_min: Option<u32>,
    pub player_rating: Option<u8>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            mode: Mode::Play,
            io: None,
            fixed_dt: None,
            headless: false,
            continue_after_mismatch: true,
            debug_logs: false,
            world_seed: None,
            link_id: None,
            weather: None,
            pp: None,
            day: None,
            mission_minutes: None,
            density_per_10k: None,
            cadence_per_min: None,
            player_rating: None,
        }
    }
}

#[derive(Debug, Error)]
pub enum CliError {
    #[error("unknown argument: {0}")]
    UnknownArgument(String),
    #[error("missing value for argument: {0}")]
    MissingValue(String),
    #[error("invalid value for {0}: {1}")]
    InvalidValue(&'static str, String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("record error: {0}")]
    Record(#[from] repro::Error),
    #[error("replay mismatch at index {index}")]
    ReplayMismatch {
        index: usize,
        expected: String,
        actual: String,
    },
    #[error("missing --io path for record/replay mode")]
    MissingIo,
}

pub fn parse_args<I, S>(args: I) -> Result<Options, CliError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut iter = args.into_iter().map(Into::into).peekable();
    let mut options = Options::default();
    let _program = iter.next();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--mode" => {
                let value = next_value(&mut iter, "--mode")?;
                options.mode = match value.as_str() {
                    "play" => Mode::Play,
                    "record" => Mode::Record,
                    "replay" => Mode::Replay,
                    other => return Err(CliError::InvalidValue("--mode", other.to_string())),
                };
            }
            "--io" => {
                let value = next_value(&mut iter, "--io")?;
                options.io = Some(PathBuf::from(value));
            }
            "--fixed-dt" => {
                let value = next_value(&mut iter, "--fixed-dt")?;
                let parsed: f64 = parse_number("--fixed-dt", &value)?;
                options.fixed_dt = Some(parsed);
            }
            "--headless" => {
                options.headless = true;
            }
            "--continue-after-mismatch" => {
                if let Some(peek) = iter.peek() {
                    if peek == "true" || peek == "false" || peek == "1" || peek == "0" {
                        let value = iter.next().expect("peeked Some");
                        options.continue_after_mismatch =
                            parse_bool("--continue-after-mismatch", &value)?;
                    } else {
                        options.continue_after_mismatch = true;
                    }
                } else {
                    options.continue_after_mismatch = true;
                }
            }
            "--stop-on-mismatch" => {
                options.continue_after_mismatch = false;
            }
            "--debug-logs" => {
                options.debug_logs = true;
            }
            "--world-seed" => {
                let value = next_value(&mut iter, "--world-seed")?;
                options.world_seed = Some(parse_number("--world-seed", &value)?);
            }
            "--link-id" => {
                let value = next_value(&mut iter, "--link-id")?;
                options.link_id = Some(parse_number("--link-id", &value)?);
            }
            "--weather" => {
                let value = next_value(&mut iter, "--weather")?;
                options.weather = Some(value);
            }
            "--pp" => {
                let value = next_value(&mut iter, "--pp")?;
                options.pp = Some(parse_number("--pp", &value)?);
            }
            "--day" => {
                let value = next_value(&mut iter, "--day")?;
                options.day = Some(parse_number("--day", &value)?);
            }
            "--mission-minutes" => {
                let value = next_value(&mut iter, "--mission-minutes")?;
                options.mission_minutes = Some(parse_number("--mission-minutes", &value)?);
            }
            "--density-per-10k" => {
                let value = next_value(&mut iter, "--density-per-10k")?;
                options.density_per_10k = Some(parse_number("--density-per-10k", &value)?);
            }
            "--cadence-per-min" => {
                let value = next_value(&mut iter, "--cadence-per-min")?;
                options.cadence_per_min = Some(parse_number("--cadence-per-min", &value)?);
            }
            "--player-rating" => {
                let value = next_value(&mut iter, "--player-rating")?;
                options.player_rating = Some(parse_number("--player-rating", &value)?);
            }
            other => return Err(CliError::UnknownArgument(other.to_string())),
        }
    }
    Ok(options)
}

pub fn run(options: Options) -> Result<(), CliError> {
    match options.mode {
        Mode::Play => run_play(options),
        Mode::Record => run_record(options),
        Mode::Replay => run_replay(options),
    }
}

fn run_play(options: Options) -> Result<(), CliError> {
    if options.debug_logs {
        m2::enable_debug_logs();
    }
    let mut app = build_headless_app();
    apply_options(&mut app, &options)?;
    collect_commands(&mut app, 5);
    Ok(())
}

fn run_record(options: Options) -> Result<(), CliError> {
    let path = options.io.clone().ok_or(CliError::MissingIo)?;
    if options.debug_logs {
        m2::enable_debug_logs();
    }
    let mut app = build_headless_app();
    apply_options(&mut app, &options)?;
    let commands = collect_commands(&mut app, 240);
    let record = build_record(&mut app, commands);
    record.write_to_path(&path)?;
    let hash = record.hash()?;
    let hash_path = hash_path(&path);
    fs::write(hash_path, format!("{}\n", hash))?;
    Ok(())
}

fn run_replay(options: Options) -> Result<(), CliError> {
    let path = options.io.clone().ok_or(CliError::MissingIo)?;
    let record = repro::Record::read_from_path(&path)?;
    if let Ok(expected_hash) = fs::read_to_string(hash_path(&path)) {
        let trimmed = expected_hash.trim();
        let actual = record.hash()?;
        if trimmed != actual {
            return Err(CliError::InvalidValue(
                "hash",
                format!("expected {trimmed}, got {actual}"),
            ));
        }
    }

    let mut app = build_headless_app();
    if options.debug_logs {
        m2::enable_debug_logs();
    }
    apply_meta(&mut app, &record.meta)?;
    apply_options(&mut app, &options)?;
    let expected_commands = record.commands.clone();
    let max_tick = expected_commands.iter().map(|cmd| cmd.t).max().unwrap_or(0);
    let commands = collect_commands(&mut app, max_tick + 5);
    compare_commands(commands, expected_commands, options.continue_after_mismatch)
}

fn apply_options(app: &mut App, options: &Options) -> Result<(), CliError> {
    if let Some(dt) = options.fixed_dt {
        configure_fixed_dt(app, dt);
    }
    let mut queue_leg = false;
    {
        let mut inputs = director_inputs_mut(app);
        if let Some(seed) = options.world_seed {
            if inputs.world_seed != seed {
                inputs.world_seed = seed;
                queue_leg = true;
            }
        }
        if let Some(link) = options.link_id {
            let route = economy::RouteId(link);
            if inputs.link_id != route {
                inputs.link_id = route;
                queue_leg = true;
            }
        }
        if let Some(pp) = options.pp {
            if inputs.pp != economy::Pp(pp) {
                inputs.pp = economy::Pp(pp);
                queue_leg = true;
            }
        }
        if let Some(day) = options.day {
            if inputs.day != day {
                inputs.day = day;
                queue_leg = true;
            }
        }
        if let Some(minutes) = options.mission_minutes {
            if inputs.mission_minutes != minutes {
                inputs.mission_minutes = minutes;
                queue_leg = true;
            }
        }
        if let Some(density) = options.density_per_10k {
            if inputs.density_per_10k != density {
                inputs.density_per_10k = density;
                queue_leg = true;
            }
        }
        if let Some(cadence) = options.cadence_per_min {
            if inputs.cadence_per_min != cadence {
                inputs.cadence_per_min = cadence;
                queue_leg = true;
            }
        }
        if let Some(rating) = options.player_rating {
            if inputs.player_rating != rating {
                inputs.player_rating = rating;
                queue_leg = true;
            }
        }
    }
    {
        let mut state = app.world_mut().resource_mut::<DirectorState>();
        if let Some(link) = options.link_id {
            let route = economy::RouteId(link);
            if state.link_id != route {
                state.link_id = route;
                queue_leg = true;
            }
        }
        if let Some(weather) = options.weather.as_deref() {
            let parsed = parse_weather(weather)?;
            if state.weather != parsed {
                state.weather = parsed;
                queue_leg = true;
            }
        }
    }
    if queue_leg {
        request_new_leg(app);
    }
    Ok(())
}

fn apply_meta(app: &mut App, meta: &repro::RecordMeta) -> Result<(), CliError> {
    let mut options = Options::default();
    options.world_seed = Some(
        meta.world_seed
            .parse()
            .map_err(|_| CliError::InvalidValue("meta.world_seed", meta.world_seed.clone()))?,
    );
    options.link_id = Some(
        meta.link_id
            .parse()
            .map_err(|_| CliError::InvalidValue("meta.link_id", meta.link_id.clone()))?,
    );
    options.weather = Some(meta.weather.clone());
    let pp = u16::try_from(meta.pp)
        .map_err(|_| CliError::InvalidValue("meta.pp", meta.pp.to_string()))?;
    options.pp = Some(pp);
    options.day = Some(meta.day);
    options.mission_minutes = Some(meta.mission_minutes);
    options.density_per_10k = Some(meta.density_per_10k);
    options.cadence_per_min = Some(meta.cadence_per_min);
    options.player_rating = Some(meta.player_rating);
    apply_options(app, &options)?;
    let salt_str = meta.rng_salt.trim_start_matches("0x");
    let rng_salt = u64::from_str_radix(salt_str, 16)
        .map_err(|_| CliError::InvalidValue("meta.rng_salt", meta.rng_salt.clone()))?;
    app.world_mut().resource_mut::<DirectorState>().rng_salt = rng_salt;
    Ok(())
}

fn build_record(app: &mut App, commands: Vec<repro::Command>) -> repro::Record {
    let state = app.world().resource::<DirectorState>().clone();
    let inputs = app.world().resource::<DirectorInputs>().clone();
    let meta = repro::RecordMeta {
        schema: 1,
        world_seed: inputs.world_seed.to_string(),
        link_id: state.link_id.0.to_string(),
        rulepack: "assets/rulepacks/day_001.toml".into(),
        weather: format!("{:?}", state.weather),
        rng_salt: format!("{:016x}", state.rng_salt),
        pp: u32::from(inputs.pp.0),
        mission_minutes: inputs.mission_minutes,
        density_per_10k: inputs.density_per_10k,
        cadence_per_min: inputs.cadence_per_min,
        player_rating: inputs.player_rating,
        day: inputs.day,
    };
    let mut record = repro::Record::new(meta);
    for command in commands {
        record.add_command(command);
    }
    record
}

fn compare_commands(
    actual: Vec<repro::Command>,
    expected: Vec<repro::Command>,
    continue_after_mismatch: bool,
) -> Result<(), CliError> {
    let mut mismatch = None;
    for (idx, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
        if a != e {
            mismatch = Some((idx, a.clone(), e.clone()));
            if !continue_after_mismatch {
                break;
            }
        }
    }
    if mismatch.is_none() && actual.len() != expected.len() {
        mismatch = Some((
            expected.len(),
            actual
                .get(expected.len())
                .cloned()
                .unwrap_or_else(|| repro::Command {
                    t: 0,
                    kind: repro::CommandKind::Meter {
                        key: "missing".into(),
                        value: 0,
                    },
                }),
            expected
                .get(expected.len())
                .cloned()
                .unwrap_or_else(|| repro::Command {
                    t: 0,
                    kind: repro::CommandKind::Meter {
                        key: "missing".into(),
                        value: 0,
                    },
                }),
        ));
    }
    if let Some((idx, actual_cmd, expected_cmd)) = mismatch {
        m2::log_replay_mismatch(idx, &expected_cmd, &actual_cmd);
        if continue_after_mismatch {
            eprintln!(
                "Replay mismatch at {idx}: expected {:?}, got {:?}",
                expected_cmd, actual_cmd
            );
            Ok(())
        } else {
            Err(CliError::ReplayMismatch {
                index: idx,
                expected: format!("{:?}", expected_cmd),
                actual: format!("{:?}", actual_cmd),
            })
        }
    } else {
        Ok(())
    }
}

fn next_value(
    iter: &mut std::iter::Peekable<impl Iterator<Item = String>>,
    flag: &'static str,
) -> Result<String, CliError> {
    iter.next()
        .ok_or_else(|| CliError::MissingValue(flag.to_string()))
}

fn parse_number<T>(flag: &'static str, value: &str) -> Result<T, CliError>
where
    T: std::str::FromStr,
{
    value
        .parse()
        .map_err(|_| CliError::InvalidValue(flag, value.to_string()))
}

fn parse_bool(flag: &'static str, value: &str) -> Result<bool, CliError> {
    match value {
        "true" | "1" => Ok(true),
        "false" | "0" => Ok(false),
        other => Err(CliError::InvalidValue(flag, other.to_string())),
    }
}

fn hash_path(path: &Path) -> PathBuf {
    let mut hash_path = path.to_path_buf();
    hash_path.set_extension("hash");
    hash_path
}

fn parse_weather(input: &str) -> Result<economy::Weather, CliError> {
    match input {
        "Clear" | "clear" => Ok(economy::Weather::Clear),
        "Rains" | "rains" => Ok(economy::Weather::Rains),
        "Fog" | "fog" => Ok(economy::Weather::Fog),
        "Windy" | "windy" => Ok(economy::Weather::Windy),
        other => Err(CliError::InvalidValue("--weather", other.to_string())),
    }
}
