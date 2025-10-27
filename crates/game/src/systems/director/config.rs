use std::collections::HashMap;
use std::fs;

use serde::Deserialize;
use thiserror::Error;

use crate::systems::economy::Weather;

#[derive(Debug, Deserialize, Clone)]
pub struct DirectorCfg {
    pub spawn: SpawnCfg,
    pub missions: HashMap<String, MissionCfg>,
    #[serde(default)]
    pub types: Option<HashMap<String, f32>>,
    #[serde(default)]
    pub weather_types: Option<HashMap<String, HashMap<String, f32>>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SpawnCfg {
    pub base: u32,
    pub alpha_pp_per_100: i32,
    pub beta_weather: HashMap<String, i32>,
    pub growth_cap_per_leg: u32,
    pub clamp_min: u32,
    pub clamp_max: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MissionCfg {
    pub pp_success: i16,
    pub pp_fail: i16,
    #[serde(default)]
    pub basis_bp_success: i16,
    #[serde(default)]
    pub basis_bp_fail: i16,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read director config: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse director config: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("invalid director config: {0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, ConfigError>;

pub fn load_director_cfg(path: &str) -> Result<DirectorCfg> {
    let contents = fs::read_to_string(path)?;
    let cfg: DirectorCfg = toml::from_str(&contents)?;
    validate_spawn_weather(&cfg.spawn.beta_weather)?;
    validate_weather_map(cfg.weather_types.as_ref())?;
    Ok(cfg)
}

fn validate_spawn_weather(map: &HashMap<String, i32>) -> Result<()> {
    let expected = ["Clear", "Rains", "Fog", "Windy"];
    for key in expected.iter() {
        if !map.contains_key(*key) {
            return Err(ConfigError::Invalid(format!("missing weather key {key}")));
        }
    }
    Ok(())
}

fn validate_weather_map(map: Option<&HashMap<String, HashMap<String, f32>>>) -> Result<()> {
    if let Some(map) = map {
        for (weather, table) in map {
            if table.is_empty() {
                return Err(ConfigError::Invalid(format!(
                    "weather {weather} has empty weight table"
                )));
            }
        }
    }
    Ok(())
}

pub fn weather_key(weather: Weather) -> &'static str {
    match weather {
        Weather::Clear => "Clear",
        Weather::Rains => "Rains",
        Weather::Fog => "Fog",
        Weather::Windy => "Windy",
    }
}
