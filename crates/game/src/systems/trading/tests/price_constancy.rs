use crate::systems::economy::{BasisBp, CommodityId, EconState, HubId, MoneyCents, Rulepack};
use crate::systems::trading::types::CommoditySpec;
use crate::systems::trading::{
    execute_trade, inventory::Cargo, pricing_vm::price_view, types, TradeKind, TradeTx,
};

use super::load_fixture_rulepack;

fn prepare_state(commodity: CommodityId, hub: HubId) -> (EconState, Rulepack) {
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
fn quoted_prices_remain_constant_within_a_day() {
    let _guard = types::global_commodities_guard();
    let commodity = CommodityId(2);
    let hub = HubId(6);
    register_metadata(commodity, MoneyCents(250), 2, 3);
    let (state, rulepack) = prepare_state(commodity, hub);
    let quoted = price_view(hub, commodity, &state, &rulepack)
        .expect("price view")
        .price_cents();

    let mut cargo = Cargo::default();
    cargo.capacity_total = 40;
    cargo.mass_capacity_total = 40;
    let mut wallet = MoneyCents(1_000);

    let tx = TradeTx {
        kind: TradeKind::Buy,
        hub,
        commodity,
        units: 1,
    };

    let first =
        execute_trade(&tx, &state, &mut cargo, &mut wallet, &rulepack).expect("first trade");
    let second =
        execute_trade(&tx, &state, &mut cargo, &mut wallet, &rulepack).expect("second trade");

    assert_eq!(first.unit_price, quoted);
    assert_eq!(second.unit_price, quoted);
    assert_eq!(first.unit_price, second.unit_price);
}
