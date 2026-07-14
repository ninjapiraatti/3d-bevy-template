//! Top-down RTS camera: the strategy half of the template's camera duality.
//!
//! There is one camera entity in-game (spawned and persisted by the
//! third-person rig); this plugin owns the [`CameraMode`] toggle and drives
//! that camera's transform while [`CameraMode::TopDown`] is active — WASD
//! pans, scroll zooms, pitch is fixed. The third-person rig's state is left
//! untouched, so toggling back resumes exactly where it was.

use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::controls::PlayerAction;
use crate::player::Player;
use crate::states::{CameraMode, PauseState};

pub struct RtsCameraPlugin;

/// Where the top-down camera looks and from how far. Runtime-only: re-created
/// (centered on the player) every time top-down mode is entered.
#[derive(Resource)]
struct RtsRig {
    focus: Vec3,
    distance: f32,
}

/// Fixed downward pitch, radians from horizontal. Steep enough to read as
/// top-down, shallow enough to keep depth cues.
const PITCH: f32 = 1.1;
const DISTANCE_RANGE: (f32, f32) = (8.0, 40.0);
const START_DISTANCE: f32 = 20.0;
/// Pan speed scales with zoom so screen-space speed stays constant.
const PAN_SPEED_PER_DISTANCE: f32 = 0.8;
const ZOOM_STEP: f32 = 0.9;

impl Plugin for RtsCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, toggle_mode.run_if(in_state(PauseState::Running)))
            .add_systems(OnEnter(CameraMode::TopDown), center_on_player)
            .add_systems(
                Update,
                drive_camera
                    .run_if(in_state(PauseState::Running).and_then(in_state(CameraMode::TopDown))),
            );
    }
}

fn toggle_mode(
    actions: Query<&ActionState<PlayerAction>, With<Player>>,
    mode: Res<State<CameraMode>>,
    mut next: ResMut<NextState<CameraMode>>,
) {
    let Ok(actions) = actions.single() else {
        return;
    };
    if actions.just_pressed(&PlayerAction::ToggleCameraMode) {
        next.set(match mode.get() {
            CameraMode::ThirdPerson => CameraMode::TopDown,
            CameraMode::TopDown => CameraMode::ThirdPerson,
        });
    }
}

fn center_on_player(mut commands: Commands, players: Query<&Transform, With<Player>>) {
    let focus = players
        .single()
        .map(|player| player.translation)
        .unwrap_or(Vec3::ZERO);
    commands.insert_resource(RtsRig {
        focus,
        distance: START_DISTANCE,
    });
}

fn drive_camera(
    time: Res<Time>,
    mut rig: ResMut<RtsRig>,
    actions: Query<&ActionState<PlayerAction>, With<Player>>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
) {
    let (Ok(actions), Ok(mut camera)) = (actions.single(), cameras.single_mut()) else {
        return;
    };

    // The camera faces world -Z, so pan input maps straight onto XZ:
    // stick/WASD up pans away from the screen bottom.
    let input = actions.clamped_axis_pair(&PlayerAction::Move);
    let pan = Vec3::new(input.x, 0.0, -input.y) * PAN_SPEED_PER_DISTANCE * rig.distance;
    rig.focus += pan * time.delta_secs();

    let scroll = actions.value(&PlayerAction::Zoom);
    if scroll != 0.0 {
        rig.distance =
            (rig.distance * ZOOM_STEP.powf(scroll)).clamp(DISTANCE_RANGE.0, DISTANCE_RANGE.1);
    }

    let toward_camera = Vec3::new(0.0, PITCH.sin(), PITCH.cos());
    *camera = Transform::from_translation(rig.focus + toward_camera * rig.distance)
        .looking_at(rig.focus, Vec3::Y);
}
