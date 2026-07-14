//! Squad layer: unit selection and player-issued orders.
//!
//! Selection (click or drag-box, top-down mode only) marks friendly units
//! [`Selected`] and gives each a ground ring, so selection state is always
//! visible. Orders — move, attack-move, stop/hold — go to the selected units;
//! an empty selection addresses the whole squad, which is also what the
//! third-person click-to-move uses. Group moves spread destinations in a
//! ring formation around the clicked point (snapped to the navmesh) so units
//! don't pile onto one spot; landmass local avoidance handles the traffic
//! on the way.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_landmass::PointSampleDistance3d;
use bevy_landmass::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::controls::PlayerAction;
use crate::nav::{CommandKind, Commandable, Commanded, Hold};
use crate::npcs::{CAPSULE_LENGTH, CAPSULE_RADIUS};
use crate::player::Player;
use crate::states::{AppState, CameraMode, PauseState};

pub struct SquadPlugin;

/// Units in the player's current selection. Orders address this set.
#[derive(Component)]
pub struct Selected;

/// The ground ring child of a selected unit.
#[derive(Component)]
struct SelectionRing;

/// The drag-box UI rectangle while a drag-select is in progress.
#[derive(Component)]
struct DragRect;

/// An in-progress left-mouse press in top-down mode: where it started and,
/// once it travels past [`DRAG_THRESHOLD`], the UI rectangle showing it.
#[derive(Resource, Default)]
struct DragSelect {
    start: Option<Vec2>,
    rect: Option<Entity>,
}

const SELECTION_COLOR: Color = Color::srgb(0.3, 1.0, 0.45);
/// Cursor travel (logical px) below which a press-release is a click select.
const DRAG_THRESHOLD: f32 = 6.0;
/// Clicks are ray-cast this far into the world.
const COMMAND_RAY_LENGTH: f32 = 250.0;
/// Distance between formation slots; comfortably over two agent radii.
const FORMATION_SPACING: f32 = 1.4;

impl Plugin for SquadPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectionAssets>()
            .init_resource::<DragSelect>()
            .add_systems(
                Update,
                drag_select
                    .run_if(in_state(PauseState::Running).and_then(in_state(CameraMode::TopDown))),
            )
            .add_systems(
                Update,
                (issue_move_commands, stop_hold).run_if(in_state(PauseState::Running)),
            )
            .add_systems(
                Update,
                (add_rings, remove_rings).run_if(in_state(AppState::InGame)),
            );
    }
}

/// Shared mesh/material for selection rings.
#[derive(Resource)]
struct SelectionAssets {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

impl FromWorld for SelectionAssets {
    fn from_world(world: &mut World) -> Self {
        let mesh = world.resource_mut::<Assets<Mesh>>().add(Torus {
            minor_radius: 0.05,
            major_radius: 0.55,
        });
        let material = world
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color: SELECTION_COLOR,
                unlit: true,
                ..default()
            });
        Self { mesh, material }
    }
}

/// Left mouse in top-down mode: a short press-release click-selects the unit
/// under the cursor; a drag draws a box and selects every friendly unit whose
/// position projects inside it. Either way the result replaces the selection.
#[allow(clippy::too_many_arguments)]
fn drag_select(
    mut commands: Commands,
    mut drag: ResMut<DragSelect>,
    actions: Query<&ActionState<PlayerAction>, With<Player>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    spatial: SpatialQuery,
    mut rects: Query<&mut Node, With<DragRect>>,
    units: Query<(Entity, &GlobalTransform), With<Commandable>>,
    selected: Query<Entity, With<Selected>>,
) {
    let (Ok(actions), Ok(window)) = (actions.single(), windows.single()) else {
        return;
    };

    if actions.just_pressed(&PlayerAction::Select) {
        drag.start = window.cursor_position();
    }
    let Some(start) = drag.start else {
        return;
    };

    if actions.pressed(&PlayerAction::Select) {
        let Some(cursor) = window.cursor_position() else {
            return;
        };
        if drag.rect.is_none() && cursor.distance(start) > DRAG_THRESHOLD {
            drag.rect = Some(commands.spawn(drag_rect()).id());
        }
        if let Some(rect) = drag.rect
            && let Ok(mut node) = rects.get_mut(rect)
        {
            let min = start.min(cursor);
            let size = (start - cursor).abs();
            node.left = px(min.x);
            node.top = px(min.y);
            node.width = px(size.x);
            node.height = px(size.y);
        }
        return;
    }

    if !actions.just_released(&PlayerAction::Select) {
        return;
    }
    let dragging = drag.rect.is_some();
    if let Some(rect) = drag.rect.take() {
        commands.entity(rect).despawn();
    }
    drag.start = None;

    let (Some(cursor), Ok((camera, camera_transform))) =
        (window.cursor_position(), cameras.single())
    else {
        return;
    };
    let picked: Vec<Entity> = if dragging {
        let rect = Rect::from_corners(start, cursor);
        units
            .iter()
            .filter(|(_, transform)| {
                camera
                    .world_to_viewport(camera_transform, transform.translation())
                    .is_ok_and(|viewport| rect.contains(viewport))
            })
            .map(|(unit, _)| unit)
            .collect()
    } else {
        cursor_ray_hit(camera, camera_transform, cursor, &spatial)
            .map(|hit| hit.0)
            .filter(|&hit| units.contains(hit))
            .into_iter()
            .collect()
    };

    for unit in &selected {
        if !picked.contains(&unit) {
            commands.entity(unit).remove::<Selected>();
        }
    }
    for unit in picked {
        if !selected.contains(unit) {
            commands.entity(unit).insert(Selected);
        }
    }
}

