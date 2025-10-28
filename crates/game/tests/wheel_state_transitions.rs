use bevy::prelude::*;
use bevy::time::Virtual;
use bevy::MinimalPlugins;
use game::systems::command_queue::CommandQueue;
use game::systems::director::pause_wheel::{PauseState, Stance, ToolSlot, WheelState};

#[test]
fn wheel_updates_emit_meters_once() {
    let mut wheel = WheelState::default();
    let mut pause = PauseState::default();
    let mut queue = CommandQueue::default();

    queue.set_tick(1);
    wheel.set_stance(Stance::Vault, &mut queue);
    queue.set_tick(2);
    wheel.set_stance(Stance::Vault, &mut queue);
    queue.set_tick(3);
    wheel.set_tool(ToolSlot::B, &mut queue);
    queue.set_tick(4);
    wheel.set_overwatch(true, &mut queue);
    queue.set_tick(5);
    wheel.set_move_mode(true, &mut queue);
    queue.set_tick(6);
    wheel.set_slowmo(true, &mut queue);
    queue.set_tick(7);
    pause.set_hard_pause(true, &mut queue);
    queue.set_tick(8);
    pause.set_hard_pause(true, &mut queue);

    let commands = queue.drain();
    let meters: Vec<_> = commands
        .into_iter()
        .filter_map(|cmd| match cmd.kind {
            repro::CommandKind::Meter { key, value } => Some((cmd.t, key, value)),
            _ => None,
        })
        .collect();
    assert_eq!(meters.len(), 6);
    assert!(meters.contains(&(1, "wheel_stance".into(), 1)));
    assert!(meters.contains(&(3, "wheel_tool".into(), 1)));
    assert!(meters.contains(&(4, "wheel_overwatch".into(), 1)));
    assert!(meters.contains(&(5, "wheel_move_mode".into(), 1)));
    assert!(meters.contains(&(6, "wheel_slowmo".into(), 1)));
    assert!(meters.contains(&(7, "wheel_hard_pause".into(), 1)));
}

#[test]
fn slowmo_adjusts_time_without_extra_meters() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    game::scheduling::configure(&mut app);
    app.add_plugins(game::systems::director::DirectorPlugin);

    app.update();

    {
        let time = app.world().resource::<Time<Virtual>>();
        assert_eq!(time.relative_speed().to_bits(), 1.0_f32.to_bits());
    }

    app.world_mut()
        .resource_scope(|world, mut queue: Mut<CommandQueue>| {
            queue.set_tick(1);
            let mut wheel = world.resource_mut::<WheelState>();
            wheel.set_slowmo(true, &mut queue);
        });

    app.update();

    {
        let time = app.world().resource::<Time<Virtual>>();
        assert_eq!(time.relative_speed().to_bits(), 0.8_f32.to_bits());
    }

    app.world_mut()
        .resource_scope(|world, mut queue: Mut<CommandQueue>| {
            queue.set_tick(2);
            let mut wheel = world.resource_mut::<WheelState>();
            wheel.set_slowmo(false, &mut queue);
        });

    app.update();

    {
        let time = app.world().resource::<Time<Virtual>>();
        assert_eq!(time.relative_speed().to_bits(), 1.0_f32.to_bits());
    }

    let mut queue = app.world_mut().resource_mut::<CommandQueue>();
    let commands = queue.drain();
    let meters: Vec<_> = commands
        .into_iter()
        .filter_map(|cmd| match cmd.kind {
            repro::CommandKind::Meter { key, value } => Some((cmd.t, key, value)),
            _ => None,
        })
        .collect();

    assert_eq!(meters.len(), 2);
    assert!(meters.contains(&(1, "wheel_slowmo".into(), 1)));
    assert!(meters.contains(&(2, "wheel_slowmo".into(), 0)));
}
