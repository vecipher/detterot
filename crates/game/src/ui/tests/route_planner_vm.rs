use std::path::PathBuf;

use bevy::prelude::*;

use crate::scheduling;
use crate::systems::director::LegContext;
use crate::systems::economy::{EconState, EconomyDay, HubId, Rulepack, Weather};
use crate::systems::trading::inventory::Cargo;
use crate::systems::trading::types::{load_commodities, Commodities};
use crate::systems::trading::{
    EnterRoutePlannerViewEvent, EnterTradingViewEvent, TradingPlugin, TradingViewState,
};
use crate::ui::hub_trade::{ActiveHub, HubTradeViewModel};
use crate::ui::route_planner::{RoutePlannerUiPlugin, RoutePlannerViewModel};
use crate::world::WorldIndex;

fn asset_path(relative: &str) -> String {
    let direct = PathBuf::from(relative);
    if direct.exists() {
        return direct
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .into_owned();
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../{relative}"))
        .to_string_lossy()
        .into_owned()
}

fn load_rulepack() -> Rulepack {
    crate::systems::economy::load_rulepack(&asset_path("assets/rulepacks/day_001.toml")).unwrap()
}

fn load_commodities_specs() -> Commodities {
    load_commodities(&asset_path("assets/trading/commodities.toml")).unwrap()
}

fn load_world_index() -> WorldIndex {
    WorldIndex::from_path(asset_path("assets/world/hubs_min.toml")).unwrap()
}

#[test]
fn planner_vm_reflects_world_index() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    scheduling::configure(&mut app);
    app.add_plugins(RoutePlannerUiPlugin);

    app.insert_resource(load_world_index());
    app.insert_resource(EconState {
        day: EconomyDay(5),
        ..Default::default()
    });
    app.insert_resource(ActiveHub(HubId(0)));
    app.insert_resource(LegContext {
        world_seed: 123,
        ..Default::default()
    });

    app.update();
    app.world_mut().run_schedule(FixedUpdate);

    let index = app.world().resource::<WorldIndex>().clone();
    let vm = app.world().resource::<RoutePlannerViewModel>().clone();

    assert!(!vm.is_visible);
    assert_eq!(vm.forecast_strip.icons.len(), 2);
    assert_eq!(vm.forecast_strip.icons[0].weather, Weather::Clear);
    assert_eq!(vm.forecast_strip.icons[0].weight, 3);
    assert_eq!(vm.forecast_strip.icons[1].weather, Weather::Rains);
    assert_eq!(vm.forecast_strip.icons[1].weight, 1);

    assert_eq!(vm.rumors.entries.len(), 2);
    let first = &vm.rumors.entries[0];
    assert_eq!(first.destination, HubId(1));
    assert_eq!(
        first.weather,
        index.deterministic_rumor(123, HubId(0), HubId(1), EconomyDay(5))
    );
    let second = &vm.rumors.entries[1];
    assert_eq!(second.destination, HubId(2));
    assert_eq!(
        second.weather,
        index.deterministic_rumor(123, HubId(0), HubId(2), EconomyDay(5))
    );
}

#[test]
fn trading_view_transitions_toggle_visibility() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    scheduling::configure(&mut app);

    app.insert_resource(load_rulepack());
    app.insert_resource(load_commodities_specs());
    app.insert_resource(load_world_index());
    app.insert_resource(EconState::default());
    app.insert_resource(Cargo::default());
    app.insert_resource(LegContext {
        world_seed: 77,
        ..Default::default()
    });

    app.add_plugins(TradingPlugin);

    app.insert_resource(ActiveHub(HubId(0)));

    app.update();
    app.world_mut().run_schedule(FixedUpdate);

    {
        let state = app.world().resource::<TradingViewState>();
        assert!(state.is_trading());
        let trade_vm = app.world().resource::<HubTradeViewModel>();
        assert!(trade_vm.is_visible);
        let planner_vm = app.world().resource::<RoutePlannerViewModel>();
        assert!(!planner_vm.is_visible);
    }

    {
        let world = app.world_mut();
        world
            .resource_mut::<Messages<EnterRoutePlannerViewEvent>>()
            .write(EnterRoutePlannerViewEvent);
    }
    app.world_mut().run_schedule(FixedUpdate);

    {
        let state = app.world().resource::<TradingViewState>();
        assert!(state.is_route_planner());
        let trade_vm = app.world().resource::<HubTradeViewModel>();
        assert!(!trade_vm.is_visible);
        let planner_vm = app.world().resource::<RoutePlannerViewModel>();
        assert!(planner_vm.is_visible);
    }

    {
        let world = app.world_mut();
        world
            .resource_mut::<Messages<EnterTradingViewEvent>>()
            .write(EnterTradingViewEvent);
    }
    app.world_mut().run_schedule(FixedUpdate);

    {
        let state = app.world().resource::<TradingViewState>();
        assert!(state.is_trading());
        let trade_vm = app.world().resource::<HubTradeViewModel>();
        assert!(trade_vm.is_visible);
        let planner_vm = app.world().resource::<RoutePlannerViewModel>();
        assert!(!planner_vm.is_visible);
    }
}
