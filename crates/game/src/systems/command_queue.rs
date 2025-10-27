use bevy::prelude::Resource;
use repro::Command;

/// Buffer accumulating deterministic outputs each fixed tick.
#[derive(Resource, Default, Debug)]
pub struct CommandQueue {
    pub buf: Vec<Command>,
}

impl CommandQueue {
    pub fn spawn(&mut self, kind: &str, x_mm: i32, y_mm: i32, z_mm: i32) {
        self.buf.push(Command::spawn(kind, x_mm, y_mm, z_mm));
    }

    pub fn meter(&mut self, key: &str, value: i32) {
        self.buf.push(Command::meter(key, value));
    }

    pub fn drain(&mut self) -> impl Iterator<Item = Command> + '_ {
        self.buf.drain(..)
    }
}
