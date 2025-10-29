use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use bevy::prelude::Resource;
use serde::Deserialize;
use thiserror::Error;

use crate::systems::economy::{EconomyDay, HubId, Weather};

/// Schema version supported by this loader.
const SCHEMA_VERSION: u32 = 1;

/// In-memory representation of the world index used by trading and UI code.
#[derive(Debug, Clone, PartialEq, Eq, Resource)]
pub struct WorldIndex {
    hubs: BTreeMap<HubId, HubRecord>,
}

impl WorldIndex {
    /// Loads the world index from the provided TOML source string.
    pub fn from_str(raw: &str) -> Result<Self, WorldIndexLoadError> {
        let doc: WorldIndexDocument = toml::from_str(raw)?;
        if doc.schema != SCHEMA_VERSION {
            return Err(WorldIndexLoadError::UnsupportedSchema {
                found: doc.schema,
                expected: SCHEMA_VERSION,
            });
        }

        let mut hubs = BTreeMap::new();
        for hub in doc.hubs {
            let hub_id = HubId(hub.id);
            if hubs.contains_key(&hub_id) {
                return Err(WorldIndexLoadError::DuplicateHub(hub_id));
            }

            let neighbors: Vec<HubId> = hub.neighbors.into_iter().map(HubId).collect();
            let mut sorted = neighbors.clone();
            sorted.sort_by_key(|id| id.0);
            sorted.dedup();
            if sorted.len() != neighbors.len() {
                return Err(WorldIndexLoadError::DuplicateNeighbor(hub_id));
            }

            let hints = hub
                .weather_hints
                .into_iter()
                .map(WeatherHint::try_from)
                .collect::<Result<Vec<_>, _>>()?;

            if hints.is_empty() {
                return Err(WorldIndexLoadError::MissingWeatherHints(hub_id));
            }

            hubs.insert(
                hub_id,
                HubRecord {
                    neighbors: sorted,
                    weather_hints: hints,
                },
            );
        }

        for (hub_id, record) in &hubs {
            for neighbor in &record.neighbors {
                if let Some(neighbor_record) = hubs.get(neighbor) {
                    if !neighbor_record.neighbors.contains(hub_id) {
                        return Err(WorldIndexLoadError::AsymmetricEdge {
                            hub: *hub_id,
                            neighbor: *neighbor,
                        });
                    }
                } else {
                    return Err(WorldIndexLoadError::UnknownNeighbor {
                        hub: *hub_id,
                        neighbor: *neighbor,
                    });
                }
            }
        }

        Ok(Self { hubs })
    }

    /// Loads the world index from the provided file path.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, WorldIndexLoadError> {
        let raw = fs::read_to_string(path)?;
        Self::from_str(&raw)
    }

    /// Returns an iterator over all known hub identifiers.
    pub fn hub_ids(&self) -> impl Iterator<Item = HubId> + '_ {
        self.hubs.keys().copied()
    }

    /// Returns the neighbours connected to the specified hub in deterministic order.
    pub fn neighbors(&self, hub: HubId) -> &[HubId] {
        self.hubs
            .get(&hub)
            .map(|record| record.neighbors.as_slice())
            .unwrap_or(&[])
    }

    /// Returns the weather hints associated with the specified hub.
    pub fn weather_hints(&self, hub: HubId) -> &[WeatherHint] {
        self.hubs
            .get(&hub)
            .map(|record| record.weather_hints.as_slice())
            .unwrap_or(&[])
    }

    /// Generates a deterministic rumour about weather conditions for a neighbour hub.
    pub fn deterministic_rumor(
        &self,
        world_seed: u64,
        origin: HubId,
        neighbour: HubId,
        day: EconomyDay,
    ) -> Option<Weather> {
        let record = self.hubs.get(&origin)?;
        if !record.neighbors.contains(&neighbour) {
            return None;
        }
        let hints = &record.weather_hints;
        if hints.is_empty() {
            return None;
        }

        let mut key = [0u8; 32];
        key[0..8].copy_from_slice(&world_seed.to_le_bytes());
        key[8..16].copy_from_slice(&(origin.0 as u64).to_le_bytes());
        key[16..24].copy_from_slice(&(neighbour.0 as u64).to_le_bytes());
        key[24..32].copy_from_slice(&(day.0 as u64).to_le_bytes());

        let hash = wyhash::wyhash(&key, 0);
        let mut state = hash;
        let draw = splitmix64(&mut state);

        let total_weight: u64 = hints.iter().map(|hint| hint.weight as u64).sum();
        if total_weight == 0 {
            return None;
        }

        let mut cumulative = 0u64;
        for hint in hints {
            cumulative += hint.weight as u64;
            if draw % total_weight < cumulative {
                return Some(hint.weather);
            }
        }

        hints.last().map(|hint| hint.weather)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HubRecord {
    neighbors: Vec<HubId>,
    weather_hints: Vec<WeatherHint>,
}

/// Weighted hint describing the prevailing weather at a hub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeatherHint {
    weather: Weather,
    weight: u16,
}

impl WeatherHint {
    /// Weather associated with this hint.
    pub fn weather(&self) -> Weather {
        self.weather
    }

    /// Weight assigned to this hint.
    pub fn weight(&self) -> u16 {
        self.weight
    }
}

impl TryFrom<WeatherHintDocument> for WeatherHint {
    type Error = WorldIndexLoadError;

    fn try_from(value: WeatherHintDocument) -> Result<Self, Self::Error> {
        if value.weight == 0 {
            return Err(WorldIndexLoadError::ZeroWeightHint);
        }

        Ok(Self {
            weather: value.weather,
            weight: value.weight,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorldIndexDocument {
    schema: u32,
    #[serde(default, rename = "hub")]
    hubs: Vec<HubDocument>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HubDocument {
    id: u16,
    neighbors: Vec<u16>,
    #[serde(default, rename = "weather_hint")]
    weather_hints: Vec<WeatherHintDocument>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WeatherHintDocument {
    weather: Weather,
    weight: u16,
}

/// Errors that can occur while loading the world index.
#[derive(Debug, Error)]
pub enum WorldIndexLoadError {
    #[error("failed to read world index: {0}")]
    Read(#[from] std::io::Error),
    #[error("failed to parse world index: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("unsupported world index schema: found {found}, expected {expected}")]
    UnsupportedSchema { found: u32, expected: u32 },
    #[error("duplicate hub entry detected for id {0:?}")]
    DuplicateHub(HubId),
    #[error("duplicate neighbour detected while loading hub {0:?}")]
    DuplicateNeighbor(HubId),
    #[error("hub {hub:?} references unknown neighbour {neighbor:?}")]
    UnknownNeighbor { hub: HubId, neighbor: HubId },
    #[error("edge between {hub:?} and {neighbor:?} is not symmetric")]
    AsymmetricEdge { hub: HubId, neighbor: HubId },
    #[error("hub {0:?} is missing weather hints")]
    MissingWeatherHints(HubId),
    #[error("weather hints must have a positive weight")]
    ZeroWeightHint,
}

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}
