use anyhow::Result;

use crate::systems::economy::{CommodityId, HubId, MoneyCents, Rulepack};

use super::{inventory::Cargo, pricing_vm::PriceView};

/// Direction of a requested trade transaction.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TradeKind {
    Buy,
    Sell,
}

/// High level trade order that can be executed by the engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeTx {
    pub kind: TradeKind,
    pub hub: HubId,
    pub commodity: CommodityId,
    pub units: u32,
    pub base_price: MoneyCents,
    pub volume_per_unit: u32,
    pub mass_per_unit: u32,
}

/// Outcome of executing a trade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeResult {
    pub units_executed: u32,
    pub unit_price: MoneyCents,
    pub gross: MoneyCents,
    pub fee: MoneyCents,
    pub wallet_delta: MoneyCents,
}

impl TradeResult {
    fn empty(unit_price: MoneyCents) -> Self {
        Self {
            units_executed: 0,
            unit_price,
            gross: MoneyCents::ZERO,
            fee: MoneyCents::ZERO,
            wallet_delta: MoneyCents::ZERO,
        }
    }
}

/// Executes a trade, mutating cargo and wallet balances while observing mass,
/// volume, and wallet constraints.
pub fn execute_trade(
    tx: &TradeTx,
    view: &PriceView<'_>,
    rulepack: &Rulepack,
    cargo: &mut Cargo,
    wallet: &mut MoneyCents,
) -> Result<TradeResult> {
    let unit_price = view.quote(tx.hub, tx.commodity, tx.base_price);
    if tx.units == 0 {
        return Ok(TradeResult::empty(unit_price));
    }

    let fee_bp = rulepack.trading.transaction_fee_bp;
    let requested_units = tx.units;
    let units_executed = match tx.kind {
        TradeKind::Buy => {
            let available_volume = cargo.capacity_total.saturating_sub(cargo.capacity_used);
            let available_mass = cargo
                .mass_capacity_total
                .saturating_sub(cargo.mass_capacity_used);
            let volume_cap = if tx.volume_per_unit == 0 {
                requested_units
            } else {
                available_volume / tx.volume_per_unit
            };
            let mass_cap = if tx.mass_per_unit == 0 {
                requested_units
            } else {
                available_mass / tx.mass_per_unit
            };
            let wallet_cap = max_units_affordable(*wallet, unit_price, fee_bp, requested_units);
            requested_units
                .min(volume_cap)
                .min(mass_cap)
                .min(wallet_cap)
        }
        TradeKind::Sell => {
            let inventory = cargo.units(tx.commodity);
            requested_units.min(inventory)
        }
    };

    if units_executed == 0 {
        return Ok(TradeResult::empty(unit_price));
    }

    let gross = multiply(unit_price, units_executed);
    let fee = apply_basis_points(gross, fee_bp);
    let (volume_delta, mass_delta) = capacity_deltas(tx, units_executed);
    let wallet_delta = match tx.kind {
        TradeKind::Buy => {
            let total_cost = gross.saturating_add(fee);
            negate(total_cost)
        }
        TradeKind::Sell => gross.saturating_sub(fee),
    };

    match tx.kind {
        TradeKind::Buy => {
            let prior_units = cargo.units(tx.commodity);
            cargo.set_units(tx.commodity, prior_units.saturating_add(units_executed));
            cargo.capacity_used =
                (cargo.capacity_used.saturating_add(volume_delta)).min(cargo.capacity_total);
            cargo.mass_capacity_used = (cargo.mass_capacity_used.saturating_add(mass_delta))
                .min(cargo.mass_capacity_total);
        }
        TradeKind::Sell => {
            let prior_units = cargo.units(tx.commodity);
            cargo.set_units(tx.commodity, prior_units.saturating_sub(units_executed));
            cargo.capacity_used = cargo.capacity_used.saturating_sub(volume_delta);
            cargo.mass_capacity_used = cargo.mass_capacity_used.saturating_sub(mass_delta);
        }
    }

    let new_wallet = (*wallet).saturating_add(wallet_delta);
    *wallet = new_wallet;

    let result = TradeResult {
        units_executed,
        unit_price,
        gross,
        fee,
        wallet_delta,
    };

    #[cfg(feature = "econ_logs")]
    crate::logs::trading::log_trade(tx, &result, new_wallet);

    Ok(result)
}

fn capacity_deltas(tx: &TradeTx, units: u32) -> (u32, u32) {
    let volume = units.saturating_mul(tx.volume_per_unit);
    let mass = units.saturating_mul(tx.mass_per_unit);
    (volume, mass)
}

fn multiply(amount: MoneyCents, units: u32) -> MoneyCents {
    let value = i128::from(amount.as_i64()).saturating_mul(i128::from(units));
    MoneyCents::from_i128_clamped(value)
}

fn apply_basis_points(amount: MoneyCents, bp: i32) -> MoneyCents {
    if bp == 0 {
        return MoneyCents::ZERO;
    }
    let scaled = i128::from(amount.as_i64()).saturating_mul(i128::from(bp));
    let divisor = 10_000i128;
    let result = scaled / divisor;
    MoneyCents::from_i128_clamped(result)
}

fn negate(amount: MoneyCents) -> MoneyCents {
    MoneyCents::from_i128_clamped(-i128::from(amount.as_i64()))
}

fn max_units_affordable(
    wallet: MoneyCents,
    unit_price: MoneyCents,
    fee_bp: i32,
    requested: u32,
) -> u32 {
    if requested == 0 {
        return 0;
    }
    let balance = wallet.as_i64();
    if balance <= 0 {
        return 0;
    }

    let mut lo = 0u32;
    let mut hi = requested;
    while lo < hi {
        let mid = lo + (hi - lo).div_ceil(2);
        let cost = trade_cost(unit_price, mid, fee_bp).as_i64();
        if cost <= balance {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }

    lo
}

fn trade_cost(unit_price: MoneyCents, units: u32, fee_bp: i32) -> MoneyCents {
    if units == 0 {
        return MoneyCents::ZERO;
    }
    let gross = multiply(unit_price, units);
    let fee = apply_basis_points(gross, fee_bp);
    gross.saturating_add(fee)
}
