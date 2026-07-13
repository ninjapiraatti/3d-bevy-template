# Blender → glTF pipeline

## Blender CLI setup (macOS)

The scripts below need `blender` on your PATH. **Do not symlink the binary** —
Blender resolves its bundled Python and resource files relative to the
executable path, so a symlinked `blender` crashes at startup with
`Bundled Python not found` / `couldn't find 'scripts/modules'`
(`--version` still works, which makes it look installed). Use a wrapper
script instead, which `exec`s the real path:

```sh
printf '#!/bin/sh\nexec /Applications/Blender.app/Contents/MacOS/Blender "$@"\n' \
    > /opt/homebrew/bin/blender
chmod +x /opt/homebrew/bin/blender
```

Verify with:

```sh
blender --background --python-expr "import bpy; print('PYTHON OK', bpy.app.version_string)"
```

Blender is this template's level editor. Levels are authored as `.blend`
files in `assets_src/` (editable sources) and exported as `.glb` into
`assets/levels/` (what the game loads). The game never reads `.blend` files.

## Folder conventions

| Folder | Contents |
|---|---|
| `assets_src/` | Editable `.blend` sources (committed, never loaded by the game) |
| `assets/levels/` | Exported level `.glb` files |
| `assets/characters/` | Rigged/animated character `.glb` files (from step 4 on) |
| `assets/props/` | Reusable prop `.glb` files |
| `tools/blender/` | Blender automation scripts |

## Authoring rules

- **Units:** metric, 1 Blender unit = 1 meter. Leave scene units at defaults.
- **Apply scale** (`Ctrl+A` → Scale) on every mesh object, or export with
  "Apply Modifiers" on. Colliders are generated from exported geometry;
  unapplied scale is the classic source of "collider doesn't match visual".
- **Stable names.** Object names end up on Bevy entities as `Name` components
  and appear in logs — rename deliberately, not accidentally.
- **Orientation:** the exporter converts Blender's Z-up to glTF's Y-up
  automatically; author levels normally. (Character facing conventions get
  pinned down in roadmap step 4.)

## Gameplay data: custom properties

Gameplay data is authored as **object custom properties**
(Object Properties → Custom Properties). They export as glTF "extras" and
`template_core::levels` turns them into typed components at spawn.

| Property | Value | Effect in Bevy |
|---|---|---|
| `marker` | `player_spawn` | `PlayerSpawn` component (use on an empty) |
| `collider` | `trimesh` | Static physics collider generated from the object's meshes |

Property values must be strings. Unknown values log a warning at load rather
than failing, so typos are visible in the terminal.

## Exporting

Either run the export script (repeatable, right settings guaranteed):

```sh
blender assets_src/test_level.blend --background \
    --python tools/blender/export_level.py -- assets/levels/test_level.glb
```

or use File → Export → glTF 2.0 manually with:

- Format: **glTF Binary (.glb)**
- Include → Data → **Custom Properties: ON** ← easy to miss, markers die without it
- Mesh → **Apply Modifiers: ON**

Then just `cargo run` — no further conversion step exists.

## Regenerating the test level

The test level is script-generated so it can be rebuilt from nothing:

```sh
blender --background --python tools/blender/make_test_level.py
```

This writes both `assets_src/test_level.blend` and `assets/levels/test_level.glb`.
