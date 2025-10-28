use bevy::input::{keyboard::KeyCode, ButtonInput};
use bevy::prelude::*;

use crate::systems::command_queue::CommandQueue;

use super::pause_wheel::{PauseState, Stance, ToolSlot, WheelState};
use super::LegContext;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WheelInputAction {
    SetStance(Stance),
    SetTool(ToolSlot),
    SetOverwatch(bool),
    SetMoveMode(bool),
    SetSlowmo(bool),
    SetHardPause(bool),
}

#[derive(Resource, Default, Debug)]
pub struct WheelInputQueue {
    actions: Vec<WheelInputAction>,
}

impl WheelInputQueue {
    pub fn push(&mut self, action: WheelInputAction) {
        self.actions.push(action);
    }

    pub fn extend<I: IntoIterator<Item = WheelInputAction>>(&mut self, iter: I) {
        self.actions.extend(iter);
    }

    pub fn take(&mut self) -> Vec<WheelInputAction> {
        std::mem::take(&mut self.actions)
    }
}

pub fn apply_wheel_inputs(
    mut wheel: ResMut<WheelState>,
    mut pause: ResMut<PauseState>,
    mut command_queue: ResMut<CommandQueue>,
    mut input_queue: ResMut<WheelInputQueue>,
    context: Option<Res<LegContext>>,
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
) {
    let allow_slowmo = context.as_ref().map(|c| !c.multiplayer).unwrap_or(true);
    let allow_hard_pause = allow_slowmo;

    for action in input_queue.take() {
        match action {
            WheelInputAction::SetStance(stance) => {
                wheel.set_stance(&mut command_queue, stance);
            }
            WheelInputAction::SetTool(tool) => {
                wheel.set_tool(&mut command_queue, tool);
            }
            WheelInputAction::SetOverwatch(enabled) => {
                wheel.set_overwatch(&mut command_queue, enabled);
            }
            WheelInputAction::SetMoveMode(enabled) => {
                wheel.set_move_mode(&mut command_queue, enabled);
            }
            WheelInputAction::SetSlowmo(enabled) => {
                if allow_slowmo {
                    wheel.set_slowmo(&mut command_queue, enabled);
                }
            }
            WheelInputAction::SetHardPause(enabled) => {
                if allow_hard_pause {
                    pause.set_hard_pause(&mut command_queue, enabled);
                }
            }
        }
    }

    if let Some(keys) = keyboard {
        let stance = if keys.pressed(KeyCode::Digit2) {
            Some(Stance::Vault)
        } else if keys.pressed(KeyCode::Digit1) {
            Some(Stance::Brace)
        } else {
            None
        };
        if let Some(stance) = stance {
            wheel.set_stance(&mut command_queue, stance);
        }

        let tool = if keys.pressed(KeyCode::Digit4) {
            Some(ToolSlot::B)
        } else if keys.pressed(KeyCode::Digit3) {
            Some(ToolSlot::A)
        } else {
            None
        };
        if let Some(tool) = tool {
            wheel.set_tool(&mut command_queue, tool);
        }

        if keys.pressed(KeyCode::KeyO) {
            wheel.set_overwatch(&mut command_queue, true);
        } else if keys.just_released(KeyCode::KeyO) {
            wheel.set_overwatch(&mut command_queue, false);
        }

        if keys.pressed(KeyCode::KeyM) {
            wheel.set_move_mode(&mut command_queue, true);
        } else if keys.just_released(KeyCode::KeyM) {
            wheel.set_move_mode(&mut command_queue, false);
        }

        if allow_slowmo {
            if keys.pressed(KeyCode::KeyL) {
                wheel.set_slowmo(&mut command_queue, true);
            } else if keys.just_released(KeyCode::KeyL) {
                wheel.set_slowmo(&mut command_queue, false);
            }
        }

        if allow_hard_pause {
            if keys.pressed(KeyCode::Space) {
                pause.set_hard_pause(&mut command_queue, true);
            } else if keys.just_released(KeyCode::Space) {
                pause.set_hard_pause(&mut command_queue, false);
            }
        }
    }
}
