# Roadmap

A Bevy template for 3rd-person games: adventure, squad mechanics, and strategy.
Not aimed at platformers or fast-paced FPS.

Each step below ends in something runnable and observable. A step is **done**
when its "Verify" checklist passes — not before, and nothing from a later step
should be started while the current step's checklist fails.

Engine: **Bevy 0.19** (desktop: Windows/macOS/Linux only).
Asset pipeline: **Blender → glTF → Bevy**, with glTF-native animated packs
(KayKit, Quaternius, Kenney) as the primary character/animation source.

---

## Step 0 — Workspace scaffold

Cargo workspace, window opens, tooling in place.

- Cargo workspace: a thin `game/` binary crate and a `template_core/` library
  crate that games are built from (one Bevy plugin per concern lives here).
- Bevy 0.19 with dynamic linking in dev profile for fast iteration.
- `rustfmt.toml`, clippy clean, `.gitignore` (target/, Blender backup files).
- README stub explaining the template's scope.

**Verify**

- [x] `cargo run` opens a window with a flat ground plane, a light, and a camera.
- [x] `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass.
- [x] Incremental rebuild after a one-line change is under ~5 s. (0.89 s measured)

## Step 1 — App states and menu shell

Game-state plumbing everything else hangs off.

- `AppState`: `MainMenu` → `Loading` → `InGame` ⇄ `Paused`.
- Main menu (New Game / Load Game stub / Settings stub / Quit) in `bevy_ui`.
- Pause menu on Esc: Resume / Settings stub / Quit to Menu.
- Entering/leaving `InGame` despawns state-scoped entities cleanly.

**Verify**

- [x] Full loop works: menu → new game → pause → quit to menu → new game again.
- [x] Second "new game" produces no duplicate entities or leaked state
      (log entity counts on state entry to confirm; counts plateau at 384
      after first-cycle engine warm-up).
- [x] Game simulation is actually frozen while paused.

## Step 2 — Asset conventions and Blender scene import

The Blender-as-level-editor pipeline.

- Folder conventions: `assets/levels/`, `assets/characters/`, `assets/props/`.
- A documented Blender export checklist (or export script): units, +Y up
  handling, apply transforms, glTF settings.
- Level loading: point the loader at one glTF, get a spawned scene.
- Marker convention for gameplay data authored in Blender (spawn points,
  colliders, triggers) — via naming convention or custom properties.
  Evaluate **Blenvy** here; adopt it only if it works on Bevy 0.19, otherwise
  use a naming-convention parser of our own.
- Decision to record: physics/collider crate (likely `avian3d`) since colliders
  from Blender need somewhere to go.

**Verify**

- [x] A test level authored in Blender (ground, ramps, obstacles, a spawn-point
      marker) loads and renders correctly with `cargo run`. (owner-authored
      `my_level.glb`; script-generated `test_level.glb` also available)
- [x] Re-exporting from Blender and re-running shows the change — no manual
      conversion steps in between.
- [x] Named markers from Blender are queryable as entities/components in Bevy.

## Step 3 — Third-person character and camera

- Player character controller: walk/run on the physics colliders from Step 2,
  slope handling, no jumping required (genre scope).
- Orbit/follow third-person camera with collision (doesn't clip through walls),
  zoom, and sensible defaults.
- Input mapping layer (evaluate `leafwing-input-manager`) so bindings are
  data, not scattered key checks.

**Verify**

- [x] Character walks around the Step 2 test level, collides with obstacles,
      handles ramps, and cannot escape the map.
- [x] Camera follows smoothly, never clips through level geometry, zooms.
- [x] Rebinding a key in one place (the input map) changes the control.

## Step 4 — Animation pipeline

The riskiest step for our constraints — buy/skip animations, no in-engine
retargeting. Do it early enough to change course cheaply.

- Import one animated glTF character from a glTF-native pack
  (KayKit / Quaternius Universal Animation Library / Kenney).
- Thin animation-controller layer over Bevy's `AnimationGraph`: named states
  (idle / walk / run / action), crossfade times, driven by controller velocity.
- Record which pack/rig the template blesses as its default.
  **Decided: KayKit Adventurers 2.0 (`Rig_Medium`)** — characters and shared
  animation libraries under `assets/characters/adventurers/`, retargeted onto
  character scenes at runtime by `CharacterAnimationPlugin` (clips address
  bones by name path, which all pack characters share). A Mixamo/FBX fallback
  path was documented, then descoped: the template is glTF-native only
  (README mentions the DIY conversion route).

**Verify**

- [x] Player character idles, blends to walk, blends to run based on speed —
      no popping, no T-posing, no disappearing mesh. (Transitions blend into
      the new clip from its first frame — phase-synced walk↔run blending was
      considered and deliberately left out of scope.)
- [x] A second character from the same pack reuses the same animation set
      without code changes.

## Step 5 — Save / load

- Choose the mechanism (evaluate `moonshine-save` and `bevy_save` on 0.19;
  fall back to hand-rolled reflection-based serialization).
- Convention: components opt in to persistence via a marker; save files are
  versioned from day one.
- Wire into the menu: Save in pause menu, Load from main menu, slots or
  timestamped files.
- Persist at minimum: player transform, camera state, level id, and one piece
  of arbitrary gameplay state to prove extensibility.
  **Decided:** timestamped files (`saves/save-<unix-secs>.ron`, Load Game
  loads the newest; a slot picker can come with step 9's settings UI).
  Persistence = entity opt-in via moonshine's `Save` marker
  (`#[require(Save)]` on persistent markers) **plus** a component allowlist
  in `saves.rs` — runtime state (physics, input, render handles) never
  enters files; per-plugin "hydration" systems rebuild it, so fresh spawns
  and loaded saves share one code path. The spinning dev cube's rotation is
  the arbitrary gameplay state.

