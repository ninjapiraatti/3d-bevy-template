//! Main menu and pause menu, sharing one button widget and action handler.

use bevy::prelude::*;

use crate::states::{AppState, PauseState};

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::MainMenu), spawn_main_menu)
            .add_systems(OnEnter(PauseState::Paused), spawn_pause_menu)
            .add_systems(Update, (style_buttons, handle_menu_actions));
    }
}

/// What a menu button does when clicked.
#[derive(Component, Clone, Copy, Debug)]
enum MenuAction {
    NewGame,
    LoadGame,
    Settings,
    Resume,
    QuitToMenu,
    QuitToDesktop,
}

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
            button("Settings", MenuAction::Settings),
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
            button("Settings", MenuAction::Settings),
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

fn button(label: &str, action: MenuAction) -> impl Bundle {
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
) {
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            MenuAction::NewGame => next_app.set(AppState::Loading),
            MenuAction::LoadGame => info!("Load Game: not implemented until roadmap step 5"),
            MenuAction::Settings => info!("Settings: not implemented until roadmap step 9"),
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
