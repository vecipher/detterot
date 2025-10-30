use crate::systems::economy::{BasisBp, CommodityId, EconState, HubId, MoneyCents, Rulepack};
use crate::systems::trading::{
    execute_trade, inventory::Cargo, pricing_vm::price_view, TradeKind, TradeTx,
};

use super::load_fixture_rulepack;

fn prepare_state(commodity: CommodityId, hub: HubId) -> (EconState, Rulepack) {
    let mut state = EconState::default();
    state.di_bp.insert(commodity, BasisBp(0));
    state.basis_bp.insert((hub, commodity), BasisBp(0));
    let pack = load_fixture_rulepack();
    (state, pack)
}

#[test]
fn quoted_prices_remain_constant_within_a_day() {
    let commodity = CommodityId(2);
    let hub = HubId(6);
    let (state, rulepack) = prepare_state(commodity, hub);
    let view = price_view(hub, commodity, &state, &rulepack)
        .with_price(MoneyCents(250), &rulepack.pricing);
    let quoted = view.price_cents();

    let mut cargo = Cargo::default();
    cargo.capacity_total = 40;
    cargo.mass_capacity_total = 40;
    let mut wallet = MoneyCents(1_000);

    let tx = TradeTx {
        kind: TradeKind::Buy,
        hub,
        commodity,
        units: 1,
        base_price: MoneyCents(250),
        volume_per_unit: 2,
        mass_per_unit: 3,
    };

    let first = execute_trade(&tx, &view, &rulepack, &mut cargo, &mut wallet).expect("first trade");
    let second =
        execute_trade(&tx, &view, &rulepack, &mut cargo, &mut wallet).expect("second trade");

    assert_eq!(first.unit_price, quoted);
    assert_eq!(second.unit_price, quoted);
    assert_eq!(first.unit_price, second.unit_price);
}
