//! Save/load on moonshine-save, with versioned, timestamped save files.
//!
//! Persistence convention:
//! - An entity persists if it carries moonshine's [`Save`] marker. Persistent
//!   marker components declare `#[require(Save)]`, so entities loaded from a
//!   file re-mark themselves (`Save` itself is never serialized).
//! - A component persists only if it is listed in [`persisted_components`].
//!   The allowlist keeps runtime state (physics, input, render handles) out
//!   of save files; the plugin owning that state rebuilds ("hydrates") it on
//!   any marked entity missing it, which makes fresh spawns and loaded saves
//!   share one code path.
//! - Every file records [`SaveVersion`] so a future format change can migrate
//!   old files in [`check_version`] instead of rejecting them. Files older
//!   than the current version load fine as long as types only get *added*:
//!   absent components/resources simply stay absent and hydration fills the
//!   gaps.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use bevy_world_serialization::WorldFilter;
use moonshine_save::prelude::*;

use crate::camera_rig::ThirdPersonCamera;
use crate::dev_scene::Spinner;
use crate::levels::CurrentLevel;
use crate::player::Player;
use crate::states::AppState;

pub struct SavePlugin;

/// Bump when the save format changes incompatibly; migrate in [`check_version`].
pub const SAVE_VERSION: u32 = 1;

const SAVE_DIR: &str = "saves";

/// Format version stamped into every save file (and read back from it).
#[derive(Resource, Reflect, Debug, Clone, Copy, PartialEq, Eq)]
#[reflect(Resource)]
pub struct SaveVersion(pub u32);

impl Default for SaveVersion {
    fn default() -> Self {
        Self(SAVE_VERSION)
    }
}

/// Request writing a new timestamped save file (pause menu's Save button).
#[derive(Message)]
pub struct SaveGame;

/// Request loading the most recent save file (main menu's Load Game button).
#[derive(Message)]
pub struct LoadGame;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<SaveVersion>()
            .init_resource::<SaveVersion>()
            .add_message::<SaveGame>()
            .add_message::<LoadGame>()
            .add_observer(save_on_default_event)
            .add_observer(load_on_default_event)
            .add_observer(check_version)
            .add_systems(Update, (save_game, load_game))
            // Safety net: saved entities must never outlive a trip through the
            // menu (hydration only tags entities DespawnOnExit once in-game).
            .add_systems(OnEnter(AppState::MainMenu), despawn_saved_entities);
    }
}

/// The only component types that enter save files. Add a type here (it must
/// be `Reflect` + `#[reflect(Component)]` + registered) to persist it;
/// everything else is rebuilt by its owning plugin after load.
fn persisted_components() -> WorldFilter {
    WorldFilter::deny_all()
        .allow::<Name>()
        .allow::<Transform>()
        .allow::<Player>()
        .allow::<ThirdPersonCamera>()
        .allow::<Spinner>()
}

