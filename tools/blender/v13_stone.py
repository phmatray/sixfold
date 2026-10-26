"""Stone: The Stonebacks (Fighting). Slate mountains and mining terraces, rails and carts, a headframe, cairns;
cool blue-greys with rust-orange accents; the dark Undermine under it all.

Vocabulary: stacked slate plates with dark seams (cliffs, stairs, rubble, cairns, menhirs), rust-red painted
iron (headframe, cart, rails, gutters), dark mine timber (support frames, cladding), pale cave crystals.
Glow parts: sn_gem_glow (crystals, the glints in the cart's ore), sn_window_glow (terrace house windows),
sn_lamp_glow (headframe and porch lamps).
"""
import math
import random

import bmesh
from mathutils import Vector

from v13_kit import door_leaf  # noqa: E402
from v13_kit import (B, T, along, beam, box, centre, chain, contact, cyl, empty, facet_block, ico, jitter, lathe,  # noqa: F401
                     prism, quad, ribbon, rock, shadow, sphere, torus, tube, curve, clamp01, FRONT)

PALETTE = {
    "sn_slate": "#6c7380", "sn_slate_dk": "#535a67", "sn_slate_lt": "#838a96", "sn_slate_top": "#a3a7ad", "sn_shard": "#b4b8bd",
    "sn_bed_a": "#58606f", "sn_bed_b": "#4b5262", "sn_bed_c": "#646c7b", "sn_bed_d": "#525a6b", "sn_crack": "#2f343e",
    "sn_top_a": "#7a818d", "sn_top_b": "#6e7582", "sn_top_c": "#858b96", "sn_drip": "#b05f3a",
    "sn_stain": "#9c6048",
    "sn_break": "#b9bdc3",
    "sn_seam": "#3f4550", "sn_rusty": "#7e6e6a", "sn_lichen": "#b5a862", "sn_menhir": "#8b9097", "sn_menhir_dk": "#72777f", "sn_grass": "#7f9a4c",
    "sn_rust": "#b85a32", "sn_rust_dk": "#8a4126", "sn_steel": "#8a9099", "sn_ballast": "#8d8a84",
    "sn_timber": "#7a5838", "sn_timber_dk": "#5a3f28", "sn_timber_lt": "#9a7450", "sn_shaft": "#16141a",
    "sn_masonry": "#aaa59c", "sn_masonry_dk": "#8d887f", "sn_roof": "#4c5361", "sn_roof_dk": "#3c424e",
    "sn_door": "#a8492f", "sn_frame": "#e9e1cf", "sn_window_glow": "#f4d58a", "sn_lamp_glow": "#ffd98a",
    "sn_gem_glow": "#9fe9f4", "sn_ore": "#5d6470", "sn_carve": "#c4c6c8", "sn_cloth": "#c8553a",
    # 1.3 PR 4: lanterns, the smithy, the items, the Tally Room and the climbing school
    "sn_brass_dk": "#a87a2a", "sn_brass_lt": "#ecc766", "sn_copper": "#c27a43",
    "sn_anvil": "#3b3f47", "sn_anvil_face": "#b9c0c9", "sn_tin": "#b9c0c8", "sn_tin_dk": "#8e959e", "sn_tin_lt": "#d3d8de",
    "sn_glove": "#c96a3a", "sn_glove_dk": "#8a4126", "sn_chalk": "#f4f1ea", "sn_stitch": "#3a2a24", "sn_bag": "#3f6f8a",
    "sn_board": "#3b4549", "sn_board_dk": "#2c3336", "sn_bead_a": "#c8553a", "sn_bead_b": "#e8b649", "sn_bead_c": "#3f7f9a",
    "sn_hold_r": "#e2553a", "sn_hold_y": "#f0b830", "sn_hold_b": "#2f7fd0", "sn_hold_g": "#3f9a52", "sn_hold_p": "#8a4fc9",
    "sn_mat": "#4a6f8a", "sn_mat_dk": "#3a5a72", "sn_plank": "#b98a5c", "sn_plank_lt": "#cca06e", "sn_plank_dk": "#9c7048",
    "sn_rope_red": "#c8473d", "sn_soup": "#d99a3a", "sn_carrot": "#e8813a", "sn_potato": "#e9c86a", "sn_leek": "#7fae5a",
    "sn_char": "#2e2622", "sn_ash": "#5b5652", "sn_ring": "#c9a06a",
    "fire_glow": "#ff8a3a", "fire_core_glow": "#ffd66a",
}

WIND = {"cairn": lambda x, y, z, r: clamp01((z - 0.95) / 0.25)}   # only the pennant moves

FOOT = {"headframe": (3, 3), "minecart": (2, 1), "terrace_house": (4, 3), "cave_mouth_slate": (2, 1),
        "hold_wall": (2, 1), "tally_board": (2, 1)}
INTERIOR = {"hold_wall", "rope_coil", "tally_board", "soup_cauldron"}   # exported to <out>/interior/
FOOT_OTHER = {"seal_stone": (2, 2), "cave_mouth": (2, 1), "pine": (1, 1), "campfire": (1, 1)}
ANIM = set()

LIGHT = dict(sun=("#ffffff", 2.6), from_hero=(8, 18, 6), hemi=("#f4f1ea", "#5d5a55", 1.5), bg="#c9c3b8", fog=(16, 34))


def drop_bottom(bm):
    """Remove a solid's lowest cap (hidden in the ground or inside a block)."""
    zmin = min(v.co.z for v in bm.verts)
    bmesh.ops.delete(bm, geom=[f for f in bm.faces if f.normal.z < -0.9 and all(abs(v.co.z - zmin) < 1e-5 for v in f.verts)],
                     context="FACES")
    return bm


def plate(rng, w, d, t, amt=0.015, bev=0.01):
    """A flat slate plate with chipped edges."""
    return jitter(box(w, d, t, bev), rng, amt, squash=0.3)


# ---------------------------------------------------------------- terrain: layered slate

# Thin beds, each tilted a few degrees about X or Y in turn (so any 90 deg turn still looks like bedding),
# thick ones proud and thin dark ones recessed: (z0, z1, jut)
BEDS = ((-0.02, 0.12, 0.022), (0.12, 0.15, -0.03), (0.15, 0.27, 0.018), (0.27, 0.36, 0.026), (0.36, 0.39, -0.03),
        (0.39, 0.5, 0.02), (0.5, 0.61, 0.024), (0.61, 0.64, -0.03), (0.64, 0.75, 0.016), (0.75, 0.86, 0.022))


def slate_block(k, rng, lichen=False):
    """1 x 1 x 1 slate: thin tilted beds in four blue-greys with dark recessed partings, rust stains fanning
    down from the rim, broken plate lips overhanging at the top, and a top of three or four plates at exactly 1.0 with
    dark cracks between, tonal patches and loose chips a few mm proud."""
    tones = ["sn_bed_a", "sn_bed_b", "sn_bed_c", "sn_bed_d"]
    thick = []   # (half width, half height, placement) of each proud bed, top first: the stains are laid on these
    for i, (z0, z1, jut) in enumerate(BEDS):
        thin = jut < 0
        j = jut + rng.uniform(-0.006, 0.006)
        tilt = (rng.uniform(2.0, 4.0) * rng.choice([-1, 1]), 0, 0) if i % 2 else (0, rng.uniform(2.0, 4.0) * rng.choice([-1, 1]), 0)
        off = (0, 0) if thin else (rng.uniform(-0.01, 0.01), rng.uniform(-0.01, 0.01))
        M = T((off[0], off[1], (z0 + z1) / 2), tilt if not thin else (0, 0, 0))
        k.add(facet_block(rng, 1.0 + j, 1.0 + j, z0 - (z0 + z1) / 2, z1 - (z0 + z1) / 2, bulge=0.0 if thin else 0.01, cuts=1),
              "sn_crack" if thin else rng.choice(tones), M)
        if not thin:
            thick.insert(0, ((1.0 + j) / 2, (z1 - z0) / 2, M))
    k.add(facet_block(rng, 1.0, 1.0, 0.86, 0.995, bulge=0.0, cuts=0, top_in=True), "sn_crack")
    # broken plate lips overhanging at the rim, on two sides
    for side in rng.sample(range(4), 2):
        a = math.radians(90 * side)
        nx, ny = round(math.cos(a)), round(math.sin(a))
        u, L = rng.uniform(-0.2, 0.2), rng.uniform(0.3, 0.5)
        lip = jitter(box(L if ny else 0.14, 0.14 if ny else L, 0.06), rng, 0.012, squash=0.3)
        k.add(lip, rng.choice(tones), T((nx * 0.5 + (-ny) * u, ny * 0.5 + nx * u, 0.93), (rng.uniform(-6, 6), rng.uniform(-6, 6), 0)))
    # rust stains on two faces: an iron-rich bed weathered rust-brown along part of its length (ragged ends, a hotter
    # core), a paler wash on the bed under it, and thin runs dribbling a few beds down from it, fading toward slate.
    # Flat polygons laid on each bed's own face (tilted with it, 1-2 cm proud), so the dark partings cross them.
    def on_bed(lv, side, pts, mat, proud):
        if lv >= len(thick):
            return
        hw, hh, M = thick[lv]
        a = math.radians(90 * side)
        n, tv = Vector((round(math.cos(a)), round(math.sin(a)), 0)), Vector((-round(math.sin(a)), round(math.cos(a)), 0))
        bm = bmesh.new()
        f = bm.faces.new([bm.verts.new(n * (hw + proud) + tv * u + Vector((0, 0, (hh - 0.006) * (2 * v - 1)))) for u, v in pts])
        f.normal_update()
        if f.normal.dot(n) < 0:
            f.normal_flip()
        k.add(bm, mat, M)

    def patch(lv, side, ua, ub, mat, proud, v0=0.0, v1=1.0):   # a ragged-ended strip along a bed, v = 0 bottom .. 1 top
        r = lambda: rng.uniform(0.0, 0.07)  # noqa: E731
        vm = (v0 + v1) / 2
        on_bed(lv, side, [(ua + r(), v1), (ub - r(), v1), (ub + r() * 0.6, vm), (ub - r(), v0), (ua + r(), v0), (ua - r() * 0.6, vm)],
               mat, proud)

    for si, side in enumerate(rng.sample(range(4), 2)):
        lv = rng.choice([0, 1, 1, 2, 3])
        L = rng.uniform(0.5, 0.85) * (1.0 if si == 0 else 0.7)
        ua = rng.uniform(-0.48, 0.48 - L)
        patch(lv, side, ua, ua + L, "sn_stain", 0.011)
        c = ua + L * rng.uniform(0.3, 0.6)
        patch(lv, side, c - L * 0.22, c + L * 0.22, "sn_drip", 0.017, rng.uniform(0.0, 0.35), 1.0)
        patch(lv + 1, side, ua + L * rng.uniform(0.1, 0.3), ua + L * rng.uniform(0.6, 0.85), "sn_rusty", 0.011, rng.uniform(0.2, 0.5), 1.0)
        for _ in range(rng.choice([2, 3])):  # thin runs, each a little narrower and paler per bed, ending in a point
            u, w = rng.uniform(ua + 0.05, ua + L - 0.05), rng.uniform(0.016, 0.026)
            depth = rng.randint(2, 3)
            for d in range(depth):
                last = d == depth - 1
                wb = 0.0 if last else w * 0.75
                u2 = u + rng.uniform(-0.012, 0.012)
                v_end = rng.uniform(0.0, 0.5) if last else 0.0
                on_bed(lv + 1 + d, side, [(u - w, 1.0), (u + w, 1.0), (u2 + max(wb, 0.003), v_end), (u2 - max(wb, 0.003), v_end)],
                       "sn_stain" if d == 0 else "sn_rusty", 0.017)
                u, w = u2, wb
    # the top: a random split into plates (top exactly 1.0), inset so dark cracks show between them
    sx, sy = rng.uniform(-0.2, 0.2), rng.uniform(-0.2, 0.2)
    cells = [(-0.5, -0.5, sx, sy), (sx, -0.5, 0.5, sy + rng.uniform(-0.12, 0.12)), (-0.5, sy, sx + rng.uniform(-0.15, 0.15), 0.5)]
    cells.append((cells[2][2], cells[1][3], 0.5, 0.5))
    cells[1] = (sx, -0.5, 0.5, cells[1][3])
    for n, (x0, y0, x1, y1) in enumerate(cells):
        g = 0.012   # cracks only between plates: the outer edges reach the tile edge, so runs don't show the grid
        gx0, gx1 = (0 if x0 <= -0.5 else g), (0 if x1 >= 0.5 else g)
        gy0, gy1 = (0 if y0 <= -0.5 else g), (0 if y1 >= 0.5 else g)
        x0, x1, y0, y1 = x0 + gx0, x1 - gx1, y0 + gy0, y1 - gy1
        w, d = x1 - x0, y1 - y0
        if w < 0.08 or d < 0.08:
            continue
        top = 1.0 if n != 3 else 0.997   # one plate sits a little lower: the rim is never one flat slab
        k.add(drop_bottom(box(w, d, top - 0.9)), ("sn_top_a", "sn_top_b", "sn_top_c", "sn_top_b")[n],
              T(((x0 + x1) / 2, (y0 + y1) / 2, (top + 0.9) / 2)))
    for i in range(2 + lichen):  # loose chips
        k.add(plate(rng, rng.uniform(0.08, 0.16), rng.uniform(0.06, 0.1), 0.012, 0.004, bev=0), rng.choice(["sn_shard", "sn_top_c", "sn_bed_c"]),
              T((rng.uniform(-0.35, 0.35), rng.uniform(-0.35, 0.35), 1.006 + 0.004 * i), (rng.uniform(-6, 6), rng.uniform(-6, 6), rng.uniform(0, 180))))
    if lichen:  # an ochre lichen patch and a tuft of grass in a crack
        patch = jitter(cyl(0.11, 0.11, 0.006, 8), rng, 0.03, squash=0.0)
        k.add(patch, "sn_lichen", T((rng.uniform(-0.25, 0.25), rng.choice([-1, 1]) * 0.3, 1.003), (0, 0, rng.uniform(0, 90)), (1.3, 0.6, 1)))
        for i in range(8):
            h = rng.uniform(0.07, 0.14)
            k.add(cyl(0.014, 0.0, h, 3), "sn_grass", T((sx + rng.uniform(-0.04, 0.04), sy + rng.uniform(-0.04, 0.04), 0.995 + h / 2),
                                                       (rng.uniform(-18, 18), rng.uniform(-18, 18), rng.uniform(0, 120))))


