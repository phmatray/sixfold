"""Build the props of the 0.7 outdoor areas (woodland, seaside, quarry, square) and export them as .glb.

Headless:
    blender -b --factory-startup -P tools/blender/build_area_props.py -- \
        --out web/world/assets [--only pine,birch] [--preview path/to/areas.png]

Reuses the helpers of build_assets.py (imported as a module) and only ever writes the files of
the props below: the existing .glb files are never re-exported. Same conventions: Blender Z-up,
front facing -Y (glTF +Z), origin at the footprint centre on the ground, 1 tile = 1 m.

Shared file: run against art/Sixfold.blend to keep the scene "Areas" in it; this script owns that
scene only.
"""
import math
import os
import random
import sys

import bmesh
import bpy
from mathutils import Matrix, Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import build_assets as B  # noqa: E402
from build_assets import T, box, contact, cyl, ico, jitter, prism, sphere  # noqa: E402

B.PALETTE.update({
    # woodland
    "pine": "#2f6b3a", "pine_dk": "#24532f", "pine_lt": "#3f8445", "birch_bark": "#efe9dc",
    "bark_mark": "#3a3430", "birch_leaf": "#9cc95a", "birch_leaf_lt": "#b9d96c", "birch_leaf_dk": "#78ab48",
    "reed": "#86a64c", "reed_dk": "#617f37", "cattail": "#6b4a2e",
    # seaside
    "plank": "#b8875a", "plank_lt": "#c99a6a", "plank_dk": "#a07448", "post": "#5e4630",
    "lh_white": "#f4efe4", "lh_red": "#c8473d", "lh_glass": "#cfeaf2", "lamp_glow": "#ffe39a",
    "hull": "#2f6f7a", "hull_cream": "#f1e2c2", "oar": "#c89a62", "iron": "#30353b",
    "lantern_glass": "#ffd98a", "slate": "#4f6a86", "slate_dk": "#3c5268",
    "plank_grey": "#8f8370", "plank_grey_lt": "#a69b86", "plank_grey_dk": "#756a59",
    # quarry: warm layered sandstone
    "sand_a": "#d29e62", "sand_b": "#b97f4e", "sand_c": "#e0b479", "sand_d": "#9a663d",
    "cliff_top": "#ead6ac", "crack": "#a87a4c", "moss": "#6f9a3e", "moss_lt": "#8db552",
    "step_stone": "#d2c2a2", "step_dk": "#ad9b7c",
    # square
    "roof_brown": "#a4553a", "roof_brown_dk": "#7c3d29", "apple": "#d9473d", "pear": "#b9c94a",
    "bread": "#d79a52", "fish": "#9fb8c4", "crate": "#b07a46", "crate_dk": "#8a5c34",
    "fstone": "#d8cdb8", "fstone_dk": "#b9ad97", "fcap": "#ece2cf", "water": "#6cc3d1",
    # doors: the dark doorway behind a leaf that swings open
    "doorway": "#1d1714",
    # items
    "brass": "#d4a43c", "paper": "#f3ead6", "paper_dk": "#e0d2b4", "wax": "#b8333a", "wax_dk": "#8e2229",
})


def clamp01(v):
    return max(0.0, min(1.0, v))


WIND = {
    "pine": lambda x, y, z, r: clamp01((z - 0.6) / 3.2 * 0.8 + r / 0.9 * 0.2) if z > 0.6 else 0.0,
    "birch": lambda x, y, z, r: clamp01((z - 1.4) / 2.2 * 0.75 + r / 0.8 * 0.25) if z > 1.4 else 0.0,
    "reeds": lambda x, y, z, r: clamp01(z / 0.85),
}
B.WIND.update(WIND)  # export() writes COLOR_0 for every name in B.WIND


def shadow(k, r, sx=1.0, sy=1.0):
    """B.contact squashed to an ellipse (for long footprints)."""
    contact(k, r)
    for bm, *_ in k.parts[-2:]:
        bmesh.ops.scale(bm, vec=(sx, sy, 1), verts=bm.verts)


def empty(name, loc):
    """after() hook: an empty parented to the prop (light_origin, smoke_origin)."""
    def after(ob, coll):
        e = bpy.data.objects.new(name, None)
        e.parent = ob
        e.location = loc
        coll.objects.link(e)
        return [e]
    return after


def door_leaf(parts, hinge, then=None, name="door"):
    """after() hook: a door leaf as its own node, origin on its hinge, so the game swings it inward (+80 deg about +Y:
    web/world/ASSETS.md). `door` hangs on its -X edge; the right leaf of a double door is `door_r`, on its +X edge.
    parts: [(bm, mat, M)] in the prop's coordinates; hinge: the node's matrix (its Z is the hinge line, at the sill).
    `then`: another after() hook to chain (smoke_origin)."""
    def after(ob, coll):
        k = B.Kit(name)
        inv = hinge.inverted()
        for bm, mat, M in parts:
            k.add(bm, mat, inv @ M)
        d = k.build(coll)
        d.parent = ob
        d.matrix_basis = hinge
        return [d] + (then(ob, coll) if then else [])
    return after


def courses(k, w, d, h, y0, z0, n, mat, lip=0.035):
    """Shingle courses on a gable roof (base w along X by d along Y at z0, ridge at z0 + h over y0): n strips per
    slope, so the slopes read as roofs, not flat slabs, from the 50 deg camera."""
    a = math.atan2(h, d / 2)
    for s in (-1, 1):
        nrm = Vector((0, s * h, d / 2)).normalized()
        for i in range(1, n + 1):
            t = i / (n + 1)
            p = Vector((0, y0 + s * d / 2 * (1 - t), z0 + h * t)) + nrm * (lip / 2 + 0.005)
            k.add(box(w + 0.02, 0.08, lip), mat, T(p, (-s * math.degrees(a), 0, 0)))


# ---------------------------------------------------------------- woodland


