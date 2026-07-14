//! NPC archetype: spawned from Blender `marker = "npc_spawn"` empties, moved
//! by landmass agents (see `nav.rs`), animated like the player.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_landmass::prelude::*;

use crate::animation::CharacterAnimations;
use crate::levels::NpcSpawn;
use crate::nav::Commandable;
use crate::npc_ai::{Faction, NpcBehavior, PLAYER_FACTION, Perception};
use crate::player::{RUN_SPEED, WALK_SPEED};
use crate::states::AppState;

pub struct NpcPlugin;

#[derive(Component)]
pub struct Npc;

/// Marks an [`NpcSpawn`] that already produced its NPC.
#[derive(Component)]
struct NpcSpawned;

const CAPSULE_RADIUS: f32 = 0.35;
const CAPSULE_LENGTH: f32 = 1.0;

impl Plugin for NpcPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_npcs.run_if(in_state(AppState::InGame)));
    }
}

/// Spawn markers and the archipelago both appear asynchronously; retries
/// until both exist, then spawns each NPC once.
fn spawn_npcs(
    mut commands: Commands,
    spawns: Query<(Entity, &NpcSpawn, &GlobalTransform), Without<NpcSpawned>>,
    archipelagos: Query<Entity, With<Archipelago3d>>,
    assets: Res<AssetServer>,
) {
    let Ok(archipelago) = archipelagos.single() else {
        return;
    };
    for (marker, config, spawn) in &spawns {
        commands.entity(marker).insert(NpcSpawned);
        let position =
            spawn.translation() + Vec3::Y * (CAPSULE_LENGTH / 2.0 + CAPSULE_RADIUS + 0.1);
        let faction = config
            .faction
            .clone()
            .unwrap_or_else(|| PLAYER_FACTION.into());
        let character = config.character.as_deref().unwrap_or("Barbarian");
        let mut npc = commands.spawn((
            Name::new(format!("Npc ({character})")),
            Npc,
            DespawnOnExit(AppState::InGame),
            Transform::from_translation(position),
            Visibility::default(),
            RigidBody::Dynamic,
            Collider::capsule(CAPSULE_RADIUS, CAPSULE_LENGTH),
            LockedAxes::ROTATION_LOCKED,
            Friction::new(0.3),
            Agent3dBundle {
                agent: default(),
                settings: AgentSettings {
                    radius: CAPSULE_RADIUS,
                    desired_speed: WALK_SPEED,
                    max_speed: RUN_SPEED,
                },
                archipelago_ref: ArchipelagoRef3d::new(archipelago),
            },
            NpcBehavior::from_spawn(
                config.behavior.as_deref(),
                config.route.as_deref(),
                position,
            ),
            Perception::default(),
            Faction(faction.clone()),
        ));
        if faction == PLAYER_FACTION {
            npc.insert(Commandable);
        }
        npc.with_children(|parent| {
            // Same conventions as the player model (feet origin, +Z rig).
            parent.spawn((
                Name::new("NpcModel"),
                WorldAssetRoot(
                    assets.load(
                        GltfAssetLabel::Scene(0)
                            .from_asset(format!("characters/adventurers/{character}.glb")),
                    ),
                ),
                CharacterAnimations::kaykit_adventurer(&assets),
                Transform::from_xyz(0.0, -(CAPSULE_LENGTH / 2.0 + CAPSULE_RADIUS), 0.0)
                    .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
            ));
        });
        info!("npc '{character}' ({faction}) spawned at {position}");
    }
}
