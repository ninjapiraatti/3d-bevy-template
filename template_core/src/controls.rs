//! Input mapping: one data-driven map from devices to game actions.
//! Rebinding means editing [`PlayerAction::default_input_map`] (or, from
//! roadmap step 9 on, the settings screen) — gameplay systems only ever see
//! actions, never keys.

use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use serde::{Deserialize, Serialize};

pub struct ControlsPlugin;

impl Plugin for ControlsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InputManagerPlugin::<PlayerAction>::default());
    }
}

/// One action set for both control schemes ([`CameraMode`]): systems gate on
/// the mode, so the same physical input can drive different actions per mode
/// (left mouse is `CommandMove` in third person but `Select` in top-down).
/// `Move` and `Zoom` are deliberately shared — they mean "directional input"
/// and "zoom" to whichever camera controller is active.
///
/// [`CameraMode`]: crate::states::CameraMode
#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect, Serialize, Deserialize)]
pub enum PlayerAction {
    #[actionlike(DualAxis)]
    Move,
    Run,
    /// Order the squad to a clicked point in third person (left mouse).
    CommandMove,
    /// Hold to orbit the camera (right mouse button by default).
    Orbit,
    #[actionlike(DualAxis)]
    OrbitDelta,
    #[actionlike(Axis)]
    Zoom,
    /// Select units — click or drag a box — in top-down mode (left mouse).
    Select,
    /// Order selected units to a clicked point in top-down mode (right mouse).
    Command,
    /// Held while commanding turns the order into an attack-move (left ctrl).
    AttackModifier,
    /// Stop selected units and hold position (H).
    StopHold,
    /// Switch between the third-person and top-down control schemes (Tab).
    ToggleCameraMode,
}

impl PlayerAction {
    pub fn default_input_map() -> InputMap<Self> {
        let mut map = InputMap::default();

        map.insert_dual_axis(Self::Move, VirtualDPad::wasd());
        map.insert(Self::Run, KeyCode::ShiftLeft);
        map.insert(Self::CommandMove, MouseButton::Left);
        map.insert(Self::Orbit, MouseButton::Right);
        map.insert_dual_axis(Self::OrbitDelta, MouseMove::default());
        map.insert_axis(Self::Zoom, MouseScrollAxis::Y);

        // Squad/strategy scheme (top-down mode). Mouse-driven by design;
        // no gamepad bindings for it.
        map.insert(Self::Select, MouseButton::Left);
        map.insert(Self::Command, MouseButton::Right);
        map.insert(Self::AttackModifier, KeyCode::ControlLeft);
        map.insert(Self::StopHold, KeyCode::KeyH);
        map.insert(Self::ToggleCameraMode, KeyCode::Tab);

        map.insert_dual_axis(Self::Move, GamepadStick::LEFT);
        map.insert(Self::Run, GamepadButton::LeftTrigger);
        map.insert(Self::Orbit, GamepadButton::RightTrigger);
        map.insert_dual_axis(Self::OrbitDelta, GamepadStick::RIGHT);

        map
    }
}
