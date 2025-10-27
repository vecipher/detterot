use crate::systems::economy::types::{Pp, Weather};

use super::config::DirectorCfg;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpawnBudget {
    pub enemies: u32,
    pub obstacles: u32,
}

impl SpawnBudget {
    pub fn empty() -> Self {
        Self {
            enemies: 0,
            obstacles: 0,
        }
    }
}

pub fn compute_spawn_budget(
    pp: Pp,
    weather: Weather,
    prior: Option<u32>,
    cfg: &DirectorCfg,
) -> SpawnBudget {
    let spawn_cfg = &cfg.spawn;
    let pp_step = (pp.0 as i32) / 100;
    let weather_key = weather_key(weather);
    let weather_delta = spawn_cfg
        .beta_weather
        .get(weather_key)
        .copied()
        .unwrap_or_default();

    let mut enemies_raw =
        spawn_cfg.base as i32 + spawn_cfg.alpha_pp_per_100 * pp_step + weather_delta;

    if enemies_raw < 0 {
        enemies_raw = 0;
    }

    let prior_enemies = prior.map(|v| v as i32).unwrap_or(enemies_raw);
    let delta = enemies_raw - prior_enemies;
    let capped_delta = if delta > 0 {
        delta.min(spawn_cfg.growth_cap_per_leg as i32)
    } else {
        0
    };

    let next = (prior_enemies + capped_delta)
        .clamp(spawn_cfg.clamp_min as i32, spawn_cfg.clamp_max as i32);

    SpawnBudget {
        enemies: next as u32,
        obstacles: 0,
    }
}

pub fn weather_key(weather: Weather) -> &'static str {
    match weather {
        Weather::Clear => "Clear",
        Weather::Rains => "Rains",
        Weather::Fog => "Fog",
        Weather::Windy => "Windy",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::director::config::SpawnCfg;
    use crate::systems::economy::types::Pp;

    fn cfg() -> DirectorCfg {
        DirectorCfg {
            spawn: SpawnCfg {
                base: 8,
                alpha_pp_per_100: 5,
                beta_weather: [
                    ("Clear".to_string(), 0),
                    ("Rains".to_string(), 4),
                    ("Fog".to_string(), 6),
                    ("Windy".to_string(), 3),
                ]
                .into_iter()
                .collect(),
                growth_cap_per_leg: 8,
                clamp_min: 2,
                clamp_max: 40,
            },
            missions: Default::default(),
            types: None,
            weather_types: None,
        }
    }

    #[test]
    fn pp_increase_is_monotonic_across_weather() {
        let cfg = cfg();
        let pps = [0u16, 100, 200, 300, 400];
        for &weather in &[Weather::Clear, Weather::Rains, Weather::Fog, Weather::Windy] {
            let mut prior_count: Option<u32> = None;
            for &pp in &pps {
                let budget = compute_spawn_budget(Pp(pp), weather, prior_count, &cfg);
                let enemies = budget.enemies;
                assert!(
                    enemies >= cfg.spawn.clamp_min,
                    "enemies should respect clamp_min for {:?}",
                    weather
                );
                assert!(
                    enemies <= cfg.spawn.clamp_max,
                    "enemies should respect clamp_max for {:?}",
                    weather
                );
                if let Some(prev) = prior_count {
                    assert!(
                        enemies >= prev,
                        "spawn budget must be monotonic for {:?} at {pp} PP",
                        weather
                    );
                    let delta = enemies - prev;
                    assert!(
                        delta <= cfg.spawn.growth_cap_per_leg,
                        "growth cap exceeded for {:?}: {} > {}",
                        weather,
                        delta,
                        cfg.spawn.growth_cap_per_leg
                    );
                }
                prior_count = Some(enemies);
            }
        }
    }

    #[test]
    fn harsher_weather_increases_budget_when_available() {
        let cfg = cfg();
        // Start from a fresh leg so that growth-cap smoothing does not mask deltas.
        let clear = compute_spawn_budget(Pp(0), Weather::Clear, None, &cfg).enemies;
        let rains = compute_spawn_budget(Pp(0), Weather::Rains, Some(clear), &cfg).enemies;
        let fog = compute_spawn_budget(Pp(0), Weather::Fog, Some(clear), &cfg).enemies;
        let windy = compute_spawn_budget(Pp(0), Weather::Windy, Some(clear), &cfg).enemies;
        assert!(rains >= clear, "Rains weather should not reduce budget");
        assert!(
            fog >= rains,
            "Fog weather should be at least as harsh as Rains"
        );
        assert!(windy >= clear, "Windy weather should not reduce budget");
    }
}
