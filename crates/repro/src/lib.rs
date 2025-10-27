use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

use serde_json::{json, Map, Number, Value};
use thiserror::Error;

/// Command emitted by deterministic systems.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub t: u32,
    pub kind: CommandKind,
}

/// Variants of commands that are part of the authoritative record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandKind {
    Spawn {
        kind: String,
        x_mm: i32,
        y_mm: i32,
        z_mm: i32,
    },
    Meter {
        key: String,
        value: i32,
    },
}

/// Auxiliary input event stored alongside the authoritative log (not hashed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Input {
    pub t: u32,
    pub kind: InputKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputKind {
    Button { key: String, pressed: bool },
    Axis { axis: String, value: i32 },
}

/// Metadata header for each record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordMeta {
    pub schema: u32,
    pub world_seed: String,
    pub link_id: String,
    pub rulepack: String,
    pub weather: String,
    pub rng_salt: String,
    pub pp: u32,
    pub mission_minutes: u32,
    pub density_per_10k: u32,
    pub cadence_per_min: u32,
    pub player_rating: u8,
    pub day: u32,
}

/// Canonical deterministic record structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub meta: RecordMeta,
    pub commands: Vec<Command>,
    pub inputs: Vec<Input>,
}

impl Record {
    pub fn new(meta: RecordMeta) -> Self {
        Self {
            meta,
            commands: Vec::new(),
            inputs: Vec::new(),
        }
    }

    pub fn add_command(&mut self, command: Command) {
        self.commands.push(command);
    }

    pub fn add_input(&mut self, input: Input) {
        self.inputs.push(input);
    }

    pub fn to_writer<W: Write>(&self, mut writer: W) -> Result<(), Error> {
        let value = self.to_json_value();
        write_canonical_value(&value, &mut writer)?;
        writer.write_all(b"\n")?;
        Ok(())
    }

    pub fn write_to_path<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        if let Some(parent) = path.as_ref().parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let mut file = File::create(path)?;
        self.to_writer(&mut file)
    }

    pub fn from_reader<R: Read>(mut reader: R) -> Result<Self, Error> {
        let mut buf = String::new();
        reader.read_to_string(&mut buf)?;
        let value: Value = serde_json::from_str(&buf)?;
        Self::from_json_value(value)
    }

    pub fn read_from_path<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let file = File::open(path)?;
        Self::from_reader(file)
    }

    pub fn hash(&self) -> Result<String, Error> {
        let hashed = json!({
            "meta": meta_to_hashed_value(&self.meta),
            "commands": commands_to_value(&self.commands),
        });
        let mut bytes = Vec::new();
        write_canonical_value(&hashed, &mut bytes)?;
        Ok(blake3::hash(&bytes).to_hex().to_string())
    }

    fn to_json_value(&self) -> Value {
        let mut map = Map::new();
        map.insert("meta".to_owned(), meta_to_value(&self.meta));
        map.insert("commands".to_owned(), commands_to_value(&self.commands));
        map.insert("inputs".to_owned(), inputs_to_value(&self.inputs));
        Value::Object(map)
    }

    fn from_json_value(value: Value) -> Result<Self, Error> {
        let obj = value
            .as_object()
            .ok_or_else(|| Error::Invalid("record must be JSON object".into()))?;
        let meta_value = obj
            .get("meta")
            .ok_or_else(|| Error::Invalid("missing meta".into()))?;
        let commands_value = obj
            .get("commands")
            .ok_or_else(|| Error::Invalid("missing commands".into()))?;
        let inputs_value = obj
            .get("inputs")
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new()));

        Ok(Self {
            meta: value_to_meta(meta_value)?,
            commands: value_to_commands(commands_value)?,
            inputs: value_to_inputs(&inputs_value)?,
        })
    }
}

