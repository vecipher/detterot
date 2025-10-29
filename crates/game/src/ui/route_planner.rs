use bevy::prelude::*;

use crate::scheduling::sets;
use crate::systems::director::LegContext;
use crate::systems::economy::{EconState, EconomyDay, HubId, Weather};
use crate::systems::trading::TradingViewState;
use crate::ui::hub_trade::ActiveHub;
use crate::world::WorldIndex;

/// Plugin wiring the route planner UI view models into the Bevy app.
#[derive(Default)]
pub struct RoutePlannerUiPlugin;

impl Plugin for RoutePlannerUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RoutePlannerViewModel>().add_systems(
            FixedUpdate,
            update_route_planner_vm.in_set(sets::DETTEROT_Cleanup),
        );
    }
}

/// Aggregated view model for the route planner panel.
#[derive(Resource, Default, Clone, Debug)]
pub struct RoutePlannerViewModel {
    pub is_visible: bool,
    pub forecast_strip: ForecastStripVm,
    pub rumors: RumorListVm,
}

/// Forecast strip rendered as weather icons.
#[derive(Default, Clone, Debug)]
pub struct ForecastStripVm {
    pub icons: Vec<ForecastIconVm>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForecastIconVm {
    pub weather: Weather,
    pub weight: u16,
}

/// Rumour list describing neighbouring hubs.
#[derive(Default, Clone, Debug)]
pub struct RumorListVm {
    pub entries: Vec<RumorVm>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RumorVm {
    pub destination: HubId,
    pub weather: Option<Weather>,
}

fn update_route_planner_vm(
    mut vm: ResMut<RoutePlannerViewModel>,
    world_index: Option<Res<WorldIndex>>,
    active_hub: Option<Res<ActiveHub>>,
    view_state: Option<Res<TradingViewState>>,
    econ_state: Option<Res<EconState>>,
    leg_context: Option<Res<LegContext>>,
) {
    vm.is_visible = view_state
        .map(|state| state.is_route_planner())
        .unwrap_or(false);

    vm.forecast_strip.icons.clear();
    vm.rumors.entries.clear();

    let Some(index) = world_index else {
        return;
    };

    let hub = active_hub.map(|hub| hub.0).unwrap_or(HubId(0));

    vm.forecast_strip
        .icons
        .extend(index.weather_hints(hub).iter().map(|hint| ForecastIconVm {
            weather: hint.weather(),
            weight: hint.weight(),
        }));

    let world_seed = leg_context
        .as_ref()
        .map(|ctx| ctx.world_seed)
        .unwrap_or_default();
    let fallback_day = leg_context
        .as_ref()
        .map(|ctx| EconomyDay(ctx.day))
        .unwrap_or_default();
    let day = econ_state
        .as_ref()
        .map(|state| state.day)
        .unwrap_or(fallback_day);

    for neighbour in index.neighbors(hub) {
        let weather = index.deterministic_rumor(world_seed, hub, *neighbour, day);
        vm.rumors.entries.push(RumorVm {
            destination: *neighbour,
            weather,
        });
    }
}