def build_pine(k):
    rng = random.Random(41)
    k.add(cyl(0.13, 0.08, 1.3, 7, 0.02), "bark", T((0, 0, 0.63)), smooth=True)
    for zb, r, h, m in ((0.62, 0.95, 1.3, "pine_dk"), (1.32, 0.76, 1.15, "pine"), (1.98, 0.56, 1.0, "pine"),
                        (2.62, 0.36, 1.18, "pine_lt")):
        bm = cyl(r, 0.0, h, 9)
        for v in bm.verts:  # ragged skirt
            if v.co.z < 0:
                v.co.x *= rng.uniform(0.9, 1.1)
                v.co.y *= rng.uniform(0.9, 1.1)
                v.co.z += rng.uniform(-0.06, 0.06)
        k.add(bm, m, T((0, 0, zb + h / 2), (rng.uniform(-3, 3), rng.uniform(-3, 3), rng.uniform(0, 40))), smooth=True)
    contact(k, 0.55)


def build_birch(k):
    rng = random.Random(42)
    k.add(cyl(0.1, 0.065, 2.4, 8, 0.015), "birch_bark", T((0, 0, 1.18)), smooth=True)
    for i in range(9):  # dark bark marks wrapped on the trunk
        a = rng.uniform(0, 360)
        z = rng.uniform(0.25, 2.2)
        rad = 0.1 - 0.035 * z / 2.4
        k.add(box(0.012, rng.uniform(0.05, 0.1), 0.025), "bark_mark",
              T((rad * math.cos(math.radians(a)), rad * math.sin(math.radians(a)), z), (0, 0, a)))
    for a, z in ((40, 1.75), (220, 2.0)):  # two branches up into the canopy
        k.add(cyl(0.035, 0.02, 0.6, 5), "birch_bark", T((0.18 * math.cos(math.radians(a)), 0.18 * math.sin(math.radians(a)), z), (0, 40, a)))
    for loc, r, m in (((0, 0, 2.75), 0.68, "birch_leaf"), ((0.42, -0.15, 2.45), 0.48, "birch_leaf_dk"),
                      ((-0.4, 0.18, 2.5), 0.5, "birch_leaf_dk"), ((0.05, 0.42, 2.6), 0.45, "birch_leaf"),
                      ((-0.12, -0.3, 3.2), 0.46, "birch_leaf_lt"), ((0.25, 0.15, 3.25), 0.4, "birch_leaf_lt")):
        k.add(jitter(ico(r, 2), rng, r * 0.08), m, T(loc, scale=(1, 1, 0.92)), smooth=True)
    contact(k, 0.45)


def build_reeds(k):
    rng = random.Random(43)
    for i in range(16):
        a = rng.uniform(0, math.tau)
        d = rng.uniform(0, 0.3)
        h = rng.uniform(0.5, 0.9)
        k.add(cyl(0.022, 0.0, h, 3), rng.choice(["reed", "reed_dk"]),
              T((d * math.cos(a), d * math.sin(a), h / 2 - 0.02), (rng.uniform(-10, 10), rng.uniform(-10, 10), rng.uniform(0, 120))))
    for i in range(3):
        a = i / 3 * math.tau + 0.4
        x, y = 0.15 * math.cos(a), 0.15 * math.sin(a)
        h = 0.75 + 0.08 * i
        k.add(cyl(0.01, 0.01, h, 4), "reed_dk", T((x, y, h / 2 - 0.02)))
        k.add(cyl(0.032, 0.032, 0.14, 6), "cattail", T((x, y, h - 0.06)), smooth=True)
    contact(k, 0.4)


# ---------------------------------------------------------------- seaside


def planks(k, top, thick, rng, length=1.0):
    """Five planks along X across a 1 m tile, top face at `top`, reaching the X edges."""
    for i in range(5):
        y = -0.4 + 0.2 * i
        k.add(box(length, 0.186, thick, 0.006), rng.choice(["plank", "plank_lt", "plank_dk"]), T((0, y, top - thick / 2)))


def build_boardwalk(k):
    rng = random.Random(51)
    planks(k, 0.10, 0.04, rng)
    for x in (-0.3, 0.3):  # joists along Z, down into the sand
        k.add(box(0.08, 1.0, 0.08), "post", T((x, 0, 0.02)))


def build_pier(k):
    rng = random.Random(52)
    planks(k, 0.30, 0.05, rng)
    for x in (-0.35, 0.35):
        k.add(box(0.09, 1.0, 0.08), "post", T((x, 0, 0.21)))
    for sx in (-1, 1):
        for sy in (-1, 1):
            k.add(cyl(0.065, 0.06, 0.55, 8), "post", T((sx * 0.42, sy * 0.42, -0.025)), smooth=True)


BRIDGE = ((0.0, 0.25), (0.5, 0.24), (1.5, 0.12), (2.0, 0.03))  # (|y|, deck top)


def deck_h(y):
    y = abs(y)
    for (y0, h0), (y1, h1) in zip(BRIDGE, BRIDGE[1:]):
        if y <= y1:
            return h0 + (h1 - h0) * (y - y0) / (y1 - y0)
    return BRIDGE[-1][1]


def beam(k, a, b, w, h, mat):
    """A box from point a to point b (centre line), w wide (X) and h tall."""
    a, b = Vector(a), Vector(b)
    d = b - a
    rot = d.normalized().to_track_quat("Y", "Z").to_matrix().to_4x4()
    k.add(box(w, d.length, h), mat, Matrix.Translation((a + b) / 2) @ rot)


def build_bridge(k):
    """2 x 4 footbridge spanning along Z; deck heights 0.12 / 0.24 / 0.24 / 0.12 at the row centres."""
    rng = random.Random(53)
    for i in range(20):
        y = -1.9 + 0.2 * i
        slope = math.degrees(math.atan2(deck_h(y + 0.01) - deck_h(y - 0.01), 0.02))
        k.add(box(1.72, 0.186, 0.05, 0.006), rng.choice(["plank", "plank_lt", "plank_dk"]),
              T((0, y, deck_h(y) - 0.025), (slope, 0, 0)))
    ys = [-2.0 + 0.25 * i for i in range(17)]
    for x in (-0.8, 0.8):  # stringers under the deck
        for y0, y1 in zip(ys, ys[1:]):
            beam(k, (x, y0, deck_h(y0) - 0.1), (x, y1, deck_h(y1) - 0.1), 0.1, 0.12, "post")
    posts = (-1.85, -0.95, 0.0, 0.95, 1.85)
    for x in (-0.92, 0.92):
        for y in posts:
            h = deck_h(y)
            k.add(box(0.1, 0.1, 1.02 + h, 0.015), "wood_dk", T((x, y, (h + 0.95 - 0.07) / 2 - 0.05)))
            k.add(box(0.13, 0.13, 0.05, 0.015), "wood_dk", T((x, y, h + 0.95)))
        for y0, y1 in zip(posts, posts[1:]):
            for up, w in ((0.9, 0.08), (0.45, 0.05)):
                beam(k, (x, y0, deck_h(y0) + up), (x, y1, deck_h(y1) + up), w, w, "wood")
    for y in (-1.9, 1.9):  # stone footings where the bridge meets the bank
        k.add(jitter(box(1.9, 0.3, 0.12, 0.04), rng, 0.02), "stone", T((0, y, 0.02)))


