use std::collections::BTreeMap;
use std::fmt;

use blake3::Hasher;
use serde::de::DeserializeOwned;
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Canonical JSON serialization error.
#[derive(Debug)]
pub enum CanonicalJsonError {
    Serialize(serde_json::Error),
}

impl fmt::Display for CanonicalJsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Serialize(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for CanonicalJsonError {}

impl From<serde_json::Error> for CanonicalJsonError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serialize(value)
    }
}

/// Canonical JSON serialization helper. Ensures deterministic key ordering and
/// appends a trailing newline so hash inputs are stable across platforms.
pub fn canonical_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, CanonicalJsonError> {
    let mut json = serde_json::to_value(value)?;
    canonicalize_value(&mut json);
    let mut bytes = serde_json::to_vec(&json)?;
    if !bytes.ends_with(b"\n") {
        bytes.push(b'\n');
    }
    Ok(bytes)
}

/// Deserialize a value that was produced with [`canonical_json_bytes`].
pub fn from_canonical_json_bytes<T: DeserializeOwned>(
    bytes: &[u8],
) -> Result<T, CanonicalJsonError> {
    let mut json: Value = serde_json::from_slice(bytes)?;
    canonicalize_value(&mut json);
    Ok(serde_json::from_value(json)?)
}

fn canonicalize_value(value: &mut Value) {
    match value {
        Value::Object(map) => {
            let mut sorted = BTreeMap::new();
            for (key, mut v) in std::mem::take(map) {
                canonicalize_value(&mut v);
                sorted.insert(key, v);
            }
            *map = sorted.into_iter().collect();
        }
        Value::Array(array) => {
            for item in array {
                canonicalize_value(item);
            }
        }
        _ => {}
    }
}

/// Deterministic command emitted by gameplay systems.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub t: u32,
    pub kind: CommandKind,
}

impl Command {
    pub fn spawn_at(t: u32, kind: &str, x_mm: i32, y_mm: i32, z_mm: i32) -> Self {
        Self {
            t,
            kind: CommandKind::Spawn(SpawnCommand {
                kind: kind.to_owned(),
                x_mm,
                y_mm,
                z_mm,
            }),
        }
    }

