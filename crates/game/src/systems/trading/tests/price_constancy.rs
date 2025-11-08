use std::path::PathBuf;

use crate::systems::economy::rulepack::load_rulepack;
use crate::systems::economy::{BasisBp, CommodityId, EconState, HubId, MoneyCents};
use crate::systems::trading::engine::{execute_trade, TradeKind, TradeTx};
use crate::systems::trading::inventory::Cargo;
use crate::systems::trading::pricing_vm::price_view;
use crate::systems::trading::types::{CommodityCatalog, TradingConfig};

fn asset_path(relative: &str) -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest).join("..").join("..").join(relative)
}

fn load_catalog() -> CommodityCatalog {
    let path = asset_path("assets/trading/commodities.toml");
    CommodityCatalog::load_from_path(path.as_path()).expect("catalog")
}

fn install_globals() {
    let catalog = load_catalog();
    CommodityCatalog::install_global(catalog);
    TradingConfig::install_global(TradingConfig { fee_bp: 75 });
}

fn load_rulepack_fixture() -> crate::systems::economy::Rulepack {
    let path = asset_path("assets/rulepacks/day_001.toml");
    load_rulepack(path.to_str().expect("utf-8 path")).expect("rulepack")
}

fn setup_state() -> EconState {
    let mut econ = EconState::default();
    econ.di_bp.insert(CommodityId(1), BasisBp(500));
    econ.basis_bp
        .insert((HubId(1), CommodityId(1)), BasisBp(250));
    econ
}

#[test]
fn price_stable_within_day() {
    install_globals();
    let rp = load_rulepack_fixture();
    let mut econ = setup_state();
    let mut cargo = Cargo {
        capacity_mass_kg: 1_000,
        capacity_volume_l: 1_000,
        items: Default::default(),
    };
    let mut wallet = MoneyCents(100_000);

    let baseline = price_view(HubId(1), CommodityId(1), &econ, &rp).price_cents;

    let buy = TradeTx {
        hub: HubId(1),
        com: CommodityId(1),
        units: 1,
        kind: TradeKind::Buy,
    };
    execute_trade(&buy, &econ, &mut cargo, &mut wallet, &rp).expect("buy");

    let after_buy = price_view(HubId(1), CommodityId(1), &econ, &rp).price_cents;
    assert_eq!(after_buy, baseline);

    econ.basis_bp
        .insert((HubId(1), CommodityId(1)), BasisBp(350));
    let after_shift = price_view(HubId(1), CommodityId(1), &econ, &rp).price_cents;
    assert_ne!(after_shift, baseline);
}
