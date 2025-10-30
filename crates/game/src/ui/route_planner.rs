use bevy::prelude::*;
use bevy::text::{Font, TextColor, TextFont};
use smallvec::SmallVec;

use crate::app_state::AppState;
use crate::systems::economy::{HubId, RouteId, Weather};
use crate::ui::styles::{
    COLOR_ACCENT_NEG, COLOR_ACCENT_POS, COLOR_BG, COLOR_TEXT_PRIMARY, COLOR_TEXT_SECONDARY,
};
use crate::world::index::{deterministic_rumor, RumorKind, StaticWorldIndex, WorldIndex};

#[derive(Resource, Default)]
pub struct RoutePlannerState {
    pub last_forecast: Vec<RouteForecast>,
}

pub struct RoutePlannerPlugin;

impl Plugin for RoutePlannerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RoutePlannerState>()
            .add_systems(Startup, spawn_route_planner_panel)
            .add_systems(Update, sync_route_planner_ui);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Component)]
struct RoutePlannerRoot;

#[derive(Component)]
struct RoutePlannerRows;

#[derive(Component, Clone, Copy)]
#[allow(dead_code)]
struct ForecastRow {
    route: RouteId,
}

#[derive(Component)]
struct RouteLabel;

#[derive(Component)]
struct WeatherLabel;

#[derive(Component)]
struct RumorLabel;

fn spawn_route_planner_panel(
    mut commands: Commands,
    asset_server: Option<Res<AssetServer>>,
    existing: Query<Entity, With<RoutePlannerRoot>>,
) {
    if existing.iter().next().is_some() {
        return;
    }

    let asset_server = asset_server.as_ref().map(|server| server.as_ref());
    let title_font = TextFont {
        font: load_font(asset_server, "fonts/inter-semibold.ttf"),
        font_size: 18.0,
        ..default()
    };
    let title_color = TextColor(COLOR_TEXT_PRIMARY);

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(24.0),
                top: Val::Px(24.0),
                padding: UiRect::axes(Val::Px(16.0), Val::Px(12.0)),
                row_gap: Val::Px(12.0),
                flex_direction: FlexDirection::Column,
                min_width: Val::Px(240.0),
                ..default()
            },
            BackgroundColor(COLOR_BG),
            BorderRadius::all(Val::Px(12.0)),
            RoutePlannerRoot,
            Name::new("RoutePlannerPanel"),
        ))
        .with_children(|parent| {
            parent.spawn((Text::new("Route Planner"), title_font, title_color));
            parent.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    ..default()
                },
                RoutePlannerRows,
            ));
        });
}

fn sync_route_planner_ui(
    mut commands: Commands,
    app_state: Option<Res<AppState>>,
    planner_state: Option<ResMut<RoutePlannerState>>,
    rows: Query<(Entity, &Children), With<RoutePlannerRows>>,
    hierarchy: Query<&Children>,
    asset_server: Option<Res<AssetServer>>,
) {
    let Some(app_state) = app_state else {
        return;
    };
    let Some(mut planner_state) = planner_state else {
        return;
    };
    let Some((rows_entity, child_entities)) = rows
        .iter()
        .next()
        .map(|(entity, children)| (entity, children.iter().collect::<Vec<Entity>>()))
    else {
        return;
    };

    if !app_state.is_changed() && !planner_state.last_forecast.is_empty() {
        return;
    }

    let forecast = build_forecast(app_state.world_seed, app_state.last_hub);
    if planner_state.last_forecast == forecast {
        return;
    }
    planner_state.last_forecast = forecast.clone();

    if !child_entities.is_empty() {
        let mut to_remove = Vec::new();
        for entity in &child_entities {
            collect_descendants(*entity, &hierarchy, &mut to_remove);
            to_remove.push(*entity);
        }
        for entity in to_remove {
            commands.entity(entity).despawn();
        }
    }

    let asset_server = asset_server.as_ref().map(|server| server.as_ref());
    let body_font = TextFont {
        font: load_font(asset_server, "fonts/inter-regular.ttf"),
        font_size: 14.0,
        ..default()
    };

    commands.entity(rows_entity).with_children(|parent| {
        if forecast.is_empty() {
            parent.spawn((
                Text::new("No routes available"),
                body_font.clone(),
                TextColor(COLOR_TEXT_SECONDARY),
            ));
            return;
        }

        for entry in &forecast {
            let route = entry.route;
            let weather = entry.weather;
            let (rumor_kind, confidence) = entry.rumor;
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(12.0),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    ForecastRow { route },
                    Name::new(format!("RouteRow_{}", route.0)),
                ))
                .with_children(|row| {
                    row.spawn((
                        Text::new(route_label(route)),
                        body_font.clone(),
                        TextColor(COLOR_TEXT_PRIMARY),
                        RouteLabel,
                    ));
                    row.spawn((
                        Text::new(weather_display(weather)),
                        body_font.clone(),
                        TextColor(COLOR_TEXT_SECONDARY),
                        WeatherLabel,
                    ));
                    row.spawn((
                        Text::new(rumor_display(rumor_kind, confidence)),
                        body_font.clone(),
                        TextColor(rumor_color(rumor_kind)),
                        RumorLabel,
                    ));
                });
        }
    });
}

