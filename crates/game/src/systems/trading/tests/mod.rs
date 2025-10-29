use std::path::{Path, PathBuf};

use crate::systems::economy::{load_rulepack, Rulepack};

mod accounting_identity;
mod capacity_enforcement;
mod commodities_loader;
mod price_constancy;
mod pricing_vm_rounding;

pub(super) fn load_fixture_rulepack() -> Rulepack {
    let path = workspace_path("assets/rulepacks/day_001.toml");
    load_rulepack(path.to_str().expect("utf-8 path")).expect("load rulepack")
}

fn workspace_path(relative: &str) -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root");
    root.join(relative)
}
