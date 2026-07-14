//! Main menu, pause menu, and the settings screen, sharing one button widget
//! and per-concern action handlers. The settings screen edits [`GameSettings`]
//! (the settings plugin applies and persists it) and rebinds keys through the
//! step 3 input layer.

use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::controls::PlayerAction;
use crate::player::Player;
use crate::saves::{LoadGame, SaveGame};
use crate::settings::{GameSettings, WindowModeSetting, bound_key, rebind_key};
use crate::states::{AppState, PauseState};

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RebindTarget>()
            .add_systems(OnEnter(AppState::MainMenu), spawn_main_menu)
            .add_systems(OnEnter(PauseState::Paused), spawn_pause_menu)
            .add_systems(
                Update,
                (
                    style_buttons,
                    handle_menu_actions,
                    handle_settings_actions,
                    capture_rebind,
                ),
            )
            .add_systems(
                Update,
                refresh_settings_labels.run_if(
                    resource_exists_and_changed::<GameSettings>
                        .or_else(resource_changed::<RebindTarget>),
                ),
            );
    }
}

/// What a menu button does when clicked.
#[derive(Component, Clone, Copy, Debug)]
enum MenuAction {
    NewGame,
    LoadGame,
    SaveGame,
    Resume,
    QuitToMenu,
    QuitToDesktop,
}

/// What a settings-screen button does when clicked ([`SettingsAction::Open`]
/// sits on the main/pause menus' Settings button).
#[derive(Component, Clone, Copy, Debug)]
enum SettingsAction {
    Open,
    Close,
    CycleWindowMode,
    CycleResolution,
    VolumeDown,
    VolumeUp,
    Rebind(PlayerAction),
}

/// The settings screen root (at most one exists).
#[derive(Component)]
struct SettingsScreen;

/// Marks a text node showing a live settings value.
#[derive(Component, Clone, Copy, PartialEq)]
enum SettingsLabel {
    WindowMode,
    Resolution,
    Volume,
    Key(PlayerAction),
}

/// The action whose keyboard binding the next key press replaces.
#[derive(Resource, Default)]
struct RebindTarget(Option<PlayerAction>);

/// Windowed-mode resolutions the settings menu cycles through.
const RESOLUTIONS: [(f32, f32); 4] = [
    (1280.0, 720.0),
    (1600.0, 900.0),
    (1920.0, 1080.0),
    (2560.0, 1440.0),
];

/// Actions the settings menu offers keyboard rebinding for (buttons only;
/// axes and mouse bindings are data in the input map, not rebind UI).
const REBINDABLE: [(PlayerAction, &str); 3] = [
    (PlayerAction::Run, "Run"),
    (PlayerAction::StopHold, "Stop/Hold"),
    (PlayerAction::ToggleCameraMode, "Camera Mode"),
];

const BUTTON_NORMAL: Color = Color::srgb(0.15, 0.15, 0.15);
const BUTTON_HOVERED: Color = Color::srgb(0.25, 0.25, 0.25);
const BUTTON_PRESSED: Color = Color::srgb(0.35, 0.55, 0.35);
const TEXT_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);

fn spawn_main_menu(mut commands: Commands) {
    // The menu has no game world to borrow a camera from.
    commands.spawn((Camera2d, DespawnOnExit(AppState::MainMenu)));
    commands.spawn((
        DespawnOnExit(AppState::MainMenu),
        menu_root(),
        BackgroundColor(Color::srgb(0.10, 0.10, 0.12)),
        children![
            title("3D Template"),
            button("New Game", MenuAction::NewGame),
            button("Load Game", MenuAction::LoadGame),
            button("Settings", SettingsAction::Open),
            button("Quit", MenuAction::QuitToDesktop),
        ],
    ));
}

fn spawn_pause_menu(mut commands: Commands) {
    // Rendered by the in-game camera; the translucent backdrop keeps the
    // frozen game world visible.
    commands.spawn((
        DespawnOnExit(PauseState::Paused),
        menu_root(),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
        children![
            title("Paused"),
            button("Resume", MenuAction::Resume),
            button("Save", MenuAction::SaveGame),
            button("Settings", SettingsAction::Open),
            button("Quit to Menu", MenuAction::QuitToMenu),
        ],
    ));
}

fn menu_root() -> Node {
    Node {
        width: percent(100),
        height: percent(100),
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        row_gap: px(12),
        ..default()
    }
}

fn title(text: &str) -> impl Bundle {
    (
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(48.0),
            ..default()
        },
        TextColor(TEXT_COLOR),
        Node {
            margin: UiRect::bottom(px(24)),
            ..default()
        },
    )
}

