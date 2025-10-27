use std::fmt;

use blake3::Hasher;
use serde::{
    de::Error as DeError,
    ser::{SerializeMap, SerializeStruct},
    Deserialize, Deserializer, Serialize, Serializer,
};
use serde_json::{self, Value};

/// Minimal deterministic RNG wrapper (32-bit LCG).
#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct DetRng {
    state: u32,
}

impl DetRng {
    pub fn from_seed(seed: u64) -> Self {
        Self {
            state: (seed as u32).wrapping_mul(747_796_405) ^ 2_891_336_453,
        }
    }

    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        self.state
    }

    #[inline]
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() as f32) / (u32::MAX as f32)
    }
}

/// Canonical command emitted by deterministic gameplay systems.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "PascalCase")]
pub enum Command {
    #[serde(rename_all = "camelCase")]
    Spawn {
        kind: String,
        x_mm: i32,
        y_mm: i32,
        z_mm: i32,
    },
    #[serde(rename_all = "camelCase")]
    Meter { key: String, value: i32 },
}

impl Command {
    pub fn spawn(kind: &str, x_mm: i32, y_mm: i32, z_mm: i32) -> Self {
        Self::Spawn {
            kind: kind.to_owned(),
            x_mm,
            y_mm,
            z_mm,
        }
    }

    pub fn meter(key: &str, value: i32) -> Self {
        Self::Meter {
            key: key.to_owned(),
            value,
        }
    }
}

/// Command coupled with a fixed-tick timestamp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimedCommand {
    pub tick: u32,
    pub command: Command,
}

impl Serialize for TimedCommand {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("t", &self.tick)?;
        match &self.command {
            Command::Spawn {
                kind,
                x_mm,
                y_mm,
                z_mm,
            } => {
                let payload = SpawnPayload {
                    kind,
                    x_mm: *x_mm,
                    y_mm: *y_mm,
                    z_mm: *z_mm,
                };
                map.serialize_entry("Spawn", &payload)?;
            }
            Command::Meter { key, value } => {
                let payload = MeterPayload { key, value: *value };
                map.serialize_entry("Meter", &payload)?;
            }
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for TimedCommand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let mut map = match value {
            Value::Object(map) => map,
            _ => return Err(D::Error::custom("command must be an object")),
        };

        let tick_value = map
            .remove("t")
            .ok_or_else(|| D::Error::custom("missing t field"))?;
        let tick = tick_value
            .as_u64()
            .ok_or_else(|| D::Error::custom("t must be an unsigned integer"))?;

        if let Some(spawn) = map.remove("Spawn") {
            let payload: SpawnPayloadOwned =
                SpawnPayloadOwned::deserialize(spawn).map_err(D::Error::custom)?;
            return Ok(Self {
                tick: tick as u32,
                command: Command::Spawn {
                    kind: payload.kind,
                    x_mm: payload.x_mm,
                    y_mm: payload.y_mm,
                    z_mm: payload.z_mm,
                },
            });
        }

        if let Some(meter) = map.remove("Meter") {
            let payload: MeterPayloadOwned =
                MeterPayloadOwned::deserialize(meter).map_err(D::Error::custom)?;
            return Ok(Self {
                tick: tick as u32,
                command: Command::Meter {
                    key: payload.key,
                    value: payload.value,
                },
            });
        }

        Err(D::Error::custom("missing Spawn or Meter payload"))
    }
}

#[derive(Serialize)]
struct SpawnPayload<'a> {
    kind: &'a str,
    x_mm: i32,
    y_mm: i32,
    z_mm: i32,
}

#[derive(Serialize)]
struct MeterPayload<'a> {
    key: &'a str,
    value: i32,
}

#[derive(Deserialize)]
struct SpawnPayloadOwned {
    kind: String,
    x_mm: i32,
    y_mm: i32,
    z_mm: i32,
}

#[derive(Deserialize)]
struct MeterPayloadOwned {
    key: String,
    value: i32,
}

/// Auxiliary input events that are not part of the canonical hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub enum InputEvent {
    #[default]
    None,
    #[serde(rename_all = "camelCase")]
    Input { kind: String, key: String },
}

/// Timed input wrapper mirroring [`TimedCommand`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimedInput {
    #[serde(rename = "t")]
    pub tick: u32,
    #[serde(flatten)]
    pub event: InputEvent,
}

/// Metadata describing the recorded simulation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordMeta {
    pub schema: u32,
    pub world_seed: String,
    pub link_id: String,
    pub rulepack: String,
    pub weather: String,
    pub rng_salt: String,
}

/// Canonical record (meta + outputs + auxiliary inputs).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub meta: RecordMeta,
    #[serde(default)]
    pub commands: Vec<TimedCommand>,
    #[serde(default)]
    pub inputs: Vec<TimedInput>,
}

impl Record {
    pub fn new(meta: RecordMeta) -> Self {
        Self {
            meta,
            commands: Vec::new(),
            inputs: Vec::new(),
        }
    }

    pub fn push_command(&mut self, tick: u32, command: Command) {
        self.commands.push(TimedCommand { tick, command });
    }

    pub fn push_input(&mut self, tick: u32, event: InputEvent) {
        self.inputs.push(TimedInput { tick, event });
    }

    pub fn hash_hex(&self) -> Result<String, serde_json::Error> {
        let bytes = canonical_json_bytes(&HashableRecordView {
            meta: &self.meta,
            commands: &self.commands,
        })?;
        let mut hasher = Hasher::new();
        hasher.update(&bytes);
        Ok(hasher.finalize().to_hex().to_string())
    }

    pub fn canonical_json(&self) -> Result<String, serde_json::Error> {
        let bytes = canonical_json_bytes(self)?;
        Ok(String::from_utf8(bytes).expect("canonical output is valid UTF-8"))
    }
}

struct HashableRecordView<'a> {
    meta: &'a RecordMeta,
    commands: &'a [TimedCommand],
}

impl<'a> Serialize for HashableRecordView<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Record", 2)?;
        state.serialize_field("meta", self.meta)?;
        state.serialize_field("commands", self.commands)?;
        state.end()
    }
}

fn canonical_json_bytes<T>(value: &T) -> Result<Vec<u8>, serde_json::Error>
where
    T: Serialize,
{
    let mut json = serde_json::to_value(value)?;
    sort_value(&mut json);
    let mut buf = Vec::new();
    {
        let formatter = serde_json::ser::CompactFormatter {};
        let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
        json.serialize(&mut ser)?;
    }
    buf.push(b'\n');
    Ok(buf)
}

fn sort_value(value: &mut Value) {
    match value {
        Value::Array(items) => {
            for item in items.iter_mut() {
                sort_value(item);
            }
        }
        Value::Object(map) => {
            let mut entries: Vec<_> = map
                .iter_mut()
                .map(|(key, value)| (key.clone(), value.take()))
                .collect();
            map.clear();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            for (key, mut value) in entries {
                sort_value(&mut value);
                map.insert(key, value);
            }
        }
        _ => {}
    }
}

impl fmt::Display for Record {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.canonical_json() {
            Ok(text) => write!(f, "{}", text),
            Err(_) => Err(fmt::Error),
        }
    }
}
