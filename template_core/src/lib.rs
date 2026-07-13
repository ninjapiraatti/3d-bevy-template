//! Core plugins for the 3rd-person game template.
//!
//! One Bevy plugin per concern; games compose the plugins they need.

use bevy::prelude::*;

/// Minimal scene used to verify the scaffold: a ground plane, a directional
/// light and a camera. Later roadmap steps replace this with glTF levels.
pub struct DevScenePlugin;

impl Plugin for DevScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_dev_scene);
    }
}

fn spawn_dev_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.35, 0.5, 0.35),
            perceptual_roughness: 1.0,
            ..default()
        })),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, 0.4, 0.0)),
    ));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-6.0, 6.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
