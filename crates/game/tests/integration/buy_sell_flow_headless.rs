use std::path::PathBuf;

use game::systems::command_queue::CommandQueue;
use game::systems::economy::rulepack::load_rulepack;
use game::systems::economy::{BasisBp, CommodityId, EconState, HubId, MoneyCents};
use game::systems::trading::engine::{TradeKind, TradeTx};
use game::systems::trading::inventory::Cargo;
use game::systems::trading::types::{CommodityCatalog, TradingConfig};
use game::ui::hub_trade::HubTradeActions;
use repro::CommandKind;

fn asset_path(relative: &str) -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest).join("..").join("..").join(relative)
}

fn install_globals() {
    let catalog_path = asset_path("assets/trading/commodities.toml");
    let catalog = CommodityCatalog::load_from_path(catalog_path.as_path()).expect("catalog");
    CommodityCatalog::install_global(catalog);
    TradingConfig::install_global(TradingConfig { fee_bp: 75 });
}

fn load_rulepack_fixture() -> game::systems::economy::Rulepack {
    let path = asset_path("assets/rulepacks/day_001.toml");
    load_rulepack(path.to_str().expect("utf-8 path")).expect("rulepack")
}

#[test]
fn buy_then_sell_updates_state() {
    install_globals();
    let rp = load_rulepack_fixture();
    let mut econ = EconState::default();
    econ.di_bp.insert(CommodityId(1), BasisBp(200));
    econ.basis_bp
        .insert((HubId(1), CommodityId(1)), BasisBp(80));

    let mut cargo = Cargo {
        capacity_mass_kg: 500,
        capacity_volume_l: 500,
        items: Default::default(),
    };
    let mut wallet = MoneyCents(100_000);
    let mut queue = CommandQueue::default();
    queue.begin_tick(0);

    let buy = TradeTx {
        hub: HubId(1),
        com: CommodityId(1),
        units: 1,
        kind: TradeKind::Buy,
    };
    let buy_result = HubTradeActions::buy(&mut queue, buy, &econ, &mut cargo, &mut wallet, &rp)
        .expect("buy result");
    assert!(buy_result.total_cents.as_i64() > 0);
    assert_eq!(cargo.units(CommodityId(1)), 1);

    let sell = TradeTx {
        hub: HubId(1),
        com: CommodityId(1),
        units: 1,
        kind: TradeKind::Sell,
    };
    let sell_result = HubTradeActions::sell(&mut queue, sell, &econ, &mut cargo, &mut wallet, &rp)
        .expect("sell result");
    assert!(sell_result.total_cents.as_i64() < 0);
    assert_eq!(cargo.units(CommodityId(1)), 0);

    let meters: Vec<_> = queue
        .drain()
        .into_iter()
        .filter_map(|cmd| match cmd.kind {
            CommandKind::Meter(m) => Some((m.key, m.value)),
            _ => None,
        })
        .collect();
    assert!(meters.iter().any(|(key, _)| key == "ui_click_buy"));
    assert!(meters.iter().any(|(key, _)| key == "ui_click_sell"));
}
