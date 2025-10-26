#![allow(dead_code)]

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::systems::economy::state::RngCursor;
use crate::systems::economy::{
    BasisBp, CommodityId, EconomyDay, HubId, MoneyCents, PendingPlanting, Pp,
};
use crate::systems::migrations::{migrate_to_latest, MigrateError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveV1 {
    pub econ_version: u32,
    pub world_seed: u64,
    pub day: EconomyDay,
    pub di: Vec<CommoditySave>,
    pub basis: Vec<BasisSave>,
    pub pp: Pp,
    pub rot: u16,
    #[serde(default)]
    pub debt_cents: MoneyCents,
    pub inventory: Vec<InventorySlot>,
    pub pending_planting: Vec<PendingPlanting>,
    pub rng_cursors: Vec<RngCursor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommoditySave {
    pub commodity: CommodityId,
    pub value: BasisBp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BasisSave {
    pub hub: HubId,
    pub commodity: CommodityId,
    pub value: BasisBp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct InventorySlot {
    pub commodity: CommodityId,
    pub amount: u32,
}

#[derive(Debug, Error)]
pub enum SaveError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error(transparent)]
    Migrate(#[from] MigrateError),
}

pub fn save(path: &Path, snapshot: &SaveV1) -> Result<(), SaveError> {
    let mut normalized = snapshot.clone();
    normalized.di.sort_by_key(|entry| entry.commodity.0);
    normalized
        .basis
        .sort_by(|a, b| (a.hub.0, a.commodity.0).cmp(&(b.hub.0, b.commodity.0)));
    normalized.inventory.sort_by_key(|slot| slot.commodity.0);
    let json = serde_json::to_string_pretty(&normalized)?;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(path, json)?;
    Ok(())
}

pub fn load(path: &Path) -> Result<SaveV1, SaveError> {
    let raw = fs::read_to_string(path)?;
    let value: serde_json::Value = serde_json::from_str(&raw)?;
    Ok(migrate_to_latest(value)?)
}