fn button(label: &str, action: impl Bundle) -> impl Bundle {
    (
        Button,
        action,
        Node {
            width: px(220),
            height: px(56),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(BUTTON_NORMAL),
        children![(
            Text::new(label),
            TextFont {
                font_size: FontSize::Px(24.0),
                ..default()
            },
            TextColor(TEXT_COLOR),
        )],
    )
}

fn style_buttons(
    mut buttons: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<Button>)>,
) {
    for (interaction, mut color) in &mut buttons {
        *color = match interaction {
            Interaction::Pressed => BUTTON_PRESSED.into(),
            Interaction::Hovered => BUTTON_HOVERED.into(),
            Interaction::None => BUTTON_NORMAL.into(),
        };
    }
}

fn handle_menu_actions(
    buttons: Query<(&Interaction, &MenuAction), Changed<Interaction>>,
    mut next_app: ResMut<NextState<AppState>>,
    // The pause sub-state's resources only exist while in-game.
    mut next_pause: Option<ResMut<NextState<PauseState>>>,
    mut app_exit: MessageWriter<AppExit>,
    mut save_requests: MessageWriter<SaveGame>,
    mut load_requests: MessageWriter<LoadGame>,
) {
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            MenuAction::NewGame => next_app.set(AppState::Loading),
            MenuAction::LoadGame => {
                load_requests.write(LoadGame);
            }
            MenuAction::SaveGame => {
                save_requests.write(SaveGame);
            }
            MenuAction::Resume => {
                if let Some(next) = next_pause.as_mut() {
                    next.set(PauseState::Running);
                }
            }
            MenuAction::QuitToMenu => next_app.set(AppState::MainMenu),
            MenuAction::QuitToDesktop => {
                app_exit.write(AppExit::Success);
            }
        }
    }
}

/// Settings-screen clicks mutate [`GameSettings`]; the settings plugin
/// applies them live and writes the file. Absent `GameSettings` (a game
/// composed without `SettingsPlugin`) turns the Settings button into a no-op.
#[allow(clippy::too_many_arguments)]
fn handle_settings_actions(
    mut commands: Commands,
    buttons: Query<(&Interaction, &SettingsAction), Changed<Interaction>>,
    mut settings: Option<ResMut<GameSettings>>,
    mut rebind: ResMut<RebindTarget>,
    app_state: Res<State<AppState>>,
    screens: Query<Entity, With<SettingsScreen>>,
) {
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let Some(settings) = settings.as_deref_mut() else {
            warn!("settings button clicked but SettingsPlugin is not installed");
            return;
        };
        match action {
            SettingsAction::Open => {
                if screens.is_empty() {
                    spawn_settings_screen(&mut commands, *app_state.get(), settings);
                }
            }
            SettingsAction::Close => {
                rebind.0 = None;
                for screen in &screens {
                    commands.entity(screen).despawn();
                }
            }
            SettingsAction::CycleWindowMode => {
                settings.window_mode = match settings.window_mode {
                    WindowModeSetting::Windowed => WindowModeSetting::BorderlessFullscreen,
                    WindowModeSetting::BorderlessFullscreen => WindowModeSetting::Windowed,
                };
            }
            SettingsAction::CycleResolution => {
                // A file-edited custom resolution simply cycles back to preset 0.
                let next = RESOLUTIONS
                    .iter()
                    .position(|&preset| preset == settings.resolution)
                    .map_or(0, |index| (index + 1) % RESOLUTIONS.len());
                settings.resolution = RESOLUTIONS[next];
            }
            SettingsAction::VolumeDown | SettingsAction::VolumeUp => {
                let step = if matches!(action, SettingsAction::VolumeUp) {
                    1.0
                } else {
                    -1.0
                };
                // Work in tenths so repeated steps can't accumulate float drift.
                let tenths = (settings.master_volume * 10.0).round() + step;
                settings.master_volume = (tenths / 10.0).clamp(0.0, 1.0);
            }
            SettingsAction::Rebind(action) => rebind.0 = Some(*action),
        }
    }
}

/// While a rebind is armed, the next key press replaces that action's
/// keyboard binding — in the settings file and on the live player, so the
/// change applies immediately. Escape cancels.
fn capture_rebind(
    mut rebind: ResMut<RebindTarget>,
    keys: Res<ButtonInput<KeyCode>>,
    mut settings: Option<ResMut<GameSettings>>,
    mut players: Query<&mut InputMap<PlayerAction>, With<Player>>,
) {
    let Some(action) = rebind.0 else {
        return;
    };
    let Some(&key) = keys.get_just_pressed().next() else {
        return;
    };
    rebind.0 = None;
    if key == KeyCode::Escape {
        return;
    }
    let Some(settings) = settings.as_deref_mut() else {
        return;
    };
    info!("rebound {action:?} to {key:?}");
    rebind_key(&mut settings.input_map, action, key);
    for mut map in &mut players {
        rebind_key(&mut map, action, key);
    }
}

