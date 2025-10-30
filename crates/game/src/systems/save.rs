use std::fs;
use std::path::Path;

use thiserror::Error;

use crate::systems::economy::state::EconState;
use crate::systems::economy::{HubId, MoneyCents};
use crate::systems::migrations::{migrate_to_latest, MigrateError};
use crate::systems::trading::inventory::Cargo;

mod v1_1;

pub use v1_1::{
    BasisSave, CargoSave, CargoSlot, CommoditySave, InventorySlot, SaveSchema, SaveV1_1,
    SchemaVersion,
};

pub(crate) mod legacy {
    use serde::{Deserialize, Serialize};

    use crate::systems::economy::state::RngCursor;
    use crate::systems::economy::{EconomyDay, MoneyCents, PendingPlanting, Pp};

    use super::{BasisSave, CommoditySave, InventorySlot};

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

pub fn save(path: &Path, snapshot: &SaveV1_1) -> Result<(), SaveError> {
    let mut normalized = snapshot.clone();
    normalized.schema = SaveSchema::v1_1();
    normalized.di.sort_by_key(|entry| entry.commodity.0);
    normalized
        .basis
        .sort_by(|a, b| (a.hub.0, a.commodity.0).cmp(&(b.hub.0, b.commodity.0)));
    normalized.inventory.sort_by_key(|slot| slot.commodity.0);
    normalized
        .cargo
        .manifest
        .sort_by_key(|slot| slot.commodity.0);
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

pub fn load(path: &Path) -> Result<SaveV1_1, SaveError> {
    let raw = fs::read_to_string(path)?;
    let value: serde_json::Value = serde_json::from_str(&raw)?;
    Ok(migrate_to_latest(value)?)
}

impl SaveV1_1 {
    pub fn hydrate_econ_state(&self, econ: &mut EconState) {
        econ.day = self.day;
        econ.di_bp.clear();
        for entry in &self.di {
            econ.di_bp.insert(entry.commodity, entry.value);
        }
        econ.di_overlay_bp = self.di_overlay_bp;
        econ.basis_bp.clear();
        for entry in &self.basis {
            econ.basis_bp
                .insert((entry.hub, entry.commodity), entry.value);
        }
        econ.pp = self.pp;
        econ.rot_u16 = self.rot;
        econ.pending_planting = self.pending_planting.clone();
        econ.debt_cents = self.debt_cents;
        econ.last_clamp_hit = self.last_clamp_hit;
    }

    pub fn update_from_econ_state(&mut self, econ: &EconState) {
        self.day = econ.day;
        self.di = econ
            .di_bp
            .iter()
            .map(|(commodity, value)| CommoditySave {
                commodity: *commodity,
                value: *value,
            })
            .collect();
        self.di.sort_by_key(|entry| entry.commodity.0);
        self.di_overlay_bp = econ.di_overlay_bp;
        self.basis = econ
            .basis_bp
            .iter()
            .map(|((hub, commodity), value)| BasisSave {
                hub: *hub,
                commodity: *commodity,
                value: *value,
            })
            .collect();
        self.basis
            .sort_by(|a, b| (a.hub.0, a.commodity.0).cmp(&(b.hub.0, b.commodity.0)));
        self.pp = econ.pp;
        self.rot = econ.rot_u16;
        self.debt_cents = econ.debt_cents;
        self.pending_planting = econ.pending_planting.clone();
        self.last_clamp_hit = econ.last_clamp_hit;
    }

    pub fn hydrate_cargo(&self, cargo: &mut Cargo) {
        cargo.clear();
        cargo.capacity_total = self.cargo.capacity_total;
        cargo.capacity_used = self.cargo.capacity_used;
        cargo.mass_capacity_total = self.cargo.mass_capacity_total;
        cargo.mass_capacity_used = self.cargo.mass_capacity_used;
        for slot in &self.cargo.manifest {
            cargo.set_units(slot.commodity, slot.units);
        }
    }

    pub fn update_from_cargo(&mut self, cargo: &Cargo) {
        self.cargo.capacity_total = cargo.capacity_total;
        self.cargo.capacity_used = cargo.capacity_used;
        self.cargo.mass_capacity_total = cargo.mass_capacity_total;
        self.cargo.mass_capacity_used = cargo.mass_capacity_used;
        self.cargo.manifest = cargo
            .manifest_snapshot()
            .into_iter()
            .map(|(commodity, units)| CargoSlot { commodity, units })
            .collect();
    }

    pub fn wallet_balance(&self) -> MoneyCents {
        self.wallet_cents
    }

    pub fn set_wallet_balance(&mut self, balance: MoneyCents) {
        self.wallet_cents = balance;
    }

    pub fn set_last_hub(&mut self, hub: Option<HubId>) {
        self.last_hub = hub;
    }
}

#[cfg(test)]
mod tests;
