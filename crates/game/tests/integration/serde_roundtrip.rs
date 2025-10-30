use game::systems::economy::state::RngCursor;
use game::systems::economy::{
    BasisBp, CommodityId, EconomyDay, HubId, MoneyCents, PendingPlanting, Pp,
};
use game::systems::save::{
    load, save, BasisSave, CargoSave, CargoSlot, CommoditySave, InventorySlot, SaveSchema, SaveV1_1,
};
use std::fs;
use tempfile::tempdir;

fn sample_save() -> SaveV1_1 {
    SaveV1_1 {
        schema: SaveSchema::v1_1(),
        econ_version: 7,
        world_seed: 42,
        day: EconomyDay(3),
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
        wallet_cents: MoneyCents(9_900),
        inventory: vec![InventorySlot {
            commodity: CommodityId(9),
            amount: 33,
        }],
        pending_planting: vec![PendingPlanting {
            hub: HubId(1),
            size: 4,
            age_days: 2,
        }],
        rng_cursors: vec![RngCursor {
            label: "di".to_string(),
            draws: 24,
        }],
        last_hub: Some(HubId(3)),
        cargo: CargoSave {
            capacity_total: 40,
            capacity_used: 21,
            mass_capacity_total: 120,
            mass_capacity_used: 76,
            manifest: vec![
                CargoSlot {
                    commodity: CommodityId(7),
                    units: 10,
                },
                CargoSlot {
                    commodity: CommodityId(1),
                    units: 3,
                },
            ],
        },
        last_clamp_hit: false,
    }
}

#[test]
fn save_roundtrip_is_byte_identical() {
    let dir = tempdir().expect("temp dir");
    let path = dir.path().join("save_v1_1.json");
    let snapshot = sample_save();
    save(&path, &snapshot).expect("write save");
    let written = fs::read_to_string(&path).expect("read save");
    let golden = include_str!("../goldens/save_v1_1_roundtrip.json");
    assert_eq!(written, golden);
    let loaded = load(&path).expect("load save");
    let mut expected = snapshot.clone();
    expected.cargo.manifest.sort_by_key(|slot| slot.commodity.0);
    assert_eq!(loaded, expected);
}

#[test]
fn rejects_unknown_keys() {
    let dir = tempdir().expect("temp dir");
    let path = dir.path().join("bad_save.json");
    fs::write(
        &path,
        r#"{
            "schema": { "version": { "major": 1, "minor": 1 } },
            "econ_version": 1,
            "world_seed": 1,
            "day": 0,
            "di": [],
            "basis": [],
            "pp": 0,
            "rot": 0,
            "inventory": [],
            "pending_planting": [],
            "rng_cursors": [],
            "last_hub": null,
            "cargo": { "capacity_total": 0, "capacity_used": 0, "manifest": [] },
            "extra": 1
        }"#,
    )
    .expect("write bad save");

    let err = load(&path).expect_err("unknown key should fail");
    assert!(format!("{}", err).contains("unknown"));
}
