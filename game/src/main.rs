use avian3d::PhysicsPlugins;
use bevy::prelude::*;
use template_core::{
    AppStatePlugin, CharacterAnimationPlugin, ControlsPlugin, DevScenePlugin,
    DiagnosticsOverlayPlugin, GameAudioPlugin, LevelPlugin, MenuPlugin, NavigationPlugin,
    NpcAiPlugin, NpcPlugin, PlayerPlugin, RtsCameraPlugin, SavePlugin, SettingsPlugin, SquadPlugin,
    ThirdPersonCameraPlugin,
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(AssetPlugin {
                // Assets live at the workspace root, one level above this
                // crate's manifest dir (the asset base under `cargo run`).
                // Shipped builds use the default: assets/ next to the binary.
                file_path: if cfg!(feature = "dev") {
                    "../assets".into()
                } else {
                    "assets".into()
                },
                ..default()
            }),
            PhysicsPlugins::default(),
        ))
        // App shell: states, menus, persistence, input, presentation.
        .add_plugins((
            AppStatePlugin,
            MenuPlugin,
            LevelPlugin,
            DevScenePlugin,
            ControlsPlugin,
            SettingsPlugin,
            SavePlugin,
            GameAudioPlugin,
            DiagnosticsOverlayPlugin,
        ))
        // Gameplay: characters, cameras, navigation, AI, squad layer.
        .add_plugins((
            PlayerPlugin,
            CharacterAnimationPlugin,
            ThirdPersonCameraPlugin,
            RtsCameraPlugin,
            NavigationPlugin,
            NpcPlugin,
            NpcAiPlugin,
            SquadPlugin,
        ))
        .run();
}
