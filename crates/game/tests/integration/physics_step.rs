use avian3d::prelude::{Physics, SubstepCount};
use bevy::prelude::*;
use bevy::time::{Fixed, Time as BevyTime};

const FIXED_STEP_SECONDS: f64 = f64::from_bits(0x3F91_1111_1111_1111);

use game::scheduling;
use game::systems::command_queue::CommandQueue;
use game::systems::director::{
    DirectorPlugin, DirectorState, LegContext, LegStatus, Outcome, WheelState,
};
use game::systems::economy::{Pp, RouteId, Weather};
use repro::Command;

#[cfg(feature = "deterministic")]
use blake3::hash as blake3_hash;

#[cfg(feature = "deterministic")]
use repro::canonical_json_bytes;

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
    let _ = step_once_collect(app);
}

fn step_once_collect(app: &mut App) -> Vec<Command> {
    let current_tick = app.world().resource::<DirectorState>().leg_tick;
    let commands = {
        let world = app.world_mut();
        {
            let mut queue = world.resource_mut::<CommandQueue>();
            queue.begin_tick(current_tick);
        }
        world.run_schedule(FixedUpdate);
        world.resource_mut::<CommandQueue>().drain()
    };
    commands
}

#[test]
fn finalize_leg_clamps_to_mission_window() {
    let mut app = build_director_app();

    for _ in 0..200 {
        step_once(&mut app);
    }

    {
        let mut context = app.world_mut().resource_mut::<LegContext>();
        context.mission_minutes = 1;
    }

    let (current_tick, commands) = {
        let world = app.world_mut();
        let current_tick = world.resource::<DirectorState>().leg_tick;
        {
            let mut queue = world.resource_mut::<CommandQueue>();
            queue.begin_tick(current_tick);
        }
        world.run_schedule(FixedUpdate);
        let commands = world.resource_mut::<CommandQueue>().drain();
        (current_tick, commands)
    };

    let target_tick = 1_u32.saturating_mul(60);
    let tolerance = 60;
    let expected_max = target_tick + tolerance;
    let overflow = current_tick.saturating_sub(expected_max) as i32;

    let state = app.world().resource::<DirectorState>();
    assert_eq!(state.leg_tick, expected_max);
    assert!(matches!(
        state.status,
        LegStatus::Completed(Outcome::Success)
    ));

    assert!(
        commands.iter().any(|command| {
            *command == Command::meter_at(current_tick, "leg_tick_over_window", overflow)
        }),
        "expected leg_tick_over_window meter with overflow {overflow}"
    );
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
fn physics_step_slowmo_preserves_fixed_cadence() {
    let mut app = build_director_app();

    step_once(&mut app);
    let cadence = test_leg_context().cadence_per_min;
    let expected_substeps = expected_substeps(cadence);
    let baseline_substeps = app.world().resource::<SubstepCount>().0;
    assert_eq!(baseline_substeps, expected_substeps);

    app.world_mut()
        .resource_scope(|world, mut queue: Mut<CommandQueue>| {
            world
                .resource_mut::<WheelState>()
                .set_slowmo(&mut queue, true);
        });

    step_once(&mut app);
    let slowmo_substeps = app.world().resource::<SubstepCount>().0;
    assert_eq!(slowmo_substeps, expected_substeps);
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

#[cfg(feature = "deterministic")]
#[test]
fn physics_step_slowmo_toggle_preserves_command_trace() {
    fn trace_hash(commands: &[Command]) -> String {
        let bytes = canonical_json_bytes(&commands.to_vec()).expect("canonical command trace");
        blake3_hash(&bytes).to_hex().to_string()
    }

    let mut baseline_app = build_director_app();
    let baseline_commands = {
        let mut commands = Vec::new();
        for _ in 0..6 {
            commands.extend(step_once_collect(&mut baseline_app));
        }
        commands
    };

    let mut slowmo_app = build_director_app();
    let mut slowmo_commands = Vec::new();
    for tick in 0..6 {
        if tick == 1 {
            slowmo_app
                .world_mut()
                .resource_scope(|world, mut queue: Mut<CommandQueue>| {
                    world
                        .resource_mut::<WheelState>()
                        .set_slowmo(&mut queue, true);
                });
        }
        if tick == 4 {
            slowmo_app
                .world_mut()
                .resource_scope(|world, mut queue: Mut<CommandQueue>| {
                    world
                        .resource_mut::<WheelState>()
                        .set_slowmo(&mut queue, false);
                });
        }
        slowmo_commands.extend(step_once_collect(&mut slowmo_app));
    }

    assert_eq!(baseline_commands, slowmo_commands);
    assert_eq!(trace_hash(&baseline_commands), trace_hash(&slowmo_commands));
}
