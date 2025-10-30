use crate::systems::economy::{BasisBp, CommodityId, EconState, HubId, MoneyCents, Rulepack};
use crate::systems::trading::{
    execute_trade, inventory::Cargo, pricing_vm::price_view, TradeKind, TradeTx,
};

use super::load_fixture_rulepack;

fn build_view(commodity: CommodityId, hub: HubId) -> (EconState, Rulepack) {
    let mut state = EconState::default();
    state.di_bp.insert(commodity, BasisBp(0));
    state.basis_bp.insert((hub, commodity), BasisBp(0));
    let pack = load_fixture_rulepack();
    (state, pack)
}

#[test]
fn buy_trade_clamps_to_mass_and_volume() {
    let commodity = CommodityId(5);
    let hub = HubId(2);
    let (state, rulepack) = build_view(commodity, hub);
    let view = price_view(hub, commodity, &state, &rulepack)
        .with_price(MoneyCents(120), &rulepack.pricing);

    let mut cargo = Cargo::default();
    cargo.capacity_total = 9;
    cargo.mass_capacity_total = 11;
    let mut wallet = MoneyCents(10_000);

    let tx = TradeTx {
        kind: TradeKind::Buy,
        hub,
        commodity,
        units: 10,
        base_price: MoneyCents(120),
        volume_per_unit: 4,
        mass_per_unit: 6,
    };

    let result = execute_trade(&tx, &view, &rulepack, &mut cargo, &mut wallet).expect("trade");
    assert_eq!(result.units_executed, 1);
    assert_eq!(cargo.capacity_used, 4);
    assert_eq!(cargo.mass_capacity_used, 6);
}

#[test]
fn buy_trade_clamps_to_wallet_balance() {
    let commodity = CommodityId(6);
    let hub = HubId(4);
    let (state, rulepack) = build_view(commodity, hub);
    let view = price_view(hub, commodity, &state, &rulepack)
        .with_price(MoneyCents(250), &rulepack.pricing);

    let mut cargo = Cargo::default();
    cargo.capacity_total = 50;
    cargo.mass_capacity_total = 50;
    let mut wallet = MoneyCents(600);

    let tx = TradeTx {
        kind: TradeKind::Buy,
        hub,
        commodity,
        units: 10,
        base_price: MoneyCents(250),
        volume_per_unit: 1,
        mass_per_unit: 1,
    };

    let result = execute_trade(&tx, &view, &rulepack, &mut cargo, &mut wallet).expect("trade");
    assert_eq!(result.units_executed, 2);
    assert_eq!(cargo.capacity_used, 2);
    assert_eq!(cargo.mass_capacity_used, 2);
    let expected_wallet = MoneyCents(600).saturating_add(result.wallet_delta);
    assert_eq!(wallet, expected_wallet);
}

#[test]
fn sell_trade_cannot_exceed_inventory() {
    let commodity = CommodityId(7);
    let hub = HubId(5);
    let (state, rulepack) = build_view(commodity, hub);
    let view = price_view(hub, commodity, &state, &rulepack)
        .with_price(MoneyCents(200), &rulepack.pricing);

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
        base_price: MoneyCents(200),
        volume_per_unit: 3,
        mass_per_unit: 3,
    };

    let result = execute_trade(&tx, &view, &rulepack, &mut cargo, &mut wallet).expect("trade");
    assert_eq!(result.units_executed, 3);
    assert_eq!(cargo.units(commodity), 0);
    assert_eq!(cargo.capacity_used, 0);
    assert_eq!(cargo.mass_capacity_used, 0);
}
