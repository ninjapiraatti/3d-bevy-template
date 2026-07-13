//! Core plugins for the 3rd-person game template.
//!
//! One Bevy plugin per concern; games compose the plugins they need.

pub mod dev_scene;
pub mod levels;
pub mod menus;
pub mod states;

pub use dev_scene::DevScenePlugin;
pub use levels::{CurrentLevel, LevelPlugin, PlayerSpawn};
pub use menus::MenuPlugin;
pub use states::{AppState, AppStatePlugin, PauseState};
