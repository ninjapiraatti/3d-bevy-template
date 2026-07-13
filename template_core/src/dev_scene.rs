//! Placeholder in-game props: light and a spinning cube (the visible proof
//! that pausing freezes the simulation). Level geometry comes from the glTF
//! level; the camera belongs to the third-person rig.

use bevy::prelude::*;

use crate::states::AppState;

pub struct DevScenePlugin;

impl Plugin for DevScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::InGame), spawn_dev_scene)
            .add_systems(Update, spin.run_if(in_state(AppState::InGame)));
    }
}

#[derive(Component)]
struct Spinner;

fn spawn_dev_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        DespawnOnExit(AppState::InGame),
        Spinner,
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.6, 0.4, 0.8))),
        Transform::from_xyz(0.0, 1.5, 0.0),
    ));
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

/// Driven by virtual time, so pausing visibly freezes the cube — the proof
/// for the roadmap's "simulation frozen while paused" check.
fn spin(time: Res<Time>, mut spinners: Query<&mut Transform, With<Spinner>>) {
    for mut transform in &mut spinners {
        transform.rotate_y(0.9 * time.delta_secs());
    }
}
