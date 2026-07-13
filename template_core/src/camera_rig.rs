//! Third-person orbit/follow camera: hold right mouse (or right stick) to
//! orbit, scroll to zoom, raycast keeps it from clipping through level
//! geometry.

use avian3d::prelude::*;
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use moonshine_save::prelude::Save;

use crate::controls::PlayerAction;
use crate::player::Player;
use crate::states::{AppState, PauseState};

pub struct ThirdPersonCameraPlugin;

/// Persistent camera-rig state; the render-side components are rebuilt by
/// [`hydrate_camera`] for fresh spawns and loaded saves alike.
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(Save)]
pub struct ThirdPersonCamera {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    /// Point above the player's feet the camera looks at.
    pub focus_height: f32,
}

impl Default for ThirdPersonCamera {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.5,
            distance: 7.0,
            focus_height: 1.2,
        }
    }
}

const PITCH_RANGE: (f32, f32) = (0.05, 1.35);
const DISTANCE_RANGE: (f32, f32) = (2.0, 14.0);
const ORBIT_SENSITIVITY: f32 = 0.004;
const ZOOM_STEP: f32 = 0.9;
/// Keeps the near plane out of walls when the collision ray hits.
const COLLISION_MARGIN: f32 = 0.25;

impl Plugin for ThirdPersonCameraPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ThirdPersonCamera>()
            .add_systems(OnEnter(AppState::InGame), spawn_camera)
            .add_systems(Update, hydrate_camera.run_if(in_state(AppState::InGame)))
            .add_systems(
                Update,
                (orbit_and_zoom, follow_player)
                    .chain()
                    .run_if(in_state(PauseState::Running)),
            );
    }
}

/// Spawns the rig's persistent core unless a loaded save brought one along.
fn spawn_camera(mut commands: Commands, cameras: Query<(), With<ThirdPersonCamera>>) {
    if !cameras.is_empty() {
        return;
    }
    commands.spawn((
        Name::new("ThirdPersonCamera"),
        ThirdPersonCamera::default(),
        Transform::from_xyz(-6.0, 6.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Rebuilds the render side of the rig; never part of save data.
fn hydrate_camera(
    mut commands: Commands,
    cameras: Query<Entity, (With<ThirdPersonCamera>, Without<Camera3d>)>,
) {
    for entity in &cameras {
        commands
            .entity(entity)
            .insert((Camera3d::default(), DespawnOnExit(AppState::InGame)));
    }
}

fn orbit_and_zoom(
    actions: Query<&ActionState<PlayerAction>, With<Player>>,
    mut rigs: Query<&mut ThirdPersonCamera>,
) {
    let (Ok(actions), Ok(mut rig)) = (actions.single(), rigs.single_mut()) else {
        return;
    };

    if actions.pressed(&PlayerAction::Orbit) {
        let delta = actions.axis_pair(&PlayerAction::OrbitDelta);
        rig.yaw -= delta.x * ORBIT_SENSITIVITY;
        rig.pitch = (rig.pitch + delta.y * ORBIT_SENSITIVITY).clamp(PITCH_RANGE.0, PITCH_RANGE.1);
    }

    let scroll = actions.value(&PlayerAction::Zoom);
    if scroll != 0.0 {
        rig.distance =
            (rig.distance * ZOOM_STEP.powf(scroll)).clamp(DISTANCE_RANGE.0, DISTANCE_RANGE.1);
    }
}

fn follow_player(
    spatial: SpatialQuery,
    players: Query<(Entity, &Transform), With<Player>>,
    mut cameras: Query<(&ThirdPersonCamera, &mut Transform), Without<Player>>,
) {
    let (Ok((player, player_transform)), Ok((rig, mut camera_transform))) =
        (players.single(), cameras.single_mut())
    else {
        return;
    };

    let focus = player_transform.translation + Vec3::Y * rig.focus_height;
    let toward_camera = Vec3::new(
        rig.pitch.cos() * rig.yaw.sin(),
        rig.pitch.sin(),
        rig.pitch.cos() * rig.yaw.cos(),
    );

    // Pull the camera in front of whatever level geometry the ray hits so
    // walls never end up between the player and the camera.
    let mut distance = rig.distance;
    if let Ok(direction) = Dir3::new(toward_camera) {
        let filter = SpatialQueryFilter::default().with_excluded_entities([player]);
        if let Some(hit) = spatial.cast_ray(focus, direction, rig.distance, true, &filter) {
            distance = (hit.distance - COLLISION_MARGIN).max(DISTANCE_RANGE.0.min(1.0));
        }
    }

    *camera_transform =
        Transform::from_translation(focus + toward_camera * distance).looking_at(focus, Vec3::Y);
}
