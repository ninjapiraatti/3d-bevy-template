//! Player character: a physics-driven capsule moved with camera-relative
//! input, visualized by an animated character scene parented to the capsule.
//!
//! The controller is deliberately simple (dynamic body + velocity control,
//! no jump — out of genre scope). If character feel ever needs more
//! (step-ups, moving platforms), bevy-tnua is the designated upgrade path.

use avian3d::prelude::*;
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::animation::CharacterAnimations;
use crate::camera_rig::ThirdPersonCamera;
use crate::controls::PlayerAction;
use crate::levels::PlayerSpawn;
use crate::states::{AppState, PauseState};

pub struct PlayerPlugin;

#[derive(Component)]
pub struct Player;

pub const WALK_SPEED: f32 = 3.0;
pub const RUN_SPEED: f32 = 6.0;
const TURN_SPEED: f32 = 12.0;
const CAPSULE_RADIUS: f32 = 0.35;
const CAPSULE_LENGTH: f32 = 1.0;
/// Below this height the player has left the map; put them back on the spawn.
const KILL_HEIGHT: f32 = -50.0;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (spawn_player, respawn_fallen).run_if(in_state(AppState::InGame)),
        )
        .add_systems(Update, move_player.run_if(in_state(PauseState::Running)));
    }
}

fn spawn_height() -> f32 {
    CAPSULE_LENGTH / 2.0 + CAPSULE_RADIUS + 0.1
}

/// Waits for the level's [`PlayerSpawn`] marker (scene instances appear
/// asynchronously), then spawns the player once.
fn spawn_player(
    mut commands: Commands,
    players: Query<(), With<Player>>,
    spawn_points: Query<&GlobalTransform, With<PlayerSpawn>>,
    assets: Res<AssetServer>,
) {
    if !players.is_empty() {
        return;
    }
    let Ok(spawn) = spawn_points.single() else {
        return;
    };
    let position = spawn.translation() + Vec3::Y * spawn_height();
    commands
        .spawn((
            Name::new("Player"),
            Player,
            DespawnOnExit(AppState::InGame),
            Transform::from_translation(position),
            Visibility::default(),
            RigidBody::Dynamic,
            Collider::capsule(CAPSULE_RADIUS, CAPSULE_LENGTH),
            LockedAxes::ROTATION_LOCKED,
            Friction::new(0.3),
            PlayerAction::default_input_map(),
        ))
        .with_children(|parent| {
            // Model origin is at the feet; the capsule origin is its center.
            // KayKit characters face +Z, entity forward is Bevy's -Z: yaw 180°.
            parent.spawn((
                Name::new("PlayerModel"),
                WorldAssetRoot(assets.load(
                    GltfAssetLabel::Scene(0).from_asset("characters/adventurers/Knight.glb"),
                )),
                CharacterAnimations::kaykit_adventurer(&assets),
                Transform::from_xyz(0.0, -(CAPSULE_LENGTH / 2.0 + CAPSULE_RADIUS), 0.0)
                    .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
            ));
        });
    info!("player spawned at {position}");
}

fn move_player(
    time: Res<Time>,
    mut players: Query<
        (
            &ActionState<PlayerAction>,
            &mut LinearVelocity,
            &mut Transform,
        ),
        With<Player>,
    >,
    camera: Query<&Transform, (With<ThirdPersonCamera>, Without<Player>)>,
) {
    let Ok((actions, mut velocity, mut transform)) = players.single_mut() else {
        return;
    };
    let Ok(camera) = camera.single() else {
        return;
    };

    let input = actions.clamped_axis_pair(&PlayerAction::Move);
    let speed = if actions.pressed(&PlayerAction::Run) {
        RUN_SPEED
    } else {
        WALK_SPEED
    };
    let direction = camera_relative_direction(camera.forward().as_vec3(), input);

    // Gravity keeps ownership of the vertical axis.
    velocity.x = direction.x * speed;
    velocity.z = direction.z * speed;

    if direction.length_squared() > 0.01 {
        let target = Quat::from_rotation_y(f32::atan2(-direction.x, -direction.z));
        let t = (TURN_SPEED * time.delta_secs()).min(1.0);
        transform.rotation = transform.rotation.slerp(target, t);
    }
}

/// Maps stick/WASD input into a world-space direction on the ground plane,
/// where "up" on the stick means "away from the camera".
fn camera_relative_direction(camera_forward: Vec3, input: Vec2) -> Vec3 {
    let forward = Vec3::new(camera_forward.x, 0.0, camera_forward.z).normalize_or_zero();
    let right = forward.cross(Vec3::Y).normalize_or_zero();
    (right * input.x + forward * input.y).clamp_length_max(1.0)
}

fn respawn_fallen(
    mut players: Query<(&mut Transform, &mut LinearVelocity), With<Player>>,
    spawn_points: Query<&GlobalTransform, With<PlayerSpawn>>,
) {
    for (mut transform, mut velocity) in &mut players {
        if transform.translation.y < KILL_HEIGHT {
            warn!("player fell out of the map, respawning");
            if let Ok(spawn) = spawn_points.single() {
                transform.translation = spawn.translation() + Vec3::Y * spawn_height();
            } else {
                transform.translation = Vec3::Y * spawn_height();
            }
            velocity.0 = Vec3::ZERO;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_up_moves_away_from_camera() {
        // Camera looking straight down +Z: stick-up must move toward +Z.
        let dir = camera_relative_direction(Vec3::Z, Vec2::Y);
        assert!((dir - Vec3::Z).length() < 1e-5, "got {dir}");
    }

    #[test]
    fn camera_pitch_does_not_leak_into_speed() {
        // A steeply pitched camera must still produce a full-speed
        // ground direction, not a shortened projection.
        let steep = (Vec3::Z - Vec3::Y * 5.0).normalize();
        let dir = camera_relative_direction(steep, Vec2::Y);
        assert!(
            (dir.length() - 1.0).abs() < 1e-4,
            "got length {}",
            dir.length()
        );
        assert_eq!(dir.y, 0.0);
    }

    #[test]
    fn diagonal_input_is_not_faster() {
        let dir = camera_relative_direction(Vec3::Z, Vec2::ONE);
        assert!(dir.length() <= 1.0 + 1e-5);
    }
}
