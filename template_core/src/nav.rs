//! Navigation on landmass: navmesh islands from Blender-authored meshes,
//! move-order lifecycle, and desired-velocity → physics application.
//! Orders are issued by the squad plugin (`squad.rs`); this plugin owns the
//! order vocabulary ([`Commanded`], [`Hold`]) and completes fulfilled orders.
//!
//! The navmesh is not generated — it is a mesh authored in the level's
//! .blend file, marked `marker = "navmesh"` (hidden at runtime). Re-exporting
//! the level re-exports the navmesh with it.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_landmass::debug::{EnableLandmassDebug, Landmass3dDebugPlugin};
use bevy_landmass::nav_mesh::bevy_mesh_to_landmass_nav_mesh;
use bevy_landmass::prelude::*;
use bevy_landmass::{AgentState, NavMeshHandle, PointSampleDistance3d};

use crate::levels::NavMeshSource;
use crate::states::{AppState, PauseState};

pub struct NavigationPlugin;

/// Agents that obey the player's move commands (player-faction NPCs).
/// Selection and command issuing live in the squad plugin (`squad.rs`).
#[derive(Component)]
pub struct Commandable;

/// What a player order asks of a unit. `AttackMove` is a stub: it moves
/// exactly like `Move`, but the intent is data — a downstream game's combat
/// systems read it to engage hostiles encountered on the way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandKind {
    Move,
    AttackMove,
}

/// A move order in flight: the destination of the last command.
/// While present, the behavior FSM (`npc_ai`) leaves the agent's target
/// alone; cleared on arrival so the assigned behavior resumes.
#[derive(Component)]
pub struct Commanded {
    pub destination: Vec3,
    pub kind: CommandKind,
}

/// A stop/hold order: the unit stands its ground — no behavior, no chase —
/// until the next explicit command removes this.
#[derive(Component)]
pub struct Hold;

/// Arrival slack for clearing [`Commanded`]: the agent transform sits at the
/// capsule center (~1 m above the clicked ground point), and landmass may
/// settle slightly short of the exact point.
const COMMAND_DONE_DISTANCE: f32 = 2.0;

/// How fast agents turn to face their direction of travel (same feel as the
/// player controller).
const TURN_SPEED: f32 = 12.0;

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
            (build_islands, finish_commands, apply_agent_velocity)
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

/// Clears a fulfilled move order. The distance check guards against a stale
/// `ReachedTarget` (landmass may not have processed the new target yet).
fn finish_commands(
    mut commands: Commands,
    agents: Query<(Entity, &GlobalTransform, &Commanded, &AgentState)>,
) {
    for (agent, transform, commanded, state) in &agents {
        if *state == AgentState::ReachedTarget
            && transform.translation().distance(commanded.destination) < COMMAND_DONE_DISTANCE
        {
            commands.entity(agent).remove::<Commanded>();
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_clears_on_arrival_but_not_before() {
        let mut app = App::new();
        app.add_systems(Update, finish_commands);
        let point = Vec3::new(2.0, 0.0, 3.0);
        let order = || Commanded {
            destination: point,
            kind: CommandKind::Move,
        };
        let arrived = app
            .world_mut()
            .spawn((
                GlobalTransform::from_translation(point + Vec3::Y),
                order(),
                AgentState::ReachedTarget,
            ))
            .id();
        let moving = app
            .world_mut()
            .spawn((
                GlobalTransform::from_translation(Vec3::ZERO),
                order(),
                AgentState::Moving,
            ))
            .id();
        // A stale ReachedTarget from a previous target must not clear a
        // fresh command to a distant point.
        let stale = app
            .world_mut()
            .spawn((
                GlobalTransform::from_translation(point + Vec3::X * 10.0),
                order(),
                AgentState::ReachedTarget,
            ))
            .id();
        app.update();
        assert!(!app.world().entity(arrived).contains::<Commanded>());
        assert!(app.world().entity(moving).contains::<Commanded>());
        assert!(app.world().entity(stale).contains::<Commanded>());
    }
}