def build_cliff_slate(k):
    slate_block(k, random.Random(601))


def build_cliff_slate_b(k):
    slate_block(k, random.Random(602), lichen=True)


def build_stairs_slate(k):
    """1 x 1 slate steps up toward -Z (glTF): tread tops 0.125, 0.375, 0.625, 0.875 (floor 0.5, stacks with y),
    overhanging slab treads, layered slate cheeks rising to the back."""
    rng = random.Random(603)
    for i in range(4):
        y0 = -0.5 + 0.25 * i
        topz = 0.125 + 0.25 * i
        y1 = 0.5 - 0.006 * i
        k.add(drop_bottom(box(0.74 - 0.012 * i, y1 - y0, topz - 0.03 + 0.02)), "sn_slate_dk", T((0, (y0 + y1) / 2, (topz - 0.03 - 0.02) / 2)))
        k.add(plate(rng, 0.8, 0.29, 0.045, 0.006), "sn_slate_top" if i % 2 else "sn_slate_lt", T((0, y0 + 0.13, topz - 0.0225)))
    for sx in (-1, 1):  # cheeks: a stepped stack of plates, each a little proud of the one below
        for i in range(4):
            y0 = -0.5 + 0.25 * i
            z1 = 0.25 * (i + 1) + 0.04 * (i == 3)
            z0 = 0.25 * i - (0.02 if i == 0 else 0.0)
            m = ("sn_slate_dk", "sn_slate", "sn_slate_lt", "sn_slate")[i]
            k.add(facet_block(rng, 0.13, 0.5 - y0 if i < 3 else 0.25, z0, z1, bulge=0.008), m,
                  T((sx * 0.445, (y0 + 0.5) / 2 if i < 3 else 0.375, 0)))
    contact(k, 0.5)


# ---------------------------------------------------------------- landmarks


def build_headframe(k):
    """3 x 3 winding tower over the old shaft: four rust-red legs braced in three bays, a head platform with
    two spoked sheave wheels facing the front, a cage hanging in the timber shaft collar, the rope running
    down to a winding drum on a trestle, a ladder up one leg and a lamp on top."""
    rng = random.Random(611)
    top = 4.6
    corners = [(sx, sy) for sx in (-1, 1) for sy in (-1, 1)]
    leg = lambda sx, sy, z: (sx * (1.15 - 0.62 * z / top), sy * (0.95 - 0.55 * z / top), z)  # noqa: E731
    for sx, sy in corners:  # concrete footings, then the legs
        k.add(box(0.36, 0.36, 0.22, 0.03), "sn_masonry_dk", T((sx * 1.15, sy * 0.95, 0.08)))
        beam(k, leg(sx, sy, 0.15), leg(sx, sy, top), 0.14, 0.14, "sn_rust")
    levels = (0.15, 1.6, 3.1, top)
    for z in levels[1:]:  # girts round each level (sides a little higher and slimmer: no shared faces at the corners)
        for n, ((ax, ay), (bx, by)) in enumerate((((-1, -1), (1, -1)), ((1, -1), (1, 1)), ((1, 1), (-1, 1)), ((-1, 1), (-1, -1)))):
            dz, h = (0.0, 0.1) if n % 2 == 0 else (0.017, 0.084)
            a, b = leg(ax, ay, z), leg(bx, by, z)
            beam(k, (a[0], a[1], z + dz), (b[0], b[1], z + dz), 0.09 if n % 2 == 0 else 0.08, h, "sn_rust_dk" if z == top else "sn_rust")
    for z0, z1 in zip(levels, levels[1:]):  # X bracing on the front and both sides (the back stays open)
        for (ax, ay), (bx, by) in (((-1, -1), (1, -1)), ((1, -1), (1, 1)), ((-1, 1), (-1, -1))):
            beam(k, leg(ax, ay, z0 + 0.1), leg(bx, by, z1 - 0.05), 0.05, 0.05, "sn_rust_dk")
            a, b = Vector(leg(bx, by, z0 + 0.1)), Vector(leg(ax, ay, z1 - 0.05))
            out = Vector((ax + bx, ay + by, 0)).normalized() * 0.03   # the second diagonal sits a little proud
            beam(k, a + out, b + out, 0.045, 0.045, "sn_rust_dk")
    # head platform with a rail
    k.add(box(1.5, 1.2, 0.1, 0.02), "sn_timber_dk", T((0, 0, top + 0.08)))
    for n, (x0, y0, x1, y1) in enumerate(((-0.75, -0.6, 0.75, -0.6), (0.75, -0.6, 0.75, 0.6), (-0.75, 0.6, -0.75, -0.6))):
        beam(k, (x0, y0, top + 0.55 + 0.012 * n), (x1, y1, top + 0.55 + 0.012 * n), 0.04, 0.04, "sn_rust_dk")
    for x, y in ((-0.75, -0.6), (0.75, -0.6), (0.75, 0.6), (-0.75, 0.6)):
        k.add(box(0.05, 0.05, 0.47), "sn_rust_dk", T((x * 1.02, y * 1.02, top + 0.36)))
    # two sheave wheels side by side in the XZ plane (they face the camera), axle bearings on A-frames
    R, wz = 0.6, top + 0.85
    for wx in (-0.36, 0.36):
        k.add(torus(R, 0.055, 20, 4), "sn_rust", T((wx, 0.0, wz), (90, 0, 0)))
        k.add(torus(R - 0.1, 0.025, 16, 4), "sn_rust_dk", T((wx, 0.0, wz), (90, 0, 0)))
        for i in range(6):
            a = math.radians(i * 60 + 15)
            yo = (i % 3 - 1) * 0.006   # spokes 60 deg apart never share a plane
            beam(k, (wx + 0.08 * math.cos(a), yo, wz + 0.08 * math.sin(a)), (wx + (R - 0.05) * math.cos(a), yo, wz + (R - 0.05) * math.sin(a)), 0.035, 0.035, "sn_rust_dk")
        k.add(cyl(0.09, 0.09, 0.16, 8), "sn_steel", T((wx, 0, wz), (90, 0, 0)))
    k.add(cyl(0.035, 0.035, 0.9, 6), "sn_steel", T((0, 0, wz), (0, 90, 0)))
    for wx in (-0.62, 0.62):  # A-frame bearings
        for sy in (-1, 1):
            beam(k, (wx + sy * 0.014, sy * 0.45, top + 0.13), (wx + sy * 0.014, 0, wz), 0.06, 0.06, "sn_rust_dk")
    # shaft collar: a timber frame round a dark hole, the cage hanging in it
    k.add(box(1.5, 1.5, 0.1, 0.02), "sn_masonry_dk", T((0, 0, 0.03)))
    k.add(box(1.0, 1.0, 0.02), "sn_shaft", T((0, 0, 0.09)))
    for sx in (-1, 1):
        k.add(box(0.16, 1.32, 0.16, 0.02), "sn_timber_dk", T((sx * 0.58, 0, 0.16)))
        k.add(box(1.0, 0.16, 0.16, 0.02), "sn_timber_dk", T((0, sx * 0.58, 0.165)))
    cx, cz0, cz1 = -0.36 - R, 0.95, 1.95   # the cage hangs under the left wheel's outer rim
    cx = -0.36 + R - 0.6                   # ... pulled to the shaft centre (rope drops from the wheel rim)
    for sx in (-1, 1):
        for sy in (-1, 1):
            k.add(box(0.05, 0.05, cz1 - cz0), "sn_steel", T((cx + sx * 0.3, sy * 0.3, (cz0 + cz1) / 2)))
    for z in (cz0, cz1):
        k.add(box(0.66, 0.66, 0.05, 0.01), "sn_rust_dk", T((cx, 0, z)))
    k.add(box(0.62, 0.02, 0.7), "sn_timber", T((cx, 0.29, (cz0 + cz1) / 2)))   # the cage's back boards
    k.add(tube([(cx, 0, cz1 + 0.02), (cx, 0, wz - 0.02)], 0.018, 4), "sn_steel")
    # rope from the right wheel's rim down to the winding drum on its trestle (front right)
    dx, dz = 1.05, 0.62
    k.add(tube([(0.36 + R, 0, wz), (dx + 0.1, -0.7, dz + 0.22)], 0.018, 4), "sn_steel")
    k.add(cyl(0.22, 0.22, 0.5, 12), "sn_timber_lt", T((dx, -0.9, dz), (0, 90, 0)))
    for x in (dx - 0.27, dx + 0.27):
        k.add(cyl(0.26, 0.26, 0.04, 12), "sn_rust_dk", T((x, -0.9, dz), (0, 90, 0)))
        beam(k, (x - 0.012, -1.25, 0.0), (x - 0.012, -0.9, dz), 0.07, 0.07, "sn_timber_dk")
        beam(k, (x + 0.012, -0.55, 0.0), (x + 0.012, -0.9, dz), 0.07, 0.07, "sn_timber_dk")
    k.add(box(0.06, 0.25, 0.06), "sn_steel", T((dx + 0.32, -0.9 - 0.12, dz)))  # crank
    # ladder up the front-left leg
    for i in range(11):
        z = 0.45 + i * 0.38
        a, b = leg(-1, -1, z), leg(1, -1, z)
        p = (a[0] + 0.32, a[1] - 0.06, z)
        beam(k, p, (p[0] + 0.3, p[1], z), 0.03, 0.03, "sn_steel")
    for off in (0.32, 0.62):
        beam(k, (leg(-1, -1, 0.15)[0] + off, -0.95 - 0.06, 0.15), (leg(-1, -1, top)[0] + off, leg(-1, -1, top)[1] - 0.06, top), 0.035, 0.035, "sn_steel")
    # lamp on the platform corner
    k.add(box(0.03, 0.03, 0.5), "iron", T((0.7, -0.55, top + 0.35)))
    k.add(box(0.14, 0.14, 0.18), "sn_lamp_glow", T((0.7, -0.55, top + 0.68)))
    k.add(cyl(0.12, 0.0, 0.1, 4), "iron", T((0.7, -0.55, top + 0.82), (0, 0, 45)))
    for _ in range(4):  # slate chips round the collar
        k.add(plate(rng, rng.uniform(0.12, 0.2), rng.uniform(0.08, 0.14), 0.03), "sn_slate",
              T((rng.uniform(-1.2, 1.2), rng.uniform(0.5, 1.3), 0.012), (rng.uniform(-8, 8), 0, rng.uniform(0, 180))))
    shadow(k, 1.45, 1.0, 0.9)
    return empty("light_origin", (0.7, -0.55, top + 0.68))


