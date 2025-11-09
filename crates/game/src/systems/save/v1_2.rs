use serde::{Deserialize, Serialize};

use crate::systems::economy::state::RngCursor;
use crate::systems::economy::{EconomyDay, HubId, MoneyCents, PendingPlanting, Pp, RouteId};

use super::{BasisSave, CargoSave, CommoditySave, InventorySlot, SaveV11};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveV12 {
    // All v1.1 fields
    pub econ_version: u32,
    pub world_seed: u64,
    pub day: EconomyDay,
    pub last_hub: HubId,
    pub di: Vec<CommoditySave>,
    pub di_overlay_bp: i32,
    pub basis: Vec<BasisSave>,
    pub pp: Pp,
    pub rot: u16,
    pub debt_cents: MoneyCents,
    pub inventory: Vec<InventorySlot>,
    pub wallet_cents: MoneyCents,
    pub cargo: CargoSave, // Reuse the v1.1 CargoSave
    pub pending_planting: Vec<PendingPlanting>,
    pub rng_cursors: Vec<RngCursor>,

    // M4 additions
    pub last_board_hash: u64,
    pub visited_links: Vec<RouteId>,
}

impl From<SaveV11> for SaveV12 {
    fn from(v11: SaveV11) -> Self {
        SaveV12 {
            // All v1.1 fields
            econ_version: v11.econ_version,
            world_seed: v11.world_seed,
            day: v11.day,
            last_hub: v11.last_hub,
            di: v11.di,
            di_overlay_bp: v11.di_overlay_bp,
            basis: v11.basis,
            pp: v11.pp,
            rot: v11.rot,
            debt_cents: v11.debt_cents,
            inventory: v11.inventory,
            wallet_cents: v11.wallet_cents,
            cargo: v11.cargo,
            pending_planting: v11.pending_planting,
            rng_cursors: v11.rng_cursors,

            // M4 additions - defaults
            last_board_hash: 0,
            visited_links: Vec::new(),
        }
    }
}

pub fn migrate_v11_to_v12(v11: SaveV11) -> SaveV12 {
    SaveV12::from(v11)
}

#[cfg(test)]
mod tests {
    use crate::systems::economy::{CommodityId, RouteId};
    use crate::systems::save::CargoItemSave;

    use super::*;

    #[test]
    fn migrate_v11_to_v12_preserves_data() {
        let v11 = SaveV11 {
            econ_version: 1,
            world_seed: 12345,
            day: EconomyDay(100),
            last_hub: HubId(5),
            di: vec![],
            di_overlay_bp: 50,
            basis: vec![],
            pp: Pp(1000),
            rot: 500,
            debt_cents: MoneyCents(10000),
            inventory: vec![],
            wallet_cents: MoneyCents(5000),
            cargo: CargoSave {
                capacity_mass_kg: 1000,
                capacity_volume_l: 500,
                items: vec![],
            },
            pending_planting: vec![],
            rng_cursors: vec![],
        };

        let v12 = migrate_v11_to_v12(v11.clone());

        // Check that v11 fields are preserved
        assert_eq!(v12.econ_version, v11.econ_version);
        assert_eq!(v12.world_seed, v11.world_seed);
        assert_eq!(v12.day, v11.day);
        assert_eq!(v12.last_hub, v11.last_hub);
        assert_eq!(v12.di_overlay_bp, v11.di_overlay_bp);
        assert_eq!(v12.pp, v11.pp);
        assert_eq!(v12.rot, v11.rot);
        assert_eq!(v12.debt_cents, v11.debt_cents);
        assert_eq!(v12.wallet_cents, v11.wallet_cents);

        // Check that new fields have defaults
        assert_eq!(v12.last_board_hash, 0);
        assert_eq!(v12.visited_links, Vec::<RouteId>::new());
    }

    #[test]
    fn serde_v12_roundtrip() {
        let original = SaveV12 {
            econ_version: 1,
            world_seed: 12345,
            day: EconomyDay(100),
            last_hub: HubId(5),
            di: vec![],
            di_overlay_bp: 50,
            basis: vec![],
            pp: Pp(1000),
            rot: 500,
            debt_cents: MoneyCents(10000),
            inventory: vec![],
            wallet_cents: MoneyCents(5000),
            cargo: CargoSave {
                capacity_mass_kg: 1000,
                capacity_volume_l: 500,
                items: vec![CargoItemSave {
                    commodity: CommodityId(1),
                    units: 5,
                }],
            },
            pending_planting: vec![],
            rng_cursors: vec![],
            last_board_hash: 123456789,
            visited_links: vec![RouteId(1), RouteId(2), RouteId(5)],
        };

        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: SaveV12 = serde_json::from_str(&serialized).unwrap();

        assert_eq!(original, deserialized);
    }

    #[test]
    fn unknown_fields_rejected() {
        let bad_json = r#"{
            "econ_version": 1,
            "world_seed": 12345,
            "day": {"0": 100},
            "last_hub": {"0": 5},
            "di": [],
            "di_overlay_bp": 50,
            "basis": [],
            "pp": {"0": 1000},
            "rot": 500,
            "debt_cents": {"0": 10000},
            "inventory": [],
            "wallet_cents": {"0": 5000},
            "cargo": {
                "capacity_mass_kg": 1000,
                "capacity_volume_l": 500,
                "items": []
            },
            "pending_planting": [],
            "rng_cursors": [],
            "last_board_hash": 123456789,
            "visited_links": [{"0": 1}, {"0": 2}],
            "unknown_field": 42
        }"#;

        let result: Result<SaveV12, _> = serde_json::from_str(bad_json);
        assert!(result.is_err(), "Expected unknown field to be rejected");
    }
}
