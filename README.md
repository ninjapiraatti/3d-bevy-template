# 3d-bevy-template

A Bevy template for 3rd-person games: adventure, squad mechanics and strategy.
Desktop only (Windows/macOS/Linux). Levels are authored in Blender and imported
as glTF; characters and animations come from glTF-native asset packs
(see [docs/CHARACTERS.md](docs/CHARACTERS.md)). FBX-only sources such as
Mixamo are not supported — convert them to glTF in Blender yourself if you
need them.

See [ROADMAP.md](ROADMAP.md) for the build plan and current status.

## Running

```sh
cargo run                                      # dev build (dynamic linking, fast rebuilds)
cargo build --release --no-default-features    # shippable build
```

The demo: **New Game** loads the test level. Walk around, drag-select your
squad in top-down mode and order it across the map, pause, save, quit, load —
the full loop works out of the box.

### Controls (demo defaults)

| Input | Third-person mode | Top-down mode (Tab toggles) |
|---|---|---|
| WASD / left stick | Move the player | Pan the camera |
| Mouse wheel | Camera zoom | Camera zoom |
| Right mouse (hold) | Orbit camera | — |
| Left mouse | Order squad to point | Select unit / drag a selection box |
| Right mouse (click) | — | Order selection to point |
| Ctrl + order click | Attack-move (stub) | Attack-move (stub) |
| H | Stop/hold selection | Stop/hold selection |
| Shift | Run | — |
| Esc | Pause menu | Pause menu |
| F1 / F3 | Diagnostics / navmesh overlay | same |

Run, Stop/Hold and the camera toggle are rebindable in Settings; every
binding lives in one input map (`controls.rs`) persisted to `settings.ron`.

## Starting a game from this template

1. **Clone, rename, run.** `cargo run` must show the demo before you change
   anything. The workspace is two crates: `game/` (thin binary — compose the
   plugins you want) and `template_core/` (library — one Bevy plugin per
   concern).
2. **Author a level in Blender** following
   [docs/blender-pipeline.md](docs/blender-pipeline.md): geometry with
   `collider = "trimesh"`, a hidden `marker = "navmesh"` mesh, a
   `player_spawn` empty, and any `npc_spawn` empties (behavior, faction and
   character are per-empty properties — gameplay data lives in Blender, not
   code). Export to `assets/levels/`, point `CurrentLevel` at it.
3. **Bring in characters** from a glTF-native pack per
   [docs/CHARACTERS.md](docs/CHARACTERS.md); the animation controller
   retargets the pack's shared clips onto every character at runtime.
4. **Compose your game** in `game/src/main.rs`: keep the plugins you need,
   drop the ones you don't (see the table below), and add your own alongside.
   Plugins talk through events, states and shared components — no plugin
   reaches into another's internals, so swapping one out is local surgery.
5. **Make gameplay yours.** Factions and hostility are data
   (`FactionRelations`); commands carry `CommandKind` for your combat systems;
   persistence is opt-in per entity (`Save` marker + the allowlist in
   `saves.rs`).

## The plugins

| Plugin (module) | Concern | Notes for downstream games |
|---|---|---|
| `AppStatePlugin` (`states`) | `MainMenu → Loading → InGame ⇄ Paused`, plus the `CameraMode` sub-state | Everything hangs off these states; entities are state-scoped |
| `MenuPlugin` (`menus`) | Main/pause menus, settings screen | Replace wholesale for a real UI; actions are plain components |
| `LevelPlugin` (`levels`) | glTF level loading, Blender marker → component conversion | Add your own markers in `process_gltf_extras` |
| `ControlsPlugin` (`controls`) | The one input map (leafwing-input-manager) | Add actions to `PlayerAction`; bindings are data |
| `SettingsPlugin` (`settings`) | `settings.ron`: window, volume, key rebinds; live apply | Extend `GameSettings` — old files keep loading (`serde(default)`) |
| `SavePlugin` (`saves`) | Versioned save files (moonshine-save), component allowlist | Opt entities in with `Save`; allowlist what persists |
| `GameAudioPlugin` (`audio`) | UI click + positional 3D reference sound | Placeholder WAVs are script-generated (`tools/audio/`) — swap the files |
| `DiagnosticsOverlayPlugin` (`diagnostics`) | F1 overlay: FPS, entity count | — |
| `PlayerPlugin` (`player`) | Physics-capsule character controller (no jump, by scope) | bevy-tnua is the upgrade path if feel demands more |
| `CharacterAnimationPlugin` (`animation`) | Named states (idle/walk/run) over `AnimationGraph`, speed-driven | Pack-agnostic as long as clips address bones by name |
| `ThirdPersonCameraPlugin` (`camera_rig`) | Orbit/follow camera with collision | Persisted rig state; render side rebuilt on load |
| `RtsCameraPlugin` (`rts_camera`) | Top-down camera, Tab toggle | Drives the same camera entity as the third-person rig |
| `NavigationPlugin` (`nav`) | bevy_landmass navmesh islands, order lifecycle | Navmeshes are *authored in Blender*, not generated |
| `NpcPlugin` (`npcs`) | NPC archetype spawned from Blender markers | Character/faction/behavior are per-marker data |
| `NpcAiPlugin` (`npc_ai`) | Hand-rolled FSM: idle/wander/patrol + chase-on-sight | The designated swap point for real BT/utility AI |
| `SquadPlugin` (`squad`) | Selection, rings, move/attack-move/stop orders, formation | Empty selection = whole squad; attack-move is a data stub |
| `DevScenePlugin` (`dev_scene`) | Demo props (spinning cube, animation-reuse rogue) | Drop it in your game |

## Repository layout

- `game/`, `template_core/` — the code (see above)
- `assets/` — what the game loads (levels, characters, audio)
- `assets_src/` — editable Blender sources; never loaded by the game
- `tools/blender/`, `tools/audio/` — asset regeneration scripts
- `docs/` — the Blender pipeline and character-pack guides
- `saves/`, `settings.ron` — written by the demo at runtime (gitignored)