def build_terrace_house(k):
    """4 x 3 miner's house backed into a cliff (back flush with the north edge): slate-rubble ground floor, a
    timber-clad upper floor, a lean-to slate roof falling to the front with a rust gutter, a stone chimney, a
    rust-red door on column 1 of the front row (origin + (1, 2)) under a little canopy with a lamp."""
    rng = random.Random(621)
    by, fy, bk = 0.2, -1.1, 1.5          # wall box centre y, front wall face, back face (flush)
    wd = bk - fy
    k.add(box(3.9, wd + 0.1, 0.22, 0.03), "sn_masonry_dk", T((0, (fy + bk) / 2 - 0.03, 0.08)))
    k.add(box(3.7, wd - 0.05, 1.55), "sn_masonry", T((0, (fy + bk) / 2, 0.9)))
    for i in range(26):  # rubble stones on the front and sides of the ground floor
        x, z = rng.uniform(-1.75, 1.75), rng.uniform(0.3, 1.55)
        if -0.95 < x < -0.05 or 0.55 < x < 1.45 and 0.65 < z < 1.45:
            continue
        k.add(plate(rng, rng.uniform(0.2, 0.36), 0.05, rng.uniform(0.09, 0.15), 0.01), rng.choice(["sn_masonry_dk", "sn_slate_lt", "sn_masonry_dk"]),
              T((x, fy - 0.025, z)))
    k.add(box(3.8, wd, 0.1, 0.02), "sn_timber_dk", T((0, (fy + bk) / 2, 1.7)))
    k.add(box(3.66, wd - 0.1, 1.15), "sn_timber", T((0, (fy + bk) / 2, 2.3)))
    for i in range(14):  # vertical cladding boards
        x = -1.7 + i * 0.262
        k.add(box(0.2, 0.03, 1.12), rng.choice(["sn_timber_lt", "sn_timber", "sn_timber_lt"]), T((x, fy - 0.0, 2.3)))
    for sx in (-1, 1):
        for i in range(9):
            y = fy + 0.15 + i * (wd - 0.3) / 8
            k.add(box(0.03, 0.2, 1.12), rng.choice(["sn_timber_lt", "sn_timber"]), T((sx * 1.845, y, 2.3)))
    # lean-to roof: high at the back (4.0), eave at the front (2.95)
    zf, zb = 2.95, 3.95
    yf, yb = fy - 0.45, bk
    slope = math.degrees(math.atan2(zb - zf, yb - yf))
    L = math.hypot(zb - zf, yb - yf)
    roofM = T((0, (yf + yb) / 2, (zf + zb) / 2 + 0.06), (slope, 0, 0))
    k.add(box(4.2, L, 0.12, 0.02), "sn_roof", roofM)
    for i in range(6):  # tile courses
        k.add(box(4.22, 0.04, 0.03), "sn_roof_dk", roofM @ T((0, -L / 2 + 0.2 + i * (L - 0.3) / 5, 0.07)))
    k.add(box(4.25, 0.14, 0.12, 0.02), "sn_rust", T((0, yb - 0.07, zb + 0.13)))           # flashing at the cliff
    k.add(cyl(0.07, 0.07, 4.26, 8), "sn_rust", T((0, yf - 0.02, zf - 0.02), (0, 90, 0)))    # gutter
    k.add(cyl(0.04, 0.04, 2.8, 6), "sn_rust_dk", T((1.98, yf + 0.0, 1.55)))                  # downpipe
    # gable infill under the sloping roof, both sides
    for sx in (-1, 1):
        k.add(prism([(fy, 2.85), (bk - 0.02, 2.85), (bk - 0.02, zb - 0.02), (fy, zf + 0.0)], 0.1), "sn_timber_dk", T((sx * 1.82, 0, 0), (0, 0, 90)))
    # chimney at the back right
    k.add(box(0.55, 0.55, 2.2, 0.03), "sn_masonry_dk", T((1.35, 1.1, 3.6)))
    k.add(box(0.66, 0.66, 0.1, 0.02), "sn_slate_dk", T((1.35, 1.1, 4.72)))
    for px in (-0.12, 0.12):
        k.add(cyl(0.07, 0.06, 0.18, 8), "sn_rust_dk", T((1.35 + px, 1.1, 4.86)), smooth=True)
    # door on column 1 (x = -0.5), step, canopy and lamp
    k.add(box(0.95, 0.1, 1.85, 0.02), "sn_timber_dk", T((-0.5, fy - 0.04, 1.05)))
    k.add(box(0.76, 0.01, 1.68), "doorway", T((-0.5, fy - 0.1, 1.0)))  # behind the leaf: dark when it swings in
    leaf = [(box(0.78, 0.08, 1.7, 0.02), "sn_door", T((-0.5, fy - 0.08, 1.0)))]
    for z in (0.5, 1.45):
        leaf.append((box(0.78, 0.03, 0.08), "sn_rust_dk", T((-0.5, fy - 0.13, z))))
    leaf.append((sphere(0.04, 6, 4), "brass", T((-0.22, fy - 0.15, 1.0))))
    k.add(box(1.2, 0.34, 0.14, 0.03), "sn_slate_top", T((-0.5, fy - 0.2, 0.06)))
    k.add(box(1.25, 0.55, 0.06, 0.02), "sn_roof", T((-0.5, fy - 0.27, 2.12), (-14, 0, 0)))
    for sx in (-1, 1):
        beam(k, (-0.5 + sx * 0.55, fy - 0.05, 1.7), (-0.5 + sx * 0.55, fy - 0.45, 2.07), 0.05, 0.05, "sn_timber_dk")
    k.add(box(0.03, 0.22, 0.03), "iron", T((-1.2, fy - 0.12, 1.95)))
    k.add(box(0.13, 0.13, 0.17), "sn_lamp_glow", T((-1.2, fy - 0.24, 1.82)))
    k.add(cyl(0.11, 0.0, 0.08, 4), "iron", T((-1.2, fy - 0.24, 1.94), (0, 0, 45)))
    # windows: ground floor right, two above; glowing panes at night
    for wx, wz, ww, wh in ((1.0, 1.05, 0.8, 0.65), (-0.95, 2.35, 0.6, 0.55), (0.75, 2.35, 0.6, 0.55)):
        k.add(box(ww + 0.12, 0.1, wh + 0.12, 0.02), "sn_frame", T((wx, fy - 0.05, wz)))
        k.add(box(ww, 0.04, wh), "sn_window_glow", T((wx, fy - 0.1, wz)))
        k.add(box(0.04, 0.04, wh - 0.01), "sn_frame", T((wx, fy - 0.13, wz)))
        k.add(box(ww - 0.01, 0.03, 0.04), "sn_frame", T((wx, fy - 0.13, wz)))
    k.add(box(0.95, 0.22, 0.16, 0.02), "sn_timber_dk", T((1.0, fy - 0.16, 0.66)))      # window box with flowers
    for i in range(5):
        k.add(sphere(0.06, 6, 4), "petal" if i % 2 else "sn_lichen", T((0.66 + i * 0.17, fy - 0.16, 0.78)), smooth=True)
    # a pick and a shovel leaning by the door, a barrel on the right
    beam(k, (-1.25, fy - 0.3, 0.0), (-1.18, fy - 0.08, 1.05), 0.04, 0.04, "sn_timber_lt")
    k.add(prism([(-0.22, 0), (0.22, 0), (0.18, 0.04), (0, 0.06), (-0.18, 0.04)], 0.04), "sn_steel", T((-1.18, fy - 0.1, 1.0), (0, 0, 0)))
    beam(k, (-1.55, fy - 0.3, 0.12), (-1.52, fy - 0.08, 1.0), 0.035, 0.035, "sn_timber_lt")
    k.add(box(0.2, 0.04, 0.26, 0.01), "sn_steel", T((-1.55, fy - 0.31, 0.15), (-12, 0, 0)))
    k.add(cyl(0.2, 0.2, 0.55, 10, 0.02), "sn_timber", T((1.6, fy - 0.3, 0.26)), smooth=True)
    for z in (0.08, 0.45):
        k.add(cyl(0.21, 0.21, 0.04, 10), "sn_rust_dk", T((1.6, fy - 0.3, z)))
    shadow(k, 2.0, 1.0, 0.8)
    return door_leaf(leaf, T((-0.89, fy - 0.08, 0.15)), empty("smoke_origin", (1.35, 1.1, 4.95)))


# ---------------------------------------------------------------- mining kit


def build_minecart(k):
    """2 x 1 ore tub along X on four wheels at the rail gauge (0.6 m, wheels on a 0.075 m rail top): rust-red
    body with iron bands, heaped with slate ore and a few glowing crystal glints."""
    rng = random.Random(631)
    zb, zt = 0.32, 0.88
    k.add(prism([(-0.6, zb), (0.6, zb), (0.72, zt), (-0.72, zt)], 0.74), "sn_rust", T())
    for x in (-0.42, 0.0, 0.42):  # bands down the two long sides
        for sy in (-1, 1):
            k.add(box(0.07, 0.02, zt - zb - 0.02), "sn_rust_dk", T((x, sy * 0.375, (zb + zt) / 2)))
    k.add(ribbon([(-0.73, -0.38), (0.73, -0.38), (0.73, 0.38), (-0.73, 0.38)], 0.05, 0.05, closed=True), "sn_rust_dk", T((0, 0, zt - 0.02)))
    for _ in range(12):  # the ore heap
        r = rng.uniform(0.13, 0.22)
        k.add(rock(rng, r, 1, 0.18), rng.choice(["sn_ore", "sn_slate", "sn_slate_dk"]),
              T((rng.uniform(-0.48, 0.48), rng.uniform(-0.22, 0.22), zt - 0.03 + rng.uniform(0, 0.08)), scale=(1, 1, 0.7)))
    for x, y, a in ((-0.2, 0.05, 20), (0.3, -0.1, -15), (0.05, 0.15, 35)):
        k.add(cyl(0.04, 0.0, 0.18, 6), "sn_gem_glow", T((x, y, zt + 0.12), (a, a * 0.5, 0)))
    # chassis, axles, wheels (flanged), couplings
    k.add(box(1.25, 0.5, 0.08, 0.02), "sn_timber_dk", T((0, 0, zb - 0.02)))
    for x in (-0.45, 0.45):
        k.add(cyl(0.025, 0.025, 0.7, 6), "sn_steel", T((x, 0, 0.215), (90, 0, 0)))
        for sy in (-1, 1):
            k.add(cyl(0.14, 0.14, 0.05, 12), "iron", T((x, sy * 0.3, 0.215), (90, 0, 0)))
            k.add(cyl(0.165, 0.165, 0.02, 12), "iron", T((x, sy * 0.3 - sy * 0.03, 0.215), (90, 0, 0)))
            k.add(cyl(0.05, 0.05, 0.06, 6), "sn_rust_dk", T((x, sy * 0.3 + sy * 0.01, 0.215), (90, 0, 0)))
    for sx in (-1, 1):
        k.add(box(0.16, 0.08, 0.06), "iron", T((sx * 0.8, 0, zb + 0.02)))
        k.add(torus(0.04, 0.012, 8, 4), "iron", T((sx * 0.9, 0, zb + 0.02), (90, 0, 0)))
    shadow(k, 0.85, 1.0, 0.5)


