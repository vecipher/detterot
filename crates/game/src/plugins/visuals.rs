use bevy::color::LinearRgba;
use bevy::math::primitives::Cuboid;
use bevy::mesh::Mesh;
use bevy::pbr::{MeshMaterial3d, StandardMaterial};
use bevy::prelude::*;

pub struct VisualsPlugin;
impl Plugin for VisualsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
    }
}

fn setup(
    mut cmds: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    cmds.spawn((
        Camera3d::default(),
        Camera::default(),
        Transform::from_xyz(0.0, 20.0, 40.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    cmds.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 35_000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.8, 0.0)),
    ));
    // A neon cube as a sanity check
    cmds.spawn((
        Mesh3d(meshes.add(Mesh::from(Cuboid::new(2.0, 2.0, 2.0)))),
        MeshMaterial3d(materials.add(StandardMaterial {
            emissive: LinearRgba::new(2.0, 0.2, 4.0, 1.0),
            ..default()
        })),
        Transform::from_xyz(0.0, 1.0, 0.0),
    ));
}
