use bevy::input::{keyboard::KeyCode, ButtonInput};
use bevy::prelude::*;

use game::scheduling;
use game::scheduling::sets;
use game::systems::command_queue::CommandQueue;
use game::systems::director::input::{apply_wheel_inputs, WheelInputAction, WheelInputQueue};
use game::systems::director::pause_wheel::{PauseState, Stance, ToolSlot, WheelState};
use game::systems::director::LegContext;
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
fn multiplayer_input_ignores_hard_pause_requests() {
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
        queue.push(WheelInputAction::SetHardPause(true));
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