def build_rail(k):
    """1 x 1 walkable track along X: ballast strip (top 1.5 cm), three sleepers, two rails (top 0.075 m) at
    a 0.6 m gauge, reaching the X edges so tiles chain; rot 90 for a track along Z."""
    rng = random.Random(641)
    k.add(drop_bottom(box(1.0, 0.92, 0.025)), "sn_ballast", T((0, 0, 0.0025)))
    for x in (-0.333, 0.0, 0.333):
        k.add(drop_bottom(box(0.13, 0.84, 0.032, 0.006)), "sn_timber_dk", T((x + rng.uniform(-0.01, 0.01), 0, 0.03)))
    for y in (-0.3, 0.3):
        k.add(box(1.0, 0.04, 0.03), "sn_steel", T((0, y, 0.06)))


def build_rubble(k):
    """1 x 1 fall of slate that fills the passage (~1.3 m): big plates heaped and leaning on each other over a scree
    mound, two with fresh pale broken edges, a half-buried snapped pit prop, chips spilled at the foot.
    The Undermine's prop_closed."""
    rng = random.Random(651)
    k.add(drop_bottom(rock(rng, 0.56, 1, 0.1)), "sn_bed_b", T((0, 0.02, 0.0), scale=(0.98, 0.95, 0.85)))
    for i in range(12):  # lower plates flatter and wider, upper ones steeper, all leaning in toward the middle
        t = i / 11
        a = rng.uniform(0, math.tau) if i else 4.4
        d = 0.38 * (1 - t) ** 0.7 + rng.uniform(0, 0.04)
        x, y = d * math.cos(a), d * math.sin(a) + 0.02
        tilt = 12 + 60 * t + rng.uniform(-8, 8)
        m = "sn_break" if i in (6, 10) else rng.choice(["sn_bed_a", "sn_bed_c", "sn_bed_c", "sn_bed_d", "sn_slate_lt"])
        k.add(plate(rng, rng.uniform(0.55, 0.75) * (1 - 0.35 * t), rng.uniform(0.34, 0.48), 0.085, 0.022), m,
              T((x, y, 0.24 + 0.85 * t), (tilt * math.sin(a), -tilt * math.cos(a), math.degrees(a) + 90)))
    k.add(plate(rng, 0.22, 0.3, 0.1, 0.02), "sn_break", T((0.05, 0.0, 1.17), (70, 10, 30)))   # the top shard
    beam(k, (-0.32, -0.12, 0.4), (0.28, 0.32, 0.98), 0.09, 0.09, "sn_timber_dk")
    k.add(prism([(-0.045, 0), (0.045, 0), (0.03, 0.1), (0.0, 0.04), (-0.02, 0.13)], 0.09), "sn_timber_lt",
          T((0.28, 0.32, 0.98), (-35, 30, 45)))
    for i in range(7):
        k.add(plate(rng, rng.uniform(0.09, 0.17), rng.uniform(0.06, 0.12), 0.025, bev=0), rng.choice(["sn_bed_c", "sn_break", "sn_bed_a"]),
              T((rng.uniform(-0.48, 0.48), rng.uniform(-0.52, -0.38), 0.012 + 0.004 * i), (rng.uniform(-10, 10), rng.uniform(-10, 10), rng.uniform(0, 180))))
    contact(k, 0.6)


def build_cave_mouth_slate(k):
    """2 x 1 mine adit at the foot of a slate cliff (the cliff stands on the row behind, >= 2 m; back flush with
    the north edge): jambs and lintel of tilted slate beds, a timber mine frame set just inside, a dark recess,
    rails running out over the sill, rust drips under the lintel and a rusted lamp bracket. The door warp is its tile."""
    rng = random.Random(661)
    tones = ["sn_bed_a", "sn_bed_b", "sn_bed_c", "sn_bed_d"]
    z = -0.02
    for i, h in enumerate((0.26, 0.22, 0.3, 0.24, 0.28, 0.26, 0.3, 0.25)):
        tilt = (0, (-1) ** i * 2.5, 0)
        for sx in (-1, 1):  # jambs
            k.add(facet_block(rng, 0.5 + rng.uniform(0, 0.04), 0.92, -h / 2, h / 2, bulge=0.012), rng.choice(tones),
                  T((sx * 0.75, 0.04, z + h / 2), tilt))
        if z > 1.3:  # lintel across the top
            k.add(facet_block(rng, 1.06, 0.92, -h / 2, h / 2, bulge=0.012), rng.choice(tones), T((0, 0.04, z + h / 2), tilt))
        z += h + 0.012
    k.add(drop_bottom(box(2.02, 0.94, 0.06)), "sn_top_b", T((0, 0.04, z + 0.02)))
    for x in (-0.3, 0.05, 0.35):  # drips under the lintel
        L = rng.uniform(0.3, 0.55)
        k.add(prism([(-0.025, 0), (0.025, 0), (0.006, -L), (-0.006, -L)], 0.01), "sn_drip", T((x, -0.43, 1.95)))
    for x, zz, r in ((-0.88, 0.18, 0.16), (0.92, 0.12, 0.12), (0.62, 0.06, 0.08)):
        k.add(rock(rng, r, 1, 0.2), "sn_bed_c", T((x, -0.42, zz), scale=(1, 0.8, 0.7)))
    # dark recess
    k.add(drop_bottom(box(1.0, 0.04, 1.3)), "sn_shaft", T((0, 0.44, 0.63)))
    for sx in (-1, 1):
        k.add(drop_bottom(box(0.04, 0.8, 1.32)), "sn_shaft", T((sx * 0.52, 0.0, 0.64)))
    k.add(box(1.08, 0.88, 0.04), "sn_shaft", T((0, 0.02, 1.33)))
    k.add(drop_bottom(box(0.98, 0.86, 0.024)), "sn_ballast", T((0, 0.02, 0.002)))
    # timber mine frame just inside the mouth
    for sx in (-1, 1):
        k.add(drop_bottom(box(0.12, 0.12, 1.26)), "sn_timber", T((sx * 0.42, -0.3, 0.62)))
    k.add(box(1.0, 0.14, 0.14, 0.015), "sn_timber_dk", T((0, -0.3, 1.26)))
    for sx in (-1, 1):
        beam(k, (sx * 0.4, -0.3, 0.98), (sx * 0.18, -0.3, 1.2), 0.07, 0.07, "sn_timber_dk")
    # rails out over the sill
    for y0, y1 in ((-0.5, 0.4),):
        for x in (-0.22, 0.22):
            k.add(box(0.035, y1 - y0, 0.03), "sn_steel", T((x, (y0 + y1) / 2, 0.055)))
        for y in (-0.38, -0.05, 0.28):
            k.add(drop_bottom(box(0.62, 0.11, 0.032)), "sn_timber_dk", T((0, y, 0.026)))
    # rusted lamp bracket on the right jamb
    k.add(box(0.04, 0.22, 0.04), "sn_rust_dk", T((0.62, -0.5, 1.45)))
    k.add(box(0.13, 0.13, 0.16), "sn_lamp_glow", T((0.62, -0.62, 1.33)))
    k.add(cyl(0.11, 0.0, 0.08, 4), "sn_rust_dk", T((0.62, -0.62, 1.45), (0, 0, 45)))


def build_cairn(k):
    """1 x 1 cairn (~1.2 m): flat stones stacked and turned, a capstone, a stick with a rust-red pennant."""
    rng = random.Random(661)
    z = 0.0
    for i, (r, h) in enumerate(((0.34, 0.16), (0.3, 0.14), (0.27, 0.15), (0.22, 0.13), (0.18, 0.12), (0.14, 0.11))):
        bm = jitter(cyl(r, r * 0.88, h, 7), rng, 0.018, squash=0.5)
        if i == 0:
            drop_bottom(bm)
        k.add(bm, ("sn_slate", "sn_slate_lt", "sn_slate_dk", "sn_slate_lt", "sn_slate", "sn_slate_top")[i],
              T((rng.uniform(-0.03, 0.03), rng.uniform(-0.03, 0.03), z + h / 2 - (0.02 if i == 0 else 0.004)), (rng.uniform(-5, 5), rng.uniform(-5, 5), rng.uniform(0, 60))))
        z += h - 0.006
    k.add(rock(rng, 0.09, 1, 0.1), "sn_slate_top", T((0, 0, z + 0.04), scale=(1, 1, 1.3)))
    for _ in range(4):  # lichen dabs
        a = rng.uniform(0, math.tau)
        zz = rng.uniform(0.1, 0.5)
        k.add(sphere(0.045, 6, 4), "sn_lichen", T((0.3 * math.cos(a), 0.3 * math.sin(a), zz), scale=(1, 1, 0.5)))
    k.add(cyl(0.015, 0.012, 0.75, 5), "sn_timber_lt", T((0.06, 0.04, z + 0.3), (4, -6, 0)))
    k.add(prism([(0, 0), (0.24, 0.04), (0.0, 0.12)], 0.012), "sn_cloth", T((0.09, 0.04, z + 0.52)))
    for _ in range(3):  # stones waiting to be added
        a = rng.uniform(math.pi * 0.9, math.pi * 1.9)
        k.add(plate(rng, rng.uniform(0.1, 0.16), rng.uniform(0.08, 0.12), 0.05, bev=0), "sn_slate", T((0.42 * math.cos(a), 0.4 * math.sin(a), 0.02), (0, 0, rng.uniform(0, 180))))
    contact(k, 0.45)


def build_support_beam(k):
    """1 x 1 mine timber set: two posts at the tile's side edges, a cap beam across with lagging boards on top,
    knee braces, wedges and stone pads. Spans the tile along X (walk under it along Z; rot 90 to turn)."""
    rng = random.Random(671)
    H = 2.0
    for sx in (-1, 1):
        k.add(box(0.26, 0.26, 0.1, 0.02), "sn_slate_dk", T((sx * 0.4, 0, 0.03)))
        k.add(box(0.16, 0.16, H - 0.08), "sn_timber", T((sx * 0.4, 0, (H - 0.08) / 2 + 0.06), (0, sx * 2, 0)))
        beam(k, (sx * 0.38, 0, H - 0.45), (sx * 0.15, 0, H - 0.1), 0.08, 0.08, "sn_timber_dk")
        k.add(prism([(-0.06, 0), (0.06, 0), (0.0, 0.1)], 0.14), "sn_timber_lt", T((sx * 0.4, 0, H - 0.18), (0, 0, 0)))
    k.add(box(1.0, 0.2, 0.2, 0.015), "sn_timber_dk", T((0, 0, H)))
    for i in range(5):  # lagging across the cap
        k.add(box(0.17, 0.62, 0.04), rng.choice(["sn_timber", "sn_timber_lt"]), T((-0.38 + i * 0.19, rng.uniform(-0.03, 0.03), H + 0.12 + 0.004 * (i % 2))))
    k.add(box(0.03, 0.03, 0.2), "iron", T((0.2, -0.11, H - 0.17)))   # a hook (for the hero's lantern, in the story)
    k.add(torus(0.035, 0.008, 8, 4), "iron", T((0.2, -0.11, H - 0.29), (90, 0, 0)))
    shadow(k, 0.55, 1.0, 0.4)


