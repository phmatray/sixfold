"""Build the Sixfold overworld props procedurally and export them as .glb.

Headless:
    blender -b --factory-startup -P tools/blender/build_assets.py -- \
        --out web/world/assets [--preview path/to/preview.png]

Everything is original, built from primitives in bmesh (no imported assets).
Blender is Z-up with the front facing -Y; the glTF exporter (+Y up) turns that
into Y-up with the front facing +Z, as web/world/ASSETS.md asks.
Grid: 1 tile = 1 m, origin at the footprint centre on the ground.

Shared file: run against art/Sixfold.blend (`blender -b art/Sixfold.blend -P ...` or live via
runpy). This script owns the scenes "Assets" and "Preview" only; it never touches the others.
"""
import contextlib
import math
import os
import random
import sys

import bmesh
import bpy
import numpy as np
from mathutils import Euler, Matrix, Vector

PREFIX = "sx_"
FPS = 24

# ---------------------------------------------------------------- helpers


def lin(hexstr):
    """sRGB hex -> linear RGBA (shader inputs are linear)."""
    out = []
    for i in (0, 2, 4):
        c = int(hexstr.lstrip("#")[i:i + 2], 16) / 255
        out.append(c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4)
    return (*out, 1.0)


PALETTE = {
    # house
    "plinth": "#9b8770", "wall": "#ecd09a", "timber": "#6e4b33", "roof": "#e9b43a",
    "roof_trim": "#9c6b24", "door": "#7fa868", "glass": "#8fc6ea", "frame": "#f6ecd2",
    "hex": "#2f6db5", "hex_in": "#f6d77e", "brick": "#b8664b",
    "knob": "#e2b84a", "step": "#b9ab95", "petal": "#f0855a",
    "petal_heart": "#fff2c4", "leaf": "#3f9a3a", "leaf_lt": "#62b84c", "leaf_dk": "#2f7d33",
    "stone": "#c9bfae", "brick_wall": "#a9553f", "plaster": "#f3ead6", "roof_teal": "#2f7f86",
    "roof_dk": "#1f565c", "door_navy": "#2c3e6b", "awning_red": "#c8473d", "cream": "#f6ecd2",
    "gold": "#e9b949", "roof_tiles": "#ffffff",
    # interior
    "leaf_xdk": "#24602a", "leaf_yl": "#8cc451", "fruit": "#f08a3c", "contact_in": "#1c3216", "contact_out": "#1c3216",
    "plaster_in": "#f1e2c4", "wainscot": "#9a6440", "wood_mid": "#a86f45", "wood_lt": "#c99263",
    "till": "#5f8a83", "till_dk": "#3f5f5a", "card_a": "#d9574a", "card_b": "#4f86c6", "card_c": "#e8b649",
    "card_d": "#6aa86b", "binder": "#7b5ea7", "card_face": "#f5efe0", "mat": "#2f6f6a", "mat_line": "#d9c98f",
    "cushion": "#c8473d", "rug": "#a8443a", "rug_border": "#e9cf94", "soil": "#4a3527", "shade": "#f6e3b4",
    "bulb": "#fff4c8", "mat_rug": "#7a5a3a", "glass_in": "#bfe3f5", "glass_case": "#d6eef8", "brick_dk": "#8c3f2e", "pot": "#c9714a",
    # nature
    "bark": "#7a5536", "stone": "#a39f92", "stone_dk": "#857f72", "dirt": "#d6a466",
    "dirt_dk": "#c08d55", "dirt_lt": "#e3b981", "pebble": "#b8a68c",
    "wood": "#a8754a", "wood_dk": "#7f5534", "board": "#f2e3c2",
    # hero
    "skin": "#f6cfae", "hair": "#4a2e6b", "hair_lt": "#6b4a94", "jacket": "#e8892f",
    "shirt": "#fff3d6", "shorts": "#2f4a6e", "shoe": "#3fae9a", "sole": "#f6ecd2",
    "eye": "#2a2238", "white": "#ffffff", "blush": "#f4a39a", "bag": "#8a5a3a",
    "grass": "#5cae45", "pack": "#ffd23f", "pack_dk": "#e0a722",
}


def material(key):
    name = PREFIX + key
    m = bpy.data.materials.get(name)
    if m is None:
        m = bpy.data.materials.new(name)
        m.use_nodes = True
    bsdf = next(n for n in m.node_tree.nodes if n.type == "BSDF_PRINCIPLED")
    bsdf.inputs["Base Color"].default_value = lin(PALETTE[key])
    bsdf.inputs["Roughness"].default_value = 0.9
    m.diffuse_color = lin(PALETTE[key])
    if key in ALPHA:
        bsdf.inputs["Alpha"].default_value = ALPHA[key]
        for attr, val in (("surface_render_method", "BLENDED"), ("blend_method", "BLEND")):
            try:
                setattr(m, attr, val)
            except (AttributeError, TypeError):
                pass
    if key in TEXTURES and not bsdf.inputs["Base Color"].links:
        tex = m.node_tree.nodes.new("ShaderNodeTexImage")
        tex.image = TEXTURES[key]()
        m.node_tree.links.new(tex.outputs["Color"], bsdf.inputs["Base Color"])
    return m