def build_lighthouse(k):
    """2 x 2, ~7 m: white tower with red bands, gallery, glazed lantern room, red cap."""
    k.add(cyl(0.98, 0.98, 0.36, 8, 0.03), "stone_dk", T((0, 0, 0.16), (0, 0, 22.5)))
    bands = [(0.34, 1.3, "lh_white"), (1.3, 2.1, "lh_red"), (2.1, 3.3, "lh_white"), (3.3, 4.1, "lh_red"), (4.1, 5.1, "lh_white")]
    r = lambda z: 0.86 - 0.05 * z  # noqa: E731 (taper)
    for z0, z1, m in bands:
        k.add(cyl(r(z0), r(z1), z1 - z0, 14), m, T((0, 0, (z0 + z1) / 2)), smooth=True)
    # door (front, -Y) and two small windows
    k.add(box(0.5, 0.12, 0.95, 0.03), "wood_dk", T((0, -r(0.8) - 0.0, 0.85), (-3, 0, 0)))
    k.add(box(0.6, 0.12, 0.08, 0.02), "lh_red", T((0, -r(1.35), 1.36)))
    for z in (2.7, 4.55):
        k.add(box(0.2, 0.1, 0.32, 0.02), "lh_glass", T((0, -r(z) + 0.02, z), (-3, 0, 0)))
    # gallery: floor, posts, rail
    k.add(cyl(0.82, 0.82, 0.12, 14, 0.02), "lh_red", T((0, 0, 5.16)))
    n = 14
    for i in range(n):
        a = i / n * math.tau
        k.add(box(0.035, 0.035, 0.4), "iron", T((0.76 * math.cos(a), 0.76 * math.sin(a), 5.42)))
        a2 = (i + 1) / n * math.tau
        beam(k, (0.76 * math.cos(a), 0.76 * math.sin(a), 5.62), (0.76 * math.cos(a2), 0.76 * math.sin(a2), 5.62), 0.04, 0.04, "iron")
    # lantern room: glass, mullions, lamp, cap
    k.add(cyl(0.44, 0.44, 0.78, 8), "lh_glass", T((0, 0, 5.61)), smooth=False)
    for i in range(8):
        a = (i + 0.5) / 8 * math.tau
        k.add(box(0.04, 0.04, 0.8), "iron", T((0.45 * math.cos(a), 0.45 * math.sin(a), 5.61)))
    k.add(sphere(0.2, 10, 6), "lamp_glow", T((0, 0, 5.62)), smooth=True)
    k.add(cyl(0.58, 0.58, 0.08, 14, 0.02), "lh_red", T((0, 0, 6.04)))
    k.add(cyl(0.54, 0.05, 0.62, 14), "lh_red", T((0, 0, 6.38)), smooth=True)
    k.add(sphere(0.09, 8, 5), "iron", T((0, 0, 6.75)), smooth=True)
    k.add(cyl(0.012, 0.012, 0.3, 4), "iron", T((0, 0, 6.95)))
    contact(k, 1.1)
    return empty("light_origin", (0, 0, 5.62))


def loft(stations, idx, flip=False):
    """Quads between consecutive cross-sections (lists of points), over section point indices idx."""
    bm = bmesh.new()
    rows = [[bm.verts.new(p) for p in sec] for sec in stations]
    for a, b in zip(rows, rows[1:]):
        for i, j in zip(idx, idx[1:]):
            f = (a[i], b[i], b[j], a[j])
            bm.faces.new(f[::-1] if flip else f)
    return bm


def build_boat(k):
    """2 x 1 rowboat along X: bow at +X, transom at -X; bottom -0.15, gunwale 0.35."""
    xs = [-0.86, -0.55, -0.1, 0.35, 0.7, 0.9]
    ws = [0.3, 0.36, 0.38, 0.33, 0.2, 0.03]

    def sec(x, w, inset=0.0):
        top = 0.35 - (0.02 if inset else 0) + (0.05 if x > 0.6 else 0)  # the bow rises a little
        bot = -0.15 + (0.08 if x > 0.6 else 0)
        wi = max(w - inset, 0.01)
        return [(x, -wi, top), (x, -wi * 0.82, 0.05), (x, -wi * 0.38, bot + inset), (x, wi * 0.38, bot + inset),
                (x, wi * 0.82, 0.05), (x, wi, top)]

    outer = [sec(x, w) for x, w in zip(xs, ws)]
    inner = [sec(x, w, 0.04) for x, w in zip(xs, ws)]
    k.add(loft(outer, [0, 1]), "hull_cream")
    k.add(loft(outer, [4, 5]), "hull_cream")
    k.add(loft(outer, [1, 2, 3, 4]), "hull")
    k.add(loft(inner, [0, 1, 2, 3, 4, 5], flip=True), "wood")
    # gunwale rims and the transom
    for side in (0, 5):
        for a, b in zip(range(len(xs)), range(1, len(xs))):
            p, q = Vector(outer[a][side]), Vector(outer[b][side])
            beam(k, p + Vector((0, 0, 0.015)), q + Vector((0, 0, 0.015)), 0.07, 0.04, "wood_dk")
    bm = bmesh.new()
    bm.faces.new([bm.verts.new(p) for p in outer[0]][::-1])
    k.add(bm, "hull")
    for x in (-0.45, 0.25):  # thwarts
        k.add(box(0.16, 0.66, 0.04), "wood_lt", T((x, 0, 0.22)))
    for y, yaw in ((-0.12, 4), (0.12, -6)):  # oars laid inside
        k.add(cyl(0.018, 0.018, 1.3, 6), "oar", T((-0.05, y, 0.27), (0, 90, yaw)))
        k.add(box(0.26, 0.09, 0.015), "oar", T((0.65, y + 0.02 * (1 if y > 0 else -1), 0.27), (0, 0, yaw)))


