//! Integration tests for spawn type selection stability.
//!
//! This suite ensures that `choose_spawn_type` yields a stable multiset of
//! results for a fixed seed, weather, and range of spawn indices.  Future
//! changes to the spawn selection logic should preserve this deterministic
//! guarantee so that replayed missions remain consistent.

use std::collections::BTreeMap;
use std::path::PathBuf;

use game::systems::director::config::load_director_cfg;
use game::systems::director::spawn::{choose_spawn_type, SpawnTypeTables};
use game::systems::economy::Weather;

fn load_spawn_tables() -> SpawnTypeTables {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cfg_path = manifest_dir.join("../../assets/director/m2.toml");
    let cfg = load_director_cfg(cfg_path.to_str().expect("config path utf-8"))
        .expect("load director config");
    SpawnTypeTables::from_cfg(&cfg)
}

fn collect_spawn_counts(
    tables: &SpawnTypeTables,
    weather: Weather,
    seed: u64,
    indices: std::ops::Range<u64>,
) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for spawn_index in indices {
        let pick = choose_spawn_type(tables, weather, seed, spawn_index);
        *counts.entry(pick).or_insert(0) += 1;
    }
    counts
}

#[test]
fn choose_spawn_type_multiset_is_stable() {
    let tables = load_spawn_tables();
    let weather = Weather::Clear;
    let seed = 0x5EC0_F00Du64;
    let range = 0..2048;

    let first = collect_spawn_counts(&tables, weather, seed, range.clone());
    let second = collect_spawn_counts(&tables, weather, seed, range);

    assert_eq!(
        first, second,
        "spawn type distribution changed between runs"
    );
}
