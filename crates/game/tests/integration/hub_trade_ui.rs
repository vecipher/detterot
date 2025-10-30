use std::path::PathBuf;

use bevy::app::App;
use bevy::prelude::*;
use game::app_state::AppState;
use game::systems::command_queue::CommandQueue;
use game::systems::economy::rulepack::load_rulepack;
use game::systems::economy::{CommodityId, HubId, MoneyCents, Rulepack};
use game::systems::trading::engine::TradeKind;
use game::systems::trading::inventory::Cargo;
use game::systems::trading::types::{CommodityCatalog, TradingConfig};
use game::ui::hub_trade::{
    HubTradePlugin, HubTradeUiModel, HubTradeUiState, StepperButton, TradeButton,
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

fn warm_up_hub_trade_ui(app: &mut App) {
    for _ in 0..10 {
        app.update();
        let view_ready = {
            let model = app.world().resource::<HubTradeUiModel>();
            model.view().is_some()
        };
        if !view_ready {
            continue;
        }

        app.update();

        let stepper_ready = {
            let world = app.world_mut();
            let mut query = world.query::<&StepperButton>();
            query.iter(&*world).next().is_some()
        };

        if stepper_ready {
            return;
        }
    }
    panic!("hub trade view never became available");
}

fn collect_stepper_buttons(app: &mut App) -> Vec<(Entity, CommodityId, i32)> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &StepperButton)>();
    query
        .iter(&*world)
        .map(|(entity, button)| (entity, button.commodity(), button.delta()))
        .collect()
}

fn collect_trade_buttons(app: &mut App) -> Vec<(Entity, CommodityId, TradeKind)> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &TradeButton)>();
    query
        .iter(&*world)
        .map(|(entity, button)| (entity, button.commodity(), button.kind()))
        .collect()
}

#[test]
fn stepper_buttons_update_units_and_meter_queue() {
    install_globals();
    let rp = load_rulepack_fixture();

    let app_state = AppState {
        wallet: MoneyCents(500_000),
        last_hub: HubId(1),
        cargo: Cargo {
            capacity_mass_kg: 1_000,
            capacity_volume_l: 1_000,
            ..Default::default()
        },
        ..Default::default()
    };

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
    warm_up_hub_trade_ui(&mut app);

    let target_commodity = {
        let model = app.world().resource::<HubTradeUiModel>();
        model
            .view()
            .and_then(|view| view.commodities.first().cloned())
            .expect("commodity row present")
            .id
    };

    let steppers = collect_stepper_buttons(&mut app);
    assert!(
        steppers
            .iter()
            .any(|(_, commodity, delta)| *commodity == target_commodity && *delta > 0),
        "available steppers: {:?}",
        steppers
    );

    let plus_entity = steppers
        .iter()
        .find(|(_, commodity, delta)| *commodity == target_commodity && *delta > 0)
        .map(|(entity, _, _)| *entity)
        .expect("plus button");
    app.world_mut()
        .entity_mut(plus_entity)
        .insert(Interaction::Pressed);
    warm_up_hub_trade_ui(&mut app);

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

    let app_state = AppState {
        wallet: MoneyCents(600_000),
        last_hub: HubId(1),
        cargo: Cargo {
            capacity_mass_kg: 1_000,
            capacity_volume_l: 1_000,
            ..Default::default()
        },
        ..Default::default()
    };

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
    warm_up_hub_trade_ui(&mut app);

    let target_commodity = {
        let model = app.world().resource::<HubTradeUiModel>();
        model
            .view()
            .and_then(|view| view.commodities.first().cloned())
            .expect("commodity row present")
            .id
    };

    let steppers = collect_stepper_buttons(&mut app);
    let plus_entity = steppers
        .iter()
        .find(|(_, commodity, delta)| *commodity == target_commodity && *delta > 0)
        .map(|(entity, _, _)| *entity)
        .expect("plus button");
    app.world_mut()
        .entity_mut(plus_entity)
        .insert(Interaction::Pressed);
    warm_up_hub_trade_ui(&mut app);

    let trade_buttons = collect_trade_buttons(&mut app);
    let buy_entity = trade_buttons
        .iter()
        .find(|(_, commodity, kind)| {
            *commodity == target_commodity && matches!(kind, TradeKind::Buy)
        })
        .map(|(entity, _, _)| *entity)
        .expect("buy button");
    app.world_mut()
        .entity_mut(buy_entity)
        .insert(Interaction::Pressed);
    warm_up_hub_trade_ui(&mut app);

    {
        let mut queue = app.world_mut().resource_mut::<CommandQueue>();
        queue.begin_tick(1);
    }

    let trade_buttons = collect_trade_buttons(&mut app);

    let sell_entity = trade_buttons
        .iter()
        .find(|(_, commodity, kind)| {
            *commodity == target_commodity && matches!(kind, TradeKind::Sell)
        })
        .map(|(entity, _, _)| *entity)
        .expect("sell button");
    app.world_mut()
        .entity_mut(sell_entity)
        .insert(Interaction::Pressed);
    warm_up_hub_trade_ui(&mut app);

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
