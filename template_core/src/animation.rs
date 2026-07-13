//! Character animation: plays clips from shared-rig glTF animation libraries
//! on character scenes, driving idle/walk/run from movement speed.
//!
//! KayKit-style packs ship characters and animations in separate glTF files
//! sharing one rig. Bevy's glTF loader only wires up animation components for
//! files that contain animations, so [`attach_rigs`] reproduces that wiring on
//! the character scene: clips address bones by the hash of their name path
//! from the scene root node down (root inclusive), which both files share.

use std::time::Duration;

use avian3d::prelude::*;
use bevy::animation::{AnimatedBy, AnimationTargetId};
use bevy::gltf::Gltf;
use bevy::prelude::*;

use crate::states::AppState;

pub struct CharacterAnimationPlugin;

impl Plugin for CharacterAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (attach_rigs, drive_locomotion).run_if(in_state(AppState::InGame)),
        );
    }
}

/// Attach next to a character's `WorldAssetRoot` to animate it. The libraries
/// must be built on the same rig as the character (same bone names and
/// hierarchy); clip names are looked up across all libraries.
#[derive(Component, Clone)]
pub struct CharacterAnimations {
    pub libraries: Vec<Handle<Gltf>>,
    pub idle: String,
    pub walk: String,
    pub run: String,
    /// Horizontal speed (m/s) above which idle becomes walk.
    pub walk_threshold: f32,
    /// Horizontal speed (m/s) above which walk becomes run.
    pub run_threshold: f32,
    pub crossfade: Duration,
}

impl CharacterAnimations {
    /// The template's blessed default rig: KayKit Adventurers 2.0
    /// (`Rig_Medium`), shared by every character in the pack.
    pub fn kaykit_adventurer(assets: &AssetServer) -> Self {
        Self {
            libraries: vec![
                assets.load("characters/adventurers/animations/Rig_Medium_General.glb"),
                assets.load("characters/adventurers/animations/Rig_Medium_MovementBasic.glb"),
            ],
            idle: "Idle_A".into(),
            walk: "Walking_A".into(),
            run: "Running_A".into(),
            walk_threshold: 0.5,
            run_threshold: 4.5,
            crossfade: Duration::from_millis(250),
        }
    }
}

/// Added by [`attach_rigs`] once the scene and libraries are loaded.
#[derive(Component)]
struct Locomotion {
    idle: AnimationNodeIndex,
    walk: AnimationNodeIndex,
    run: AnimationNodeIndex,
    current: Option<AnimationNodeIndex>,
}

/// Waits until the character scene has spawned and all animation libraries
/// have loaded, then builds the animation graph and stamps the scene's nodes
/// with animation targets the way the glTF loader would have, had the clips
/// lived in the character's own file.
fn attach_rigs(
    mut commands: Commands,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    gltfs: Res<Assets<Gltf>>,
    pending: Query<(Entity, &CharacterAnimations, &Children), Without<Locomotion>>,
    children: Query<&Children>,
    names: Query<&Name>,
) {
    for (entity, config, scene_roots) in &pending {
        let Some(libraries) = config
            .libraries
            .iter()
            .map(|handle| gltfs.get(handle))
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };

        let find_clip = |clip: &str| {
            libraries
                .iter()
                .find_map(|gltf| gltf.named_animations.get(clip))
                .cloned()
        };
        let (Some(idle), Some(walk), Some(run)) = (
            find_clip(&config.idle),
            find_clip(&config.walk),
            find_clip(&config.run),
        ) else {
            let available: Vec<_> = libraries
                .iter()
                .flat_map(|gltf| gltf.named_animations.keys())
                .collect();
            warn!("animation clips missing, not animating {entity}; libraries have {available:?}");
            commands.entity(entity).remove::<CharacterAnimations>();
            continue;
        };

        // Clip curves address bones by AnimationTargetId: the hash of the
        // Name path from the glTF scene root *node* (inclusive) to the bone.
        // The spawned scene wraps those nodes in one extra entity (the glTF
        // scene itself), which the paths must not include.
        let mut stack: Vec<(Entity, Vec<Name>)> = scene_roots
            .iter()
            .filter_map(|wrapper| children.get(wrapper).ok())
            .flatten()
            .filter_map(|&node| Some((node, vec![names.get(node).ok()?.clone()])))
            .collect();
        while let Some((node, path)) = stack.pop() {
            commands.entity(node).insert((
                AnimationTargetId::from_names(path.iter()),
                AnimatedBy(entity),
            ));
            for &child in children.get(node).into_iter().flatten() {
                if let Ok(name) = names.get(child) {
                    let mut path = path.clone();
                    path.push(name.clone());
                    stack.push((child, path));
                }
            }
        }

        let mut graph = AnimationGraph::new();
        let root = graph.root;
        commands.entity(entity).insert((
            AnimationPlayer::default(),
            AnimationTransitions::new(),
            Locomotion {
                idle: graph.add_clip(idle, 1.0, root),
                walk: graph.add_clip(walk, 1.0, root),
                run: graph.add_clip(run, 1.0, root),
                current: None,
            },
            AnimationGraphHandle(graphs.add(graph)),
        ));
    }
}

fn drive_locomotion(
    mut characters: Query<(
        Entity,
        &CharacterAnimations,
        &mut Locomotion,
        &mut AnimationPlayer,
        &mut AnimationTransitions,
    )>,
    velocities: Query<&LinearVelocity>,
    parents: Query<&ChildOf>,
) {
    for (entity, config, mut locomotion, mut player, mut transitions) in &mut characters {
        let speed = horizontal_speed(entity, &velocities, &parents);
        let target = if speed < config.walk_threshold {
            locomotion.idle
        } else if speed < config.run_threshold {
            locomotion.walk
        } else {
            locomotion.run
        };
        if locomotion.current != Some(target) {
            locomotion.current = Some(target);
            transitions
                .play(&mut player, target, config.crossfade)
                .repeat();
        }
    }
}

/// The physics body owning the velocity may be an ancestor (the model scene
/// is a child of the collider capsule); bodiless characters count as idle.
fn horizontal_speed(
    entity: Entity,
    velocities: &Query<&LinearVelocity>,
    parents: &Query<&ChildOf>,
) -> f32 {
    let mut current = entity;
    loop {
        if let Ok(velocity) = velocities.get(current) {
            return Vec2::new(velocity.x, velocity.z).length();
        }
        match parents.get(current) {
            Ok(child_of) => current = child_of.parent(),
            Err(_) => return 0.0,
        }
    }
}
