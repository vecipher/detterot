#[cfg(feature = "dev")]
use avian3d::debug_render::PhysicsDebugPlugin;
use avian3d::prelude::*;
use bevy::color::LinearRgba;
use bevy::math::primitives::{Cuboid, Sphere};
use bevy::mesh::Mesh;
use bevy::pbr::{MeshMaterial3d, StandardMaterial};
use bevy::prelude::*;
use bevy_kira_audio::prelude::*;

mod diagnostics;
mod perf_scene;
mod plugins;

fn main() {
    let asset_path = resolve_asset_directory();
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
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
            }),
    );
    app.add_plugins(PhysicsPlugins::default().set(PhysicsInterpolationPlugin::interpolate_all()));
    #[cfg(feature = "dev")]
    {
        app.add_plugins(PhysicsDebugPlugin);
    }
    app.add_plugins(AudioPlugin);
    app.add_plugins((
        plugins::VisualsPlugin,
        diagnostics::DiagnosticsUiPlugin,
        perf_scene::PerfScenePlugin,
    ));
    app.add_systems(Startup, (spawn_world, play_boot_sound));
    app.add_systems(FixedUpdate, drive_sim);
    app.run();
}

fn drive_sim() {}

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

fn play_boot_sound(server: Res<AssetServer>, audio: Res<Audio>) {
    let handle: Handle<AudioSource> = server.load("audio/boot.wav");
    audio.play(handle).looped();
}
