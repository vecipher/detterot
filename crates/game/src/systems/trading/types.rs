use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use anyhow::Context;
use bevy::prelude::Resource;
use serde::Deserialize;

use crate::systems::economy::CommodityId;

static GLOBAL_CATALOG: OnceLock<Mutex<Arc<CommodityCatalog>>> = OnceLock::new();
static GLOBAL_TRADING_CONFIG: OnceLock<Mutex<Arc<TradingConfig>>> = OnceLock::new();

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommoditySpec {
    pub id: CommodityId,
    pub name: String,
    pub mass_kg: u16,
    pub volume_l: u16,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Commodities {
    pub list: Vec<CommoditySpec>,
}

#[derive(Debug, Clone, Resource)]
pub struct CommodityCatalog {
    list: Vec<CommoditySpec>,
    by_id: HashMap<CommodityId, CommoditySpec>,
}

impl From<Commodities> for CommodityCatalog {
    fn from(value: Commodities) -> Self {
        let mut by_id = HashMap::new();
        for spec in &value.list {
            by_id.insert(spec.id, spec.clone());
        }
        Self {
            list: value.list,
            by_id,
        }
    }
}

impl CommodityCatalog {
    pub fn load_from_path(path: &Path) -> anyhow::Result<Self> {
        let raw =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let parsed: Commodities =
            toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
        Ok(parsed.into())
    }

    pub fn list(&self) -> &[CommoditySpec] {
        &self.list
    }

    pub fn get(&self, id: CommodityId) -> Option<&CommoditySpec> {
        self.by_id.get(&id)
    }
}

impl CommodityCatalog {
    pub fn install_global(catalog: CommodityCatalog) {
        let lock = GLOBAL_CATALOG.get_or_init(|| Mutex::new(Arc::new(catalog.clone())));
        let mut guard = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        *guard = Arc::new(catalog);
    }

    pub fn global() -> Arc<CommodityCatalog> {
        GLOBAL_CATALOG
            .get()
            .expect("commodity catalog not installed before trade execution")
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

#[derive(Debug, Clone, Deserialize, Resource)]
#[serde(deny_unknown_fields)]
pub struct TradingConfig {
    pub fee_bp: i32,
}

impl TradingConfig {
    pub fn load_from_path(path: &Path) -> anyhow::Result<Self> {
        let raw =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
    }

    pub fn install_global(config: TradingConfig) {
        let lock = GLOBAL_TRADING_CONFIG.get_or_init(|| Mutex::new(Arc::new(config.clone())));
        let mut guard = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        *guard = Arc::new(config);
    }

    pub fn global() -> Arc<TradingConfig> {
        GLOBAL_TRADING_CONFIG
            .get()
            .expect("trading config not installed before trade execution")
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_fields_rejected() {
        let raw = r#"
            list = [
                { id = 1, name = "Test", mass_kg = 1, volume_l = 1, extra = 5 }
            ]
        "#;
        let parsed = toml::from_str::<Commodities>(raw);
        assert!(parsed.is_err());
    }
}
