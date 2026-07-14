//! Audio plumbing reference: a UI click on every menu button and one looping
//! positional 3D sound (the spinner cube hums; the camera listens).
//!
//! Volume: new playbacks inherit [`GlobalVolume`] (kept in sync by the
//! settings plugin); this plugin re-levels already-playing sinks when
//! [`GameSettings`] changes, so the volume setting is audibly live. All
//! template sounds play at base volume 1.0, which keeps "sink volume =
//! master volume" true.

use bevy::audio::{AudioSink, AudioSinkPlayback, SpatialAudioSink, Volume};
use bevy::prelude::*;

use crate::dev_scene::Spinner;
use crate::settings::GameSettings;
use crate::states::AppState;

pub struct GameAudioPlugin;

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                click_on_buttons,
                attach_listener,
                attach_hum.run_if(in_state(AppState::InGame)),
                relevel_sinks.run_if(resource_exists_and_changed::<GameSettings>),
            ),
        );
    }
}

/// One shared click for every `Button`, menus and settings alike.
fn click_on_buttons(
    mut commands: Commands,
    buttons: Query<&Interaction, (Changed<Interaction>, With<Button>)>,
    assets: Res<AssetServer>,
) {
    for interaction in &buttons {
        if *interaction == Interaction::Pressed {
            commands.spawn((
                Name::new("UiClick"),
                AudioPlayer::new(assets.load("audio/ui_click.wav")),
                PlaybackSettings::DESPAWN,
            ));
        }
    }
}

/// The active camera hears the world; ear offsets stay at the defaults.
fn attach_listener(
    mut commands: Commands,
    cameras: Query<Entity, (With<Camera3d>, Without<SpatialListener>)>,
) {
    for camera in &cameras {
        commands.entity(camera).insert(SpatialListener::default());
    }
}

/// The reference emitter: the spinner cube hums in a loop. Walk around it —
/// panning and distance attenuation are the observable result.
fn attach_hum(
    mut commands: Commands,
    spinners: Query<Entity, (With<Spinner>, Without<AudioPlayer>)>,
    assets: Res<AssetServer>,
) {
    for spinner in &spinners {
        commands.entity(spinner).insert((
            AudioPlayer::new(assets.load("audio/spatial_hum.wav")),
            PlaybackSettings::LOOP.with_spatial(true),
        ));
    }
}

/// [`GlobalVolume`] only applies to sounds started after the change; playing
/// sinks (the hum) are re-leveled here.
fn relevel_sinks(
    settings: Res<GameSettings>,
    mut sinks: Query<&mut AudioSink>,
    mut spatial_sinks: Query<&mut SpatialAudioSink>,
) {
    let volume = Volume::Linear(settings.master_volume);
    for mut sink in &mut sinks {
        sink.set_volume(volume);
    }
    for mut sink in &mut spatial_sinks {
        sink.set_volume(volume);
    }
}