def build_lantern(k):
    k.add(box(0.3, 0.3, 0.14, 0.03), "stone_dk", T((0, 0, 0.05)))
    k.add(cyl(0.05, 0.04, 1.75, 8), "iron", T((0, 0, 0.95)), smooth=True)
    k.add(cyl(0.09, 0.09, 0.06, 8), "iron", T((0, 0, 1.82)))
    k.add(box(0.2, 0.2, 0.3), "lantern_glass", T((0, 0, 2.0)))
    for sx in (-1, 1):
        for sy in (-1, 1):
            k.add(box(0.03, 0.03, 0.34), "iron", T((sx * 0.11, sy * 0.11, 2.0)))
    k.add(box(0.26, 0.26, 0.04), "iron", T((0, 0, 1.84)))
    k.add(cyl(0.22, 0.0, 0.18, 4), "iron", T((0, 0, 2.24), (0, 0, 45)))
    k.add(sphere(0.035, 6, 4), "iron", T((0, 0, 2.36)))
    contact(k, 0.3)
    return empty("light_origin", (0, 0, 2.0))


def build_stall(k, awning, goods):
    """2 x 1 market stall, counter on the front (-Y = glTF +Z), striped awning."""
    rng = random.Random(54 if goods == "fruit" else 55)
    k.add(box(1.8, 0.5, 0.82, 0.02), "crate", T((0, -0.12, 0.39)))
    k.add(box(1.86, 0.56, 0.05, 0.015), "wood_dk", T((0, -0.12, 0.82)))
    for x in (-0.6, 0.0, 0.6):  # front boards
        k.add(box(0.03, 0.02, 0.7), "crate_dk", T((x, -0.375, 0.4)))
    for sx in (-1, 1):
        for sy, h in ((-1, 1.9), (1, 2.15)):
            k.add(box(0.08, 0.08, h + 0.02, 0.01), "wood_dk", T((sx * 0.9, sy * 0.4, h / 2 - 0.01)))
    # awning: alternating stripes, sloping from the back (2.2) to the front (1.92), overhanging to -0.6
    n = 8
    slope = math.degrees(math.atan2(0.28, 1.0))
    for i in range(n):
        x = -1.0 + (i + 0.5) * 2.0 / n
        k.add(box(2.0 / n - 0.002, 1.1, 0.035), awning[i % 2], T((x, -0.07, 2.08), (slope, 0, 0)))
        k.add(prism([(-0.125, 0), (0.125, 0), (0, -0.16)], 0.02), awning[i % 2], T((x, -0.625, 1.92)))  # valance
    # goods: crates on the counter
    for x in (-0.55, 0.0, 0.55):
        k.add(box(0.46, 0.34, 0.12, 0.01), "crate_dk", T((x, -0.14, 0.9)))
        if goods == "fruit":
            m = rng.choice(["apple", "fruit", "pear"])
            for j in range(7):
                k.add(sphere(0.065, 8, 5), m, T((x + rng.uniform(-0.16, 0.16), -0.14 + rng.uniform(-0.1, 0.1), 0.99 + rng.uniform(0, 0.03))), smooth=True)
        elif x < 0.3:
            for j in range(3):  # loaves
                k.add(sphere(0.1, 10, 6), "bread", T((x - 0.12 + 0.12 * j, -0.14, 0.99), (0, 0, rng.uniform(-20, 20)), (0.6, 1.0, 0.45)), smooth=True)
        else:
            for j in range(3):  # fish
                M = T((x, -0.24 + 0.1 * j, 0.99), (0, 0, rng.uniform(-8, 8)))
                k.add(sphere(0.1, 10, 6), "fish", M @ T(scale=(1.4, 0.4, 0.3)), smooth=True)
                k.add(prism([(0, 0), (0.08, 0.05), (0.08, -0.05)], 0.015), "fish", M @ T((0.12, 0, 0), (90, 0, 0)))
    shadow(k, 1.0, 1.0, 0.6)


# ---------------------------------------------------------------- buildings


def gable(w, d, h):
    """Solid gable roof: base w (X) x d (Y) at z = 0, ridge along X at height h."""
    return prism([(-d / 2, 0), (d / 2, 0), (0, h)], w)  # in (y, z), extruded along X once turned


def windowed(k, x, y, z, w, h, frame="timber"):
    k.add(box(w + 0.12, 0.1, h + 0.12, 0.02), frame, T((x, y, z)))
    k.add(box(w, 0.04, h), "glass", T((x, y - 0.05, z)))
    k.add(box(0.04, 0.04, h), frame, T((x, y - 0.08, z)))
    k.add(box(w, 0.04, 0.04), frame, T((x, y - 0.08, z)))