fn drag_rect() -> impl Bundle {
    (
        Name::new("DragSelectRect"),
        DragRect,
        Node {
            position_type: PositionType::Absolute,
            border: UiRect::all(px(1)),
            ..default()
        },
        BackgroundColor(SELECTION_COLOR.with_alpha(0.08)),
        BorderColor::all(SELECTION_COLOR.with_alpha(0.8)),
        GlobalZIndex(10),
        // Also despawns a mid-drag rectangle if the mode toggles away.
        DespawnOnExit(CameraMode::TopDown),
    )
}

/// Casts a ray from the cursor into the world; returns the hit entity and point.
fn cursor_ray_hit(
    camera: &Camera,
    camera_transform: &GlobalTransform,
    cursor: Vec2,
    spatial: &SpatialQuery,
) -> Option<(Entity, Vec3)> {
    let ray = camera.viewport_to_world(camera_transform, cursor).ok()?;
    let hit = spatial.cast_ray(
        ray.origin,
        ray.direction,
        COMMAND_RAY_LENGTH,
        true,
        &SpatialQueryFilter::default(),
    )?;
    Some((hit.entity, ray.origin + *ray.direction * hit.distance))
}

/// Sends units to a clicked point: right click in top-down mode, left click
/// in third person (both address the selection; empty selection = whole
/// squad). Held [`PlayerAction::AttackModifier`] issues the attack-move stub
/// instead — same movement, but the order's [`CommandKind`] records the
/// intent for downstream combat systems.
#[allow(clippy::too_many_arguments)]
fn issue_move_commands(
    mut commands: Commands,
    mode: Res<State<CameraMode>>,
    actions: Query<&ActionState<PlayerAction>, With<Player>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    spatial: SpatialQuery,
    archipelagos: Query<&Archipelago3d>,
    selected: Query<(Entity, &GlobalTransform), (With<Commandable>, With<Selected>)>,
    all_units: Query<(Entity, &GlobalTransform), With<Commandable>>,
    mut targets: Query<&mut AgentTarget3d>,
) {
    let Ok(actions) = actions.single() else {
        return;
    };
    let order_action = match mode.get() {
        CameraMode::ThirdPerson => PlayerAction::CommandMove,
        CameraMode::TopDown => PlayerAction::Command,
    };
    if !actions.just_pressed(&order_action) {
        return;
    }
    let (Ok(window), Ok((camera, camera_transform))) = (windows.single(), cameras.single()) else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Some((_, point)) = cursor_ray_hit(camera, camera_transform, cursor, &spatial) else {
        return;
    };
    let kind = if actions.pressed(&PlayerAction::AttackModifier) {
        CommandKind::AttackMove
    } else {
        CommandKind::Move
    };

    let mut units: Vec<(Entity, Vec3)> = if selected.is_empty() {
        all_units
            .iter()
            .map(|(e, t)| (e, t.translation()))
            .collect()
    } else {
        selected.iter().map(|(e, t)| (e, t.translation())).collect()
    };
    if units.is_empty() {
        return;
    }
    // Offsets are generated center-out; giving the nearest unit the
    // centermost slot keeps paths from crossing more than they must.
    units.sort_by(|(_, a), (_, b)| {
        a.distance_squared(point)
            .total_cmp(&b.distance_squared(point))
    });

    info!("{kind:?} command to {point} for {} unit(s)", units.len());
    let archipelago = archipelagos.single().ok();
    let offsets = formation_offsets(units.len(), FORMATION_SPACING);
    for ((unit, _), offset) in units.into_iter().zip(offsets) {
        let destination = snap_to_navmesh(archipelago, point + Vec3::new(offset.x, 0.0, offset.y));
        if let Ok(mut target) = targets.get_mut(unit) {
            *target = AgentTarget3d::Point(destination);
        }
        commands
            .entity(unit)
            .insert(Commanded { destination, kind })
            .remove::<Hold>();
    }
}

