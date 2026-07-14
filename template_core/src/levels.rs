//! Level loading: glTF scenes exported from Blender, with gameplay data
//! authored as Blender custom properties (arriving as glTF "extras").
//!
//! Conventions are documented in `docs/blender-pipeline.md`. Currently:
//! - `marker = "player_spawn"` → [`PlayerSpawn`]
//! - `marker = "navmesh"` → [`NavMeshSource`]
//! - `marker = "npc_spawn"` (+ optional NPC properties) → [`NpcSpawn`]
//! - `marker = "waypoint"` → [`Waypoint`]
//! - `collider = "trimesh"` → static physics collider from the node's meshes

use avian3d::prelude::*;
use bevy::asset::LoadState;
use bevy::gltf::GltfExtras;
use bevy::prelude::*;

use crate::states::AppState;

pub struct LevelPlugin;

/// Asset path of the level to load when a game starts. Persisted in save
/// files: loading a save restores this before [`AppState::Loading`] reads it,
/// so the save's level is what gets spawned.
#[derive(Resource, Debug, Clone, Reflect)]
#[reflect(Resource)]
pub struct CurrentLevel(pub String);

impl Default for CurrentLevel {
    fn default() -> Self {
        // The script-generated test level; it carries the navmesh and NPC
        // markers that step 6 needs. `my_level.glb` works too once it has
        // them (see docs/blender-pipeline.md).
        Self("levels/test_level.glb".into())
    }
}

#[derive(Resource)]
struct LoadingLevel(Handle<WorldAsset>);

/// Where the player character appears; authored in Blender as an empty with
/// custom property `marker = "player_spawn"`.
#[derive(Component, Debug)]
pub struct PlayerSpawn;

/// A mesh describing the walkable area; authored in Blender as a (hidden at
/// runtime) mesh object with `marker = "navmesh"`. Consumed by the
/// navigation plugin.
#[derive(Component, Debug)]
pub struct NavMeshSource;

/// Where an NPC appears; authored in Blender as an empty with
/// `marker = "npc_spawn"`. Consumed by the NPC plugin, which resolves the
/// optional string properties (and their defaults) into typed components.
#[derive(Component, Debug, Default)]
pub struct NpcSpawn {
    /// `faction = "<name>"`; defaults to the player's faction.
    pub faction: Option<String>,
    /// `behavior = "idle" | "wander" | "patrol"`; defaults to idle.
    pub behavior: Option<String>,
    /// `route = "<name>"`: the waypoint route a patrol behavior follows.
    pub route: Option<String>,
    /// `character = "<file stem>"` under `assets/characters/adventurers/`.
    pub character: Option<String>,
}

/// A patrol waypoint; authored in Blender as an empty with
/// `marker = "waypoint"`. `route` groups waypoints, `order` sorts them.
#[derive(Component, Debug)]
pub struct Waypoint {
    pub route: String,
    pub order: f64,
}

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<CurrentLevel>()
            .init_resource::<CurrentLevel>()
            .add_systems(OnEnter(AppState::Loading), start_level_load)
            .add_systems(
                Update,
                advance_when_loaded.run_if(in_state(AppState::Loading)),
            )
            .add_systems(OnEnter(AppState::InGame), spawn_level)
            .add_systems(Update, process_gltf_extras);
    }
}

fn start_level_load(mut commands: Commands, level: Res<CurrentLevel>, assets: Res<AssetServer>) {
    info!("loading level {}", level.0);
    let scene = assets.load(GltfAssetLabel::Scene(0).from_asset(level.0.clone()));
    commands.insert_resource(LoadingLevel(scene));
}

fn advance_when_loaded(
    loading: Res<LoadingLevel>,
    assets: Res<AssetServer>,
    mut next: ResMut<NextState<AppState>>,
) {
    match assets.load_state(&loading.0) {
        LoadState::Failed(err) => {
            error!("level failed to load, returning to menu: {err}");
            next.set(AppState::MainMenu);
        }
        _ if assets.is_loaded_with_dependencies(&loading.0) => next.set(AppState::InGame),
        _ => {}
    }
}

fn spawn_level(mut commands: Commands, loading: Option<Res<LoadingLevel>>) {
    // Absent when InGame is entered without the loading flow (e.g. tests).
    let Some(loading) = loading else { return };
    commands.spawn((
        Name::new("Level"),
        WorldAssetRoot(loading.0.clone()),
        DespawnOnExit(AppState::InGame),
    ));
    commands.remove_resource::<LoadingLevel>();
}

