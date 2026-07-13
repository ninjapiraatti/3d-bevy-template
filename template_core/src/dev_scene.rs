//! Placeholder in-game props: light, a spinning cube (the visible proof
//! that pausing freezes the simulation), and an idle Rogue (the proof that a
//! second character reuses the player's animation set). Level geometry comes
//! from the glTF level; the camera belongs to the third-person rig.

use bevy::prelude::*;
use moonshine_save::prelude::Save;

use crate::animation::CharacterAnimations;
use crate::levels::PlayerSpawn;
use crate::states::AppState;

pub struct DevScenePlugin;

impl Plugin for DevScenePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Spinner>()
            .add_systems(OnEnter(AppState::InGame), spawn_dev_scene)
            .add_systems(
                Update,
                (spin, hydrate_spinner, spawn_reuse_demo).run_if(in_state(AppState::InGame)),
            );
    }
}

/// The cube's rotation is the roadmap's "one piece of arbitrary gameplay
/// state" in save files: saving mid-spin and loading must resume the angle.
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
#[require(Save)]
pub(crate) struct Spinner;

fn spawn_dev_scene(mut commands: Commands, spinners: Query<(), With<Spinner>>) {
    // A loaded save may have brought the spinner along already.
    if spinners.is_empty() {
        commands.spawn((
            Name::new("Spinner"),
            Spinner,
            Transform::from_xyz(0.0, 1.5, 0.0),
        ));
    }
    commands.spawn((
        DespawnOnExit(AppState::InGame),
        DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, 0.4, 0.0)),
    ));
}

/// Rebuilds the cube's visuals; never part of save data.
fn hydrate_spinner(
    mut commands: Commands,
    spinners: Query<Entity, (With<Spinner>, Without<Mesh3d>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for entity in &spinners {
        commands.entity(entity).insert((
            DespawnOnExit(AppState::InGame),
            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            MeshMaterial3d(materials.add(Color::srgb(0.6, 0.4, 0.8))),
        ));
    }
}

#[derive(Component)]
struct ReuseDemo;

/// Waits for the level's [`PlayerSpawn`] marker (like the player does), then
/// places a second KayKit character beside it, sharing the player's animation
/// libraries — the roadmap step 4 "second character, no code changes" check.
fn spawn_reuse_demo(
    mut commands: Commands,
    demos: Query<(), With<ReuseDemo>>,
    spawn_points: Query<&GlobalTransform, With<PlayerSpawn>>,
    assets: Res<AssetServer>,
) {
    if !demos.is_empty() {
        return;
    }
    let Ok(spawn) = spawn_points.single() else {
        return;
    };
    commands.spawn((
        Name::new("Rogue (animation reuse demo)"),
        ReuseDemo,
        DespawnOnExit(AppState::InGame),
        WorldAssetRoot(
            assets.load(GltfAssetLabel::Scene(0).from_asset("characters/adventurers/Rogue.glb")),
        ),
        CharacterAnimations::kaykit_adventurer(&assets),
        Transform::from_translation(spawn.translation() + Vec3::X * 2.0),
    ));
}

/// Driven by virtual time, so pausing visibly freezes the cube — the proof
/// for the roadmap's "simulation frozen while paused" check.
fn spin(time: Res<Time>, mut spinners: Query<&mut Transform, With<Spinner>>) {
    for mut transform in &mut spinners {
        transform.rotate_y(0.9 * time.delta_secs());
    }
}
