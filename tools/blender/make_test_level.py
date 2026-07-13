"""Generate the test level: assets_src/test_level.blend + assets/levels/test_level.glb.

Run from the repo root:

    blender --background --python tools/blender/make_test_level.py

The generated .blend is the editable source; re-export it after changes with
tools/blender/export_level.py (see docs/blender-pipeline.md).
"""

import math
import os

import bpy

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))


def make_material(name, rgba):
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    mat.node_tree.nodes["Principled BSDF"].inputs["Base Color"].default_value = rgba
    return mat


def add_box(name, size, location, rotation=(0, 0, 0), material=None, props=None):
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=location, rotation=rotation)
    obj = bpy.context.active_object
    obj.name = name
    obj.scale = size
    # Colliders are built from exported geometry; bake scale into the mesh.
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    if material:
        obj.data.materials.append(material)
    for key, value in (props or {}).items():
        obj[key] = value
    return obj


bpy.ops.wm.read_factory_settings(use_empty=True)

ground_mat = make_material("Ground", (0.35, 0.5, 0.35, 1.0))
obstacle_mat = make_material("Obstacle", (0.55, 0.45, 0.35, 1.0))
ramp_mat = make_material("Ramp", (0.4, 0.45, 0.6, 1.0))

add_box("Ground", (24, 24, 0.2), (0, 0, -0.1), material=ground_mat, props={"collider": "trimesh"})
add_box(
    "Ramp",
    (4, 8, 0.3),
    (6, 0, 1.4),
    rotation=(math.radians(-20), 0, 0),
    material=ramp_mat,
    props={"collider": "trimesh"},
)
for i, (x, y) in enumerate([(-5, 4), (-3, -5), (2, 6)]):
    add_box(
        f"Pillar.{i}",
        (1, 1, 3),
        (x, y, 1.5),
        material=obstacle_mat,
        props={"collider": "trimesh"},
    )

bpy.ops.object.empty_add(type="PLAIN_AXES", location=(0, -8, 0.1))
spawn = bpy.context.active_object
spawn.name = "PlayerSpawn"
spawn["marker"] = "player_spawn"

os.makedirs(os.path.join(ROOT, "assets_src"), exist_ok=True)
bpy.ops.wm.save_as_mainfile(filepath=os.path.join(ROOT, "assets_src", "test_level.blend"))

os.makedirs(os.path.join(ROOT, "assets", "levels"), exist_ok=True)
bpy.ops.export_scene.gltf(
    filepath=os.path.join(ROOT, "assets", "levels", "test_level.glb"),
    export_format="GLB",
    export_extras=True,  # custom properties -> glTF extras -> Bevy components
    export_apply=True,
)
print("wrote assets_src/test_level.blend and assets/levels/test_level.glb")
