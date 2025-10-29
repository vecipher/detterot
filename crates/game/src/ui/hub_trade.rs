use std::collections::HashMap;

use bevy::log::warn;
use bevy::prelude::*;

use crate::scheduling::sets;
use crate::systems::economy::{BasisBp, CommodityId, EconState, HubId, MoneyCents, Rulepack};
use crate::systems::trading::inventory::Cargo;
use crate::systems::trading::types::{Commodities, CommoditySpec};
use crate::systems::trading::{execute_trade, price_view, TradeKind, TradeTx};
use crate::ui::styles;

/// Plugin wiring the hub trade UI view models into the Bevy app.
#[derive(Default)]
pub struct HubTradeUiPlugin;

impl Plugin for HubTradeUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HubTradeViewModel>()
            .init_resource::<HubTradeCatalog>()
            .init_resource::<ActiveHub>()
            .init_resource::<SelectedCommodity>()
            .init_resource::<WalletBalance>()
            .init_resource::<UnitSteppersState>()
            .add_message::<BuyUnitsEvent>()
            .add_message::<SellUnitsEvent>()
            .add_systems(
                FixedUpdate,
                (
                    drive_buy_units,
                    drive_sell_units,
                    update_di_ticker,
                    update_driver_chips,
                    update_cargo_wallet_panels,
                    update_commodity_list,
                    update_unit_steppers,
                )
                    .chain()
                    .in_set(sets::DETTEROT_Cleanup),
            );
    }
}

/// Resource tracking which hub the UI is currently displaying.
#[derive(Resource, Copy, Clone, Debug, PartialEq, Eq)]
pub struct ActiveHub(pub HubId);

impl Default for ActiveHub {
    fn default() -> Self {
        Self(HubId(0))
    }
}

/// Resource representing the selected commodity row (if any).
#[derive(Resource, Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct SelectedCommodity(pub Option<CommodityId>);

/// Player wallet balance mirrored into the UI layer.
#[derive(Resource, Copy, Clone, Debug, PartialEq, Eq)]
pub struct WalletBalance(pub MoneyCents);

impl Default for WalletBalance {
    fn default() -> Self {
        Self(MoneyCents::ZERO)
    }
}

/// Metadata describing a tradable commodity in the UI context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommodityMarketMetadata {
    pub base_price: MoneyCents,
    pub volume_per_unit: u32,
    pub mass_per_unit: u32,
}

/// Catalog mapping commodity identifiers to market metadata required by the UI.
#[derive(Resource, Default, Debug, Clone)]
pub struct HubTradeCatalog {
    entries: HashMap<CommodityId, CommodityMarketMetadata>,
}

impl HubTradeCatalog {
    pub fn insert(
        &mut self,
        commodity: CommodityId,
        base_price: MoneyCents,
        volume_per_unit: u32,
        mass_per_unit: u32,
    ) {
        self.entries.insert(
            commodity,
            CommodityMarketMetadata {
                base_price,
                volume_per_unit,
                mass_per_unit,
            },
        );
    }

    pub fn get(&self, commodity: CommodityId) -> Option<&CommodityMarketMetadata> {
        self.entries.get(&commodity)
    }
}

/// Event emitted by unit steppers to request a buy trade for a commodity.
#[derive(Debug, Clone, PartialEq, Eq, Message)]
pub struct BuyUnitsEvent {
    pub commodity: CommodityId,
    pub units: u32,
}

/// Event emitted by unit steppers to request a sell trade for a commodity.
#[derive(Debug, Clone, PartialEq, Eq, Message)]
pub struct SellUnitsEvent {
    pub commodity: CommodityId,
    pub units: u32,
}

/// Aggregated UI view model for the hub trade screen.
#[derive(Resource, Default, Clone, Debug)]
pub struct HubTradeViewModel {
    pub di_ticker: DiTickerVm,
    pub commodity_list: CommodityListVm,
    pub driver_chips: DriverChipsVm,
    pub cargo_panel: CargoPanelVm,
    pub wallet_panel: WalletPanelVm,
    pub buy_stepper: UnitStepperVm,
    pub sell_stepper: UnitStepperVm,
}

