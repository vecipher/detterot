use serde::{Deserialize, Serialize};

use crate::systems::economy::state::RngCursor;
use crate::systems::economy::{CommodityId, EconomyDay, HubId, MoneyCents, PendingPlanting, Pp};

use super::{BasisSave, CommoditySave, InventorySlot, SaveV1};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveV11 {
    pub econ_version: u32,
    pub world_seed: u64,
    pub day: EconomyDay,
    #[serde(default)]
    pub last_hub: HubId,
    pub di: Vec<CommoditySave>,
    #[serde(default)]
    pub di_overlay_bp: i32,
    pub basis: Vec<BasisSave>,
    pub pp: Pp,
    pub rot: u16,
    #[serde(default)]
    pub debt_cents: MoneyCents,
    pub inventory: Vec<InventorySlot>,
    #[serde(default)]
    pub wallet_cents: MoneyCents,
    pub cargo: CargoSave,
    pub pending_planting: Vec<PendingPlanting>,
    pub rng_cursors: Vec<RngCursor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct CargoSave {
    pub capacity_mass_kg: u32,
    pub capacity_volume_l: u32,
    #[serde(default)]
    pub items: Vec<CargoItemSave>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CargoItemSave {
    pub commodity: CommodityId,
    pub units: u32,
}

impl From<SaveV1> for SaveV11 {
    fn from(v1: SaveV1) -> Self {
        SaveV11 {
            econ_version: v1.econ_version,
            world_seed: v1.world_seed,
            day: v1.day,
            last_hub: HubId(0),
            di: v1.di,
            di_overlay_bp: v1.di_overlay_bp,
            basis: v1.basis,
            pp: v1.pp,
            rot: v1.rot,
            debt_cents: v1.debt_cents,
            inventory: v1.inventory,
            wallet_cents: MoneyCents::ZERO,
            cargo: CargoSave::default(),
            pending_planting: v1.pending_planting,
            rng_cursors: v1.rng_cursors,
        }
    }
}

pub fn migrate_v1_to_v11(v1: SaveV1) -> SaveV11 {
    SaveV11::from(v1)
}
