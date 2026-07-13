//! Core plugins for the 3rd-person game template.
//!
//! One Bevy plugin per concern; games compose the plugins they need.

pub mod animation;
pub mod camera_rig;
pub mod controls;
pub mod dev_scene;
pub mod levels;
pub mod menus;
pub mod nav;
pub mod npcs;
pub mod player;
pub mod saves;
pub mod states;

pub use animation::{CharacterAnimationPlugin, CharacterAnimations};
pub use camera_rig::{ThirdPersonCamera, ThirdPersonCameraPlugin};
pub use controls::{ControlsPlugin, PlayerAction};
pub use dev_scene::DevScenePlugin;
pub use levels::{CurrentLevel, LevelPlugin, PlayerSpawn};
pub use menus::MenuPlugin;
pub use nav::NavigationPlugin;
pub use npcs::{Npc, NpcPlugin};
pub use player::{Player, PlayerPlugin};
pub use saves::{LoadGame, SaveGame, SavePlugin, SaveVersion};
pub use states::{AppState, AppStatePlugin, PauseState};