def build_cottage(k):
    """5 x 4 bakery cottage: door on the centre column of the front row (origin + (2, 3) in tiles)."""
    ww, wd, by = 4.6, 3.0, 0.3   # walls; front wall at y = -1.2 (glTF z = +1.2, front row)
    fy = by - wd / 2
    top = 2.5
    k.add(box(4.8, 3.3, 0.24, 0.03), "stone", T((0, by, 0.1)))
    k.add(box(ww, wd, top - 0.2), "plaster", T((0, by, 0.2 + (top - 0.2) / 2)))
    for sx in (-1, 1):  # timber frame: corner posts and a band
        for sy in (-1, 1):
            k.add(box(0.16, 0.16, top - 0.2, 0.02), "timber", T((sx * (ww / 2 - 0.02), by + sy * (wd / 2 - 0.02), 0.2 + (top - 0.2) / 2)))
    k.add(box(ww + 0.06, wd + 0.06, 0.12, 0.02), "timber", T((0, by, 1.25)))
    k.add(box(ww + 0.08, wd + 0.08, 0.14, 0.02), "timber", T((0, by, top)))
    for x in (-1.6, -0.75, 0.75, 1.6):  # front studs above the band
        k.add(box(0.1, 0.06, 1.1), "timber", T((x, fy - 0.02, 1.85)))
    # roof (gable along X) with eaves, ridge, gable-end timber
    k.add(gable(ww + 0.5, wd + 0.5, 1.6), "roof_brown", T((0, by, top + 0.06), (0, 0, 90)))
    courses(k, ww + 0.5, wd + 0.5, 1.6, by, top + 0.06, 4, "roof_brown_dk")
    k.add(box(ww + 0.6, 0.26, 0.16, 0.04), "roof_brown_dk", T((0, by, top + 1.66)))
    for sy in (-1, 1):
        k.add(box(ww + 0.55, 0.08, 0.1), "roof_brown_dk", T((0, by + sy * (wd + 0.5) / 2, top + 0.06)))
    # chimney (back right)
    k.add(box(0.5, 0.5, 2.0, 0.03), "brick_wall", T((1.5, by + 0.6, top + 1.2)))
    k.add(box(0.6, 0.6, 0.1, 0.02), "stone_dk", T((1.5, by + 0.6, top + 2.2)))
    # door, centre column, with a step and a little canopy
    k.add(box(1.0, 0.12, 2.0, 0.03), "timber", T((0, fy - 0.02, 1.15)))
    k.add(box(0.78, 0.01, 1.83), "doorway", T((0, fy - 0.09, 1.1)))  # behind the leaf: dark when it swings in
    leaf = [(box(0.8, 0.08, 1.85, 0.02), "brick_dk", T((0, fy - 0.07, 1.1))), (sphere(0.04, 6, 4), "knob", T((0.28, fy - 0.12, 1.05)))]
    k.add(box(1.2, 0.36, 0.14, 0.03), "stone", T((0, fy - 0.2, 0.06)))
    k.add(box(1.3, 0.5, 0.06, 0.02), "roof_brown_dk", T((0, fy - 0.25, 2.2), (-12, 0, 0)))
    # bay window on the left
    k.add(box(1.3, 0.45, 1.0, 0.03), "plaster", T((-1.5, fy - 0.2, 0.75)))
    k.add(box(1.4, 0.55, 0.08, 0.02), "timber", T((-1.5, fy - 0.2, 1.3)))
    k.add(box(1.2, 0.04, 0.8), "glass", T((-1.5, fy - 0.44, 1.75)))
    for x in (-2.05, -1.5, -0.95):
        k.add(box(0.06, 0.06, 0.82), "timber", T((x, fy - 0.45, 1.75)))
    k.add(box(1.45, 0.6, 0.1, 0.02), "roof_brown_dk", T((-1.5, fy - 0.22, 2.2), (-15, 0, 0)))
    # window with a striped awning on the right
    windowed(k, 1.5, fy, 1.75, 1.0, 0.75)
    for i in range(5):
        k.add(box(0.24, 0.55, 0.03), ("awning_red", "cream")[i % 2], T((1.02 + 0.24 * i, fy - 0.25, 2.27), (-20, 0, 0)))
    # hanging loaf sign beside the door (a shape, no letters)
    k.add(box(0.5, 0.05, 0.05), "timber", T((-0.85, fy - 0.25, 2.05)))
    k.add(cyl(0.2, 0.2, 0.04, 16), "board", T((-0.95, fy - 0.42, 1.8), (90, 0, 0)))
    k.add(sphere(0.12, 10, 6), "bread", T((-0.95, fy - 0.45, 1.8), scale=(1.1, 0.25, 0.55)), smooth=True)
    for x in (-2.2, 2.2):  # side windows
        k.add(box(0.1, 0.7, 0.6, 0.02), "timber", T((x * 1.005 + (0.02 if x > 0 else -0.02), by + 0.3, 1.8)))
    return door_leaf(leaf, T((-0.4, fy - 0.07, 0.175)), empty("smoke_origin", (1.5, by + 0.6, top + 2.3)))


def build_workshop(k):
    """3 x 3 weathered boathouse; double door on the centre column of the front row (origin + (1, 2))."""
    rng = random.Random(61)
    ww, wd, by = 2.6, 2.3, 0.1
    fy = by - wd / 2   # -1.05: glTF z = +1.05, front row
    top = 2.2
    k.add(box(ww + 0.1, wd + 0.1, 0.16, 0.02), "stone_dk", T((0, by, 0.06)))
    k.add(box(ww, wd, top), "plank_grey", T((0, by, top / 2 + 0.02)))
    for face_y, n in ((fy - 0.015, 12), (by + wd / 2 + 0.015, 12)):  # vertical boards front and back
        for i in range(n):
            x = -ww / 2 + (i + 0.5) * ww / n
            if face_y < 0 and abs(x) < 0.6:
                continue
            k.add(box(ww / n - 0.03, 0.03, top - 0.04), rng.choice(["plank_grey_lt", "plank_grey_dk"]), T((x, face_y, top / 2 + 0.02)))
    for sx in (-1, 1):
        for i in range(10):
            y = by - wd / 2 + (i + 0.5) * wd / 10
            k.add(box(0.03, wd / 10 - 0.03, top - 0.04), rng.choice(["plank_grey_lt", "plank_grey_dk"]), T((sx * (ww / 2 + 0.015), y, top / 2 + 0.02)))
    # gable roof, slate blue, and a filled gable end
    k.add(gable(ww + 0.45, wd + 0.6, 1.45), "slate", T((0, by, top + 0.02), (0, 0, 90)))
    courses(k, ww + 0.45, wd + 0.6, 1.45, by, top + 0.02, 3, "slate_dk")
    k.add(box(ww + 0.5, 0.18, 0.12, 0.03), "slate_dk", T((0, by, top + 1.49)))
    for sy in (-1, 1):
        k.add(box(ww + 0.5, 0.06, 0.08), "slate_dk", T((0, by + sy * (wd + 0.6) / 2, top + 0.02)))
    # double door with braces, each leaf hinged on its outer edge; the doorway behind is dark
    k.add(box(1.04, 0.01, 1.83), "doorway", T((0, fy - 0.04, 0.97)))
    leaves = {}
    for sx in (-1, 1):
        lk = B.Kit("leaf")
        lk.add(box(0.52, 0.08, 1.85, 0.02), "wood_dk", T((sx * 0.27, fy - 0.03, 0.97)))
        beam(lk, (sx * 0.06, fy - 0.08, 0.2), (sx * 0.48, fy - 0.08, 1.7), 0.07, 0.03, "wood")
        leaves[sx] = [(bm, mat, Matrix()) for bm, mat, *_ in lk.parts]
    k.add(box(1.2, 0.12, 0.12, 0.02), "wood_dk", T((0, fy - 0.03, 1.95)))
    # a coiled rope and a leaning oar by the door
    k.add(cyl(0.16, 0.16, 0.1, 10), "oar", T((0.95, fy - 0.25, 0.04)))
    k.add(cyl(0.02, 0.02, 1.4, 6), "oar", T((-0.95, fy - 0.12, 0.7), (-10, 8, 0)))
    k.add(box(0.14, 0.02, 0.38), "oar", T((-1.05, fy - 0.24, 1.35), (-10, 8, 0)))
    return door_leaf(leaves[-1], T((-0.53, fy - 0.03, 0.045)), door_leaf(leaves[1], T((0.53, fy - 0.03, 0.045)), name="door_r"))