/// Rewrites the value labels whenever the settings (or an armed rebind)
/// change; the labels' initial text is filled at spawn.
fn refresh_settings_labels(
    settings: Option<Res<GameSettings>>,
    rebind: Res<RebindTarget>,
    mut labels: Query<(&SettingsLabel, &mut Text)>,
) {
    let Some(settings) = settings else {
        return;
    };
    for (label, mut text) in &mut labels {
        text.0 = label_text(*label, &settings, rebind.0);
    }
}

fn label_text(
    label: SettingsLabel,
    settings: &GameSettings,
    rebinding: Option<PlayerAction>,
) -> String {
    match label {
        SettingsLabel::WindowMode => match settings.window_mode {
            WindowModeSetting::Windowed => "Window: Windowed".into(),
            WindowModeSetting::BorderlessFullscreen => "Window: Borderless".into(),
        },
        SettingsLabel::Resolution => format!(
            "Resolution: {}×{}",
            settings.resolution.0 as u32, settings.resolution.1 as u32
        ),
        SettingsLabel::Volume => {
            format!("Volume: {:.0}%", settings.master_volume * 100.0)
        }
        SettingsLabel::Key(action) => {
            let name = REBINDABLE
                .iter()
                .find(|(a, _)| *a == action)
                .map_or("?", |(_, name)| name);
            if rebinding == Some(action) {
                format!("{name}: press a key…")
            } else {
                match bound_key(&settings.input_map, &action) {
                    Some(key) => format!("{name}: {key:?}"),
                    None => format!("{name}: unbound"),
                }
            }
        }
    }
}

/// The settings screen overlays whichever menu opened it and despawns with
/// that menu's state (main menu, or the pause sub-state in-game).
fn spawn_settings_screen(commands: &mut Commands, app_state: AppState, settings: &GameSettings) {
    let mut root = menu_root();
    root.position_type = PositionType::Absolute;
    let text = |label: SettingsLabel| label_text(label, settings, None);
    let mut screen = commands.spawn((
        SettingsScreen,
        root,
        BackgroundColor(Color::srgb(0.10, 0.10, 0.12)),
        GlobalZIndex(5),
        children![
            title("Settings"),
            settings_button(
                text(SettingsLabel::WindowMode),
                SettingsAction::CycleWindowMode,
                SettingsLabel::WindowMode,
            ),
            settings_button(
                text(SettingsLabel::Resolution),
                SettingsAction::CycleResolution,
                SettingsLabel::Resolution,
            ),
            volume_row(text(SettingsLabel::Volume)),
            settings_button(
                text(SettingsLabel::Key(PlayerAction::Run)),
                SettingsAction::Rebind(PlayerAction::Run),
                SettingsLabel::Key(PlayerAction::Run),
            ),
            settings_button(
                text(SettingsLabel::Key(PlayerAction::StopHold)),
                SettingsAction::Rebind(PlayerAction::StopHold),
                SettingsLabel::Key(PlayerAction::StopHold),
            ),
            settings_button(
                text(SettingsLabel::Key(PlayerAction::ToggleCameraMode)),
                SettingsAction::Rebind(PlayerAction::ToggleCameraMode),
                SettingsLabel::Key(PlayerAction::ToggleCameraMode),
            ),
            button("Back", SettingsAction::Close),
        ],
    ));
    match app_state {
        AppState::MainMenu => screen.insert(DespawnOnExit(AppState::MainMenu)),
        _ => screen.insert(DespawnOnExit(PauseState::Paused)),
    };
}

/// A menu button whose label is a live settings value.
fn settings_button(initial: String, action: SettingsAction, label: SettingsLabel) -> impl Bundle {
    (
        Button,
        action,
        Node {
            width: px(340),
            height: px(48),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(BUTTON_NORMAL),
        children![(
            Text::new(initial),
            TextFont {
                font_size: FontSize::Px(20.0),
                ..default()
            },
            TextColor(TEXT_COLOR),
            label,
        )],
    )
}

/// `[-] Volume: N% [+]` — the label sits between two step buttons.
fn volume_row(initial: String) -> impl Bundle {
    let step_button = |glyph: &str, action: SettingsAction| {
        (
            Button,
            action,
            Node {
                width: px(48),
                height: px(48),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BUTTON_NORMAL),
            children![(
                Text::new(glyph),
                TextFont {
                    font_size: FontSize::Px(20.0),
                    ..default()
                },
                TextColor(TEXT_COLOR),
            )],
        )
    };
    (
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(8),
            ..default()
        },
        children![
            step_button("-", SettingsAction::VolumeDown),
            (
                Text::new(initial),
                TextFont {
                    font_size: FontSize::Px(20.0),
                    ..default()
                },
                TextColor(TEXT_COLOR),
                SettingsLabel::Volume,
                Node {
                    width: px(228),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
            ),
            step_button("+", SettingsAction::VolumeUp),
        ],
    )
}
