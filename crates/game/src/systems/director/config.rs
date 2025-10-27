use std::{collections::HashMap, fs, path::Path};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct DirectorCfg {
    pub spawn: SpawnCfg,
    #[serde(default)]
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
    #[serde(default)]
    pub beta_weather: HashMap<String, i32>,
    pub growth_cap_per_leg: u32,
    pub clamp_min: u32,
    pub clamp_max: u32,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct MissionCfg {
    pub pp_success: i16,
    pub pp_fail: i16,
    #[serde(default = "zero_i16")]
    pub basis_bp_success: i16,
    #[serde(default = "zero_i16")]
    pub basis_bp_fail: i16,
}

fn zero_i16() -> i16 {
    0
}

pub fn load_director_cfg(path: impl AsRef<Path>) -> Result<DirectorCfg> {
    let path = path.as_ref();
    let contents = fs::read_to_string(path)
        .with_context(|| format!("unable to read director config at {}", path.display()))?;
    let cfg: DirectorCfg = toml::from_str(&contents)
        .with_context(|| format!("unable to parse director config at {}", path.display()))?;
    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_fixture() {
        let data = r#"
            [spawn]
            base = 3
            alpha_pp_per_100 = 1
            growth_cap_per_leg = 2
            clamp_min = 1
            clamp_max = 10

            [spawn.beta_weather]
            Clear = 0
            Fog = 2

            [missions.example]
            pp_success = 1
            pp_fail = -1
        "#;

        let cfg: DirectorCfg = toml::from_str(data).unwrap();
        assert_eq!(cfg.spawn.base, 3);
        assert_eq!(cfg.spawn.beta_weather["Fog"], 2);
        assert_eq!(cfg.missions["example"].pp_fail, -1);
    }
}
