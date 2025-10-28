use crate::systems::economy::RouteId;

const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

#[inline]
pub fn hash_mission_name(name: &str) -> u64 {
    name.bytes().fold(FNV_OFFSET_BASIS, |mut hash, byte| {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        hash
    })
}

#[inline]
pub fn mission_seed(world_seed: u64, link_id: RouteId, day: u32, mission_id: u64) -> u64 {
    let mut key = [0u8; 32];
    key[0..8].copy_from_slice(&world_seed.to_le_bytes());
    key[8..16].copy_from_slice(&(link_id.0 as u64).to_le_bytes());
    key[16..24].copy_from_slice(&(day as u64).to_le_bytes());
    key[24..32].copy_from_slice(&mission_id.to_le_bytes());
    wyhash::wyhash(&key, 0)
}

#[inline]
pub fn spawn_subseed(seed64: u64, spawn_index: u64) -> u64 {
    let mut state = seed64 ^ spawn_index;
    splitmix64(&mut state)
}

#[inline]
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

#[derive(Clone)]
pub struct DetRng {
    state: u64,
}

impl DetRng {
    pub fn from_seed(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u32(&mut self) -> u32 {
        splitmix64(&mut self.state) as u32
    }

    pub fn next_bool(&mut self) -> bool {
        self.next_u32() & 1 == 1
    }

    pub fn range_u32(&mut self, low: u32, high: u32) -> u32 {
        debug_assert!(low <= high);
        let span = high - low;
        if span == 0 {
            return low;
        }
        let draw = self.next_u32() % (span + 1);
        low + draw
    }

    pub fn range_i32(&mut self, low: i32, high: i32) -> i32 {
        debug_assert!(low <= high);
        let span = (high - low) as u32;
        low + (self.next_u32() % (span + 1)) as i32
    }

    pub fn split_for_spawn(&mut self, index: u64) -> Self {
        let seed = spawn_subseed(self.state, index);
        Self::from_seed(seed)
    }
}