def build_crystal_cluster(k):
    """1 x 1 cluster of pale cave crystals (~0.95 m) growing out of a slate boulder; they glow at night."""
    rng = random.Random(681)
    k.add(drop_bottom(rock(rng, 0.36, 1, 0.14)), "sn_slate_dk", T((0, 0.04, 0.05), scale=(1.15, 1.0, 0.6)))
    k.add(rock(rng, 0.16, 1, 0.15), "sn_slate", T((0.3, -0.2, 0.05), scale=(1, 1, 0.6)))
    for x, y, r, h, tx, ty in ((0.0, 0.05, 0.13, 0.62, 0, 0), (-0.17, -0.06, 0.09, 0.42, -22, 10), (0.16, 0.1, 0.1, 0.48, 18, 14),
                               (0.05, -0.16, 0.08, 0.34, 8, -26), (-0.1, 0.2, 0.07, 0.3, -12, 25), (0.26, -0.1, 0.06, 0.22, 30, -10),
                               (-0.28, 0.12, 0.055, 0.2, -32, 6)):
        M = T((x, y, 0.16), (ty, tx, rng.uniform(0, 60)))
        k.add(cyl(r, r * 0.92, h, 6), "sn_gem_glow", M @ T((0, 0, h / 2)))
        k.add(cyl(r * 0.92, 0.0, r * 1.6, 6), "sn_gem_glow", M @ T((0, 0, h + r * 0.8)))
    for _ in range(3):
        a = rng.uniform(0, math.tau)
        k.add(cyl(0.03, 0.0, 0.1, 6), "sn_gem_glow", T((0.4 * math.cos(a), 0.4 * math.sin(a), 0.04), (rng.uniform(-30, 30), rng.uniform(-30, 30), 0)))
    contact(k, 0.48)


def build_menhir(k):
    """1 x 1 standing stone (~2.5 m) for the Bastion's ring: a rugged, weathered slab, wide at the foot and
    tapering to a lopsided point, leaning a little, a carved hex outline on its face, ochre lichen, packing stones."""
    rng = random.Random(691)
    bm = box(0.66, 0.42, 2.45)
    bmesh.ops.subdivide_edges(bm, edges=bm.edges[:], cuts=3, use_grid_fill=True)
    for v in bm.verts:
        t = max(0.0, v.co.z / 2.45 + 0.5)
        v.co.x *= 1 - 0.42 * t ** 1.4
        v.co.y *= 1 - 0.3 * t
        if t > 0.99:
            v.co.z += 0.32 * (v.co.x / 0.2) - 0.05   # the point leans to one shoulder
        v.co += jitter_v(rng, 0.04)
    drop_bottom(bm)
    lean = T((0, 0, 0), (-3, 4, 8))
    k.add(bm, "sn_menhir", lean @ T((0, 0, 1.2)))
    for z, sx in ((0.55, -1), (1.45, 1)):  # weathered darker flakes on the shoulders
        k.add(plate(rng, 0.14, 0.32, 0.05, 0.015), "sn_menhir_dk", lean @ T((sx * (0.33 - 0.12 * z / 2.45), 0, z), (0, sx * 78, 0)))
    hx = [(0.16 * math.cos(math.radians(30 + 60 * i)), 0.16 * math.sin(math.radians(30 + 60 * i))) for i in range(6)]
    k.add(ribbon(hx, 0.035, 0.014, closed=True), "sn_carve", lean @ T((0, -0.205, 1.3), FRONT))
    k.add(cyl(0.045, 0.045, 0.014, 6), "sn_carve", lean @ T((0, -0.207, 1.3), (90, 30, 0)))
    for i in range(3):
        z = 0.35 + 0.6 * i
        patch = jitter(cyl(0.09, 0.09, 0.02, 7), rng, 0.03, squash=0.0)
        k.add(patch, "sn_lichen", lean @ T(((-1) ** i * 0.12, -0.205 + 0.004 * i, z), (90, 0, 0), (1.4, 0.8, 1)))
    for i in range(6):
        a = i / 6 * math.tau + rng.uniform(0, 0.4)
        k.add(rock(rng, rng.uniform(0.08, 0.13), 1, 0.15), "sn_slate_dk" if i % 2 else "sn_slate_lt",
              T((0.38 * math.cos(a), 0.3 * math.sin(a), 0.03), scale=(1, 1, 0.6)))
    contact(k, 0.5)


def jitter_v(rng, a):
    from mathutils import Vector
    return Vector((rng.uniform(-a, a), rng.uniform(-a, a), rng.uniform(-a, a) * 0.5))


# ---------------------------------------------------------------- 1.3 PR 4: gloves wall, mine lanterns, the smithy


def build_climb_wall_slate(k):
    """1 x 1 climb (grip gloves), the climb_wall contract: two slate steps (tops 0.5 front, 1.0 back), floor 0.5,
    climbs toward -Z, stacks with y. Thin tilted beds with dark recessed partings, a plated top, rusty iron staples
    and pegs hammered into the risers, a knotted rope from a ring peg on the top step down both risers."""
    rng = random.Random(605)
    tones = ["sn_bed_a", "sn_bed_b", "sn_bed_c", "sn_bed_d"]
    for n, (y0, y1, top) in enumerate(((-0.5, 0.0, 0.5), (0.0, 0.5, 1.0))):
        yc, d = (y0 + y1) / 2, y1 - y0
        z = -0.02
        if n:  # the back step's foot is hidden behind the front step: one plain block
            k.add(drop_bottom(facet_block(rng, 1.0, d + 0.01, -0.02, 0.44, bulge=0.006, cuts=0)), "sn_bed_b", T((0, yc, 0)))
            z = 0.44
        i = 0
        while z < top - 0.09:
            h = min(rng.uniform(0.1, 0.14), top - 0.06 - z)
            j = rng.uniform(0.012, 0.026)
            tilt = (rng.uniform(2.0, 3.5) * rng.choice([-1, 1]), 0, 0) if i % 2 else (0, rng.uniform(2.0, 3.5) * rng.choice([-1, 1]), 0)
            k.add(facet_block(rng, 1.0 + j, d + j, -h / 2, h / 2, bulge=0.01, cuts=1), tones[(i + n) % 4],
                  T((rng.uniform(-0.008, 0.008), yc, z + h / 2), tilt))
            z += h
            if z < top - 0.09:  # a thin dark parting, recessed, bridging into the beds above and below
                k.add(facet_block(rng, 0.97, d - 0.03, -0.018, 0.018, bulge=0.0, cuts=0), "sn_crack", T((0, yc, z + 0.012)))
                z += 0.024
            i += 1
        # the top: a dark crack layer and two or three plates at exactly `top`, one a little low
        k.add(facet_block(rng, 0.99, d - 0.01, top - 0.075, top - 0.016, bulge=0.0, cuts=0, top_in=True), "sn_crack", T((0, yc, 0)))
        sx = rng.uniform(-0.2, 0.2)
        cells = [(-0.5, sx - 0.006, top), (sx + 0.006, 0.5, top - 0.003)]
        for m, (x0, x1, zt) in enumerate(cells):
            k.add(drop_bottom(box(x1 - x0, d, 0.06)), ("sn_top_a", "sn_top_c")[(m + n) % 2], T(((x0 + x1) / 2, yc, zt - 0.03)))
        # rusty staples (rungs) and pegs on the riser
        fy = y0 - 0.02
        for q in range(3):
            x = -0.3 + 0.26 * q + rng.uniform(-0.03, 0.03)
            zz = top - 0.34 + 0.13 * ((q + n) % 2)
            k.add(ribbon([(-0.07, 0), (-0.07, -0.07), (0.07, -0.07), (0.07, 0)], 0.032, 0.03), "sn_rust", T((x, fy, zz), (90, 0, 0)))
        for x, zz in ((-0.4, top - 0.2), (0.36, top - 0.12)):
            k.add(cyl(0.022, 0.016, 0.1, 6), "sn_rust_dk", T((x, fy - 0.02, zz), (90, 0, 0)))
    # a knotted rope from a ring peg on the top step, over both risers
    k.add(cyl(0.024, 0.016, 0.16, 6), "sn_rust_dk", T((0.16, 0.14, 1.04)))
    k.add(torus(0.04, 0.01, 8, 4), "sn_rust", T((0.16, 0.08, 1.06), (90, 0, 0)))
    k.add(ribbon([(0.16, 1.03), (0.17, 0.8), (0.15, 0.62), (0.18, 0.5)], 0.026, 0.02), "rope", T((0, -0.045, 0), FRONT))
    k.add(ribbon([(0.18, 0.5), (0.17, 0.3), (0.19, 0.12), (0.16, -0.01)], 0.026, 0.02), "rope", T((0, -0.545, 0), FRONT))
    for x, y, z in ((0.17, -0.058, 0.82), (0.155, -0.058, 0.66), (0.17, -0.558, 0.33), (0.185, -0.558, 0.15)):
        k.add(sphere(0.028, 6, 4), "rope", T((x, y, z)), smooth=True)
    contact(k, 0.5)


def oil_lantern(k, x, y, z, s=1.0):
    """A miner's oil lantern with its flame at (x, y, z): a brass fount, a glowing glass globe (sn_lamp_glow) caged
    by four iron guards, a vented iron cap, a wire bail up to about z + 0.21 s."""
    k.add(cyl(0.085 * s, 0.09 * s, 0.06 * s, 8), "brass", T((x, y, z - 0.115 * s)))
    k.add(cyl(0.07 * s, 0.085 * s, 0.02 * s, 8), "sn_brass_dk", T((x, y, z - 0.08 * s)))
    k.add(lathe([(0.05 * s, z - 0.08 * s), (0.08 * s, z - 0.035 * s), (0.085 * s, z + 0.015 * s), (0.065 * s, z + 0.065 * s),
                 (0.045 * s, z + 0.085 * s)], 8, 22.5), "sn_lamp_glow", T((x, y, 0)))
    for a in (45, 135, 225, 315):
        ca, sa = math.cos(math.radians(a)), math.sin(math.radians(a))
        beam(k, (x + 0.088 * s * ca, y + 0.088 * s * sa, z - 0.075 * s), (x + 0.092 * s * ca, y + 0.092 * s * sa, z + 0.07 * s),
             0.012 * s, 0.012 * s, "iron")
    k.add(cyl(0.075 * s, 0.035 * s, 0.05 * s, 8), "iron", T((x, y, z + 0.1 * s)))
    k.add(cyl(0.03 * s, 0.03 * s, 0.04 * s, 6), "iron", T((x, y, z + 0.14 * s)))
    k.add(cyl(0.045 * s, 0.0, 0.03 * s, 6), "iron", T((x, y, z + 0.175 * s)))
    bail = torus(0.095 * s, 0.006 * s, 10, 3)
    bmesh.ops.delete(bail, geom=[v for v in bail.verts if v.co.y < -0.01 * s], context="VERTS")   # keep the top arc
    k.add(bail, "iron", T((x, y, z + 0.11 * s), (90, 0, 0)))


def build_oil_lantern(k):
    """1 x 1 mine timber set (the support_beam frame: posts at the tile's X edges, a cap beam with lagging, knee
    braces) with a miner's oil lantern hung from a hook under the cap; its flame at 1.6 m (light_origin). Walk under
    it along Z like support_beam; the lantern hangs off-centre (x 0.2) with its foot at ~1.46 m."""
    rng = random.Random(672)
    H = 2.0
    for sx in (-1, 1):
        k.add(box(0.26, 0.26, 0.1, 0.02), "sn_slate_dk", T((sx * 0.4, 0, 0.03)))
        k.add(box(0.16, 0.16, H - 0.08), "sn_timber", T((sx * 0.4, 0, (H - 0.08) / 2 + 0.06), (0, sx * 2, 0)))
        beam(k, (sx * 0.38, 0, H - 0.45), (sx * 0.15, 0, H - 0.1), 0.08, 0.08, "sn_timber_dk")
        k.add(prism([(-0.06, 0), (0.06, 0), (0.0, 0.1)], 0.14), "sn_timber_lt", T((sx * 0.4, 0, H - 0.18)))
    k.add(box(1.0, 0.2, 0.2, 0.015), "sn_timber_dk", T((0, 0, H)))
    for i in range(5):
        k.add(box(0.17, 0.62, 0.04), rng.choice(["sn_timber", "sn_timber_lt"]), T((-0.38 + i * 0.19, rng.uniform(-0.03, 0.03), H + 0.12 + 0.004 * (i % 2))))
    lx, ly, lz = 0.2, -0.04, 1.6
    k.add(box(0.04, 0.04, 0.07), "iron", T((lx, ly, H - 0.12)))                     # hook plate and hook
    k.add(torus(0.025, 0.007, 8, 3), "iron", T((lx, ly, H - 0.17), (90, 0, 0)))
    k.add(torus(0.02, 0.006, 8, 3), "iron", T((lx, ly, H - 0.205), (0, 90, 0)))     # a chain link
    oil_lantern(k, lx, ly, lz)
    shadow(k, 0.55, 1.0, 0.4)
    return empty("light_origin", (lx, ly, lz))


