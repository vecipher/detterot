use std::collections::HashSet;
use std::path::PathBuf;

use crate::systems::economy::{HubId, RouteId};
use crate::world::index::{StaticWorldIndex, WorldIndex};
use serde::Deserialize;

fn asset_path(relative: &str) -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest).join("..").join("..").join(relative)
}

#[derive(Deserialize)]
struct RoutesConfig {
    routes: Vec<RouteRecord>,
}

#[derive(Deserialize)]
struct RouteRecord {
    id: RouteId,
    from: HubId,
    to: HubId,
}

#[test]
fn neighbor_lists_are_bounded() {
    let path = asset_path("assets/world/hubs_min.toml");
    let raw = std::fs::read_to_string(path).expect("read");
    let cfg: RoutesConfig = toml::from_str(&raw).expect("parse");

    let mut hubs: HashSet<HubId> = HashSet::new();
    for route in &cfg.routes {
        let _ = route.id;
        hubs.insert(route.from);
        hubs.insert(route.to);
    }

    for hub in hubs {
        let neighbors = StaticWorldIndex::neighbors(hub);
        assert!(neighbors.len() <= 6);
        for route in neighbors {
            assert_ne!(route.0, 0);
        }
    }
}
