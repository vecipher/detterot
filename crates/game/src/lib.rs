pub mod cli;
pub mod logs;
pub mod scheduling;
pub mod systems;
pub mod ui;

use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use bevy::app::App;
use bevy::log::{tracing, LogPlugin};
use bevy::prelude::*;
use bevy::time::{Fixed, Time as BevyTime};
use repro::{
    canonical_json_bytes, from_canonical_json_bytes, hash_record, Command, Record, RecordMeta,
};

use crate::logs::m2;
use cli::{CliOptions, Mode};
use std::sync::Once;
use systems::command_queue::CommandQueue;
#[cfg(feature = "deterministic")]
use systems::director::director_cfg_path;
use systems::director::{DirectorPlugin, DirectorState, LegContext, WheelState};
use systems::economy::{Pp, RouteId, Weather};
use systems::trading::TradingPlugin;

pub fn run() -> Result<()> {
    let options = CliOptions::parse();
    run_with_options(options)
}

pub fn run_with_options(options: CliOptions) -> Result<()> {
    init_logging();
    log_determinism_banner();
    m2::set_enabled(options.debug_logs || cfg!(feature = "m2_logs"));
    match options.mode() {
        Mode::Play => run_play(options),
        Mode::Record => run_record(options),
        Mode::Replay => run_replay(options),
    }
}

fn run_play(options: CliOptions) -> Result<()> {
    let context = leg_context_from_options(&options);
    let (_commands, _state) = simulate_ticks(&options, simulation_ticks(), context)?;
    let _ = _commands;
    let _ = _state;
    Ok(())
}

fn run_record(options: CliOptions) -> Result<()> {
    let path = options
        .io
        .as_ref()
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("--io path required for record mode"))?;
    let context = leg_context_from_options(&options);
    let (commands, state) = simulate_ticks(&options, simulation_ticks(), context)?;

    let record = Record {
        meta: RecordMeta {
            schema: 1,
            world_seed: format!("0x{:016X}", state.world_seed),
            link_id: format!("{}", state.link_id.0),
            rulepack: "assets/rulepacks/day_001.toml".into(),
            weather: format!("{:?}", state.weather),
            rng_salt: format!(
                "0x{:016X}",
                state.world_seed
                    ^ ((state.day as u64) << 32)
                    ^ (state.prior_danger_score as i64 as u64)
            ),
            day: state.day,
            pp: context.pp.0,
            density_per_10k: context.density_per_10k,
            cadence_per_min: context.cadence_per_min,
            mission_minutes: context.mission_minutes,
            player_rating: context.player_rating,
            prior_danger_score: context.prior_danger_score,
        },
        commands,
        inputs: Vec::new(),
    };

    let bytes = canonical_json_bytes(&record)?;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
    }
    fs::write(&path, &bytes).with_context(|| format!("writing record {}", path.display()))?;

    let hash = hash_record(&record)?;
    let mut hash_path = path.clone();
    hash_path.set_extension("hash");
    fs::write(&hash_path, format!("{}\n", hash))
        .with_context(|| format!("writing record hash {}", hash_path.display()))?;
    Ok(())
}

fn run_replay(options: CliOptions) -> Result<()> {
    let path = options
        .io
        .as_ref()
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("--io path required for replay mode"))?;
    let bytes = fs::read(&path).with_context(|| format!("reading record {}", path.display()))?;
    let record: Record = from_canonical_json_bytes(&bytes)
        .with_context(|| format!("parsing record {}", path.display()))?;

    let context = leg_context_from_record(&record.meta, &options)?;
    let (commands, _) = simulate_ticks(&options, simulation_ticks(), context)?;

    let continue_after = options.continue_after_mismatch;
    let expected_len = record.commands.len();
    let actual_len = commands.len();
    let shared_len = expected_len.min(actual_len);

    for (idx, (expected, actual)) in record.commands.iter().zip(&commands).enumerate() {
        if expected != actual {
            let _ = m2::log_replay_mismatch(idx as u32, Some(expected), Some(actual));
            if !continue_after {
                return Err(anyhow!(
                    "replay mismatch at command {idx}: expected {:?}, got {:?}",
                    expected,
                    actual
                ));
            }
        }
    }

    if expected_len != actual_len {
        let _ = m2::log_replay_mismatch(
            shared_len as u32,
            record.commands.get(shared_len),
            commands.get(shared_len),
        );
        if !continue_after {
            return Err(anyhow!(
                "replay length mismatch: expected_len={expected_len}, actual_len={actual_len}"
            ));
        }
    }

    Ok(())
}

