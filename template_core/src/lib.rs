//! Core plugins for the 3rd-person game template.
//!
//! One Bevy plugin per concern; games compose the plugins they need.

pub mod animation;
pub mod audio;
pub mod camera_rig;
pub mod controls;
pub mod dev_scene;
pub mod diagnostics;
pub mod levels;
pub mod menus;
pub mod nav;
pub mod npc_ai;
pub mod npcs;
pub mod player;
pub mod rts_camera;
pub mod saves;
pub mod settings;
pub mod squad;
pub mod states;

pub use animation::{CharacterAnimationPlugin, CharacterAnimations};
pub use audio::GameAudioPlugin;
pub use camera_rig::{ThirdPersonCamera, ThirdPersonCameraPlugin};
pub use controls::{ControlsPlugin, PlayerAction};
pub use dev_scene::DevScenePlugin;
pub use diagnostics::DiagnosticsOverlayPlugin;
pub use levels::{CurrentLevel, LevelPlugin, PlayerSpawn};
pub use menus::MenuPlugin;
pub use nav::{CommandKind, Commandable, Commanded, Hold, NavigationPlugin};
pub use npc_ai::{Faction, FactionRelations, NpcAiPlugin, NpcBehavior, Perception};
pub use npcs::{Npc, NpcPlugin};
pub use player::{Player, PlayerPlugin};
pub use rts_camera::RtsCameraPlugin;
pub use saves::{LoadGame, SaveGame, SavePlugin, SaveVersion};
pub use settings::{GameSettings, SettingsPlugin, WindowModeSetting};
pub use squad::{Selected, SquadPlugin};
pub use states::{AppState, AppStatePlugin, CameraMode, PauseState};
