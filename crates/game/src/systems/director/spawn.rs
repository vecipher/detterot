use crate::systems::economy::{Pp, RouteId, Weather};

use super::config::{weather_key, DirectorCfg};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpawnBudget {
    pub enemies: u32,
    pub obstacles: u32,
}

pub fn compute_spawn_budget(
    pp: Pp,
    weather: Weather,
    prior: Option<u32>,
    cfg: &DirectorCfg,
) -> SpawnBudget {
    let base = cfg.spawn.base as i32;
    let pp_term = cfg.spawn.alpha_pp_per_100 * (i32::from(pp.0) / 100);
    let weather_delta = cfg
        .spawn
        .beta_weather
        .get(weather_key(weather))
        .copied()
        .unwrap_or(0);
    let enemies_raw = base + pp_term + weather_delta;

    let prior_enemies = prior
        .map(|p| p as i32)
        .unwrap_or(cfg.spawn.clamp_min as i32);
    let delta = enemies_raw - prior_enemies;
    let capped_delta = delta.min(cfg.spawn.growth_cap_per_leg as i32);
    let next = prior_enemies + capped_delta.max(0);
    let clamped = next
        .clamp(cfg.spawn.clamp_min as i32, cfg.spawn.clamp_max as i32)
        .max(0);

    SpawnBudget {
        enemies: clamped as u32,
        obstacles: 0,
    }
}

pub fn danger_score(
    budget: &SpawnBudget,
    mission_minutes: u32,
    density_per_10k: u32,
    cadence_per_min: u32,
    player_rating_0_100: u8,
) -> i32 {
    let enemies_term = 1000 * budget.enemies as i32;
    let density_term = 400 * density_per_10k as i32;
    let cadence_term = 300 * cadence_per_min as i32;
    let duration_term = 50 * mission_minutes as i32;
    let raw = enemies_term + density_term + cadence_term + duration_term;

    let rating = player_rating_0_100.min(100).max(0) as i32;
    let factor = 1.0 + 0.2 * (rating - 50) as f32 / 50.0;
    (raw as f32 * factor).round() as i32
}

pub fn danger_diff_sign(current: i32, prior: i32) -> i32 {
    match current.cmp(&prior) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

pub fn wyhash64(seed: u64, link_id: RouteId, day: u32, salt: u64) -> u64 {
    let mix = seed ^ ((link_id.0 as u64) << 16) ^ ((day as u64) << 32) ^ salt;
    mix64(mix)
}

fn mix64(mut x: u64) -> u64 {
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51afd7ed558ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ceb9fe1a85ec53);
    x ^ (x >> 33)
}

pub fn split_seed(seed: u64, index: u64) -> u64 {
    splitmix64(seed ^ index)
}

fn splitmix64(mut state: u64) -> u64 {
    state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

#[derive(Debug, Clone, Copy)]
pub struct DetRng {
    state: u64,
}

impl DetRng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        let result = splitmix64(self.state);
        self.state = result;
        result
    }

    pub fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    pub fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as i32
    }

    pub fn range_i32(&mut self, min: i32, max: i32) -> i32 {
        debug_assert!(min <= max);
        if min == max {
            return min;
        }
        let span = (max as i64 - min as i64) as u64 + 1;
        let value = self.next_u64() % span;
        min + value as i32
    }
}

pub fn select_spawn_kind(cfg: &DirectorCfg, weather: Weather, rng: &mut DetRng) -> Option<String> {
    let table = cfg
        .weather_types
        .as_ref()
        .and_then(|map| map.get(weather_key(weather)))
        .or_else(|| cfg.types.as_ref());
    let table = match table {
        Some(table) => table,
        None => return None,
    };

    let mut weighted: Vec<(&String, u64)> = table
        .iter()
        .map(|(kind, weight)| (kind, weight_to_int(*weight)))
        .collect();
    if weighted.is_empty() {
        return None;
    }
    weighted.sort_by(|a, b| a.0.cmp(b.0));
    let total: u64 = weighted.iter().map(|(_, w)| *w).sum();
    if total == 0 {
        return Some(weighted[0].0.clone());
    }
    let pick = rng.next_u64() % total;
    let mut accum = 0u64;
    for (kind, weight) in weighted {
        accum += weight;
        if pick < accum {
            return Some(kind.clone());
        }
    }
    None
}

fn weight_to_int(weight: f32) -> u64 {
    if weight <= 0.0 {
        return 0;
    }
    let scaled = (weight * 1000.0).round();
    if scaled <= 0.0 {
        0
    } else {
        scaled as u64
    }
}

pub fn spawn_position(seed: u64, index: u32) -> (i32, i32, i32) {
    let mut rng = DetRng::new(split_seed(seed, index as u64));
    let x = rng.range_i32(-5000, 5000);
    let y = rng.range_i32(-5000, 5000);
    let z = rng.range_i32(0, 1000);
    (x, y, z)
}
