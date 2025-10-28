use bevy::input::{keyboard::KeyCode, ButtonInput};
use bevy::prelude::*;

use game::scheduling;
use game::scheduling::sets;
use game::systems::command_queue::CommandQueue;
use game::systems::director::input::{apply_wheel_inputs, WheelInputAction, WheelInputQueue};
use game::systems::director::pause_wheel::{PauseState, Stance, ToolSlot, WheelState};
use game::systems::director::{DirectorPlugin, DirectorState, LegContext};
use game::systems::economy::{Pp, RouteId, Weather};
use repro::Command;

#[test]
fn wheel_state_changes_emit_meters() {
    let mut queue = CommandQueue::default();
    queue.begin_tick(0);
    let mut wheel = WheelState::default();
    wheel.set_stance(&mut queue, Stance::Vault);
    wheel.set_tool(&mut queue, ToolSlot::B);
    wheel.set_overwatch(&mut queue, true);
    wheel.set_move_mode(&mut queue, true);
    wheel.set_slowmo(&mut queue, true);

    let expected = vec![
        Command::meter_at(0, "wheel_stance", 1),
        Command::meter_at(0, "wheel_tool", 1),
        Command::meter_at(0, "wheel_overwatch", 1),
        Command::meter_at(0, "wheel_move", 1),
        Command::meter_at(0, "wheel_slowmo", 1),
    ];
    assert_eq!(queue.buf, expected);
    assert!(wheel.slowmo_enabled);
}

#[test]
fn pause_state_emits_meter() {
    let mut queue = CommandQueue::default();
    queue.begin_tick(0);
    let mut pause = PauseState::default();
    pause.set_hard_pause(&mut queue, true);
    let expected = vec![Command::meter_at(0, "wheel_hard_pause", 1)];
    assert_eq!(queue.buf, expected);
}

#[test]
fn queued_input_updates_states_and_emits_meters() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    scheduling::configure(&mut app);
    app.init_resource::<CommandQueue>();
    app.init_resource::<WheelState>();
    app.init_resource::<PauseState>();
    app.init_resource::<WheelInputQueue>();
    app.insert_resource(LegContext {
        multiplayer: false,
        ..Default::default()
    });
    app.add_systems(FixedUpdate, apply_wheel_inputs.in_set(sets::DETTEROT_Input));

    {
        let mut queue = app.world_mut().resource_mut::<WheelInputQueue>();
        queue.extend([
            WheelInputAction::SetStance(Stance::Vault),
            WheelInputAction::SetTool(ToolSlot::B),
            WheelInputAction::SetOverwatch(true),
            WheelInputAction::SetMoveMode(true),
            WheelInputAction::SetSlowmo(true),
            WheelInputAction::SetHardPause(true),
        ]);
    }

    {
        let mut command_queue = app.world_mut().resource_mut::<CommandQueue>();
        command_queue.begin_tick(0);
    }

    app.world_mut().run_schedule(FixedUpdate);

    let commands = app.world_mut().resource_mut::<CommandQueue>().drain();
    let expected = vec![
        Command::meter_at(0, "wheel_stance", 1),
        Command::meter_at(0, "wheel_tool", 1),
        Command::meter_at(0, "wheel_overwatch", 1),
        Command::meter_at(0, "wheel_move", 1),
        Command::meter_at(0, "wheel_slowmo", 1),
        Command::meter_at(0, "wheel_hard_pause", 1),
    ];
    assert_eq!(commands, expected);
}

#[test]
fn multiplayer_input_ignores_hard_pause_and_slowmo_requests() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    scheduling::configure(&mut app);
    app.init_resource::<CommandQueue>();
    app.init_resource::<WheelState>();
    app.init_resource::<PauseState>();
    app.init_resource::<WheelInputQueue>();
    app.insert_resource(LegContext {
        multiplayer: true,
        ..Default::default()
    });
    app.add_systems(FixedUpdate, apply_wheel_inputs.in_set(sets::DETTEROT_Input));

    {
        let mut queue = app.world_mut().resource_mut::<WheelInputQueue>();
        queue.extend([
            WheelInputAction::SetSlowmo(true),
            WheelInputAction::SetHardPause(true),
        ]);
    }

    {
        let mut command_queue = app.world_mut().resource_mut::<CommandQueue>();
        command_queue.begin_tick(0);
    }

    app.world_mut().run_schedule(FixedUpdate);

    let commands = app.world_mut().resource_mut::<CommandQueue>().drain();
    assert!(commands.is_empty());
}