fn simulate_ticks(
    options: &CliOptions,
    ticks: u32,
    context: LegContext,
) -> Result<(Vec<Command>, DirectorState)> {
    let mut app = build_app(options, context);
    app.finish();
    app.update();
    app.world_mut()
        .resource_scope(|world, mut queue: Mut<CommandQueue>| {
            let allow_slowmo = world
                .get_resource::<LegContext>()
                .map(|ctx| !ctx.multiplayer)
                .unwrap_or(true);
            if allow_slowmo {
                queue.begin_tick(0);
                world
                    .resource_mut::<WheelState>()
                    .set_slowmo(&mut queue, true);
                world
                    .resource_mut::<WheelState>()
                    .set_slowmo(&mut queue, false);
            }
        });
    let mut commands = Vec::new();
    for _ in 0..ticks {
        let current_tick = {
            let world = app.world();
            world.resource::<DirectorState>().leg_tick
        };
        {
            let world = app.world_mut();
            {
                let mut queue = world.resource_mut::<CommandQueue>();
                queue.begin_tick(current_tick);
            }
            world.run_schedule(FixedUpdate);
        }
        let mut queue = app.world_mut().resource_mut::<CommandQueue>();
        commands.extend(queue.drain());
    }
    let state = app.world().resource::<DirectorState>().clone();
    Ok((commands, state))
}

fn build_app(options: &CliOptions, context: LegContext) -> App {
    let mut app = App::new();
    add_core_plugins(&mut app, options);
    scheduling::configure(&mut app);
    {
        let dt = options.effective_fixed_dt();
        let mut fixed = app.world_mut().resource_mut::<BevyTime<Fixed>>();
        *fixed = BevyTime::<Fixed>::from_seconds(dt);
    }
    app.init_resource::<CommandQueue>();
    app.insert_resource(context);
    ensure_trading_plugin(&mut app);
    app.add_plugins(DirectorPlugin);
    app
}

/// Adds the core plugin groups for the simulation, taking the headless flag into account.
///
/// Headless runs stick to the minimal Bevy set so that no windowing or audio backends are
/// initialised in environments without a display. Whenever we add plugins that spawn windows,
/// they must stay behind the same `!options.headless` guard to keep CI and server runs healthy.
fn add_core_plugins(app: &mut App, options: &CliOptions) {
    if options.headless {
        add_headless_plugins(app);
    } else {
        add_windowed_plugins(app);
    }
}

fn add_headless_plugins(app: &mut App) {
    add_minimal_plugins(app);
}

fn add_windowed_plugins(app: &mut App) {
    // Non-headless runs get the same minimal foundation as our deterministic harness, plus a
    // placeholder plugin that marks where window/audio stacks will hook in once we support them.
    add_minimal_plugins(app);
    app.add_plugins(WindowingPlaceholderPlugin);
}

fn add_minimal_plugins(app: &mut App) {
    use bevy::app::PluginGroup;

    let plugins = MinimalPlugins.build();
    let plugins = configure_task_pool(plugins);
    app.add_plugins(plugins);
    app.add_plugins(bevy::input::InputPlugin);
    ensure_trading_plugin(app);
}

#[derive(Default)]
struct WindowingPlaceholderPlugin;

impl Plugin for WindowingPlaceholderPlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(Resource, Default)]
struct TradingPluginMarker;

fn ensure_trading_plugin(app: &mut App) {
    if app.world().contains_resource::<TradingPluginMarker>() {
        return;
    }

    app.add_plugins(TradingPlugin);
    app.world_mut().insert_resource(TradingPluginMarker);
}

#[cfg(feature = "deterministic")]
fn configure_task_pool(builder: bevy::app::PluginGroupBuilder) -> bevy::app::PluginGroupBuilder {
    use bevy::app::{TaskPoolOptions, TaskPoolPlugin};

    builder.set(TaskPoolPlugin {
        task_pool_options: TaskPoolOptions::with_num_threads(1),
    })
}

#[cfg(not(feature = "deterministic"))]
fn configure_task_pool(builder: bevy::app::PluginGroupBuilder) -> bevy::app::PluginGroupBuilder {
    builder
}

fn simulation_ticks() -> u32 {
    120
}

fn leg_context_from_options(options: &CliOptions) -> LegContext {
    LegContext {
        world_seed: options.world_seed(),
        link_id: RouteId(options.link_id()),
        day: options.day(),
        weather: options.weather(),
        pp: Pp(options.pp()),
        density_per_10k: options.density_per_10k(),
        cadence_per_min: options.cadence_per_min(),
        mission_minutes: options.mission_minutes(),
        player_rating: options.player_rating(),
        multiplayer: false,
        prior_danger_score: None,
        basis_overlay_bp_total: 0,
    }
}

fn leg_context_from_record(meta: &RecordMeta, options: &CliOptions) -> Result<LegContext> {
    let mut context = leg_context_from_options(options);
    context.world_seed = parse_seed_string(&meta.world_seed)?;
    context.link_id = RouteId(parse_u16_string(&meta.link_id)?);
    context.weather = parse_weather_string(&meta.weather)?;
    context.day = meta.day;
    context.pp = Pp(meta.pp);
    context.density_per_10k = meta.density_per_10k;
    context.cadence_per_min = meta.cadence_per_min;
    context.mission_minutes = meta.mission_minutes;
    context.player_rating = meta.player_rating;
    context.prior_danger_score = meta.prior_danger_score;
    Ok(context)
}

