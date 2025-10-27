use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::Context;
use serde::Deserialize;

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

#[derive(Debug, Deserialize, Clone, Default)]
pub struct MissionCfg {
    #[serde(default)]
    pub pp_success: i16,
    #[serde(default)]
    pub pp_fail: i16,
    #[serde(default)]
    pub basis_bp_success: i16,
    #[serde(default)]
    pub basis_bp_fail: i16,
}

pub fn load_director_cfg(path: &str) -> anyhow::Result<DirectorCfg> {
    let bytes = fs::read(Path::new(path))
        .with_context(|| format!("reading director config from {path}"))?;
    let cfg_str = std::str::from_utf8(&bytes)
        .with_context(|| format!("config {path} was not valid UTF-8"))?;
    let cfg: DirectorCfg = toml::from_str(cfg_str)
        .with_context(|| format!("deserializing director config from {path}"))?;
    Ok(cfg)
}
