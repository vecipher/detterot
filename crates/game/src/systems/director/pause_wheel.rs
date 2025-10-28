use bevy::prelude::*;
use bevy::time::Virtual;

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
    pub fn set_stance(&mut self, stance: Stance, queue: &mut CommandQueue) {
        if self.stance != stance {
            self.stance = stance;
            queue.meter("wheel_stance", stance_meter_value(stance));
        }
    }

    pub fn set_tool(&mut self, slot: ToolSlot, queue: &mut CommandQueue) {
        if self.tool != slot {
            self.tool = slot;
            queue.meter(
                "wheel_tool",
                match slot {
                    ToolSlot::A => 0,
                    ToolSlot::B => 1,
                },
            );
        }
    }

    pub fn set_overwatch(&mut self, enabled: bool, queue: &mut CommandQueue) {
        if self.overwatch != enabled {
            self.overwatch = enabled;
            queue.meter("wheel_overwatch", enabled as i32);
        }
    }

    pub fn set_move_mode(&mut self, enabled: bool, queue: &mut CommandQueue) {
        if self.move_mode != enabled {
            self.move_mode = enabled;
            queue.meter("wheel_move_mode", enabled as i32);
        }
    }

    pub fn set_slowmo(&mut self, enabled: bool, queue: &mut CommandQueue) {
        if self.slowmo_enabled != enabled {
            self.slowmo_enabled = enabled;
            queue.meter("wheel_slowmo", enabled as i32);
        }
    }
}

fn stance_meter_value(stance: Stance) -> i32 {
    match stance {
        Stance::Brace => 0,
        Stance::Vault => 1,
    }
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PauseState {
    pub hard_paused_sp: bool,
}

impl PauseState {
    pub fn set_hard_pause(&mut self, paused: bool, queue: &mut CommandQueue) {
        if self.hard_paused_sp != paused {
            self.hard_paused_sp = paused;
            queue.meter("wheel_hard_pause", paused as i32);
        }
    }
}

pub fn apply_slowmo_time(mut time: ResMut<Time<Virtual>>, wheel: Res<WheelState>) {
    let target_speed = match wheel.slowmo_enabled {
        true => 0.8_f32,
        false => 1.0_f32,
    };

    if time.relative_speed().to_bits() != target_speed.to_bits() {
        time.set_relative_speed(target_speed);
    }
}
