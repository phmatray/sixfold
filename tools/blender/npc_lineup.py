"""Lineup of the hero and every NPC, front view and game camera (headless only).

    blender -b --factory-startup -P tools/blender/npc_lineup.py -- --assets web/world/assets --out /tmp/lineup
Writes <out>_front.png and <out>_game.png (all characters on their Idle pose, frame 30).
"""
import math
import os
import sys

import bpy
from mathutils import Vector

ORDER = ["hero", "npc_shopkeeper", "npc_gardener", "npc_wanderer", "npc_kid", "npc_rival"]


def main(argv):
    assets = argv[argv.index("--assets") + 1]
    out = argv[argv.index("--out") + 1]
    for o in list(bpy.data.objects):
        bpy.data.objects.remove(o)
    scn = bpy.context.scene
    for i, name in enumerate(ORDER):
        path = os.path.join(assets, "npc" if name.startswith("npc_") else "", name + ".glb")
        before = set(bpy.data.objects)
        acts = set(bpy.data.actions)
        bpy.ops.import_scene.gltf(filepath=os.path.abspath(path))
        new = [o for o in bpy.data.objects if o not in before]
        for root in (o for o in new if o.parent is None):  # rig and skinned mesh are both roots
            root.location.x = (i - (len(ORDER) - 1) / 2) * 1.1
        rig = next(o for o in new if o.type == "ARMATURE")
        idle = next(a for a in bpy.data.actions if a not in acts and a.name.startswith("Idle"))
        ad = rig.animation_data or rig.animation_data_create()
        for t in ad.nla_tracks:
            t.mute = True
        ad.action = idle
        if idle.slots:
            ad.action_slot = idle.slots[0]
    scn.frame_set(30)
    scn.render.engine = "BLENDER_EEVEE"
    scn.render.resolution_x, scn.render.resolution_y = 1500, 420
    scn.view_settings.view_transform = "Standard"
    w = bpy.data.worlds.new("w")
    w.use_nodes = True
    bg = next(n for n in w.node_tree.nodes if n.type == "BACKGROUND")
    bg.inputs["Color"].default_value = (0.36, 0.62, 0.3, 1)
    scn.world = w
    sun = bpy.data.objects.new("sun", bpy.data.lights.new("sun", "SUN"))
    sun.data.energy = 3.2
    sun.rotation_euler = (math.radians(40), math.radians(15), math.radians(-35))
    scn.collection.objects.link(sun)
    cam = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
    scn.collection.objects.link(cam)
    scn.camera = cam
    cam.data.type = "ORTHO"
    cam.data.ortho_scale = 6.9
    for label, d in (("front", (0, -1, 0.08)), ("game", (0, -math.cos(math.radians(50)), math.sin(math.radians(50))))):
        d = Vector(d).normalized()
        cam.location = Vector((0, 0, 0.8)) + d * 20
        cam.rotation_euler = (-d).to_track_quat("-Z", "Y").to_euler()
        scn.render.filepath = os.path.abspath(f"{out}_{label}.png")
        bpy.ops.render.render(write_still=True)
    print("[lineup]", out)


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1:])
