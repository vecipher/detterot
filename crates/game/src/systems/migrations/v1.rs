#![allow(dead_code)]

use serde_json::Value;

use crate::systems::save::SaveV1;

use super::MigrateError;

pub fn from_value(value: Value) -> Result<SaveV1, MigrateError> {
    serde_json::from_value(value).map_err(MigrateError::from)
}
