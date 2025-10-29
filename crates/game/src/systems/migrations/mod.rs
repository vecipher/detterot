#![allow(dead_code)]

use serde_json::Value;
use thiserror::Error;

use crate::systems::save::{v1_1::migrate_v1_to_v11, SaveV11};

pub mod v1;

#[derive(Debug, Error)]
pub enum MigrateError {
    #[error("invalid save payload: {0}")]
    Serde(#[from] serde_json::Error),
}

pub fn migrate_to_latest(value: Value) -> Result<SaveV11, MigrateError> {
    if value.get("cargo").is_some() || value.get("last_hub").is_some() {
        return serde_json::from_value(value).map_err(MigrateError::from);
    }

    let v1 = v1::from_value(value)?;
    Ok(migrate_v1_to_v11(v1))
}
