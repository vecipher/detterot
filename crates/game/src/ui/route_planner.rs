use bevy::prelude::*;
use smallvec::SmallVec;

use crate::systems::economy::{HubId, RouteId, Weather};
use crate::world::index::{deterministic_rumor, RumorKind, StaticWorldIndex, WorldIndex};

#[derive(Resource, Default)]
pub struct RoutePlannerState {
    pub last_forecast: Vec<RouteForecast>,
}

pub struct RoutePlannerPlugin;

impl Plugin for RoutePlannerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RoutePlannerState>();
    }
}

#[derive(Debug, Clone)]
pub struct RouteForecast {
    pub route: RouteId,
    pub weather: Weather,
    pub rumor: (RumorKind, u8),
}

pub fn build_forecast(seed: u64, hub: HubId) -> Vec<RouteForecast> {
    let mut neighbors: SmallVec<[RouteId; 6]> = StaticWorldIndex::neighbors(hub);
    neighbors.sort_by_key(|route| route.0);
    neighbors
        .into_iter()
        .map(|route| RouteForecast {
            route,
            weather: StaticWorldIndex::route_weather(route),
            rumor: deterministic_rumor(seed, route),
        })
        .collect()
}
