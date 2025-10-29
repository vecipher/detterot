use serde_json::Value;

use crate::systems::save::legacy;

use super::MigrateError;

pub fn from_value(value: Value) -> Result<legacy::SaveV1, MigrateError> {
    serde_json::from_value(value).map_err(MigrateError::from)
}
