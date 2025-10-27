use bevy::prelude::*;

use repro::{Command, CommandKind};

/// Buffer of deterministic commands emitted during simulation.
#[derive(Resource, Default)]
pub struct CommandQueue {
    pub buf: Vec<Command>,
    current_tick: u32,
}

impl CommandQueue {
    pub fn set_tick(&mut self, tick: u32) {
        self.current_tick = tick;
    }

    pub fn spawn(&mut self, kind: &str, x_mm: i32, y_mm: i32, z_mm: i32) {
        self.buf.push(Command {
            t: self.current_tick,
            kind: CommandKind::Spawn {
                kind: kind.to_owned(),
                x_mm,
                y_mm,
                z_mm,
            },
        });
    }

    pub fn meter(&mut self, key: &str, value: i32) {
        self.buf.push(Command {
            t: self.current_tick,
            kind: CommandKind::Meter {
                key: key.to_owned(),
                value,
            },
        });
    }

    pub fn drain(&mut self) -> Vec<Command> {
        std::mem::take(&mut self.buf)
    }
}