def build_oil_lantern_post(k):
    """1 x 1 lantern post for a miners' camp or a cave mouth: a short squared timber post (1.7 m) on a slate pad,
    a bracketed arm reaching out to the front right, an oil lantern hanging from its end (flame at 1.28 m,
    light_origin), wedges and a few stones at the foot."""
    rng = random.Random(673)
    px, py = -0.18, 0.14
    k.add(drop_bottom(box(0.36, 0.36, 0.12, 0.02)), "sn_slate_dk", T((px, py, 0.04)))
    k.add(box(0.15, 0.15, 1.66), "sn_timber", T((px, py, 0.85), (0, 1.5, 0)))
    k.add(cyl(0.11, 0.0, 0.07, 4), "sn_timber_dk", T((px, py, 1.71), (0, 0, 45)))   # weathering cap
    for a in (0, 90, 180):
        ca, sa = math.cos(math.radians(a)), math.sin(math.radians(a))
        k.add(prism([(-0.05, 0), (0.05, 0), (0.0, 0.14)], 0.1), "sn_timber_lt", T((px + 0.1 * ca, py + 0.1 * sa, 0.08), (0, 0, a + 90)))
    ax, ay, az = 0.24, -0.06, 1.56          # arm end
    beam(k, (px - 0.04, py + 0.02, az), (ax + 0.04, ay - 0.02, az), 0.08, 0.09, "sn_timber_dk")
    beam(k, (px + 0.02, py, az - 0.4), (px + 0.28, py - 0.13, az - 0.03), 0.06, 0.06, "sn_timber_dk")
    k.add(box(0.05, 0.05, 0.06), "iron", T((ax, ay, az - 0.06)))
    k.add(torus(0.025, 0.007, 8, 3), "iron", T((ax, ay, az - 0.105), (90, 0, 0)))
    oil_lantern(k, ax, ay, 1.28)
    for i in range(4):
        a = rng.uniform(0, math.tau)
        k.add(rock(rng, rng.uniform(0.06, 0.1), 0, 0.2), rng.choice(["sn_slate", "sn_slate_lt"]),
              T((px + 0.32 * math.cos(a), py + 0.28 * math.sin(a), 0.03), scale=(1, 1, 0.6)))
    contact(k, 0.42)
    return empty("light_origin", (ax, ay, 1.28))


def build_anvil(k):
    """1 x 1 smith's anvil (~0.85 m, the hammer handle to ~0.6): a big dark anvil with a bright worn face, a long horn
    (+X), a stepped heel with its hardy hole, strapped to a stout banded stump; a hammer leaning on the front right,
    tongs on the left, a scatter of scale at the foot."""
    rng = random.Random(674)
    stump = lathe([(0.34, -0.02), (0.3, 0.06), (0.27, 0.2), (0.27, 0.4), (0.28, 0.5)], 10)
    for v in stump.verts:
        if 0.0 < v.co.z < 0.48:
            f = rng.uniform(0.95, 1.05)
            v.co.x *= f
            v.co.y *= f
    drop_bottom(stump)
    k.add(stump, "sn_timber_dk")
    k.add(cyl(0.255, 0.255, 0.02, 10), "sn_ring", T((0, 0, 0.5)))
    for z in (0.1, 0.42):
        k.add(cyl(0.285, 0.285, 0.04, 10), "iron", T((0, 0, z)))
    # the anvil: feet, waist, body, face plate, horn, heel step
    k.add(box(0.4, 0.26, 0.06, 0.012), "sn_anvil", T((0, 0, 0.535)))
    k.add(prism([(-0.17, 0.56), (0.15, 0.56), (0.09, 0.62), (0.1, 0.69), (-0.15, 0.69), (-0.12, 0.62)], 0.14), "sn_anvil", T())
    k.add(box(0.42, 0.17, 0.12, 0.01), "sn_anvil", T((-0.04, 0, 0.74)))
    k.add(box(0.39, 0.15, 0.025), "sn_anvil_face", T((-0.04, 0, 0.8025)))
    k.add(cyl(0.07, 0.008, 0.32, 8), "sn_anvil", T((0.32, 0, 0.75), (0, 90, 0), (1, 1.0, 1)))
    k.add(prism([(0.14, 0.69), (0.34, 0.72), (0.14, 0.76)], 0.1), "sn_anvil", T())
    k.add(box(0.1, 0.14, 0.09, 0.008), "sn_anvil", T((-0.29, 0, 0.745)))
    k.add(box(0.035, 0.035, 0.032), "sn_crack", T((-0.29, 0, 0.785)))
    for sx in (-1, 1):   # straps pinning the feet to the stump
        k.add(ribbon([(0.0, 0.0), (0.0, 0.05), (0.06, 0.05)], 0.04, 0.02), "iron", T((sx * 0.215, -0.04, 0.51), (90, 0, 90 - 90 * sx)))
    # hammer leaning on the front right, its head on the ground
    beam(k, (0.36, -0.36, 0.06), (0.21, -0.21, 0.6), 0.035, 0.035, "sn_timber_lt")
    k.add(box(0.07, 0.17, 0.07, 0.008), "sn_steel", T((0.37, -0.37, 0.05), (0, 0, 45)) @ T(rot=(-14, 0, 0)))
    # tongs leaning on the left
    for dx in (-0.025, 0.025):
        beam(k, (-0.42 + dx, -0.3, 0.0), (-0.24 + dx * 0.3, -0.17, 0.58), 0.02, 0.02, "iron")
    k.add(cyl(0.02, 0.02, 0.06, 6), "iron", T((-0.255, -0.18, 0.54), (90, 0, 0)))
    for dx in (-1, 1):
        beam(k, (-0.255, -0.18, 0.54), (-0.235 + dx * 0.04, -0.15, 0.68), 0.022, 0.022, "iron")
    for i in range(4):
        a = rng.uniform(0, math.tau)
        k.add(plate(rng, rng.uniform(0.05, 0.09), rng.uniform(0.04, 0.07), 0.012, bev=0), "sn_crack",
              T((0.4 * math.cos(a), 0.4 * math.sin(a), 0.006 + 0.003 * i), (0, 0, rng.uniform(0, 180))))
    contact(k, 0.46)


# ---------------------------------------------------------------- 1.3 PR 4: items (the popup, centred on the origin)


def glove(k, M, mirror=1):
    """A fingerless climbing glove, hand in the XZ plane, fingers up, palm toward -Y: rust leather back, a chalk-white
    palm, four short open finger stalls with dark stitched tips, a thumb, a wrist cuff with a strap and buckle."""
    k.add(box(0.09, 0.034, 0.1, 0.012), "sn_glove", M @ T((0, 0, 0.05)))
    k.add(box(0.078, 0.012, 0.082), "sn_chalk", M @ T((0, -0.016, 0.052)))
    for i in range(4):
        x = -0.033 + 0.022 * i
        h = 0.034 - 0.006 * abs(i - 1.4)
        k.add(cyl(0.0105, 0.0105, h, 5), "sn_glove", M @ T((x, 0, 0.1 + h / 2)))
        k.add(cyl(0.012, 0.012, 0.007, 5), "sn_stitch", M @ T((x, 0, 0.1 + h)))
    k.add(cyl(0.012, 0.012, 0.035, 5), "sn_glove", M @ T((mirror * 0.05, -0.006, 0.05)) @ T(rot=(0, mirror * 55, 0)) @ T((0, 0, 0.012)))
    k.add(box(0.1, 0.042, 0.03, 0.006), "sn_glove_dk", M @ T((0, 0, -0.012)))
    k.add(box(0.05, 0.048, 0.02), "sn_bag", M @ T((mirror * 0.02, 0, -0.012)))
    k.add(box(0.016, 0.052, 0.024), "brass", M @ T((mirror * -0.008, 0, -0.012)))


def build_grip_gloves(k):
    """Item (the popup): the Grip Gloves. A pair of fingerless climbing gloves, rust-orange leather with chalk-white
    palms and stitched finger stalls, one leaning on the other, and a little blue chalk bag with a drawstring."""
    glove(k, T((-0.05, 0.0, 0.0), (0, -8, 6)), 1)
    glove(k, T((0.07, 0.03, 0.0), (0, 10, 160)), -1)
    bx, by = 0.0, -0.09
    k.add(lathe([(0.0, 0.0), (0.04, 0.0), (0.045, 0.04), (0.04, 0.07)], 8), "sn_bag", T((bx, by, -0.02)))
    k.add(cyl(0.036, 0.036, 0.008, 8), "sn_chalk", T((bx, by, 0.052)))
    k.add(torus(0.042, 0.005, 8, 3), "sn_stitch", T((bx, by, 0.045)))
    k.add(tube([(bx + 0.03, by - 0.03, 0.045), (bx + 0.05, by - 0.05, 0.02), (bx + 0.045, by - 0.055, -0.005)], 0.004, 3), "sn_stitch")
    k.add(sphere(0.008, 5, 3), "brass", T((bx + 0.045, by - 0.055, -0.01)))
    centre(k)


def build_lunch_pail(k):
    """Item (the popup): a miner's tin lunch pail, dented on one side, two rolled bands, a domed lid sitting a little
    askew with a knob, ears and a wire bail handle, a red cloth corner poking out under the lid."""
    body = lathe([(0.0, -0.09), (0.082, -0.09), (0.088, -0.083), (0.095, 0.06), (0.098, 0.07)], 12)
    for v in body.verts:   # the dent
        d = math.hypot(v.co.x - 0.07, v.co.y + 0.06)
        if -0.06 < v.co.z < 0.04 and d < 0.06:
            v.co.x -= 0.018 * (1 - d / 0.06) * 0.7
            v.co.y += 0.018 * (1 - d / 0.06) * 0.6
    k.add(body, "sn_tin")
    for z in (-0.06, 0.035):
        k.add(cyl(0.093 + 0.002 * (z > 0), 0.093 + 0.002 * (z > 0), 0.012, 12), "sn_tin_dk", T((0, 0, z)))
    L = T((0, 0, 0.068), (4, -3, 0))
    k.add(lathe([(0.104, 0.0), (0.104, 0.014), (0.09, 0.03), (0.05, 0.045), (0.0, 0.05)], 12), "sn_tin_lt", L)
    k.add(cyl(0.012, 0.016, 0.018, 6), "sn_tin_dk", L @ T((0, 0, 0.056)))
    k.add(prism([(-0.035, 0.0), (0.035, 0.0), (0.0, -0.045)], 0.004), "sn_cloth", T((-0.03, -0.098, 0.072), (0, 0, 15)))
    for sx in (-1, 1):
        k.add(box(0.012, 0.022, 0.03), "sn_tin_dk", T((sx * 0.099, 0, 0.045)))
    bail = torus(0.1, 0.0045, 12, 3)
    bmesh.ops.delete(bail, geom=[v for v in bail.verts if v.co.y < 0.0], context="VERTS")
    k.add(bail, "iron", T((0, 0, 0.05), (90, 0, 0), (1, 1.15, 1)))
    k.add(box(0.012, 0.004, 0.15), "sn_tin_dk", T((0.0, 0.094, -0.01)))   # the seam at the back
    centre(k)


