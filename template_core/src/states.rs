//! Application-level state machine: menu → loading → in-game, with a pause
//! sub-state that only exists while in-game.

use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[states(scoped_entities)]
pub enum AppState {
    #[default]
    MainMenu,
    Loading,
    InGame,
}

/// Pausing must not despawn the game world, so it is a sub-state of
/// [`AppState::InGame`] rather than a variant of [`AppState`]: entities scoped
/// to `InGame` survive a pause, and leaving `InGame` discards pause state.
#[derive(SubStates, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[source(AppState = AppState::InGame)]
#[states(scoped_entities)]
pub enum PauseState {
    #[default]
    Running,
    Paused,
}

/// Which control scheme drives the game: third-person follow (adventure) or
/// top-down RTS (squad/strategy). Both act on the same world and the same
/// camera entity; systems of each scheme gate on this state. A sub-state of
/// [`AppState::InGame`] so every new game starts in third person and leaving
/// the game discards the mode.
#[derive(SubStates, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[source(AppState = AppState::InGame)]
#[states(scoped_entities)]
pub enum CameraMode {
    #[default]
    ThirdPerson,
    TopDown,
}

pub struct AppStatePlugin;

impl Plugin for AppStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .add_sub_state::<PauseState>()
            .add_sub_state::<CameraMode>()
            .add_systems(Update, toggle_pause.run_if(in_state(AppState::InGame)))
            .add_systems(OnEnter(PauseState::Paused), pause_time)
            .add_systems(OnExit(PauseState::Paused), resume_time)
            .add_systems(OnEnter(AppState::MainMenu), log_entity_count)
            .add_systems(OnEnter(AppState::InGame), log_entity_count);
    }
}

fn toggle_pause(
    input: Res<ButtonInput<KeyCode>>,
    pause: Res<State<PauseState>>,
    mut next: ResMut<NextState<PauseState>>,
) {
    if input.just_pressed(KeyCode::Escape) {
        next.set(match pause.get() {
            PauseState::Running => PauseState::Paused,
            PauseState::Paused => PauseState::Running,
        });
    }
}

/// Freezing virtual time stops everything driven by `Res<Time>`; systems
/// reacting to input rather than time must also gate on
/// `in_state(PauseState::Running)`.
fn pause_time(mut time: ResMut<Time<Virtual>>) {
    time.pause();
}

fn resume_time(mut time: ResMut<Time<Virtual>>) {
    time.unpause();
}

/// Leak detector: the count logged on each `MainMenu` entry must not grow
/// across menu → game → menu round-trips.
fn log_entity_count(state: Res<State<AppState>>, entities: Query<Entity>) {
    info!(
        "entered {:?}: {} live entities",
        state.get(),
        entities.iter().count()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::state::app::StatesPlugin;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, AppStatePlugin));
        app.init_resource::<ButtonInput<KeyCode>>();
        app.update();
        app
    }

    fn app_state(app: &App) -> AppState {
        *app.world().resource::<State<AppState>>().get()
    }

    // Advancing Loading → InGame is asset-driven and owned by the levels
    // module, so these tests enter the game directly.
    fn start_game(app: &mut App) {
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::InGame);
        app.update();
    }

    #[test]
    fn app_starts_in_main_menu() {
        let app = test_app();
        assert_eq!(app_state(&app), AppState::MainMenu);
    }

    #[test]
    fn pausing_freezes_virtual_time_and_resuming_unfreezes() {
        let mut app = test_app();
        start_game(&mut app);

        app.world_mut()
            .resource_mut::<NextState<PauseState>>()
            .set(PauseState::Paused);
        app.update();
        assert!(app.world().resource::<Time<Virtual>>().is_paused());

        app.world_mut()
            .resource_mut::<NextState<PauseState>>()
            .set(PauseState::Running);
        app.update();
        assert!(!app.world().resource::<Time<Virtual>>().is_paused());
    }

    #[test]
    fn quitting_to_menu_while_paused_resets_pause_state() {
        let mut app = test_app();
        start_game(&mut app);

        app.world_mut()
            .resource_mut::<NextState<PauseState>>()
            .set(PauseState::Paused);
        app.update();

        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::MainMenu);
        app.update();
        assert_eq!(app_state(&app), AppState::MainMenu);
        assert!(
            app.world().get_resource::<State<PauseState>>().is_none(),
            "pause sub-state must not exist outside InGame"
        );

        start_game(&mut app);
        assert_eq!(
            *app.world().resource::<State<PauseState>>().get(),
            PauseState::Running,
            "a new game must start unpaused"
        );
    }
}
