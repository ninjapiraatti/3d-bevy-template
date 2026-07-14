"""Generate the test level: assets_src/test_level.blend + assets/levels/test_level.glb.

Run from the repo root:

    blender --background --python tools/blender/make_test_level.py

The generated .blend is the editable source; re-export it after changes with
tools/blender/export_level.py (see docs/blender-pipeline.md).
"""

import math
import os

import bmesh
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

bpy.ops.object.empty_add(type="PLAIN_AXES", location=(-1, 0, 0.1))
npc = bpy.context.active_object
npc.name = "NpcSpawn"
npc["marker"] = "npc_spawn"

# Step 8: two more player-faction units near the spawn, so drag-select has a
# squad of three to work with.
for i, (x, y, character) in enumerate([(1.5, -6.5, "Mage"), (-1.5, -6.5, "Ranger")]):
    bpy.ops.object.empty_add(type="PLAIN_AXES", location=(x, y, 0.1))
    squad = bpy.context.active_object
    squad.name = f"SquadSpawn.{i}"
    squad["marker"] = "npc_spawn"
    squad["character"] = character

# Step 7: a hostile patroller looping the west side of the map (route A),
# and the waypoints it follows. Order is authored explicitly so the loop
# direction is deliberate, not alphabetical.
for i, (x, y) in enumerate([(-9, -9), (-9, 9), (-4, 9), (-4, -9)]):
    bpy.ops.object.empty_add(type="SPHERE", radius=0.3, location=(x, y, 0.1))
    waypoint = bpy.context.active_object
    waypoint.name = f"WaypointA.{i}"
    waypoint["marker"] = "waypoint"
    waypoint["route"] = "A"
    waypoint["order"] = i

bpy.ops.object.empty_add(type="PLAIN_AXES", location=(-9, -9, 0.1))
enemy = bpy.context.active_object
enemy.name = "EnemySpawn"
enemy["marker"] = "npc_spawn"
enemy["faction"] = "raiders"
enemy["behavior"] = "patrol"
enemy["route"] = "A"
enemy["character"] = "Rogue_Hooded"


def add_navmesh():
    """Walkable-area mesh (hidden at runtime; marker = "navmesh").

    A 1 m grid over the ground, skipping cells that touch an obstacle
    footprint expanded by the agent radius. Shared vertices keep the
    polygons connected, which landmass requires for pathing across them.
    """
    keepouts = [
        # (center_x, center_y, half_x, half_y): pillars + agent margin
        (-5, 4, 0.9, 0.9),
        (-3, -5, 0.9, 0.9),
        (2, 6, 0.9, 0.9),
        # The ramp is solid; keep the flat navmesh out of its footprint.
        (6, 0, 2.4, 4.4),
    ]
    half = 11  # 1 m inside the 24x24 ground edge
    mesh = bpy.data.meshes.new("Navmesh")
    bm = bmesh.new()
    verts = {}

    def vert(x, y):
        if (x, y) not in verts:
            verts[(x, y)] = bm.verts.new((x, y, 0.05))
        return verts[(x, y)]

    for x in range(-half, half):
        for y in range(-half, half):
            cx, cy = x + 0.5, y + 0.5
            blocked = any(
                abs(cx - kx) <= khx and abs(cy - ky) <= khy
                for kx, ky, khx, khy in keepouts
            )
            if not blocked:
                bm.faces.new(
                    [vert(x, y), vert(x + 1, y), vert(x + 1, y + 1), vert(x, y + 1)]
                )
    bm.to_mesh(mesh)
    bm.free()
    obj = bpy.data.objects.new("Navmesh", mesh)
    bpy.context.collection.objects.link(obj)
    obj["marker"] = "navmesh"


add_navmesh()

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