/// Stop/hold order (H): the selection (empty selection = whole squad) drops
/// any order in flight and stands its ground until the next command.
fn stop_hold(
    mut commands: Commands,
    actions: Query<&ActionState<PlayerAction>, With<Player>>,
    selected: Query<Entity, (With<Commandable>, With<Selected>)>,
    all_units: Query<Entity, With<Commandable>>,
    mut targets: Query<&mut AgentTarget3d>,
) {
    let Ok(actions) = actions.single() else {
        return;
    };
    if !actions.just_pressed(&PlayerAction::StopHold) {
        return;
    }
    let units: Vec<Entity> = if selected.is_empty() {
        all_units.iter().collect()
    } else {
        selected.iter().collect()
    };
    info!("hold command for {} unit(s)", units.len());
    for unit in units {
        if let Ok(mut target) = targets.get_mut(unit) {
            *target = AgentTarget3d::None;
        }
        commands.entity(unit).remove::<Commanded>().insert(Hold);
    }
}

/// Formation slots around the order point: one at the center, then rings of
/// six slots per ring of radius, until `count` slots exist. Center-out order.
fn formation_offsets(count: usize, spacing: f32) -> Vec<Vec2> {
    let mut offsets = vec![Vec2::ZERO];
    let mut ring = 1;
    while offsets.len() < count {
        let slots = 6 * ring;
        let radius = ring as f32 * spacing;
        for slot in 0..slots {
            if offsets.len() >= count {
                break;
            }
            let angle = std::f32::consts::TAU * slot as f32 / slots as f32;
            offsets.push(Vec2::new(angle.cos(), angle.sin()) * radius);
        }
        ring += 1;
    }
    offsets.truncate(count);
    offsets
}

/// Moves a formation slot that landed off the navmesh (inside an obstacle
/// margin, over a ledge) to the nearest walkable point; unsampleable points
/// pass through unchanged and simply leave that unit's order unfulfillable.
fn snap_to_navmesh(archipelago: Option<&Archipelago3d>, point: Vec3) -> Vec3 {
    let sample_distance = PointSampleDistance3d {
        horizontal_distance: FORMATION_SPACING * 1.5,
        distance_above: 1.0,
        distance_below: 2.0,
        vertical_preference_ratio: 2.0,
        animation_link_max_vertical_distance: 0.5,
    };
    archipelago
        .and_then(|archipelago| archipelago.sample_point(point, &sample_distance).ok())
        .map(|sampled| sampled.point())
        .unwrap_or(point)
}

fn add_rings(
    mut commands: Commands,
    assets: Res<SelectionAssets>,
    fresh: Query<Entity, Added<Selected>>,
) {
    for unit in &fresh {
        commands.entity(unit).with_children(|parent| {
            parent.spawn((
                Name::new("SelectionRing"),
                SelectionRing,
                Mesh3d(assets.mesh.clone()),
                MeshMaterial3d(assets.material.clone()),
                // Unit origin is the capsule center; the ring sits just above
                // the ground under its feet.
                Transform::from_xyz(0.0, -(CAPSULE_LENGTH / 2.0 + CAPSULE_RADIUS) + 0.05, 0.0),
            ));
        });
    }
}

fn remove_rings(
    mut commands: Commands,
    mut deselected: RemovedComponents<Selected>,
    rings: Query<(Entity, &ChildOf), With<SelectionRing>>,
) {
    for unit in deselected.read() {
        for (ring, child_of) in &rings {
            if child_of.parent() == unit {
                commands.entity(ring).despawn();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formation_matches_count_and_starts_at_center() {
        for count in [1, 2, 6, 7, 19, 20] {
            let offsets = formation_offsets(count, 1.4);
            assert_eq!(offsets.len(), count);
            assert_eq!(offsets[0], Vec2::ZERO);
        }
    }

    #[test]
    fn formation_slots_keep_their_distance() {
        let spacing = 1.4;
        let offsets = formation_offsets(20, spacing);
        for (i, a) in offsets.iter().enumerate() {
            for b in offsets.iter().skip(i + 1) {
                // Ring geometry: same-ring neighbors sit exactly one chord
                // apart, which for 6k slots per ring is at least the spacing.
                assert!(
                    a.distance(*b) > spacing * 0.9,
                    "slots {a} and {b} are too close"
                );
            }
        }
    }

    #[test]
    fn formation_grows_outward() {
        let offsets = formation_offsets(19, 1.0);
        let radii: Vec<f32> = offsets.iter().map(|o| o.length()).collect();
        // Center slot, then ring 1 (6 slots at r=1), then ring 2 (12 at r=2).
        assert_eq!(radii[0], 0.0);
        assert!(radii[1..7].iter().all(|r| (r - 1.0).abs() < 1e-4));
        assert!(radii[7..19].iter().all(|r| (r - 2.0).abs() < 1e-4));
    }
}
