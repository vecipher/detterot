use std::path::{Path, PathBuf};

use bevy::prelude::*;

use crate::systems::economy::{load_rulepack, EconState, Rulepack};

pub mod inventory;
pub mod pricing_vm;
pub mod types;

#[allow(unused_imports)]
pub use inventory::*;
#[allow(unused_imports)]
pub use pricing_vm::*;
#[allow(unused_imports)]
pub use types::*;

/// Bevy plugin that wires the trading subsystem into the core simulation.
///
/// The plugin ensures that read-only trading resources sourced from the
/// existing economy module are initialised exactly once. These resources are
/// required by forthcoming trading systems (pricing view models, inventory
/// management, etc.) and remain immutable so that no cyclic dependencies are
/// introduced between trading and economy code.
pub struct TradingPlugin;

impl Plugin for TradingPlugin {
    fn build(&self, app: &mut App) {
        initialise_resources(app);
    }
}

fn initialise_resources(app: &mut App) {
    let world = app.world_mut();

    if !world.contains_resource::<EconState>() {
        world.insert_resource(EconState::default());
    }

    if !world.contains_resource::<Rulepack>() {
        let path = default_rulepack_path();
        let path_str = path
            .to_str()
            .unwrap_or_else(|| panic!("rulepack path is not valid UTF-8: {}", path.display()));
        let rulepack = load_rulepack(path_str).unwrap_or_else(|err| {
            panic!(
                "failed to load trading rulepack from {}: {err}",
                path.display()
            )
        });
        world.insert_resource(rulepack);
    }

    if !world.contains_resource::<types::Commodities>() {
        let path = default_commodities_path();
        let path_str = path
            .to_str()
            .unwrap_or_else(|| panic!("commodities path is not valid UTF-8: {}", path.display()));
        let commodities = types::load_commodities(path_str).unwrap_or_else(|err| {
            panic!(
                "failed to load commodity specs from {}: {err}",
                path.display()
            )
        });
        world.insert_resource(commodities);
    }
}

fn default_rulepack_path() -> PathBuf {
    const DEFAULT: &str = "assets/rulepacks/day_001.toml";
    let candidate = Path::new(DEFAULT);
    if candidate.exists() {
        return candidate.to_path_buf();
    }

    Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../{DEFAULT}"))
}

fn default_commodities_path() -> PathBuf {
    const DEFAULT: &str = "assets/trading/commodities.toml";
    let candidate = Path::new(DEFAULT);
    if candidate.exists() {
        return candidate.to_path_buf();
    }

    Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../{DEFAULT}"))
}

#[cfg(test)]
mod tests;
