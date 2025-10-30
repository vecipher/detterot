use std::path::PathBuf;

use crate::systems::economy::HubId;
use crate::world::WorldIndex;

fn workspace_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../{relative}"))
}

#[test]
fn neighbors_are_sorted_and_symmetric() {
    let path = workspace_path("assets/world/hubs_min.toml");
    let index = WorldIndex::from_path(&path).expect("failed to load world index");

    for hub in index.hub_ids() {
        let neighbors = index.neighbors(hub);
        let mut sorted = neighbors.to_vec();
        sorted.sort_by_key(|id| id.0);
        assert_eq!(neighbors, sorted, "neighbors for hub {:?} not sorted", hub);

        for neighbor in neighbors {
            assert!(
                index.neighbors(*neighbor).contains(&hub),
                "edge {:?} -> {:?} missing reverse entry",
                hub,
                neighbor
            );
            assert_ne!(hub, *neighbor, "self-loop detected at hub {:?}", hub);
        }
    }
}

#[test]
fn every_hub_has_weather_hints() {
    let path = workspace_path("assets/world/hubs_min.toml");
    let index = WorldIndex::from_path(&path).expect("failed to load world index");

    for hub in index.hub_ids() {
        let hints = index.weather_hints(hub);
        assert!(
            !hints.is_empty(),
            "expected at least one weather hint for hub {:?}",
            hub
        );
        for hint in hints {
            assert!(hint.weight() > 0, "weather hint weight must be positive");
        }
    }
}

#[test]
fn unknown_hub_has_no_neighbors() {
    let path = workspace_path("assets/world/hubs_min.toml");
    let index = WorldIndex::from_path(&path).expect("failed to load world index");

    assert!(index.neighbors(HubId(999)).is_empty());
}