/// Turns Blender custom properties into typed components. Scene instances
/// spawn asynchronously, so this watches for extras appearing on any frame.
fn process_gltf_extras(
    mut commands: Commands,
    extras: Query<(Entity, Option<&Name>, &GltfExtras), Added<GltfExtras>>,
) {
    for (entity, name, extras) in &extras {
        let parsed: serde_json::Value = match serde_json::from_str(&extras.value) {
            Ok(value) => value,
            Err(err) => {
                warn!("unparseable glTF extras on {name:?}: {err}");
                continue;
            }
        };
        if let Some(marker) = parsed.get("marker").and_then(|m| m.as_str()) {
            match marker {
                "player_spawn" => {
                    info!("marker player_spawn on {name:?} ({entity})");
                    commands.entity(entity).insert(PlayerSpawn);
                }
                "navmesh" => {
                    info!("marker navmesh on {name:?} ({entity})");
                    commands.entity(entity).insert(NavMeshSource);
                }
                "npc_spawn" => {
                    info!("marker npc_spawn on {name:?} ({entity})");
                    let get =
                        |key: &str| parsed.get(key).and_then(|v| v.as_str()).map(String::from);
                    commands.entity(entity).insert(NpcSpawn {
                        faction: get("faction"),
                        behavior: get("behavior"),
                        route: get("route"),
                        character: get("character"),
                    });
                }
                "waypoint" => {
                    info!("marker waypoint on {name:?} ({entity})");
                    commands.entity(entity).insert(Waypoint {
                        route: parsed
                            .get("route")
                            .and_then(|v| v.as_str())
                            .unwrap_or("A")
                            .to_string(),
                        order: parsed.get("order").and_then(|v| v.as_f64()).unwrap_or(0.0),
                    });
                }
                other => warn!("unknown marker '{other}' on {name:?}"),
            }
        }
        if let Some(collider) = parsed.get("collider").and_then(|c| c.as_str()) {
            match collider {
                "trimesh" => {
                    commands.entity(entity).insert((
                        RigidBody::Static,
                        ColliderConstructorHierarchy::new(ColliderConstructor::TrimeshFromMesh),
                    ));
                }
                other => warn!("unknown collider type '{other}' on {name:?}"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extras_app() -> App {
        let mut app = App::new();
        app.add_systems(Update, process_gltf_extras);
        app
    }

    fn spawn_with_extras(app: &mut App, json: &str) -> Entity {
        let entity = app
            .world_mut()
            .spawn(GltfExtras {
                value: json.to_string(),
            })
            .id();
        app.update();
        entity
    }

    #[test]
    fn player_spawn_marker_becomes_component() {
        let mut app = extras_app();
        let entity = spawn_with_extras(&mut app, r#"{"marker": "player_spawn"}"#);
        assert!(app.world().entity(entity).contains::<PlayerSpawn>());
    }

    #[test]
    fn navigation_markers_become_components() {
        let mut app = extras_app();
        let navmesh = spawn_with_extras(&mut app, r#"{"marker": "navmesh"}"#);
        let npc = spawn_with_extras(&mut app, r#"{"marker": "npc_spawn"}"#);
        assert!(app.world().entity(navmesh).contains::<NavMeshSource>());
        assert!(app.world().entity(npc).contains::<NpcSpawn>());
    }

    #[test]
    fn npc_spawn_properties_are_captured() {
        let mut app = extras_app();
        let bare = spawn_with_extras(&mut app, r#"{"marker": "npc_spawn"}"#);
        let spawn = app.world().entity(bare).get::<NpcSpawn>().unwrap();
        assert!(spawn.faction.is_none() && spawn.behavior.is_none());

        let configured = spawn_with_extras(
            &mut app,
            r#"{"marker": "npc_spawn", "faction": "raiders", "behavior": "patrol",
                "route": "A", "character": "Rogue_Hooded"}"#,
        );
        let spawn = app.world().entity(configured).get::<NpcSpawn>().unwrap();
        assert_eq!(spawn.faction.as_deref(), Some("raiders"));
        assert_eq!(spawn.behavior.as_deref(), Some("patrol"));
        assert_eq!(spawn.route.as_deref(), Some("A"));
        assert_eq!(spawn.character.as_deref(), Some("Rogue_Hooded"));
    }

    #[test]
    fn waypoint_marker_parses_route_and_order() {
        let mut app = extras_app();
        let waypoint = spawn_with_extras(
            &mut app,
            r#"{"marker": "waypoint", "route": "B", "order": 2}"#,
        );
        let waypoint = app.world().entity(waypoint).get::<Waypoint>().unwrap();
        assert_eq!(waypoint.route, "B");
        assert_eq!(waypoint.order, 2.0);

        let defaulted = spawn_with_extras(&mut app, r#"{"marker": "waypoint"}"#);
        let waypoint = app.world().entity(defaulted).get::<Waypoint>().unwrap();
        assert_eq!(waypoint.route, "A");
        assert_eq!(waypoint.order, 0.0);
    }

    #[test]
    fn trimesh_collider_property_adds_static_physics() {
        let mut app = extras_app();
        let entity = spawn_with_extras(&mut app, r#"{"collider": "trimesh"}"#);
        assert_eq!(
            app.world().entity(entity).get::<RigidBody>(),
            Some(&RigidBody::Static)
        );
        assert!(
            app.world()
                .entity(entity)
                .contains::<ColliderConstructorHierarchy>()
        );
    }

    #[test]
    fn unknown_and_malformed_extras_are_ignored() {
        let mut app = extras_app();
        let unknown = spawn_with_extras(&mut app, r#"{"marker": "nonsense"}"#);
        let malformed = spawn_with_extras(&mut app, "not json at all");
        assert!(!app.world().entity(unknown).contains::<PlayerSpawn>());
        assert!(!app.world().entity(malformed).contains::<PlayerSpawn>());
    }
}
