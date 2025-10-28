use bevy::prelude::Resource;

use crate::systems::command_queue::CommandQueue;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Stance {
    #[default]
    Brace,
    Vault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToolSlot {
    #[default]
    A,
    B,
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WheelState {
    pub stance: Stance,
    pub tool: ToolSlot,
    pub overwatch: bool,
    pub move_mode: bool,
    pub slowmo_enabled: bool,
}

impl WheelState {
    pub fn set_stance(&mut self, queue: &mut CommandQueue, stance: Stance) {
        if self.stance != stance {
            self.stance = stance;
            queue.meter(
                "wheel_stance",
                match stance {
                    Stance::Brace => 0,
                    Stance::Vault => 1,
                },
            );
        }
    }

    pub fn set_tool(&mut self, queue: &mut CommandQueue, tool: ToolSlot) {
        if self.tool != tool {
            self.tool = tool;
            queue.meter(
                "wheel_tool",
                match tool {
                    ToolSlot::A => 0,
                    ToolSlot::B => 1,
                },
            );
        }
    }

    pub fn set_overwatch(&mut self, queue: &mut CommandQueue, enabled: bool) {
        if self.overwatch != enabled {
            self.overwatch = enabled;
            queue.meter("wheel_overwatch", enabled as i32);
        }
    }

    pub fn set_move_mode(&mut self, queue: &mut CommandQueue, enabled: bool) {
        if self.move_mode != enabled {
            self.move_mode = enabled;
            queue.meter("wheel_move", enabled as i32);
        }
    }

    pub fn set_slowmo(&mut self, _queue: &mut CommandQueue, enabled: bool) {
        if self.slowmo_enabled != enabled {
            self.slowmo_enabled = enabled;
        }
    }
}

#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PauseState {
    pub hard_paused_sp: bool,
}

impl PauseState {
    pub fn set_hard_pause(&mut self, queue: &mut CommandQueue, paused: bool) {
        if self.hard_paused_sp != paused {
            self.hard_paused_sp = paused;
            queue.meter("wheel_hard_pause", paused as i32);
        }
    }
}
