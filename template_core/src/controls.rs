//! Input mapping: one data-driven map from devices to game actions.
//! Rebinding means editing [`PlayerAction::default_input_map`] (or, from
//! roadmap step 9 on, the settings screen) — gameplay systems only ever see
//! actions, never keys.

use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

pub struct ControlsPlugin;

impl Plugin for ControlsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InputManagerPlugin::<PlayerAction>::default());
    }
}

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
pub enum PlayerAction {
    #[actionlike(DualAxis)]
    Move,
    Run,
    /// Hold to orbit the camera (right mouse button by default).
    Orbit,
    #[actionlike(DualAxis)]
    OrbitDelta,
    #[actionlike(Axis)]
    Zoom,
}

impl PlayerAction {
    pub fn default_input_map() -> InputMap<Self> {
        let mut map = InputMap::default();

        map.insert_dual_axis(Self::Move, VirtualDPad::wasd());
        map.insert(Self::Run, KeyCode::ShiftLeft);
        map.insert(Self::Orbit, MouseButton::Right);
        map.insert_dual_axis(Self::OrbitDelta, MouseMove::default());
        map.insert_axis(Self::Zoom, MouseScrollAxis::Y);

        map.insert_dual_axis(Self::Move, GamepadStick::LEFT);
        map.insert(Self::Run, GamepadButton::LeftTrigger);
        map.insert(Self::Orbit, GamepadButton::RightTrigger);
        map.insert_dual_axis(Self::OrbitDelta, GamepadStick::RIGHT);

        map
    }
}
