# Bringing in character packs

How to go from a downloaded character bundle to an animated character in the
game. The template's blessed default is **KayKit Adventurers 2.0** (rig:
`Rig_Medium`), already set up under `assets/characters/adventurers/` —
follow the same steps for any other pack.

## What to buy

glTF-native packs with rigged characters, in order of preference: KayKit,
Quaternius (Universal Animation Library + Universal Base Characters), Kenney.
Two layouts exist, and both work:

- **Shared-rig packs** (KayKit, Quaternius): characters ship *without*
  animations; clips live in separate library files built on the same rig.
  Any character from the pack plays any library clip.
- **Self-contained characters**: mesh, rig, and clips in one file.

A character is only usable if it's available as `.glb`/`.gltf`; FBX never
enters the repo. Converting FBX sources (e.g. Mixamo) to glTF in Blender is
possible but outside this template's scope.

## 1. Download into `assets_src/`

Unpack into `assets_src/<pack_name>/`. Subfolders of `assets_src/` are
gitignored (raw packs contain FBX/OBJ duplicates and samples we don't ship);
nothing to clean up, and the folder can be deleted once the glTF files are
copied out.

## 2. Copy the glTF files into `assets/characters/`

```
assets/characters/<pack>/
├── License.txt              ← always keep the pack's license
├── Knight.glb               ← character scenes
├── ...
└── animations/              ← shared-rig clip libraries (if separate)
    └── Rig_Medium_MovementBasic.glb
```

Only `.glb` files and the license — textures are embedded in KayKit-style
GLBs, loose texture PNGs from the pack are usually redundant copies.

## 3. Check the rig actually matches (shared-rig packs)

Clips address bones by the **name path** from the glTF scene root node down
(`Rig_Medium/root/hips/…`), so character and library files must agree on
bone names *and* hierarchy exactly. Same-pack files do; verify when mixing
packs or pack versions. Any glTF inspector works for comparing node trees
(e.g. https://gltf.report, or Blender import).

Clip names differ per pack (KayKit: `Idle_A`, `Walking_A`, `Running_A`) —
find them in the library file the same way.

## 4. Wire it up in code

Spawn the character scene and put a `CharacterAnimations` next to it:

```rust
parent.spawn((
    WorldAssetRoot(assets.load(
        GltfAssetLabel::Scene(0).from_asset("characters/adventurers/Knight.glb"),
    )),
    CharacterAnimations::kaykit_adventurer(&assets),
    // Model origin is at the feet; parented to a physics capsule it sits at
    // -(half height). KayKit rigs face +Z, entity forward is -Z: yaw 180°.
    Transform::from_xyz(0.0, -0.85, 0.0)
        .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
));
```

For a non-KayKit pack, build the component yourself: `libraries` is the list
of glTF files to look clip names up in (for a self-contained character, the
character's own file), `idle`/`walk`/`run` are the pack's clip names, and the
speed thresholds/crossfade have sensible struct defaults to copy from
`kaykit_adventurer`. That component is the whole integration — locomotion
speed is read from the physics velocity on the entity or its ancestors, and
entities without a physics body idle.

Under the hood: Bevy only wires animation components for glTF files that
contain clips, so `CharacterAnimationPlugin` stamps the spawned character
scene at runtime with the same name-path target IDs the loader would have
used. That's what makes the cross-file rig sharing work; you don't interact
with it beyond the component.

## Troubleshooting

| Symptom | Cause |
|---|---|
| T-pose, warning listing available clips | Clip name typo — use a name from the logged list |
| T-pose, no warning | Rig mismatch: library bone names/hierarchy differ from the character's (step 3) |
| Walks backwards | Missing 180° yaw on the model transform |
| Floats above / sinks into ground | Model-origin offset wrong for the collider it's parented to |
| Animates but never leaves idle | No `LinearVelocity` on the entity or any ancestor |