def build_lucky_pick(k):
    """Item (the popup): a miner's pick, an old curved iron head (one long point, one chisel end) through a collar on
    a worn ash handle, the grip wound with red cord ending in a loop and two tails."""
    k.add(cyl(0.019, 0.023, 0.62, 6), "sn_timber_lt", T((0, 0, 0.0)))
    k.add(cyl(0.026, 0.026, 0.03, 6), "sn_timber", T((0, 0, -0.31)))
    k.add(box(0.07, 0.056, 0.08, 0.008), "sn_steel", T((0, 0, 0.29)))
    for s, taper, flat in ((1, 0.12, 1.0), (-1, 0.55, 0.45)):
        pts = [(s * 0.03 * i, 0, 0.3 - 0.07 * (i / 7) ** 2) for i in range(8)]
        bm = tube(pts, 0.026, 5, taper)
        if flat < 1:
            for v in bm.verts:
                v.co.y *= flat + (1 - flat) * (1 - abs(v.co.x) / 0.21)
        k.add(bm, "sn_anvil")
    n = 40
    k.add(tube([(0.024 * math.cos(i * 0.9), 0.024 * math.sin(i * 0.9), -0.27 + 0.17 * i / n) for i in range(n + 1)], 0.006, 3), "sn_rope_red")
    k.add(torus(0.025, 0.006, 8, 3), "sn_rope_red", T((0.0, -0.035, -0.29), (90, 0, 0)))
    for dx in (-0.01, 0.012):
        k.add(tube([(0.015, -0.02, -0.1), (0.03 + dx, -0.035, -0.13), (0.035 + dx, -0.04, -0.17)], 0.005, 3), "sn_rope_red")
    for bm, *_ in k.parts:
        bmesh.ops.transform(bm, matrix=T(rot=(0, 28, 0)), verts=bm.verts)
    centre(k)


def build_miner_tally(k):
    """Curio (the popup): a miner's brass tally tag, a hexagon with a punched hexagon hole and a row of notches,
    hanging from a small iron ring with three more tags spread round it (a small hex, a round one with three
    punched dots, a notched copper bar). No letters or digits."""
    R = 0.03
    k.add(torus(R, 0.0045, 14, 4), "sn_steel", T((0, 0, 0.0), (90, 0, 0)))

    def hang(a, y, L):
        """The matrix of a tag hanging from the ring at angle a (deg, -90 = bottom), its eye L above its centre."""
        px, pz = R * math.cos(math.radians(a)), R * math.sin(math.radians(a))
        tilt = (a + 90) * 1.0
        return T((px, y, pz), (0, -tilt, 0)) @ T((0, 0, -L))

    def hex_tag(M, Rt, mat, notches):
        F = M @ T(rot=(90, 0, 0))
        pts = [(Rt * 0.68 * math.cos(math.radians(90 + 60 * i)), Rt * 0.68 * math.sin(math.radians(90 + 60 * i))) for i in range(6)]
        k.add(ribbon(pts, Rt * 0.6, 0.006, closed=True), mat, F @ T((0, 0, -0.003)))
        k.add(torus(0.007, 0.0028, 6, 3), mat, F @ T((0, Rt + 0.004, 0), (90, 0, 0)))
        for i in range(notches):
            k.add(box(0.005, 0.012, 0.009), "sn_crack", F @ T((-Rt * 0.42 + i * 0.011, -Rt * 0.86, 0)))
    hex_tag(hang(-90, 0.0, 0.05), 0.042, "brass", 5)
    hex_tag(hang(-150, 0.009, 0.035), 0.022, "sn_brass_lt", 0)
    M = hang(-30, -0.009, 0.04)
    k.add(cyl(0.024, 0.024, 0.005, 10), "sn_brass_dk", M @ T(rot=(90, 0, 0)))
    for a in (200, 270, 340):
        k.add(cyl(0.0045, 0.0045, 0.008, 6), "sn_crack", M @ T((0.012 * math.cos(math.radians(a)), 0, 0.012 * math.sin(math.radians(a))), (90, 0, 0)))
    k.add(torus(0.006, 0.0025, 6, 3), "sn_brass_dk", M @ T((0, 0, 0.028)))
    M = hang(-65, 0.016, 0.045)
    k.add(box(0.016, 0.004, 0.07), "sn_copper", M)
    for i in range(4):
        k.add(box(0.008, 0.008, 0.004), "sn_crack", M @ T((0.008, 0, -0.022 + 0.012 * i)))
    k.add(torus(0.006, 0.0025, 6, 3), "sn_copper", M @ T((0, 0, 0.04)))
    centre(k)


# ---------------------------------------------------------------- 1.3 PR 4: interior (the climbing school, the Tally Room)


def build_hold_wall(k):
    """2 x 1 (interior) against the north wall, 2.6 m: a training wall of vertical planks in a timber frame, studded
    with chunky bolted holds in five colours, chalk smudges round the busy ones, a chalk-dusted blue crash mat in
    front (inside the 2 x 1)."""
    rng = random.Random(681)
    wy, H = 0.4, 2.6
    for sx in (-1, 1):
        k.add(box(0.12, 0.16, H, 0.012), "sn_timber_dk", T((sx * 0.93, wy + 0.02, H / 2 - 0.01)))
    k.add(box(1.96, 0.18, 0.12, 0.012), "sn_timber_dk", T((0, wy + 0.02, H - 0.05)))
    k.add(box(1.76, 0.03, H - 0.14), "sn_timber", T((0, wy + 0.06, H / 2 - 0.05)))
    for i in range(7):
        x = -0.74 + i * (1.48 / 6)
        k.add(box(0.243, 0.04, H - 0.14), ("sn_plank", "sn_plank_lt", "sn_plank_dk")[(i * 2) % 3], T((x, wy - 0.0 + 0.004 * (i % 2), H / 2 - 0.05)))
    face = wy - 0.02
    holds = ("sn_hold_r", "sn_hold_y", "sn_hold_b", "sn_hold_g", "sn_hold_p")
    spots = []
    for i in range(18):
        x = (-0.66, -0.2, 0.25, 0.66, -0.45, 0.02, 0.45)[i % 7] + rng.uniform(-0.1, 0.1)
        z = 0.4 + (i / 17) * 1.9 + rng.uniform(-0.05, 0.05)
        spots.append((x, z))
        r = rng.uniform(0.08, 0.12)
        k.add(jitter(ico(r, 0), rng, r * 0.18), holds[i % 5], T((x, face, z), (0, 0, rng.uniform(0, 90)), (1.3, 0.6, 1.0)))
        k.add(cyl(0.012, 0.012, 0.02, 6), "iron", T((x, face - r * 0.62, z), (90, 0, 0)))
    for x, z in spots[::3]:   # chalk smudges: thin discs straddling the plank face
        k.add(cyl(0.05, 0.05, 0.016, 7), "sn_chalk", T((x + rng.uniform(-0.04, 0.04), face, z + 0.07), (90, 0, 0), (1.2, 0.7, 1)))
    # crash mat
    k.add(box(1.9, 0.82, 0.2, 0.04), "sn_mat", T((0, -0.06, 0.09)))
    k.add(box(1.92, 0.08, 0.05), "sn_mat_dk", T((0, -0.06, 0.14)))
    for i in range(5):
        k.add(box(rng.uniform(0.15, 0.3), rng.uniform(0.1, 0.2), 0.016), "sn_chalk",
              T((rng.uniform(-0.75, 0.75), rng.uniform(-0.38, 0.25), 0.19 + 0.002 * i), (0, 0, rng.uniform(0, 90))))
    k.add(lathe([(0.0, 0.18), (0.07, 0.18), (0.08, 0.31)], 8), "sn_bag", T((0.75, -0.3, 0)))   # a chalk pot on the mat
    k.add(cyl(0.07, 0.07, 0.012, 8), "sn_chalk", T((0.75, -0.3, 0.3)))
    shadow(k, 0.55, 1.9, 1.0)


def build_rope_coil(k):
    """1 x 1 (interior) the climbing school's rope stand, 0.86 m: a post on a cross foot with two pegs, a coiled
    natural rope and a coiled red rope hanging from them, iron rings clipped on the post (generic shapes), a chalk
    bucket on the floor."""
    rng = random.Random(682)
    px, py = 0.0, 0.18
    for a, zc, h in ((0, 0.025, 0.074), (90, 0.021, 0.062)):
        k.add(box(0.7 if a == 0 else 0.5, 0.1, h, 0.01), "sn_timber_dk", T((px, py, zc), (0, 0, a)))
    k.add(box(0.08, 0.08, 0.84, 0.01), "sn_timber", T((px, py, 0.44)))
    k.add(cyl(0.06, 0.0, 0.05, 4), "sn_timber_dk", T((px, py, 0.885), (0, 0, 45)))
    for (x, z, mat, R) in ((-0.08, 0.76, "rope", 0.15), (0.1, 0.5, "sn_rope_red", 0.13)):
        k.add(cyl(0.018, 0.018, 0.3, 6), "sn_timber_lt", T((x, py - 0.18, z), (90, 0, 0)))
        for j in range(4):   # a coil: four fat loops fanned a little apart
            k.add(torus(R - 0.008 * j, 0.03 if mat == "rope" else 0.022, 12, 4), mat, T((x + 0.018 * (j - 1.5), py - 0.15 - 0.05 * j, z - R + 0.02), (90, 0, 6 * (j - 1.5))))
        k.add(torus(0.05, 0.03, 8, 4), mat, T((x, py - 0.23, z - 0.01), (0, 0, 0)))   # the hitch over the peg
    for i, z in enumerate((0.62, 0.36, 0.25)):   # rings clipped on the post's side
        k.add(torus(0.035, 0.007, 10, 3), "sn_steel", T((px + 0.06, py, z), (0, 90, 0), (1, 1.6, 1)))
    bx, by = 0.3, -0.25
    k.add(lathe([(0.0, -0.01), (0.12, -0.01), (0.14, 0.24)], 10), "sn_tin", T((bx, by, 0)))
    k.add(cyl(0.135, 0.135, 0.012, 10), "sn_chalk", T((bx, by, 0.226)))
    k.add(torus(0.141, 0.008, 10, 3), "sn_tin_dk", T((bx, by, 0.24)))
    bail = torus(0.14, 0.005, 10, 3)
    bmesh.ops.delete(bail, geom=[v for v in bail.verts if v.co.y < 0.0], context="VERTS")
    k.add(bail, "iron", T((bx, by, 0.24), (90, 0, 30)))
    contact(k, 0.45)


