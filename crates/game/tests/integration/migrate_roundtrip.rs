use game::systems::migrations::migrate_to_latest;
use game::systems::save::{
    app_state_from_snapshot, load_app_state, save_app_state, snapshot_from_app_state,
};
use serde_json::Value;
use tempfile::tempdir;

#[test]
fn migrate_then_persist_is_idempotent() {
    let raw_v1 = include_str!("../goldens/save_v1_roundtrip.json");
    let value: Value = serde_json::from_str(raw_v1).expect("parse v1 value");
    let migrated = migrate_to_latest(value).expect("migrate to v1.1");

    let app_state = app_state_from_snapshot(migrated.clone());
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("save.json");
    save_app_state(&path, &app_state).expect("save migrated state");
    let first = std::fs::read_to_string(&path).expect("first read");

    let reloaded = load_app_state(&path).expect("reload app state");
    save_app_state(&path, &reloaded).expect("resave migrated state");
    let second = std::fs::read_to_string(&path).expect("second read");

    assert_eq!(
        first, second,
        "save output should be stable across roundtrips"
    );

    let roundtrip_snapshot = snapshot_from_app_state(&reloaded);
    assert_eq!(roundtrip_snapshot, migrated);
}
