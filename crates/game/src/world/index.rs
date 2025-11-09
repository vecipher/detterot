use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::OnceLock;

use anyhow::Context;
use serde::Deserialize;
use smallvec::SmallVec;

use crate::systems::economy::{HubId, RouteId, Weather};
use crate::world::loader::load_world_graph;
use crate::world::schema::{HubSpec, LinkSpec};
use crate::world::weather::load_weather_config;

static GRAPH: OnceLock<WorldGraphData> = OnceLock::new();

fn load_route_closures() -> anyhow::Result<HashSet<RouteId>> {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let primary = Path::new(manifest)
        .join("..")
        .join("..")
        .join("assets/world/closures.toml");
    let search_paths = [Path::new("assets/world/closures.toml"), primary.as_path()];

    let mut closures_path = None;
    for path in search_paths {
        if path.exists() {
            closures_path = Some(path.to_path_buf());
            break;
        }
    }

    if let Some(path) = closures_path {
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let closures: ClosuresConfig =
            toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;

        // Convert string IDs to RouteId - for legacy format compatibility
        let mut closed_routes = HashSet::new();
        for link_id in &closures.closed {
            if link_id.starts_with("L") && link_id.len() >= 3 {
                if let Ok(route_num) = link_id[1..].parse::<u16>() {
                    closed_routes.insert(RouteId(route_num));
                }
            }
        }
        Ok(closed_routes)
    } else {
        Ok(HashSet::new()) // No closures file, return empty set
    }
}

pub trait WorldIndex {
    fn neighbors(hub: HubId) -> SmallVec<[RouteId; 6]>;
    fn route_weather(route: RouteId) -> Weather;
    fn is_route_closed(route: RouteId) -> bool;
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

    fn is_route_closed(route: RouteId) -> bool {
        ensure_loaded().closed_routes.contains(&route)
    }
}

