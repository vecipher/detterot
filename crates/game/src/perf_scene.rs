use bevy::prelude::*;
use serde::Deserialize;

pub struct PerfScenePlugin;
impl Plugin for PerfScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, maybe_load_perf_scene)
            .add_systems(Update, drive_camera_path);
    }
}

#[derive(Deserialize)]
struct PathPoint {
    t: f32,
    pos: [f32; 3],
    look_at: [f32; 3],
}
#[derive(Resource)]
struct Path {
    points: Vec<PathPoint>,
    t: f32,
}

fn maybe_load_perf_scene(mut cmds: Commands, assets: Res<AssetServer>) {
    // For M0: load the first path if present
    let _ = assets;
    let Ok(txt) = std::fs::read_to_string("repro/perf_scenes.toml") else {
        return;
    };
    let scn: toml::Value = toml::from_str(&txt).unwrap();
    let first = scn
        .get("scene")
        .and_then(|v| v.as_array())
        .and_then(|a| a.first());
    if let Some(scene) = first {
        if let Some(path) = scene.get("camera_path").and_then(|s| s.as_str()) {
            if let Ok(json) = std::fs::read_to_string(format!("repro/{}", path)) {
                let points: Vec<PathPoint> = serde_json::from_str(&json).unwrap_or_default();
                cmds.insert_resource(Path { points, t: 0.0 });
                // (Optional) pre-warm assets in M0+, not required now.
            }
        }
    }
}

fn drive_camera_path(
    time: Res<Time>,
    mut q_cam: Query<&mut Transform, With<Camera>>,
    path: Option<ResMut<Path>>,
) {
    let Some(mut path) = path else {
        return;
    };
    let Ok(mut tf) = q_cam.single_mut() else {
        return;
    };
    if path.points.is_empty() {
        return;
    }
    path.t += time.delta_secs();
    // Simple linear sample between nearest points
    if path.points.len() == 1 {
        let point = &path.points[0];
        tf.translation = Vec3::from_array(point.pos);
        *tf = tf.looking_at(Vec3::from_array(point.look_at), Vec3::Y);
        return;
    }

    let last_idx = path.points.len() - 1;
    let last_t = path.points[last_idx].t;
    if path.t > last_t {
        path.t = last_t;
    }

    let (mut a, mut b) = (last_idx - 1, last_idx);
    for i in 1..path.points.len() {
        if path.t <= path.points[i].t {
            a = i - 1;
            b = i;
            break;
        }
    }
    let pa = &path.points[a];
    let pb = &path.points[b];
    let span = (pb.t - pa.t).max(0.0001);
    let u = ((path.t - pa.t) / span).clamp(0.0, 1.0);
    let lerp = |a: [f32; 3], b: [f32; 3]| {
        [
            a[0] + (b[0] - a[0]) * u,
            a[1] + (b[1] - a[1]) * u,
            a[2] + (b[2] - a[2]) * u,
        ]
    };
    let p = lerp(pa.pos, pb.pos);
    let look = Vec3::from_array(lerp(pa.look_at, pb.look_at));
    tf.translation = Vec3::from_array(p);
    *tf = tf.looking_at(look, Vec3::Y);
}