/// View model describing the DI ticker strip.
#[derive(Default, Clone, Debug)]
pub struct DiTickerVm {
    pub entries: Vec<DiTickerEntryVm>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DiTickerEntryVm {
    pub commodity: CommodityId,
    pub display_name: String,
    pub di_bp: i32,
    pub colour: Color,
}

/// Commodity list view model covering price quotes and positions.
#[derive(Default, Clone, Debug)]
pub struct CommodityListVm {
    pub rows: Vec<CommodityRowVm>,
    pub selected: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CommodityRowVm {
    pub commodity: CommodityId,
    pub display_name: String,
    pub di_bp: i32,
    pub basis_bp: i32,
    pub unit_price: MoneyCents,
    pub held_units: u32,
    pub max_buy: u32,
    pub max_sell: u32,
    pub can_buy: bool,
    pub can_sell: bool,
}

/// Chip-style summary of pricing drivers.
#[derive(Default, Clone, Debug)]
pub struct DriverChipsVm {
    pub chips: Vec<DriverChipVm>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DriverChipVm {
    pub label: String,
    pub value: String,
    pub colour: Color,
}

/// Summary of cargo capacity usage presented to the player.
#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct CargoPanelVm {
    pub capacity_total: u32,
    pub capacity_used: u32,
    pub mass_capacity_total: u32,
    pub mass_capacity_used: u32,
}

/// Summary of wallet balance presented to the player.
#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct WalletPanelVm {
    pub balance: MoneyCents,
}

/// State of a buy/sell unit stepper in the UI.
#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct UnitStepperVm {
    pub step: u32,
    pub max: u32,
    pub last_units: u32,
}

/// Internal resource tracking stepper settings and their last executed trades.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct UnitSteppersState {
    pub buy_step: u32,
    pub sell_step: u32,
    pub max_buy: u32,
    pub max_sell: u32,
    pub last_buy_units: u32,
    pub last_sell_units: u32,
}

impl Default for UnitSteppersState {
    fn default() -> Self {
        Self {
            buy_step: 1,
            sell_step: 1,
            max_buy: 0,
            max_sell: 0,
            last_buy_units: 0,
            last_sell_units: 0,
        }
    }
}

fn update_di_ticker(
    econ: Res<EconState>,
    commodities: Res<Commodities>,
    mut vm: ResMut<HubTradeViewModel>,
) {
    if econ.is_changed() || commodities.is_changed() {
        let mut entries = Vec::new();
        for spec in commodities.iter() {
            entries.push(build_di_entry(spec, econ.di_bp.get(&spec.id()).copied()))
        }
        vm.di_ticker.entries = entries;
    }
}

fn build_di_entry(spec: &CommoditySpec, value: Option<BasisBp>) -> DiTickerEntryVm {
    let di_bp = value.unwrap_or(BasisBp(0)).0;
    let colour = if di_bp > 0 {
        styles::positive()
    } else if di_bp < 0 {
        styles::negative()
    } else {
        styles::neutral()
    };

    DiTickerEntryVm {
        commodity: spec.id(),
        display_name: spec.display_name().to_string(),
        di_bp,
        colour,
    }
}

fn update_driver_chips(
    econ: Res<EconState>,
    rulepack: Res<Rulepack>,
    mut vm: ResMut<HubTradeViewModel>,
) {
    if !econ.is_changed() && !rulepack.is_changed() {
        return;
    }

    let view = price_view(&econ, &rulepack.pricing);
    let mut chips = Vec::new();

    let pp_value = view.pp().0;
    let neutral_pp = rulepack.pp.neutral_pp;
    let pp_colour = if pp_value > neutral_pp {
        styles::positive()
    } else if pp_value < neutral_pp {
        styles::negative()
    } else {
        styles::neutral()
    };
    chips.push(DriverChipVm {
        label: "PP".to_string(),
        value: pp_value.to_string(),
        colour: pp_colour,
    });

    chips.push(DriverChipVm {
        label: "Weather".to_string(),
        value: format_weather(view.weather()).to_string(),
        colour: styles::accent(),
    });

    let closed_routes = view.closed_routes();
    chips.push(DriverChipVm {
        label: "Routes".to_string(),
        value: closed_routes.to_string(),
        colour: if closed_routes == 0 {
            styles::neutral()
        } else {
            styles::negative()
        },
    });

    let stock_dev = view.stock_dev();
    chips.push(DriverChipVm {
        label: "Stock".to_string(),
        value: stock_dev.to_string(),
        colour: if stock_dev >= 0 {
            styles::positive()
        } else {
            styles::negative()
        },
    });

    vm.driver_chips.chips = chips;
}

