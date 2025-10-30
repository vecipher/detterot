use std::collections::HashMap;

use anyhow::Result;
use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::ui::UiRect;

use crate::app_state::AppState;
use crate::systems::command_queue::CommandQueue;
use crate::systems::economy::{
    BasisBp, CommodityId, EconState, EconomyDay, HubId, MoneyCents, Rulepack,
};
use crate::systems::trading::engine::{execute_trade, TradeKind, TradeResult, TradeTx};
use crate::systems::trading::inventory::Cargo;
use crate::systems::trading::pricing_vm::{price_view, TradingDrivers};
use crate::systems::trading::types::{CommodityCatalog, TradingConfig};
use crate::ui::styles::{
    COLOR_ACCENT_NEG, COLOR_ACCENT_POS, COLOR_BG, COLOR_TEXT_PRIMARY, COLOR_TEXT_SECONDARY,
};

type ButtonInteractionFilter = (Changed<Interaction>, With<Button>);
type StepperInteraction<'w> = (&'w Interaction, &'w StepperButton);
type TradeInteraction<'w> = (&'w Interaction, &'w TradeButton);
type UiTextParamSet<'w, 's> = ParamSet<
    'w,
    's,
    (
        Query<'w, 's, &'static mut Text, With<TickerText>>,
        Query<'w, 's, &'static mut Text, With<WalletText>>,
        Query<'w, 's, &'static mut Text, With<CargoSummaryText>>,
    ),
>;

#[derive(SystemParam)]
struct UiTextQueries<'w, 's> {
    sets: UiTextParamSet<'w, 's>,
}

#[derive(Resource, Default)]
pub struct HubTradeUiState {
    pub last_view: Option<HubTradeView>,
}

#[derive(Resource, Default)]
pub struct HubTradeUiModel {
    view: Option<HubTradeView>,
    stepper_units: HashMap<CommodityId, u32>,
    dirty_view: bool,
}

pub struct HubTradePlugin;

impl Plugin for HubTradePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HubTradeUiState>()
            .init_resource::<HubTradeUiModel>()
            .add_systems(Startup, setup_hub_trade_ui)
            .add_systems(Update, apply_hub_trade_view)
            .add_systems(Update, handle_stepper_buttons)
            .add_systems(Update, handle_trade_buttons);
    }
}

impl HubTradeUiState {
    pub fn remember(&mut self, view: HubTradeView) {
        self.last_view = Some(view);
    }
}

impl HubTradeUiModel {
    pub fn set_view(&mut self, view: HubTradeView) {
        self.stepper_units.clear();
        for row in &view.commodities {
            self.stepper_units.insert(row.id, 1);
        }
        self.dirty_view = true;
        self.view = Some(view);
    }

    pub fn view(&self) -> Option<&HubTradeView> {
        self.view.as_ref()
    }

    pub fn units_for(&self, commodity: CommodityId) -> u32 {
        self.stepper_units.get(&commodity).copied().unwrap_or(1)
    }

    fn set_units(&mut self, commodity: CommodityId, units: u32) {
        self.stepper_units.insert(commodity, units);
    }

