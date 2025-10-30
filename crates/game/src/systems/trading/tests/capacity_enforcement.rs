use crate::systems::economy::{BasisBp, CommodityId, EconState, HubId, MoneyCents, Rulepack};
use crate::systems::trading::types::CommoditySpec;
use crate::systems::trading::{execute_trade, inventory::Cargo, types, TradeKind, TradeTx};

use super::load_fixture_rulepack;

fn build_view(commodity: CommodityId, hub: HubId) -> (EconState, Rulepack) {
    let mut state = EconState::default();
    state.di_bp.insert(commodity, BasisBp(0));
    state.basis_bp.insert((hub, commodity), BasisBp(0));
    let pack = load_fixture_rulepack();
    (state, pack)
}

fn register_metadata(
    commodity: CommodityId,
    base_price: MoneyCents,
    volume_per_unit: u32,
    mass_per_unit: u32,
) {
    types::clear_global_commodities();
    let spec = CommoditySpec {
        id: commodity,
        slug: format!("test-{0}", commodity.0),
        display_name: "Test".to_string(),
        base_price_cents: base_price.as_i64(),
        mass_per_unit_kg: mass_per_unit,
        volume_per_unit_l: volume_per_unit,
    };
    types::set_global_commodities(types::commodities_from_specs(vec![spec]));
}

#[test]
fn buy_trade_clamps_to_mass_and_volume() {
    let _guard = types::global_commodities_guard();
    let commodity = CommodityId(5);
    let hub = HubId(2);
    register_metadata(commodity, MoneyCents(120), 4, 6);
    let (state, rulepack) = build_view(commodity, hub);

    let mut cargo = Cargo::default();
    cargo.capacity_total = 9;
    cargo.mass_capacity_total = 11;
    let mut wallet = MoneyCents(10_000);

    let tx = TradeTx {
        kind: TradeKind::Buy,
        hub,
        commodity,
        units: 10,
    };

    let result = execute_trade(&tx, &state, &mut cargo, &mut wallet, &rulepack).expect("trade");
    assert_eq!(result.units_executed, 1);
    assert_eq!(cargo.capacity_used, 4);
    assert_eq!(cargo.mass_capacity_used, 6);
}

#[test]
fn buy_trade_clamps_to_wallet_balance() {
    let _guard = types::global_commodities_guard();
    let commodity = CommodityId(6);
    let hub = HubId(4);
    register_metadata(commodity, MoneyCents(250), 1, 1);
    let (state, rulepack) = build_view(commodity, hub);

    let mut cargo = Cargo::default();
    cargo.capacity_total = 50;
    cargo.mass_capacity_total = 50;
    let mut wallet = MoneyCents(600);

    let tx = TradeTx {
        kind: TradeKind::Buy,
        hub,
        commodity,
        units: 10,
    };

    let result = execute_trade(&tx, &state, &mut cargo, &mut wallet, &rulepack).expect("trade");
    assert_eq!(result.units_executed, 2);
    assert_eq!(cargo.capacity_used, 2);
    assert_eq!(cargo.mass_capacity_used, 2);
    let expected_wallet = MoneyCents(600).saturating_add(result.wallet_delta);
    assert_eq!(wallet, expected_wallet);
}

#[test]
fn sell_trade_cannot_exceed_inventory() {
    let _guard = types::global_commodities_guard();
    let commodity = CommodityId(7);
    let hub = HubId(5);
    register_metadata(commodity, MoneyCents(200), 3, 3);
    let (state, rulepack) = build_view(commodity, hub);

    let mut cargo = Cargo::default();
    cargo.capacity_total = 30;
    cargo.capacity_used = 9;
    cargo.mass_capacity_total = 30;
    cargo.mass_capacity_used = 9;
    cargo.set_units(commodity, 3);
    let mut wallet = MoneyCents(0);

    let tx = TradeTx {
        kind: TradeKind::Sell,
        hub,
        commodity,
        units: 7,
    };

    let result = execute_trade(&tx, &state, &mut cargo, &mut wallet, &rulepack).expect("trade");
    assert_eq!(result.units_executed, 3);
    assert_eq!(cargo.units(commodity), 0);
    assert_eq!(cargo.capacity_used, 0);
    assert_eq!(cargo.mass_capacity_used, 0);
}