fn meta_to_value(meta: &RecordMeta) -> Value {
    let mut map = Map::new();
    map.insert("schema".to_owned(), Number::from(meta.schema).into());
    map.insert(
        "world_seed".to_owned(),
        Value::String(meta.world_seed.clone()),
    );
    map.insert("link_id".to_owned(), Value::String(meta.link_id.clone()));
    map.insert("rulepack".to_owned(), Value::String(meta.rulepack.clone()));
    map.insert("weather".to_owned(), Value::String(meta.weather.clone()));
    map.insert("rng_salt".to_owned(), Value::String(meta.rng_salt.clone()));
    map.insert("pp".to_owned(), Number::from(meta.pp).into());
    map.insert(
        "mission_minutes".to_owned(),
        Number::from(meta.mission_minutes).into(),
    );
    map.insert(
        "density_per_10k".to_owned(),
        Number::from(meta.density_per_10k).into(),
    );
    map.insert(
        "cadence_per_min".to_owned(),
        Number::from(meta.cadence_per_min).into(),
    );
    map.insert(
        "player_rating".to_owned(),
        Number::from(meta.player_rating).into(),
    );
    map.insert("day".to_owned(), Number::from(meta.day).into());
    Value::Object(map)
}

fn meta_to_hashed_value(meta: &RecordMeta) -> Value {
    let mut map = Map::new();
    map.insert("schema".to_owned(), Number::from(meta.schema).into());
    map.insert(
        "world_seed".to_owned(),
        Value::String(meta.world_seed.clone()),
    );
    map.insert("link_id".to_owned(), Value::String(meta.link_id.clone()));
    map.insert("rulepack".to_owned(), Value::String(meta.rulepack.clone()));
    map.insert("weather".to_owned(), Value::String(meta.weather.clone()));
    map.insert("rng_salt".to_owned(), Value::String(meta.rng_salt.clone()));
    Value::Object(map)
}

fn value_to_meta(value: &Value) -> Result<RecordMeta, Error> {
    let obj = value
        .as_object()
        .ok_or_else(|| Error::Invalid("meta must be object".into()))?;
    let schema = obj
        .get("schema")
        .and_then(Value::as_u64)
        .ok_or_else(|| Error::Invalid("schema must be u64".into()))?;
    let world_seed = obj
        .get("world_seed")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Invalid("world_seed must be string".into()))?;
    let link_id = obj
        .get("link_id")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Invalid("link_id must be string".into()))?;
    let rulepack = obj
        .get("rulepack")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Invalid("rulepack must be string".into()))?;
    let weather = obj
        .get("weather")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Invalid("weather must be string".into()))?;
    let rng_salt = obj
        .get("rng_salt")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Invalid("rng_salt must be string".into()))?;
    let pp = obj.get("pp").and_then(Value::as_u64).unwrap_or(0) as u32;
    let mission_minutes = obj
        .get("mission_minutes")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;
    let density_per_10k = obj
        .get("density_per_10k")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;
    let cadence_per_min = obj
        .get("cadence_per_min")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;
    let player_rating = obj
        .get("player_rating")
        .and_then(Value::as_u64)
        .map(|value| u8::try_from(value).unwrap_or(50))
        .unwrap_or(50);
    let day = obj.get("day").and_then(Value::as_u64).unwrap_or(0) as u32;
    Ok(RecordMeta {
        schema: schema as u32,
        world_seed: world_seed.to_owned(),
        link_id: link_id.to_owned(),
        rulepack: rulepack.to_owned(),
        weather: weather.to_owned(),
        rng_salt: rng_salt.to_owned(),
        pp,
        mission_minutes,
        density_per_10k,
        cadence_per_min,
        player_rating,
        day,
    })
}

