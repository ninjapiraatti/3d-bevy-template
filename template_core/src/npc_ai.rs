//! NPC AI: a hand-rolled behavior state machine (idle / wander / patrol,
//! chase-when-spotted) with faction-driven perception.
//!
//! Behaviors are data: which behavior an NPC runs comes from Blender
//! `npc_spawn` properties (see `levels.rs`), factions are free-form strings,
//! and hostility lives in the [`FactionRelations`] resource. Downstream games
//! with heavier AI needs swap this plugin out (see ROADMAP step 7 decision).

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_landmass::AgentState;
use bevy_landmass::prelude::*;

use crate::levels::Waypoint;
use crate::nav::Commanded;
use crate::npcs::Npc;
use crate::player::{RUN_SPEED, WALK_SPEED};
use crate::states::PauseState;

pub struct NpcAiPlugin;

/// Which side an entity is on. Free-form names, authored in Blender for NPCs;
/// the player is [`PLAYER_FACTION`].
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct Faction(pub String);

pub const PLAYER_FACTION: &str = "player";

/// Directional hostility between factions: an `(aggressor, victim)` pair
/// means aggressor-faction NPCs chase victim-faction entities on sight.
/// Sides are data — mutate this resource to define them per game.
#[derive(Resource, Debug)]
pub struct FactionRelations {
    hostile: Vec<(String, String)>,
}

impl Default for FactionRelations {
    fn default() -> Self {
        // The template's demo sides: raiders attack the player's faction.
        Self {
            hostile: vec![("raiders".into(), PLAYER_FACTION.into())],
        }
    }
}

impl FactionRelations {
    pub fn is_hostile(&self, aggressor: &Faction, victim: &Faction) -> bool {
        self.hostile
            .iter()
            .any(|(a, v)| *a == aggressor.0 && *v == victim.0)
    }

    pub fn declare_hostile(&mut self, aggressor: &str, victim: &str) {
        self.hostile.push((aggressor.into(), victim.into()));
    }
}

/// What an NPC does when not aggroed. Swapping this component swaps the
/// behavior; no other code knows which behavior an entity runs.
#[derive(Component, Clone, Debug, PartialEq)]
pub enum NpcBehavior {
    Idle,
    /// Walks to successive points around `home`.
    Wander {
        home: Vec3,
        radius: f32,
    },
    /// Loops over the level's `marker = "waypoint"` empties of this route.
    Patrol {
        route: String,
    },
}

const WANDER_RADIUS: f32 = 5.0;

impl NpcBehavior {
    /// Resolves Blender `npc_spawn` properties; unknown behavior strings warn
    /// and fall back to idle so a typo is visible, not a crash.
    pub fn from_spawn(behavior: Option<&str>, route: Option<&str>, home: Vec3) -> Self {
        match behavior {
            None | Some("idle") => Self::Idle,
            Some("wander") => Self::Wander {
                home,
                radius: WANDER_RADIUS,
            },
            Some("patrol") => Self::Patrol {
                route: route.unwrap_or("A").to_string(),
            },
            Some(other) => {
                warn!("unknown behavior '{other}' on an npc_spawn, using idle");
                Self::Idle
            }
        }
    }
}

/// Sight cone and range. Aggro requires the target inside the cone, in
/// range, and visible via a line-of-sight raycast (walls block perception).
#[derive(Component, Clone, Debug)]
pub struct Perception {
    pub range: f32,
    /// Half-angle of the sight cone, radians, measured around entity forward.
    pub half_angle: f32,
}

impl Default for Perception {
    fn default() -> Self {
        Self {
            range: 8.0,
            half_angle: 50_f32.to_radians(),
        }
    }
}

/// Aggro: chasing a spotted hostile. Removed once the target stays unseen
/// long enough, after which the NPC's [`NpcBehavior`] resumes.
#[derive(Component)]
pub struct Chasing {
    pub target: Entity,
    unseen: Timer,
}

/// How long a chased target must stay unseen before the NPC gives up.
const LOSE_SIGHT_SECS: f32 = 2.5;
/// While already chasing, the cone no longer applies and range gets this
/// slack, so a target on the range edge doesn't flicker aggro on and off.
const CHASE_RANGE_SLACK: f32 = 1.5;
/// Eye offset above the capsule center, where sight rays start.
const EYE_HEIGHT: f32 = 0.5;

/// Index of the next waypoint within a patrol route.
#[derive(Component, Default)]
struct PatrolProgress {
    next: usize,
}

/// Counts wander legs; feeds the golden-angle point sequence.
#[derive(Component, Default)]
struct WanderProgress {
    leg: u32,
}