def build_fountain(k):
    """3 x 3 hexagonal basin (the Sixfold hex), centre column, top bowl. Water surfaces use `water`."""
    R, wall, h = 1.45, 0.22, 0.55
    side = R  # a regular hexagon's side equals its corner radius
    k.add(cyl(R + 0.02, R + 0.02, 0.12, 6, 0.02), "fstone_dk", T((0, 0, 0.04)))           # base slab, sunk
    for i in range(6):  # wall segments along the edges, then corner piers
        a = math.radians(30 + 60 * i)
        apo = R * math.cos(math.radians(30)) - wall / 2
        k.add(box(side - 0.12, wall, h, 0.02), "fstone", T((apo * math.cos(a), apo * math.sin(a), h / 2), (0, 0, math.degrees(a) + 90)))
        k.add(box(side + 0.08, wall + 0.1, 0.08, 0.025), "fcap", T(((apo + 0.02) * math.cos(a), (apo + 0.02) * math.sin(a), h + 0.03), (0, 0, math.degrees(a) + 90)))
        c = math.radians(60 * i)
        k.add(cyl(0.16, 0.16, h + 0.14, 6, 0.02), "fstone_dk", T(((R - 0.14) * math.cos(c), (R - 0.14) * math.sin(c), (h + 0.14) / 2)))
    k.add(cyl(R - 0.2, R - 0.2, 0.02, 6), "water", T((0, 0, 0.44)))
    k.add(cyl(0.2, 0.24, 1.2, 6, 0.02), "fstone", T((0, 0, 0.6)))
    k.add(cyl(0.16, 0.6, 0.26, 12), "fstone_dk", T((0, 0, 1.3)), smooth=True)
    k.add(cyl(0.62, 0.62, 0.06, 12, 0.015), "fcap", T((0, 0, 1.45)))
    k.add(cyl(0.54, 0.54, 0.02, 12), "water", T((0, 0, 1.44)))
    k.add(cyl(0.08, 0.06, 0.3, 6), "fstone", T((0, 0, 1.6)))
    k.add(sphere(0.1, 8, 5), "fcap", T((0, 0, 1.78)), smooth=True)
    for a in (-90, 90):  # the hex emblem on the column, front and back
        n = Vector((math.cos(math.radians(a)), math.sin(math.radians(a)), 0))
        k.add(cyl(0.12, 0.12, 0.03, 6), "hex", T(n * 0.22 + Vector((0, 0, 0.75)), (90, 0, a + 90)))
        k.add(cyl(0.065, 0.065, 0.03, 6), "hex_in", T(n * 0.235 + Vector((0, 0, 0.75)), (90, 0, a + 90)))
    contact(k, 1.5)


# ---------------------------------------------------------------- quarry


def stratum(rng, z0, z1, bulge, inward_top=False):
    """A 1 x 1 slab between z0 and z1, side verts pushed out by up to `bulge` (in only on the top rim)."""
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)
    bmesh.ops.subdivide_edges(bm, edges=bm.edges[:], cuts=2, use_grid_fill=True)
    for v in bm.verts:
        v.co.z = z0 + (v.co.z + 0.5) * (z1 - z0)
        top = abs(v.co.z - z1) < 1e-4
        for ax in (0, 1):
            if abs(abs(v.co[ax]) - 0.5) < 1e-4 and not (top and inward_top):
                v.co[ax] *= 1 + rng.uniform(-0.2, 1.0) * bulge * 2
            elif abs(abs(v.co[ax]) - 0.5) < 1e-4:
                v.co[ax] *= 1 - rng.uniform(0, 0.03)
    bottom = [f for f in bm.faces if f.normal.z < -0.9 and all(abs(v.co.z - z0) < 1e-4 for v in f.verts)]
    bmesh.ops.delete(bm, geom=bottom, context="FACES")
    return bm


def build_cliff(k, mossy=False):
    """1 x 1 x 1 sandstone block: stratified sides to the tile edges, flat top at exactly 1.0."""
    rng = random.Random(71)
    layers = ((-0.02, 0.3, "sand_d", 0.025), (0.3, 0.56, "sand_b", 0.02), (0.56, 0.8, "sand_a", 0.03),
              (0.8, 0.94, "sand_c", 0.015))
    for z0, z1, m, b in layers:
        k.add(stratum(rng, z0, z1, b), m)
    k.add(stratum(rng, 0.94, 1.0, 0.0, inward_top=True), "cliff_top")
    for _ in range(2):  # a hairline crack and pebbles on the top, 1-8 mm above it
        k.add(box(rng.uniform(0.2, 0.35), 0.01, 0.002), "crack", T((rng.uniform(-0.2, 0.2), rng.uniform(-0.2, 0.2), 1.001), (0, 0, rng.uniform(0, 180))))
    for _ in range(4):
        r = rng.uniform(0.02, 0.04)
        k.add(jitter(ico(r, 1), rng, r * 0.2), "pebble", T((rng.uniform(-0.38, 0.38), rng.uniform(-0.38, 0.38), 1.0), scale=(1, 0.8, 0.25)))
    if mossy:
        for _ in range(7):  # low moss along the top edges, inside the tile
            edge = rng.choice([0, 1])
            s = rng.choice([-1, 1])
            p = [rng.uniform(-0.38, 0.38), rng.uniform(-0.38, 0.38)]
            p[edge] = s * rng.uniform(0.32, 0.42)
            r = rng.uniform(0.06, 0.1)
            k.add(jitter(ico(r, 1), rng, r * 0.15), rng.choice(["moss", "moss_lt"]), T((p[0], p[1], 1.0), scale=(1, 1, 0.45)), smooth=True)
        for _ in range(10):
            h = rng.uniform(0.03, 0.06)
            k.add(cyl(0.012, 0.0, h, 3), "moss_lt", T((rng.uniform(-0.42, 0.42), rng.uniform(-0.42, 0.42), 1.0 + h / 2 - 0.005)))