def scallop_tiles(name, base_hex, size=512, rows=4, cols=4, seed=9):
    """Painted roof tiles (base colour only): rounded bottoms, shadowed gaps, per-tile tint.
    One repeat = `cols` tiles across and `rows` rows up; v = 0 is the eave side."""
    img = bpy.data.images.get(name)
    if img:
        return img
    rng = np.random.default_rng(seed)
    yy, xx = np.mgrid[0:size, 0:size].astype(np.float32)
    rh, cw = size / rows, size / cols
    row = (yy // rh).astype(int)
    fy = (yy % rh) / rh                                    # 0 at the exposed (lower) edge of a row
    xo = xx + (row % 2) * cw / 2
    col = (xo // cw).astype(int) % cols
    fx = (xo % cw) / cw
    edge = 0.22 * (1 - np.sin(np.pi * fx))                 # scalloped lower edge
    tint = rng.normal(0, 0.05, (rows, cols))[row, col]
    shade = 0.78 + 0.3 * fy ** 0.7 + tint                  # darker where the row above overlaps
    shade = np.where(fy < edge, 0.55, shade)               # gap under the scallops
    shade = np.where((fy >= edge) & (fy < edge + 0.05), shade * 0.72, shade)   # outline
    shade = np.where((fx < 0.025) | (fx > 0.975), shade * 0.85, shade)          # side seams
    shade += rng.normal(0, 0.015, shade.shape)             # a little paint grain
    base = np.array([int(base_hex[i:i + 2], 16) / 255 for i in (1, 3, 5)], dtype=np.float32)  # byte image: sRGB values
    rgb = np.clip(base[None, None, :] * shade[..., None], 0, 1)
    px = np.concatenate([rgb, np.ones((size, size, 1), np.float32)], axis=2)
    img = bpy.data.images.new(name, size, size)
    img.colorspace_settings.name = "sRGB"
    img.pixels.foreach_set(px.astype(np.float32).ravel())
    img.pack()
    return img


ALPHA = {"glass_in": 0.45, "glass_case": 0.3, "contact_in": 0.32, "contact_out": 0.14}
TEXTURES = {"roof_tiles": lambda: scallop_tiles(PREFIX + "roof_tiles", "#3a929a")}


def T(loc=(0, 0, 0), rot=(0, 0, 0), scale=(1, 1, 1)):
    return (Matrix.Translation(loc) @ Euler([math.radians(a) for a in rot]).to_matrix().to_4x4()
            @ Matrix.Diagonal((*scale, 1)))


def bevel(bm, amount, seg=1):
    bmesh.ops.bevel(bm, geom=list(bm.verts) + list(bm.edges), offset=amount, segments=seg,
                    profile=0.5, affect="EDGES", clamp_overlap=True)
    return bm


def box(sx, sy, sz, bev=0.0, seg=1):
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)
    bmesh.ops.scale(bm, vec=(sx, sy, sz), verts=bm.verts)
    return bevel(bm, bev, seg) if bev else bm


def cyl(r1, r2, h, seg=8, bev=0.0):
    bm = bmesh.new()
    bmesh.ops.create_cone(bm, cap_ends=True, cap_tris=False, segments=seg,
                          radius1=r1, radius2=r2, depth=h)
    return bevel(bm, bev) if bev else bm


def sphere(r, u=10, v=6):
    bm = bmesh.new()
    bmesh.ops.create_uvsphere(bm, u_segments=u, v_segments=v, radius=r)
    return bm


def ico(r, sub=2):
    bm = bmesh.new()
    bmesh.ops.create_icosphere(bm, subdivisions=sub, radius=r)
    return bm


def prism(pts_xz, depth, bev=0.0):
    """Polygon in the XZ plane, extruded along Y (centred)."""
    bm = bmesh.new()
    front = [bm.verts.new((x, -depth / 2, z)) for x, z in pts_xz]
    back = [bm.verts.new((x, depth / 2, z)) for x, z in pts_xz]
    n = len(pts_xz)
    bm.faces.new(front[::-1])
    bm.faces.new(back)
    for i in range(n):
        j = (i + 1) % n
        bm.faces.new((front[i], front[j], back[j], back[i]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return bevel(bm, bev) if bev else bm


def jitter(bm, rng, amt, squash=1.0):
    for v in bm.verts:
        v.co += Vector((rng.uniform(-amt, amt), rng.uniform(-amt, amt), rng.uniform(-amt, amt) * squash))
    return bm


class Kit:
    """Collects bmesh parts and merges them into one mesh object (one material slot per colour)."""

    def __init__(self, name):
        self.name = name
        self.parts = []

    def add(self, bm, mat, M=None, smooth=False, bone=None, uv=None):
        """uv=(su, sv): planar UVs per face (u along the face's horizontal, v up its slope), in metres per repeat."""
        if M is not None:
            bmesh.ops.transform(bm, matrix=M, verts=bm.verts)
        self.parts.append((bm, mat, smooth, bone, uv))

    def build(self, coll):
        mats, bones = [], []
        out = bmesh.new()
        dl = out.verts.layers.deform.verify()
        uvl = out.loops.layers.uv.verify() if any(p[4] for p in self.parts) else None  # UVs only when textured
        for bm, mat, smooth, bone, uv in self.parts:
            if mat not in mats:
                mats.append(mat)
            if bone and bone not in bones:
                bones.append(bone)
            vmap = {}
            for v in bm.verts:
                nv = out.verts.new(v.co)
                if bone:
                    nv[dl][bones.index(bone)] = 1.0
                vmap[v] = nv
            for f in bm.faces:
                nf = out.faces.new([vmap[v] for v in f.verts])
                nf.material_index = mats.index(mat)
                nf.smooth = smooth
                if uv:
                    nf.normal_update()
                    n = nf.normal
                    t = Vector((1, 0, 0)) if abs(n.z) > 0.999 else Vector((0, 0, 1)).cross(n).normalized()
                    b = n.cross(t)
                    for lp in nf.loops:
                        lp[uvl].uv = (lp.vert.co.dot(t) / uv[0], lp.vert.co.dot(b) / uv[1])
            bm.free()
        me = bpy.data.meshes.new(PREFIX + self.name)
        out.to_mesh(me)
        out.free()
        for m in mats:
            me.materials.append(material(m))
        ob = bpy.data.objects.new(self.name, me)
        for b in bones:
            ob.vertex_groups.new(name=b)
        coll.objects.link(ob)
        return ob


def fcurves(action):
    """All F-curves of an action (Blender 5 keeps them in layered channelbags)."""
    return [fc for layer in action.layers for strip in layer.strips
            for bag in strip.channelbags for fc in bag.fcurves]


def add_wind(ob, weight):
    """Wind sway weight in the red channel of COLOR_0 (G = B = A = 1): 0 at the base, 1 at the tips."""
    me = ob.data
    attr = me.color_attributes.new("wind", "FLOAT_COLOR", "POINT")
    for v, c in zip(me.vertices, attr.data):
        x, y, z = v.co
        c.color = (max(0.0, min(1.0, weight(x, y, z, math.hypot(x, y)))), 1.0, 1.0, 1.0)
    me.color_attributes.active_color = attr


def clamp01(v):
    return max(0.0, min(1.0, v))


WIND = {  # (x, y, z, radius) -> weight
    "tree": lambda x, y, z, r: clamp01((z - 1.0) / 2.6 * 0.7 + r / 1.3 * 0.3) if z > 1.0 else 0.0,
    "bush": lambda x, y, z, r: clamp01(z / 0.15) * clamp01(z / 0.75 * 0.6 + r / 0.6 * 0.4),
    "flowers": lambda x, y, z, r: clamp01(z / 0.3),
    "grass_edge": lambda x, y, z, r: clamp01(z / 0.28),
}
WIND.update({"tree_b": WIND["tree"], "tree_c": WIND["tree"], "bush_dark": WIND["bush"]})


def tris(ob):
    return sum(len(p.vertices) - 2 for p in ob.data.polygons)


# ---------------------------------------------------------------- house (7 x 5 tiles)


def hip_roof(w, d, h):
    """Solid hipped roof: base w x d at z = 0, ridge along X at height h (equal pitch all round)."""
    bm = bmesh.new()
    rl = max(w - d, 0) / 2
    b = [bm.verts.new(c) for c in ((-w / 2, -d / 2, 0), (w / 2, -d / 2, 0), (w / 2, d / 2, 0), (-w / 2, d / 2, 0))]
    r0, r1 = bm.verts.new((-rl, 0, h)), bm.verts.new((rl, 0, h))
    for f in ((b[0], b[1], r1, r0), (b[1], b[2], r1), (b[2], b[3], r0, r1), (b[3], b[0], r0), (b[3], b[2], b[1], b[0])):
        bm.faces.new(f)
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return bm


def build_house(k):
    """Sixfold duel hall: hipped teal roof, hex corner turret with a spire, brick-and-plaster walls,
    navy door with a round window under a striped awning, a hanging hex sign. Footprint 7 x 5."""
    ww, wd = 6.8, 4.4        # body walls; the front wall sits 0.5 m back for the stoop
    by = 0.2                 # body centre y (back wall 0.1 m inside the back edge)
    fy = by - wd / 2         # front wall plane, y = -2.0 (Blender -Y = glTF +Z)
    wall_top = 2.9
    band = 1.2               # brick below, plaster above
    sill = 0.28              # plinth top = door sill; two 0.14 m steps lead up to it

    # plinth (pale stone, sunk 2 cm), brick lower walls, plaster upper walls, timber band and posts
    k.add(box(7.0, 2.5 - (fy - 0.05), sill + 0.02, 0.05), "stone", T((0, (2.5 + fy - 0.05) / 2, sill / 2 - 0.01)))
    k.add(box(ww + 0.04, wd + 0.04, band - 0.2), "brick_wall", T((0, by, 0.2 + (band - 0.2) / 2)))
    k.add(box(ww, wd, wall_top - band), "plaster", T((0, by, band + (wall_top - band) / 2)))
    k.add(box(ww + 0.1, wd + 0.1, 0.12, 0.03), "timber", T((0, by, band)))
    for sx in (-1, 1):
        for sy in (-1, 1):
            k.add(box(0.22, 0.22, wall_top - 0.3, 0.03), "timber",
                  T((sx * (ww / 2 - 0.04), by + sy * (wd / 2 - 0.04), 0.21 + (wall_top - 0.3) / 2)))
    k.add(box(ww + 0.12, wd + 0.12, 0.16, 0.03), "timber", T((0, by, wall_top - 0.02)))

    # hipped roof, deep teal, eaves 0.25 out (<= 0.15 m past the back edge of the footprint)
    pitch = math.radians(35)
    rw, rd = ww + 0.5, wd + 0.5
    rh = rd / 2 * math.tan(pitch)
    rz = wall_top + 0.04
    k.add(box(rw + 0.08, rd + 0.08, 0.12, 0.03), "roof_dk", T((0, by, rz)))   # eave fascia
    k.add(hip_roof(rw, rd, rh), "roof_tiles", T((0, by, rz)), uv=(1.2, 1.3))  # painted scallop tiles
    rl = (rw - rd) / 2
    for sx in (-1, 1):  # caps along the four hips, from the eave corners up to the ridge ends
        for sy in (-1, 1):
            a = Vector((sx * rw / 2, by + sy * rd / 2, rz))
            b = Vector((sx * rl, by, rz + rh))
            d = b - a
            cap = box(0.16, 0.16, d.length + 0.1, 0.04)
            rot = d.normalized().to_track_quat("Z", "Y").to_matrix().to_4x4()
            k.add(cap, "roof_dk", Matrix.Translation((a + b) / 2 + Vector((0, 0, 0.05))) @ rot)
    k.add(box(rw - rd + 0.3, 0.26, 0.16, 0.05), "roof_dk", T((0, by, rz + rh + 0.04)))  # ridge cap

    # hex turret on the front-right corner, poking through the roof, with a spire
    tc = Vector((2.35, -1.32, 0))
    tr, apo = 1.1, 1.1 * math.cos(math.radians(30))
    TURRET_ROT = (0, 0, 30)  # bmesh hexes have a corner at +Y; turn so a flat face looks to the front
    k.add(cyl(tr + 0.03, tr + 0.03, band - 0.18, 6), "brick_wall", T(tc + Vector((0, 0, 0.19 + (band - 0.18) / 2)), TURRET_ROT))
    k.add(cyl(tr, tr, 4.6 - band - 0.01, 6), "plaster", T(tc + Vector((0, 0, band + 0.01 + (4.6 - band - 0.01) / 2)), TURRET_ROT))
    k.add(cyl(tr + 0.08, tr + 0.08, 0.12, 6, 0.02), "timber", T(tc + Vector((0, 0, band + 0.005)), TURRET_ROT))
    k.add(cyl(tr + 0.1, tr + 0.1, 0.16, 6, 0.03), "timber", T(tc + Vector((0, 0, 4.6)), TURRET_ROT))
    k.add(cyl(tr + 0.3, tr + 0.24, 0.12, 6, 0.02), "roof_dk", T(tc + Vector((0, 0, 4.72)), TURRET_ROT))
    k.add(cyl(tr + 0.24, 0.0, 1.75, 6), "roof_tiles", T(tc + Vector((0, 0, 4.78 + 0.875)), TURRET_ROT), uv=(0.9, 1.0))
    k.add(sphere(0.11, 8, 5), "gold", T(tc + Vector((0, 0, 6.6)), TURRET_ROT), smooth=True)

    def on_face(theta, out, z, bm, mat, rot_x=90, smooth=False):
        """Put a part on the turret face whose outward normal is at angle theta (deg)."""
        n = Vector((math.cos(math.radians(theta)), math.sin(math.radians(theta)), 0))
        k.add(bm, mat, T(tc + n * (apo + out) + Vector((0, 0, z)), (rot_x, 0, theta + 90)), smooth=smooth)

    for th in (270, 330, 210):  # upper hex windows
        on_face(th, 0.03, 3.78, cyl(0.34, 0.34, 0.06, 6, 0.015), "timber")
        on_face(th, 0.06, 3.78, cyl(0.26, 0.26, 0.04, 6), "glass")
        on_face(th, 0.09, 3.78, box(0.04, 0.03, 0.44), "timber", rot_x=0)
    on_face(270, 0.03, 0.62 + 0.6, box(0.8, 0.1, 1.2, 0.02), "timber", rot_x=0)  # display window
    on_face(270, 0.07, 1.22, box(0.62, 0.04, 1.0), "glass", rot_x=0)
    on_face(270, 0.1, 1.22, box(0.64, 0.04, 0.05), "timber", rot_x=0)

    # shop window left of the door, above the timber band
    wx = -2.3
    k.add(box(1.4, 0.12, 1.2, 0.03), "timber", T((wx, fy - 0.03, 1.95)))
    k.add(box(1.2, 0.04, 1.0), "glass", T((wx, fy - 0.11, 1.95)))
    for mx in (-0.2, 0.2):
        k.add(box(0.05, 0.04, 1.04), "timber", T((wx + mx, fy - 0.15, 1.95)))
    k.add(box(1.55, 0.22, 0.1, 0.03), "stone", T((wx, fy - 0.1, 1.33)))  # ledge
    for sx in (-1,):  # one window on the left side wall
        k.add(box(0.12, 1.0, 0.9, 0.03), "timber", T((sx * (ww / 2 + 0.03), by + 0.4, 1.95)))
        k.add(box(0.04, 0.8, 0.7), "glass", T((sx * (ww / 2 + 0.11), by + 0.4, 1.95)))

    # stoop: two 0.14 m steps up to the sill, inside the front tile row
    k.add(box(1.8, 0.22, 0.16, 0.03), "stone", T((0, -2.39, 0.06)))
    k.add(box(1.5, 0.23, sill + 0.02, 0.03), "stone", T((0, -2.165, sill / 2 - 0.01)))

    # door frame (the leaf is its own node, below)
    k.add(box(1.16, 0.14, 2.2, 0.04), "timber", T((0, fy - 0.03, sill - 0.02 + 1.1)))

    # striped awning over the door, sloping out to y = -2.45, with a valance
    for i in range(5):
        x = -0.72 + 0.36 * i
        m = "awning_red" if i % 2 == 0 else "cream"
        k.add(box(0.36, 0.52, 0.05, 0.01), m, T((x, -2.225, 2.52), (28, 0, 0)))
        k.add(box(0.36, 0.04, 0.14, 0.01), m, T((x, -2.465, 2.33)))
    for sx in (-1, 1):  # awning arms
        k.add(box(0.04, 0.5, 0.04), "timber", T((sx * 0.92, -2.22, 2.48), (28, 0, 0)))

    # hanging hex sign on a bracket, between the shop window and the door
    sxp = -1.25
    k.add(box(0.06, 0.48, 0.06, 0.015), "timber", T((sxp, fy - 0.24, 2.62)))
    for cx in (-0.17, 0.17):
        k.add(box(0.025, 0.025, 0.24), "timber", T((sxp + cx, fy - 0.4, 2.47)))
    k.add(cyl(0.3, 0.3, 0.05, 6, 0.012), "roof_teal", T((sxp, fy - 0.4, 2.08), (90, 0, 0)))
    k.add(cyl(0.17, 0.17, 0.04, 6), "gold", T((sxp, fy - 0.43, 2.08), (90, 30, 0)))

    door = Kit("door")  # separate node, hinged on its -X edge; origin = hinge
    hinge = Vector((-0.46, fy - 0.08, sill))
    door.add(box(0.92, 0.12, 2.05, 0.03), "door_navy", T(Vector((0, fy - 0.08, sill + 1.025)) - hinge))
    door.add(cyl(0.2, 0.2, 0.03, 16), "gold", T(Vector((0, fy - 0.15, sill + 1.55)) - hinge, (90, 0, 0)))
    door.add(cyl(0.15, 0.15, 0.04, 16), "glass", T(Vector((0, fy - 0.17, sill + 1.55)) - hinge, (90, 0, 0)))
    door.add(box(0.7, 0.04, 0.06, 0.015), "timber", T(Vector((0, fy - 0.155, sill + 0.7)) - hinge))
    door.add(sphere(0.06, 8, 5), "gold", T(Vector((0.3, fy - 0.19, sill + 1.0)) - hinge), smooth=True)

    # chimney on the back-left slope
    cx, cy = -2.0, by + 1.15
    roof_at = rz + (rd / 2 - (cy - by)) * math.tan(pitch)
    k.add(box(0.55, 0.55, 1.4, 0.04), "brick", T((cx, cy, roof_at + 0.35)))          # sunk into the roof
    k.add(box(0.62, 0.62, 0.12, 0.03), "brick_dk", T((cx, cy, roof_at + 0.85)))      # corbel band
    k.add(box(0.66, 0.66, 0.1, 0.03), "stone", T((cx, cy, roof_at + 1.1)))
    for px in (-0.12, 0.12):                                                         # two clay pots
        k.add(cyl(0.08, 0.065, 0.2, 8), "pot", T((cx + px, cy, roof_at + 1.24)), smooth=True)
    smoke = Vector((cx - 0.12, cy, roof_at + 1.35))

    def after(house, coll):
        d = door.build(coll)
        d.parent = house
        d.location = hinge
        d.rotation_mode = "XYZ"
        d.animation_data_create()
        act = bpy.data.actions.new("Open")  # 0.42 s, swings 80 deg inward
        act.use_fake_user = True
        d.animation_data.action = act
        for f, a in ((0, 0), (10, 80)):
            d.rotation_euler = (0, 0, math.radians(a))
            d.keyframe_insert("rotation_euler", frame=f)
        tr = d.animation_data.nla_tracks.new()
        tr.name = "Open"  # glTF clip names come from the NLA track names
        tr.strips.new("Open", 0, act)
        d.animation_data.action = None
        d.rotation_euler = (0, 0, 0)
        e = bpy.data.objects.new("smoke_origin", None)
        e.empty_display_size = 0.2
        e.parent = house
        e.location = smoke
        coll.objects.link(e)
        return [d, e]
    return after


# ---------------------------------------------------------------- nature


def build_tree(k):
    rng = random.Random(2)
    k.add(cyl(0.24, 0.15, 1.9, 8, 0.03), "bark", T((0, 0, 0.95)), smooth=True)
    for a in range(0, 360, 90):  # root flares
        k.add(cyl(0.12, 0.03, 0.5, 5), "bark", T((0.2 * math.cos(math.radians(a)), 0.2 * math.sin(math.radians(a)), 0.1),
                                                 (0, 60, a), (1, 1, 1)).copy(), smooth=True)
    blobs = [((0, 0, 2.3), 1.05, "leaf"), ((0.55, -0.3, 2.05), 0.75, "leaf_dk"),
             ((-0.52, 0.2, 2.1), 0.75, "leaf_dk"), ((0.1, 0.55, 2.2), 0.7, "leaf"),
             ((-0.15, -0.35, 2.95), 0.75, "leaf_lt"), ((0.4, 0.25, 2.85), 0.6, "leaf_lt"),
             ((-0.45, -0.1, 2.75), 0.6, "leaf")]
    for loc, r, m in blobs:
        k.add(jitter(ico(r, 2), rng, r * 0.08), m, T(loc, scale=(1, 1, 0.88)), smooth=True)


def build_bush(k):
    rng = random.Random(3)
    for loc, r, m in [((0, 0, 0.36), 0.4, "leaf"), ((0.3, -0.12, 0.3), 0.3, "leaf_lt"),
                      ((-0.3, -0.08, 0.3), 0.32, "leaf"), ((0.05, 0.28, 0.32), 0.3, "leaf_dk"),
                      ((-0.1, -0.28, 0.25), 0.25, "leaf_lt"), ((0.12, -0.05, 0.52), 0.24, "leaf_lt")]:
        k.add(jitter(ico(r, 2), rng, r * 0.1), m, T(loc, scale=(1, 1, 0.9)), smooth=True)
    contact(k, 0.6)


def build_flowers(k):
    rng = random.Random(4)
    for i in range(7):  # leafy base
        a = i / 7 * math.tau
        k.add(jitter(ico(0.16, 1), rng, 0.02), "leaf" if i % 2 else "leaf_dk",
              T((0.24 * math.cos(a), 0.24 * math.sin(a), 0.07), scale=(1.2, 1.2, 0.55)), smooth=True)
    k.add(ico(0.2, 1), "leaf_dk", T((0, 0, 0.08), scale=(1.3, 1.3, 0.5)), smooth=True)
    heads = [(-0.18, -0.15, 0.22), (0.2, -0.1, 0.24), (0.0, 0.2, 0.26), (-0.25, 0.18, 0.2), (0.27, 0.22, 0.2)]
    for hx, hy, hz in heads:
        tilt = Euler((math.radians(-25), 0, 0)).to_matrix().to_4x4()
        base = Matrix.Translation((hx, hy, hz)) @ tilt
        for p in range(5):
            a = p / 5 * math.tau + rng.uniform(0, 0.3)
            k.add(sphere(0.065, 8, 4), "petal",
                  base @ T((0.07 * math.cos(a), 0.07 * math.sin(a), 0), (0, 0, math.degrees(a)), (1.3, 0.85, 0.4)),
                  smooth=True)
        k.add(sphere(0.04, 8, 4), "petal_heart", base @ T((0, 0, 0.02), scale=(1, 1, 0.6)), smooth=True)
    contact(k, 0.45)


def contact(k, r):
    """Soft contact shadow just above the ground: an inner disc and an outer ring, translucent and dark,
    side by side (never overlapping, never coplanar with y = 0)."""
    seg = 20
    k.add(cyl(r * 0.6, r * 0.6, 0.002, seg), "contact_in", T((0, 0, 0.004)))
    bm = bmesh.new()
    inner = [bm.verts.new((r * 0.6 * math.cos(a), r * 0.6 * math.sin(a), 0.008)) for a in (i / seg * math.tau for i in range(seg))]
    outer = [bm.verts.new((r * math.cos(a), r * math.sin(a), 0.008)) for a in (i / seg * math.tau for i in range(seg))]
    for i in range(seg):
        j = (i + 1) % seg
        bm.faces.new((inner[i], outer[i], outer[j], inner[j]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    for f in bm.faces:
        if f.normal.z < 0:
            f.normal_flip()
    k.add(bm, "contact_out")


def build_tree_b(k):
    """Tall tiered tree (~4.3 m), darker foliage."""
    rng = random.Random(12)
    k.add(cyl(0.2, 0.12, 2.4, 8, 0.03), "bark", T((0, 0, 1.2)), smooth=True)
    for a in range(0, 360, 120):
        k.add(cyl(0.1, 0.03, 0.45, 5), "bark", T((0.17 * math.cos(math.radians(a)), 0.17 * math.sin(math.radians(a)), 0.1), (0, 60, a)), smooth=True)
    for z, r, n, m in ((1.9, 1.0, 5, "leaf_dk"), (2.85, 0.78, 4, "leaf"), (3.65, 0.55, 3, "leaf")):
        for i in range(n):
            a = i / n * math.tau + rng.uniform(0, 0.5)
            k.add(jitter(ico(r * 0.62, 2), rng, r * 0.05), m if i % 2 else "leaf_xdk",
                  T((r * 0.45 * math.cos(a), r * 0.45 * math.sin(a), z), scale=(1, 1, 0.8)), smooth=True)
    k.add(jitter(ico(0.32, 2), rng, 0.02), "leaf_lt", T((0, 0, 4.05), scale=(1, 1, 0.9)), smooth=True)


def build_tree_c(k):
    """Short wide tree (~3 m), light foliage with a few fruit."""
    rng = random.Random(13)
    k.add(cyl(0.26, 0.17, 1.4, 8, 0.03), "bark", T((0, 0, 0.7)), smooth=True)
    for a in (30, 210):  # two forked limbs
        k.add(cyl(0.1, 0.06, 0.8, 6), "bark", T((0.25 * math.cos(math.radians(a)), 0.25 * math.sin(math.radians(a)), 1.5), (0, 35, a)), smooth=True)
    blobs = [((0, 0, 2.15), 0.95, "leaf_lt"), ((0.65, -0.2, 1.95), 0.6, "leaf"), ((-0.65, 0.15, 1.95), 0.62, "leaf"),
             ((0.1, 0.62, 2.0), 0.55, "leaf_lt"), ((-0.2, -0.55, 2.05), 0.55, "leaf_yl"), ((0.2, 0.0, 2.6), 0.6, "leaf_yl")]
    for loc, r, m in blobs:
        k.add(jitter(ico(r, 2), rng, r * 0.07), m, T(loc, scale=(1.1, 1.1, 0.75)), smooth=True)
    for i in range(7):
        a = i / 7 * math.tau + rng.uniform(0, 0.4)
        k.add(sphere(0.06, 8, 5), "fruit", T((0.95 * math.cos(a), 0.95 * math.sin(a), 1.75 + rng.uniform(0, 0.4))), smooth=True)


def build_bush_dark(k):
    rng = random.Random(14)
    for loc, r, m in [((0, 0, 0.38), 0.42, "leaf_xdk"), ((0.32, -0.12, 0.3), 0.32, "leaf_dk"),
                      ((-0.32, -0.06, 0.3), 0.33, "leaf_xdk"), ((0.05, 0.3, 0.32), 0.32, "leaf_dk"),
                      ((-0.1, -0.3, 0.26), 0.27, "leaf_dk"), ((0.1, -0.05, 0.56), 0.26, "leaf")]:
        k.add(jitter(ico(r, 2), rng, r * 0.1), m, T(loc, scale=(1, 1, 0.9)), smooth=True)
    contact(k, 0.62)


def build_grass_edge(k):
    """A 1 m strip of grass blades along X (to soften grass/path borders), wind weighted."""
    rng = random.Random(15)
    for i in range(46):
        x = -0.48 + 0.96 * i / 45 + rng.uniform(-0.01, 0.01)
        h = rng.uniform(0.1, 0.28)
        k.add(cyl(0.022, 0.0, h, 3), rng.choice(["leaf", "leaf_lt", "leaf_dk", "leaf_yl"]),
              T((x, rng.uniform(-0.07, 0.07), h / 2 - 0.01), (rng.uniform(-20, 20), rng.uniform(-20, 20), rng.uniform(0, 120))),
              smooth=True)


def build_shoes(k):
    """Pair of original running shoes for the item popup (~0.3 m), centred on the origin."""
    for sx, yaw in ((-0.075, 8), (0.075, -4)):
        M = Matrix.Translation((sx, 0, -0.045)) @ Euler((0, 0, math.radians(yaw))).to_matrix().to_4x4()
        k.add(box(0.1, 0.27, 0.035, 0.012), "sole", M @ T((0, 0, 0.0175)))
        k.add(box(0.11, 0.08, 0.015, 0.005), "pack", M @ T((0, 0.07, 0.006)))           # heel stripe in the sole
        k.add(box(0.095, 0.25, 0.07, 0.03, 2), "shoe", M @ T((0, 0.005, 0.07)))         # upper
        k.add(box(0.09, 0.09, 0.06, 0.03, 2), "shoe", M @ T((0, 0.075, 0.12)))          # ankle collar
        k.add(box(0.06, 0.12, 0.012, 0.004), "cream", M @ T((0, -0.04, 0.108)))         # tongue
        for j in range(3):                                                             # laces
            k.add(box(0.07, 0.012, 0.01), "awning_red", M @ T((0, -0.075 + j * 0.035, 0.112)))
        for side in (-1, 1):                                                           # hex on both sides
            k.add(cyl(0.022, 0.022, 0.006, 6), "pack", M @ T((side * 0.049, 0.02, 0.075), (0, 90, 0)))


def build_rock(k):
    rng = random.Random(5)
    k.add(jitter(ico(0.5, 2), rng, 0.07), "stone", T((0.02, 0.05, 0.22), scale=(0.9, 0.82, 0.62)))
    k.add(jitter(ico(0.22, 1), rng, 0.04), "stone_dk", T((-0.3, -0.22, 0.08), scale=(1, 0.9, 0.6)))
    k.add(jitter(ico(0.14, 1), rng, 0.03), "stone", T((0.32, -0.28, 0.05), scale=(1, 1, 0.6)))
    contact(k, 0.52)


def build_sign(k):
    k.add(box(0.13, 0.13, 1.07, 0.03), "wood_dk", T((0, 0.05, 0.515)))
    k.add(box(0.86, 0.1, 0.56, 0.04), "wood", T((0, -0.03, 1.0)))
    k.add(box(0.74, 0.04, 0.44, 0.02), "board", T((0, -0.09, 1.0)))
    k.add(box(0.96, 0.18, 0.08, 0.03), "wood_dk", T((0, -0.03, 1.31)))
    k.add(cyl(0.12, 0.12, 0.03, 6, 0.008), "hex", T((-0.2, -0.115, 1.03), (90, 30, 0)))
    k.add(cyl(0.065, 0.065, 0.03, 6), "hex_in", T((-0.2, -0.135, 1.03), (90, 30, 0)))
    for i, w in enumerate((0.3, 0.22, 0.26)):  # abstract "text" strokes, no letters
        k.add(box(w, 0.02, 0.035), "wood_dk", T((0.03 + w / 2, -0.115, 1.12 - i * 0.09)))
    k.add(box(0.3, 0.3, 0.065, 0.02), "stone_dk", T((0, 0.05, 0.0075)))
    contact(k, 0.32)


FENCE_ARMS = {"fence": "WE", "fence_corner": "WS", "fence_end": "W", "fence_t": "WES"}
ARM_DIR = {"W": (-1, 0), "E": (1, 0), "S": (0, -1), "N": (0, 1)}  # S = glTF +Z = Blender -Y
TURN = {"S": "E", "E": "N", "N": "W", "W": "S"}  # +90 deg about glTF +Y (CCW from above)


def fence_pick(nbrs):
    """Fence piece(s) and rot for a tile whose fence neighbours are `nbrs` (set of N/E/S/W)."""
    if len(nbrs) == 4:
        return [("fence_t", 0), ("fence_end", 270)]
    if not nbrs:
        return [("fence", 0)]
    for piece, arms in FENCE_ARMS.items():
        a = set(arms)
        for r in range(4):
            if a == nbrs:
                return [(piece, 90 * r)]
            a = {TURN[d] for d in a}


def build_fence_piece(k, arms, post):
    """Fence line through the tile centre; each arm reaches the tile edge so neighbours chain."""
    for arm in arms:
        dx, dy = ARM_DIR[arm]
        for z in (0.3, 0.62):  # rails
            k.add(box(0.47 if dx else 0.07, 0.07 if dx else 0.47, 0.12, 0.015), "wood", T((dx * 0.265, dy * 0.265, z)))
        # one picket per arm, just in front of the rails (never coplanar with them)
        k.add(box(0.09 if dx else 0.05, 0.05 if dx else 0.09, 0.66, 0.015), "wood",
              T((dx * 0.27 + (0.06 if dy else 0), dy * 0.27 - (0.06 if dx else 0), 0.42)))
    w, h = {"std": (0.13, 0.84), "t": (0.15, 0.9), "corner": (0.17, 0.98)}[post]
    k.add(box(w, w, h, 0.025), "wood_dk", T((0, 0, h / 2 - 0.02)))  # sunk 2 cm
    if post != "std":
        k.add(box(w + 0.05, w + 0.05, 0.05, 0.015), "wood_dk", T((0, 0, h - 0.0)))
    k.add(cyl(w * 0.75, 0.0, 0.12, 4), "wood_dk", T((0, 0, h + 0.07), (0, 0, 45)))


# ---------------------------------------------------------------- interior kit (web/world/assets/interior/)

WALL_T, WAIN, WAIN_T = 0.25, 0.9, 0.29   # wall thickness, wainscot height and thickness


def build_wall(k, arms, height):
    """Wall line through the tile centre, arms to the tile edges like the fence (fence_pick rules).
    Plaster core, wood wainscot proud on both sides, timber cap on top. A centre block joins the arms
    and each arm starts at its edge, so no two top faces overlap (the camera looks in from above)."""
    c = WALL_T / 2
    wh = min(WAIN, height - 0.1)  # always a plaster band above the wainscot (low walls too)
    k.add(box(WALL_T, WALL_T, height + 0.01), "plaster_in", T((0, 0, height / 2 - 0.005)))
    k.add(box(WAIN_T, WAIN_T, wh + 0.015), "wainscot", T((0, 0, wh / 2 - 0.0075)))
    k.add(box(WALL_T + 0.07, WALL_T + 0.07, 0.07, 0.015), "timber", T((0, 0, height + 0.035)))
    for arm in arms:
        dx, dy = ARM_DIR[arm]
        for z0, z1, mat, th in ((-0.01, height, "plaster_in", WALL_T), (-0.015, wh, "wainscot", WAIN_T),
                                (height, height + 0.07, "timber", WALL_T + 0.07)):
            ln, m = 0.5 - th / 2, (0.5 + th / 2) / 2  # from the centre block's face to the tile edge
            sx, sy = (ln, th) if dx else (th, ln)
            k.add(box(sx, sy, z1 - z0), mat, T((dx * m, dy * m, (z0 + z1) / 2)))


def build_wall_window(k, height=2.6):
    """Straight wall (arms -X/+X) with a glazed window 1.0 .. 2.0 m high, 0.9 m wide."""
    for z0, z1 in ((-0.01, 1.0), (2.0, height)):
        k.add(box(1.0, WALL_T, z1 - z0), "plaster_in", T((0, 0, (z0 + z1) / 2)))
    for x in (-0.475, 0.475):
        k.add(box(0.05, WALL_T, 1.0), "plaster_in", T((x, 0, 1.5)))
    k.add(box(1.0, WAIN_T, WAIN + 0.015), "wainscot", T((0, 0, WAIN / 2 - 0.0075)))
    k.add(box(1.0, WALL_T + 0.07, 0.07), "timber", T((0, 0, height + 0.035)))
    k.add(box(0.9, 0.04, 0.9), "glass_in", T((0, 0, 1.5)))
    for x in (-0.43, 0.43):
        k.add(box(0.06, WALL_T + 0.04, 0.99), "timber", T((x, 0, 1.5)))
    for z in (1.035, 1.965):  # rails sit 5 mm off the plaster faces
        k.add(box(0.8, WALL_T + 0.04, 0.06), "timber", T((0, 0, z)))
    k.add(box(0.05, 0.08, 0.88), "timber", T((0, 0, 1.5)))
    k.add(box(1.0, WALL_T + 0.18, 0.05, 0.015), "wainscot", T((0, 0, 0.985)))  # sill, 1 cm above the plaster


def build_counter(k):
    k.add(box(2.8, 0.7, 0.98, 0.03), "wood_mid", T((0, 0.05, 0.47)))
    k.add(box(2.6, 0.04, 0.7, 0.02), "wood_dk", T((0, -0.315, 0.5)))               # front panel
    for x in (-0.87, 0.0, 0.87):
        k.add(box(0.06, 0.06, 0.7, 0.015), "timber", T((x, -0.34, 0.5)))           # panel posts
    k.add(box(2.92, 0.82, 0.06, 0.02), "wood_lt", T((0, 0.05, 0.99)))              # top
    k.add(box(0.36, 0.3, 0.16, 0.03), "till", T((0.8, 0.12, 1.1)))                 # plain till
    k.add(prism([(-0.17, 0), (0.17, 0), (0.17, 0.08), (-0.17, 0.16)], 0.26, 0.01), "till_dk",
          T((0.8, 0.12, 1.18), (0, 0, 90)))
    rng = random.Random(21)
    for i in range(3):  # stack of card boxes
        k.add(box(0.2, 0.14, 0.07, 0.01), rng.choice(["card_a", "card_b", "card_c"]),
              T((-0.9, 0.1, 1.06 + 0.075 * i), (0, 0, rng.uniform(-12, 12))))


def build_shelf(k):
    """2 x 1, backed against the +Y (north) wall: frame, four shelves, boxes and binders."""
    rng = random.Random(22)
    w, d, h, y = 1.9, 0.42, 2.0, 0.27
    for sx in (-1, 1):
        k.add(box(0.06, d, h, 0.015), "wood_mid", T((sx * (w / 2 - 0.03), y, h / 2 - 0.01)))
    k.add(box(w - 0.12, 0.03, h, 0.01), "wood_dk", T((0, y + d / 2 - 0.02, h / 2 - 0.01)))
    for i, z in enumerate((0.09, 0.55, 1.02, 1.49, 1.96)):
        k.add(box(w - 0.12, d - 0.03, 0.04), "wood_lt", T((0, y - 0.015, z)))
        if i == 4:
            continue
        x = -w / 2 + 0.1
        while x < w / 2 - 0.15:
            bw = rng.uniform(0.06, 0.16)
            bh = rng.uniform(0.22, 0.38)
            tilt = rng.choice([0, 0, 0, 8])
            k.add(box(bw - 0.01, rng.uniform(0.22, 0.32), bh), rng.choice(["card_a", "card_b", "card_c", "card_d", "binder"]),
                  T((x + bw / 2, y - 0.03, z + 0.02 + bh / 2), (0, tilt, 0)))
            x += bw + rng.uniform(0.0, 0.04)


def build_duel_table(k):
    k.add(box(1.86, 0.9, 0.06, 0.02), "wood_mid", T((0, 0, 0.75)))
    for sx in (-1, 1):
        for sy in (-1, 1):
            k.add(box(0.07, 0.07, 0.74, 0.015), "wood_dk", T((sx * 0.84, sy * 0.36, 0.36)))
    k.add(box(1.7, 0.76, 0.01), "mat", T((0, 0, 0.785)))                           # play mat
    for sx in (-1, 1):  # zones: thin raised outlines, 3 mm above the mat
        k.add(box(0.5, 0.5, 0.004), "mat_line", T((sx * 0.5, 0, 0.792)))
        k.add(box(0.44, 0.44, 0.004), "mat", T((sx * 0.5, 0, 0.796)))
        k.add(cyl(0.11, 0.11, 0.004, 6), "mat_line", T((sx * 0.5, 0, 0.8)))
        for j in range(3):  # a few cards laid out
            k.add(box(0.09, 0.13, 0.006), "card_face", T((sx * (0.12 + 0.1 * j), -0.25 + 0.03 * j, 0.804 + 0.002 * j), (0, 0, 5 * j)))


def build_chair(k):
    k.add(box(0.46, 0.46, 0.06, 0.02), "wood_mid", T((0, 0, 0.46)))
    for sx in (-1, 1):
        for sy in (-1, 1):
            k.add(box(0.05, 0.05, 0.46, 0.01), "wood_dk", T((sx * 0.19, sy * 0.19, 0.22)))
    k.add(box(0.46, 0.05, 0.5, 0.02), "wood_mid", T((0, 0.21, 0.74)))               # backrest (behind, +Y)
    k.add(box(0.36, 0.36, 0.03, 0.01), "cushion", T((0, -0.01, 0.505)))


def build_rug(k):
    """3 x 2 flat rug: base top at 0.015 (bottom 1 cm under the floor), border and hex motif above it."""
    k.add(box(2.7, 1.7, 0.025, 0.008), "rug", T((0, 0, 0.0025)))
    k.add(box(2.5, 1.5, 0.004), "rug_border", T((0, 0, 0.0165)))
    k.add(box(2.36, 1.36, 0.004), "rug", T((0, 0, 0.0185)))
    k.add(cyl(0.42, 0.42, 0.004, 6), "rug_border", T((0, 0, 0.0205)))
    k.add(cyl(0.3, 0.3, 0.004, 6), "gold", T((0, 0, 0.0225)))


def build_plant(k):
    rng = random.Random(23)
    k.add(cyl(0.2, 0.15, 0.36, 10, 0.02), "pot", T((0, 0, 0.17)), smooth=True)
    k.add(cyl(0.22, 0.22, 0.05, 10, 0.015), "pot", T((0, 0, 0.35)), smooth=True)
    k.add(cyl(0.18, 0.18, 0.02, 10), "soil", T((0, 0, 0.37)))
    for i in range(7):
        a = i / 7 * math.tau
        k.add(jitter(ico(0.15, 1), rng, 0.02), "leaf" if i % 2 else "leaf_lt",
              T((0.12 * math.cos(a), 0.12 * math.sin(a), 0.6 + 0.1 * (i % 3)), (rng.uniform(-30, 30), 0, math.degrees(a)), (0.7, 1.4, 0.5)),
              smooth=True)
    k.add(jitter(ico(0.17, 1), rng, 0.02), "leaf_dk", T((0, 0, 0.82), scale=(1, 1, 0.9)), smooth=True)


def build_display_case(k):
    k.add(box(0.82, 0.62, 0.86, 0.03), "wood_mid", T((0, 0, 0.42)))
    k.add(box(0.86, 0.66, 0.04, 0.015), "wood_lt", T((0, 0, 0.87)))
    for i, (x, m) in enumerate(((-0.22, "card_a"), (0.0, "card_face"), (0.22, "card_c"))):
        k.add(box(0.13, 0.18, 0.006), m, T((x, 0.02 * i - 0.02, 0.9 + 0.002 * i), (0, 0, (i - 1) * 8)))
    k.add(box(0.78, 0.58, 0.3, 0.01), "glass_case", T((0, 0, 1.045)))
    for sx in (-1, 1):
        for sy in (-1, 1):
            k.add(box(0.04, 0.04, 0.32), "timber", T((sx * 0.39, sy * 0.29, 1.04)))


def build_lamp(k):
    k.add(cyl(0.18, 0.2, 0.05, 12, 0.015), "timber", T((0, 0, 0.015)))
    k.add(cyl(0.025, 0.025, 1.5, 8), "gold", T((0, 0, 0.78)), smooth=True)
    k.add(cyl(0.3, 0.17, 0.32, 12), "shade", T((0, 0, 1.55)), smooth=True)
    k.add(sphere(0.07, 8, 5), "bulb", T((0, 0, 1.46)), smooth=True)

    def after(lamp, coll):
        e = bpy.data.objects.new("light_origin", None)  # the web places its point light here
        e.parent = lamp
        e.location = (0, 0, 1.46)
        coll.objects.link(e)
        return [e]
    return after


def build_exit_mat(k):
    k.add(box(0.9, 0.6, 0.025, 0.008), "mat_rug", T((0, 0, 0.0025)))
    for i in range(5):  # bristle stripes, 3 mm above the mat top
        k.add(box(0.7, 0.05, 0.004), "rug_border", T((0, -0.2 + i * 0.1, 0.0165)))


def dirt_tile(top=0.02, edge=-0.01, inset=0.045):
    """1 m square raised dirt tile, no bottom face: flat top at `top`, edges sloping into the ground."""
    bm = bmesh.new()
    h = 0.5
    i = h - inset
    outer = [bm.verts.new(c) for c in ((-h, -h, edge), (h, -h, edge), (h, h, edge), (-h, h, edge))]
    inner = [bm.verts.new(c) for c in ((-i, -i, top), (i, -i, top), (i, i, top), (-i, i, top))]
    bm.faces.new(inner)
    for n in range(4):
        m = (n + 1) % 4
        bm.faces.new((outer[n], outer[m], inner[m], inner[n]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return bm


def build_path(k, seed):
    """1 m square dirt tile; top 2 cm above the ground, nothing at y = 0 to z-fight with grass.
    Details sit >= 6 mm above the top; everything stays under 3 cm."""
    rng = random.Random(seed)
    k.add(dirt_tile(), "dirt")
    for i in range(rng.randint(2, 3)):  # lighter / darker worn patches, 3 mm apart in height
        bm = bmesh.new()
        r = rng.uniform(0.12, 0.2)
        cx, cy = rng.uniform(-0.22, 0.22), rng.uniform(-0.22, 0.22)
        bmesh.ops.create_circle(bm, cap_ends=True, segments=9, radius=r)
        jitter(bm, rng, r * 0.25, squash=0)
        k.add(bm, rng.choice(["dirt_lt", "dirt_dk"]), T((cx, cy, 0.023 + 0.003 * i), scale=(rng.uniform(1, 1.4), 1, 1)))
    for _ in range(rng.randint(3, 5)):  # pebbles
        r = rng.uniform(0.03, 0.05)
        k.add(jitter(ico(r, 1), rng, r * 0.2), "pebble",
              T((rng.uniform(-0.36, 0.36), rng.uniform(-0.36, 0.36), 0.016), scale=(1, 0.8, 0.22)))


# ---------------------------------------------------------------- hero

HEAD_SCALE, NECK = 1.1, Vector((0, 0, 0.62))  # bigger chibi head (about 1:1.6), grown from the neck
HERO_SCALE = 1.45 / 1.2  # modelled at ~1.2 m, shipped at ~1.45 m

BONES = {  # name: (head, tail, parent) in metres, all pointing up so local axes = world axes
    "Root": ((0, 0, 0), (0, 0, 0.15), None),
    "Hips": ((0, 0, 0.32), (0, 0, 0.42), "Root"),
    "Torso": ((0, 0, 0.38), (0, 0, 0.6), "Hips"),
    "Head": ((0, 0, 0.62), (0, 0, 0.9), "Torso"),
    "Leg.L": ((0.085, 0, 0.3), (0.085, 0, 0.4), "Hips"),
    "Leg.R": ((-0.085, 0, 0.3), (-0.085, 0, 0.4), "Hips"),
    "Arm.L": ((0.2, 0, 0.56), (0.2, 0, 0.66), "Torso"),
    "Arm.R": ((-0.2, 0, 0.56), (-0.2, 0, 0.66), "Torso"),
    "Eye.L": ((0.085, -0.2, 0.82), (0.085, -0.2, 0.87), "Head"),   # scaled on Y (world Z) to blink
    "Eye.R": ((-0.085, -0.2, 0.82), (-0.085, -0.2, 0.87), "Head"),
}


def build_hero_mesh(k):
    """Original chibi: violet bob with a curl and a short tied tuft at the back, orange open
    jacket over a cream shirt with a small hex badge, navy shorts, teal sneakers."""
    for s, b in ((1, "Leg.L"), (-1, "Leg.R")):
        x = s * 0.085
        k.add(box(0.13, 0.2, 0.09, 0.035, 2), "shoe", T((x, -0.025, 0.055)), bone=b)
        k.add(box(0.14, 0.21, 0.035, 0.01), "sole", T((x, -0.025, 0.0075)), bone=b)
        k.add(cyl(0.05, 0.055, 0.2, 8), "skin", T((x, 0, 0.18)), smooth=True, bone=b)
        k.add(cyl(0.075, 0.08, 0.1, 8), "shorts", T((x, 0, 0.29)), smooth=True, bone=b)
    k.add(box(0.31, 0.2, 0.13, 0.05, 2), "shorts", T((0, 0, 0.36)), bone="Hips")
    k.add(cyl(0.135, 0.16, 0.24, 10, 0.03), "jacket", T((0, 0, 0.5), scale=(1.05, 0.8, 1)), smooth=True, bone="Torso")
    k.add(box(0.11, 0.03, 0.22, 0.012), "shirt", T((0, -0.124, 0.5)), bone="Torso")        # shirt in the open jacket
    k.add(cyl(0.028, 0.028, 0.012, 6), "hex_in", T((0, -0.142, 0.54), (90, 30, 0)), bone="Torso")  # hex badge
    # bright backpack: the accent that reads from behind and far away at phone size
    k.add(box(0.24, 0.12, 0.23, 0.045, 2), "pack", T((0, 0.15, 0.5)), bone="Torso")
    k.add(box(0.18, 0.04, 0.09, 0.02), "pack_dk", T((0, 0.215, 0.45)), bone="Torso")       # front pocket
    k.add(cyl(0.035, 0.035, 0.012, 6), "shoe", T((0, 0.24, 0.53), (90, 30, 0)), bone="Torso")  # hex badge
    for sx in (-1, 1):  # straps over the shoulders
        k.add(box(0.035, 0.03, 0.2, 0.01), "shoe", T((sx * 0.085, -0.13, 0.5)), bone="Torso")
        k.add(box(0.035, 0.26, 0.03, 0.01), "shoe", T((sx * 0.085, 0.0, 0.615)), bone="Torso")
    k.add(sphere(0.13, 10, 5), "jacket", T((0, 0.08, 0.6), scale=(1.2, 0.7, 0.5)), smooth=True, bone="Torso")  # hood
    for s, b in ((1, "Arm.L"), (-1, "Arm.R")):
        x = s * 0.2
        k.add(cyl(0.05, 0.045, 0.2, 8), "jacket", T((x, 0, 0.47), (0, s * 8, 0)), smooth=True, bone=b)
        k.add(sphere(0.05, 8, 5), "skin", T((x + s * 0.015, 0, 0.35)), smooth=True, bone=b)
    # head
    k.add(sphere(0.23, 14, 9), "skin", T((0, 0, 0.84), scale=(1, 0.95, 0.95)), smooth=True, bone="Head")
    for s, eb in ((1, "Eye.L"), (-1, "Eye.R")):
        k.add(sphere(0.048, 8, 6), "eye", T((s * 0.085, -0.203, 0.82), scale=(0.8, 0.5, 1.2)), smooth=True, bone=eb)
        k.add(sphere(0.016, 6, 4), "white", T((s * 0.085 + 0.013, -0.226, 0.85)), smooth=True, bone=eb)
        k.add(sphere(0.03, 8, 4), "blush", T((s * 0.14, -0.18, 0.76), scale=(1.2, 0.4, 0.6)), smooth=True, bone="Head")
    k.add(sphere(0.022, 8, 4), "eye", T((0, -0.221, 0.735), scale=(1.5, 0.4, 0.55)), smooth=True, bone="Head")  # mouth
    # hair: cap tilted back (forehead visible from the game camera) + side bob + short fringe
    # + a curl on top + a short tied tuft at the back (silhouette from behind and the side)
    cap = sphere(0.25, 14, 9)
    bmesh.ops.bisect_plane(cap, geom=list(cap.verts) + list(cap.edges) + list(cap.faces),
                           plane_co=(0, 0, -0.02), plane_no=(0, 0, -1), clear_outer=True)
    k.add(cap, "hair", T((0, 0.03, 0.87), (-22, 0, 0)), smooth=True, bone="Head")
    for s in (1, -1):
        k.add(sphere(0.12, 10, 6), "hair", T((s * 0.17, 0.03, 0.78), scale=(0.55, 1.25, 1.25)), smooth=True, bone="Head")
    for i, x in enumerate((-0.12, -0.04, 0.05, 0.13)):
        k.add(sphere(0.068, 8, 5), "hair_lt" if i == 2 else "hair", T((x, -0.165, 1.0 - abs(x) * 0.3), (30, 0, x * 120), (1, 0.55, 0.8)),
              smooth=True, bone="Head")
    k.add(sphere(0.085, 8, 6), "hair", T((0, 0.25, 0.8), (-35, 0, 0), (0.85, 1.35, 0.8)), smooth=True, bone="Head")  # tuft
    k.add(cyl(0.045, 0.045, 0.04, 8), "shoe", T((0, 0.2, 0.82), (55, 0, 0)), smooth=True, bone="Head")          # hair tie
    k.add(sphere(0.21, 12, 7), "hair", T((0, 0.07, 0.79), scale=(1.12, 0.95, 1.05)), smooth=True, bone="Head")
    k.add(cyl(0.03, 0.0, 0.11, 6), "hair_lt", T((0.02, 0.0, 1.12), (-35, 15, 0)), smooth=True, bone="Head")


def build_hero(coll):
    kit = Kit("hero")
    build_hero_mesh(kit)
    return rig_character(kit, coll, "hero", HERO_SCALE)


HIP = 0.32  # model-space hip line: legs below it stretch with `leg`, everything above moves up


def rig_character(kit, coll, name, S, head_scale=HEAD_SCALE, leg=1.0,
                  clips=("Idle", "Fidget", "Walk", "Run"), pose=None, arm_swing=1.0):
    """Shared by the hero and the NPC variants: same 10-bone rig, same clips.
    head_scale grows the head from the neck, leg stretches the legs (1.0 = hero proportions),
    pose = {bone: (loc, rot)} static offsets added to Idle and Fidget (pockets, confident stance),
    arm_swing scales the Walk/Run arm swing."""
    head_bones = ("Head", "Eye.L", "Eye.R")

    def zmap(v):
        return Vector((v.x, v.y, v.z * leg if v.z <= HIP else v.z + HIP * (leg - 1)))
    for bm, _, _, bone, _ in kit.parts:
        if bone in head_bones:
            bmesh.ops.transform(bm, matrix=Matrix.Translation(NECK) @ Matrix.Scale(head_scale, 4)
                                @ Matrix.Translation(-NECK), verts=bm.verts)
        if leg != 1.0:
            for v in bm.verts:
                v.co = zmap(v.co)
        bmesh.ops.scale(bm, vec=(S, S, S), verts=bm.verts)
    body = kit.build(coll)

    arm_data = bpy.data.armatures.new(PREFIX + name + "_rig")
    rig = bpy.data.objects.new(name + "_rig", arm_data)
    coll.objects.link(rig)
    vl = bpy.context.view_layer
    vl.objects.active = rig
    bpy.ops.object.mode_set(mode="EDIT")
    for name, (h, t, p) in BONES.items():
        eb = arm_data.edit_bones.new(name)
        hh, tt = Vector(h), Vector(t)
        if name in head_bones:
            hh, tt = NECK + (hh - NECK) * head_scale, NECK + (tt - NECK) * head_scale
        eb.head, eb.tail, eb.roll = zmap(hh) * S, zmap(tt) * S, 0
        if p:
            eb.parent = arm_data.edit_bones[p]
    bpy.ops.object.mode_set(mode="OBJECT")
    for b in arm_data.bones:
        b.use_deform = b.name != "Root"
    body.parent = rig
    mod = body.modifiers.new("Armature", "ARMATURE")
    mod.object = rig

    rig.animation_data_create()
    pb = rig.pose.bones
    for b in pb:
        b.rotation_mode = "XYZ"

    def key(frame, poses, offset=None):
        for bname, bpose in poses.items():
            b = pb[bname]
            dl, dr = (offset or {}).get(bname, ((0, 0, 0), (0, 0, 0)))
            b.location = (Vector(bpose[0]) + Vector(dl)) * S
            b.rotation_euler = [math.radians(a + d) for a, d in zip(bpose[1], dr)]
            b.keyframe_insert("location", frame=frame)
            b.keyframe_insert("rotation_euler", frame=frame)
            if len(bpose) > 2:
                b.scale = bpose[2]
                b.keyframe_insert("scale", frame=frame)

    def make_action(clip, keys, offset=None):
        if clip not in clips:
            return None
        act = bpy.data.actions.new(clip)
        act.use_fake_user = True
        rig.animation_data.action = act
        for frame, poses in keys:
            key(frame, poses, offset)
        track = rig.animation_data.nla_tracks.new()
        track.name = clip
        track.strips.new(clip, int(keys[0][0]), act)
        track.mute = clip != "Idle"
        rig.animation_data.action = None
        for b in pb:
            b.location = (0, 0, 0)
            b.rotation_euler = (0, 0, 0)
            b.scale = (1, 1, 1)
        return act

    z = ((0, 0, 0), (0, 0, 0))
    # Bone axes: x = world X, y = world up, z = world back. Rot x > 0 tips forward.
    # Idle: 4 s loop. Two breaths (chest and shoulders rise, head bobs), one slow weight shift
    # (hips roll, legs counter-roll so the feet stay planted) and one blink at 2.5 s.
    idle = []
    for f in sorted(set(range(0, 97, 6)) | {58, 60, 62}):
        br = (1 - math.cos(math.tau * f / 48)) / 2
        ws = math.sin(math.tau * f / 96)
        blink = 0.1 if f == 60 else 1.0
        idle.append((f, {"Hips": ((0, 0, 0), (0, 0, 2.5 * ws)),
                         "Leg.L": ((0, 0, 0), (0, 0, -2.5 * ws)), "Leg.R": ((0, 0, 0), (0, 0, -2.5 * ws)),
                         "Torso": ((0, 0.008 * br, 0), (-1.5 * br, 0, -1.5 * ws)),
                         "Head": ((0, 0.005 * br, 0), (2.5 * br - 1, 4 * ws, ws)),
                         "Arm.L": ((0, 0.01 * br, 0), (0, 0, 2 + 2 * br)),
                         "Arm.R": ((0, 0.01 * br, 0), (0, 0, -2 - 2 * br)),
                         "Eye.L": ((0, 0, 0), (0, 0, 0), (1, blink, 1)),
                         "Eye.R": ((0, 0, 0), (0, 0, 0), (1, blink, 1))}))
    make_action("Idle", idle, pose)
    # Fidget: 2.5 s look around (head turns left, then right, torso follows a little)
    fidget = []
    for f, turn in ((0, 0), (12, 32), (26, 32), (38, -32), (50, -32), (60, 0)):
        fidget.append((f, {"Head": ((0, 0, 0), (-3 if turn else 0, turn, 0)),
                           "Torso": ((0, 0, 0), (0, turn * 0.25, 0)),
                           "Hips": z, "Leg.L": z, "Leg.R": z, "Arm.L": ((0, 0, 0), (0, 0, 3)),
                           "Arm.R": ((0, 0, 0), (0, 0, -3))}))
    make_action("Fidget", fidget, pose)
    # Walk: 0.5 s loop = two steps; each step is one tile (game moves 1 tile / 0.25 s)
    walk = []
    for f, sw, bob in ((0, 1, -1), (3, 0, 0), (6, -1, -1), (9, 0, 0), (12, 1, -1)):
        # dip at contact (legs spread), level at passing: keeps the feet on the ground
        walk.append((f, {"Hips": ((0, 0.035 * bob, 0), (0, 0, 6 * sw)),
                         "Torso": ((0, 0, 0), (6, 0, -10 * sw)),
                         "Head": ((0, 0, 0), (-4, 0, 6 * sw)),
                         "Leg.L": ((0, 0, 0), (-32 * sw, 0, 0)), "Leg.R": ((0, 0, 0), (32 * sw, 0, 0)),
                         "Arm.L": ((0, 0, 0), (36 * sw * arm_swing, 0, 4)),
                         "Arm.R": ((0, 0, 0), (-36 * sw * arm_swing, 0, -4))}))
    make_action("Walk", walk)
    # Run: 0.25 s loop = two steps of 0.125 s (one tile each at run speed); lean in, bigger swing, flight bob
    run = []
    for f, sw, bob in ((0, 1, -1), (1.5, 0, 1), (3, -1, -1), (4.5, 0, 1), (6, 1, -1)):
        run.append((f, {"Hips": ((0, 0.03 * bob, 0), (0, 0, 8 * sw)),
                        "Torso": ((0, 0, 0), (14, 0, -14 * sw)),
                        "Head": ((0, 0, 0), (-10, 0, 8 * sw)),
                        "Leg.L": ((0, 0, 0), (-48 * sw, 0, 0)), "Leg.R": ((0, 0, 0), (48 * sw, 0, 0)),
                        "Arm.L": ((0, 0, 0), (60 * sw * arm_swing, 0, 8)),
                        "Arm.R": ((0, 0, 0), (-60 * sw * arm_swing, 0, -8))}))
    make_action("Run", run)
    return [body, rig]


# ---------------------------------------------------------------- scene, export, preview

PROPS = {
    "house": build_house, "tree": build_tree, "bush": build_bush, "flowers": build_flowers,
    "rock": build_rock, "sign": build_sign,
    "fence": lambda k: build_fence_piece(k, "WE", "std"),
    "fence_corner": lambda k: build_fence_piece(k, "WS", "corner"),
    "fence_end": lambda k: build_fence_piece(k, "W", "std"),
    "fence_t": lambda k: build_fence_piece(k, "WES", "t"),
    "path": lambda k: build_path(k, 11), "path_b": lambda k: build_path(k, 23),
    "path_c": lambda k: build_path(k, 37),
}
PROPS.update({"tree_b": build_tree_b, "tree_c": build_tree_c, "bush_dark": build_bush_dark,
              "grass_edge": build_grass_edge, "shoes": build_shoes})
INTERIOR = {
    "wall": lambda k: build_wall(k, "WE", 2.6), "wall_corner": lambda k: build_wall(k, "WS", 2.6),
    "wall_end": lambda k: build_wall(k, "W", 2.6), "wall_t": lambda k: build_wall(k, "WES", 2.6),
    "wall_window": build_wall_window,
    "wall_low": lambda k: build_wall(k, "WE", 0.6), "wall_low_corner": lambda k: build_wall(k, "WS", 0.6),
    "wall_low_end": lambda k: build_wall(k, "W", 0.6), "wall_low_t": lambda k: build_wall(k, "WES", 0.6),
    "counter": build_counter, "shelf": build_shelf, "duel_table": build_duel_table, "chair": build_chair,
    "rug": build_rug, "plant": build_plant, "display_case": build_display_case, "lamp": build_lamp,
    "exit_mat": build_exit_mat,
}
PROPS.update(INTERIOR)


ASSETS_SCENE, PREVIEW_SCENE = "Assets", "Preview"
CLIPS = {"Idle", "Walk", "Run", "Fidget", "Open"}


def use_scene(scn):
    """Make scn the context scene for operators: switch the window live, override headless."""
    if bpy.context.window:
        bpy.context.window.scene = scn
        return contextlib.nullcontext()
    return bpy.context.temp_override(scene=scn, view_layer=scn.view_layers[0])


def clear_scene(scn):
    """Delete this scene's objects, its child collections and the data only they used."""
    acts = set()
    for o in scn.objects:
        ad = o.animation_data
        if ad:
            acts |= {ad.action} | {st.action for t in ad.nla_tracks for st in t.strips}
    datas = {o.data for o in scn.objects if o.data}
    colls = list(scn.collection.children_recursive)
    for o in list(scn.objects):
        bpy.data.objects.remove(o)
    others = [c for o in bpy.data.scenes if o != scn for c in o.collection.children_recursive]
    for c in colls:
        if c not in others:  # never remove a collection another scene also shows
            bpy.data.collections.remove(c)
    store = {bpy.types.Mesh: bpy.data.meshes, bpy.types.Armature: bpy.data.armatures,
             bpy.types.Camera: bpy.data.cameras, bpy.types.SunLight: bpy.data.lights}
    for d in datas:
        if d.users == 0 and type(d) in store:
            store[type(d)].remove(d)
    for a in list(bpy.data.actions):  # this scene's clips, and orphaned clip copies left by imports
        if (a in acts or a.name.split(".")[0] in CLIPS) and a.users <= int(a.use_fake_user):
            bpy.data.actions.remove(a)
    return scn


def fresh_scene(name=ASSETS_SCENE):
    scn = bpy.data.scenes.get(name) or bpy.data.scenes.new(name)
    clear_scene(scn)
    scn.render.fps = FPS
    return scn


def save_if_file():
    if bpy.data.filepath:
        world = bpy.data.scenes.get("World")
        if world and bpy.app.background:  # the file always opens on the World scene
            for win in bpy.data.window_managers[0].windows:
                win.scene = world
        bpy.ops.wm.save_mainfile()
        print("[sixfold] saved", bpy.data.filepath)


SLOTS = {  # where each asset sits in the Assets scene (tile centres on integers, 2x2 tree on a corner)
    "house": (0, 5), "tree": (-6.5, 5.5), "bush": (-4, 1), "flowers": (-3, 1), "rock": (-2, 1),
    "sign": (2, 1), "fence": (4, 1), "fence_corner": (5, 1), "fence_end": (6, 1), "fence_t": (7, 1),
    "path": (-1, -1), "path_b": (0, -1), "path_c": (1, -1), "hero": (0, 1),
    "tree_b": (-9.5, 5.5), "tree_c": (9.5, 5.5), "bush_dark": (-5, 1), "grass_edge": (3, -1), "shoes": (9, 1),
    # interior kit, two rows further south
    "wall": (-9, -4), "wall_corner": (-7, -4), "wall_end": (-5, -4), "wall_t": (-3, -4), "wall_window": (-1, -4),
    "wall_low": (1, -4), "wall_low_corner": (3, -4), "wall_low_end": (5, -4), "wall_low_t": (7, -4), "lamp": (9, -4),
    "counter": (-8, -7), "shelf": (-4.5, -7), "duel_table": (-1.5, -7), "chair": (1, -7), "rug": (4, -7.5),
    "plant": (7, -7), "display_case": (9, -7), "exit_mat": (6, -9),
}


def build_all(scn):
    built = {}
    for name, fn in PROPS.items():
        coll = bpy.data.collections.new(PREFIX + name)
        scn.collection.children.link(coll)
        kit = Kit(name)
        after = fn(kit)
        ob = kit.build(coll)
        if name in WIND:
            add_wind(ob, WIND[name])
        built[name] = [ob] + (after(ob, coll) if after else [])
    coll = bpy.data.collections.new(PREFIX + "hero")
    scn.collection.children.link(coll)
    built["hero"] = build_hero(coll)
    for name, objs in built.items():  # lay out side by side; exports put roots back at the origin
        x, y = SLOTS[name]
        bpy.data.collections[PREFIX + name].instance_offset = (x, y, 0)
        for o in objs:
            if o.parent is None:
                o.location = (x, y, 0)
    grid = bpy.data.collections.new(PREFIX + "grid")
    scn.collection.children.link(grid)
    verts, edges = [], []
    for i in range(-10, 12):
        verts += [(i - 0.5, -10.5, 0.06), (i - 0.5, 8.5, 0.06)]
        edges.append((len(verts) - 2, len(verts) - 1))
    for j in range(-10, 10):
        verts += [(-10.5, j - 0.5, 0.063), (10.5, j - 0.5, 0.063)]
        edges.append((len(verts) - 2, len(verts) - 1))
    go = bpy.data.objects.new("grid_1m", grid_strips(verts, edges))
    go.data.materials.append(material("eye"))
    go.hide_select = True
    grid.objects.link(go)
    return built


def validate_glb(path, name):
    """Fail loudly on export leaks: extra scenes, hero data in props, duplicate clip names."""
    import json
    import struct
    with open(path, "rb") as f:
        data = f.read()
    j = json.loads(data[20:20 + struct.unpack("<I", data[12:16])[0]])
    clips = [a["name"] for a in j.get("animations", [])]
    hero_nodes = [n.get("name", "") for n in j["nodes"] if n.get("name", "").startswith("hero")]
    problems = []
    if len(j.get("scenes", [])) != 1:
        problems.append(f"{len(j.get('scenes', []))} scenes")
    if len(clips) != len(set(clips)) or any("." in c for c in clips):
        problems.append(f"duplicate/suffixed clips {clips}")
    npc = is_npc(name)
    if name != "hero" and hero_nodes:
        problems.append(f"hero nodes in {name}: {hero_nodes}")
    if name != "hero" and not npc and j.get("skins"):
        problems.append("skin in a prop")
    if name == "hero" and (sorted(clips) != sorted(HERO_CLIPS) or len(j.get("skins", [])) != 1):
        problems.append(f"hero clips {clips}, skins {len(j.get('skins', []))}")
    if npc and (sorted(clips) != sorted(NPC_CLIPS) or len(j.get("skins", [])) != 1):
        problems.append(f"npc clips {clips}, skins {len(j.get('skins', []))}")
    if name != "hero" and not npc and clips and name != "house":
        problems.append(f"unexpected clips {clips}")
    if problems:
        raise RuntimeError(f"[sixfold] bad export {os.path.basename(path)}: " + "; ".join(problems))


HERO_CLIPS = ["Idle", "Walk", "Run", "Fidget"]


def is_npc(name):
    """NPC and rival characters: the hero's rig with Idle/Walk/Fidget, exported to assets/npc/."""
    return name.startswith(("npc_", "rival_"))

NPC_CLIPS = ["Idle", "Walk", "Fidget"]


def export(built, out_dir):
    os.makedirs(out_dir, exist_ok=True)
    report = []
    for name, objs in built.items():
        for o in bpy.context.view_layer.objects:
            o.select_set(False)
        for o in objs:
            o.select_set(True)
        bpy.context.view_layer.objects.active = objs[0]
        roots = [(o, o.location.copy()) for o in objs if o.parent is None]
        for o, _ in roots:
            o.location = (0, 0, 0)
        sub = "interior" if name in INTERIOR else "npc" if is_npc(name) else ""
        path = os.path.join(out_dir, sub, name + ".glb")
        os.makedirs(os.path.dirname(path), exist_ok=True)
        bpy.ops.export_scene.gltf(
            filepath=path, export_format="GLB", use_selection=True, use_active_scene=True, export_yup=True,
            export_apply=True, export_animations=name in ("hero", "house") or is_npc(name),
            export_vertex_color="ACTIVE" if name in WIND else "NONE",
            export_animation_mode="NLA_TRACKS", export_skins=True, export_def_bones=True,
            export_cameras=False, export_lights=False, export_image_format="JPEG", export_jpeg_quality=85)
        for o, loc in roots:
            o.location = loc
        validate_glb(path, name)
        report.append((name, sum(tris(o) for o in objs if o.type == "MESH"), os.path.getsize(path)))
    for name, t, size in report:
        print(f"[sixfold] {name:8s} {t:5d} tris {size / 1024:7.1f} KB")
    return report


LAYOUT = [  # preview placement on the 1 m grid (x, y), Blender coords (front = -Y)
    ("house", 0, 3.5), ("tree", -6.5, 4.5), ("tree", 6.5, 4.5), ("bush", -4, 1), ("bush", 4, 0),
    ("flowers", -2, -1), ("flowers", -3, -1), ("flowers", 2, -1), ("rock", 5, -2),
    ("sign", 3, 0),
    ("path", 0, 0), ("path_b", 0, -1), ("path_c", 0, -2), ("path", 0, -3), ("path_b", 1, -3),
    ("hero", 0, -1), ("tree_b", -9.5, 4.5), ("tree_c", 9.5, 4.5), ("bush_dark", -5, 1), ("bush_dark", 5, 1),
    ("grass_edge", -0.56, -1, 90), ("grass_edge", 0.56, -1, 90), ("grass_edge", -0.56, -2, 90), ("grass_edge", 0.56, -2, 270),
]
# fence tiles (tx, tz) in map terms (z grows toward the camera): a closed 4 x 3 ring plus a stub (T + end)
FENCE_TILES = {(x, z) for x in range(4) for z in range(3) if x in (0, 3) or z in (0, 2)} | {(1, 3)}
FENCE_ORIGIN = (-8, -2)


def fence_layout(tiles, origin):
    out = []
    for tx, tz in sorted(tiles):
        nb = {d for d, (dx, dz) in {"N": (0, -1), "E": (1, 0), "S": (0, 1), "W": (-1, 0)}.items()
              if (tx + dx, tz + dz) in tiles}
        for piece, rot in fence_pick(nb):
            out.append((piece, origin[0] + tx, origin[1] - tz, rot))
    return out


def preview(scn, built, path):
    pv = fresh_scene(PREVIEW_SCENE)
    for name, x, y, *rot in LAYOUT + fence_layout(FENCE_TILES, FENCE_ORIGIN):
        e = bpy.data.objects.new("pv_" + name, None)
        e.instance_type = "COLLECTION"
        e.instance_collection = bpy.data.collections[PREFIX + name]
        e.location = (x, y, 0)
        e.rotation_euler = (0, 0, math.radians(rot[0] if rot else 0))
        pv.collection.objects.link(e)

    # grass and a 1 m grid overlay (tile edges at half-integers)
    coll = bpy.data.collections.new(PREFIX + "pv")
    pv.collection.children.link(coll)
    g = Kit("pv_grass")
    g.add(box(25, 19, 0.02), "grass", T((0.5, 1, -0.012)))
    g.build(coll)
    verts, edges = [], []
    for i in range(-12, 14):
        verts += [(i - 0.5, -8.5, 0.06), (i - 0.5, 10.5, 0.06)]
        edges.append((len(verts) - 2, len(verts) - 1))
    for j in range(-8, 12):
        verts += [(-12.5, j - 0.5, 0.06), (12.5, j - 0.5, 0.06)]
        edges.append((len(verts) - 2, len(verts) - 1))
    go = bpy.data.objects.new("pv_grid", grid_strips(verts, edges))
    go.data.materials.append(material("eye"))
    coll.objects.link(go)
    pv_coll = coll

    cam = bpy.data.objects.new("pv_cam", bpy.data.cameras.new(PREFIX + "pv_cam"))
    cam.data.lens = 35
    cam.data.clip_start = 0.5
    pitch = 50
    dist = 21
    cam.rotation_euler = (math.radians(90 - pitch), 0, 0)
    cam.location = (0, 1.8 - dist * math.cos(math.radians(pitch)), dist * math.sin(math.radians(pitch)))
    pv_coll.objects.link(cam)
    pv.camera = cam
    sun = bpy.data.objects.new("pv_sun", bpy.data.lights.new(PREFIX + "pv_sun", "SUN"))
    sun.data.energy = 3.5
    sun.rotation_euler = (math.radians(40), math.radians(20), math.radians(-30))
    pv_coll.objects.link(sun)
    world = bpy.data.worlds.new(PREFIX + "pv_world")
    world.use_nodes = True
    bg = next(n for n in world.node_tree.nodes if n.type == "BACKGROUND")
    bg.inputs["Color"].default_value = (0.55, 0.62, 0.7, 1)
    bg.inputs["Strength"].default_value = 0.9
    pv.world = world
    pv.render.resolution_x, pv.render.resolution_y = 1600, 1000
    pv.render.film_transparent = False
    pv.view_settings.view_transform = "Standard"
    if path:
        os.makedirs(os.path.dirname(path), exist_ok=True)
        pv.render.filepath = path
        pv.render.image_settings.file_format = "PNG"
        for eng in ("BLENDER_EEVEE", "BLENDER_EEVEE_NEXT", "BLENDER_WORKBENCH"):
            try:
                pv.render.engine = eng
                break
            except TypeError:
                continue
        bpy.ops.render.render(write_still=True, scene=pv.name)
        print("[sixfold] preview ->", path)
    return pv


def grid_strips(verts, edges, w=0.025):
    bm = bmesh.new()
    for a, b in edges:
        p, q = Vector(verts[a]), Vector(verts[b])
        d = (q - p).normalized()
        n = Vector((-d.y, d.x, 0)) * w / 2
        bm.faces.new([bm.verts.new(p - n), bm.verts.new(q - n), bm.verts.new(q + n), bm.verts.new(p + n)])
    me = bpy.data.meshes.new(PREFIX + "pv_grid")
    bm.to_mesh(me)
    bm.free()
    return me


def main(argv):
    out, pv_path = None, None
    if "--out" in argv:
        out = argv[argv.index("--out") + 1]
    if "--preview" in argv:
        pv_path = argv[argv.index("--preview") + 1]
    scn = fresh_scene()
    with use_scene(scn):
        built = build_all(scn)
        if out:
            export(built, out)
    if pv_path is not None:
        preview(scn, built, pv_path)
    save_if_file()
    return built


if __name__ == "__main__":
    args = globals().get("ARGS")
    if args is None:
        args = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    main(args)