impl Plugin for NpcAiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FactionRelations>().add_systems(
            Update,
            (perceive, drive_behaviors)
                .chain()
                .run_if(in_state(PauseState::Running)),
        );
    }
}

/// Aggro and de-aggro. Perceivers scan for the nearest visible entity of a
/// faction they are hostile toward; an aggroed NPC keeps its target until it
/// stays unseen for [`LOSE_SIGHT_SECS`].
fn perceive(
    mut commands: Commands,
    time: Res<Time>,
    relations: Res<FactionRelations>,
    spatial: SpatialQuery,
    mut perceivers: Query<
        (
            Entity,
            &Transform,
            &Faction,
            &Perception,
            Option<&mut Chasing>,
        ),
        With<Npc>,
    >,
    targets: Query<(Entity, &GlobalTransform, &Faction)>,
) {
    for (npc, transform, faction, perception, chasing) in &mut perceivers {
        let eye = transform.translation + Vec3::Y * EYE_HEIGHT;
        let sees = |target: Entity, target_pos: Vec3| {
            line_of_sight(&spatial, npc, eye, target, target_pos)
        };

        if let Some(mut chasing) = chasing {
            let visible = targets.get(chasing.target).is_ok_and(|(target, tf, _)| {
                let target_pos = tf.translation();
                target_pos.distance(transform.translation) <= perception.range * CHASE_RANGE_SLACK
                    && sees(target, target_pos)
            });
            if visible {
                chasing.unseen.reset();
            } else {
                chasing.unseen.tick(time.delta());
                if chasing.unseen.is_finished() {
                    info!("npc {npc} lost its target, resuming normal behavior");
                    commands.entity(npc).remove::<Chasing>();
                }
            }
            continue;
        }

        let spotted = targets
            .iter()
            .filter(|(target, _, target_faction)| {
                *target != npc && relations.is_hostile(faction, target_faction)
            })
            .filter_map(|(target, tf, _)| {
                let target_pos = tf.translation();
                let to_target = target_pos - transform.translation;
                (to_target.length() <= perception.range
                    && within_cone(
                        transform.forward().as_vec3(),
                        to_target,
                        perception.half_angle,
                    )
                    && sees(target, target_pos))
                .then_some((target, to_target.length()))
            })
            .min_by(|(_, a), (_, b)| a.total_cmp(b));
        if let Some((target, _)) = spotted {
            info!("npc {npc} spotted {target}, chasing");
            commands.entity(npc).insert(Chasing {
                target,
                unseen: Timer::from_seconds(LOSE_SIGHT_SECS, TimerMode::Once),
            });
        }
    }
}

/// True when nothing solid stands between the eye and the target (the ray
/// hitting the target itself counts as seeing it).
fn line_of_sight(
    spatial: &SpatialQuery,
    npc: Entity,
    eye: Vec3,
    target: Entity,
    target_pos: Vec3,
) -> bool {
    let Ok(direction) = Dir3::new(target_pos - eye) else {
        return true; // standing inside each other
    };
    let filter = SpatialQueryFilter::default().with_excluded_entities([npc]);
    spatial
        .cast_ray(
            eye,
            direction,
            target_pos.distance(eye) + 0.1,
            true,
            &filter,
        )
        .is_none_or(|hit| hit.entity == target)
}

/// Cone test on the ground plane, so height differences (a target up a ramp)
/// don't rotate it out of a level sight cone.
fn within_cone(forward: Vec3, to_target: Vec3, half_angle: f32) -> bool {
    let forward = Vec3::new(forward.x, 0.0, forward.z);
    let to_target = Vec3::new(to_target.x, 0.0, to_target.z);
    // angle_between is NaN for zero-length vectors; NaN fails the comparison,
    // which reads as "not in the cone" — the safe answer for both.
    forward.angle_between(to_target) <= half_angle
}

