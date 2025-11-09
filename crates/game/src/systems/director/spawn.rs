use crate::systems::economy::{Pp, Weather};
use bevy::prelude::Resource;
use serde::Serialize;

use super::config::DirectorCfg;
use super::rng::{spawn_subseed, DetRng};
use crate::world::weather::WeatherConfig;

const DEFAULT_SPAWN_KIND: &str = "bandit";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SpawnBudget {
    pub enemies: u32,
    pub obstacles: u32,
}

impl SpawnBudget {
    pub fn new(enemies: u32, obstacles: u32) -> Self {
        Self { enemies, obstacles }
    }
}

#[derive(Clone, Default)]
pub struct SpawnTypeEntry {
    name: String,
    cumulative_weight: u32,
}

#[derive(Clone, Default)]
pub struct SpawnTypeTable {
    entries: Vec<SpawnTypeEntry>,
    total_weight: u32,
}

impl SpawnTypeTable {
    #[allow(clippy::float_arithmetic)]
    fn from_weights(weights: &std::collections::HashMap<String, f32>) -> Self {
        let mut entries = Vec::new();
        let mut total = 0u32;
        let mut sorted: Vec<_> = weights.iter().collect();
        sorted.sort_by(|(a, _), (b, _)| a.cmp(b));
        for (name, weight) in sorted {
            if *weight <= 0.0 {
                continue;
            }
            let scaled = (*weight * 1000.0).round();
            if scaled <= 0.0 {
                continue;
            }
            total = total.saturating_add(scaled as u32);
            entries.push(SpawnTypeEntry {
                name: name.clone(),
                cumulative_weight: total,
            });
        }
        Self {
            entries,
            total_weight: total,
        }
    }

    pub fn choose(&self, rng: &mut DetRng) -> String {
        if self.entries.is_empty() {
            return DEFAULT_SPAWN_KIND.to_owned();
        }
        let draw = rng.range_u32(0, self.total_weight.saturating_sub(1));
        for entry in &self.entries {
            if draw < entry.cumulative_weight {
                return entry.name.clone();
            }
        }
        self.entries
            .last()
            .map(|entry| entry.name.clone())
            .unwrap_or_else(|| DEFAULT_SPAWN_KIND.to_owned())
    }
}

#[derive(Clone, Default, Resource)]
pub struct SpawnTypeTables {
    fallback: SpawnTypeTable,
    by_weather: std::collections::HashMap<Weather, SpawnTypeTable>,
}

impl SpawnTypeTables {
    pub fn from_cfg(cfg: &DirectorCfg) -> Self {
        use std::collections::HashMap;

        let fallback = cfg
            .types
            .as_ref()
            .map(SpawnTypeTable::from_weights)
            .unwrap_or_default();

        let mut by_weather = HashMap::new();
        if let Some(weather_maps) = &cfg.weather_types {
            for (weather_key, weights) in weather_maps {
                if let Some(weather) = parse_weather(weather_key) {
                    by_weather.insert(weather, SpawnTypeTable::from_weights(weights));
                }
            }
        }

        Self {
            fallback,
            by_weather,
        }
    }

    pub fn table_for(&self, weather: Weather) -> &SpawnTypeTable {
        self.by_weather.get(&weather).unwrap_or(&self.fallback)
    }
}

fn parse_weather(key: &str) -> Option<Weather> {
    match key {
        "Clear" => Some(Weather::Clear),
        "Rains" => Some(Weather::Rains),
        "Fog" => Some(Weather::Fog),
        "Windy" => Some(Weather::Windy),
        _ => None,
    }
}

pub fn compute_spawn_budget(
    pp: Pp,
    weather: Weather,
    prior: Option<u32>,
    cfg: &DirectorCfg,
    weather_config: Option<&WeatherConfig>,
) -> SpawnBudget {
    let pp_band = (pp.0 as i32) / 100;
    let weather_key = format!("{weather:?}");
    let weather_delta = cfg
        .spawn
        .beta_weather
        .get(&weather_key)
        .copied()
        .unwrap_or_default();

    // Start with base calculation
    let mut enemies_raw =
        cfg.spawn.base as i32 + cfg.spawn.alpha_pp_per_100 * pp_band + weather_delta;

    // Apply weather aggression effect if config is available
    if let Some(weather_config) = weather_config {
        let agg_pct = weather_config.get_agg_pct(weather);
        if agg_pct != 0 {
            // Apply percentage as a multiplier: enemies_raw * (100 + agg_pct) / 100
            enemies_raw = (enemies_raw * (100 + agg_pct)) / 100;
        }
    }

    enemies_raw = enemies_raw.max(0);
    let desired = enemies_raw as u32;
    let desired_clamped = desired.clamp(cfg.spawn.clamp_min, cfg.spawn.clamp_max);

    let prior_enemies = prior.unwrap_or(desired_clamped);
    let capped_prior = prior_enemies.clamp(cfg.spawn.clamp_min, cfg.spawn.clamp_max);
    let increase = desired_clamped.saturating_sub(capped_prior);
    let delta = increase.min(cfg.spawn.growth_cap_per_leg);
    let enemies = (capped_prior + delta).clamp(cfg.spawn.clamp_min, cfg.spawn.clamp_max);

    SpawnBudget {
        enemies,
        obstacles: 0,
    }
}

pub fn choose_spawn_type(
    tables: &SpawnTypeTables,
    weather: Weather,
    seed: u64,
    spawn_index: u64,
) -> String {
    let mut rng = DetRng::from_seed(spawn_subseed(seed, spawn_index));
    tables.table_for(weather).choose(&mut rng)
}

