use crate::systems::command_queue::CommandQueue;
use crate::systems::economy::{BasisBp, CommodityId, EconState, HubId, MoneyCents};
use crate::systems::trading::{
    execute_trade, inventory::Cargo, meters, pricing_vm::price_view, TradeKind, TradeTx,
};

use repro::Command;

use super::load_fixture_rulepack;

fn setup_view(
    commodity: CommodityId,
    hub: HubId,
) -> (EconState, crate::systems::economy::PricingCfg) {
    let mut state = EconState::default();
    state.di_bp.insert(commodity, BasisBp(0));
    state.basis_bp.insert((hub, commodity), BasisBp(0));
    let pack = load_fixture_rulepack();
    (state, pack.pricing)
}

#[test]
fn buy_trade_preserves_accounting_identity() {
    let commodity = CommodityId(3);
    let hub = HubId(2);
    let (state, pricing) = setup_view(commodity, hub);
    let rulepack = load_fixture_rulepack();
    let view = price_view(&state, &pricing);

    let mut cargo = Cargo::default();
    cargo.capacity_total = 20;
    cargo.mass_capacity_total = 20;
    let mut wallet = MoneyCents(2_000);

    let tx = TradeTx {
        kind: TradeKind::Buy,
        hub,
        commodity,
        units: 4,
        base_price: MoneyCents(250),
        volume_per_unit: 2,
        mass_per_unit: 3,
    };

    let wallet_before = wallet;
    let mut queue = CommandQueue::default();
    queue.begin_tick(0);
    let result = execute_trade(&tx, &view, &rulepack, &mut cargo, &mut wallet).expect("trade");

    assert_eq!(result.units_executed, 4);
    let expected_fee = MoneyCents::from_i128_clamped(
        i128::from(result.gross.as_i64()) * i128::from(rulepack.trading.transaction_fee_bp)
            / 10_000,
    );
    assert_eq!(result.fee, expected_fee);
    let total_cost = result.gross.saturating_add(result.fee);
    let expected_delta = MoneyCents::from_i128_clamped(-i128::from(total_cost.as_i64()));
    assert_eq!(result.wallet_delta, expected_delta);
    assert_eq!(wallet_before.saturating_add(result.wallet_delta), wallet);

    assert_eq!(cargo.units(commodity), 4);
    assert_eq!(cargo.capacity_used, 8);
    assert_eq!(cargo.mass_capacity_used, 12);

    meters::record_trade(&mut queue, TradeKind::Buy, &result);
    let expected_wallet_value = result.wallet_delta.as_i64();
    assert!(expected_wallet_value >= i64::from(i32::MIN));
    assert!(expected_wallet_value <= i64::from(i32::MAX));
    let expected_wallet_value = expected_wallet_value as i32;
    let commands = queue.drain();
    assert_eq!(
        commands,
        vec![
            Command::meter_at(0, meters::UI_CLICK_BUY, 1),
            Command::meter_at(0, meters::WALLET_DELTA_BUY, expected_wallet_value),
        ]
    );
}

#[test]
fn sell_trade_preserves_accounting_identity() {
    let commodity = CommodityId(4);
    let hub = HubId(1);
    let (state, pricing) = setup_view(commodity, hub);
    let rulepack = load_fixture_rulepack();
    let view = price_view(&state, &pricing);

    let mut cargo = Cargo::default();
    cargo.capacity_total = 30;
    cargo.capacity_used = 12;
    cargo.mass_capacity_total = 40;
    cargo.mass_capacity_used = 18;
    cargo.set_units(commodity, 6);
    let mut wallet = MoneyCents(1_000);

    let tx = TradeTx {
        kind: TradeKind::Sell,
        hub,
        commodity,
        units: 5,
        base_price: MoneyCents(250),
        volume_per_unit: 2,
        mass_per_unit: 3,
    };

    let wallet_before = wallet;
    let mut queue = CommandQueue::default();
    queue.begin_tick(0);
    let result = execute_trade(&tx, &view, &rulepack, &mut cargo, &mut wallet).expect("trade");

    assert_eq!(result.units_executed, 5);
    let expected_fee = MoneyCents::from_i128_clamped(
        i128::from(result.gross.as_i64()) * i128::from(rulepack.trading.transaction_fee_bp)
            / 10_000,
    );
    assert_eq!(result.fee, expected_fee);
    let expected_delta = result.gross.saturating_sub(result.fee);
    assert_eq!(result.wallet_delta, expected_delta);
    assert_eq!(wallet_before.saturating_add(result.wallet_delta), wallet);

    assert_eq!(cargo.units(commodity), 1);
    assert_eq!(cargo.capacity_used, 2);
    assert_eq!(cargo.mass_capacity_used, 3);

    meters::record_trade(&mut queue, TradeKind::Sell, &result);
    let expected_wallet_value = result.wallet_delta.as_i64();
    assert!(expected_wallet_value >= i64::from(i32::MIN));
    assert!(expected_wallet_value <= i64::from(i32::MAX));
    let expected_wallet_value = expected_wallet_value as i32;
    let commands = queue.drain();
    assert_eq!(
        commands,
        vec![
            Command::meter_at(0, meters::UI_CLICK_SELL, 1),
            Command::meter_at(0, meters::WALLET_DELTA_SELL, expected_wallet_value),
        ]
    );
}
