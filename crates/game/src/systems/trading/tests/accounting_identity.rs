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
    econ.di_bp.insert(CommodityId(1), BasisBp(250));
    econ.basis_bp
        .insert((HubId(1), CommodityId(1)), BasisBp(150));
    econ
}

#[test]
fn wallet_delta_matches_identity() {
    install_globals();
    let rp = load_rulepack_fixture();
    let econ = setup_state();
    let mut cargo = Cargo {
        capacity_mass_kg: 1_000,
        capacity_volume_l: 1_000,
        items: Default::default(),
    };
    let mut wallet = MoneyCents(50_000);

    let buy = TradeTx {
        hub: HubId(1),
        com: CommodityId(1),
        units: 2,
        kind: TradeKind::Buy,
    };
    let buy_result = execute_trade(&buy, &econ, &mut cargo, &mut wallet, &rp).expect("buy");

    let sell = TradeTx {
        hub: HubId(1),
        com: CommodityId(1),
        units: 1,
        kind: TradeKind::Sell,
    };
    let sell_result = execute_trade(&sell, &econ, &mut cargo, &mut wallet, &rp).expect("sell");

    let wallet_delta = wallet.as_i64() - 50_000;
    let cost = buy_result.subtotal.as_i64();
    let proceeds = sell_result.subtotal.as_i64();
    let fees = buy_result.fee_cents.as_i64() + sell_result.fee_cents.as_i64();

    assert_eq!(wallet_delta, -cost + proceeds - fees);
    assert_eq!(cargo.units(CommodityId(1)), 1);
}