fn commands_to_value(commands: &[Command]) -> Value {
    let mut array = Vec::with_capacity(commands.len());
    for command in commands {
        let mut map = Map::new();
        map.insert("t".to_owned(), Number::from(command.t).into());
        match &command.kind {
            CommandKind::Spawn {
                kind,
                x_mm,
                y_mm,
                z_mm,
            } => {
                let mut inner = Map::new();
                inner.insert("kind".to_owned(), Value::String(kind.clone()));
                inner.insert("x_mm".to_owned(), Number::from(*x_mm).into());
                inner.insert("y_mm".to_owned(), Number::from(*y_mm).into());
                inner.insert("z_mm".to_owned(), Number::from(*z_mm).into());
                map.insert("Spawn".to_owned(), Value::Object(inner));
            }
            CommandKind::Meter { key, value } => {
                let mut inner = Map::new();
                inner.insert("key".to_owned(), Value::String(key.clone()));
                inner.insert("value".to_owned(), Number::from(*value).into());
                map.insert("Meter".to_owned(), Value::Object(inner));
            }
        }
        array.push(Value::Object(map));
    }
    Value::Array(array)
}

fn value_to_commands(value: &Value) -> Result<Vec<Command>, Error> {
    let array = value
        .as_array()
        .ok_or_else(|| Error::Invalid("commands must be array".into()))?;
    let mut commands = Vec::with_capacity(array.len());
    for entry in array {
        let obj = entry
            .as_object()
            .ok_or_else(|| Error::Invalid("command must be object".into()))?;
        let t = obj
            .get("t")
            .and_then(Value::as_u64)
            .ok_or_else(|| Error::Invalid("command missing tick".into()))?;
        if let Some(inner) = obj.get("Spawn") {
            let inner_obj = inner
                .as_object()
                .ok_or_else(|| Error::Invalid("Spawn value must be object".into()))?;
            let kind = inner_obj
                .get("kind")
                .and_then(Value::as_str)
                .ok_or_else(|| Error::Invalid("Spawn.kind missing".into()))?;
            let x_mm = inner_obj
                .get("x_mm")
                .and_then(Value::as_i64)
                .ok_or_else(|| Error::Invalid("Spawn.x_mm missing".into()))?;
            let y_mm = inner_obj
                .get("y_mm")
                .and_then(Value::as_i64)
                .ok_or_else(|| Error::Invalid("Spawn.y_mm missing".into()))?;
            let z_mm = inner_obj
                .get("z_mm")
                .and_then(Value::as_i64)
                .ok_or_else(|| Error::Invalid("Spawn.z_mm missing".into()))?;
            commands.push(Command {
                t: t as u32,
                kind: CommandKind::Spawn {
                    kind: kind.to_owned(),
                    x_mm: x_mm as i32,
                    y_mm: y_mm as i32,
                    z_mm: z_mm as i32,
                },
            });
        } else if let Some(inner) = obj.get("Meter") {
            let inner_obj = inner
                .as_object()
                .ok_or_else(|| Error::Invalid("Meter value must be object".into()))?;
            let key = inner_obj
                .get("key")
                .and_then(Value::as_str)
                .ok_or_else(|| Error::Invalid("Meter.key missing".into()))?;
            let value_i = inner_obj
                .get("value")
                .and_then(Value::as_i64)
                .ok_or_else(|| Error::Invalid("Meter.value missing".into()))?;
            commands.push(Command {
                t: t as u32,
                kind: CommandKind::Meter {
                    key: key.to_owned(),
                    value: value_i as i32,
                },
            });
        } else {
            return Err(Error::Invalid("unknown command variant".into()));
        }
    }
    Ok(commands)
}

fn inputs_to_value(inputs: &[Input]) -> Value {
    let mut array = Vec::with_capacity(inputs.len());
    for input in inputs {
        let mut map = Map::new();
        map.insert("t".to_owned(), Number::from(input.t).into());
        let mut inner = Map::new();
        match &input.kind {
            InputKind::Button { key, pressed } => {
                inner.insert("kind".to_owned(), Value::String("Button".into()));
                inner.insert("key".to_owned(), Value::String(key.clone()));
                inner.insert("pressed".to_owned(), Value::Bool(*pressed));
            }
            InputKind::Axis { axis, value } => {
                inner.insert("kind".to_owned(), Value::String("Axis".into()));
                inner.insert("axis".to_owned(), Value::String(axis.clone()));
                inner.insert("value".to_owned(), Number::from(*value).into());
            }
        }
        map.insert("Input".to_owned(), Value::Object(inner));
        array.push(Value::Object(map));
    }
    Value::Array(array)
}

