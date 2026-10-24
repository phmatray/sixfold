"""Hero turnaround sheet (headless only): 6 views + one strip per animation clip.

    blender -b --factory-startup -P tools/blender/hero_turnaround.py -- \
        --glb web/world/assets/hero.glb --out /tmp/hero-turnaround.png [--label before]

Row 1: front, back, left, right, top, three-quarter (orthographic, on a grass disc).
Next rows: each clip in 6 evenly spaced frames, from the game camera (50 deg pitch,
looking at the hero's face) and, for Walk/Run, from the side.
"""
import math
import os
import sys

import bpy
import numpy as np
from mathutils import Vector

TILE = 260


def arg(argv, name, default=None):
    return argv[argv.index(name) + 1] if name in argv else default


def setup(glb):
    for o in list(bpy.data.objects):
        bpy.data.objects.remove(o)
    bpy.ops.import_scene.gltf(filepath=os.path.abspath(glb))
    scn = bpy.context.scene
    scn.render.engine = "BLENDER_EEVEE"
    scn.render.resolution_x = scn.render.resolution_y = TILE
    scn.render.film_transparent = False
    scn.view_settings.view_transform = "Standard"
    w = bpy.data.worlds.new("w")
    w.use_nodes = True
    bg = next(n for n in w.node_tree.nodes if n.type == "BACKGROUND")
    bg.inputs["Color"].default_value = (0.62, 0.78, 0.55, 1)
    bg.inputs["Strength"].default_value = 0.9
    scn.world = w
    sun = bpy.data.objects.new("sun", bpy.data.lights.new("sun", "SUN"))
    sun.data.energy = 3.2
    sun.rotation_euler = (math.radians(40), math.radians(15), math.radians(-35))
    scn.collection.objects.link(sun)
    disc = bpy.data.meshes.new("disc")
    import bmesh
    bm = bmesh.new()
    bmesh.ops.create_circle(bm, cap_ends=True, segments=32, radius=0.6)
    bm.to_mesh(disc)
    mat = bpy.data.materials.new("grass")
    mat.use_nodes = True
    next(n for n in mat.node_tree.nodes if n.type == "BSDF_PRINCIPLED").inputs["Base Color"].default_value = (0.08, 0.36, 0.05, 1)
    disc.materials.append(mat)
    d = bpy.data.objects.new("disc", disc)
    d.location.z = -0.005
    scn.collection.objects.link(d)
    cam = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
    cam.data.clip_start, cam.data.clip_end = 0.1, 100
    scn.collection.objects.link(cam)
    scn.camera = cam
    rig = next((o for o in scn.objects if o.type == "ARMATURE"), None)
    return scn, cam, rig


def aim(cam, direction, ortho=1.9, target=(0, 0, 0.72), dist=12):
    d = Vector(direction).normalized()
    cam.data.type = "ORTHO"
    cam.data.ortho_scale = ortho
    cam.location = Vector(target) + d * dist
    cam.rotation_euler = (-d).to_track_quat("-Z", "Y").to_euler()


def play(rig, name):
    ad = rig.animation_data
    for t in ad.nla_tracks:
        t.mute = True
    act = next((a for a in bpy.data.actions if a.name.split(".")[0] == name), None)
    ad.action = act
    if act and act.slots:
        ad.action_slot = act.slots[0]
    return act


def shoot(scn, path):
    scn.render.filepath = path
    bpy.ops.render.render(write_still=True)
    img = bpy.data.images.load(path)
    px = np.array(img.pixels[:], dtype=np.float32).reshape(TILE, TILE, 4)
    bpy.data.images.remove(img)
    return px


def main(argv):
    glb, out = arg(argv, "--glb"), arg(argv, "--out")
    scn, cam, rig = setup(glb)
    tmp = os.path.join(os.path.dirname(os.path.abspath(out)), "_turn_tile.png")
    rows = []
    views = {"front": (0, -1, 0.12), "back": (0, 1, 0.12), "left": (-1, 0, 0.12), "right": (1, 0, 0.12),
             "top": (0, -0.001, 1), "3/4": (0.7, -0.7, 0.45)}
    if rig:
        play(rig, "Idle")
    scn.frame_set(0)
    row = []
    for d in views.values():
        aim(cam, d)
        row.append(shoot(scn, tmp))
    rows.append(row)
    game = (0, -math.cos(math.radians(50)), math.sin(math.radians(50)))
    for clip, view in (("Idle", game), ("Walk", game), ("Walk", (1, 0, 0.08)), ("Run", (1, 0, 0.08)), ("Fidget", game)):
        act = play(rig, clip) if rig else None
        if not act:
            continue
        s, e = act.frame_range
        aim(cam, view)
        row = []
        for i in range(6):
            scn.frame_set(int(round(s + (e - s) * i / 6)))
            row.append(shoot(scn, tmp))
        rows.append(row)
    os.remove(tmp)
    sheet = np.concatenate([np.concatenate(r, axis=1) for r in reversed(rows)], axis=0)  # pixels are bottom-up
    h, w = sheet.shape[:2]
    img = bpy.data.images.new("sheet", w, h)
    img.pixels.foreach_set(sheet.ravel())
    img.filepath_raw = os.path.abspath(out)
    img.file_format = "PNG"
    img.save()
    print("[turnaround]", out, f"{len(rows)} rows")


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
