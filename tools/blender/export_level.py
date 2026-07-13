"""Export the opened .blend to .glb with the project's settings.

    blender <file.blend> --background --python tools/blender/export_level.py -- <out.glb>

Example:

    blender assets_src/test_level.blend --background \
        --python tools/blender/export_level.py -- assets/levels/test_level.glb
"""

import os
import sys

import bpy

out = os.path.abspath(sys.argv[sys.argv.index("--") + 1])
os.makedirs(os.path.dirname(out), exist_ok=True)
bpy.ops.export_scene.gltf(
    filepath=out,
    export_format="GLB",
    export_extras=True,
    export_apply=True,
)
print(f"wrote {out}")