fn value_to_inputs(value: &Value) -> Result<Vec<Input>, Error> {
    if value.is_null() {
        return Ok(Vec::new());
    }
    let array = value
        .as_array()
        .ok_or_else(|| Error::Invalid("inputs must be array".into()))?;
    let mut inputs = Vec::with_capacity(array.len());
    for entry in array {
        let obj = entry
            .as_object()
            .ok_or_else(|| Error::Invalid("input must be object".into()))?;
        let t = obj
            .get("t")
            .and_then(Value::as_u64)
            .ok_or_else(|| Error::Invalid("input missing tick".into()))?;
        let inner_obj = obj
            .get("Input")
            .and_then(Value::as_object)
            .ok_or_else(|| Error::Invalid("Input object missing".into()))?;
        let kind = inner_obj
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::Invalid("Input.kind missing".into()))?;
        let input_kind = match kind {
            "Button" => {
                let key = inner_obj
                    .get("key")
                    .and_then(Value::as_str)
                    .ok_or_else(|| Error::Invalid("Button.key missing".into()))?;
                let pressed = inner_obj
                    .get("pressed")
                    .and_then(Value::as_bool)
                    .ok_or_else(|| Error::Invalid("Button.pressed missing".into()))?;
                InputKind::Button {
                    key: key.to_owned(),
                    pressed,
                }
            }
            "Axis" => {
                let axis = inner_obj
                    .get("axis")
                    .and_then(Value::as_str)
                    .ok_or_else(|| Error::Invalid("Axis.axis missing".into()))?;
                let value_i = inner_obj
                    .get("value")
                    .and_then(Value::as_i64)
                    .ok_or_else(|| Error::Invalid("Axis.value missing".into()))?;
                InputKind::Axis {
                    axis: axis.to_owned(),
                    value: value_i as i32,
                }
            }
            _ => return Err(Error::Invalid("unknown input kind".into())),
        };
        inputs.push(Input {
            t: t as u32,
            kind: input_kind,
        });
    }
    Ok(inputs)
}

fn write_canonical_value<W: Write>(value: &Value, writer: &mut W) -> Result<(), Error> {
    match value {
        Value::Null => writer.write_all(b"null")?,
        Value::Bool(b) => writer.write_all(if *b { b"true" } else { b"false" })?,
        Value::Number(n) => writer.write_all(n.to_string().as_bytes())?,
        Value::String(s) => {
            let quoted = serde_json::to_string(s)?;
            writer.write_all(quoted.as_bytes())?;
        }
        Value::Array(array) => {
            writer.write_all(b"[")?;
            for (idx, item) in array.iter().enumerate() {
                if idx > 0 {
                    writer.write_all(b",")?;
                }
                write_canonical_value(item, writer)?;
            }
            writer.write_all(b"]")?;
        }
        Value::Object(obj) => {
            writer.write_all(b"{")?;
            let mut first = true;
            let mut ordered: BTreeMap<&str, &Value> = BTreeMap::new();
            for (k, v) in obj.iter() {
                ordered.insert(k, v);
            }
            for (key, val) in ordered {
                if !first {
                    writer.write_all(b",")?;
                }
                first = false;
                let quoted = serde_json::to_string(key)?;
                writer.write_all(quoted.as_bytes())?;
                writer.write_all(b":")?;
                write_canonical_value(val, writer)?;
            }
            writer.write_all(b"}")?;
        }
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid record: {0}")]
    Invalid(String),
}
