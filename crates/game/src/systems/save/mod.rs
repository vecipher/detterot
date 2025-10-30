#![allow(dead_code)]

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::app_state::AppState;
use crate::systems::economy::state::RngCursor;
use crate::systems::economy::{
    BasisBp, CommodityId, EconState, EconomyDay, HubId, MoneyCents, PendingPlanting, Pp,
};
use crate::systems::migrations::{migrate_to_latest, MigrateError};
use crate::systems::trading::inventory::Cargo;

pub mod v1_1;

pub use v1_1::{CargoItemSave, CargoSave, SaveV11};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveV1 {
    pub econ_version: u32,
    pub world_seed: u64,
    pub day: EconomyDay,
    pub di: Vec<CommoditySave>,
    #[serde(default)]
    pub di_overlay_bp: i32,
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

pub fn save(path: &Path, snapshot: &SaveV11) -> Result<(), SaveError> {
    let mut normalized = snapshot.clone();
    normalized.di.sort_by_key(|entry| entry.commodity.0);
    normalized
        .basis
        .sort_by(|a, b| (a.hub.0, a.commodity.0).cmp(&(b.hub.0, b.commodity.0)));
    normalized.inventory.sort_by_key(|slot| slot.commodity.0);
    normalized.cargo.items.sort_by_key(|item| item.commodity.0);
    let mut json = serde_json::to_string_pretty(&normalized)?;
    if !json.ends_with('\n') {
        json.push('\n');
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(path, json)?;
    Ok(())
}

pub fn load(path: &Path) -> Result<SaveV11, SaveError> {
    let raw = fs::read_to_string(path)?;
    let value: serde_json::Value = serde_json::from_str(&raw)?;
    Ok(migrate_to_latest(value)?)
}

pub fn save_app_state(path: &Path, state: &AppState) -> Result<(), SaveError> {
    let snapshot = snapshot_from_app_state(state);
    save(path, &snapshot)
}

pub fn load_app_state(path: &Path) -> Result<AppState, SaveError> {
    let snapshot = load(path)?;
    Ok(app_state_from_snapshot(snapshot))
}

pub fn snapshot_from_app_state(state: &AppState) -> SaveV11 {
    let mut di: Vec<CommoditySave> = state
        .econ
        .di_bp
        .iter()
        .map(|(commodity, value)| CommoditySave {
            commodity: *commodity,
            value: *value,
        })
        .collect();
    di.sort_by_key(|entry| entry.commodity.0);

    let mut basis: Vec<BasisSave> = state
        .econ
        .basis_bp
        .iter()
        .map(|((hub, commodity), value)| BasisSave {
            hub: *hub,
            commodity: *commodity,
            value: *value,
        })
        .collect();
    basis.sort_by(|a, b| (a.hub.0, a.commodity.0).cmp(&(b.hub.0, b.commodity.0)));

    SaveV11 {
        econ_version: state.econ_version,
        world_seed: state.world_seed,
        day: state.econ.day,
        last_hub: state.last_hub,
        di,
        di_overlay_bp: state.econ.di_overlay_bp,
        basis,
        pp: state.econ.pp,
        rot: state.econ.rot_u16,
        debt_cents: state.econ.debt_cents,
        inventory: state.inventory.clone(),
        wallet_cents: state.wallet,
        cargo: cargo_to_save(&state.cargo),
        pending_planting: state.econ.pending_planting.clone(),
        rng_cursors: state.rng_cursors.clone(),
    }
}

pub fn app_state_from_snapshot(snapshot: SaveV11) -> AppState {
    let di_bp = snapshot
        .di
        .iter()
        .map(|entry| (entry.commodity, entry.value))
        .collect();
    let basis_bp = snapshot
        .basis
        .iter()
        .map(|entry| ((entry.hub, entry.commodity), entry.value))
        .collect();

    let econ = EconState {
        day: snapshot.day,
        di_bp,
        di_overlay_bp: snapshot.di_overlay_bp,
        basis_bp,
        pp: snapshot.pp,
        rot_u16: snapshot.rot,
        pending_planting: snapshot.pending_planting.clone(),
        debt_cents: snapshot.debt_cents,
        ..Default::default()
    };

    AppState {
        econ_version: snapshot.econ_version,
        world_seed: snapshot.world_seed,
        econ,
        last_hub: snapshot.last_hub,
        inventory: snapshot.inventory,
        cargo: cargo_from_save(snapshot.cargo),
        rng_cursors: snapshot.rng_cursors,
        wallet: snapshot.wallet_cents,
    }
}

fn cargo_to_save(cargo: &Cargo) -> CargoSave {
    let mut items: Vec<CargoItemSave> = cargo
        .items
        .iter()
        .map(|(commodity, units)| CargoItemSave {
            commodity: *commodity,
            units: *units,
        })
        .collect();
    items.sort_by_key(|item| item.commodity.0);
    CargoSave {
        capacity_mass_kg: cargo.capacity_mass_kg,
        capacity_volume_l: cargo.capacity_volume_l,
        items,
    }
}

fn cargo_from_save(save: CargoSave) -> Cargo {
    let mut items: HashMap<CommodityId, u32> = HashMap::new();
    for item in save.items {
        items.insert(item.commodity, item.units);
    }
    Cargo {
        capacity_mass_kg: save.capacity_mass_kg,
        capacity_volume_l: save.capacity_volume_l,
        items,
    }
}