fn route_label(route: RouteId) -> String {
    format!("Route {}", route.0)
}

fn weather_display(weather: Weather) -> String {
    format!("{} {}", weather_icon(weather), weather_name(weather))
}

fn rumor_display(kind: RumorKind, confidence: u8) -> String {
    format!("{} {} {confidence}%", rumor_icon(kind), rumor_name(kind))
}

fn weather_icon(weather: Weather) -> &'static str {
    match weather {
        Weather::Clear => "☀",
        Weather::Rains => "🌧",
        Weather::Fog => "🌫",
        Weather::Windy => "💨",
    }
}

fn weather_name(weather: Weather) -> &'static str {
    match weather {
        Weather::Clear => "Clear",
        Weather::Rains => "Rains",
        Weather::Fog => "Fog",
        Weather::Windy => "Windy",
    }
}

fn rumor_icon(kind: RumorKind) -> &'static str {
    match kind {
        RumorKind::Wind => "🌀",
        RumorKind::Fog => "🌁",
        RumorKind::Patrol => "🚨",
    }
}

fn rumor_name(kind: RumorKind) -> &'static str {
    match kind {
        RumorKind::Wind => "Wind",
        RumorKind::Fog => "Fog",
        RumorKind::Patrol => "Patrol",
    }
}

fn rumor_color(kind: RumorKind) -> Color {
    match kind {
        RumorKind::Patrol => COLOR_ACCENT_NEG,
        RumorKind::Wind | RumorKind::Fog => COLOR_ACCENT_POS,
    }
}

fn collect_descendants(entity: Entity, hierarchy: &Query<&Children>, buffer: &mut Vec<Entity>) {
    if let Ok(children) = hierarchy.get(entity) {
        for child in children.iter() {
            collect_descendants(child, hierarchy, buffer);
            buffer.push(child);
        }
    }
}

fn load_font(asset_server: Option<&AssetServer>, path: &'static str) -> Handle<Font> {
    asset_server
        .map(|server| server.load(path))
        .unwrap_or_else(Handle::default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::app::App;
    use bevy::prelude::{Children, Text};
    use bevy::MinimalPlugins;

    #[test]
    fn planner_ui_matches_forecast_for_fixed_seed() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        app.init_resource::<AppState>();
        {
            let mut state = app.world_mut().resource_mut::<AppState>();
            state.world_seed = 0xDEADBEEFCAFEBABE;
            state.last_hub = HubId(7);
        }

        app.add_plugins(RoutePlannerPlugin);

        app.update();
        app.update();

        let state = app.world().resource::<AppState>().clone();
        let expected = build_forecast(state.world_seed, state.last_hub);

        let last_forecast = {
            let planner_state = app.world().resource::<RoutePlannerState>();
            planner_state.last_forecast.clone()
        };
        assert_eq!(last_forecast, expected);

        let mut rows: Vec<(RouteId, Vec<String>)> = Vec::new();
        {
            let world = app.world_mut();
            let mut row_query = world.query::<(&ForecastRow, &Children)>();
            let mut text_query = world.query::<&Text>();
            for (row, children) in row_query.iter(world) {
                let mut labels = Vec::new();
                for child in children.iter() {
                    if let Ok(text) = text_query.get(world, child) {
                        labels.push(text.0.clone());
                    }
                }
                rows.push((row.route, labels));
            }
        }

        rows.sort_by_key(|(route, _)| route.0);
        let mut expected_sorted = expected.clone();
        expected_sorted.sort_by_key(|entry| entry.route.0);

        assert_eq!(rows.len(), expected_sorted.len());
        for (actual, target) in rows.iter().zip(expected_sorted.iter()) {
            let (_, labels) = actual;
            assert_eq!(labels.len(), 3);
            assert_eq!(labels[0], route_label(target.route));
            assert_eq!(labels[1], weather_display(target.weather));
            assert_eq!(labels[2], rumor_display(target.rumor.0, target.rumor.1));
        }
    }
}
