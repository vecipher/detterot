use std::collections::HashSet;
use std::path::PathBuf;

use crate::systems::economy::{EconomyDay, HubId};
use crate::world::WorldIndex;

fn workspace_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../{relative}"))
}

#[test]
fn rumor_is_stable_for_fixed_inputs() {
    let path = workspace_path("assets/world/hubs_min.toml");
    let index = WorldIndex::from_path(&path).expect("failed to load world index");
    let world_seed = 0xDEADBEEFCAFEBABE;
    let origin = HubId(1);
    let neighbour = index.neighbors(origin)[0];

    let rumor_a = index
        .deterministic_rumor(world_seed, origin, neighbour, EconomyDay(7))
        .expect("missing rumour");
    for _ in 0..10 {
        let rumor_b = index
            .deterministic_rumor(world_seed, origin, neighbour, EconomyDay(7))
            .expect("missing rumour");
        assert_eq!(rumor_a, rumor_b);
    }
}

#[test]
fn rumor_changes_across_days_when_multiple_hints_exist() {
    let path = workspace_path("assets/world/hubs_min.toml");
    let index = WorldIndex::from_path(&path).expect("failed to load world index");
    let world_seed = 0x1234_5678_9ABC_DEF0;
    let origin = HubId(1);
    let neighbour = index.neighbors(origin)[0];

    let hints = index.weather_hints(origin);
    assert!(hints.len() > 1, "test dataset should have multiple hints");

    let mut rumours = HashSet::new();
    for day in 0..16 {
        let rumor = index
            .deterministic_rumor(world_seed, origin, neighbour, EconomyDay(day))
            .expect("missing rumour");
        rumours.insert(rumor);
    }

    assert!(
        rumours.len() > 1,
        "expected multiple distinct rumours across days"
    );
}

#[test]
fn rumor_absent_for_non_neighbors() {
    let path = workspace_path("assets/world/hubs_min.toml");
    let index = WorldIndex::from_path(&path).expect("failed to load world index");

    assert!(index
        .deterministic_rumor(42, HubId(0), HubId(999), EconomyDay(0))
        .is_none());
}