    fn take_dirty(&mut self) -> bool {
        let dirty = self.dirty_view;
        self.dirty_view = false;
        dirty
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

fn ticker_line(view: &HubTradeView) -> String {
    let mut parts = vec![
        format!("Hub {}", view.hub.0),
        format!("Day {}", view.day.0),
        format!("Avg ΔDI {}bp", view.di_bp.0),
    ];
    if view.clamp_hit {
        parts.push("Clamp active".to_string());
    }
    parts.join(" • ")
}

fn wallet_line(view: &HubTradeView) -> String {
    format!(
        "Wallet: {}¢ (fee {}bp)",
        view.wallet_cents.as_i64(),
        view.fee_bp
    )
}

fn cargo_line(view: &HubTradeView) -> String {
    let total_units: u32 = view.cargo.items.iter().map(|row| row.units).sum();
    format!(
        "Cargo: {} units • {}kg / {}L",
        total_units, view.cargo.capacity_mass_kg, view.cargo.capacity_volume_l
    )
}

fn format_price(cents: MoneyCents) -> String {
    format!("{}¢", cents.as_i64())
}

#[derive(Component)]
struct HubTradeRoot;

#[derive(Component)]
struct CommodityTableRoot;

#[derive(Component)]
struct CommodityRowUi;

#[derive(Component)]
struct StepperValueText {
    commodity: CommodityId,
}

#[derive(Component, Clone, Copy)]
pub struct StepperButton {
    commodity: CommodityId,
    delta: i32,
}

#[derive(Component, Clone, Copy)]
pub struct TradeButton {
    commodity: CommodityId,
    kind: TradeKind,
}

impl StepperButton {
    pub fn commodity(&self) -> CommodityId {
        self.commodity
    }

    pub fn delta(&self) -> i32 {
        self.delta
    }
}

impl TradeButton {
    pub fn commodity(&self) -> CommodityId {
        self.commodity
    }

    pub fn kind(&self) -> TradeKind {
        self.kind
    }
}

#[derive(Component)]
struct TickerText;

#[derive(Component)]
struct WalletText;

#[derive(Component)]
struct CargoSummaryText;

fn setup_hub_trade_ui(mut commands: Commands) {
    let (ticker_text, ticker_font, ticker_color) =
        text_components("Awaiting market data", 18.0, COLOR_TEXT_PRIMARY);
    let (wallet_text, wallet_font, wallet_color) =
        text_components("Wallet: --", 16.0, COLOR_TEXT_PRIMARY);
    let (cargo_text, cargo_font, cargo_color) =
        text_components("Cargo: --", 14.0, COLOR_TEXT_SECONDARY);

    commands
        .spawn((
            HubTradeRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                padding: UiRect::all(Val::Px(12.0)),
                ..Default::default()
            },
            BackgroundColor(COLOR_BG),
        ))
        .with_children(|root| {
            root.spawn((TickerText, ticker_text, ticker_font, ticker_color));

            root.spawn((
                CommodityTableRoot,
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    ..Default::default()
                },
            ));

            root.spawn((Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..Default::default()
            },))
                .with_children(|panel| {
                    panel.spawn((WalletText, wallet_text, wallet_font, wallet_color));
                    panel.spawn((CargoSummaryText, cargo_text, cargo_font, cargo_color));
                });
        });
}

fn apply_hub_trade_view(
    mut commands: Commands,
    mut model: ResMut<HubTradeUiModel>,
    mut ui_state: ResMut<HubTradeUiState>,
    mut text_queries: UiTextQueries,
    table_query: Query<Entity, With<CommodityTableRoot>>,
    existing_rows: Query<Entity, With<CommodityRowUi>>,
    children_query: Query<&Children>,
) {
    if !model.take_dirty() {
        return;
    }
    let Some(view) = model.view.clone() else {
        return;
    };

    ui_state.remember(view.clone());

    if let Some(mut ticker) = text_queries.sets.p0().iter_mut().next() {
        ticker.0 = ticker_line(&view);
    }
    if let Some(mut wallet_text) = text_queries.sets.p1().iter_mut().next() {
        wallet_text.0 = wallet_line(&view);
    }
    if let Some(mut cargo_text) = text_queries.sets.p2().iter_mut().next() {
        cargo_text.0 = cargo_line(&view);
    }

    for entity in existing_rows.iter() {
        despawn_recursive(&mut commands, entity, &children_query);
    }

    let Some(table_entity) = table_query.iter().next() else {
        return;
    };

    let units_snapshot = model.stepper_units.clone();
    commands.entity(table_entity).with_children(|table| {
        for row in &view.commodities {
            let units = units_snapshot.get(&row.id).copied().unwrap_or(1);
            spawn_commodity_row(table, row, units);
        }
    });
}

fn handle_stepper_buttons(
    mut interactions: Query<StepperInteraction<'_>, ButtonInteractionFilter>,
    mut model: ResMut<HubTradeUiModel>,
    mut queue: ResMut<CommandQueue>,
    mut texts: Query<(&mut Text, &StepperValueText)>,
) {
    if model.view().is_none() {
        return;
    }

    let mut updates: Vec<(CommodityId, u32)> = Vec::new();
    for (interaction, button) in interactions.iter_mut() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let current = model.units_for(button.commodity);
        let updated = if button.delta < 0 {
            current.saturating_sub(button.delta.unsigned_abs())
        } else {
            current.saturating_add(button.delta as u32)
        };
        if updated == current {
            continue;
        }

        model.set_units(button.commodity, updated);
        let diff_i64 = updated as i64 - current as i64;
        let diff = diff_i64.clamp(i32::MIN as i64, i32::MAX as i64) as i32;
        queue.meter("ui_stepper_delta", diff);
        updates.push((button.commodity, updated));
    }

    for (commodity, value) in updates {
        for (mut text, marker) in texts.iter_mut() {
            if marker.commodity == commodity {
                text.0 = value.to_string();
            }
        }
    }
}

