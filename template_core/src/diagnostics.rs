//! Diagnostics overlay: FPS and live entity count in the top-left corner,
//! toggled with F1 (a debug-tooling key outside the player's input map,
//! like the F3 navmesh overlay).

use bevy::diagnostic::{
    DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin,
};
use bevy::prelude::*;

pub struct DiagnosticsOverlayPlugin;

#[derive(Component)]
struct OverlayText;

/// Refresh four times a second; per-frame text rebuilds are churn without
/// added information.
const REFRESH_SECS: f32 = 0.25;

impl Plugin for DiagnosticsOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            EntityCountDiagnosticsPlugin::default(),
        ))
        .add_systems(Startup, spawn_overlay)
        .add_systems(Update, (toggle_overlay, refresh_overlay));
    }
}

fn spawn_overlay(mut commands: Commands) {
    commands.spawn((
        Name::new("DiagnosticsOverlay"),
        OverlayText,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(16.0),
            ..default()
        },
        TextColor(Color::srgb(0.6, 1.0, 0.6)),
        Node {
            position_type: PositionType::Absolute,
            left: px(8),
            top: px(8),
            ..default()
        },
        GlobalZIndex(100),
        Visibility::Hidden,
    ));
}

fn toggle_overlay(
    input: Res<ButtonInput<KeyCode>>,
    mut overlays: Query<&mut Visibility, With<OverlayText>>,
) {
    if !input.just_pressed(KeyCode::F1) {
        return;
    }
    for mut visibility in &mut overlays {
        *visibility = match *visibility {
            Visibility::Hidden => Visibility::Visible,
            _ => Visibility::Hidden,
        };
    }
}

fn refresh_overlay(
    time: Res<Time<Real>>,
    diagnostics: Res<DiagnosticsStore>,
    mut overlays: Query<(&mut Text, &Visibility), With<OverlayText>>,
    mut last_refresh: Local<f32>,
) {
    let now = time.elapsed_secs();
    if now - *last_refresh < REFRESH_SECS {
        return;
    }
    *last_refresh = now;

    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|fps| fps.smoothed());
    let entities = diagnostics
        .get(&EntityCountDiagnosticsPlugin::ENTITY_COUNT)
        .and_then(|count| count.value());
    for (mut text, visibility) in &mut overlays {
        if *visibility == Visibility::Hidden {
            continue;
        }
        text.0 = format!(
            "{} fps\n{} entities",
            fps.map_or("--".into(), |fps| format!("{fps:.0}")),
            entities.map_or("--".into(), |count| format!("{count:.0}")),
        );
    }
}
