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

- [ ] A test level authored in Blender (ground, ramps, obstacles, a spawn-point
      marker) loads and renders correctly with `cargo run`.
- [ ] Re-exporting from Blender and re-running shows the change — no manual
      conversion steps in between.
- [ ] Named markers from Blender are queryable as entities/components in Bevy.

## Step 3 — Third-person character and camera

- Player character controller: walk/run on the physics colliders from Step 2,
  slope handling, no jumping required (genre scope).
- Orbit/follow third-person camera with collision (doesn't clip through walls),
  zoom, and sensible defaults.
- Input mapping layer (evaluate `leafwing-input-manager`) so bindings are
  data, not scattered key checks.

**Verify**

- [ ] Character walks around the Step 2 test level, collides with obstacles,
      handles ramps, and cannot escape the map.
- [ ] Camera follows smoothly, never clips through level geometry, zooms.
- [ ] Rebinding a key in one place (the input map) changes the control.

## Step 4 — Animation pipeline

The riskiest step for our constraints — buy/skip animations, no in-engine
retargeting. Do it early enough to change course cheaply.

- Import one animated glTF character from a glTF-native pack
  (KayKit / Quaternius Universal Animation Library / Kenney).
- Thin animation-controller layer over Bevy's `AnimationGraph`: named states
  (idle / walk / run / action), crossfade times, driven by controller velocity.
- Document the Mixamo fallback path: FBX → Blender conversion script → glTF.
- Record which pack/rig the template blesses as its default.

**Verify**

- [ ] Player character idles, blends to walk, blends to run based on speed —
      no popping, no T-posing, no disappearing mesh.
- [ ] A second character from the same pack reuses the same animation set
      without code changes.
- [ ] The Mixamo fallback doc has been executed once end-to-end on a real
      Mixamo download and the result plays in-game.

## Step 5 — Save / load

- Choose the mechanism (evaluate `moonshine-save` and `bevy_save` on 0.19;
  fall back to hand-rolled reflection-based serialization).
- Convention: components opt in to persistence via a marker; save files are
  versioned from day one.
- Wire into the menu: Save in pause menu, Load from main menu, slots or
  timestamped files.
- Persist at minimum: player transform, camera state, level id, and one piece
  of arbitrary gameplay state to prove extensibility.

**Verify**

- [ ] Save, quit to desktop, relaunch, load → same position, camera, and
      gameplay state.
- [ ] Loading from a *different* level than the save was made in works
      (level id drives what gets spawned).
- [ ] A save file from before adding a new persisted component still loads
      (version tolerance demonstrated, not just designed).

## Step 6 — Navmesh and NPC movement

- Navmesh generation from level geometry (evaluate `vleue_navigator` first,
  `oxidized_navigation` as fallback).
- NPC entity archetype: spawned from Blender markers, walks to a target point.
- Click-to-move command for the player-controlled case (groundwork for squad
  and strategy modes).
- Debug overlay: draw the navmesh and active paths (toggleable).

**Verify**

- [ ] An NPC pathfinds around obstacles to a clicked destination in the test
      level — no wall clipping, no stuck-on-corner within a 2-minute watch.
- [ ] Re-exporting a modified level from Blender produces a correct new
      navmesh without hand-editing.
- [ ] Debug overlay toggles on a keypress and matches observed behavior.

## Step 7 — NPC behaviors and perception

- Behavior layer (evaluate `big-brain` utility AI — suits squad/strategy) with
  three reference behaviors: idle/wander, patrol (waypoints from Blender
  markers), chase-when-spotted.
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
- Custom rigging/retargeting tooling — tiered animation sourcing instead
  (glTF packs → Mixamo+Blender script → manual retargeting, in that order).

## Open crate decisions (resolve in the step that needs them)

| Concern | Candidates | Decided in |
|---|---|---|
| Blender→Bevy metadata | **Decided (step 2): custom properties via glTF extras.** Blenvy is unmaintained (last release 2024-08, Bevy 0.14). | Step 2 |
| Physics | **Decided (step 2): avian3d 0.7** — targets Bevy 0.19 exactly, ECS-native. (bevy_rapier3d 0.35 also current; revisit only if avian blocks us.) | Step 2 |
| Input | leafwing-input-manager vs hand-rolled | Step 3 |
| Save/load | moonshine-save vs bevy_save vs hand-rolled | Step 5 |
| Navmesh | vleue_navigator vs oxidized_navigation | Step 6 |
| AI | big-brain vs bevy_behave | Step 7 |