fn save_game(mut requests: MessageReader<SaveGame>, mut commands: Commands) {
    if requests.read().count() == 0 {
        return;
    }
    if let Err(err) = std::fs::create_dir_all(SAVE_DIR) {
        error!("cannot create {SAVE_DIR}/: {err}");
        return;
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before 1970")
        .as_secs();
    let path = format!("{SAVE_DIR}/save-{stamp}.ron");
    let mut event = SaveWorld::default_into_file(&path)
        .include_resource::<SaveVersion>()
        .include_resource::<CurrentLevel>();
    event.components = persisted_components();
    commands.trigger_save(event);
    info!("saved game to {path}");
}

fn load_game(
    mut requests: MessageReader<LoadGame>,
    mut commands: Commands,
    mut next: ResMut<NextState<AppState>>,
) {
    if requests.read().count() == 0 {
        return;
    }
    let Some(path) = latest_save() else {
        warn!("Load Game: no save files in {SAVE_DIR}/");
        return;
    };
    info!("loading {}", path.display());
    // The load applies during this frame's command flush — including the
    // saved CurrentLevel — before the Loading state reads it.
    commands.trigger_load(LoadWorld::default_from_file(path));
    next.set(AppState::Loading);
}

fn latest_save() -> Option<PathBuf> {
    let files = std::fs::read_dir(SAVE_DIR).ok()?;
    files
        .filter_map(|entry| Some(entry.ok()?.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "ron"))
        .max_by_key(|path| path.metadata().and_then(|meta| meta.modified()).ok())
}

/// Runs right after a file has been applied to the world. The migration
/// match on old versions goes here when `SAVE_VERSION` first grows.
fn check_version(_: On<Loaded>, mut version: ResMut<SaveVersion>) {
    match version.0 {
        SAVE_VERSION => info!("loaded save (version {SAVE_VERSION})"),
        older if older < SAVE_VERSION => {
            info!("loaded save version {older} (current {SAVE_VERSION})");
        }
        newer => warn!("save version {newer} is newer than this build ({SAVE_VERSION})"),
    }
    // The resource now tags this session's future saves, not the loaded file.
    *version = SaveVersion::default();
}

fn despawn_saved_entities(mut commands: Commands, saved: Query<Entity, With<Save>>) {
    for entity in &saved {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::AssetPlugin;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .register_type::<Name>()
            .register_type::<Transform>()
            .register_type::<Player>()
            .register_type::<SaveVersion>()
            .register_type::<CurrentLevel>()
            .init_resource::<SaveVersion>()
            .init_resource::<CurrentLevel>()
            .add_observer(save_on_default_event)
            .add_observer(load_on_default_event)
            .add_observer(check_version);
        app
    }

    fn temp_save_path(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("template-core-{tag}-{}.ron", std::process::id()))
    }

    fn save_to(app: &mut App, path: &PathBuf) {
        let mut event = SaveWorld::default_into_file(path)
            .include_resource::<SaveVersion>()
            .include_resource::<CurrentLevel>();
        event.components = persisted_components();
        app.world_mut().trigger_save(event);
        app.update();
    }

    #[test]
    fn save_roundtrip_restores_player_level_and_version() {
        let path = temp_save_path("roundtrip");
        let mut app = test_app();
        app.world_mut().resource_mut::<CurrentLevel>().0 = "levels/other_level.glb".into();
        let player = app
            .world_mut()
            .spawn((Player, Transform::from_xyz(1.0, 2.0, 3.0)))
            .id();
        save_to(&mut app, &path);

        // Mutate everything the load must restore.
        app.world_mut()
            .entity_mut(player)
            .insert(Transform::IDENTITY);
        app.world_mut().resource_mut::<CurrentLevel>().0 = "levels/wrong.glb".into();

        app.world_mut()
            .trigger_load(LoadWorld::default_from_file(path.clone()));
        app.update();

        let mut players = app.world_mut().query_filtered::<&Transform, With<Player>>();
        let transform = players.single(app.world()).expect("player restored");
        assert_eq!(transform.translation, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(
            app.world().resource::<CurrentLevel>().0,
            "levels/other_level.glb"
        );
        assert_eq!(
            *app.world().resource::<SaveVersion>(),
            SaveVersion::default()
        );

        // `Save` is re-required by `Player`, so the loaded player saves again.
        let mut saved = app
            .world_mut()
            .query_filtered::<(), (With<Player>, With<Save>)>();
        assert!(saved.single(app.world()).is_ok(), "loaded player lost Save");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn old_save_missing_newly_persisted_component_still_loads() {
        // "Old build": persists only Player + Transform.
        let path = temp_save_path("version-tolerance");
        let mut app = test_app();
        app.world_mut()
            .spawn((Player, Transform::from_xyz(5.0, 0.0, 5.0)));
        save_to(&mut app, &path);

        // "New build": also persists a component the old file has never
        // heard of. The old file must still load; the component is absent.
        #[derive(Component, Default, Reflect)]
        #[reflect(Component)]
        struct Stamina(f32);

        let mut app = test_app();
        app.register_type::<Stamina>();
        app.world_mut()
            .trigger_load(LoadWorld::default_from_file(path.clone()));
        app.update();

        let mut players = app
            .world_mut()
            .query_filtered::<(&Transform, Option<&Stamina>), With<Player>>();
        let (transform, stamina) = players.single(app.world()).expect("old save loads");
        assert_eq!(transform.translation, Vec3::new(5.0, 0.0, 5.0));
        assert!(stamina.is_none(), "old save cannot carry the new component");
        let _ = std::fs::remove_file(path);
    }
}
