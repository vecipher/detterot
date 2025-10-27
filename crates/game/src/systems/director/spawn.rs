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
    use crate::systems::economy::types::Pp;

    use super::*;
    use crate::systems::director::config::SpawnCfg;

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
    fn monotonic_with_pp() {
        let cfg = cfg();
        let base = compute_spawn_budget(Pp(0), Weather::Clear, None, &cfg).enemies;
        let higher = compute_spawn_budget(Pp(100), Weather::Clear, Some(base), &cfg).enemies;
        assert!(higher >= base);
    }
}
