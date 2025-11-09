use game::systems::economy::MoneyCents;
use game::systems::migrations::migrate_to_latest;
use game::systems::save::{v1_1::migrate_v1_to_v11, CargoSave, SaveV1};
use serde_json::Value;

#[test]
fn migration_preserves_econ_fields_and_sets_defaults() {
    let raw = include_str!("../goldens/save_v1_roundtrip.json");
    let original: SaveV1 = serde_json::from_str(raw).expect("parse v1");
    let value: Value = serde_json::from_str(raw).expect("value");

    let migrated = migrate_to_latest(value).expect("migrate via dispatcher");

    assert_eq!(migrated.econ_version, original.econ_version);
    assert_eq!(migrated.world_seed, original.world_seed);
    assert_eq!(migrated.day, original.day);
    assert_eq!(migrated.di, original.di);
    assert_eq!(migrated.di_overlay_bp, original.di_overlay_bp);
    assert_eq!(migrated.basis, original.basis);
    assert_eq!(migrated.pp, original.pp);
    assert_eq!(migrated.rot, original.rot);
    assert_eq!(migrated.debt_cents, original.debt_cents);
    assert_eq!(migrated.inventory, original.inventory);
    assert_eq!(migrated.pending_planting, original.pending_planting);
    assert_eq!(migrated.rng_cursors, original.rng_cursors);

    assert_eq!(migrated.last_hub, Default::default());
    assert_eq!(migrated.cargo, CargoSave::default());
    assert_eq!(migrated.wallet_cents, MoneyCents::ZERO);

    let manual_v11 = migrate_v1_to_v11(original.clone());
    let manual_v12 = game::systems::save::v1_2::migrate_v11_to_v12(manual_v11);
    assert_eq!(migrated, manual_v12);

    // Ensure econ bytes stable by comparing serialized slices
    let original_econ = serde_json::to_string_pretty(&original).expect("serialize v1");
    let migrated_econ = serde_json::to_string_pretty(&manual_v12).expect("serialize v12");
    let original_fragment: Value = serde_json::from_str(&original_econ).expect("value");
    let migrated_fragment: Value = serde_json::from_str(&migrated_econ).expect("value");

    // Note: Since we're comparing to SaveV12, we need to check that V1 fields are equal
    assert_eq!(original_fragment["di"], migrated_fragment["di"]);
    assert_eq!(original_fragment["basis"], migrated_fragment["basis"]);
}
