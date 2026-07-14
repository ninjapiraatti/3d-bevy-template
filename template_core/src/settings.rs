//! Persistent game settings: window mode, resolution, master volume, and the
//! full input map (so any rebinding survives, mouse and gamepad included).
//!
//! The file (`settings.ron` in the working directory, next to `saves/`) is
//! loaded before startup, applied live whenever [`GameSettings`] changes, and
//! written back on every change. Missing fields fall back to defaults
//! (`#[serde(default)]`), so files from older versions keep loading as long
//! as fields only get added — same tolerance policy as save files.

use std::fs;

use bevy::audio::{GlobalVolume, Volume};
use bevy::prelude::*;
use bevy::window::{MonitorSelection, PrimaryWindow, WindowMode};
use leafwing_input_manager::prelude::*;
use serde::{Deserialize, Serialize};

use crate::controls::PlayerAction;

pub struct SettingsPlugin;

/// Bump when the settings format changes incompatibly; until then, added
/// fields default in and removed fields are ignored.
pub const SETTINGS_VERSION: u32 = 1;

const SETTINGS_FILE: &str = "settings.ron";

/// The window modes the settings menu offers — a deliberately smaller
/// vocabulary than [`WindowMode`] (exclusive fullscreen needs video-mode
/// selection and is finicky across platforms; out of template scope).
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum WindowModeSetting {
    Windowed,
    BorderlessFullscreen,
}

impl WindowModeSetting {
    fn to_bevy(self) -> WindowMode {
        match self {
            Self::Windowed => WindowMode::Windowed,
            Self::BorderlessFullscreen => {
                WindowMode::BorderlessFullscreen(MonitorSelection::Current)
            }
        }
    }
}

#[derive(Resource, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct GameSettings {
    pub version: u32,
    pub window_mode: WindowModeSetting,
    /// Logical pixels; applies in windowed mode.
    pub resolution: (f32, f32),
    /// Linear master volume, `0.0..=1.0`.
    pub master_volume: f32,
    /// The live bindings; the settings menu's rebinds land here. Player
    /// hydration reads this instead of [`PlayerAction::default_input_map`].
    pub input_map: InputMap<PlayerAction>,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            window_mode: WindowModeSetting::Windowed,
            resolution: (1280.0, 720.0),
            master_volume: 1.0,
            input_map: PlayerAction::default_input_map(),
        }
    }
}

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        // PreStartup so every plugin's deserializer registrations (leafwing's
        // input types) exist before the file is read, and every Startup
        // system already sees the resource.
        app.add_systems(PreStartup, load_settings).add_systems(
            Update,
            (
                apply_window_settings.run_if(resource_changed::<GameSettings>),
                apply_volume.run_if(resource_changed::<GameSettings>),
                persist_settings.run_if(
                    resource_changed::<GameSettings>.and_then(not(resource_added::<GameSettings>)),
                ),
            ),
        );
    }
}

fn load_settings(mut commands: Commands) {
    let settings = match fs::read_to_string(SETTINGS_FILE) {
        Ok(text) => match ron::from_str::<GameSettings>(&text) {
            Ok(settings) => {
                info!("loaded {SETTINGS_FILE}");
                settings
            }
            Err(err) => {
                warn!("unreadable {SETTINGS_FILE}, using defaults: {err}");
                GameSettings::default()
            }
        },
        // Absent on first launch; written on the first settings change.
        Err(_) => GameSettings::default(),
    };
    commands.insert_resource(GlobalVolume::new(Volume::Linear(settings.master_volume)));
    commands.insert_resource(settings);
}

/// Applies mode and (windowed-mode) resolution; also runs once on load, so
/// the file's window settings take effect at launch.
fn apply_window_settings(
    settings: Res<GameSettings>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let Ok(mut window) = windows.single_mut() else {
        return;
    };
    window.mode = settings.window_mode.to_bevy();
    if settings.window_mode == WindowModeSetting::Windowed {
        window
            .resolution
            .set(settings.resolution.0, settings.resolution.1);
    }
}

