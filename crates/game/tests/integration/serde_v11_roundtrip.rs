use game::systems::economy::state::RngCursor;
use game::systems::economy::{
    BasisBp, CommodityId, EconomyDay, HubId, MoneyCents, PendingPlanting, Pp,
};
use game::systems::save::{
    load, save, BasisSave, CargoItemSave, CargoSave, CommoditySave, InventorySlot, SaveV11,
};
use std::fs;
use tempfile::tempdir;

fn sample_save() -> SaveV11 {
    SaveV11 {
        econ_version: 7,
        world_seed: 42,
        day: EconomyDay(3),
        last_hub: HubId(2),
        di: vec![
            CommoditySave {
                commodity: CommodityId(1),
                value: BasisBp(125),
            },
            CommoditySave {
                commodity: CommodityId(2),
                value: BasisBp(-45),
            },
        ],
        di_overlay_bp: 120,
        basis: vec![
            BasisSave {
                hub: HubId(1),
                commodity: CommodityId(1),
                value: BasisBp(15),
            },
            BasisSave {
                hub: HubId(1),
                commodity: CommodityId(2),
                value: BasisBp(-10),
            },
        ],
        pp: Pp(5_100),
        rot: 12,
        debt_cents: MoneyCents(4_200),
        inventory: vec![InventorySlot {
            commodity: CommodityId(9),
            amount: 33,
        }],
        wallet_cents: MoneyCents(37_217),
        cargo: CargoSave {
            capacity_mass_kg: 2_000,
            capacity_volume_l: 1_500,
            items: vec![CargoItemSave {
                commodity: CommodityId(1),
                units: 7,
            }],
        },
        pending_planting: vec![PendingPlanting {
            hub: HubId(1),
            size: 4,
            age_days: 2,
        }],
        rng_cursors: vec![RngCursor {
            label: "di".to_string(),
            draws: 24,
        }],
    }
}

#[test]
fn save_roundtrip_is_byte_identical() {
    let dir = tempdir().expect("temp dir");
    let path = dir.path().join("save_v11.json");
    let snapshot = sample_save();
    save(&path, &snapshot).expect("write save");
    let written = fs::read_to_string(&path).expect("read save");
    let golden = include_str!("../goldens/save_v11_roundtrip.json");
    assert_eq!(written, golden);
    let loaded = load(&path).expect("load save");
    assert_eq!(loaded, snapshot);
}

#[test]
fn rejects_unknown_keys() {
    let dir = tempdir().expect("temp dir");
    let path = dir.path().join("bad_save.json");
    fs::write(
        &path,
        r#"{
            "econ_version": 1,
            "world_seed": 1,
            "day": 0,
            "di": [],
            "last_hub": 0,
            "basis": [],
            "pp": 0,
            "rot": 0,
            "inventory": [],
            "wallet_cents": 0,
            "cargo": {
                "capacity_mass_kg": 0,
                "capacity_volume_l": 0,
                "items": []
            },
            "pending_planting": [],
            "rng_cursors": [],
            "extra": 1
        }"#,
    )
    .expect("write bad save");

    let err = load(&path).expect_err("unknown key should fail");
    assert!(format!("{}", err).contains("unknown"));
}
