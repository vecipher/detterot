use avian3d::prelude::{Physics, SubstepCount};
use bevy::prelude::*;
use bevy::time::{Fixed, Time as BevyTime};

const FIXED_STEP_SECONDS: f64 = f64::from_bits(0x3F91_1111_1111_1111);

use game::scheduling;
use game::systems::command_queue::CommandQueue;
use game::systems::director::{DirectorPlugin, DirectorState, LegContext, WheelState};
use game::systems::economy::{Pp, RouteId, Weather};

const SLOWMO_NUMERATOR: u32 = 4;
const SLOWMO_DENOMINATOR: u32 = 5;

fn build_director_app() -> App {
    let mut app = App::new();

    #[cfg(feature = "deterministic")]
    {
        use bevy::app::{PluginGroup, TaskPoolOptions, TaskPoolPlugin};
        let plugins = bevy::MinimalPlugins.build().set(TaskPoolPlugin {
            task_pool_options: TaskPoolOptions::with_num_threads(1),
        });
        app.add_plugins(plugins);
    }

    #[cfg(not(feature = "deterministic"))]
    {
        app.add_plugins(MinimalPlugins);
    }

    scheduling::configure(&mut app);
    {
        let mut fixed = app.world_mut().resource_mut::<BevyTime<Fixed>>();
        *fixed = BevyTime::<Fixed>::from_seconds(FIXED_STEP_SECONDS);
    }

    app.init_resource::<CommandQueue>();
    app.insert_resource(test_leg_context());
    app.add_plugins(DirectorPlugin);
    app.finish();
    app.update();
    app
}

fn test_leg_context() -> LegContext {
    LegContext {
        world_seed: 0xD77E_2024_ABCD_0001,
        link_id: RouteId(7),
        day: 3,
        weather: Weather::Clear,
        pp: Pp(150),
        density_per_10k: 5,
        cadence_per_min: 90,
        mission_minutes: 12,
        player_rating: 40,
        multiplayer: false,
        prior_danger_score: None,
        basis_overlay_bp_total: 0,
    }
}

fn expected_substeps(cadence_per_min: u32) -> u32 {
    if cadence_per_min == 0 {
        1
    } else {
        ((cadence_per_min - 1) / 60).saturating_add(1).clamp(1, 12)
    }
}

fn step_once(app: &mut App) {
    let current_tick = app.world().resource::<DirectorState>().leg_tick;
    {
        let world = app.world_mut();
        {
            let mut queue = world.resource_mut::<CommandQueue>();
            queue.begin_tick(current_tick);
        }
        world.run_schedule(FixedUpdate);
    }
    app.world_mut().resource_mut::<CommandQueue>().drain();
}

#[test]
fn physics_step_advances_physics_time() {
    let mut app = build_director_app();

    let before = app.world().resource::<Time<Physics>>().elapsed();
    step_once(&mut app);
    let after = app.world().resource::<Time<Physics>>().elapsed();

    assert!(after > before, "physics time should advance after stepping");

    let cadence = test_leg_context().cadence_per_min;
    let expected = expected_substeps(cadence);
    let actual = app.world().resource::<SubstepCount>().0;
    assert_eq!(actual, expected);
}

#[test]
fn physics_step_slowmo_scales_cadence_and_time() {
    let mut app = build_director_app();

    let initial_elapsed = app.world().resource::<Time<Physics>>().elapsed();
    step_once(&mut app);
    let cadence = test_leg_context().cadence_per_min;
    let expected_normal = expected_substeps(cadence);
    let (first_elapsed, normal_delta, normal_substeps) = {
        let world = app.world();
        let first_elapsed = world.resource::<Time<Physics>>().elapsed();
        let normal_substeps = world.resource::<SubstepCount>().0;
        (
            first_elapsed,
            first_elapsed - initial_elapsed,
            normal_substeps,
        )
    };

    assert_eq!(normal_substeps, expected_normal);

    app.world_mut()
        .resource_scope(|world, mut queue: Mut<CommandQueue>| {
            world
                .resource_mut::<WheelState>()
                .set_slowmo(&mut queue, true);
        });

    step_once(&mut app);
    let (slowmo_delta, slowmo_substeps) = {
        let world = app.world();
        let second_elapsed = world.resource::<Time<Physics>>().elapsed();
        let slowmo_substeps = world.resource::<SubstepCount>().0;
        (second_elapsed - first_elapsed, slowmo_substeps)
    };

    let expected_cadence = cadence.saturating_mul(SLOWMO_NUMERATOR) / SLOWMO_DENOMINATOR;
    let expected_slowmo = expected_substeps(expected_cadence);

    assert_eq!(slowmo_substeps, expected_slowmo);

    let normal_nanos = normal_delta.as_nanos();
    let expected_slowmo_nanos = normal_nanos
        .saturating_mul(SLOWMO_NUMERATOR as u128)
        .saturating_div(SLOWMO_DENOMINATOR as u128);
    assert_eq!(slowmo_delta.as_nanos(), expected_slowmo_nanos);
}

#[cfg(feature = "deterministic")]
#[test]
fn physics_step_deterministic_under_feature() {
    let mut app_a = build_director_app();
    let mut app_b = build_director_app();

    let trace_a = capture_trace(&mut app_a, 5);
    let trace_b = capture_trace(&mut app_b, 5);

    assert_eq!(trace_a, trace_b, "physics stepping should be deterministic");
}

#[cfg(feature = "deterministic")]
fn capture_trace(app: &mut App, ticks: usize) -> Vec<(u32, u128)> {
    let mut trace = Vec::with_capacity(ticks);
    for _ in 0..ticks {
        step_once(app);
        let world = app.world();
        let tick = world.resource::<DirectorState>().leg_tick;
        let elapsed = world.resource::<Time<Physics>>().elapsed().as_nanos();
        trace.push((tick, elapsed));
    }
    trace
}