fn handle_trade_buttons(
    mut interactions: Query<TradeInteraction<'_>, ButtonInteractionFilter>,
    mut model: ResMut<HubTradeUiModel>,
    mut ui_state: ResMut<HubTradeUiState>,
    mut queue: ResMut<CommandQueue>,
    mut app_state: ResMut<AppState>,
    rp: Res<Rulepack>,
) {
    let Some(view) = model.view().cloned() else {
        return;
    };

    let mut triggered: Vec<TradeButton> = Vec::new();
    for (interaction, button) in interactions.iter_mut() {
        if *interaction == Interaction::Pressed {
            triggered.push(*button);
        }
    }

    for button in triggered {
        let units = model.units_for(button.commodity);
        if units == 0 {
            continue;
        }
        let tx = TradeTx {
            hub: view.hub,
            com: button.commodity,
            units,
            kind: button.kind,
        };
        let result = {
            let AppState {
                econ,
                cargo,
                wallet,
                ..
            } = &mut *app_state;
            match button.kind {
                TradeKind::Buy => {
                    HubTradeActions::buy(queue.as_mut(), tx, &*econ, cargo, wallet, rp.as_ref())
                }
                TradeKind::Sell => {
                    HubTradeActions::sell(queue.as_mut(), tx, &*econ, cargo, wallet, rp.as_ref())
                }
            }
        };

        match result {
            Ok(_) => {
                let new_view = build_view(
                    view.hub,
                    &app_state.econ,
                    rp.as_ref(),
                    &app_state.cargo,
                    app_state.wallet,
                );
                model.set_view(new_view.clone());
                ui_state.remember(new_view);
            }
            Err(err) => {
                warn!("failed to execute trade: {err:?}");
            }
        }
    }
}

fn despawn_recursive(commands: &mut Commands, entity: Entity, children_query: &Query<&Children>) {
    if let Ok(children) = children_query.get(entity) {
        for child in children.iter() {
            despawn_recursive(commands, child, children_query);
        }
    }
    commands.entity(entity).despawn();
}

fn spawn_commodity_row(parent: &mut ChildSpawnerCommands, row: &CommodityRow, units: u32) {
    parent
        .spawn((
            CommodityRowUi,
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
                ..Default::default()
            },
        ))
        .with_children(|row_node| {
            let (name_text, name_font, name_color) =
                text_components(row.name.clone(), 16.0, COLOR_TEXT_PRIMARY);
            row_node.spawn((name_text, name_font, name_color));

            let (price_text, price_font, price_color) =
                text_components(format_price(row.price_cents), 14.0, COLOR_TEXT_SECONDARY);
            row_node.spawn((price_text, price_font, price_color));

            let (units_text, units_font, units_color) =
                text_components(units.to_string(), 14.0, COLOR_TEXT_PRIMARY);
            row_node.spawn((
                StepperValueText { commodity: row.id },
                units_text,
                units_font,
                units_color,
            ));

            spawn_stepper_button(row_node, row.id, -1, "−");
            spawn_stepper_button(row_node, row.id, 1, "+");
            spawn_trade_button(row_node, row.id, TradeKind::Buy, "Buy");
            spawn_trade_button(row_node, row.id, TradeKind::Sell, "Sell");
        });
}

fn spawn_stepper_button(
    parent: &mut ChildSpawnerCommands,
    commodity: CommodityId,
    delta: i32,
    label: &str,
) {
    parent
        .spawn((
            StepperButton { commodity, delta },
            Button,
            Node {
                padding: UiRect::all(Val::Px(6.0)),
                min_width: Val::Px(28.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..Default::default()
            },
            BackgroundColor(COLOR_TEXT_SECONDARY.with_alpha(0.25)),
        ))
        .with_children(|button| {
            let (text, font, color) = text_components(label, 14.0, COLOR_TEXT_PRIMARY);
            button.spawn((text, font, color));
        });
}

fn spawn_trade_button(
    parent: &mut ChildSpawnerCommands,
    commodity: CommodityId,
    kind: TradeKind,
    label: &str,
) {
    let color = match kind {
        TradeKind::Buy => COLOR_ACCENT_POS,
        TradeKind::Sell => COLOR_ACCENT_NEG,
    };
    parent
        .spawn((
            TradeButton { commodity, kind },
            Button,
            Node {
                padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..Default::default()
            },
            BackgroundColor(color.with_alpha(0.85)),
        ))
        .with_children(|button| {
            let (text, font, color) = text_components(label, 14.0, COLOR_TEXT_PRIMARY);
            button.spawn((text, font, color));
        });
}

fn text_components(
    value: impl Into<String>,
    size: f32,
    color: Color,
) -> (Text, TextFont, TextColor) {
    (
        Text::new(value.into()),
        TextFont {
            font_size: size,
            ..Default::default()
        },
        TextColor(color),
    )
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
