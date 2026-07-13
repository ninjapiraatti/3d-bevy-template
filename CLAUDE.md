# 3d-bevy-template

A Bevy template for 3rd-person games: adventure, squad mechanics, and strategy.
Explicitly NOT for platformers or fast-paced FPS. Desktop only (Win/macOS/Linux).

## 1. Think Before Coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

Before implementing:
- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them - don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop. Name what's confusing. Ask.

## 2. Simplicity First

**Minimum code that solves the problem. Nothing speculative.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

## 3. Surgical Changes

**Touch only what you must. Clean up only your own mess.**

When editing existing code:
- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- If you notice unrelated dead code, mention it - don't delete it.

When your changes create orphans:
- Remove imports/variables/functions that YOUR changes made unused.
- Don't remove pre-existing dead code unless asked.

## 4. Goal-Driven Execution

**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals:
- "Add validation" → "Write tests for invalid inputs, then make them pass"
- "Fix the bug" → "Write a test that reproduces it, then make it pass"
- "Refactor X" → "Ensure tests pass before and after"

For multi-step tasks, state a brief plan:
```
1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]
```

## Ground rules

- Engine: **Bevy 0.19**. Check crate compatibility against this version before
  adding any dependency; the Bevy ecosystem lags releases and a crate that
  hasn't updated is a real blocker, not a detail.
- Work proceeds by **ROADMAP.md steps, in order**. Finish the current step's
  "Verify" checklist before starting the next. If a step forces a decision the
  roadmap left open (see its crate-decision table), record the choice and the
  reason in ROADMAP.md when making it.
- Asset pipeline is **Blender → glTF** exclusively. No FBX in the repo; FBX
  sources get converted via the documented Blender path first. Characters and
  animations come from glTF-native packs (KayKit, Quaternius, Kenney) —
  the project owner prefers buying/skipping rigging and animation work.
- Levels are authored in **Blender, not in code**. Gameplay data (spawn points,
  triggers, patrol waypoints) is authored in Blender via the marker convention
  established in roadmap Step 2.

## Architecture

- Cargo workspace: `game/` (thin binary) + `template_core/` (library).
- **One Bevy plugin per concern** (menus, saves, camera, animation controller,
  NPC AI, squad commands, …) so downstream games can swap pieces. No
  cross-plugin reach-ins; plugins communicate via events and shared components.
- Everything hangs off `AppState` (`MainMenu` / `Loading` / `InGame` /
  `Paused`). Entities are state-scoped; entering a state twice must not leak
  or duplicate anything.
- Prefer data over code: input bindings, behavior assignment, factions, and
  persistence opt-in are components/assets, not hardcoded branches.

## Commands

```sh
cargo run                                    # run the demo game
cargo clippy --all-targets -- -D warnings    # must stay clean
cargo fmt --check
cargo test
```

Dev profile uses Bevy dynamic linking for fast iteration; don't ship it.

## Verification

Steps are verified by running the game and observing behavior (see each
roadmap step's checklist), not just by compiling. Unit tests cover logic that
doesn't need a window (save versioning, state machines, selection math).

**The project owner always launches the game themselves — never `cargo run`
(or otherwise start the app) from the agent.** When a checklist item needs
runtime observation, get the code building/clippy-clean, then hand over a
short list of what to look for and wait for the owner's report.

## Style

- Rust 2021+, idiomatic Bevy: systems small and named for what they do,
  run conditions over internal early-returns where possible.
- Comments only for constraints the code can't express (e.g. "glTF forward is
  -Z, Blender export flips it here").
