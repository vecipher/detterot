use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

use anyhow::Context;
use serde::Deserialize;
use smallvec::SmallVec;

use crate::systems::economy::{HubId, RouteId, Weather};

static ROUTES: OnceLock<RoutesData> = OnceLock::new();

pub trait WorldIndex {
    fn neighbors(hub: HubId) -> SmallVec<[RouteId; 6]>;
    fn route_weather(route: RouteId) -> Weather;
}

pub struct StaticWorldIndex;

impl WorldIndex for StaticWorldIndex {
    fn neighbors(hub: HubId) -> SmallVec<[RouteId; 6]> {
        ensure_loaded()
            .neighbors
            .get(&hub)
            .cloned()
            .unwrap_or_default()
    }

    fn route_weather(route: RouteId) -> Weather {
        ensure_loaded()
            .weather
            .get(&route)
            .copied()
            .unwrap_or(Weather::Clear)
    }
}

pub fn deterministic_rumor(seed: u64, route: RouteId) -> (RumorKind, u8) {
    let mut state = wyhash::wyhash(&route.0.to_le_bytes(), seed);
    let first = splitmix64(&mut state);
    let second = splitmix64(&mut state);

    let kind = match first % 3 {
        0 => RumorKind::Wind,
        1 => RumorKind::Fog,
        _ => RumorKind::Patrol,
    };
    let confidence = ((second % 51) + 50) as u8;
    (kind, confidence)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RumorKind {
    Wind,
    Fog,
    Patrol,
}

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

struct RoutesData {
    neighbors: HashMap<HubId, SmallVec<[RouteId; 6]>>,
    weather: HashMap<RouteId, Weather>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RoutesConfig {
    routes: Vec<RouteSpec>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RouteSpec {
    id: RouteId,
    from: HubId,
    to: HubId,
    weather: Weather,
}

fn ensure_loaded() -> &'static RoutesData {
    ROUTES.get_or_init(|| load_routes().expect("failed to load world index"))
}

fn load_routes() -> anyhow::Result<RoutesData> {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let primary = Path::new(manifest)
        .join("..")
        .join("..")
        .join("assets/world/hubs_min.toml");
    let search_paths = [Path::new("assets/world/hubs_min.toml"), primary.as_path()];
    for path in search_paths {
        if path.exists() {
            return parse_routes(path);
        }
    }
    Err(anyhow::anyhow!(
        "missing world index asset at {}",
        primary.display()
    ))
}

fn parse_routes(path: &Path) -> anyhow::Result<RoutesData> {
    let raw =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let config: RoutesConfig =
        toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;

    let mut neighbors: HashMap<HubId, SmallVec<[RouteId; 6]>> = HashMap::new();
    let mut weather = HashMap::new();
    for route in &config.routes {
        neighbors.entry(route.from).or_default().push(route.id);
        neighbors.entry(route.to).or_default().push(route.id);
        weather.insert(route.id, route.weather);
    }

    for list in neighbors.values_mut() {
        list.sort_by_key(|id| id.0);
        list.truncate(6);
    }

    Ok(RoutesData { neighbors, weather })
}

#[cfg(test)]
#[path = "tests/forecast_determinism.rs"]
mod forecast_determinism;
#[cfg(test)]
#[path = "tests/neighbors_shape.rs"]
mod neighbors_shape;
