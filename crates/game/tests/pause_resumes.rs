use bevy::prelude::FixedUpdate;
use game::systems::command_queue::CommandQueue;
use game::systems::director::pause_wheel::PauseState;
use game::systems::director::{DirectorState, LegStatus};
use game::{build_headless_app, request_new_leg};

#[test]
fn fixed_update_respects_pause_state() {
    let mut app = build_headless_app();

    // Prime the simulation until the director starts emitting commands.
    let mut baseline_commands = Vec::new();
    for _ in 0..50 {
        app.world_mut().run_schedule(FixedUpdate);
        let drained = {
            let mut queue = app.world_mut().resource_mut::<CommandQueue>();
            queue.drain()
        };
        if !drained.is_empty() {
            baseline_commands = drained;
            break;
        }
    }

    assert!(
        !baseline_commands.is_empty(),
        "expected the director to emit commands before pausing",
    );

    let leg_tick_before_pause = {
        let state = app.world().resource::<DirectorState>();
        assert!(
            !matches!(state.status, LegStatus::Completed(_)),
            "director should still be mid-leg before pausing",
        );
        state.leg_tick
    };

    {
        let mut pause = app.world_mut().resource_mut::<PauseState>();
        pause.hard_paused_sp = true;
    }
    request_new_leg(&mut app);

    for _ in 0..10 {
        app.world_mut().run_schedule(FixedUpdate);
    }

    let paused_commands = {
        let mut queue = app.world_mut().resource_mut::<CommandQueue>();
        queue.drain()
    };
    assert!(
        paused_commands.is_empty(),
        "no commands should be emitted while hard paused",
    );

    let (leg_tick_after_pause, status_after_pause) = {
        let state = app.world().resource::<DirectorState>();
        assert_eq!(
            state.leg_tick, leg_tick_before_pause,
            "leg tick should not advance while paused"
        );
        (state.leg_tick, state.status)
    };

    {
        let mut pause = app.world_mut().resource_mut::<PauseState>();
        pause.hard_paused_sp = false;
    }

    let mut resumed_commands = Vec::new();
    let mut resumed_state = None;
    for _ in 0..200 {
        app.world_mut().run_schedule(FixedUpdate);
        let drained = {
            let mut queue = app.world_mut().resource_mut::<CommandQueue>();
            queue.drain()
        };
        if !drained.is_empty() {
            resumed_commands = drained;
            let state = app.world().resource::<DirectorState>();
            resumed_state = Some((state.leg_tick, state.status));
            break;
        }
    }

    let (resumed_tick, resumed_status) =
        resumed_state.expect("director state should advance after clearing the hard pause");
    assert!(
        !resumed_commands.is_empty(),
        "commands should resume after clearing the hard pause",
    );
    assert!(
        resumed_tick != leg_tick_after_pause || resumed_status != status_after_pause,
        "director should make progress once unpaused",
    );
}
