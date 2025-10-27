use game::systems::command_queue::CommandQueue;
use game::systems::director::pause_wheel::{PauseState, Stance, ToolSlot, WheelState};
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
