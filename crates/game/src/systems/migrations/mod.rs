#![allow(dead_code)]

use serde_json::Value;
use thiserror::Error;

use crate::systems::save::{
    v1_1::migrate_v1_to_v11,
    v1_2::{migrate_v11_to_v12, SaveV12},
    SaveV11,
};

pub mod v1;

#[derive(Debug, Error)]
pub enum MigrateError {
    #[error("invalid save payload: {0}")]
    Serde(#[from] serde_json::Error),
}

pub fn migrate_to_latest(value: Value) -> Result<SaveV12, MigrateError> {
    if value.get("last_board_hash").is_some() && value.get("visited_links").is_some() {
        // This is already v1.2
        return serde_json::from_value(value).map_err(MigrateError::from);
    } else if value.get("cargo").is_some() || value.get("last_hub").is_some() {
        // This is v1.1, migrate to v1.2
        let v11: SaveV11 = serde_json::from_value(value)?;
        return Ok(migrate_v11_to_v12(v11));
    }

    // This is v1.0, migrate to v1.1 then to v1.2
    let v1 = v1::from_value(value)?;
    let v11 = migrate_v1_to_v11(v1);
    Ok(migrate_v11_to_v12(v11))
}
