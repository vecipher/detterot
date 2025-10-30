use crate::systems::economy::rulepack::load_rulepack;
use crate::systems::economy::{BasisBp, CommodityId, EconState, HubId, MoneyCents};
use crate::systems::trading::engine::{execute_trade, TradeKind, TradeTx};
use crate::systems::trading::inventory::Cargo;
use crate::systems::trading::types::{CommodityCatalog, TradingConfig};
use std::path::PathBuf;

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
    econ.di_bp.insert(CommodityId(1), BasisBp(100));
    econ.basis_bp
        .insert((HubId(1), CommodityId(1)), BasisBp(50));
    econ
}

#[test]
fn buy_respects_mass_and_volume_caps() {
    install_globals();
    let rp = load_rulepack_fixture();
    let econ = setup_state();
    let mut cargo = Cargo {
        capacity_mass_kg: 15,
        capacity_volume_l: 15,
        items: Default::default(),
    };
    let mut wallet = MoneyCents(100_000);

    let buy = TradeTx {
        hub: HubId(1),
        com: CommodityId(1),
        units: 1,
        kind: TradeKind::Buy,
    };
    execute_trade(&buy, &econ, &mut cargo, &mut wallet, &rp).expect("initial buy");

    let err = execute_trade(&buy, &econ, &mut cargo, &mut wallet, &rp).expect_err("capacity");
    assert!(format!("{err}").contains("capacity"));

    let sell = TradeTx {
        kind: TradeKind::Sell,
        ..buy
    };
    execute_trade(&sell, &econ, &mut cargo, &mut wallet, &rp).expect("sell");
    assert_eq!(cargo.units(CommodityId(1)), 0);
}