fn init_logging() {
    static LOG_INIT: Once = Once::new();

    if tracing::dispatcher::has_been_set() {
        return;
    }

    LOG_INIT.call_once(|| {
        let mut app = App::new();
        LogPlugin::default().build(&mut app);
    });
}

#[cfg(feature = "deterministic")]
fn log_determinism_banner() {
    let features = determinism_feature_flags();
    match director_config_hash() {
        Ok(hash) => info!(
            "=== Deterministic build active: features=[{}] director_cfg_hash={} ===",
            features, hash
        ),
        Err(err) => {
            info!(
                "=== Deterministic build active: features=[{}] director_cfg_hash=unavailable ===",
                features
            );
            warn!("failed to hash director config: {err:?}");
        }
    }
}

#[cfg(not(feature = "deterministic"))]
fn log_determinism_banner() {}

#[cfg(feature = "deterministic")]
fn determinism_feature_flags() -> String {
    let mut features = vec!["deterministic"];
    if cfg!(feature = "econ_logs") {
        features.push("econ_logs");
    }
    if cfg!(feature = "m2_logs") {
        features.push("m2_logs");
    }
    features.join(",")
}

#[cfg(feature = "deterministic")]
fn director_config_hash() -> Result<String> {
    let path = director_cfg_path();
    let bytes =
        fs::read(&path).with_context(|| format!("reading director config {}", path.display()))?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

fn parse_seed_string(value: &str) -> Result<u64> {
    let trimmed = value.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        u64::from_str_radix(hex, 16).map_err(|err| anyhow!("invalid world seed: {err}"))
    } else {
        trimmed
            .parse::<u64>()
            .map_err(|err| anyhow!("invalid world seed: {err}"))
    }
}

fn parse_u16_string(value: &str) -> Result<u16> {
    let trimmed = value.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        u16::from_str_radix(hex, 16).map_err(|err| anyhow!("invalid link id: {err}"))
    } else {
        trimmed
            .parse::<u16>()
            .map_err(|err| anyhow!("invalid link id: {err}"))
    }
}

fn parse_weather_string(value: &str) -> Result<Weather> {
    match value {
        "Clear" => Ok(Weather::Clear),
        "Rains" => Ok(Weather::Rains),
        "Fog" => Ok(Weather::Fog),
        "Windy" => Ok(Weather::Windy),
        other => Err(anyhow!("unknown weather: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_context(options: &CliOptions) -> LegContext {
        leg_context_from_options(options)
    }

    #[cfg(feature = "deterministic")]
    #[test]
    fn deterministic_banner_is_logged() {
        use bevy::log::tracing_subscriber;
        use std::io::{Result as IoResult, Write};
        use std::sync::{Arc, Mutex};
        use tracing_subscriber::fmt::MakeWriter;

        #[derive(Clone)]
        struct BufferWriter(Arc<Mutex<Vec<u8>>>);

        struct BufferHandle(Arc<Mutex<Vec<u8>>>);

        impl<'a> MakeWriter<'a> for BufferWriter {
            type Writer = BufferHandle;

            fn make_writer(&'a self) -> Self::Writer {
                BufferHandle(self.0.clone())
            }
        }

        impl Write for BufferHandle {
            fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
                self.0.lock().unwrap().extend_from_slice(buf);
                Ok(buf.len())
            }

            fn flush(&mut self) -> IoResult<()> {
                Ok(())
            }
        }

        let buffer = Arc::new(Mutex::new(Vec::new()));
        let subscriber = tracing_subscriber::fmt()
            .with_writer(BufferWriter(buffer.clone()))
            .with_ansi(false)
            .without_time()
            .finish();

        bevy::log::tracing::subscriber::with_default(subscriber, || {
            log_determinism_banner();
        });

        let output = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
        assert!(output.contains("Deterministic build active"));
        assert!(output.contains("features=[deterministic"));
        assert!(output.contains("director_cfg_hash="));
    }

    #[test]
    fn headless_mode_skips_window_plugin() {
        let mut options = CliOptions::for_mode(Mode::Play);
        options.headless = true;
        let context = default_context(&options);

        let app = build_app(&options, context);

        assert!(options.headless);
        assert!(!app.is_plugin_added::<WindowingPlaceholderPlugin>());
    }

    #[test]
    fn windowed_mode_registers_window_plugin() {
        let options = CliOptions::for_mode(Mode::Play);
        let context = default_context(&options);

        let app = build_app(&options, context);

        assert!(!options.headless);
        assert!(app.is_plugin_added::<WindowingPlaceholderPlugin>());
    }
}