def build_stairs(k):
    """1 x 1 stone steps: up from the front (-Y = glTF +Z) at ~0 to the back (+Y = glTF -Z).
    Tread tops 0.125, 0.375, 0.625, 0.875 (a last 12.5 cm riser onto a 1 m cliff top)."""
    rng = random.Random(72)
    for i in range(4):
        y0 = -0.5 + 0.25 * i
        topz = 0.125 + 0.25 * i
        k.add(box(0.74, 0.5 - y0, topz + 0.02, 0.015), "step_dk", T((0, (y0 + 0.5) / 2, (topz - 0.02) / 2)))
        k.add(box(0.76, 0.26, 0.04, 0.012), "step_stone", T((0, y0 + 0.12, topz - 0.015)))
    for sx in (-1, 1):  # rocky cheeks rising toward the back
        pts = [(-0.5, -0.02), (0.5, -0.02), (0.5, 1.04), (0.2, 0.92), (-0.25, 0.3), (-0.5, 0.12)]
        cheek = jitter(prism(pts, 0.13, 0.015), rng, 0.012)
        k.add(cheek, "sand_b", T((sx * 0.44, 0, 0), (0, 0, 90)))
    contact(k, 0.5)


# ---------------------------------------------------------------- small props


def build_crates(k):
    rng = random.Random(81)

    def crate(x, y, z, s, yaw):
        M = T((x, y, z + s / 2), (0, 0, yaw))
        k.add(box(s, s, s, 0.015), "crate", M)
        for sy in (-1, 1):
            for dz in (-s * 0.3, s * 0.3):
                k.add(box(s + 0.01, 0.02, 0.06), "crate_dk", M @ T((0, sy * s / 2, dz)))
            k.add(box(0.02, s + 0.01, 0.06), "crate_dk", M @ T((sy * s / 2, 0, s * 0.3)))
    crate(-0.15, 0.1, -0.02, 0.44, 8)
    crate(-0.12, 0.08, 0.42, 0.36, -10)
    crate(0.25, -0.2, -0.02, 0.32, 25)
    k.add(cyl(0.16, 0.16, 0.5, 10, 0.02), "wood", T((0.27, 0.25, 0.23)), smooth=True)
    for z in (0.08, 0.38):
        k.add(cyl(0.17, 0.17, 0.035, 10), "iron", T((0.27, 0.25, z)))
    _ = rng
    contact(k, 0.5)


def build_signpost(k):
    k.add(box(0.3, 0.3, 0.1, 0.03), "stone_dk", T((0, 0, 0.03)))
    k.add(box(0.11, 0.11, 1.62, 0.02), "wood_dk", T((0, 0, 0.8)))
    k.add(cyl(0.1, 0.0, 0.12, 4), "wood_dk", T((0, 0, 1.67), (0, 0, 45)))
    arrow = [(0.06, -0.09), (0.44, -0.09), (0.56, 0.0), (0.44, 0.09), (0.06, 0.09)]
    for z, yaw, m in ((1.38, 0, "wood"), (1.12, 180, "wood_lt"), (0.86, 18, "wood")):
        M = T((0, 0, z), (0, 0, yaw))
        k.add(prism(arrow, 0.04, 0.008), m, M)
        for j, w in enumerate((0.22, 0.16)):  # abstract strokes on both faces
            for sy in (-1, 1):
                k.add(box(w, 0.006, 0.025), "wood_dk", M @ T((0.12 + w / 2, sy * 0.022, 0.03 - j * 0.055)))
    contact(k, 0.3)


def build_bench(k):
    """2 x 1 park bench facing the front (-Y = glTF +Z)."""
    for i in range(3):
        k.add(box(1.6, 0.1, 0.04, 0.01), "wood", T((0, -0.16 + 0.12 * i, 0.45)))
    for i in range(2):
        k.add(box(1.6, 0.1, 0.04, 0.01), "wood", T((0, 0.2, 0.62 + 0.14 * i), (75, 0, 0)))
    for x in (-0.68, 0.68):
        k.add(box(0.05, 0.42, 0.05, 0.01), "iron", T((x, -0.02, 0.41)))
        for y in (-0.18, 0.14):
            k.add(box(0.05, 0.05, 0.45), "iron", T((x, y, 0.21)))
        k.add(box(0.05, 0.05, 0.5), "iron", T((x, 0.22, 0.65), (-15, 0, 0)))
        k.add(box(0.05, 0.36, 0.04, 0.01), "iron", T((x, 0.0, 0.62)))
    shadow(k, 0.95, 1.0, 0.45)


def build_lens(k):
    """A brass field lens (small spyglass), lying along X, centred on the origin."""
    k.add(cyl(0.05, 0.05, 0.16, 12), "brass", T((0.04, 0, 0), (0, 90, 0)), smooth=True)
    k.add(cyl(0.04, 0.04, 0.1, 12), "brass", T((-0.09, 0, 0), (0, 90, 0)), smooth=True)
    k.add(cyl(0.03, 0.03, 0.06, 10), "brass", T((-0.16, 0, 0), (0, 90, 0)), smooth=True)
    k.add(cyl(0.056, 0.056, 0.03, 12), "brass", T((0.12, 0, 0), (0, 90, 0)), smooth=True)
    k.add(cyl(0.044, 0.044, 0.01, 12), "glass", T((0.137, 0, 0), (0, 90, 0)))
    k.add(cyl(0.052, 0.052, 0.05, 12), "bag", T((0.0, 0, 0), (0, 90, 0)), smooth=True)
    k.add(cyl(0.042, 0.042, 0.012, 12), "iron", T((-0.14, 0, 0), (0, 90, 0)))


def build_letter(k):
    """An envelope standing upright facing the front, with a round wax seal carrying the hex."""
    k.add(box(0.3, 0.012, 0.2, 0.004), "paper", T())
    k.add(prism([(-0.145, 0.095), (0.145, 0.095), (0, -0.01)], 0.004), "paper_dk", T((0, -0.008, 0)))
    k.add(cyl(0.04, 0.04, 0.012, 16), "wax", T((0, -0.014, -0.005), (90, 0, 0)), smooth=True)
    k.add(cyl(0.024, 0.024, 0.006, 6), "wax_dk", T((0, -0.022, -0.005), (90, 30, 0)))


