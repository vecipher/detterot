use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

use crate::systems::save::{SaveV1_1, SchemaVersion};

pub mod v1;

#[derive(Debug, Error)]
pub enum MigrateError {
    #[error("invalid save payload: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("unsupported save schema version {major}.{minor}")]
    UnsupportedVersion { major: u32, minor: u32 },
}

#[derive(Deserialize)]
struct SchemaProbe {
    version: SchemaVersion,
}

pub fn migrate_to_latest(value: Value) -> Result<SaveV1_1, MigrateError> {
    match value.get("schema").cloned() {
        Some(schema_value) => {
            let schema: SchemaProbe = serde_json::from_value(schema_value)?;
            match (schema.version.major, schema.version.minor) {
                (1, 1) => serde_json::from_value(value).map_err(MigrateError::from),
                (1, 0) => {
                    let legacy = v1::from_value(value)?;
                    Ok(SaveV1_1::upgrade_from_v1(legacy))
                }
                (major, minor) => Err(MigrateError::UnsupportedVersion { major, minor }),
            }
        }
        None => {
            let legacy = v1::from_value(value)?;
            Ok(SaveV1_1::upgrade_from_v1(legacy))
        }
    }
}
