pub mod engine;
pub mod inventory;
pub mod pricing_vm;
pub mod types;

#[cfg(test)]
#[path = "tests/accounting_identity.rs"]
mod accounting_identity;
#[cfg(test)]
#[path = "tests/capacity_enforcement.rs"]
mod capacity_enforcement;
#[cfg(test)]
#[path = "tests/price_constancy.rs"]
mod price_constancy;
#[cfg(test)]
#[path = "tests/pricing_vm_rounding.rs"]
mod pricing_vm_rounding;

use anyhow::anyhow;
use bevy::prelude::*;

use self::types::{CommodityCatalog, TradingConfig};

pub struct TradingPlugin;

impl Plugin for TradingPlugin {
    fn build(&self, app: &mut App) {
        let commodities = load_default_commodities().expect("failed to load default commodities");
        CommodityCatalog::install_global(commodities.clone());
        app.insert_resource(commodities);

        let config = load_default_trading_config().expect("failed to load trading config");
        TradingConfig::install_global(config.clone());
        app.insert_resource(config);
    }
}

fn load_default_commodities() -> anyhow::Result<CommodityCatalog> {
    let workspace_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets/trading/commodities.toml");
    let search_paths = [
        std::path::Path::new("assets/trading/commodities.toml"),
        workspace_path.as_path(),
    ];
    for path in search_paths {
        if path.exists() {
            return CommodityCatalog::load_from_path(path);
        }
    }
    let last = workspace_path.display();
    Err(anyhow!("missing commodities asset at {last}"))
}

fn load_default_trading_config() -> anyhow::Result<TradingConfig> {
    let workspace_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets/trading/config.toml");
    let search_paths = [
        std::path::Path::new("assets/trading/config.toml"),
        workspace_path.as_path(),
    ];
    for path in search_paths {
        if path.exists() {
            return TradingConfig::load_from_path(path);
        }
    }
    let last = workspace_path.display();
    Err(anyhow!("missing trading config asset at {last}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_catalog_loads() {
        let catalog = load_default_commodities().expect("catalog");
        assert!(!catalog.list().is_empty());
    }

    #[test]
    fn default_config_loads() {
        let config = load_default_trading_config().expect("config");
        assert!(config.fee_bp >= 0);
    }
}
