# Mixamo fallback: FBX → Blender → glTF

The template's primary animation source is glTF-native packs (KayKit,
Quaternius, Kenney) — see `docs/blender-pipeline.md` for the folder layout and
`CharacterAnimations::kaykit_adventurer` for the blessed default rig. Use this
path only when a character exists solely as FBX (e.g. Mixamo): the FBX is
converted once in Blender and never enters the repo (`assets_src/` downloads
are gitignored; only the exported `.glb` under `assets/` is committed).

Unlike the KayKit setup (shared rig, clips in separate library files), the
result here is **one self-contained `.glb`**: mesh, rig, and named clips
together. `CharacterAnimations` then points `libraries` at the character's own
file, so no cross-file rig matching is involved.

## 1. Download from Mixamo

1. Pick a character, download as **FBX Binary, T-pose, With Skin**.
2. For each clip (idle, walk, run, …), apply the animation to that same
   character and download as **FBX Binary, Without Skin**, 30 fps, no
   keyframe reduction.

Put everything in `assets_src/mixamo/<character>/` (gitignored).

## 2. Combine in Blender

1. Import the skinned character FBX (File → Import → FBX).
2. Import each animation FBX the same way. Each import adds a duplicate
   armature carrying one action; Mixamo bone names match, so the action also
   works on the character's armature.
3. On the character's armature, open the **Action Editor** and assign each
   imported action; rename actions to the clip names the game will use
   (`Idle`, `Walk`, `Run`). **Stash each action** (Action Editor → Stash) so
   it survives export, then delete the now-empty imported armatures.
4. Select the character armature + meshes, **Apply All Transforms**
   (Mixamo FBX arrives at 0.01 scale — skipping this is the classic
   "character is 1.7 cm tall" bug).

## 3. Export

File → Export → glTF 2.0:

- Format: **glTF Binary (.glb)**, into `assets/characters/<character>.glb`
- Mesh → **Apply Modifiers: ON**
- Animation → mode **Actions** (each action becomes a named glTF clip)

## 4. Use in the game

```rust
CharacterAnimations {
    libraries: vec![assets.load("characters/<character>.glb")],
    idle: "Idle".into(),
    walk: "Walk".into(),
    run: "Run".into(),
    ..CharacterAnimations::kaykit_adventurer(&assets)
}
```

on the entity holding the character's `WorldAssetRoot` (see how
`spawn_player` wires the Knight). Clip names are listed at load in the log if
you get one wrong — `CharacterAnimationPlugin` warns and skips animating
rather than crashing.
