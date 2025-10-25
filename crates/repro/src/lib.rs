use serde::{Deserialize, Serialize};

/// Minimal deterministic RNG wrapper (32-bit LCG).
#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct DetRng {
    state: u32,
}
impl DetRng {
    pub fn from_seed(seed: u64) -> Self {
        Self {
            state: (seed as u32).wrapping_mul(747796405) ^ 2891336453,
        }
    }
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        self.state
    }
    #[inline]
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() as f32) / (u32::MAX as f32)
    }
}

/// Command stream we can record/replay (expand later).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    Spawn {
        kind: String,
        x: f32,
        y: f32,
        z: f32,
    },
    Meter {
        key: String,
        value: i32,
    },
    // ...
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub world_seed: u64,
    pub link_id: String,
    pub rulepack: String,
    pub weather: String,
    pub rng_salt: u64,
    pub commands: Vec<Command>,
}

pub fn hash_record(rec: &Record) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&serde_json::to_vec(rec).unwrap());
    hasher.finalize().to_hex().to_string()
}