fn format_weather(weather: crate::systems::economy::Weather) -> &'static str {
    use crate::systems::economy::Weather;
    match weather {
        Weather::Clear => "Clear",
        Weather::Rains => "Rain",
        Weather::Fog => "Fog",
        Weather::Windy => "Wind",
    }
}

fn update_cargo_wallet_panels(
    cargo: Res<Cargo>,
    wallet: Res<WalletBalance>,
    mut vm: ResMut<HubTradeViewModel>,
) {
    if cargo.is_changed() {
        vm.cargo_panel = CargoPanelVm {
            capacity_total: cargo.capacity_total,
            capacity_used: cargo.capacity_used,
            mass_capacity_total: cargo.mass_capacity_total,
            mass_capacity_used: cargo.mass_capacity_used,
        };
    }

    if wallet.is_changed() {
        vm.wallet_panel = WalletPanelVm { balance: wallet.0 };
    }
}

#[allow(clippy::too_many_arguments)]
fn update_commodity_list(
    econ: Res<EconState>,
    rulepack: Res<Rulepack>,
    commodities: Res<Commodities>,
    catalog: Res<HubTradeCatalog>,
    cargo: Res<Cargo>,
    wallet: Res<WalletBalance>,
    hub: Res<ActiveHub>,
    selected: Res<SelectedCommodity>,
    mut vm: ResMut<HubTradeViewModel>,
) {
    if !(econ.is_changed()
        || rulepack.is_changed()
        || commodities.is_changed()
        || catalog.is_changed()
        || cargo.is_changed()
        || wallet.is_changed()
        || selected.is_changed())
    {
        return;
    }

    let pricing = price_view(&econ, &rulepack.pricing);
    let mut rows = Vec::new();
    let mut selected_index = None;

    for spec in commodities.iter() {
        let Some(meta) = catalog.get(spec.id()) else {
            continue;
        };

        let di_bp = econ.di_bp.get(&spec.id()).copied().unwrap_or(BasisBp(0)).0;
        let basis_bp = econ
            .basis_bp
            .get(&(hub.0, spec.id()))
            .copied()
            .unwrap_or(BasisBp(0))
            .0;
        let unit_price = pricing.quote(hub.0, spec.id(), meta.base_price);
        let held_units = cargo.units(spec.id());
        let max_buy = compute_max_buy_units(
            &cargo,
            meta,
            wallet.0,
            unit_price,
            rulepack.trading.transaction_fee_bp,
        );
        let max_sell = held_units;
        let can_buy = max_buy > 0;
        let can_sell = max_sell > 0;

        if selected.0 == Some(spec.id()) {
            selected_index = Some(rows.len());
        }

        rows.push(CommodityRowVm {
            commodity: spec.id(),
            display_name: spec.display_name().to_string(),
            di_bp,
            basis_bp,
            unit_price,
            held_units,
            max_buy,
            max_sell,
            can_buy,
            can_sell,
        });
    }

    vm.commodity_list.rows = rows;
    vm.commodity_list.selected = selected_index;
}

#[allow(clippy::too_many_arguments)]
fn update_unit_steppers(
    econ: Res<EconState>,
    rulepack: Res<Rulepack>,
    catalog: Res<HubTradeCatalog>,
    cargo: Res<Cargo>,
    wallet: Res<WalletBalance>,
    hub: Res<ActiveHub>,
    selected: Res<SelectedCommodity>,
    mut steppers: ResMut<UnitSteppersState>,
    mut vm: ResMut<HubTradeViewModel>,
) {
    let pricing = price_view(&econ, &rulepack.pricing);

    if let Some(commodity) = selected.0 {
        if let Some(meta) = catalog.get(commodity) {
            let unit_price = pricing.quote(hub.0, commodity, meta.base_price);
            steppers.max_buy = compute_max_buy_units(
                &cargo,
                meta,
                wallet.0,
                unit_price,
                rulepack.trading.transaction_fee_bp,
            );
            steppers.max_sell = cargo.units(commodity);
        } else {
            steppers.max_buy = 0;
            steppers.max_sell = 0;
        }
    } else {
        steppers.max_buy = 0;
        steppers.max_sell = 0;
    }

    vm.buy_stepper = UnitStepperVm {
        step: steppers.buy_step,
        max: steppers.max_buy,
        last_units: steppers.last_buy_units,
    };
    vm.sell_stepper = UnitStepperVm {
        step: steppers.sell_step,
        max: steppers.max_sell,
        last_units: steppers.last_sell_units,
    };
}

