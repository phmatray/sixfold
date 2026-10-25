"""Bust portraits of the campaign rivals for the duel screen (headless only).

    blender -b --factory-startup -P tools/blender/rival_portraits.py -- \
        --assets web/world/assets --out web/world/assets/portraits [--size 512] [--only bramble,warden]

Each rival_*.glb on its Idle pose (frame 30, eyes open), three-quarter view, ortho, framed on the
head sphere (skin faces weighted to the Head bone) so every bust has the same scale whatever the
hat or hair. Key + soft rim light, then composited over a radial gradient in the rival's colour.
Writes <out>/<rival id>.webp (cwebp, must be on PATH); ids as in engine/src/campaign.rs.
"""
import math
import os
import subprocess
import sys

import bpy
import numpy as np
from mathutils import Vector

RIVALS = {  # id: (model, centre tint, edge colour) — the card colours of web/style.css (.t-*)
    "bramble": ("rival_grass", "#c6eba9", "#3f9a52"),
    "marlo": ("rival_water", "#b3e0ff", "#2f7fd0"),
    "cinza": ("rival_fire", "#ffd08a", "#e1572f"),
    "quill": ("rival_lightning", "#fff2a8", "#e2ad19"),
    "orla": ("rival_fighting", "#f2c595", "#a8642f"),
    "lumen": ("rival_psychic", "#e5c7ff", "#8a4fc9"),
    "warden": ("rival_champion", "#efe0b0", "#3a3f5c"),
}


def rgb(h):
    return np.array([int(h[i:i + 2], 16) / 255 for i in (1, 3, 5)], np.float32)


def clear():
    for o in list(bpy.data.objects):
        bpy.data.objects.remove(o)
    for a in list(bpy.data.actions):
        bpy.data.actions.remove(a)


def load(path):
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=path)
    new = [o for o in bpy.data.objects if o not in before]
    rig = next(o for o in new if o.type == "ARMATURE")
    idle = next(a for a in bpy.data.actions if a.name.startswith("Idle"))
    ad = rig.animation_data or rig.animation_data_create()
    for t in ad.nla_tracks:
        t.mute = True
    ad.action = idle
    if idle.slots:
        ad.action_slot = idle.slots[0]
    return new


def head_sphere(objs):
    """Centre and radius of the face: evaluated skin faces whose verts belong to the Head group."""
    dg = bpy.context.evaluated_depsgraph_get()
    pts = []
    for o in objs:
        if o.type != "MESH" or "Head" not in o.vertex_groups:
            continue
        gi = o.vertex_groups["Head"].index
        skin = {i for i, m in enumerate(o.data.materials) if m and m.name.split(".")[0].endswith("skin")}
        head = {v.index for v in o.data.vertices if any(g.group == gi and g.weight > 0.5 for g in v.groups)}
        ev = o.evaluated_get(dg)
        me = ev.to_mesh()
        for p in me.polygons:
            if p.material_index in skin and all(i in head for i in p.vertices):
                pts += [ev.matrix_world @ me.vertices[i].co for i in p.vertices]
        ev.to_mesh_clear()
    lo = Vector([min(p[i] for p in pts) for i in range(3)])
    hi = Vector([max(p[i] for p in pts) for i in range(3)])
    return (lo + hi) / 2, max(hi - lo) / 2


def light(name, kind, energy, rot, size=None, color=(1, 1, 1)):
    ld = bpy.data.lights.new(name, kind)
    ld.energy = energy
    ld.color = color
    if size:
        ld.size = size
    o = bpy.data.objects.new(name, ld)
    o.rotation_euler = [math.radians(a) for a in rot]
    bpy.context.scene.collection.objects.link(o)
    return o


def render(rid, assets, out, size):
    model, inner, outer = RIVALS[rid]
    clear()
    scn = bpy.context.scene
    objs = load(os.path.join(assets, "npc", model + ".glb"))
    scn.frame_set(30)
    c, r = head_sphere(objs)
    scn.render.engine = "BLENDER_EEVEE"
    scn.render.resolution_x = scn.render.resolution_y = size
    scn.render.film_transparent = True
    scn.view_settings.view_transform = "Standard"
    w = bpy.data.worlds.new("pw")
    w.use_nodes = True
    bg = next(n for n in w.node_tree.nodes if n.type == "BACKGROUND")
    bg.inputs["Color"].default_value = (0.75, 0.75, 0.8, 1)
    bg.inputs["Strength"].default_value = 0.55
    scn.world = w
    for o in (light("key", "SUN", 2.6, (50, 0, -35)),                              # front-left, above
              light("rim", "SUN", 9.0, (-65, 0, 160), color=(1.0, 0.95, 0.85))):   # behind, right: the rim
        o.data.angle = math.radians(12)                                             # soft edges
    cam = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
    scn.collection.objects.link(cam)
    scn.camera = cam
    cam.data.type = "ORTHO"
    cam.data.ortho_scale = r * 3.7
    target = c - Vector((0, 0, r * 0.3))
    d = Vector((0.66, -0.75, 0.1)).normalized()                          # three-quarter, front-right
    cam.location = target + d * 10
    cam.rotation_euler = (-d).to_track_quat("-Z", "Y").to_euler()
    tmp = os.path.join(out, f"_{rid}.png")
    scn.render.filepath = tmp
    scn.render.image_settings.file_format = "PNG"
    scn.render.image_settings.color_mode = "RGBA"
    bpy.ops.render.render(write_still=True)
    img = bpy.data.images.load(tmp)
    px = np.array(img.pixels[:], np.float32).reshape(size, size, 4)
    bpy.data.images.remove(img)
    yy, xx = np.mgrid[0:size, 0:size].astype(np.float32) / (size - 1)
    t = np.clip(np.hypot(xx - 0.5, yy - 0.62) / 0.75, 0, 1)[..., None] ** 1.3   # bottom-up rows: light behind the head
    grad = rgb(inner) * (1 - t) + rgb(outer) * t
    a = px[..., 3:4]
    px[..., :3] = px[..., :3] * a + grad * (1 - a)
    px[..., 3] = 1
    img = bpy.data.images.new("pp", size, size)
    img.pixels.foreach_set(px.ravel())
    img.filepath_raw = tmp
    img.file_format = "PNG"
    img.save()
    dst = os.path.join(out, rid + ".webp")
    subprocess.run(["cwebp", "-quiet", "-q", "80", "-m", "6", tmp, "-o", dst], check=True)
    os.remove(tmp)
    kb = os.path.getsize(dst) / 1024
    print(f"[portrait] {rid:8s} {model:16s} {kb:5.1f} KB")
    if kb > 60:
        raise RuntimeError(f"[portrait] {dst} is over 60 KB")


def main(argv):
    arg = lambda k, d=None: argv[argv.index(k) + 1] if k in argv else d  # noqa: E731
    assets, out = os.path.abspath(arg("--assets")), os.path.abspath(arg("--out"))
    size = int(arg("--size", "512"))
    only = arg("--only")
    os.makedirs(out, exist_ok=True)
    for rid in (only.split(",") if only else RIVALS):
        render(rid, assets, out, size)


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1:])
