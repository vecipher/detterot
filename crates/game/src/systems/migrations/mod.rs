#![allow(dead_code)]

use serde_json::Value;
use thiserror::Error;

use crate::systems::save::SaveV1;

pub mod v1;

#[derive(Debug, Error)]
pub enum MigrateError {
    #[error("invalid save payload: {0}")]
    Serde(#[from] serde_json::Error),
}

pub fn migrate_to_latest(value: Value) -> Result<SaveV1, MigrateError> {
    v1::from_value(value)
}