#[allow(clippy::too_many_arguments)]
fn drive_buy_units(
    mut events: MessageReader<BuyUnitsEvent>,
    mut cargo: ResMut<Cargo>,
    econ: Res<EconState>,
    rulepack: Res<Rulepack>,
    catalog: Res<HubTradeCatalog>,
    hub: Res<ActiveHub>,
    mut wallet: ResMut<WalletBalance>,
    mut steppers: ResMut<UnitSteppersState>,
) {
    if events.is_empty() {
        return;
    }

    let pricing = price_view(&econ, &rulepack.pricing);

    for event in events.read() {
        if event.units == 0 {
            continue;
        }
        let Some(meta) = catalog.get(event.commodity) else {
            continue;
        };
        let requested = event.units;
        if requested == 0 {
            continue;
        }
        let tx = TradeTx {
            kind: TradeKind::Buy,
            hub: hub.0,
            commodity: event.commodity,
            units: requested,
            base_price: meta.base_price,
            volume_per_unit: meta.volume_per_unit,
            mass_per_unit: meta.mass_per_unit,
        };
        let mut wallet_value = wallet.0;
        match execute_trade(&tx, &pricing, &rulepack, cargo.as_mut(), &mut wallet_value) {
            Ok(result) => {
                wallet.0 = wallet_value;
                steppers.last_buy_units = result.units_executed;
            }
            Err(err) => {
                warn!("buy trade failed for {:?}: {err}", event.commodity);
                steppers.last_buy_units = 0;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn drive_sell_units(
    mut events: MessageReader<SellUnitsEvent>,
    mut cargo: ResMut<Cargo>,
    econ: Res<EconState>,
    rulepack: Res<Rulepack>,
    catalog: Res<HubTradeCatalog>,
    hub: Res<ActiveHub>,
    mut wallet: ResMut<WalletBalance>,
    mut steppers: ResMut<UnitSteppersState>,
) {
    if events.is_empty() {
        return;
    }

    let pricing = price_view(&econ, &rulepack.pricing);

    for event in events.read() {
        if event.units == 0 {
            continue;
        }
        let Some(meta) = catalog.get(event.commodity) else {
            continue;
        };
        let requested = event.units;
        if requested == 0 {
            continue;
        }
        let tx = TradeTx {
            kind: TradeKind::Sell,
            hub: hub.0,
            commodity: event.commodity,
            units: requested,
            base_price: meta.base_price,
            volume_per_unit: meta.volume_per_unit,
            mass_per_unit: meta.mass_per_unit,
        };
        let mut wallet_value = wallet.0;
        match execute_trade(&tx, &pricing, &rulepack, cargo.as_mut(), &mut wallet_value) {
            Ok(result) => {
                wallet.0 = wallet_value;
                steppers.last_sell_units = result.units_executed;
            }
            Err(err) => {
                warn!("sell trade failed for {:?}: {err}", event.commodity);
                steppers.last_sell_units = 0;
            }
        }
    }
}

fn compute_max_buy_units(
    cargo: &Cargo,
    meta: &CommodityMarketMetadata,
    wallet: MoneyCents,
    unit_price: MoneyCents,
    fee_bp: i32,
) -> u32 {
    const MAX_REQUEST_BOUND: u32 = 1_000_000;
    let available_volume = cargo.capacity_total.saturating_sub(cargo.capacity_used);
    let available_mass = cargo
        .mass_capacity_total
        .saturating_sub(cargo.mass_capacity_used);

    let volume_cap = if meta.volume_per_unit == 0 {
        u32::MAX
    } else {
        available_volume / meta.volume_per_unit
    };
    let mass_cap = if meta.mass_per_unit == 0 {
        u32::MAX
    } else {
        available_mass / meta.mass_per_unit
    };
    let mut capacity_cap = volume_cap.min(mass_cap);
    if capacity_cap == 0 {
        return 0;
    }
    capacity_cap = capacity_cap.min(MAX_REQUEST_BOUND);

    let wallet_cap = max_units_affordable(wallet, unit_price, fee_bp, capacity_cap);
    capacity_cap.min(wallet_cap)
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
            hi = mid.saturating_sub(1);
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
