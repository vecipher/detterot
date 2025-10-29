use serde_json::json;
use tempfile::tempdir;

use game::systems::economy::{BasisBp, CommodityId, HubId, MoneyCents};
use game::systems::migrations::migrate_to_latest;
use game::systems::save::{load, save};

#[test]
fn migrating_legacy_payload_is_byte_stable() {
    let legacy = json!({
        "econ_version": 2,
        "world_seed": 4242,
        "day": 8,
        "di": [
            { "commodity": CommodityId(1).0, "value": BasisBp(15).0 },
            { "commodity": CommodityId(3).0, "value": BasisBp(-10).0 }
        ],
        "di_overlay_bp": -5,
        "basis": [
            {
                "hub": HubId(1).0,
                "commodity": CommodityId(1).0,
                "value": BasisBp(2).0
            }
        ],
        "pp": 5_000,
        "rot": 4,
        "debt_cents": MoneyCents(1_200).0,
        "inventory": [],
        "pending_planting": [],
        "rng_cursors": []
    });

    let migrated = migrate_to_latest(legacy).expect("migrate legacy payload");
    assert_eq!(migrated.wallet_balance(), MoneyCents::ZERO);
    assert!(migrated.cargo.manifest.is_empty());

    let dir = tempdir().expect("temp dir");
    let path = dir.path().join("migrated.json");

    save(&path, &migrated).expect("write migrated snapshot");
    let first_bytes = std::fs::read(&path).expect("read first snapshot");

    let loaded = load(&path).expect("load migrated snapshot");
    assert_eq!(loaded, migrated);

    save(&path, &loaded).expect("rewrite snapshot");
    let second_bytes = std::fs::read(&path).expect("read rewritten snapshot");

    assert_eq!(first_bytes, second_bytes);
}
