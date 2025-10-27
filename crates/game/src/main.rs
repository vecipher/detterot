use std::{fs, path::PathBuf};

use anyhow::{anyhow, Context, Result};
#[cfg(feature = "dev")]
use avian3d::debug_render::PhysicsDebugPlugin;
use avian3d::prelude::*;
#[cfg(feature = "deterministic")]
use bevy::{app::TaskPoolPlugin, tasks::TaskPoolOptions};
use bevy::{
    color::LinearRgba,
    math::primitives::{Cuboid, Sphere},
    mesh::Mesh,
    pbr::{MeshMaterial3d, StandardMaterial},
    prelude::*,
    time::Fixed,
};
#[cfg(feature = "audio")]
use bevy_kira_audio::prelude::*;
use repro::Record;

mod diagnostics;
mod perf_scene;
mod plugins;

use game::{
    runtime,
    systems::{
        command_queue::CommandQueue,
        director::{DirectorPlugin, LegParameters, LogSettings},
    },
    CliOptions, RunMode,
};

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err:?}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let options = CliOptions::parse()?;
    match options.mode {
        RunMode::Play => run_play(options),
        RunMode::Record => run_record(options),
        RunMode::Replay => run_replay(options),
    }
}

fn run_play(options: CliOptions) -> Result<()> {
    let asset_path = resolve_asset_directory();
    let mut app = App::new();
    app.insert_resource(LegParameters::default());
    let logs_enabled = options.debug_logs || cfg!(feature = "m2_logs");
    app.insert_resource(LogSettings {
        enabled: logs_enabled,
    });
    let default_plugins = DefaultPlugins
        .set(AssetPlugin {
            file_path: asset_path,
            watch_for_changes_override: Some(cfg!(debug_assertions)),
            ..default()
        })
        .set(WindowPlugin {
            primary_window: Some(Window {
                title: "Detterot".into(),
                ..default()
            }),
            ..default()
        });
    #[cfg(feature = "deterministic")]
    let default_plugins = {
        default_plugins.set(TaskPoolPlugin {
            task_pool_options: TaskPoolOptions::with_num_threads(1),
        })
    };
    app.add_plugins(default_plugins);
    app.add_plugins(PhysicsPlugins::default().set(PhysicsInterpolationPlugin::interpolate_all()));
    #[cfg(feature = "dev")]
    {
        app.add_plugins(PhysicsDebugPlugin);
    }
    #[cfg(feature = "audio")]
    {
        app.add_plugins(AudioPlugin);
    }
    app.add_plugins((
        DirectorPlugin,
        plugins::VisualsPlugin,
        diagnostics::DiagnosticsUiPlugin,
        perf_scene::PerfScenePlugin,
    ));
    app.add_systems(Startup, spawn_world);
    #[cfg(feature = "audio")]
    app.add_systems(Startup, play_boot_sound);
    app.add_systems(Update, drop_commands);
    app.insert_resource(Time::<Fixed>::from_seconds(1.0 / 30.0));
    app.run();
    Ok(())
}

fn drop_commands(mut queue: ResMut<CommandQueue>) {
    if !queue.buf.is_empty() {
        queue.buf.clear();
    }
}

fn run_record(options: CliOptions) -> Result<()> {
    let io_path = options
        .io
        .clone()
        .ok_or_else(|| anyhow!("--io is required in record mode"))?;
    let mut params = if io_path.exists() {
        load_params_from_record(&io_path)?
    } else {
        LegParameters::default()
    };
    let mut cfg = runtime::apply_manifest_for_path(&io_path, &mut params)?;
    let dt = options
        .fixed_dt
        .ok_or_else(|| anyhow!("--fixed-dt is required in record mode"))?;
    cfg.dt = dt;
    cfg.logs_enabled = options.debug_logs || cfg.logs_enabled || cfg!(feature = "m2_logs");
    let record = runtime::record_leg(&params, &cfg)?;
    write_record(&io_path, &record)?;
    Ok(())
}

