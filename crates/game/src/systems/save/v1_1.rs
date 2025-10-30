use serde::{Deserialize, Serialize};

use crate::systems::economy::state::RngCursor;
use crate::systems::economy::{
    BasisBp, CommodityId, EconomyDay, HubId, MoneyCents, PendingPlanting, Pp,
};

use super::legacy;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaVersion {
    pub major: u32,
    pub minor: u32,
}

impl SchemaVersion {
    pub const fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    pub const fn v1_1() -> Self {
        Self::new(1, 1)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveSchema {
    pub version: SchemaVersion,
}

impl SaveSchema {
    pub fn v1_1() -> Self {
        Self {
            version: SchemaVersion::v1_1(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveV1_1 {
    pub schema: SaveSchema,
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
    #[serde(default)]
    pub wallet_cents: MoneyCents,
    pub inventory: Vec<InventorySlot>,
    pub pending_planting: Vec<PendingPlanting>,
    pub rng_cursors: Vec<RngCursor>,
    #[serde(default)]
    pub last_hub: Option<HubId>,
    #[serde(default)]
    pub cargo: CargoSave,
    #[serde(default)]
    pub last_clamp_hit: bool,
}

impl From<legacy::SaveV1> for SaveV1_1 {
    fn from(value: legacy::SaveV1) -> Self {
        Self {
            schema: SaveSchema::v1_1(),
            econ_version: value.econ_version,
            world_seed: value.world_seed,
            day: value.day,
            di: value.di,
            di_overlay_bp: value.di_overlay_bp,
            basis: value.basis,
            pp: value.pp,
            rot: value.rot,
            debt_cents: value.debt_cents,
            wallet_cents: MoneyCents::ZERO,
            inventory: value.inventory,
            pending_planting: value.pending_planting,
            rng_cursors: value.rng_cursors,
            last_hub: None,
            cargo: CargoSave::default(),
            last_clamp_hit: false,
        }
    }
}

impl SaveV1_1 {
    pub fn upgrade_from_v1(value: legacy::SaveV1) -> Self {
        value.into()
    }
}

impl Default for SaveV1_1 {
    fn default() -> Self {
        Self {
            schema: SaveSchema::v1_1(),
            econ_version: 0,
            world_seed: 0,
            day: EconomyDay(0),
            di: Vec::new(),
            di_overlay_bp: 0,
            basis: Vec::new(),
            pp: Pp(0),
            rot: 0,
            debt_cents: MoneyCents::ZERO,
            wallet_cents: MoneyCents::ZERO,
            inventory: Vec::new(),
            pending_planting: Vec::new(),
            rng_cursors: Vec::new(),
            last_hub: None,
            cargo: CargoSave::default(),
            last_clamp_hit: false,
        }
    }
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct CargoSave {
    pub capacity_total: u32,
    pub capacity_used: u32,
    #[serde(default)]
    pub mass_capacity_total: u32,
    #[serde(default)]
    pub mass_capacity_used: u32,
    #[serde(default)]
    pub manifest: Vec<CargoSlot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CargoSlot {
    pub commodity: CommodityId,
    pub units: u32,
}