def build_tally_board(k):
    """2 x 1 (interior) the Tally Room's board against the north wall, 2.2 m: a big slate chalkboard in a timber frame
    on two posts, ruled into rows and covered in chalk tally gates (four strokes and a strike, a few unfinished; no
    letters or digits) and a few hexagon doodles; a chalk ledge with sticks and a felt duster, and a shelf below it
    with a wooden abacus and a box of chalk."""
    rng = random.Random(683)
    by = 0.38
    for sx in (-1, 1):
        k.add(box(0.1, 0.1, 2.2, 0.012), "sn_timber_dk", T((sx * 0.93, by, 1.09)))
        k.add(box(0.2, 0.3, 0.06, 0.012), "sn_timber_dk", T((sx * 0.93, by - 0.05, 0.025)))
    k.add(box(2.0, 0.12, 0.1, 0.012), "sn_timber_dk", T((0, by, 2.15)))
    bz0, bz1 = 0.82, 2.06
    k.add(box(1.76, 0.04, bz1 - bz0), "sn_board", T((0, by, (bz0 + bz1) / 2)))
    for z in (bz0, bz1):
        k.add(box(1.74, 0.07, 0.06), "sn_timber", T((0, by - 0.01, z)))
    for sx in (-1, 1):
        k.add(box(0.06, 0.1, bz1 - bz0 + 0.1), "sn_timber", T((sx * 0.87, by - 0.01, (bz0 + bz1) / 2)))
    fy = by - 0.02 - 0.011   # marks float 11 mm off the board face (no z-fighting)
    N = (0, -1, 0)

    def stroke(x0, z0, x1, z1, w=0.012, off=0.0):
        fy = by - 0.031 - off
        dx, dz = x1 - x0, z1 - z0
        L = math.hypot(dx, dz)
        nx, nz = -dz / L * w / 2, dx / L * w / 2
        k.add(quad([(x0 - nx, fy, z0 - nz), (x1 - nx, fy, z1 - nz), (x1 + nx, fy, z1 + nz), (x0 + nx, fy, z0 + nz)], N), "sn_chalk")
    rows = (1.9, 1.62, 1.34, 1.06)
    for z in rows[1:]:
        stroke(-0.8, z + 0.13, 0.8, z + 0.13, 0.008, 0.003)
    stroke(-0.42, 0.88, -0.42, 2.0, 0.008)
    for r, z in enumerate(rows):
        x = -0.36
        groups = rng.randint(4, 6)
        for g in range(groups):
            n = 4 if g < groups - 1 else rng.randint(1, 4)
            for s_ in range(n):
                xs = x + s_ * 0.04 + rng.uniform(-0.004, 0.004)
                stroke(xs, z - 0.09, xs + rng.uniform(-0.01, 0.01), z + 0.09)
            if n == 4:
                stroke(x - 0.025, z - 0.07, x + 0.145, z + 0.07, 0.011, 0.0015)
            x += 0.21
            if x > 0.72:
                break
        hx = [(-0.62 + 0.06 * math.cos(math.radians(30 + 60 * i)), z + 0.06 * math.sin(math.radians(30 + 60 * i))) for i in range(7)]
        for (ax, az), (bx_, bz) in zip(hx, hx[1:]):
            stroke(ax, az, bx_, bz, 0.01)
    # chalk ledge, sticks and a duster
    k.add(box(1.7, 0.12, 0.03), "sn_timber_lt", T((0, by - 0.07, bz0 - 0.03)))
    for i, m in enumerate(("sn_chalk", "sn_chalk", "sn_hold_y", "sn_chalk", "sn_hold_r")):
        k.add(cyl(0.008, 0.008, 0.07, 5), m, T((-0.5 + 0.09 * i + rng.uniform(-0.02, 0.02), by - 0.09, bz0 - 0.006), (0, 90, rng.uniform(-20, 20))))
    k.add(box(0.14, 0.06, 0.03, 0.006), "sn_timber", T((0.45, by - 0.08, bz0 + 0.005)))
    k.add(box(0.13, 0.055, 0.012), "sn_mat_dk", T((0.45, by - 0.08, bz0 - 0.014)))
    # shelf with the abacus and a chalk box
    sz = 0.42
    k.add(box(1.76, 0.3, 0.04), "sn_timber_lt", T((0, by - 0.08, sz)))
    ax, aw, ah = 0.3, 0.56, 0.36
    A = T((ax, by - 0.1, sz + 0.015), (-6, 0, 0))
    for sx in (-1, 1):
        k.add(box(0.04, 0.05, ah + 0.03), "sn_timber", A @ T((sx * aw / 2, 0, ah / 2)))
    for z in (0.02, ah - 0.02):
        k.add(box(aw - 0.01, 0.062, 0.04), "sn_timber_dk", A @ T((0, 0, z)))
    for r in range(4):
        z = 0.08 + r * (ah - 0.16) / 3
        k.add(cyl(0.005, 0.005, aw, 4), "sn_steel", A @ T((0, 0, z), (0, 90, 0)))
        left = rng.randint(1, 4)
        for b in range(5):
            x = -aw / 2 + 0.05 + 0.045 * b + (0.0 if b < left else 0.2)
            k.add(cyl(0.022, 0.022, 0.035, 6), ("sn_bead_a", "sn_bead_b", "sn_bead_c", "sn_bead_a")[r], A @ T((x, 0, z), (0, 90, 0)))
    k.add(box(0.26, 0.16, 0.1, 0.008), "sn_timber", T((-0.5, by - 0.1, sz + 0.066)))
    for i in range(5):
        k.add(cyl(0.009, 0.009, 0.06, 5), "sn_chalk", T((-0.58 + 0.04 * i, by - 0.1, sz + 0.13), (rng.uniform(-10, 10), 0, 0)))
    shadow(k, 0.5, 2.0, 0.6)


def build_soup_cauldron(k):
    """1 x 1 (interior) the Tally Room's soup, 1.3 m: an iron cauldron of thick orange soup (carrot, potato, leek)
    hung on a chain from an iron tripod over a little log fire (fire_glow, light_origin at the flames) on a slate
    hearth ringed with stones, a ladle hooked on the rim."""
    rng = random.Random(684)
    k.add(drop_bottom(box(0.92, 0.92, 0.06, 0.012)), "sn_slate_dk", T((0, 0, 0.02)))
    for i in range(9):
        a = i / 9 * math.tau + rng.uniform(-0.1, 0.1)
        k.add(rock(rng, rng.uniform(0.07, 0.09), 0, 0.2), rng.choice(["sn_slate", "sn_slate_lt", "sn_slate_top"]),
              T((0.3 * math.cos(a), 0.3 * math.sin(a), 0.08), (0, 0, rng.uniform(0, 90)), (1.2, 1.0, 0.75)))
    k.add(cyl(0.22, 0.22, 0.02, 9), "sn_ash", T((0, 0, 0.055)))
    for a in (20, 110, 200, 290):   # logs
        ca, sa = math.cos(math.radians(a)), math.sin(math.radians(a))
        k.add(tube([(0.22 * ca, 0.22 * sa, 0.08), (0.03 * ca, 0.03 * sa, 0.2)], 0.035, 5), "sn_char" if a % 180 == 20 else "sn_timber_dk")
    flame = [(-0.11, 0), (0.11, 0), (0.08, 0.1), (0.05, 0.16), (0.06, 0.24), (0.0, 0.33), (-0.035, 0.22), (-0.07, 0.15), (-0.09, 0.08)]
    for i, a in enumerate((0, 60, 120)):
        k.add(prism(flame, 0.012), "fire_glow", T((0, 0, 0.05 + 0.012 * i), (0, 0, a)))
        k.add(prism([(x * 0.55, z * 0.6) for x, z in flame], 0.012), "fire_core_glow", T((0, 0, 0.09 + 0.012 * i), (0, 0, a + 30)))
    for i in range(3):   # tripod
        a = math.radians(90 + 120 * i)
        beam(k, (0.4 * math.cos(a), 0.4 * math.sin(a), 0.03), (0.02 * math.cos(a), 0.02 * math.sin(a), 1.3), 0.03, 0.03, "iron")
    k.add(sphere(0.04, 6, 4), "iron", T((0, 0, 1.3)))
    for i in range(3):   # the chain
        k.add(torus(0.02, 0.006, 6, 3), "iron", T((0, 0, 1.24 - 0.035 * i), (90, 90 * (i % 2), 0)))
    pz = 0.52
    k.add(lathe([(0.0, pz - 0.2), (0.12, pz - 0.19), (0.19, pz - 0.12), (0.21, pz - 0.02), (0.19, pz + 0.09), (0.17, pz + 0.12),
                 (0.19, pz + 0.14), (0.165, pz + 0.14), (0.15, pz + 0.12)], 12), "sn_anvil", smooth=True)
    k.add(cyl(0.162, 0.162, 0.012, 12), "sn_soup", T((0, 0, pz + 0.118)))
    for i, m in enumerate(("sn_carrot", "sn_potato", "sn_leek", "sn_carrot", "sn_potato")):
        a = i * 1.3 + 0.4
        r = 0.04 + 0.02 * i
        k.add(jitter(ico(0.026, 0), rng, 0.005), m, T((r * math.cos(a), r * math.sin(a), pz + 0.126), scale=(1, 1, 0.6)))
    bail = torus(0.2, 0.007, 12, 3)
    bmesh.ops.delete(bail, geom=[v for v in bail.verts if v.co.y < 0.0], context="VERTS")
    k.add(bail, "iron", T((0, 0, pz + 0.14), (90, 0, 0), (1, 2.6, 1)))
    for sx in (-1, 1):
        k.add(box(0.03, 0.03, 0.04), "iron", T((sx * 0.2, 0, pz + 0.13)))
    beam(k, (0.05, -0.05, pz + 0.08), (0.2, -0.24, pz + 0.38), 0.022, 0.014, "sn_steel")   # ladle
    k.add(half_bowl(0.045), "sn_steel", T((0.05, -0.05, pz + 0.11)))
    contact(k, 0.47)
    return empty("light_origin", (0, 0, 0.22))


def half_bowl(r):
    bm = sphere(r, 8, 5)
    bmesh.ops.delete(bm, geom=[v for v in bm.verts if v.co.z > 0.3 * r], context="VERTS")
    return bm


PROPS = {
    "cliff_slate": build_cliff_slate, "cliff_slate_b": build_cliff_slate_b, "stairs_slate": build_stairs_slate,
    "headframe": build_headframe, "terrace_house": build_terrace_house, "minecart": build_minecart, "rail": build_rail,
    "rubble": build_rubble, "cave_mouth_slate": build_cave_mouth_slate, "cairn": build_cairn, "support_beam": build_support_beam,
    "crystal_cluster": build_crystal_cluster, "menhir": build_menhir,
    "climb_wall_slate": build_climb_wall_slate, "oil_lantern": build_oil_lantern, "oil_lantern_post": build_oil_lantern_post,
    "anvil": build_anvil, "grip_gloves": build_grip_gloves, "lunch_pail": build_lunch_pail, "lucky_pick": build_lucky_pick,
    "miner_tally": build_miner_tally, "hold_wall": build_hold_wall, "rope_coil": build_rope_coil, "tally_board": build_tally_board,
    "soup_cauldron": build_soup_cauldron,
}

VIGNETTE = dict(
    ground=("slate", "stone", "#8c8b88"),
    cliff=("cliff_slate", "cliff_slate_b"),
    ring=["pine", "rock", "bush_dark"],
    tiles=[
        "333333333333333333",
        "333333333333333333",
        "....222222........",
        "....11111.........",
        ".p................",
        ".p................",
        ".pppppp...........",
        "......p...........",
        "......pppppppp....",
        "..............p...",
        "..............p...",
        "..............p...",
    ],
    props=[
        ("terrace_house", 0, 2), ("stairs_slate", 6, 4), ("stairs_slate", 6, 3, 0, 1),
        ("cave_mouth_slate", 11, 2), ("oil_lantern", 10, 2), ("support_beam", 13, 2), ("crystal_cluster", 14, 2),
        ("crystal_cluster", 9, 3, 90), ("rubble", 15, 3), ("rubble", 16, 3, 90),
        ("headframe", 7, 5), ("rail", 10, 6), ("rail", 11, 6), ("rail", 12, 6), ("rail", 13, 6), ("rail", 14, 6),
        ("rail", 15, 6), ("rail", 16, 6), ("rail", 17, 6), ("minecart", 12, 6),
        ("seal_stone", 15, 9), ("menhir", 14, 8), ("menhir", 17, 8, 90), ("menhir", 13, 10, 180), ("menhir", 17, 11, 270),
        ("cairn", 3, 7), ("cairn", 4, 10), ("waymark", 2, 5), ("rock", 9, 9), ("pine", 0, 9), ("pine", 1, 11),
        ("hero", 5, 6),
        ("climb_wall_slate", 8, 3, 0, 1), ("anvil", 8, 9), ("campfire", 2, 8), ("oil_lantern_post", 3, 9), ("oil_lantern_post", 10, 4),
    ],
    aim=(9, -6.5, 0), dist=21,
)
