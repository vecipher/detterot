use bevy::prelude::Resource;

use crate::systems::command_queue::CommandQueue;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Stance {
    #[default]
    Brace,
    Vault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ToolSlot {
    #[default]
    A,
    B,
}

#[derive(Resource, Default, Debug)]
pub struct WheelState {
    pub stance: Stance,
    pub tool: ToolSlot,
    pub overwatch: bool,
    pub move_mode: bool,
    pub slowmo_enabled: bool,
}

impl WheelState {
    pub fn emit_meters(&self, queue: &mut CommandQueue) {
        queue.meter(
            "wheel_stance",
            match self.stance {
                Stance::Brace => 0,
                Stance::Vault => 1,
            },
        );
        queue.meter(
            "wheel_tool",
            match self.tool {
                ToolSlot::A => 0,
                ToolSlot::B => 1,
            },
        );
        queue.meter("wheel_overwatch", bool_to_i32(self.overwatch));
        queue.meter("wheel_move_mode", bool_to_i32(self.move_mode));
        queue.meter("wheel_slowmo", bool_to_i32(self.slowmo_enabled));
    }
}

#[derive(Resource, Default, Debug)]
pub struct PauseState {
    pub hard_paused_sp: bool,
}

pub fn bool_to_i32(value: bool) -> i32 {
    if value {
        1
    } else {
        0
    }
}