pub fn danger_score(
    budget: &SpawnBudget,
    mission_minutes: u32,
    density_per_10k: u32,
    cadence_per_min: u32,
    player_rating_0_100: u8,
) -> i32 {
    let enemies = budget.enemies as i32;
    let density = density_per_10k as i32;
    let cadence = cadence_per_min as i32;
    let minutes = mission_minutes as i32;

    let danger_raw = 1000 * enemies + 400 * density + 300 * cadence + 50 * minutes;
    let rating = i32::from(player_rating_0_100.clamp(0, 100));
    let delta = rating - 50;
    let numerator = danger_raw as i64 * (250 + i64::from(delta));
    ((numerator + 125) / 250) as i32
}

pub fn danger_diff_sign(current: i32, prior: i32) -> i32 {
    match current.cmp(&prior) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::director::config::{DirectorCfg, SpawnCfg};
    use std::collections::HashMap;

    #[test]
    fn spawn_table_prefers_higher_weights() {
        let mut weights = HashMap::new();
        weights.insert("alpha".to_string(), 1.0);
        weights.insert("beta".to_string(), 3.0);
        let table = SpawnTypeTable::from_weights(&weights);
        let mut alpha_hits = 0;
        let mut beta_hits = 0;
        for idx in 0..256 {
            let pick = table.choose(&mut DetRng::from_seed(spawn_subseed(0xABCD_EF01, idx)));
            match pick.as_str() {
                "alpha" => alpha_hits += 1,
                "beta" => beta_hits += 1,
                _ => {}
            }
        }
        assert!(beta_hits > alpha_hits);
    }

    #[test]
    fn tables_fall_back_to_default() {
        let cfg = DirectorCfg {
            spawn: SpawnCfg {
                base: 1,
                alpha_pp_per_100: 0,
                beta_weather: HashMap::new(),
                growth_cap_per_leg: 1,
                clamp_min: 1,
                clamp_max: 1,
            },
            missions: HashMap::new(),
            types: None,
            weather_types: None,
        };
        let tables = SpawnTypeTables::from_cfg(&cfg);
        let pick = choose_spawn_type(&tables, Weather::Clear, 0xDEAD_BEEF, 0);
        assert_eq!(pick, DEFAULT_SPAWN_KIND);
    }

    #[test]
    fn agg_budget_delta() {
        let mut cfg = DirectorCfg {
            spawn: SpawnCfg {
                base: 10,
                alpha_pp_per_100: 0,
                beta_weather: HashMap::new(),
                growth_cap_per_leg: 10,
                clamp_min: 1,
                clamp_max: 100,
            },
            missions: HashMap::new(),
            types: None,
            weather_types: None,
        };
        let weather_config = WeatherConfig::default();
        let pp = Pp(0);

        // Test that different weather produces different spawn budgets due to aggression effects
        let budget_clear =
            compute_spawn_budget(pp, Weather::Clear, None, &cfg, Some(&weather_config));
        let budget_fog = compute_spawn_budget(pp, Weather::Fog, None, &cfg, Some(&weather_config));
        let budget_rains =
            compute_spawn_budget(pp, Weather::Rains, None, &cfg, Some(&weather_config));

        // Clear should have base value (0% effect) - unchanged
        assert_eq!(budget_clear.enemies, 10);

        // With percentage scaling, for small base values the effect might be minimal due to integer division
        // Fog (8%) and Rains (5%) applied to base 10: base * (100 + pct) / 100
        // Fog: 10 * 108 / 100 = 10 (with integer division)
        // Rains: 10 * 105 / 100 = 10 (with integer division)
        // So for small base values, the percentage effect might be 0 in integer math

        // Fog has higher aggression than Rains
        assert!(
            budget_fog.enemies >= budget_rains.enemies,
            "Fog should have higher or equal spawn budget than Rains"
        );

        // Test with a larger base value to see the percentage effect
        cfg.spawn.base = 100;
        let budget_clear_large =
            compute_spawn_budget(pp, Weather::Clear, None, &cfg, Some(&weather_config));

        // Clear should have base value (0% effect) - unchanged
        assert_eq!(budget_clear_large.enemies, 100);

        // Fog (8%): 100 * 108 / 100 = 108, but then clamped to cfg.spawn.clamp_max of 100
        // Rains (5%): 100 * 105 / 100 = 105, but then clamped to cfg.spawn.clamp_max of 100
        // So with current clamp_max of 100, both may be clamped to 100.
        // Let's use higher clamp values to see the effect
        let mut cfg_high_clamp = cfg;
        cfg_high_clamp.spawn.clamp_max = 200; // Higher clamp to see actual effect

        let budget_clear_unclamped = compute_spawn_budget(
            pp,
            Weather::Clear,
            None,
            &cfg_high_clamp,
            Some(&weather_config),
        );
        let budget_fog_unclamped = compute_spawn_budget(
            pp,
            Weather::Fog,
            None,
            &cfg_high_clamp,
            Some(&weather_config),
        );
        let budget_rains_unclamped = compute_spawn_budget(
            pp,
            Weather::Rains,
            None,
            &cfg_high_clamp,
            Some(&weather_config),
        );

        // Clear should have base value (0% effect) - unchanged
        assert_eq!(budget_clear_unclamped.enemies, 100);

        // Fog (8%): 100 * 108 / 100 = 108
        // Rains (5%): 100 * 105 / 100 = 105
        assert_eq!(
            budget_fog_unclamped.enemies, 108,
            "Fog should increase spawn budget by 8%"
        );
        assert_eq!(
            budget_rains_unclamped.enemies, 105,
            "Rains should increase spawn budget by 5%"
        );
        assert!(
            budget_fog_unclamped.enemies > budget_clear_unclamped.enemies,
            "Fog weather should increase spawn budget"
        );
        assert!(
            budget_rains_unclamped.enemies > budget_clear_unclamped.enemies,
            "Rains weather should increase spawn budget"
        );
    }
}
