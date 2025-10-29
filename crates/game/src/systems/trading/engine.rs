use anyhow::{anyhow, ensure};

use crate::systems::economy::{CommodityId, EconState, HubId, MoneyCents, Rulepack};
use crate::systems::trading::inventory::Cargo;
use crate::systems::trading::pricing_vm::price_view;
use crate::systems::trading::types::{CommodityCatalog, CommoditySpec, TradingConfig};

#[derive(Debug, Clone, Copy)]
pub struct TradeTx {
    pub hub: HubId,
    pub com: CommodityId,
    pub units: u32,
    pub kind: TradeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeKind {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeResult {
    pub unit_price: MoneyCents,
    pub subtotal: MoneyCents,
    pub fee_cents: MoneyCents,
    pub total_cents: MoneyCents,
}

pub fn execute_trade(
    tx: &TradeTx,
    econ: &EconState,
    cargo: &mut Cargo,
    wallet: &mut MoneyCents,
    rp: &Rulepack,
) -> anyhow::Result<TradeResult> {
    ensure!(tx.units > 0, "trade requires at least one unit");

    let catalog = CommodityCatalog::global();
    let spec = catalog
        .get(tx.com)
        .ok_or_else(|| anyhow!("unknown commodity {:?}", tx.com))?;
    let config = TradingConfig::global();
    ensure!(config.fee_bp >= 0, "negative trade fees unsupported");

    if matches!(tx.kind, TradeKind::Sell) {
        let stored = cargo.units(tx.com);
        ensure!(stored >= tx.units, "insufficient units to sell");
    }

    if matches!(tx.kind, TradeKind::Buy) {
        ensure_cargo_capacity(cargo, spec, tx.units, &catalog)?;
    }

    let view = price_view(tx.hub, tx.com, econ, rp);
    let unit_price = view.price_cents;
    let subtotal_i128 = i128::from(unit_price.as_i64()) * i128::from(tx.units);
    let subtotal = MoneyCents::from_i128_clamped(subtotal_i128);

    let fee_i128 = subtotal_i128 * i128::from(config.fee_bp) / 10_000;
    let fee_cents = MoneyCents::from_i128_clamped(fee_i128);

    let result = match tx.kind {
        TradeKind::Buy => {
            let total_i128 = subtotal_i128 + fee_i128;
            let total = MoneyCents::from_i128_clamped(total_i128);
            ensure!(
                wallet.as_i64() >= total.as_i64(),
                "insufficient wallet balance"
            );
            apply_buy(cargo, tx.com, tx.units)?;
            *wallet = wallet.saturating_sub(total);
            TradeResult {
                unit_price,
                subtotal,
                fee_cents,
                total_cents: total,
            }
        }
        TradeKind::Sell => {
            let net_i128 = subtotal_i128 - fee_i128;
            let net = MoneyCents::from_i128_clamped(net_i128);
            apply_sell(cargo, tx.com, tx.units);
            *wallet = wallet.saturating_add(net);
            TradeResult {
                unit_price,
                subtotal,
                fee_cents,
                total_cents: MoneyCents::from_i128_clamped(-net_i128),
            }
        }
    };

    #[cfg(feature = "m3_logs")]
    {
        if let Err(err) = crate::logs::trading::log_trade(tx, &result, *wallet) {
            log::warn!("failed to log trade: {err}");
        }
    }

    Ok(result)
}

fn ensure_cargo_capacity(
    cargo: &Cargo,
    spec: &CommoditySpec,
    units: u32,
    catalog: &CommodityCatalog,
) -> anyhow::Result<()> {
    let mut total_mass: u128 = 0;
    let mut total_volume: u128 = 0;
    for (id, &held_units) in &cargo.items {
        let held_spec = catalog
            .get(*id)
            .ok_or_else(|| anyhow!("unknown commodity {:?} in cargo", id))?;
        total_mass = total_mass
            .checked_add(u128::from(held_spec.mass_kg) * u128::from(held_units))
            .ok_or_else(|| anyhow!("cargo mass overflow"))?;
        total_volume = total_volume
            .checked_add(u128::from(held_spec.volume_l) * u128::from(held_units))
            .ok_or_else(|| anyhow!("cargo volume overflow"))?;
    }

    let added_mass = u128::from(spec.mass_kg) * u128::from(units);
    let added_volume = u128::from(spec.volume_l) * u128::from(units);

    let projected_mass = total_mass
        .checked_add(added_mass)
        .ok_or_else(|| anyhow!("cargo mass overflow"))?;
    let projected_volume = total_volume
        .checked_add(added_volume)
        .ok_or_else(|| anyhow!("cargo volume overflow"))?;

    ensure!(
        projected_mass <= u128::from(cargo.capacity_mass_kg),
        "cargo mass capacity exceeded"
    );
    ensure!(
        projected_volume <= u128::from(cargo.capacity_volume_l),
        "cargo volume capacity exceeded"
    );
    Ok(())
}

fn apply_buy(cargo: &mut Cargo, com: CommodityId, units: u32) -> anyhow::Result<()> {
    let entry = cargo.items.entry(com).or_insert(0);
    *entry = entry
        .checked_add(units)
        .ok_or_else(|| anyhow!("cargo units overflow for commodity {:?}", com))?;
    Ok(())
}

fn apply_sell(cargo: &mut Cargo, com: CommodityId, units: u32) {
    if let Some(entry) = cargo.items.get_mut(&com) {
        *entry -= units;
        if *entry == 0 {
            cargo.items.remove(&com);
        }
    }
}
