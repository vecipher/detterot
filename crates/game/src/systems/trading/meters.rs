use crate::systems::command_queue::CommandQueue;
use crate::systems::economy::MoneyCents;

use super::engine::{TradeKind, TradeResult};

pub const UI_CLICK_BUY: &str = "ui_click_buy";
pub const UI_CLICK_SELL: &str = "ui_click_sell";
pub const WALLET_DELTA_BUY: &str = "wallet_delta_buy";
pub const WALLET_DELTA_SELL: &str = "wallet_delta_sell";

/// Records both click and wallet delta meters for a completed trade.
pub fn record_trade(queue: &mut CommandQueue, kind: TradeKind, result: &TradeResult) {
    record_ui_click(queue, kind);
    record_wallet_delta(queue, kind, result.wallet_delta);
}

/// Emits a UI click meter for the provided trade kind.
pub fn record_ui_click(queue: &mut CommandQueue, kind: TradeKind) {
    queue.meter(ui_click_key(kind), 1);
}

/// Emits a wallet delta meter, clamping the value to the i32 range accepted by repro commands.
pub fn record_wallet_delta(queue: &mut CommandQueue, kind: TradeKind, delta: MoneyCents) {
    queue.meter(wallet_delta_key(kind), wallet_delta_value(delta));
}

/// Meter key associated with the UI click for a trade kind.
pub fn ui_click_key(kind: TradeKind) -> &'static str {
    match kind {
        TradeKind::Buy => UI_CLICK_BUY,
        TradeKind::Sell => UI_CLICK_SELL,
    }
}

/// Meter key used to record the wallet delta for a trade kind.
pub fn wallet_delta_key(kind: TradeKind) -> &'static str {
    match kind {
        TradeKind::Buy => WALLET_DELTA_BUY,
        TradeKind::Sell => WALLET_DELTA_SELL,
    }
}

/// Converts a wallet delta (stored in cents) to the i32 range used by repro meter commands.
pub fn wallet_delta_value(delta: MoneyCents) -> i32 {
    let cents = delta.as_i64();
    if cents > i32::MAX as i64 {
        i32::MAX
    } else if cents < i32::MIN as i64 {
        i32::MIN
    } else {
        cents as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wallet_delta_value_clamps_to_i32_range() {
        assert_eq!(
            wallet_delta_value(MoneyCents(i64::from(i32::MAX))),
            i32::MAX
        );
        assert_eq!(
            wallet_delta_value(MoneyCents(i64::from(i32::MIN))),
            i32::MIN
        );
        assert_eq!(wallet_delta_value(MoneyCents(42)), 42);
        assert_eq!(wallet_delta_value(MoneyCents(-99)), -99);
        assert_eq!(wallet_delta_value(MoneyCents(i64::MAX)), i32::MAX);
        assert_eq!(wallet_delta_value(MoneyCents(i64::MIN)), i32::MIN);
    }
}
