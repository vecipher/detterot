use std::collections::HashMap;
use std::path::Path;

use anyhow::Context;
use serde::Deserialize;

use crate::systems::economy::Weather;

use bevy::prelude::Resource;

#[derive(Debug, Clone, Deserialize, Resource)]
#[serde(deny_unknown_fields)]
pub struct WeatherConfig {
    #[serde(default)]
    pub defaults: HashMap<String, Weather>,
    #[serde(default)]
    pub overrides: HashMap<String, Weather>,
    #[serde(default)]
    pub effects: WeatherEffects,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeatherEffects {
    #[serde(default = "default_clear_los")]
    pub clear_los_m: u32,
    #[serde(default = "default_rains_los")]
    pub rains_los_m: u32,
    #[serde(default = "default_fog_los")]
    pub fog_los_m: u32,
    #[serde(default = "default_windy_los")]
    pub windy_los_m: u32,

    #[serde(default = "default_clear_drift")]
    pub clear_drift_mm: u32,
    #[serde(default = "default_rains_drift")]
    pub rains_drift_mm: u32,
    #[serde(default = "default_fog_drift")]
    pub fog_drift_mm: u32,
    #[serde(default = "default_windy_drift")]
    pub windy_drift_mm: u32,

    #[serde(default = "default_clear_agg")]
    pub clear_agg_pct: i32,
    #[serde(default = "default_rains_agg")]
    pub rains_agg_pct: i32,
    #[serde(default = "default_fog_agg")]
    pub fog_agg_pct: i32,
    #[serde(default = "default_windy_agg")]
    pub windy_agg_pct: i32,
}

impl Default for WeatherEffects {
    fn default() -> Self {
        Self {
            clear_los_m: default_clear_los(),
            rains_los_m: default_rains_los(),
            fog_los_m: default_fog_los(),
            windy_los_m: default_windy_los(),
            clear_drift_mm: default_clear_drift(),
            rains_drift_mm: default_rains_drift(),
            fog_drift_mm: default_fog_drift(),
            windy_drift_mm: default_windy_drift(),
            clear_agg_pct: default_clear_agg(),
            rains_agg_pct: default_rains_agg(),
            fog_agg_pct: default_fog_agg(),
            windy_agg_pct: default_windy_agg(),
        }
    }
}

impl Default for WeatherConfig {
    fn default() -> Self {
        Self {
            defaults: HashMap::new(),
            overrides: HashMap::new(),
            effects: WeatherEffects::default(),
        }
    }
}

fn default_clear_los() -> u32 {
    1000
}
fn default_rains_los() -> u32 {
    800
}
fn default_fog_los() -> u32 {
    600
}
fn default_windy_los() -> u32 {
    900
}

fn default_clear_drift() -> u32 {
    0
}
fn default_rains_drift() -> u32 {
    20
}
fn default_fog_drift() -> u32 {
    0
}
fn default_windy_drift() -> u32 {
    35
}

fn default_clear_agg() -> i32 {
    0
}
fn default_rains_agg() -> i32 {
    5
}
fn default_fog_agg() -> i32 {
    8
}
fn default_windy_agg() -> i32 {
    3
}

impl WeatherConfig {
    pub fn load_from_path(path: &Path) -> anyhow::Result<Self> {
        let raw =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
    }

    pub fn resolve_weather(&self, style: &str, link_id: &str) -> Weather {
        // Check for overrides first
        if let Some(override_weather) = self.overrides.get(link_id) {
            return *override_weather;
        }

        // Check defaults for the style
        self.defaults.get(style).copied().unwrap_or(Weather::Clear)
    }

    pub fn get_los_m(&self, weather: Weather) -> u32 {
        match weather {
            Weather::Clear => self.effects.clear_los_m,
            Weather::Rains => self.effects.rains_los_m,
            Weather::Fog => self.effects.fog_los_m,
            Weather::Windy => self.effects.windy_los_m,
        }
    }

    pub fn get_drift_mm(&self, weather: Weather) -> u32 {
        match weather {
            Weather::Clear => self.effects.clear_drift_mm,
            Weather::Rains => self.effects.rains_drift_mm,
            Weather::Fog => self.effects.fog_drift_mm,
            Weather::Windy => self.effects.windy_drift_mm,
        }
    }

    pub fn get_agg_pct(&self, weather: Weather) -> i32 {
        match weather {
            Weather::Clear => self.effects.clear_agg_pct,
            Weather::Rains => self.effects.rains_agg_pct,
            Weather::Fog => self.effects.fog_agg_pct,
            Weather::Windy => self.effects.windy_agg_pct,
        }
    }
}

pub fn load_weather_config(path: &Path) -> anyhow::Result<WeatherConfig> {
    WeatherConfig::load_from_path(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::economy::Weather;

    #[test]
    fn weather_resolve() {
        let config = WeatherConfig {
            defaults: vec![
                ("coast".to_string(), Weather::Rains),
                ("ridge".to_string(), Weather::Windy),
                ("wetland".to_string(), Weather::Fog),
            ]
            .into_iter()
            .collect(),
            overrides: vec![("L01".to_string(), Weather::Fog)]
                .into_iter()
                .collect(),
            effects: WeatherEffects::default(),
        };

        // Test default mapping
        assert_eq!(config.resolve_weather("coast", "L02"), Weather::Rains);
        assert_eq!(config.resolve_weather("ridge", "L03"), Weather::Windy);
        assert_eq!(config.resolve_weather("wetland", "L04"), Weather::Fog);

        // Test override precedence
        assert_eq!(config.resolve_weather("coast", "L01"), Weather::Fog); // Override wins

        // Test unknown style defaults to Clear
        assert_eq!(
            config.resolve_weather("unknown_style", "L05"),
            Weather::Clear
        );
    }

    #[test]
    fn weather_effects_access() {
        let config = WeatherConfig::default();

        // Verify the defaults match spec
        assert_eq!(config.get_los_m(Weather::Clear), 1000);
        assert_eq!(config.get_los_m(Weather::Rains), 800);
        assert_eq!(config.get_los_m(Weather::Fog), 600);
        assert_eq!(config.get_los_m(Weather::Windy), 900);

        assert_eq!(config.get_drift_mm(Weather::Clear), 0);
        assert_eq!(config.get_drift_mm(Weather::Rains), 20);
        assert_eq!(config.get_drift_mm(Weather::Fog), 0);
        assert_eq!(config.get_drift_mm(Weather::Windy), 35);

        assert_eq!(config.get_agg_pct(Weather::Clear), 0);
        assert_eq!(config.get_agg_pct(Weather::Rains), 5);
        assert_eq!(config.get_agg_pct(Weather::Fog), 8);
        assert_eq!(config.get_agg_pct(Weather::Windy), 3);
    }

    #[test]
    fn unknown_fields_rejected() {
        let bad_toml = r#"
            [defaults]
            coast = "Rains"
            
            [effects]
            clear_los_m = 1000
            unknown_field = 42
        "#;

        let result = toml::from_str::<WeatherConfig>(bad_toml);
        assert!(result.is_err(), "Expected unknown field to cause error");
    }
}
