#![allow(dead_code)]

use blake3::Hasher;
use rand_core::{RngCore, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;

use super::{BasisBp, EconomyDay, HubId};

#[derive(Clone)]
pub struct DetRng {
    rng: Xoshiro256PlusPlus,
    draws: u32,
}

impl DetRng {
    pub fn from_seed(
        world_seed: u64,
        econ_version: u32,
        hub: HubId,
        day: EconomyDay,
        tag: u32,
    ) -> Self {
        Self::from_seed_inner(world_seed, econ_version, Some(hub), day, tag)
    }

    pub fn from_seed_global(world_seed: u64, econ_version: u32, day: EconomyDay, tag: u32) -> Self {
        Self::from_seed_inner(world_seed, econ_version, None, day, tag)
    }

    fn from_seed_inner(
        world_seed: u64,
        econ_version: u32,
        hub: Option<HubId>,
        day: EconomyDay,
        tag: u32,
    ) -> Self {
        let mut hasher = Hasher::new();
        hasher.update(b"det_rng_v1");
        hasher.update(&world_seed.to_le_bytes());
        hasher.update(&econ_version.to_le_bytes());
        if let Some(hub) = hub {
            hasher.update(&hub.0.to_le_bytes());
        }
        hasher.update(&day.0.to_le_bytes());
        hasher.update(&tag.to_le_bytes());
        let hash = hasher.finalize();
        let mut seed = [0u8; 32];
        seed.copy_from_slice(hash.as_bytes());
        Self {
            rng: Xoshiro256PlusPlus::from_seed(seed),
            draws: 0,
        }
    }

    pub fn u32(&mut self) -> u32 {
        self.draws = self.draws.saturating_add(1);
        self.rng.next_u32()
    }

    pub fn norm_bounded_bp(&mut self, mu_bp: i32, sigma_bp: u32, clamp_bp: i32) -> BasisBp {
        const SAMPLE_COUNT: usize = 6;
        const SAMPLE_MASK: u32 = 0xFFFF;
        const SAMPLE_HALF: i64 = (SAMPLE_MASK as i64 + 1) / 2; // 32768
        const NORMALIZER: i64 = 46341; // ~= sqrt(SAMPLE_COUNT) * SAMPLE_HALF / sqrt(3)

        let mut acc: i64 = 0;
        for _ in 0..SAMPLE_COUNT {
            let draw = (self.u32() & SAMPLE_MASK) as i64 - SAMPLE_HALF;
            acc += draw;
        }

        let scaled = if sigma_bp == 0 {
            0
        } else {
            (acc * sigma_bp as i64) / NORMALIZER
        };

        let value = mu_bp as i64 + scaled;
        let clamp = clamp_bp.unsigned_abs() as i64;
        let clamped = value.clamp(-clamp, clamp);

        BasisBp(clamped as i32)
    }

    pub fn cursor(&self) -> u32 {
        self.draws
    }
}
