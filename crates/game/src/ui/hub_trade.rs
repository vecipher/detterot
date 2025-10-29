use anyhow::Result;
use bevy::prelude::*;

use crate::app_state::AppState;
use crate::systems::command_queue::CommandQueue;
use crate::systems::economy::{
    BasisBp, CommodityId, EconState, EconomyDay, HubId, MoneyCents, Rulepack,
};
use crate::systems::trading::engine::{execute_trade, TradeKind, TradeResult, TradeTx};
use crate::systems::trading::inventory::Cargo;
use crate::systems::trading::pricing_vm::{price_view, TradingDrivers};
use crate::systems::trading::types::{CommodityCatalog, TradingConfig};

#[derive(Resource, Default)]
pub struct HubTradeUiState {
    pub last_view: Option<HubTradeView>,
}

pub struct HubTradePlugin;

impl Plugin for HubTradePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HubTradeUiState>();
    }
}

impl HubTradeUiState {
    pub fn remember(&mut self, view: HubTradeView) {
        self.last_view = Some(view);
    }
}

#[derive(Debug, Clone)]
pub struct CommodityRow {
    pub id: CommodityId,
    pub name: String,
    pub di_bp: BasisBp,
    pub basis_bp: BasisBp,
    pub price_cents: MoneyCents,
    pub drivers: TradingDrivers,
}

#[derive(Debug, Clone)]
pub struct CargoItemRow {
    pub commodity: CommodityId,
    pub units: u32,
}

#[derive(Debug, Clone)]
pub struct CargoSummary {
    pub capacity_mass_kg: u32,
    pub capacity_volume_l: u32,
    pub items: Vec<CargoItemRow>,
}

#[derive(Debug, Clone)]
pub struct HubTradeView {
    pub hub: HubId,
    pub day: EconomyDay,
    pub di_bp: BasisBp,
    pub clamp_hit: bool,
    pub commodities: Vec<CommodityRow>,
    pub cargo: CargoSummary,
    pub wallet_cents: MoneyCents,
    pub fee_bp: i32,
}

pub fn build_view(
    hub: HubId,
    econ: &EconState,
    rp: &Rulepack,
    cargo: &Cargo,
    wallet: MoneyCents,
) -> HubTradeView {
    let catalog = CommodityCatalog::global();
    let di_cfg = &rp.di;
    let basis_cfg = &rp.basis;
    let mut di_total: i64 = 0;
    let mut commodities: Vec<CommodityRow> = Vec::with_capacity(catalog.list().len());
    let mut clamp_hit = false;
    if econ.di_overlay_bp.abs() >= rp.di.per_day_clamp_bp
        || econ.di_overlay_bp <= rp.di.overlay_min_bp
        || econ.di_overlay_bp >= rp.di.overlay_max_bp
    {
        clamp_hit = true;
    }
    for spec in catalog.list() {
        let view = price_view(hub, spec.id, econ, rp);
        di_total += i64::from(view.di_bp.0);
        if view.di_bp.0 <= di_cfg.absolute_min_bp || view.di_bp.0 >= di_cfg.absolute_max_bp {
            clamp_hit = true;
        }
        if view.basis_bp.0 <= basis_cfg.absolute_min_bp
            || view.basis_bp.0 >= basis_cfg.absolute_max_bp
        {
            clamp_hit = true;
        }
        commodities.push(CommodityRow {
            id: spec.id,
            name: spec.name.clone(),
            di_bp: view.di_bp,
            basis_bp: view.basis_bp,
            price_cents: view.price_cents,
            drivers: view.drivers,
        });
    }

    let mut cargo_items: Vec<CargoItemRow> = cargo
        .items
        .iter()
        .map(|(commodity, units)| CargoItemRow {
            commodity: *commodity,
            units: *units,
        })
        .collect();
    cargo_items.sort_by_key(|row| row.commodity.0);

    let fee_bp = TradingConfig::global().fee_bp;
    let di_bp = if commodities.is_empty() {
        BasisBp(0)
    } else {
        let average = di_total / commodities.len() as i64;
        BasisBp(average as i32)
    };

    HubTradeView {
        hub,
        day: econ.day,
        di_bp,
        clamp_hit,
        commodities,
        cargo: CargoSummary {
            capacity_mass_kg: cargo.capacity_mass_kg,
            capacity_volume_l: cargo.capacity_volume_l,
            items: cargo_items,
        },
        wallet_cents: wallet,
        fee_bp,
    }
}

pub struct HubTradeActions;

impl HubTradeActions {
    pub fn buy(
        queue: &mut CommandQueue,
        tx: TradeTx,
        econ: &EconState,
        cargo: &mut Cargo,
        wallet: &mut MoneyCents,
        rp: &Rulepack,
    ) -> Result<TradeResult> {
        debug_assert!(matches!(tx.kind, TradeKind::Buy));
        queue.meter_units("ui_click_buy", tx.units);
        execute_trade(&tx, econ, cargo, wallet, rp)
    }

    pub fn sell(
        queue: &mut CommandQueue,
        tx: TradeTx,
        econ: &EconState,
        cargo: &mut Cargo,
        wallet: &mut MoneyCents,
        rp: &Rulepack,
    ) -> Result<TradeResult> {
        debug_assert!(matches!(tx.kind, TradeKind::Sell));
        queue.meter_units("ui_click_sell", tx.units);
        execute_trade(&tx, econ, cargo, wallet, rp)
    }
}

pub fn persist_on_exit(state: &HubTradeUiState, app: &mut AppState) {
    if let Some(view) = &state.last_view {
        app.last_hub = view.hub;
        app.wallet = view.wallet_cents;
        app.cargo.capacity_mass_kg = view.cargo.capacity_mass_kg;
        app.cargo.capacity_volume_l = view.cargo.capacity_volume_l;
        app.cargo.items = view
            .cargo
            .items
            .iter()
            .map(|row| (row.commodity, row.units))
            .collect();
    }
}
