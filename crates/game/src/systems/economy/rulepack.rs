#![allow(dead_code)]

use std::fs;

use blake3::Hasher;
use log::info;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rulepack {
    pub di: DiCfg,
    pub basis: BasisCfg,
    pub interest: InterestCfg,
    pub rot: RotCfg,
    pub pp: PpCfg,
    pub pricing: PricingCfg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiCfg {
    pub long_run_mean_bp: i32,
    pub retention_bp: i32,
    pub noise_sigma_bp: u32,
    pub noise_clamp_bp: i32,
    pub per_day_clamp_bp: i32,
    pub absolute_min_bp: i32,
    pub absolute_max_bp: i32,
    pub overlay_decay_bp: i32,
    pub overlay_min_bp: i32,
    pub overlay_max_bp: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BasisCfg {
    pub beta_pp_bp: i32,
    pub beta_routes_bp: i32,
    pub beta_stock_bp: i32,
    pub noise_sigma_bp: u32,
    pub noise_clamp_bp: i32,
    pub per_day_clamp_bp: i32,
    pub absolute_min_bp: i32,
    pub absolute_max_bp: i32,
    pub weather: BasisWeatherCfg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BasisWeatherCfg {
    pub clear_bp: i32,
    pub rains_bp: i32,
    pub fog_bp: i32,
    pub windy_bp: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InterestCfg {
    pub base_leg_bp: i32,
    pub linear_leg_bp: i32,
    pub linear_scale_cents: i64,
    pub convex_leg_bp: i32,
    pub convex_gamma_q16: u32,
    pub per_leg_cap_bp: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RotCfg {
    pub rot_floor: u16,
    pub rot_ceiling: u16,
    pub rot_decay_per_day: u16,
    pub conversion_chunk: u16,
    pub debt_per_chunk_cents: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PpCfg {
    pub min_pp: u16,
    pub max_pp: u16,
    pub neutral_pp: u16,
    pub planting_size_to_pp_bp: i32,
    pub planting_max_age_days: u16,
    pub decay_per_day_bp: i32,
    pub pull_strength_bp: i32,
    pub pull_decay_bp: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PricingCfg {
    pub min_multiplier_bp: i32,
    pub max_multiplier_bp: i32,
}

#[derive(Debug, Error)]
pub enum RulepackError {
    #[error("failed to read rulepack: {0}")]
    Read(#[from] std::io::Error),
    #[error("failed to parse rulepack: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("failed to hash rulepack: {0}")]
    Hash(#[from] serde_json::Error),
}

pub fn load_rulepack(path: &str) -> Result<Rulepack, RulepackError> {
    let raw = fs::read_to_string(path)?;
    let rulepack: Rulepack = toml::from_str(&raw)?;
    log_schema_hash(&rulepack)?;
    Ok(rulepack)
}

fn log_schema_hash(rulepack: &Rulepack) -> Result<(), RulepackError> {
    let bytes = serde_json::to_vec(rulepack)?;
    let mut hasher = Hasher::new();
    hasher.update(&bytes);
    let hash = hasher.finalize();
    info!("rulepack_schema_hash={}", hash.to_hex());
    Ok(())
}