    pub fn meter_at(t: u32, key: &str, value: i32) -> Self {
        Self {
            t,
            kind: CommandKind::Meter(MeterCommand {
                key: key.to_owned(),
                value,
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandKind {
    Spawn(SpawnCommand),
    Meter(MeterCommand),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpawnCommand {
    pub kind: String,
    pub x_mm: i32,
    pub y_mm: i32,
    pub z_mm: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeterCommand {
    pub key: String,
    pub value: i32,
}

impl Serialize for Command {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("t", &self.t)?;
        match &self.kind {
            CommandKind::Spawn(cmd) => map.serialize_entry("Spawn", cmd)?,
            CommandKind::Meter(cmd) => map.serialize_entry("Meter", cmd)?,
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for Command {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        match value {
            Value::Object(mut map) => {
                let t_value = map
                    .remove("t")
                    .ok_or_else(|| serde::de::Error::custom("missing command tick"))?;
                let t: u32 = serde_json::from_value(t_value).map_err(serde::de::Error::custom)?;

                if map.len() != 1 {
                    return Err(serde::de::Error::custom("expected single command variant"));
                }

                let (key, value) = map.into_iter().next().unwrap();
                let kind = match key.as_str() {
                    "Spawn" => {
                        let cmd: SpawnCommand =
                            serde_json::from_value(value).map_err(serde::de::Error::custom)?;
                        CommandKind::Spawn(cmd)
                    }
                    "Meter" => {
                        let cmd: MeterCommand =
                            serde_json::from_value(value).map_err(serde::de::Error::custom)?;
                        CommandKind::Meter(cmd)
                    }
                    other => {
                        return Err(serde::de::Error::custom(format!(
                            "unknown command type: {other}"
                        )))
                    }
                };

                Ok(Command { t, kind })
            }
            _ => Err(serde::de::Error::custom("expected object for command")),
        }
    }
}

/// Player input captured alongside authoritative commands (not hashed).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct InputEvent {
    pub t: u32,
    pub input: String,
}

/// Metadata recorded for a deterministic leg.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RecordMeta {
    pub schema: u32,
    pub world_seed: String,
    pub link_id: String,
    pub rulepack: String,
    pub weather: String,
    pub rng_salt: String,
    #[serde(default)]
    pub day: u32,
    #[serde(default)]
    pub pp: u16,
    #[serde(default)]
    pub density_per_10k: u32,
    #[serde(default)]
    pub cadence_per_min: u32,
    #[serde(default)]
    pub mission_minutes: u32,
    #[serde(default)]
    pub player_rating: u8,
}

#[derive(Serialize)]
struct RecordMetaHashView<'a> {
    schema: u32,
    world_seed: &'a str,
    link_id: &'a str,
    rulepack: &'a str,
    weather: &'a str,
    rng_salt: &'a str,
}

impl RecordMeta {
    fn hash_view(&self) -> RecordMetaHashView<'_> {
        RecordMetaHashView {
            schema: self.schema,
            world_seed: &self.world_seed,
            link_id: &self.link_id,
            rulepack: &self.rulepack,
            weather: &self.weather,
            rng_salt: &self.rng_salt,
        }
    }
}

/// Canonical record encompassing commands and auxiliary inputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Record {
    pub meta: RecordMeta,
    #[serde(default)]
    pub commands: Vec<Command>,
    #[serde(default)]
    pub inputs: Vec<InputEvent>,
}

impl Record {
    /// Returns canonical bytes for the hash-relevant view of the record.
    fn hash_view_bytes(&self) -> Result<Vec<u8>, CanonicalJsonError> {
        #[derive(Serialize)]
        struct HashView<'a> {
            meta: RecordMetaHashView<'a>,
            commands: &'a [Command],
        }
        canonical_json_bytes(&HashView {
            meta: self.meta.hash_view(),
            commands: &self.commands,
        })
    }
}

/// Compute the canonical BLAKE3 hash for the provided record.
pub fn hash_record(record: &Record) -> Result<String, CanonicalJsonError> {
    let bytes = record.hash_view_bytes()?;
    let mut hasher = Hasher::new();
    hasher.update(&bytes);
    Ok(hasher.finalize().to_hex().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_round_trip() {
        let record = Record {
            meta: RecordMeta {
                schema: 1,
                world_seed: "seed".into(),
                link_id: "link".into(),
                rulepack: "rulepack".into(),
                weather: "Clear".into(),
                rng_salt: "salt".into(),
                day: 3,
                pp: 120,
                density_per_10k: 5,
                cadence_per_min: 3,
                mission_minutes: 8,
                player_rating: 50,
            },
            commands: vec![Command::meter_at(0, "danger_score", 42)],
            inputs: vec![InputEvent {
                t: 7,
                input: "KeyDown(Q)".into(),
            }],
        };
        let bytes = canonical_json_bytes(&record).unwrap();
        let parsed: Record = from_canonical_json_bytes(&bytes).unwrap();
        assert_eq!(parsed, record);
    }

    #[test]
    fn hash_is_stable() {
        let mut record = Record {
            meta: RecordMeta {
                schema: 1,
                world_seed: "alpha".into(),
                link_id: "link".into(),
                rulepack: "assets/rulepack.toml".into(),
                weather: "Fog".into(),
                rng_salt: "1234".into(),
                day: 4,
                pp: 80,
                density_per_10k: 7,
                cadence_per_min: 5,
                mission_minutes: 9,
                player_rating: 60,
            },
            ..Record::default()
        };
        record
            .commands
            .push(Command::meter_at(0, "danger_score", 123));
        let hash_a = hash_record(&record).unwrap();
        let hash_b = hash_record(&record).unwrap();
        assert_eq!(hash_a, hash_b);
    }
}