fn run_replay(options: CliOptions) -> Result<()> {
    let io_path = options
        .io
        .clone()
        .ok_or_else(|| anyhow!("--io is required in replay mode"))?;
    let data =
        fs::read_to_string(&io_path).with_context(|| format!("unable to read {:?}", io_path))?;
    let record: Record = serde_json::from_str(&data)
        .with_context(|| format!("invalid record json at {:?}", io_path))?;
    let mut params = LegParameters::from_record_meta(&record.meta);
    let mut cfg = runtime::apply_manifest_for_path(&io_path, &mut params)?;
    let dt = options
        .fixed_dt
        .ok_or_else(|| anyhow!("--fixed-dt is required in replay mode"))?;
    cfg.dt = dt;
    cfg.logs_enabled = options.debug_logs || cfg.logs_enabled || cfg!(feature = "m2_logs");
    runtime::replay_leg(&params, &cfg, &record, options.continue_after_mismatch)?;
    Ok(())
}

fn write_record(path: &PathBuf, record: &Record) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("unable to create directory for {:?}", path))?;
    }
    let canonical = record
        .canonical_json()
        .context("failed to canonicalize record json")?;
    fs::write(path, canonical.as_bytes())
        .with_context(|| format!("unable to write record {:?}", path))?;
    let hash = record.hash_hex().context("failed to compute record hash")?;
    let mut hash_path = path.clone();
    hash_path.set_extension("hash");
    fs::write(&hash_path, format!("{hash}\n"))
        .with_context(|| format!("unable to write hash {:?}", hash_path))?;
    Ok(())
}

fn load_params_from_record(path: &PathBuf) -> Result<LegParameters> {
    let data = fs::read_to_string(path).with_context(|| format!("unable to read {:?}", path))?;
    if data.trim().is_empty() {
        return Ok(LegParameters::default());
    }
    let record: Record = serde_json::from_str(&data)
        .with_context(|| format!("invalid record json at {:?}", path))?;
    Ok(LegParameters::from_record_meta(&record.meta))
}

fn resolve_asset_directory() -> String {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(mut directory) = exe_path.parent() {
            loop {
                let candidate = directory.join("assets");
                if candidate.is_dir() {
                    return candidate.to_string_lossy().into_owned();
                }

                match directory.parent() {
                    Some(parent) => directory = parent,
                    None => break,
                }
            }
        }
    }

    if let Ok(current_dir) = std::env::current_dir() {
        let candidate = current_dir.join("assets");
        if candidate.is_dir() {
            return candidate.to_string_lossy().into_owned();
        }
    }

    "assets".to_string()
}

fn spawn_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let ground_mesh = meshes.add(Mesh::from(Cuboid::new(100.0, 1.0, 100.0)));
    let ground_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.08, 0.09, 0.11),
        perceptual_roughness: 1.0,
        ..default()
    });
    commands.spawn((
        Mesh3d(ground_mesh),
        MeshMaterial3d(ground_material),
        Transform::from_xyz(0.0, -1.5, 0.0),
        RigidBody::Static,
        Collider::cuboid(50.0, 0.5, 50.0),
    ));

    let drop_mesh = meshes.add(Mesh::from(Sphere::new(1.0)));
    let drop_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.6, 0.8, 1.0),
        emissive: LinearRgba::new(0.1, 0.3, 0.6, 1.0),
        ..default()
    });
    commands.spawn((
        Mesh3d(drop_mesh),
        MeshMaterial3d(drop_material),
        Transform::from_xyz(0.0, 6.0, 0.0),
        RigidBody::Dynamic,
        Collider::sphere(1.0),
    ));
}

#[cfg(feature = "audio")]
fn play_boot_sound(server: Res<AssetServer>, audio: Res<Audio>) {
    let handle: Handle<AudioSource> = server.load("audio/boot.wav");
    audio.play(handle).looped();
}