PROPS = {
    "pine": build_pine, "birch": build_birch, "reeds": build_reeds,
    "boardwalk": build_boardwalk, "pier": build_pier, "bridge": build_bridge,
    "lighthouse": build_lighthouse, "boat": build_boat, "lantern": build_lantern,
    "stall": lambda k: build_stall(k, ("awning_red", "cream"), "fruit"),
    "stall_b": lambda k: build_stall(k, ("roof_teal", "cream"), "bakery"),
    "cottage": build_cottage, "workshop": build_workshop, "fountain": build_fountain,
    "cliff": build_cliff, "cliff_b": lambda k: build_cliff(k, mossy=True), "stairs": build_stairs,
    "crates": build_crates, "signpost": build_signpost, "bench": build_bench,
    "lens": build_lens, "letter": build_letter,
}
SLOTS = {  # Areas scene layout (Blender x, y); footprints never overlap
    "pine": (-9, 6), "birch": (-7, 6), "reeds": (-5, 6), "crates": (-3, 6), "signpost": (-1, 6),
    "lantern": (1, 6), "lens": (3, 6), "letter": (4, 6), "bench": (6, 6),
    "boardwalk": (-9, 3), "pier": (-8, 3), "stairs": (-6, 3), "cliff": (-5, 3), "cliff_b": (-4, 3),
    "boat": (-1.5, 3), "stall": (1.5, 3), "stall_b": (4.5, 3), "bridge": (7.5, 2.5),
    "lighthouse": (-8.5, -1.5), "fountain": (-5, -1.5), "workshop": (-1, -1.5), "cottage": (4, -1.5),
}


def build_all(scn, only):
    built = {}
    for name, fn in PROPS.items():
        if only and name not in only:
            continue
        coll = bpy.data.collections.new(B.PREFIX + name)
        scn.collection.children.link(coll)
        kit = B.Kit(name)
        after = fn(kit)
        ob = kit.build(coll)
        if name in WIND:
            B.add_wind(ob, WIND[name])
        built[name] = [ob] + (after(ob, coll) if after else [])
        ob.location = (*SLOTS[name], 0)
        coll.instance_offset = (*SLOTS[name], 0)  # instances (preview) show the prop at their own origin
    return built


def preview(scn, path, aim=(-1.0, 0.5, 26.0)):
    """Contact sheet of the Areas scene: a cliff terrace, decks, the props; game-like 3/4 camera."""
    coll = bpy.data.collections.new(B.PREFIX + "areas_pv")
    scn.collection.children.link(coll)

    def inst(name, x, y, z=0, rot=0):
        e = bpy.data.objects.new("pv_" + name, None)
        e.instance_type = "COLLECTION"
        e.instance_collection = bpy.data.collections[B.PREFIX + name]
        e.location = (x, y, z)
        e.rotation_euler = (0, 0, math.radians(rot))
        coll.objects.link(e)
    # extra copies to check tiling: a 3 x 2 cliff terrace (two high at the back), a boardwalk and a pier run
    names = {o.name for o in scn.objects}
    if "cliff" in names:
        for ix in range(3):
            for iy in range(2):
                for z in range(1 + iy):
                    inst("cliff_b" if (ix + iy + z) % 2 else "cliff", -5 + ix, -6 + iy, z, 90 * ((ix * 3 + iy) % 4))
        inst("stairs", -6, -6, 0, 0)
    if "boardwalk" in names:
        for i in range(4):
            inst("boardwalk", 1 + i, -6)
            inst("pier", 1 + i, -7)
    g = B.Kit("pv_ground")
    g.add(box(24, 20, 0.02), "grass", T((-1, 0, -0.012)))
    g.add(box(6, 3, 0.02), "water", T((3, -7.5, -0.008)))
    g.build(coll)
    cam = bpy.data.objects.new("pv_cam", bpy.data.cameras.new(B.PREFIX + "areas_cam"))
    cam.data.lens = 30
    pitch, (ax, ay, dist) = 50, aim
    cam.rotation_euler = (math.radians(90 - pitch), 0, 0)
    cam.location = (ax, ay - dist * math.cos(math.radians(pitch)), dist * math.sin(math.radians(pitch)))
    coll.objects.link(cam)
    scn.camera = cam
    sun = bpy.data.objects.new("pv_sun", bpy.data.lights.new(B.PREFIX + "areas_sun", "SUN"))
    sun.data.energy = 3.5
    sun.rotation_euler = (math.radians(40), math.radians(20), math.radians(-30))
    coll.objects.link(sun)
    world = bpy.data.worlds.new(B.PREFIX + "areas_world")
    world.use_nodes = True
    bg = next(n for n in world.node_tree.nodes if n.type == "BACKGROUND")
    bg.inputs["Color"].default_value = (0.55, 0.62, 0.7, 1)
    bg.inputs["Strength"].default_value = 0.9
    scn.world = world
    scn.render.resolution_x, scn.render.resolution_y = 1800, 1200
    scn.view_settings.view_transform = "Standard"
    scn.render.filepath = path
    scn.render.image_settings.file_format = "PNG"
    for eng in ("BLENDER_EEVEE", "BLENDER_EEVEE_NEXT", "BLENDER_WORKBENCH"):
        try:
            scn.render.engine = eng
            break
        except TypeError:
            continue
    bpy.ops.render.render(write_still=True, scene=scn.name)
    print("[sixfold] areas preview ->", path)


def main(argv):
    arg = lambda k: argv[argv.index(k) + 1] if k in argv else None  # noqa: E731
    out, pv, only, cam = arg("--out"), arg("--preview"), arg("--only"), arg("--cam")  # --cam x,y,distance
    only = set(only.split(",")) if only else None
    scn = B.fresh_scene("Areas")
    with B.use_scene(scn):
        built = build_all(scn, only)
        if out:
            for name, objs in built.items():
                for e in objs[1:]:  # Blender suffixes repeated names (light_origin.001, door.001); the game wants the plain one
                    base = e.name.split(".")[0]
                    other = bpy.data.objects.get(base)
                    if other and other is not e:
                        other.name = base + "_parked"
                    e.name = base
                B.export({name: objs}, out)
        if pv:
            preview(scn, os.path.abspath(pv), *([tuple(map(float, cam.split(",")))] if cam else []))
    B.save_if_file()


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
