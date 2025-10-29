use crate::systems::migrations::{migrate_to_latest, MigrateError};
use crate::systems::save::SchemaVersion;

#[test]
fn legacy_payloads_upgrade_with_defaults() {
    let legacy_value = serde_json::json!({
        "econ_version": 3,
        "world_seed": 99,
        "day": 12,
        "di": [],
        "basis": [],
        "pp": 0,
        "rot": 0,
        "inventory": [],
        "pending_planting": [],
        "rng_cursors": []
    });

    let snapshot = migrate_to_latest(legacy_value).expect("legacy payload migrates");

    assert_eq!(snapshot.schema.version, SchemaVersion::v1_1());
    assert_eq!(snapshot.world_seed, 99);
    assert_eq!(snapshot.day.0, 12);
    assert!(snapshot.last_hub.is_none());
    assert_eq!(snapshot.cargo.capacity_total, 0);
    assert_eq!(snapshot.cargo.capacity_used, 0);
    assert!(snapshot.cargo.manifest.is_empty());
}

#[test]
fn rejects_unknown_schema_versions() {
    let bad_value = serde_json::json!({
        "schema": { "version": { "major": 2, "minor": 0 } },
        "econ_version": 1,
        "world_seed": 1,
        "day": 0,
        "di": [],
        "basis": [],
        "pp": 0,
        "rot": 0,
        "inventory": [],
        "pending_planting": [],
        "rng_cursors": []
    });

    let err = migrate_to_latest(bad_value).expect_err("unsupported version fails");
    match err {
        MigrateError::UnsupportedVersion { major, minor } => {
            assert_eq!((major, minor), (2, 0));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}
