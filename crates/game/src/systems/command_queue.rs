use bevy::prelude::Resource;
use repro::{Command, CommandKind, MeterCommand, SpawnCommand};

/// Buffer of deterministic commands emitted during gameplay. The queue is
/// flushed when the record writer commits a new tick to disk.
#[derive(Resource, Default)]
pub struct CommandQueue {
    pub buf: Vec<Command>,
    current_tick: u32,
}

impl CommandQueue {
    /// Set the active tick before running FixedUpdate systems.
    pub fn begin_tick(&mut self, tick: u32) {
        self.current_tick = tick;
    }

    /// Queue a spawn command. Positions are recorded in millimetres to avoid
    /// floating point drift in deterministic replays.
    pub fn spawn(&mut self, kind: &str, x_mm: i32, y_mm: i32, z_mm: i32) {
        self.buf.push(Command {
            t: self.current_tick,
            kind: CommandKind::Spawn(SpawnCommand {
                kind: kind.to_owned(),
                x_mm,
                y_mm,
                z_mm,
            }),
        });
    }

    /// Queue a metric update for downstream analytics.
    pub fn meter(&mut self, key: &str, value: i32) {
        self.buf.push(Command {
            t: self.current_tick,
            kind: CommandKind::Meter(MeterCommand {
                key: key.to_owned(),
                value,
            }),
        });
    }

    /// Convenience helper for recording unit counts without leaking u32 into
    /// the deterministic command stream format.
    pub fn meter_units(&mut self, key: &str, units: u32) {
        let clamped = units.min(i32::MAX as u32) as i32;
        self.meter(key, clamped);
    }

    /// Drain the queue, returning all buffered commands.
    pub fn drain(&mut self) -> Vec<Command> {
        std::mem::take(&mut self.buf)
    }
}
