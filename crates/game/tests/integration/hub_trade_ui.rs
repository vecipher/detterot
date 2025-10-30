use std::path::PathBuf;

use bevy::app::App;
use bevy::prelude::*;
use game::app_state::AppState;
use game::systems::command_queue::CommandQueue;
use game::systems::economy::rulepack::load_rulepack;
use game::systems::economy::{HubId, MoneyCents, Rulepack};
use game::systems::trading::engine::TradeKind;
use game::systems::trading::types::{CommodityCatalog, TradingConfig};
use game::ui::hub_trade::{
    build_view, HubTradePlugin, HubTradeUiModel, HubTradeUiState, StepperButton, TradeButton,
};
use repro::CommandKind;

fn asset_path(relative: &str) -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest).join("..").join("..").join(relative)
}

fn install_globals() {
    let catalog_path = asset_path("assets/trading/commodities.toml");
    let catalog = CommodityCatalog::load_from_path(catalog_path.as_path()).expect("catalog");
    CommodityCatalog::install_global(catalog);
    TradingConfig::install_global(TradingConfig { fee_bp: 75 });
}

fn load_rulepack_fixture() -> Rulepack {
    let path = asset_path("assets/rulepacks/day_001.toml");
    load_rulepack(path.to_str().expect("utf-8 path")).expect("rulepack")
}

#[test]
fn stepper_buttons_update_units_and_meter_queue() {
    install_globals();
    let rp = load_rulepack_fixture();

    let mut app_state = AppState::default();
    app_state.wallet = MoneyCents(500_000);
    app_state.last_hub = HubId(1);
    app_state.cargo.capacity_mass_kg = 1_000;
    app_state.cargo.capacity_volume_l = 1_000;

    let view = build_view(
        HubId(1),
        &app_state.econ,
        &rp,
        &app_state.cargo,
        app_state.wallet,
    );

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(HubTradePlugin);
    app.world_mut().insert_resource(CommandQueue::default());
    app.world_mut().insert_resource(rp);
    app.world_mut().insert_resource(app_state);
    {
        let mut queue = app.world_mut().resource_mut::<CommandQueue>();
        queue.begin_tick(0);
    }
    {
        let mut model = app.world_mut().resource_mut::<HubTradeUiModel>();
        model.set_view(view);
    }
    app.update();

    let target_commodity = {
        let model = app.world().resource::<HubTradeUiModel>();
        model
            .view()
            .and_then(|view| view.commodities.first().cloned())
            .expect("commodity row present")
            .id
    };

    let plus_entity = {
        let world = app.world_mut();
        let mut query = world.query::<(Entity, &StepperButton)>();
        query
            .iter(&*world)
            .find(|(_, button)| button.commodity() == target_commodity && button.delta() > 0)
            .map(|(entity, _)| entity)
            .expect("plus button")
    };
    app.world_mut()
        .entity_mut(plus_entity)
        .insert(Interaction::Pressed);
    app.update();

    let units = {
        let model = app.world().resource::<HubTradeUiModel>();
        model.units_for(target_commodity)
    };
    assert_eq!(units, 2, "stepper should increment units");

    let queue_snapshot = {
        let queue = app.world().resource::<CommandQueue>();
        queue.buf.clone()
    };
    assert!(queue_snapshot.iter().any(|cmd| match &cmd.kind {
        CommandKind::Meter(m) => m.key == "ui_stepper_delta" && m.value == 1,
        _ => false,
    }));
}

#[test]
fn trade_buttons_execute_actions_and_refresh_view() {
    install_globals();
    let rp = load_rulepack_fixture();

    let mut app_state = AppState::default();
    app_state.wallet = MoneyCents(600_000);
    app_state.last_hub = HubId(1);
    app_state.cargo.capacity_mass_kg = 1_000;
    app_state.cargo.capacity_volume_l = 1_000;

    let view = build_view(
        HubId(1),
        &app_state.econ,
        &rp,
        &app_state.cargo,
        app_state.wallet,
    );

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(HubTradePlugin);
    app.world_mut().insert_resource(CommandQueue::default());
    app.world_mut().insert_resource(rp);
    app.world_mut().insert_resource(app_state);
    {
        let mut queue = app.world_mut().resource_mut::<CommandQueue>();
        queue.begin_tick(0);
    }
    {
        let mut model = app.world_mut().resource_mut::<HubTradeUiModel>();
        model.set_view(view);
    }
    app.update();

    let target_commodity = {
        let model = app.world().resource::<HubTradeUiModel>();
        model
            .view()
            .and_then(|view| view.commodities.first().cloned())
            .expect("commodity row present")
            .id
    };

    let plus_entity = {
        let world = app.world_mut();
        let mut query = world.query::<(Entity, &StepperButton)>();
        query
            .iter(&*world)
            .find(|(_, button)| button.commodity() == target_commodity && button.delta() > 0)
            .map(|(entity, _)| entity)
            .expect("plus button")
    };
    app.world_mut()
        .entity_mut(plus_entity)
        .insert(Interaction::Pressed);
    app.update();

    let buy_entity = {
        let world = app.world_mut();
        let mut query = world.query::<(Entity, &TradeButton)>();
        query
            .iter(&*world)
            .find(|(_, button)| {
                button.commodity() == target_commodity && matches!(button.kind(), TradeKind::Buy)
            })
            .map(|(entity, _)| entity)
            .expect("buy button")
    };
    app.world_mut()
        .entity_mut(buy_entity)
        .insert(Interaction::Pressed);
    app.update();

    {
        let mut queue = app.world_mut().resource_mut::<CommandQueue>();
        queue.begin_tick(1);
    }

    let sell_entity = {
        let world = app.world_mut();
        let mut query = world.query::<(Entity, &TradeButton)>();
        query
            .iter(&*world)
            .find(|(_, button)| {
                button.commodity() == target_commodity && matches!(button.kind(), TradeKind::Sell)
            })
            .map(|(entity, _)| entity)
            .expect("sell button")
    };
    app.world_mut()
        .entity_mut(sell_entity)
        .insert(Interaction::Pressed);
    app.update();

    let (wallet_after, cargo_units) = {
        let state = app.world().resource::<AppState>();
        let held = state
            .cargo
            .items
            .get(&target_commodity)
            .copied()
            .unwrap_or(0);
        (state.wallet, held)
    };
    assert!(
        cargo_units >= 1,
        "cargo should retain at least one unit after sell"
    );

    let queue_snapshot = {
        let queue = app.world().resource::<CommandQueue>();
        queue.buf.clone()
    };
    assert!(queue_snapshot.iter().any(|cmd| match &cmd.kind {
        CommandKind::Meter(m) => m.key == "ui_click_buy" && m.value == 2,
        _ => false,
    }));
    assert!(queue_snapshot.iter().any(|cmd| match &cmd.kind {
        CommandKind::Meter(m) => m.key == "ui_click_sell" && m.value == 1,
        _ => false,
    }));

    let ui_state = app.world().resource::<HubTradeUiState>();
    assert!(
        ui_state.last_view.is_some(),
        "ui state should remember last view"
    );
    let remembered_wallet = ui_state.last_view.as_ref().unwrap().wallet_cents;
    assert_eq!(
        remembered_wallet, wallet_after,
        "remembered view should track wallet"
    );
}
