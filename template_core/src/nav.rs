//! Navigation on landmass: navmesh islands from Blender-authored meshes,
//! click-to-move targets, and desired-velocity → physics application.
//!
//! The navmesh is not generated — it is a mesh authored in the level's
//! .blend file, marked `marker = "navmesh"` (hidden at runtime). Re-exporting
//! the level re-exports the navmesh with it.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_landmass::debug::{EnableLandmassDebug, Landmass3dDebugPlugin};
use bevy_landmass::nav_mesh::bevy_mesh_to_landmass_nav_mesh;
use bevy_landmass::prelude::*;
use bevy_landmass::{NavMeshHandle, PointSampleDistance3d};
use leafwing_input_manager::prelude::*;

use crate::camera_rig::ThirdPersonCamera;
use crate::controls::PlayerAction;
use crate::levels::NavMeshSource;
use crate::player::Player;
use crate::states::{AppState, PauseState};

pub struct NavigationPlugin;

/// How fast agents turn to face their direction of travel (same feel as the
/// player controller).
const TURN_SPEED: f32 = 12.0;
/// Clicks are ray-cast this far into the world.
const COMMAND_RAY_LENGTH: f32 = 250.0;

impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            Landmass3dPlugin::default(),
            Landmass3dDebugPlugin {
                draw_on_start: false,
                ..default()
            },
        ))
        .add_systems(OnEnter(AppState::InGame), spawn_archipelago)
        .add_systems(Update, toggle_debug)
        .add_systems(
            Update,
            (build_islands, command_move, apply_agent_velocity)
                .run_if(in_state(PauseState::Running)),
        );
    }
}

/// Agent transforms sit at the physics-capsule center, well above the
/// navmesh, so the default (radius-derived) vertical sampling is too tight.
fn archipelago_options() -> ArchipelagoOptions<ThreeD> {
    ArchipelagoOptions {
        point_sample_distance: PointSampleDistance3d {
            horizontal_distance: 0.6,
            distance_above: 0.5,
            // Capsule center is ~0.85 above the ground; leave slack for slopes.
            distance_below: 2.0,
            vertical_preference_ratio: 2.0,
            animation_link_max_vertical_distance: 0.5,
        },
        neighbourhood: 3.5,
        avoidance_time_horizon: 0.5,
        obstacle_avoidance_time_horizon: 0.25,
        reached_destination_avoidance_responsibility: 0.1,
    }
}

fn spawn_archipelago(mut commands: Commands) {
    commands.spawn((
        Name::new("Archipelago"),
        DespawnOnExit(AppState::InGame),
        Archipelago3d::new(archipelago_options()),
    ));
}

/// Turns each Blender navmesh (`marker = "navmesh"`) into a landmass island
/// and hides its rendered mesh. Scene spawning is asynchronous, so this
/// retries until both the archipelago and the mesh exist.
fn build_islands(
    mut commands: Commands,
    sources: Query<(Entity, Option<&Children>), (With<NavMeshSource>, Without<Island>)>,
    archipelagos: Query<Entity, With<Archipelago3d>>,
    mesh_handles: Query<&Mesh3d>,
    meshes: Res<Assets<Mesh>>,
    mut nav_meshes: ResMut<Assets<NavMesh3d>>,
) {
    let Ok(archipelago) = archipelagos.single() else {
        return;
    };
    for (entity, children) in &sources {
        // glTF puts mesh primitives on child entities of the marked node.
        let handle = mesh_handles.get(entity).ok().or_else(|| {
            children
                .into_iter()
                .flatten()
                .find_map(|&child| mesh_handles.get(child).ok())
        });
        let Some(mesh) = handle.and_then(|handle| meshes.get(&handle.0)) else {
            continue;
        };

        let nav_mesh = match bevy_mesh_to_landmass_nav_mesh::<ThreeD>(mesh) {
            Ok(nav_mesh) => nav_mesh,
            Err(err) => {
                warn!("navmesh {entity} is not convertible: {err:?}");
                commands.entity(entity).remove::<NavMeshSource>();
                continue;
            }
        };
        let valid = match nav_mesh.validate() {
            Ok(valid) => valid,
            Err(err) => {
                warn!("navmesh {entity} failed validation: {err:?}");
                commands.entity(entity).remove::<NavMeshSource>();
                continue;
            }
        };

        info!("navmesh island built from {entity}");
        commands.entity(entity).insert((
            Island3dBundle {
                island: Island,
                archipelago_ref: ArchipelagoRef3d::new(archipelago),
                nav_mesh: NavMeshHandle(nav_meshes.add(NavMesh3d {
                    nav_mesh: std::sync::Arc::new(valid),
                })),
            },
            Visibility::Hidden,
        ));
    }
}

/// Left click sends every agent toward the clicked point (scoped to the
/// selected units when step 8 adds selection).
fn command_move(
    actions: Query<&ActionState<PlayerAction>, With<Player>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<ThirdPersonCamera>>,
    spatial: SpatialQuery,
    mut agents: Query<&mut AgentTarget3d>,
) {
    let Ok(actions) = actions.single() else {
        return;
    };
    if !actions.just_pressed(&PlayerAction::CommandMove) {
        return;
    }
    let (Ok(window), Ok((camera, camera_transform))) = (windows.single(), cameras.single()) else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor) else {
        return;
    };
    let Some(hit) = spatial.cast_ray(
        ray.origin,
        ray.direction,
        COMMAND_RAY_LENGTH,
        true,
        &SpatialQueryFilter::default(),
    ) else {
        return;
    };
    let point = ray.origin + *ray.direction * hit.distance;
    info!("move command to {point}");
    for mut target in &mut agents {
        *target = AgentTarget3d::Point(point);
    }
}

/// Landmass computes a desired velocity; the physics body executes it.
/// Gravity keeps the vertical axis, and the agent reports its actual
/// velocity back for other agents' avoidance.
fn apply_agent_velocity(
    time: Res<Time>,
    mut agents: Query<(
        &AgentDesiredVelocity3d,
        &mut Velocity3d,
        &mut LinearVelocity,
        &mut Transform,
    )>,
) {
    for (desired, mut reported, mut velocity, mut transform) in &mut agents {
        let desired = desired.velocity();
        velocity.x = desired.x;
        velocity.z = desired.z;
        reported.velocity = velocity.0;

        let planar = Vec3::new(desired.x, 0.0, desired.z);
        if planar.length_squared() > 0.01 {
            let target = Quat::from_rotation_y(f32::atan2(-planar.x, -planar.z));
            let t = (TURN_SPEED * time.delta_secs()).min(1.0);
            transform.rotation = transform.rotation.slerp(target, t);
        }
    }
}

/// Debug-tooling key, deliberately outside the player's input map (like the
/// pause key): F3 draws the navmesh and agent paths with gizmos.
fn toggle_debug(input: Res<ButtonInput<KeyCode>>, mut enable: ResMut<EnableLandmassDebug>) {
    if input.just_pressed(KeyCode::F3) {
        **enable = !**enable;
    }
}