**Verify**

- [x] Save, quit to desktop, relaunch, load → same position, camera, and
      gameplay state.
- [x] A save file from before adding a new persisted component still loads
      (version tolerance demonstrated by
      `old_save_missing_newly_persisted_component_still_loads` against a real
      file written by the save path).

(A "load from a different level than the save was made in" check was
descoped: the level id is persisted and drives what gets spawned, but
cross-level verification is out of the template's scope.)

## Step 6 — Navmesh and NPC movement

- Navmesh generation from level geometry (evaluate `vleue_navigator` first,
  `oxidized_navigation` as fallback).
  **Decided: bevy_landmass** (see crate table) — and with it, navmeshes are
  *authored in Blender* (hidden `marker = "navmesh"` mesh in the level file)
  rather than generated from collision geometry. "Re-export produces a
  correct new navmesh" now means: edit level + navmesh in the same .blend,
  re-export, done — no conversion or hand-tuning outside Blender.
- NPC entity archetype: spawned from Blender markers, walks to a target point.
- Click-to-move command for the player-controlled case (groundwork for squad
  and strategy modes).
- Debug overlay: draw the navmesh and active paths (toggleable).

**Verify**

- [x] An NPC pathfinds around obstacles to a clicked destination in the test
      level — no wall clipping, no stuck-on-corner within a 2-minute watch.
- [x] Re-exporting a modified level from Blender produces a correct new
      navmesh without hand-editing. (Owner sign-off 2026-07-14. Reminder the
      decision above implies: an added obstacle is only respected once its
      hole is also cut into the navmesh mesh — both edits live in the same
      .blend.)
- [x] Debug overlay toggles on a keypress (F3) and matches observed behavior.

## Step 7 — NPC behaviors and perception

- Behavior layer (evaluate `big-brain` utility AI — suits squad/strategy) with
  three reference behaviors: idle/wander, patrol (waypoints from Blender
  markers), chase-when-spotted.
  **Decided: hand-rolled FSM** (see crate table) — behaviors and factions are
  authored as `npc_spawn` properties in Blender (`behavior`, `faction`,
  `route`, `character`), patrol waypoints as `marker = "waypoint"` empties.
  Chase-when-spotted is an aggro overlay on any behavior, driven by
  perception and the `FactionRelations` resource (directional
  aggressor→victim pairs; the demo default is raiders→player).
- Perception: sight cone + range with line-of-sight raycast; aggro/de-aggro.
- Faction/team component so "enemy" vs "friendly NPC" is data, not code.

**Verify**

- [ ] A patrolling enemy spots the player entering its sight cone (and not
      through walls), chases, and returns to patrol after losing them.
- [ ] Behaviors are swappable per-entity in data without code changes.
- [ ] Animation controller from Step 4 reflects NPC state (walk on patrol,
      run on chase).

## Step 8 — Squad selection and commands

The squad/strategy layer on top of everything prior.

- Selection: click and drag-box select over friendly units; selection
  highlight rings.
- Commands: right-click move (navmesh, from Step 6), attack-move stub,
  stop/hold.
- Group movement: simple formation or local avoidance so units don't stack.
- RTS-style top-down camera mode, toggleable with the third-person camera —
  same world, two control schemes (the adventure/strategy duality of the
  template).

**Verify**

- [ ] Drag-select three units, right-click across the map: all three arrive
      without piling into one spot or shoving each other off ledges.
- [ ] Camera toggles between third-person-follow and top-down RTS mode at
      runtime; both control schemes work in the same session.
- [ ] Selected/unselected state is always visually unambiguous.

## Step 9 — Settings, polish, and template ergonomics

- Settings menu that actually works: resolution/window mode, volume, key
  rebinding (from Step 3's input layer); persisted to a config file.
- Audio plumbing: UI sounds + one positional 3D sound as reference.
- Diagnostics overlay (FPS, entity count) toggleable.
- Template docs: "how to start a game from this template" walkthrough,
  per-plugin README notes, the Blender pipeline doc finalized.
- Tag `v0.1.0`.

**Verify**

- [ ] Settings changes apply live, persist across relaunch.
- [ ] A fresh clone on another machine (or clean checkout): `cargo run`
      reaches the full loop — menu → play → save → load — following only
      the README.
- [ ] All three earlier "modes" still work in one session: adventure
      (third-person), squad (selection + commands), strategy camera.

---

## Deliberately out of scope

- Consoles, mobile, web export.
- In-engine level editing (Blender is the editor; revisit when the official
  Bevy editor ships).
- Networking/multiplayer.
- Combat depth (damage/health beyond what NPC behaviors need as a demo).
- Custom rigging/retargeting tooling — animation sourcing is glTF-native
  packs only. Converting FBX sources (e.g. Mixamo) to glTF in Blender is
  possible but DIY (brief mention in the README, nothing more).

## Open crate decisions (resolve in the step that needs them)

| Concern | Candidates | Decided in |
|---|---|---|
| Blender→Bevy metadata | **Decided (step 2): custom properties via glTF extras.** Blenvy is unmaintained (last release 2024-08, Bevy 0.14). | Step 2 |
| Physics | **Decided (step 2): avian3d 0.7** — targets Bevy 0.19 exactly, ECS-native. (bevy_rapier3d 0.35 also current; revisit only if avian blocks us.) | Step 2 |
| Input | **Decided (step 3): leafwing-input-manager 0.21** — targets Bevy 0.19; bindings live in `PlayerAction::default_input_map`. (bevy_enhanced_input 0.26 also current; revisit at step 9 if rebinding UI fits it better.) | Step 3 |
| Character controller | **Decided (step 3): hand-rolled** dynamic capsule + velocity control on avian — no jumps in genre scope. bevy-tnua 0.32 (Bevy 0.19-ready) is the upgrade path if feel demands it. | Step 3 |
| Save/load | **Decided (step 5): moonshine-save 0.7** (+ its bevy_world_serialization for filter types) — targets Bevy 0.19, released 2026-06. bevy_save's latest (2.0.1, 2025-08) is pinned to Bevy 0.16 — stale. Hand-rolled fallback not needed. | Step 5 |
| Navmesh | **Decided (step 6): bevy_landmass 0.12** — targets Bevy 0.19 (2026-06 release), brings steering + local avoidance for step 8. Both original candidates are stale: vleue_navigator 0.15 targets Bevy 0.18 (no activity since 2026-01), oxidized_navigation since 2024-12. Trade-off: landmass consumes navmeshes rather than generating them — the navmesh is authored in Blender as a hidden `marker = "navmesh"` mesh, consistent with Blender-as-editor. | Step 6 |
| AI | **Decided (step 7): hand-rolled FSM** in `npc_ai.rs` — both candidates fail the 0.19 rule: big-brain is dead (last release 2024-11, Bevy 0.15), bevy_behave 0.5 targets Bevy 0.18 with a ~quarterly cadence. bevior_tree 0.11 is 0.19-ready but tiny (single maintainer, code-defined trees). Three reference behaviors don't justify the dependency; the NPC AI plugin is the swap point if a downstream game needs real BT/utility AI. | Step 7 |
