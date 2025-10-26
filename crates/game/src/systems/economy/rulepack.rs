#![allow(dead_code)]

use std::fs;

use blake3::Hasher;
use log::info;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Collection of economy tuning parameters loaded from a TOML rulepack.
///
/// Each sub-structure focuses on one subsystem (daily index, basis, player
/// pricing power, etc.) and stores values primarily expressed in basis points
/// (bp) unless otherwise noted. Basis points are interpreted as 1/100th of a
/// percent (10_000 bp = 100%).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rulepack {
    /// Day-index configuration expressed in basis points.
    pub di: DiCfg,
    /// Basis calculation configuration, including weather overlays, in basis
    /// points.
    pub basis: BasisCfg,
    /// Interest leg configuration for loan pricing.
    pub interest: InterestCfg,
    /// ROT (rotation) conversion settings for debt/goods balancing.
    pub rot: RotCfg,
    /// Player power (PP) clamps and dynamics.
    pub pp: PpCfg,
    /// Pricing multiplier bounds expressed in basis points.
    pub pricing: PricingCfg,
}

/// Configuration for the Daily Index (DI) that anchors commodity price levels.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiCfg {
    /// Long-run mean of the DI in basis points, representing 0 bp = par.
    pub long_run_mean_bp: i32,
    /// Retention of yesterday's DI expressed as a proportion in bp (10_000 bp =
    /// 100%).
    pub retention_bp: i32,
    /// Standard deviation of stochastic shocks to the DI in bp.
    pub noise_sigma_bp: u32,
    /// Absolute clamp applied to a single sampled noise shock in bp.
    pub noise_clamp_bp: i32,
    /// Clamp on per-day DI movement after retention and noise are combined.
    pub per_day_clamp_bp: i32,
    /// Absolute lower bound the DI can reach, in bp.
    pub absolute_min_bp: i32,
    /// Absolute upper bound the DI can reach, in bp.
    pub absolute_max_bp: i32,
    /// Daily exponential decay toward zero for temporary overlay adjustments in
    /// bp.
    pub overlay_decay_bp: i32,
    /// Minimum overlay offset that can be applied to the DI, in bp.
    pub overlay_min_bp: i32,
    /// Maximum overlay offset that can be applied to the DI, in bp.
    pub overlay_max_bp: i32,
}

/// Configuration that converts economic signals into a basis spread.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BasisCfg {
    /// Sensitivity of basis to player power (PP) in bp per 100 PP deviation.
    pub beta_pp_bp: i32,
    /// Sensitivity of basis to active route count in bp per additional route.
    pub beta_routes_bp: i32,
    /// Sensitivity of basis to warehouse stock percentage in bp per percent of
    /// stock.
    pub beta_stock_bp: i32,
    /// Standard deviation of random noise injected into the basis (bp).
    pub noise_sigma_bp: u32,
    /// Clamp on the single-day noise component of the basis (bp).
    pub noise_clamp_bp: i32,
    /// Clamp on total per-day change to the basis after dynamics (bp).
    pub per_day_clamp_bp: i32,
    /// Hard lower limit for the basis, expressed in bp.
    pub absolute_min_bp: i32,
    /// Hard upper limit for the basis, expressed in bp.
    pub absolute_max_bp: i32,
    /// Weather overlay contributions to the basis.
    pub weather: BasisWeatherCfg,
}

/// Additive basis offsets contributed by each weather condition (bp).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BasisWeatherCfg {
    /// Offset applied when the forecast is clear.
    pub clear_bp: i32,
    /// Offset applied when rain is forecast.
    pub rains_bp: i32,
    /// Offset applied when fog is forecast.
    pub fog_bp: i32,
    /// Offset applied when windy weather is forecast.
    pub windy_bp: i32,
}

/// Parameters used to price installment interest on debt legs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InterestCfg {
    /// Flat spread per leg in bp.
    pub base_leg_bp: i32,
    /// Linear spread sensitivity in bp per `linear_scale_cents` of principal.
    pub linear_leg_bp: i32,
    /// Amount of notional (in cents) that produces one unit of linear spread.
    pub linear_scale_cents: i64,
    /// Convex spread contribution per leg in bp before the gamma ramp.
    pub convex_leg_bp: i32,
    /// Gamma parameter (Q16 fixed point) scaling the convex term.
    pub convex_gamma_q16: u32,
    /// Maximum total spread per leg after combining all terms, in bp.
    pub per_leg_cap_bp: i32,
}

/// Rotation (ROT) tracking that converts surplus production into debt relief.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RotCfg {
    /// Minimum stored ROT value retained after decay (raw ROT units).
    pub rot_floor: u16,
    /// Maximum stored ROT before further accumulation is clamped (raw ROT units).
    pub rot_ceiling: u16,
    /// Passive daily decay applied to stored ROT (raw ROT units per day).
    pub rot_decay_per_day: u16,
    /// ROT quantity consumed per conversion step (raw ROT units).
    pub conversion_chunk: u16,
    /// Debt value (in cents) forgiven per conversion chunk of ROT.
    pub debt_per_chunk_cents: i64,
}

/// Player Power (PP) dynamics describing how planting and decay shift PP.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PpCfg {
    /// Minimum PP allowed after updates.
    pub min_pp: u16,
    /// Maximum PP allowed after updates.
    pub max_pp: u16,
    /// Neutral PP where the economy expects supply/demand balance.
    pub neutral_pp: u16,
    /// PP gained per unit of planting size (bp per 100 PP contribution).
    pub planting_size_to_pp_bp: i32,
    /// Oldest planting age (days) that still contributes to PP pulls.
    pub planting_max_age_days: u16,
    /// Passive PP decay per day (bp of the gap toward neutral).
    pub decay_per_day_bp: i32,
    /// Strength of the pull toward neutral PP (bp per day).
    pub pull_strength_bp: i32,
    /// Daily decay applied to the pull strength (bp).
    pub pull_decay_bp: i32,
}

/// Bounds for price multipliers applied to transaction quotes (in bp).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PricingCfg {
    /// Minimum allowed multiplier (e.g., -3000 bp = 30% discount).
    pub min_multiplier_bp: i32,
    /// Maximum allowed multiplier (e.g., 4000 bp = 40% premium).
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
