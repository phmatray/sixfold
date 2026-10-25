"""Bust portraits of the campaign rivals for the duel screen (headless only).

    blender -b --factory-startup -P tools/blender/rival_portraits.py -- \
        --assets web/world/assets --out web/world/assets/portraits [--size 512] [--only bramble,warden]

Each rival_*.glb on its Idle pose (frame 30, eyes open), three-quarter view, ortho, scaled on the
head sphere (skin faces weighted to the Head bone) so every bust has the same scale whatever the
hat or hair, and framed low enough that the tallest hat or antenna still fits. Each rival faces its
own way and tilts to its temper (`POSE`); a key light, a rim light in the rival's colour, then a
background in that colour carrying a motif of its own (`MOTIF`: leaves, waves, flame rays, a
lightning zigzag, strata, stars, the six-sided seal pattern for the Warden).
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


# id: (camera side, -1 left / 1 right; roll in degrees; motif)
POSE = {
    "bramble": (1, 0, "leaves"), "marlo": (-1, -3, "waves"), "cinza": (1, 6, "rays"), "quill": (-1, 4, "zigzag"),
    "orla": (1, -2, "strata"), "lumen": (-1, -5, "stars"), "warden": (1, 0, "hexes"),
}


def motif(kind, xx, yy, rng):
    """A 0..1 mask over the unit square (rows bottom-up): the pattern drawn faintly behind the bust."""
    if kind == "rays":
        a = np.arctan2(yy - 0.55, xx - 0.5)
        return (np.sin(a * 14) > 0.35).astype(np.float32)
    if kind == "waves":
        return (np.sin((yy + 0.04 * np.sin(xx * 18)) * 40) > 0.55).astype(np.float32)
    if kind == "zigzag":
        tri = np.abs(((xx * 5) % 1) - 0.5) * 0.22
        return (np.abs(((yy + tri) * 7) % 1 - 0.5) < 0.12).astype(np.float32)
    if kind == "strata":
        return (((yy + 0.02 * np.sin(xx * 9)) * 9) % 1 < 0.16).astype(np.float32)
    if kind == "hexes":
        q = xx * 9
        r = yy * 9 / 0.866
        col = np.floor(q)
        row = np.floor(r + (col % 2) * 0.5)
        cx, cy = (col + 0.5) / 9, (row + 0.5 - (col % 2) * 0.5) * 0.866 / 9
        d = np.hypot(xx - cx, yy - cy) * 9
        return ((d > 0.42) & (d < 0.5)).astype(np.float32)
    m = np.zeros_like(xx)
    n, rad = (38, 0.022) if kind == "stars" else (26, 0.045)
    for _ in range(n):  # scattered dots (stars) or leaf ovals
        x0, y0, s = rng.uniform(0, 1), rng.uniform(0, 1), rng.uniform(0.5, 1.2)
        if kind == "stars":
            m = np.maximum(m, (np.hypot(xx - x0, yy - y0) < rad * s).astype(np.float32))
        else:
            t = rng.uniform(0, math.pi)
            u, v = (xx - x0) * math.cos(t) + (yy - y0) * math.sin(t), -(xx - x0) * math.sin(t) + (yy - y0) * math.cos(t)
            m = np.maximum(m, ((u / (rad * 1.8 * s)) ** 2 + (v / (rad * 0.8 * s)) ** 2 < 1).astype(np.float32))
    return m


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


def top_z(objs):
    """The highest evaluated vertex: hats, antennas, hair buns."""
    dg = bpy.context.evaluated_depsgraph_get()
    z = -1e9
    for o in objs:
        if o.type == "MESH":
            ev = o.evaluated_get(dg)
            me = ev.to_mesh()
            z = max([z] + [(ev.matrix_world @ v.co).z for v in me.vertices])
            ev.to_mesh_clear()
    return z


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
    side, roll, kind = POSE[rid]
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
              light("rim", "SUN", 9.0, (-65, 0, side * 160), color=tuple(0.55 + 0.45 * rgb(outer)))):  # behind: the rim, in their colour
        o.data.angle = math.radians(12)                                             # soft edges
    cam = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
    scn.collection.objects.link(cam)
    scn.camera = cam
    cam.data.type = "ORTHO"
    # Same scale for everyone (the head); the frame's top clears the tallest part with a margin.
    scale = r * 3.7
    top = top_z(objs) + r * 0.18
    cam.data.ortho_scale = scale
    target = c - Vector((0, 0, r * 0.3))
    target.z = min(target.z, top - scale * 0.5)
    d = Vector((side * 0.66, -0.75, 0.1)).normalized()                   # three-quarter, to the rival's side
    cam.location = target + d * 10
    cam.rotation_euler = (-d).to_track_quat("-Z", "Y").to_euler()
    cam.rotation_euler.rotate_axis("Z", math.radians(roll))
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
    m = motif(kind, xx, yy, np.random.default_rng(len(rid) * 7 + ord(rid[0])))[..., None]
    grad = grad * (1 - 0.14 * m) + rgb("#ffffff") * 0.1 * m * (1 - t)       # faint: darker lines, a little light near the head
    edge = np.clip((np.maximum(np.abs(xx - 0.5), np.abs(yy - 0.5)) - 0.44) / 0.06, 0, 1)[..., None]
    grad = grad * (1 - 0.25 * edge)                                           # a soft vignette at the frame
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
