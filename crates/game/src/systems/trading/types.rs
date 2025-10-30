use std::fs;

use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::systems::economy::{CommodityId, MoneyCents};

/// Canonical metadata describing a tradable commodity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommoditySpec {
    pub id: CommodityId,
    pub slug: String,
    pub display_name: String,
    pub base_price_cents: i64,
    pub mass_per_unit_kg: u32,
    pub volume_per_unit_l: u32,
}

impl CommoditySpec {
    /// Stable numeric identifier that links back to the economy layer.
    pub fn id(&self) -> CommodityId {
        self.id
    }

    /// Human-readable slug that can be used for lookups and localisation keys.
    pub fn slug(&self) -> &str {
        &self.slug
    }

    /// Display name suitable for UI surfaces.
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    /// Base price (in cents) used as the anchor for quote calculations.
    pub fn base_price(&self) -> MoneyCents {
        MoneyCents(self.base_price_cents)
    }

    /// Per-unit mass measured in kilograms.
    pub fn mass_per_unit_kg(&self) -> u32 {
        self.mass_per_unit_kg
    }

    /// Per-unit volume measured in litres.
    pub fn volume_per_unit_l(&self) -> u32 {
        self.volume_per_unit_l
    }
}

/// Collection of tradable commodity specifications loaded from disk.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Resource)]
#[serde(deny_unknown_fields)]
pub struct Commodities {
    #[serde(default, rename = "commodity")]
    entries: Vec<CommoditySpec>,
}

impl Commodities {
    /// Returns an iterator over all known commodity specifications.
    pub fn iter(&self) -> impl Iterator<Item = &CommoditySpec> {
        self.entries.iter()
    }

    /// Number of commodities defined in the dataset.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the dataset is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Finds a commodity specification by its numeric identifier.
    pub fn get_by_id(&self, id: CommodityId) -> Option<&CommoditySpec> {
        self.entries.iter().find(|spec| spec.id == id)
    }

    /// Finds a commodity specification by its slug.
    pub fn get_by_slug(&self, slug: &str) -> Option<&CommoditySpec> {
        self.entries.iter().find(|spec| spec.slug == slug)
    }
}

/// Errors that can occur when loading commodity specifications from disk.
#[derive(Debug, Error)]
pub enum CommodityLoadError {
    #[error("failed to read commodities: {0}")]
    Read(#[from] std::io::Error),
    #[error("failed to parse commodities: {0}")]
    Parse(#[from] toml::de::Error),
}

/// Loads the canonical set of tradable commodities from a TOML file.
pub fn load_commodities(path: &str) -> Result<Commodities, CommodityLoadError> {
    let raw = fs::read_to_string(path)?;
    let commodities: Commodities = toml::from_str(&raw)?;
    Ok(commodities)
}
