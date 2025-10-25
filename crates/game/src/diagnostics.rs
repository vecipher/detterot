use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::text::{TextColor, TextFont};

pub struct DiagnosticsUiPlugin;
impl Plugin for DiagnosticsUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .add_systems(Startup, setup_ui)
            .add_systems(Update, update_ui);
    }
}

#[derive(Component)]
struct FpsText;
#[derive(Component)]
struct MsText;
#[derive(Component)]
struct EntText;

fn setup_ui(mut cmds: Commands, asset_server: Res<AssetServer>) {
    let font_handle = asset_server.load("fonts/inter-regular.ttf");
    cmds.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(12.0),
            top: Val::Px(12.0),
            padding: UiRect::all(Val::Px(8.0)),
            row_gap: Val::Px(4.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.06, 0.08, 0.7)),
    ))
    .with_children(|p| {
        let font = TextFont {
            font: font_handle.clone(),
            font_size: 14.0,
            ..default()
        };
        let color = TextColor(Color::WHITE);
        p.spawn((Text::new("FPS: --"), font.clone(), color, FpsText));
        p.spawn((Text::new("CPU ms: --"), font.clone(), color, MsText));
        p.spawn((Text::new("Entities: --"), font, color, EntText));
    });
}

fn update_ui(
    diagnostics: Res<DiagnosticsStore>,
    q_count: Query<Entity>,
    mut readouts: Query<(
        &mut Text,
        Option<&FpsText>,
        Option<&MsText>,
        Option<&EntText>,
    )>,
) {
    let fps_label = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .map(|avg| format!("FPS: {:.1}", avg));
    let ms_label = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|d| d.smoothed())
        .map(|avg| format!("CPU ms: {:.2}", avg * 1000.0));
    let ent_label = format!("Entities: {}", q_count.iter().count());

    for (mut text, is_fps, is_ms, is_ent) in &mut readouts {
        if is_fps.is_some() {
            if let Some(label) = fps_label.as_ref() {
                text.0 = label.clone();
            }
            continue;
        }
        if is_ms.is_some() {
            if let Some(label) = ms_label.as_ref() {
                text.0 = label.clone();
            }
            continue;
        }
        if is_ent.is_some() {
            text.0 = ent_label.clone();
        }
    }
}
