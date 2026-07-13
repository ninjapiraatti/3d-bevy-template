//! Core plugins for the 3rd-person game template.
//!
//! One Bevy plugin per concern; games compose the plugins they need.

pub mod camera_rig;
pub mod controls;
pub mod dev_scene;
pub mod levels;
pub mod menus;
pub mod player;
pub mod states;

pub use camera_rig::{ThirdPersonCamera, ThirdPersonCameraPlugin};
pub use controls::{ControlsPlugin, PlayerAction};
pub use dev_scene::DevScenePlugin;
pub use levels::{CurrentLevel, LevelPlugin, PlayerSpawn};
pub use menus::MenuPlugin;
pub use player::{Player, PlayerPlugin};
pub use states::{AppState, AppStatePlugin, PauseState};