// Raw closure loading function for new graph format
fn load_route_closures_raw() -> anyhow::Result<HashSet<String>> {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let primary = Path::new(manifest)
        .join("..")
        .join("..")
        .join("assets/world/closures.toml");
    let search_paths = [Path::new("assets/world/closures.toml"), primary.as_path()];

    let mut closures_path = None;
    for path in search_paths {
        if path.exists() {
            closures_path = Some(path.to_path_buf());
            break;
        }
    }

    if let Some(path) = closures_path {
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let closures: ClosuresConfig =
            toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;

        // Return the raw string IDs without conversion
        Ok(closures.closed.into_iter().collect())
    } else {
        Ok(HashSet::new()) // No closures file, return empty set
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ClosuresConfig {
    #[serde(default)]
    closed: Vec<String>,
}

#[allow(dead_code)]
struct WorldGraphData {
    neighbors: HashMap<HubId, SmallVec<[RouteId; 6]>>,
    weather: HashMap<RouteId, Weather>,
    closed_routes: HashSet<RouteId>,
    #[allow(dead_code)]
    hub_names: HashMap<String, HubId>,
    #[allow(dead_code)]
    hub_specs: HashMap<HubId, HubSpec>,
}

fn ensure_loaded() -> &'static WorldGraphData {
    GRAPH.get_or_init(|| load_world_graph_data().expect("failed to load world graph"))
}

fn load_world_graph_data() -> anyhow::Result<WorldGraphData> {
    let manifest = env!("CARGO_MANIFEST_DIR");

    // For backward compatibility with tests and existing functionality,
    // load the legacy hubs_min.toml format first
    let legacy_primary = Path::new(manifest)
        .join("..")
        .join("..")
        .join("assets/world/hubs_min.toml");
    let legacy_search_paths = [
        Path::new("assets/world/hubs_min.toml"),
        legacy_primary.as_path(),
    ];

    if legacy_search_paths.iter().any(|path| path.exists()) {
        // Load legacy format to maintain compatibility with existing tests
        load_legacy_routes()
    } else {
        // If legacy doesn't exist, fall back to new format
        let new_primary = Path::new(manifest)
            .join("..")
            .join("..")
            .join("assets/world/graph_v1.toml");
        let new_search_paths = [
            Path::new("assets/world/graph_v1.toml"),
            new_primary.as_path(),
        ];

        if new_search_paths.iter().any(|path| path.exists()) {
            // Load new format
            let mut graph_path = None;
            for path in new_search_paths {
                if path.exists() {
                    graph_path = Some(path.to_path_buf());
                    break;
                }
            }

            let graph_path = graph_path.ok_or_else(|| {
                anyhow::anyhow!("missing world graph asset at {}", new_primary.display())
            })?;

            let world_graph = load_world_graph(&graph_path)?;

            // Load weather config
            let weather_primary = Path::new(manifest)
                .join("..")
                .join("..")
                .join("assets/world/weather.toml");
            let weather_search_paths = [
                Path::new("assets/world/weather.toml"),
                weather_primary.as_path(),
            ];

            let mut weather_config = None;
            for path in weather_search_paths {
                if path.exists() {
                    weather_config = Some(load_weather_config(path)?);
                    break;
                }
            }

            let weather_resolver = weather_config.unwrap_or_else(|| {
                // Default weather config if file doesn't exist
                use crate::world::weather::WeatherConfig;
                WeatherConfig::default()
            });

            // Convert the world graph to the data structures we need
            let mut hub_names = HashMap::new();
            let mut hub_specs = HashMap::new();

            // Sort hubs by their names to ensure deterministic HubId assignment
            let mut sorted_hubs: Vec<(&String, &HubSpec)> = world_graph.hubs.iter().collect();
            sorted_hubs.sort_by_key(|(name, _)| *name);

            for (i, (hub_name, hub_spec)) in sorted_hubs.into_iter().enumerate() {
                let hub_id = HubId(i as u16);
                hub_names.insert(hub_name.clone(), hub_id);
                hub_specs.insert(hub_id, hub_spec.clone());
            }

            let mut neighbors: HashMap<HubId, SmallVec<[RouteId; 6]>> = HashMap::new();
            let mut route_weather = HashMap::new();

            // Sort links by their names to ensure deterministic RouteId assignment
            let mut sorted_links: Vec<(&String, &LinkSpec)> = world_graph.links.iter().collect();
            sorted_links.sort_by_key(|(name, _)| *name);

            // Create a mapping from link names to RouteIds
            let mut link_to_route_id = HashMap::new();
            
            for (i, (link_name, link_spec)) in sorted_links.into_iter().enumerate() {
                let from_hub_id = hub_names
                    .get(&link_spec.from)
                    .ok_or_else(|| anyhow::anyhow!("unknown hub in link: {}", link_spec.from))?;
                let to_hub_id = hub_names
                    .get(&link_spec.to)
                    .ok_or_else(|| anyhow::anyhow!("unknown hub in link: {}", link_spec.to))?;

                let route_id = RouteId(i as u16);
                link_to_route_id.insert(link_name, route_id);

                neighbors.entry(*from_hub_id).or_default().push(route_id);
                neighbors.entry(*to_hub_id).or_default().push(route_id);

                // Determine weather for this route using weather resolver
                let weather = weather_resolver.resolve_weather(&link_spec.style, link_name);
                route_weather.insert(route_id, weather);
            }

            // Load route closures - convert link names to corresponding RouteIds
            let original_closures = load_route_closures_raw()?;
            let closed_routes = original_closures
                .iter()
                .filter_map(|link_name| link_to_route_id.get(link_name).copied())
                .collect();

            // Ensure no hub has more than 6 neighbors
            for list in neighbors.values_mut() {
                list.sort_by_key(|id| id.0);
                list.truncate(6);
            }

            Ok(WorldGraphData {
                neighbors,
                weather: route_weather,
                closed_routes,
                hub_names,
                hub_specs,
            })
        } else {
            Err(anyhow::anyhow!(
                "no world graph files found (neither legacy hubs_min.toml nor graph_v1.toml)"
            ))
        }
    }
}

fn load_legacy_routes() -> anyhow::Result<WorldGraphData> {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let primary = Path::new(manifest)
        .join("..")
        .join("..")
        .join("assets/world/hubs_min.toml");
    let search_paths = [Path::new("assets/world/hubs_min.toml"), primary.as_path()];

    let mut routes_path = None;
    for path in search_paths {
        if path.exists() {
            routes_path = Some(path.to_path_buf());
            break;
        }
    }

    let routes_path = routes_path
        .ok_or_else(|| anyhow::anyhow!("missing legacy routes asset at {}", primary.display()))?;

    use serde::Deserialize;

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct RoutesConfig {
        routes: Vec<RouteSpec>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct RouteSpec {
        id: RouteId,
        from: HubId,
        to: HubId,
        weather: Weather,
    }

    let raw = std::fs::read_to_string(&routes_path)
        .with_context(|| format!("reading {}", routes_path.display()))?;
    let config: RoutesConfig =
        toml::from_str(&raw).with_context(|| format!("parsing {}", routes_path.display()))?;

    let mut neighbors: HashMap<HubId, SmallVec<[RouteId; 6]>> = HashMap::new();
    let mut weather = HashMap::new();

    for route in &config.routes {
        neighbors.entry(route.from).or_default().push(route.id);
        neighbors.entry(route.to).or_default().push(route.id);
        weather.insert(route.id, route.weather);
    }

    // Load route closures
    let closed_routes = load_route_closures()?;

    for list in neighbors.values_mut() {
        list.sort_by_key(|id| id.0);
        list.truncate(6);
    }

    Ok(WorldGraphData {
        neighbors,
        weather,
        closed_routes,
        hub_names: HashMap::new(), // Empty for legacy format
        hub_specs: HashMap::new(), // Empty for legacy format
    })
}

#[cfg(test)]
#[path = "tests/forecast_determinism.rs"]
mod forecast_determinism;
#[cfg(test)]
#[path = "tests/neighbors_shape.rs"]
mod neighbors_shape;