/// The state machine's output: each NPC's landmass target and speed. Chasing
/// overrides the assigned behavior; a player move order (`Commanded`) owns
/// the target until fulfilled. Walk on behavior, run on chase, which is what
/// the animation controller turns into walk/run clips.
fn drive_behaviors(
    mut commands: Commands,
    mut npcs: Query<
        (
            Entity,
            &NpcBehavior,
            &mut AgentTarget3d,
            &mut AgentSettings,
            &AgentState,
            Option<&Chasing>,
            Option<&Commanded>,
            Option<&mut PatrolProgress>,
            Option<&mut WanderProgress>,
        ),
        With<Npc>,
    >,
    waypoints: Query<(&Waypoint, &GlobalTransform)>,
) {
    for (npc, behavior, mut target, mut settings, state, chasing, commanded, patrol, wander) in
        &mut npcs
    {
        if let Some(chasing) = chasing {
            settings.desired_speed = RUN_SPEED;
            *target = AgentTarget3d::Entity(chasing.target);
            continue;
        }
        settings.desired_speed = WALK_SPEED;
        if commanded.is_some() {
            continue;
        }

        match behavior {
            NpcBehavior::Idle => *target = AgentTarget3d::None,
            NpcBehavior::Wander { home, radius } => {
                let Some(mut progress) = wander else {
                    commands.entity(npc).insert(WanderProgress::default());
                    continue;
                };
                // A fresh spawn or a finished chase leaves a stale target.
                let needs_point = !matches!(*target, AgentTarget3d::Point(_))
                    || *state == AgentState::ReachedTarget;
                if needs_point {
                    *target = AgentTarget3d::Point(wander_point(*home, *radius, progress.leg));
                    progress.leg = progress.leg.wrapping_add(1);
                }
            }
            NpcBehavior::Patrol { route } => {
                let Some(mut progress) = patrol else {
                    commands.entity(npc).insert(PatrolProgress::default());
                    continue;
                };
                let mut route_points: Vec<_> = waypoints
                    .iter()
                    .filter(|(waypoint, _)| waypoint.route == *route)
                    .collect();
                if route_points.is_empty() {
                    *target = AgentTarget3d::None;
                    continue;
                }
                route_points.sort_by(|(a, _), (b, _)| a.order.total_cmp(&b.order));
                if *state == AgentState::ReachedTarget {
                    progress.next += 1;
                }
                progress.next %= route_points.len();
                *target = AgentTarget3d::Point(route_points[progress.next].1.translation());
            }
        }
    }
}

/// Golden-angle sequence around `home`: spreads points evenly over the disc,
/// looks aimless enough for a wander demo, and stays deterministic (and
/// dependency-free) for tests.
fn wander_point(home: Vec3, radius: f32, leg: u32) -> Vec3 {
    const GOLDEN_ANGLE: f32 = 2.399_963;
    let angle = leg as f32 * GOLDEN_ANGLE;
    let distance = radius * (0.35 + 0.65 * (leg as f32 * 0.618_034).fract());
    home + Vec3::new(angle.cos(), 0.0, angle.sin()) * distance
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostility_is_directional() {
        let relations = FactionRelations::default();
        let raiders = Faction("raiders".into());
        let player = Faction(PLAYER_FACTION.into());
        assert!(relations.is_hostile(&raiders, &player));
        assert!(!relations.is_hostile(&player, &raiders));
        assert!(!relations.is_hostile(&player, &player));
    }

    #[test]
    fn declared_hostility_applies() {
        let mut relations = FactionRelations::default();
        relations.declare_hostile("player", "raiders");
        assert!(relations.is_hostile(&Faction("player".into()), &Faction("raiders".into())));
    }

    #[test]
    fn cone_accepts_ahead_rejects_behind() {
        let half = 50_f32.to_radians();
        assert!(within_cone(Vec3::NEG_Z, Vec3::NEG_Z * 5.0, half));
        assert!(!within_cone(Vec3::NEG_Z, Vec3::Z * 5.0, half));
        // 45° off-forward is inside a 50° half-angle cone…
        assert!(within_cone(Vec3::NEG_Z, Vec3::new(1.0, 0.0, -1.0), half));
        // …but 60° is not.
        let at_60 = Vec3::new(60_f32.to_radians().sin(), 0.0, -60_f32.to_radians().cos());
        assert!(!within_cone(Vec3::NEG_Z, at_60, half));
    }

    #[test]
    fn cone_ignores_height_difference() {
        let half = 50_f32.to_radians();
        assert!(within_cone(Vec3::NEG_Z, Vec3::new(0.0, 3.0, -5.0), half));
        // Directly overhead has no planar direction: not visible.
        assert!(!within_cone(Vec3::NEG_Z, Vec3::Y * 3.0, half));
    }

    #[test]
    fn unknown_behavior_string_falls_back_to_idle() {
        let behavior = NpcBehavior::from_spawn(Some("berserk"), None, Vec3::ZERO);
        assert_eq!(behavior, NpcBehavior::Idle);
        let patrol = NpcBehavior::from_spawn(Some("patrol"), Some("B"), Vec3::ZERO);
        assert_eq!(patrol, NpcBehavior::Patrol { route: "B".into() });
    }

    #[test]
    fn wander_points_stay_within_radius() {
        let home = Vec3::new(3.0, 0.0, -2.0);
        for leg in 0..100 {
            let point = wander_point(home, 5.0, leg);
            assert!(point.distance(home) <= 5.0 + 1e-4);
            assert!(point.distance(home) >= 0.3, "degenerate leg {leg}");
            assert_eq!(point.y, home.y);
        }
    }
}