/// New playbacks pick this up automatically; already-playing sounds are
/// re-leveled by the audio plugin, which owns the sinks.
fn apply_volume(settings: Res<GameSettings>, mut volume: ResMut<GlobalVolume>) {
    volume.volume = Volume::Linear(settings.master_volume);
}

/// Replaces the keyboard key bound to `action`, leaving mouse and gamepad
/// bindings untouched.
///
/// The explicit deref matters: leafwing implements `Reflect` for the `Box`
/// itself, so `binding.as_any()` would be the box, not the input inside it.
pub fn rebind_key(map: &mut InputMap<PlayerAction>, action: PlayerAction, key: KeyCode) {
    if let Some(bindings) = map.get_buttonlike_mut(&action) {
        bindings.retain(|binding| {
            Reflect::as_any(&**binding)
                .downcast_ref::<KeyCode>()
                .is_none()
        });
    }
    map.insert(action, key);
}

/// The keyboard key currently bound to `action`, if any (for settings UI).
pub fn bound_key(map: &InputMap<PlayerAction>, action: &PlayerAction) -> Option<KeyCode> {
    map.get_buttonlike(action)?.iter().find_map(|binding| {
        Reflect::as_any(&**binding)
            .downcast_ref::<KeyCode>()
            .copied()
    })
}

fn persist_settings(settings: Res<GameSettings>) {
    let text = match ron::ser::to_string_pretty(&*settings, default()) {
        Ok(text) => text,
        Err(err) => {
            error!("settings not serializable: {err}");
            return;
        }
    };
    if let Err(err) = fs::write(SETTINGS_FILE, text) {
        error!("could not write {SETTINGS_FILE}: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fills leafwing's global input-deserializer registries, which real runs
    /// fill via `ControlsPlugin`.
    fn register_input_types() {
        let mut app = App::new();
        app.add_plugins(InputManagerPlugin::<PlayerAction>::default());
    }

    #[test]
    fn settings_round_trip_preserves_input_map() {
        register_input_types();
        let settings = GameSettings {
            master_volume: 0.4,
            window_mode: WindowModeSetting::BorderlessFullscreen,
            ..default()
        };
        let text = ron::ser::to_string_pretty(&settings, default()).unwrap();
        let loaded: GameSettings = ron::from_str(&text).unwrap();
        assert_eq!(loaded.master_volume, 0.4);
        assert_eq!(loaded.window_mode, WindowModeSetting::BorderlessFullscreen);
        assert_eq!(loaded.input_map, settings.input_map);
    }

    #[test]
    fn rebind_replaces_keyboard_but_keeps_other_devices() {
        let mut map = PlayerAction::default_input_map();
        rebind_key(&mut map, PlayerAction::Run, KeyCode::KeyR);

        assert_eq!(bound_key(&map, &PlayerAction::Run), Some(KeyCode::KeyR));
        let bindings = map.get_buttonlike(&PlayerAction::Run).unwrap();
        // The gamepad trigger from the defaults must survive the rebind.
        assert!(
            bindings.iter().any(|b| Reflect::as_any(&**b)
                .downcast_ref::<GamepadButton>()
                .is_some()),
            "gamepad binding was lost"
        );
        assert_eq!(
            bindings
                .iter()
                .filter(|b| Reflect::as_any(&***b).downcast_ref::<KeyCode>().is_some())
                .count(),
            1,
            "old keyboard binding was not replaced"
        );
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        register_input_types();
        // A file from an older version that predates every current field.
        let loaded: GameSettings = ron::from_str("(version: 1)").unwrap();
        assert_eq!(loaded.master_volume, 1.0);
        assert_eq!(loaded.window_mode, WindowModeSetting::Windowed);
        assert_eq!(loaded.input_map, PlayerAction::default_input_map());
    }
}