#[test]
fn keyboard_input_updates_wheel_state() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    scheduling::configure(&mut app);
    app.init_resource::<CommandQueue>();
    app.init_resource::<WheelState>();
    app.init_resource::<PauseState>();
    app.init_resource::<WheelInputQueue>();
    app.insert_resource(LegContext {
        multiplayer: false,
        ..Default::default()
    });
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.add_systems(FixedUpdate, apply_wheel_inputs.in_set(sets::DETTEROT_Input));

    {
        let mut keyboard = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keyboard.press(KeyCode::Digit2);
        keyboard.press(KeyCode::Digit4);
        keyboard.press(KeyCode::KeyO);
        keyboard.press(KeyCode::KeyM);
        keyboard.press(KeyCode::KeyL);
        keyboard.press(KeyCode::Space);
    }

    {
        let mut command_queue = app.world_mut().resource_mut::<CommandQueue>();
        command_queue.begin_tick(0);
    }

    app.world_mut().run_schedule(FixedUpdate);

    let commands = app.world_mut().resource_mut::<CommandQueue>().drain();
    let expected = vec![
        Command::meter_at(0, "wheel_stance", 1),
        Command::meter_at(0, "wheel_tool", 1),
        Command::meter_at(0, "wheel_overwatch", 1),
        Command::meter_at(0, "wheel_move", 1),
        Command::meter_at(0, "wheel_slowmo", 1),
        Command::meter_at(0, "wheel_hard_pause", 1),
    ];
    assert_eq!(commands, expected);
}

fn build_director_app_for_pause_tests() -> App {
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
    app.init_resource::<CommandQueue>();
    app.insert_resource(test_leg_context());
    app.add_plugins(DirectorPlugin);
    app.finish();
    app.update();
    app
}

fn test_leg_context() -> LegContext {
    LegContext {
        world_seed: 0xD77E_2024_ABCD_0002,
        link_id: RouteId(11),
        day: 4,
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

fn step_director(app: &mut App) -> Vec<Command> {
    let current_tick = app.world().resource::<DirectorState>().leg_tick;
    {
        let mut queue = app.world_mut().resource_mut::<CommandQueue>();
        queue.begin_tick(current_tick);
    }
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().resource_mut::<CommandQueue>().drain()
}

#[test]
fn hard_pause_freezes_ticks_and_commands() {
    let mut app = build_director_app_for_pause_tests();

    // Advance once to establish a baseline and ensure systems are running.
    let _ = step_director(&mut app);

    let tick_before_pause = app.world().resource::<DirectorState>().leg_tick;
    {
        let mut inputs = app.world_mut().resource_mut::<WheelInputQueue>();
        inputs.push(WheelInputAction::SetHardPause(true));
    }
    let pause_commands = step_director(&mut app);
    assert!(pause_commands.iter().any(|command| {
        *command == Command::meter_at(tick_before_pause, "wheel_hard_pause", 1)
    }));
    let paused_tick = app.world().resource::<DirectorState>().leg_tick;

    let commands_while_paused = step_director(&mut app);
    assert!(
        commands_while_paused.is_empty(),
        "no commands should be emitted while paused"
    );
    assert_eq!(
        app.world().resource::<DirectorState>().leg_tick,
        paused_tick
    );

    {
        let mut inputs = app.world_mut().resource_mut::<WheelInputQueue>();
        inputs.push(WheelInputAction::SetHardPause(false));
    }
    let tick_before_resume = app.world().resource::<DirectorState>().leg_tick;
    let resume_commands = step_director(&mut app);
    assert!(resume_commands.iter().any(|command| {
        *command == Command::meter_at(tick_before_resume, "wheel_hard_pause", 0)
    }));
    assert!(app.world().resource::<DirectorState>().leg_tick > paused_tick);
}
