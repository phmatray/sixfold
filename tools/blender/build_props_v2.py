"""Props and people for 1.2 (houses, curios) and the 1.3 shared kit, exported as .glb.

Headless:
    blender -b --factory-startup -P tools/blender/build_props_v2.py -- \
        --out web/world/assets [--only oven,bed] [--preview pv.png] [--sheet sheet.png] [--check]

Same conventions as build_assets.py / build_area_props.py (whose helpers it imports): bmesh
primitives, flat palette colours, Blender Z-up with the front facing -Y (glTF +Z), origin at the
footprint centre on the ground, every base sunk 2 cm, nothing coplanar with y = 0. It only ever
writes the files listed below. Materials whose name contains `glow` are the night-emissive parts.

  --preview  game-camera contact sheet of every new model beside the house, cottage, stall and an NPC
  --sheet    per-model tiles (front 3/4 and back 3/4) to check facing and shapes
  --check    z-fighting check (check_coplanar.py) over all new models on a y = 0 ground plane
"""
import json
import math
import os
import random
import sys

import bmesh
import bpy
from mathutils import Vector

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import build_npcs as N  # noqa: E402  (imports and reloads build_assets: keep it first)
import build_area_props as A  # noqa: E402
import build_assets as B  # noqa: E402
from build_assets import T, box, contact, cyl, ico, jitter, prism, sphere  # noqa: E402

B.PALETTE.update(N.PALETTE)
B.PALETTE.update({
    # interior kit
    "oven_clay": "#d9a77a", "oven_glow": "#ffa04a", "soot": "#2a211c", "log_end": "#e0b27a",
    "jar_glass": "#d6eef8", "coin": "#e9c349", "ink": "#25232b", "leather": "#7a4a30",
    "book_a": "#7e3b3b", "book_b": "#2f5f7a", "book_c": "#c99a3a", "book_d": "#4f7a46", "book_e": "#6b4a7b",
    "book_f": "#d9cfb8", "sheet": "#f4ede0", "quilt_a": "#d9574a", "quilt_b": "#e9c36a", "quilt_c": "#5f8fb8",
    "quilt_d": "#7fae6a", "lightbox_glow": "#eef7ff", "net": "#c9b48a", "cork": "#c68a4e", "buoy_red": "#d0453a",
    "oilskin": "#e2b13a", "tin": "#c3c8cf", "lead": "#8a9098",
    # curios
    "feather": "#aab9c6", "feather_dk": "#4f5d6e", "quill_shaft": "#efe9dc", "bottle_glass": "#9fd6c9",
    "fossil_stone": "#c9b89a", "fossil_dk": "#8a7458", "pouch": "#8a5a7a", "pouch_dk": "#6b4060",
    # curio cabinet, signs
    "plum": "#5e3a5e", "plum_dk": "#452945", "velvet": "#7b3f6e", "swap_teal": "#2f7f86",
    "inn_board": "#2c3e6b", "lamp_glow": "#ffe39a",
    # 1.3 shared
    "crystal_glow": "#eef6ff", "seal_body": "#bdb6a8", "seal_top": "#e4ded2", "seal_dk": "#9a9284",
    "seal_grass": "#3f9a52", "seal_fire": "#e1572f", "seal_water": "#2f7fd0", "seal_lightning": "#e2ad19",
    "seal_fighting": "#a8642f", "seal_psychic": "#8a4fc9", "cave_dark": "#18120f", "cave_floor": "#5a4632",
    "vine": "#4f8a3a", "vine_lt": "#78b04e", "rope": "#c8a46a",
    # new people
    "el_hair": "#eeeae6", "el_cardigan": "#8f6fa0", "el_dress": "#3f6b5a", "el_shawl": "#e8d9b0", "el_shoe": "#4a3326",
    "el_frame": "#c9a24a", "el_cane": "#6e4b33",
    "wk_hair": "#4a3326", "wk_cap": "#5a5f6a", "wk_shirt": "#c86a3a", "wk_pants": "#4c5566", "wk_boot": "#5a3f2a",
    "wk_glove": "#d9b35a", "wk_apron": "#8a6040", "wk_scarf": "#2f7f86",
    "sa_hair": "#b9b4ae", "sa_beard": "#d6d1ca", "sa_jumper": "#f1e7d2", "sa_stripe": "#2f4a7a", "sa_pants": "#3a4a5e",
    "sa_boot": "#2a2d33", "sa_hat": "#f0b830",
    "sc_hair": "#2f2420", "sc_coat": "#7a2f3a", "sc_vest": "#e9dcc0", "sc_pants": "#3a3340", "sc_shoe": "#3a2c25",
    "sc_scarf": "#e0a13a", "sc_frame": "#3a2c25", "sc_bag": "#8a5a3a",
})

# ---------------------------------------------------------------- shape helpers


def ribbon(pts, w, d, closed=False):
    """A strip w wide following 2D points (x, y) in the XY plane, from z = 0 to z = d (letters, arches, arrows)."""
    bm = bmesh.new()
    n = len(pts)
    rows = []
    for i, (x, y) in enumerate(pts):
        a = pts[i - 1] if (i > 0 or closed) else pts[i]
        b = pts[(i + 1) % n] if (i < n - 1 or closed) else pts[i]
        tx, ty = b[0] - a[0], b[1] - a[1]
        L = math.hypot(tx, ty) or 1.0
        nx, ny = -ty / L, tx / L
        rows.append([bm.verts.new((x + s * nx * w / 2, y + s * ny * w / 2, z)) for s, z in ((-1, 0), (1, 0), (1, d), (-1, d))])
    for i in range(n if closed else n - 1):
        r0, r1 = rows[i], rows[(i + 1) % n]
        for j in range(4):
            q = (j + 1) % 4
            bm.faces.new((r0[j], r1[j], r1[q], r0[q]))
    if not closed:
        bm.faces.new(rows[0][::-1])
        bm.faces.new(rows[-1])
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return bm


def arc(cx, cy, r, a0, a1, n):
    return [(cx + r * math.cos(math.radians(a0 + (a1 - a0) * i / n)), cy + r * math.sin(math.radians(a0 + (a1 - a0) * i / n)))
            for i in range(n + 1)]


def arch_pts(w, h, n=8):
    """Outline of a round-topped arch, w wide, h tall to the crown, base on y = 0 (left jamb up, arc, right jamb down)."""
    r = w / 2
    return [(-r, 0.0)] + arc(0, h - r, r, 180, 0, n) + [(r, 0.0)]


def torus(R, r, seg=12, sides=5):
    bm = bmesh.new()
    rows = [[bm.verts.new(((R + r * math.cos(b)) * math.cos(a), (R + r * math.cos(b)) * math.sin(a), r * math.sin(b)))
             for b in (j / sides * math.tau for j in range(sides))] for a in (i / seg * math.tau for i in range(seg))]
    for i in range(seg):
        for j in range(sides):
            bm.faces.new((rows[i][j], rows[(i + 1) % seg][j], rows[(i + 1) % seg][(j + 1) % sides], rows[i][(j + 1) % sides]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return bm


def half_sphere(r, u=16, v=10):
    bm = sphere(r, u, v)
    bmesh.ops.bisect_plane(bm, geom=list(bm.verts) + list(bm.edges) + list(bm.faces),
                           plane_co=(0, 0, 0), plane_no=(0, 0, -1), clear_outer=True)
    return bm


FRONT = (90, 0, 0)        # XY-plane shapes onto a -Y (front) face, raised toward -Y
BACK = (90, 0, 180)       # ... onto a +Y (back) face, raised toward +Y, reading correctly from behind
shadow, empty, beam = A.shadow, A.empty, A.beam


# ---------------------------------------------------------------- interior (assets/interior/)


def build_oven(k):
    """2 x 1 brick bread oven against the north wall: clay dome with a glowing arched mouth, flue,
    a work ledge on the right with loaves and the tip jar, logs in an alcove under the mouth."""
    rng = random.Random(201)
    k.add(box(1.9, 0.86, 0.86, 0.03), "brick_wall", T((0, 0.05, 0.41)))
    k.add(box(1.96, 0.94, 0.08, 0.02), "stone", T((0, 0.04, 0.86)))
    dome = half_sphere(0.7, 16, 8)
    k.add(dome, "oven_clay", T((-0.3, 0.1, 0.89), scale=(0.95, 0.6, 0.95)), smooth=True)
    k.add(cyl(0.68, 0.68, 0.06, 16), "stone_dk", T((-0.3, 0.1, 0.92), scale=(0.97, 0.6, 1)))      # dome footing ring
    # mouth: stone arch frame (front at y = -0.37) around a recessed glow
    k.add(ribbon(arch_pts(0.62, 0.5), 0.1, 0.14), "fcap", T((-0.3, -0.23, 0.895), FRONT))
    k.add(prism(arch_pts(0.56, 0.47), 0.1), "oven_glow", T((-0.3, -0.26, 0.895)))
    k.add(box(0.5, 0.04, 0.025), "iron", T((-0.3, -0.36, 0.91)))                                    # sill bar
    # iron door leaning on the right of the mouth
    k.add(prism(arch_pts(0.42, 0.36), 0.025, 0.006), "iron", T((0.14, -0.43, 0.92), (-8, 0, -18)))
    k.add(cyl(0.025, 0.025, 0.08, 6), "wood_dk", T((0.14, -0.46, 1.08), (0, 90, -18)))
    # flue at the back, through the dome
    k.add(box(0.32, 0.32, 1.25, 0.02), "brick", T((-0.3, 0.3, 1.75)))
    k.add(box(0.4, 0.4, 0.08, 0.02), "stone_dk", T((-0.3, 0.3, 2.4)))
    # alcove with logs under the mouth
    k.add(ribbon(arch_pts(0.86, 0.5, 6), 0.08, 0.06), "stone_dk", T((-0.3, -0.38, 0.06), FRONT))
    k.add(prism(arch_pts(0.78, 0.44, 6), 0.02), "soot", T((-0.3, -0.39, 0.07)))
    for i, (x, z) in enumerate(((-0.52, 0.12), (-0.36, 0.12), (-0.2, 0.12), (-0.05, 0.12), (-0.44, 0.26), (-0.28, 0.26), (-0.13, 0.26), (-0.36, 0.39), (-0.2, 0.39))):
        r = rng.uniform(0.06, 0.075)
        k.add(cyl(r, r, 0.3, 7), "wood_dk", T((x + rng.uniform(-0.01, 0.01), -0.3, z), (90, 0, rng.uniform(0, 40))))
        k.add(cyl(r * 0.8, r * 0.8, 0.01, 7), "log_end", T((x, -0.453, z), (90, 0, 0)))
    # work ledge on the right: loaves, a bread peel, the tip jar with coins
    for i in range(3):
        k.add(sphere(0.1, 10, 6), "bread", T((0.45 + 0.13 * i, 0.0 + 0.05 * (i % 2), 0.95), (0, 0, rng.uniform(-25, 25)), (0.62, 1.0, 0.45)), smooth=True)
    k.add(cyl(0.065, 0.065, 0.16, 12), "jar_glass", T((0.8, -0.24, 0.98)), smooth=True)
    k.add(cyl(0.055, 0.055, 0.06, 12), "coin", T((0.8, -0.24, 0.93)))
    k.add(cyl(0.07, 0.07, 0.02, 12), "wood_dk", T((0.8, -0.24, 1.07)))


def build_printing_press(k):
    """2 x 1 hand press: a tall frame with the screw and its bar, the bed on rails along X, a raised
    tympan frame on the left, ink balls and paper on the right."""
    for y in (-0.3, 0.3):
        k.add(box(0.14, 0.14, 1.74, 0.02), "wood_dk", T((0, y, 0.86)))
        k.add(box(0.5, 0.18, 0.12, 0.02), "wood_dk", T((0, y, 0.05)))
    k.add(box(0.28, 0.82, 0.22, 0.03), "wood_dk", T((0, 0, 1.56)))                                  # head
    k.add(box(0.34, 0.9, 0.07, 0.02), "wood_mid", T((0, 0, 1.75)))
    k.add(box(0.2, 0.7, 0.08, 0.02), "wood_mid", T((0, 0, 1.17)))                                  # till
    k.add(cyl(0.05, 0.05, 0.42, 10), "iron", T((0, 0, 1.24)), smooth=True)                          # screw
    for z in (1.08, 1.14, 1.2, 1.26, 1.32, 1.38):
        k.add(cyl(0.06, 0.06, 0.015, 10), "iron", T((0, 0, z)))
    k.add(box(0.48, 0.44, 0.05, 0.01), "iron", T((0, 0, 0.98)))                                     # platen
    k.add(cyl(0.02, 0.02, 0.62, 6), "iron", T((0.25, -0.18, 1.3), (90, 0, 55)), smooth=True)         # bar
    k.add(cyl(0.035, 0.03, 0.16, 8), "wood_mid", T((0.53, -0.37, 1.3), (90, 0, 55)), smooth=True)
    # rails, legs, bed with a forme of type
    for y in (-0.17, 0.17):
        k.add(box(1.86, 0.08, 0.08, 0.015), "wood", T((0, y, 0.72)))
    for x in (-0.88, 0.88):
        for y in (-0.17, 0.17):
            k.add(box(0.08, 0.08, 0.7, 0.01), "wood", T((x, y, 0.34)))
    k.add(box(0.6, 0.46, 0.08, 0.01), "iron", T((-0.05, 0, 0.8)))
    k.add(box(0.44, 0.34, 0.02), "ink", T((-0.05, 0, 0.845)))
    # tympan frame hinged at the bed's left edge, raised, with a sheet
    hinge = T((-0.35, 0, 0.85), (0, 55, 0))
    k.add(ribbon([(0, -0.22), (0.5, -0.22), (0.5, 0.22), (0, 0.22)], 0.04, 0.02, closed=True), "wood_lt", hinge @ T((-0.5, 0, 0)))
    k.add(box(0.44, 0.38, 0.006), "paper", hinge @ T((-0.25, 0, 0.012)))
    # ink table on the right: slab, two ink balls, a stack of paper
    k.add(box(0.4, 0.4, 0.04, 0.01), "wood_dk", T((0.62, 0.0, 0.8)))
    k.add(box(0.3, 0.3, 0.012), "ink", T((0.62, 0.0, 0.826)))
    for y in (-0.08, 0.1):
        k.add(sphere(0.07, 10, 5), "leather", T((0.62, y, 0.86), scale=(1, 1, 0.45)), smooth=True)
        k.add(cyl(0.018, 0.018, 0.14, 6), "wood_mid", T((0.62, y, 0.94)))
    for i in range(6):
        k.add(box(0.3, 0.22, 0.008), "paper" if i % 2 else "paper_dk", T((0.33, 0.08, 0.844 + 0.009 * i), (0, 0, i * 3)))


def build_type_case(k):
    """1 x 1 compositor's cabinet: drawers below (one pulled out), a sloping case of little compartments on top."""
    rng = random.Random(203)
    k.add(box(0.86, 0.56, 0.86, 0.02), "wood_mid", T((0, 0.08, 0.42)))
    for row, z in enumerate((0.16, 0.42, 0.68)):
        for x in (-0.21, 0.21):
            pull = 0.12 if (row, x) == (1, 0.21) else 0.0
            k.add(box(0.38, 0.03, 0.22, 0.01), "wood_lt", T((x, -0.205 - pull, z)))
            k.add(sphere(0.02, 6, 4), "brass", T((x, -0.23 - pull, z)), smooth=True)
            if pull:  # the open drawer's sides and a few sorts inside
                for sx in (-1, 1):
                    k.add(box(0.02, pull + 0.02, 0.16), "wood_lt", T((x + sx * 0.17, -0.2 - pull / 2, z)))
                for i in range(5):
                    k.add(box(0.025, 0.025, 0.04), "lead", T((x - 0.1 + 0.05 * i, -0.27, z + 0.01)))
    # sloping case: tray, rim and a grid of dividers
    case = T((0, 0.04, 0.9), (-16, 0, 0))
    k.add(box(0.84, 0.54, 0.05, 0.01), "wood_dk", case @ T((0, 0, 0.025)))
    for x in (-0.43, 0.43):
        k.add(box(0.03, 0.56, 0.08), "wood_mid", case @ T((x, 0, 0.06)))
    for y in (-0.27, 0.27):
        k.add(box(0.89, 0.03, 0.08), "wood_mid", case @ T((0, y, 0.06)))
    for i in range(1, 8):
        k.add(box(0.012, 0.52, 0.04), "wood_lt", case @ T((-0.42 + 0.105 * i, 0, 0.07)))
    for j in range(1, 5):
        k.add(box(0.83, 0.012, 0.04), "wood_lt", case @ T((0, -0.26 + 0.104 * j, 0.07)))
    for _ in range(22):
        i, j = rng.randrange(8), rng.randrange(5)
        k.add(box(0.03, 0.03, 0.02), "lead", case @ T((-0.367 + 0.105 * i + rng.uniform(-0.02, 0.02), -0.208 + 0.104 * j, 0.061)))
    k.add(box(0.86, 0.04, 0.3, 0.01), "wood_mid", T((0, 0.33, 1.09)))                                # back riser
    k.add(box(0.7, 0.18, 0.03), "wood_lt", T((0, 0.27, 1.25)))                                       # little shelf
    k.add(cyl(0.04, 0.04, 0.09, 8), "ink", T((-0.22, 0.27, 1.31)))
    k.add(box(0.12, 0.08, 0.05), "paper", T((0.2, 0.27, 1.29)))


def books(k, rng, x0, x1, y, z, depth, hmax, colours, lean=True):
    x = x0
    while x < x1 - 0.06:
        bw = rng.uniform(0.04, 0.08)
        bh = rng.uniform(0.6, 1.0) * hmax
        tilt = 12 if lean and rng.random() < 0.08 else 0
        k.add(box(bw - 0.006, depth * rng.uniform(0.82, 1.0), bh), rng.choice(colours),
              T((x + bw / 2, y + rng.uniform(-0.012, 0.012), z + bh / 2), (0, tilt, 0)))
        x += bw + (0.03 if tilt else 0.0)
        if rng.random() < 0.06 and x < x1 - 0.3:  # a short flat stack
            for i in range(3):
                k.add(box(0.2, depth * rng.uniform(0.78, 0.95), 0.035), rng.choice(colours), T((x + 0.11, y, z + 0.018 + 0.036 * i), (0, 0, rng.uniform(-6, 6))))
            x += 0.24


def build_bookcase(k):
    """2 x 1 tall bookcase against the north wall: plinth, cornice, three shelves of books, a cupboard below."""
    rng = random.Random(204)
    w, d, h, y = 1.9, 0.44, 2.05, 0.26
    cols = ["book_a", "book_b", "book_c", "book_d", "book_e", "book_f", "binder"]
    for sx in (-1, 1):
        k.add(box(0.07, d, h, 0.015), "wood_dk", T((sx * (w / 2 - 0.035), y, h / 2 - 0.01)))
    k.add(box(w - 0.1, 0.03, h - 0.1, 0.01), "wood_mid", T((0, y + d / 2 - 0.02, h / 2)))
    k.add(box(w + 0.04, d + 0.04, 0.1, 0.02), "wood_dk", T((0, y, 0.04)))                               # plinth
    k.add(box(w + 0.1, d + 0.08, 0.08, 0.02), "wood_dk", T((0, y - 0.02, h + 0.03)))                    # cornice
    k.add(box(w + 0.04, d + 0.04, 0.05, 0.015), "wood_mid", T((0, y - 0.01, h - 0.03)))
    # cupboard: two doors with raised panels and knobs
    k.add(box(w - 0.12, 0.03, 0.62, 0.01), "wood_mid", T((0, y - d / 2 + 0.02, 0.42)))
    for sx in (-1, 1):
        k.add(box(0.8, 0.03, 0.48, 0.01), "wood_lt", T((sx * 0.43, y - d / 2 + 0.0, 0.42)))
        k.add(sphere(0.022, 6, 4), "brass", T((sx * 0.06, y - d / 2 - 0.02, 0.44)), smooth=True)
    for z in (0.76, 1.2, 1.62):
        k.add(box(w - 0.12, d - 0.03, 0.035), "wood_mid", T((0, y - 0.015, z)))
        books(k, rng, -w / 2 + 0.08, w / 2 - 0.08, y - 0.03, z + 0.0175, 0.3, 0.38, cols)
    # on top: a plant and a box
    k.add(cyl(0.08, 0.06, 0.12, 8), "pot", T((-0.6, y, h + 0.13)), smooth=True)
    for i in range(4):
        a = i / 4 * math.tau
        k.add(jitter(ico(0.07, 1), rng, 0.01), "leaf" if i % 2 else "leaf_lt", T((-0.6 + 0.05 * math.cos(a), y + 0.05 * math.sin(a), h + 0.25), scale=(1, 1, 0.8)), smooth=True)
    k.add(box(0.3, 0.2, 0.12, 0.01), "card_b", T((0.5, y, h + 0.13), (0, 0, 8)))


def build_bed(k):
    """2 x 1 bed along X, head on +X: carved headboard, patchwork quilt, pillow."""
    for sx in (-1, 1):
        for sy in (-1, 1):
            k.add(box(0.08, 0.08, 0.32, 0.01), "wood_dk", T((sx * 0.9, sy * 0.4, 0.15)))
    for sy in (-1, 1):
        k.add(box(1.76, 0.05, 0.14, 0.015), "wood_mid", T((0, sy * 0.4, 0.3)))
    k.add(prism([(-0.45, 0), (0.45, 0), (0.45, 0.8), (0.3, 0.92), (0, 0.98), (-0.3, 0.92), (-0.45, 0.8)], 0.07, 0.02), "wood_mid",
          T((0.92, 0, 0.12), (0, 0, 90)))                                                               # headboard
    k.add(cyl(0.1, 0.1, 0.02, 6), "gold", T((0.884, 0, 0.82), (0, 90, 0)))
    k.add(prism([(-0.45, 0), (0.45, 0), (0.45, 0.5), (0, 0.58), (-0.45, 0.5)], 0.06, 0.02), "wood_mid", T((-0.92, 0, 0.12), (0, 0, 90)))
    k.add(box(1.78, 0.78, 0.18, 0.04), "sheet", T((0, 0, 0.44)))                                       # mattress
    k.add(box(1.2, 0.84, 0.05, 0.02), "quilt_c", T((-0.28, 0, 0.545)))                                  # quilt
    for sy in (-1, 1):
        k.add(box(1.2, 0.03, 0.2, 0.01), "quilt_c", T((-0.28, sy * 0.425, 0.46)))                      # quilt sides
    squares = ["quilt_a", "quilt_b", "quilt_d", "sheet"]
    for i in range(4):
        for j in range(3):
            k.add(box(0.26, 0.24, 0.006), squares[(i + j) % 4], T((-0.73 + 0.3 * i, -0.27 + 0.27 * j, 0.572)))
    k.add(sphere(0.2, 12, 6), "sheet", T((0.66, 0, 0.58), scale=(0.75, 1.6, 0.4)), smooth=True)       # pillow
    k.add(box(0.28, 0.78, 0.08, 0.03), "quilt_b", T((-0.73, 0, 0.6)))                                   # folded blanket
    contact(k, 0.95)
    for bm, *_ in k.parts[-2:]:
        bmesh.ops.scale(bm, vec=(1.0, 0.5, 1), verts=bm.verts)


def build_workbench(k):
    """2 x 1 workbench against the north wall: thick top, vise, a pegboard of tools, timber on the shelf."""
    rng = random.Random(206)
    for x in range(10):  # butcher-block top in strips
        k.add(box(1.9, 0.07, 0.08), rng.choice(["wood_lt", "wood", "wood_lt"]), T((0, -0.33 + 0.072 * x, 0.88)))
    for sx in (-1, 1):
        for sy in (-1, 1):
            k.add(box(0.1, 0.1, 0.84, 0.01), "wood_dk", T((sx * 0.86, sy * 0.28, 0.41)))
        k.add(box(0.08, 0.56, 0.08), "wood_dk", T((sx * 0.86, 0, 0.16)))
    k.add(box(1.7, 0.56, 0.04, 0.01), "wood_mid", T((0, 0, 0.22)))                                        # lower shelf
    for i in range(3):
        k.add(box(1.3, 0.12, 0.05, 0.01), rng.choice(["plank", "plank_lt", "plank_dk"]), T((-0.1, -0.15 + 0.14 * i, 0.265 + 0.002 * i), (0, 0, rng.uniform(-3, 3))))
    k.add(box(0.36, 0.2, 0.18, 0.015), "buoy_red", T((0.6, 0.12, 0.33)))                                  # toolbox
    k.add(box(0.2, 0.03, 0.03), "iron", T((0.6, 0.12, 0.44)))
    # vise on the front-left
    k.add(box(0.2, 0.12, 0.12, 0.01), "iron", T((-0.7, -0.37, 0.89)))
    k.add(box(0.2, 0.04, 0.1, 0.01), "iron", T((-0.7, -0.46, 0.88)))
    k.add(cyl(0.012, 0.012, 0.26, 6), "iron", T((-0.7, -0.5, 0.82), (0, 90, 0)))
    # pegboard and tools
    k.add(box(1.8, 0.03, 0.82, 0.01), "wood", T((0, 0.43, 1.33)))
    for sx in (-1, 1):
        k.add(box(0.06, 0.06, 0.9, 0.01), "wood_dk", T((sx * 0.9, 0.42, 1.37)))
    k.add(box(1.86, 0.06, 0.06, 0.01), "wood_dk", T((0, 0.42, 1.78)))
    M = T((0, 0.405, 0))
    k.add(box(0.03, 0.02, 0.28), "wood_mid", M @ T((-0.6, 0, 1.4)))                                      # hammer
    k.add(box(0.14, 0.03, 0.05), "iron", M @ T((-0.6, -0.005, 1.55)))
    k.add(prism([(-0.22, 0.0), (0.18, 0.04), (0.18, 0.14), (-0.22, 0.08)], 0.008), "tin", M @ T((-0.2, 0, 1.4)))   # saw blade
    k.add(box(0.12, 0.03, 0.12, 0.015), "wood_mid", M @ T((0.04, 0, 1.47)))
    k.add(box(0.025, 0.015, 0.24), "iron", M @ T((0.3, 0, 1.4), (0, 15, 0)))                              # wrench
    k.add(cyl(0.035, 0.035, 0.015, 8), "iron", M @ T((0.33, 0, 1.52), (90, 0, 0)))
    k.add(torus(0.08, 0.02, 10, 4), "rope", M @ T((0.6, -0.01, 1.4), (90, 0, 0)))                         # string
    for x in (0.0, 0.18):  # two chisels
        k.add(box(0.02, 0.015, 0.18), "tin", M @ T((-0.8 + x * 0.5, 0, 1.25)))
        k.add(cyl(0.017, 0.017, 0.1, 6), "wood_lt", M @ T((-0.8 + x * 0.5, -0.005, 1.38)))
    # on the top: a plane, offcuts, a jar of nails
    k.add(box(0.26, 0.08, 0.07, 0.015), "wood_mid", T((0.2, -0.1, 0.955), (0, 0, 10)))
    k.add(box(0.06, 0.03, 0.07, 0.01), "wood_dk", T((0.15, -0.09, 1.0), (0, 0, 10)))
    for i in range(3):
        k.add(box(rng.uniform(0.15, 0.3), 0.08, 0.04), rng.choice(["plank", "plank_lt"]), T((0.62, -0.12 + 0.06 * i, 0.94 + 0.04 * (i % 2)), (0, 0, rng.uniform(-20, 20))))
    k.add(cyl(0.05, 0.05, 0.1, 10), "jar_glass", T((-0.3, 0.2, 0.97)), smooth=True)
    k.add(cyl(0.045, 0.045, 0.05, 10), "iron", T((-0.3, 0.2, 0.95)))


def build_light_box(k):
    """2 x 1 deck doctor's table: a glowing light panel with cards laid out on it, a magnifier on an arm,
    a notebook and a deck box."""
    rng = random.Random(207)
    k.add(box(1.82, 0.82, 0.06, 0.02), "wood_mid", T((0, 0, 0.76)))
    for sx in (-1, 1):
        for sy in (-1, 1):
            k.add(box(0.07, 0.07, 0.75, 0.015), "wood_dk", T((sx * 0.84, sy * 0.34, 0.36)))
    k.add(box(1.7, 0.04, 0.12, 0.01), "wood_dk", T((0, -0.36, 0.68)))                                   # apron
    k.add(box(0.5, 0.03, 0.08, 0.01), "wood_lt", T((-0.4, -0.385, 0.68)))                               # drawer
    k.add(sphere(0.018, 6, 4), "brass", T((-0.4, -0.41, 0.68)), smooth=True)
    # the light box: frame and a glowing panel 4 mm proud of it
    k.add(box(1.16, 0.62, 0.06, 0.012), "wood_dk", T((-0.1, 0.0, 0.82)))
    k.add(box(1.04, 0.5, 0.02), "lightbox_glow", T((-0.1, 0.0, 0.844)))
    cards = ["card_face", "card_a", "card_b", "card_c", "card_d"]
    for i in range(5):
        for j in range(2):
            if (i, j) == (4, 0):
                continue
            k.add(box(0.13, 0.18, 0.004), rng.choice(cards), T((-0.52 + 0.21 * i, -0.11 + 0.23 * j, 0.857), (0, 0, rng.uniform(-4, 4))))
    # magnifier on an arm clamped at the back right
    k.add(box(0.08, 0.08, 0.06, 0.01), "iron", T((0.68, 0.3, 0.82)))
    beam(k, (0.68, 0.3, 0.84), (0.6, 0.18, 1.25), 0.03, 0.03, "iron")
    beam(k, (0.6, 0.18, 1.25), (0.38, -0.02, 1.12), 0.03, 0.03, "iron")
    k.add(N.ring(0.11, 0.09, 16), "brass", T((0.3, -0.06, 1.06), (-62, 0, 0)))
    k.add(cyl(0.092, 0.092, 0.008, 16), "glass_in", T((0.3, -0.06, 1.06), (28, 0, 0)))
    # notebook, pencil, deck box
    k.add(box(0.24, 0.3, 0.02, 0.005), "book_b", T((0.68, -0.18, 0.8), (0, 0, -8)))
    k.add(box(0.22, 0.28, 0.006), "paper", T((0.68, -0.18, 0.813), (0, 0, -8)))
    k.add(cyl(0.008, 0.008, 0.18, 6), "gold", T((0.6, -0.2, 0.822), (0, 90, 30)))
    k.add(box(0.1, 0.08, 0.14, 0.01), "binder", T((-0.78, 0.2, 0.86)))


def build_net_locker(k):
    """1 x 1 weathered locker: slatted door, a net hung on its side, floats, a coil of rope, an oilskin."""
    rng = random.Random(208)
    k.add(box(0.78, 0.5, 1.76, 0.02), "plank_grey", T((0, 0.18, 0.87)))
    k.add(box(0.84, 0.56, 0.06, 0.015), "plank_grey_dk", T((0, 0.18, 1.77)))
    k.add(box(0.82, 0.54, 0.08, 0.015), "plank_grey_dk", T((0, 0.18, 0.03)))
    k.add(box(0.66, 0.03, 1.56, 0.01), "plank_grey_dk", T((0, -0.075, 0.9)))                            # door
    for i in range(12):
        k.add(box(0.6, 0.04, 0.05), rng.choice(["plank_grey_lt", "plank_grey"]), T((0, -0.095, 0.24 + 0.115 * i), (-20, 0, 0)))
    for z in (0.35, 1.45):
        k.add(box(0.1, 0.02, 0.04), "iron", T((-0.3, -0.1, z)))
    k.add(box(0.03, 0.03, 0.12), "iron", T((0.27, -0.11, 0.95)))                                        # latch
    # net on the right side: a lattice with a sagging hem, floats along the top
    for i in range(6):
        y = -0.03 + 0.08 * i
        k.add(box(0.012, 0.012, 1.1 - 0.06 * (i % 2)), "net", T((0.405, y, 1.15 - 0.03 * (i % 2))))
    for j in range(8):
        k.add(box(0.012, 0.44, 0.012), "net", T((0.408, 0.17, 0.66 + 0.14 * j)))
    for i in range(3):
        k.add(cyl(0.035, 0.035, 0.06, 8), "cork" if i % 2 else "buoy_red", T((0.43, 0.0 + 0.17 * i, 1.68), (90, 0, 0)), smooth=True)
    # rope coil on top, oilskin on a hook at the left
    k.add(torus(0.16, 0.035, 12, 5), "rope", T((0.0, 0.18, 1.84)))
    k.add(torus(0.12, 0.03, 12, 5), "rope", T((0.02, 0.17, 1.9)))
    k.add(cyl(0.012, 0.012, 0.08, 6), "iron", T((-0.43, 0.0, 1.5), (0, 90, 0)))
    k.add(cyl(0.12, 0.17, 0.75, 8), "oilskin", T((-0.47, 0.0, 1.1), scale=(0.35, 1, 1)), smooth=True)
    contact(k, 0.5)


INTERIOR_V2 = {"oven": build_oven, "printing_press": build_printing_press, "type_case": build_type_case,
               "bookcase": build_bookcase, "bed": build_bed, "workbench": build_workbench,
               "light_box": build_light_box, "net_locker": build_net_locker}


# ---------------------------------------------------------------- outdoor (assets/)


def swap_symbol(k, M):
    """Two arrows chasing each other round a circle (the swap shop), raised from the XY plane."""
    for a0, m in ((25, "swap_teal"), (205, "gold")):
        pts = arc(0, 0, 0.13, a0, a0 + 130, 10)
        k.add(ribbon(pts, 0.04, 0.012), m, M)
        ex, ey = pts[-1]
        a = math.radians(a0 + 130)
        tx, ty = -math.sin(a), math.cos(a)          # tangent at the end
        nx, ny = math.cos(a), math.sin(a)
        head = [(ex + nx * 0.05, ey + ny * 0.05), (ex - nx * 0.05, ey - ny * 0.05), (ex + tx * 0.07, ey + ty * 0.07)]
        bm = bmesh.new()
        lo = [bm.verts.new((x, y, 0)) for x, y in head]
        hi = [bm.verts.new((x, y, 0.012)) for x, y in head]
        bm.faces.new(lo[::-1])
        bm.faces.new(hi)
        for i in range(3):
            j = (i + 1) % 3
            bm.faces.new((lo[i], lo[j], hi[j], hi[i]))
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
        k.add(bm, m, M)


def build_hanging_sign(k):
    """1 x 1 swap-shop sign: post with a braced arm, a board on two chains, the swap arrows on both faces."""
    px = -0.3
    k.add(box(0.3, 0.3, 0.12, 0.03), "stone_dk", T((px, 0.1, 0.04)))
    k.add(box(0.12, 0.12, 2.38, 0.02), "wood_dk", T((px, 0.1, 1.17)))
    k.add(cyl(0.1, 0.0, 0.12, 4), "wood_dk", T((px, 0.1, 2.42), (0, 0, 45)))
    k.add(box(0.8, 0.08, 0.08, 0.015), "wood_dk", T((px + 0.42, 0.1, 2.2)))
    beam(k, (px, 0.1, 1.86), (px + 0.36, 0.1, 2.18), 0.06, 0.06, "wood_dk")
    k.add(cyl(0.02, 0.0, 0.06, 6), "iron", T((px + 0.82, 0.1, 2.2), (0, 90, 0)))
    bx, bz = 0.15, 1.72
    for cx in (-0.2, 0.2):  # chains: three links each
        for i in range(3):
            k.add(N.ring(0.025, 0.014, 6), "iron", T((bx + cx, 0.1, 2.12 - 0.05 * i), (0, 0, 90 * (i % 2))))
    k.add(box(0.6, 0.05, 0.44, 0.015), "board", T((bx, 0.1, bz)))
    k.add(ribbon([(-0.29, -0.21), (0.29, -0.21), (0.29, 0.21), (-0.29, 0.21)], 0.03, 0.012, closed=True), "wood_dk", T((bx, 0.075, bz), FRONT))
    k.add(ribbon([(-0.29, -0.21), (0.29, -0.21), (0.29, 0.21), (-0.29, 0.21)], 0.03, 0.012, closed=True), "wood_dk", T((bx, 0.125, bz), BACK))
    swap_symbol(k, T((bx, 0.075, bz), FRONT))
    swap_symbol(k, T((bx, 0.125, bz), BACK))
    contact(k, 0.3)
    for bm, *_ in k.parts[-2:]:
        bmesh.ops.translate(bm, vec=(px, 0.1, 0), verts=bm.verts)


def build_curio_cabinet(k):
    """2 x 1 stall variant for the curio collector: carved plum counter, a glass vitrine of curios on it,
    drawer towers at both ends, a scalloped canopy and a small hanging lamp."""
    rng = random.Random(210)
    k.add(box(1.8, 0.5, 0.82, 0.02), "plum", T((0, -0.12, 0.39)))
    k.add(box(1.86, 0.56, 0.05, 0.015), "wood_dk", T((0, -0.12, 0.82)))
    for x in (-0.55, 0.0, 0.55):  # carved front panels with a little gold hex
        k.add(box(0.46, 0.02, 0.5, 0.01), "plum_dk", T((x, -0.375, 0.42)))
        k.add(cyl(0.05, 0.05, 0.012, 6), "gold", T((x, -0.39, 0.42), (90, 30, 0)))
    # vitrine on the counter: wooden base, velvet steps, curios, a glass hood with corner posts
    k.add(box(1.0, 0.36, 0.04, 0.01), "wood_dk", T((0, -0.14, 0.865)))
    for i, (y, z) in enumerate(((-0.22, 0.9), (-0.12, 0.94), (-0.02, 0.98))):
        k.add(box(0.94, 0.11, z - 0.88, 0.005), "velvet", T((0, y + 0.0, (z + 0.88) / 2 + 0.002)))
    curios = [(sphere(0.03, 8, 5), "bottle_glass"), (cyl(0.035, 0.035, 0.012, 6), "hex"), (cyl(0.02, 0.025, 0.05, 8), "brass"),
              (box(0.02, 0.012, 0.09), "feather"), (sphere(0.035, 8, 5), "fossil_stone"), (box(0.03, 0.03, 0.05), "lead")]
    for i, (bm, m) in enumerate(curios):
        x = -0.36 + 0.145 * i
        y, z = ((-0.22, 0.9), (-0.12, 0.94), (-0.02, 0.98))[i % 3]
        k.add(bm, m, T((x, y, z + 0.035), (90 if m == "hex" else 0, 0, rng.uniform(-20, 20))), smooth=m in ("bottle_glass", "fossil_stone"))
    k.add(box(0.98, 0.34, 0.26, 0.01), "glass_case", T((0, -0.14, 1.02)))
    for sx in (-1, 1):
        for sy in (-1, 1):
            k.add(box(0.03, 0.03, 0.28), "brass", T((sx * 0.49, -0.14 + sy * 0.17, 1.02)))
    k.add(box(1.02, 0.38, 0.03), "brass", T((0, -0.14, 1.165)))
    # drawer towers at both ends, then posts to the canopy
    for sx in (-1, 1):
        x = sx * 0.78
        k.add(box(0.3, 0.42, 0.78, 0.015), "plum", T((x, 0.02, 1.23)))
        for j in range(4):
            k.add(box(0.24, 0.02, 0.15, 0.008), "plum_dk", T((x, -0.2, 0.93 + 0.19 * j)))
            k.add(sphere(0.016, 6, 4), "brass", T((x, -0.215, 0.93 + 0.19 * j)), smooth=True)
        k.add(box(0.34, 0.46, 0.04, 0.01), "wood_dk", T((x, 0.02, 1.64)))
        for sy, h in ((-1, 1.9), (1, 2.15)):
            k.add(box(0.07, 0.07, h + 0.02, 0.01), "wood_dk", T((sx * 0.9, sy * 0.4, h / 2 - 0.01)))
    # canopy: plum and cream panels sloping to the front, scalloped valance, a lamp on a bracket
    n = 8
    slope = math.degrees(math.atan2(0.28, 1.0))
    for i in range(n):
        x = -1.0 + (i + 0.5) * 2.0 / n
        m = ("velvet", "cream")[i % 2]
        k.add(box(2.0 / n - 0.002, 1.1, 0.035), m, T((x, -0.07, 2.08), (slope, 0, 0)))
        k.add(prism(arc(0, 0, 0.125, 180, 360, 6), 0.02), m, T((x, -0.625, 1.92)))
    k.add(box(2.04, 0.04, 0.04), "gold", T((0, -0.63, 1.935)))
    k.add(box(0.03, 0.03, 0.2), "iron", T((0, -0.58, 1.82)))
    k.add(box(0.12, 0.12, 0.14), "lamp_glow", T((0, -0.58, 1.65)))
    k.add(cyl(0.1, 0.0, 0.07, 4), "iron", T((0, -0.58, 1.75), (0, 0, 45)))
    k.add(box(0.14, 0.14, 0.02), "iron", T((0, -0.58, 1.575)))
    shadow(k, 1.0, 1.0, 0.6)


# ---------------------------------------------------------------- curios (assets/, centred on the origin)


def build_curio(k):
    """The shared fallback: a drawstring pouch."""
    k.add(sphere(0.1, 12, 7), "pouch", T((0, 0, -0.02), scale=(1, 1, 0.85)), smooth=True)
    k.add(cyl(0.035, 0.05, 0.05, 10), "pouch", T((0, 0, 0.07)), smooth=True)
    k.add(cyl(0.06, 0.035, 0.04, 10), "pouch_dk", T((0, 0, 0.11)), smooth=True)
    k.add(torus(0.04, 0.008, 10, 4), "gold", T((0, 0, 0.07)))
    k.add(cyl(0.006, 0.006, 0.08, 4), "gold", T((0.05, -0.02, 0.03), (20, 25, 0)))
    k.add(cyl(0.035, 0.035, 0.006, 6), "gold", T((0, -0.095, -0.01), (90, 30, 0)))


def build_brass_thimble(k):
    k.add(cyl(0.06, 0.05, 0.12, 16), "brass", T((0, 0, -0.01)), smooth=True)
    k.add(half_sphere(0.05, 16, 6), "brass", T((0, 0, 0.05)), smooth=True)
    k.add(cyl(0.064, 0.064, 0.016, 16), "gold", T((0, 0, -0.067)), smooth=True)
    for z, r in ((-0.03, 0.058), (0.02, 0.053)):
        k.add(cyl(r, r, 0.006, 16), "roof_trim", T((0, 0, z)))
    for i in range(10):  # dimples on the crown, along its normals
        a = i / 10 * math.tau
        n = Vector((0.03 * math.cos(a), 0.03 * math.sin(a), 0.04)).normalized()
        q = n.to_track_quat("Z", "Y").to_matrix().to_4x4()
        k.add(cyl(0.007, 0.007, 0.004, 5), "roof_trim", T(Vector((0, 0, 0.05)) + n * 0.05) @ q)


def build_heron_feather(k):
    """Upright grey feather with a dark tip, slightly curved."""
    def vane(side):
        pts = [(0.0, -0.16)] + [(side * (0.035 + 0.02 * math.sin(t / 10 * math.pi)), -0.12 + 0.034 * t) for t in range(11)] + [(0.0, 0.25)]
        return prism([(x, y) for x, y in pts], 0.006)
    for side in (1, -1):
        k.add(vane(side), "feather")
        k.add(prism([(0, 0.12), (side * 0.05, 0.13), (side * 0.035, 0.2), (0, 0.25)], 0.011), "feather_dk", T((0, 0, 0)))
    k.add(cyl(0.006, 0.003, 0.48, 6), "quill_shaft", T((0, -0.005, 0.03)))
    k.add(cyl(0.008, 0.006, 0.08, 6), "quill_shaft", T((0, -0.005, -0.2)))


def build_message_bottle(k):
    k.add(cyl(0.07, 0.07, 0.2, 14), "bottle_glass", T((0, 0, -0.04)), smooth=True)
    k.add(cyl(0.07, 0.025, 0.08, 14), "bottle_glass", T((0, 0, 0.1)), smooth=True)
    k.add(cyl(0.025, 0.025, 0.07, 10), "bottle_glass", T((0, 0, 0.175)), smooth=True)
    k.add(cyl(0.022, 0.026, 0.05, 10), "cork", T((0, 0, 0.225)), smooth=True)
    k.add(cyl(0.035, 0.035, 0.16, 10), "paper", T((0, 0, -0.03)), smooth=True)
    k.add(cyl(0.037, 0.037, 0.015, 10), "wax", T((0, 0, -0.03)))


def build_fossil_fern(k):
    """An upright stone plaque, a fern frond in relief on the front."""
    rng = random.Random(214)
    k.add(jitter(cyl(0.16, 0.16, 0.05, 12, 0.012), rng, 0.012), "fossil_stone", T((0, 0, 0), (90, 0, 0), (1.1, 1.3, 1)), smooth=True)
    stem = [(0.02 * math.sin(t * 0.4), -0.15 + 0.03 * t) for t in range(11)]
    k.add(ribbon(stem, 0.012, 0.012), "fossil_dk", T((0, -0.028, 0), FRONT))
    for t in range(1, 10):
        x, y = stem[t]
        L = 0.075 * (1 - abs(t - 4) / 7)
        for side in (1, -1):
            leaf = prism([(0, -0.008), (side * L, 0.012), (side * L * 1.05, 0.02), (0, 0.01)], 0.01)
            k.add(leaf, "fossil_dk", T((x, -0.034, y), (0, side * -10, 0)))


def build_tin_hex(k):
    """A painted tin hexagon token: the Sixfold hex, with six coloured dots at its corners."""
    R = T((0, 0, 0), (90, 30, 0))
    k.add(cyl(0.15, 0.15, 0.02, 6, 0.004), "tin", R)
    k.add(cyl(0.125, 0.125, 0.026, 6), "hex", R)
    k.add(cyl(0.07, 0.07, 0.032, 6), "hex_in", R)
    for i, m in enumerate(("seal_grass", "seal_fire", "seal_water", "seal_lightning", "seal_fighting", "seal_psychic")):
        a = math.radians(90 + 60 * i)
        for sy in (-1, 1):
            k.add(cyl(0.015, 0.015, 0.006, 8), m, T((0.1 * math.cos(a), sy * 0.015, 0.1 * math.sin(a)), (90, 0, 0)))


def build_lead_type(k):
    """A lead type sort, tilted toward the viewer, a mirrored S in relief on its face."""
    M = T((0, 0, 0), (40, 0, 0))
    k.add(box(0.13, 0.13, 0.26, 0.004), "lead", M @ T((0, 0, -0.03)))
    k.add(box(0.138, 0.014, 0.014), "iron", M @ T((0, -0.06, -0.1)))                                   # the nick
    k.add(box(0.12, 0.12, 0.02, 0.004), "lead", M @ T((0, 0, 0.108)))                                  # shoulder
    r = 0.024
    s = arc(0, r, r, 40, 270, 8) + arc(0, -r, r, 90, -140, 8)[1:]
    s = [(-x, y) for x, y in s]                                                                         # type reads mirrored
    k.add(ribbon(s, 0.018, 0.022), "tin", M @ T((0, 0, 0.116), (0, 0, 0), (1.05, 1.05, 1)))


CURIOS = {"curio": build_curio, "brass_thimble": build_brass_thimble, "heron_feather": build_heron_feather,
          "message_bottle": build_message_bottle, "fossil_fern": build_fossil_fern, "tin_hex": build_tin_hex,
          "lead_type": build_lead_type}


# ---------------------------------------------------------------- 1.3 shared


def build_waymark(k):
    """1 x 1 hexagonal stone post with a crystal held in brass prongs (the game tints the crystal per region)."""
    H = (0, 0, 30)
    k.add(cyl(0.42, 0.44, 0.14, 6, 0.02), "fstone_dk", T((0, 0, 0.05), H))
    k.add(cyl(0.32, 0.34, 0.12, 6, 0.02), "fstone", T((0, 0, 0.16), H))
    k.add(cyl(0.19, 0.16, 1.0, 6, 0.015), "fstone", T((0, 0, 0.71), H))
    k.add(cyl(0.2, 0.2, 0.06, 6, 0.01), "fstone_dk", T((0, 0, 0.98), H))
    k.add(cyl(0.22, 0.24, 0.1, 6, 0.015), "fcap", T((0, 0, 1.24), H))
    for th, m in ((270, "hex"), (90, "hex")):  # Sixfold hex inlay, front and back
        n = Vector((math.cos(math.radians(th)), math.sin(math.radians(th)), 0))
        k.add(cyl(0.08, 0.08, 0.03, 6), m, T(n * 0.17 + Vector((0, 0, 0.62)), (90, 30, th + 90)))
        k.add(cyl(0.045, 0.045, 0.03, 6), "hex_in", T(n * 0.18 + Vector((0, 0, 0.62)), (90, 30, th + 90)))
    for i in range(3):  # brass prongs
        a = math.radians(90 + 120 * i)
        c, s_ = math.cos(a), math.sin(a)
        beam(k, (0.09 * c, 0.09 * s_, 1.27), (0.125 * c, 0.125 * s_, 1.52), 0.03, 0.03, "brass")
    k.add(cyl(0.11, 0.11, 0.26, 6), "crystal_glow", T((0, 0, 1.56), H))
    k.add(cyl(0.11, 0.0, 0.18, 6), "crystal_glow", T((0, 0, 1.78), H))
    k.add(cyl(0.0, 0.11, 0.12, 6), "crystal_glow", T((0, 0, 1.37), H))
    contact(k, 0.48)


SEAL_ORDER = ("seal_fire", "seal_water", "seal_lightning", "seal_psychic", "seal_fighting", "seal_grass")


def build_seal_stone(k):
    """2 x 2 seal stone: a stepped hex base and a tall hexagonal stone, one coloured face per colour
    (front = fire, then counter-clockwise from above: water, lightning, psychic, fighting, grass), a blank top."""
    H = (0, 0, 30)    # flat faces toward the front
    k.add(cyl(0.98, 0.98, 0.14, 6, 0.03), "fstone_dk", T((0, 0, 0.05), H))
    k.add(cyl(0.86, 0.86, 0.12, 6, 0.025), "fstone", T((0, 0, 0.17), H))
    k.add(cyl(0.7, 0.62, 1.5, 6, 0.03), "seal_body", T((0, 0, 0.98), H))
    k.add(cyl(0.64, 0.6, 0.08, 6, 0.02), "seal_dk", T((0, 0, 1.76), H))
    k.add(cyl(0.58, 0.52, 0.1, 6, 0.03), "seal_top", T((0, 0, 1.84), H))                             # the blank seventh face
    apo_bot, apo_top = 0.7 * math.cos(math.radians(30)), 0.62 * math.cos(math.radians(30))
    lean = math.degrees(math.atan2(apo_bot - apo_top, 1.5))
    for i, m in enumerate(SEAL_ORDER):
        th = 270 + 60 * i  # 270 = -Y = front
        n = Vector((math.cos(math.radians(th)), math.sin(math.radians(th)), 0))
        z = 1.0
        apo = apo_bot + (apo_top - apo_bot) * (z - 0.23) / 1.5
        M = T(n * (apo + 0.008) + Vector((0, 0, z)), (-lean, 0, th + 90))   # local -Y = outward
        k.add(prism([(-0.2, -0.45), (0.2, -0.45), (0.2, 0.32), (0, 0.45), (-0.2, 0.32)], 0.04, 0.008), m, M)
        k.add(cyl(0.09, 0.09, 0.03, 6), "seal_top", M @ T((0, -0.025, 0.12), (90, 30, 0)))          # a small pale hex on each
    contact(k, 1.0)


def build_cave_mouth(k):
    """2 x 1 cave opening at the foot of a cliff (the cliff stands on the row behind): sandstone jambs and
    lintel, a timber frame inside, a dark recess; the opening is the door tile's way in."""
    rng = random.Random(220)
    layers = ((-0.02, 0.5, "sand_d"), (0.5, 1.05, "sand_b"), (1.05, 1.6, "sand_a"), (1.6, 2.05, "sand_c"))
    for sx in (-1, 1):  # jambs
        for z0, z1, m in layers:
            k.add(jitter(box(0.48, 0.9, z1 - z0, 0.04), rng, 0.03), m, T((sx * 0.74, 0.05, (z0 + z1) / 2)))
    for z0, z1, m in layers[2:]:  # lintel across the top
        k.add(jitter(box(1.1, 0.9, z1 - z0, 0.04), rng, 0.03), m, T((0, 0.05, (z0 + z1) / 2)))
    k.add(jitter(box(2.0, 0.92, 0.06, 0.02), rng, 0.01), "cliff_top", T((0, 0.05, 2.05)))
    for x, z, r in ((-0.85, 0.2, 0.16), (0.9, 0.15, 0.12), (0.62, 0.08, 0.08)):
        k.add(jitter(ico(r, 1), rng, r * 0.2), "sand_b", T((x, -0.42, z), scale=(1, 0.8, 0.7)))
    # dark recess: back wall, sides, ceiling, floor
    k.add(box(1.04, 0.04, 1.2), "cave_dark", T((0, 0.44, 0.58)))
    for sx in (-1, 1):
        k.add(box(0.04, 0.84, 1.2), "cave_dark", T((sx * 0.52, 0.02, 0.58)))
    k.add(box(1.04, 0.84, 0.04), "cave_dark", T((0, 0.02, 1.16)))
    k.add(box(1.02, 0.86, 0.02), "cave_floor", T((0, 0.02, 0.0)))
    # timber frame: posts and a cap beam just inside the mouth
    for sx in (-1, 1):
        k.add(box(0.1, 0.1, 1.12, 0.015), "wood_dk", T((sx * 0.43, -0.3, 0.55)))
    k.add(box(1.0, 0.12, 0.12, 0.02), "wood_dk", T((0, -0.3, 1.1)))
    for i in range(3):
        k.add(jitter(ico(0.03, 1), rng, 0.006), "pebble", T((-0.3 + 0.25 * i, -0.36 + 0.1 * i, 0.012), scale=(1, 0.8, 0.5)))


def build_climb_wall(k):
    """1 x 1 stackable climb: two rough sandstone steps (tops 0.5 m at the front, 1.0 m at the back), handholds
    and pegs on the risers, vines hanging down. Floor 0.5 m, climbs toward -Z like stairs."""
    rng = random.Random(221)
    for y0, y1, z1, mats in ((-0.5, 0.0, 0.5, ("sand_d", "sand_b")), (0.0, 0.5, 1.0, ("sand_d", "sand_b", "sand_a"))):
        zs = [(-0.02, 0.3), (0.3, 0.44)] if z1 == 0.5 else [(-0.02, 0.3), (0.3, 0.62), (0.62, 0.94)]
        for (z0, za), m in zip(zs, mats):
            s = A.stratum(rng, z0, za, 0.015)
            bmesh.ops.scale(s, vec=(1, 0.5, 1), verts=s.verts)
            k.add(s, m, T((0, (y0 + y1) / 2, 0)))
        top = A.stratum(rng, z1 - 0.06 if z1 == 1.0 else 0.44, z1, 0.0, inward_top=True)
        bmesh.ops.scale(top, vec=(1, 0.5, 1), verts=top.verts)
        k.add(top, "cliff_top", T((0, (y0 + y1) / 2, 0)))
    for y, z0, z1 in ((-0.5, 0.05, 0.45), (0.0, 0.55, 0.95)):  # handholds and pegs on each riser
        for i in range(4):
            x = -0.3 + 0.2 * i + rng.uniform(-0.05, 0.05)
            z = z0 + (z1 - z0) * ((i * 0.37) % 1)
            if i % 2:
                k.add(cyl(0.022, 0.018, 0.12, 6), "wood_dk", T((x, y - 0.04, z), (90, 0, 0)))
            else:
                k.add(jitter(ico(0.045, 1), rng, 0.01), "sand_c", T((x, y - 0.02, z), scale=(1.2, 0.7, 0.6)))
    for x, y, zt, L in ((-0.36, 0.0, 1.0, 0.55), (0.22, 0.0, 1.0, 0.45), (0.38, -0.5, 0.5, 0.42), (-0.12, -0.5, 0.5, 0.3)):
        pts = [(x + 0.03 * math.sin(i * 1.3), zt + 0.005 - L * i / 6) for i in range(7)]
        k.add(ribbon(pts, 0.02, 0.012), "vine", T((0, y - 0.012, 0), FRONT))
        for i in range(1, 7, 2):
            px, pz = pts[i]
            k.add(sphere(0.035, 6, 4), "vine_lt" if i % 4 == 1 else "leaf", T((px + 0.02, y - 0.03, pz), (0, rng.uniform(-40, 40), 0), (1, 0.4, 0.6)), smooth=True)
    k.add(torus(0.08, 0.015, 10, 4), "rope", T((0.3, 0.2, 1.01)))
    contact(k, 0.5)


def build_inn_sign(k):
    """1 x 1 inn sign: two posts and a crossbar, a board with a crescent moon and stars hung on chains, a lantern."""
    for sx in (-1, 1):
        k.add(box(0.26, 0.26, 0.1, 0.03), "stone_dk", T((sx * 0.38, 0, 0.03)))
        k.add(box(0.1, 0.1, 2.26, 0.02), "wood_dk", T((sx * 0.38, 0, 1.11)))
        k.add(cyl(0.085, 0.0, 0.1, 4), "wood_dk", T((sx * 0.38, 0, 2.29), (0, 0, 45)))
    k.add(box(0.98, 0.1, 0.1, 0.02), "wood_dk", T((0, 0, 2.1)))
    for sx in (-1, 1):
        beam(k, (sx * 0.38, 0, 1.93), (sx * 0.25, 0, 2.06), 0.05, 0.04, "wood_dk")
        for i in range(3):
            k.add(N.ring(0.022, 0.012, 6), "iron", T((sx * 0.22, 0, 2.02 - 0.042 * i), (0, 0, 90 * (i % 2))))
    bz = 1.7
    k.add(box(0.6, 0.05, 0.42, 0.015), "inn_board", T((0, 0, bz)))
    frame = [(-0.29, -0.2), (0.29, -0.2), (0.29, 0.2), (-0.29, 0.2)]
    outer = arc(-0.06, 0, 0.13, 70, 290, 12)               # crescent: outer arc, then the inner arc back
    ix = outer[0][0] + 0.05
    r_in = math.hypot(outer[0][0] - ix, outer[0][1])
    a_in = math.degrees(math.atan2(outer[0][1], outer[0][0] - ix))
    moon = outer + arc(ix, 0, r_in, 360 - a_in, a_in, 10)[1:-1]
    stars = ((0.12, 0.08, 0.04), (0.19, -0.05, 0.03), (0.08, -0.11, 0.024))
    for side in (-1, 1):   # front (-Y) and back (+Y) faces, the back mirrored so it reads from behind
        y = side * 0.025
        k.add(ribbon(frame, 0.025, 0.01, closed=True), "gold", T((0, y, bz), FRONT if side < 0 else BACK))
        k.add(prism([(-side * x, z) for x, z in moon], 0.014), "gold", T((0, y + side * 0.005, bz)))
        for cx, cz, r in stars:
            star = [(-side * (cx + r * (1 if i % 2 == 0 else 0.42) * math.cos(math.radians(90 + 45 * i))),
                     cz + r * (1 if i % 2 == 0 else 0.42) * math.sin(math.radians(90 + 45 * i))) for i in range(8)]
            k.add(prism(star, 0.014), "hex_in", T((0, y + side * 0.005, bz)))
    # lantern hanging from the right end of the crossbar
    k.add(box(0.025, 0.025, 0.12), "iron", T((0.47, 0, 1.99)))
    k.add(box(0.13, 0.13, 0.16), "lamp_glow", T((0.47, 0, 1.84)))
    for sx in (-1, 1):
        for sy in (-1, 1):
            k.add(box(0.02, 0.02, 0.18), "iron", T((0.47 + sx * 0.07, sy * 0.07, 1.84)))
    k.add(cyl(0.11, 0.0, 0.08, 4), "iron", T((0.47, 0, 1.96), (0, 0, 45)))
    k.add(box(0.16, 0.16, 0.02), "iron", T((0.47, 0, 1.755)))
    shadow(k, 0.55, 1.0, 0.45)


SHARED = {"waymark": build_waymark, "seal_stone": build_seal_stone, "cave_mouth": build_cave_mouth,
          "climb_wall": build_climb_wall, "inn_sign": build_inn_sign}

OUTDOOR = {"hanging_sign": build_hanging_sign, "curio_cabinet": build_curio_cabinet}
OUTDOOR.update(CURIOS)
OUTDOOR.update(SHARED)
PROPS = dict(INTERIOR_V2)
PROPS.update(OUTDOOR)
B.INTERIOR.update(INTERIOR_V2)   # export() files these under assets/interior/
B.ALPHA.update({"jar_glass": 0.4, "bottle_glass": 0.45})


# ---------------------------------------------------------------- people (assets/npc/, the hero's rig)


def elder(k):
    """Stooped older adult: white bun, gold glasses, knitted shawl, long dress, a cane in the right hand."""
    N.body(k, {"shoe": "el_shoe", "pants": "el_dress", "top": "el_cardigan", "sleeve": "el_cardigan"})
    N.hair_cap(k, "el_hair", tilt=-30, cut=0.0)
    k.add(sphere(0.11, 10, 6), "el_hair", T((0, 0.17, 1.0)), smooth=True, bone="Head")                  # bun
    k.add(cyl(0.05, 0.05, 0.03, 8), "el_shawl", T((0, 0.15, 0.95), (60, 0, 0)), smooth=True, bone="Head")
    for s in (1, -1):
        k.add(N.ring(0.06, 0.048), "el_frame", T((s * 0.085, -0.228, 0.82)), bone="Head")
        k.add(box(0.012, 0.16, 0.012), "el_frame", T((s * 0.15, -0.15, 0.83)), bone="Head")
    k.add(box(0.05, 0.012, 0.012), "el_frame", T((0, -0.229, 0.83)), bone="Head")
    N.skirt(k, "el_dress", 0.17, 0.235, 0.03, 0.4, 14)
    k.add(cyl(0.18, 0.2, 0.1, 14, 0.02), "el_shawl", T((0, 0.0, 0.6), scale=(1.05, 0.85, 1)), smooth=True, bone="Torso")
    k.add(prism([(-0.14, 0), (0.14, 0), (0, -0.18)], 0.03, 0.008), "el_shawl", T((0, 0.135, 0.58), (0, 0, 0)), bone="Torso")
    for z in (0.44, 0.5):
        k.add(sphere(0.013, 6, 4), "el_shawl", T((0, -0.13, z)), smooth=True, bone="Torso")
    hx = -0.235
    k.add(cyl(0.016, 0.016, 0.36, 6), "el_cane", T((hx, -0.07, 0.17)), bone="Arm.R")
    k.add(cyl(0.016, 0.016, 0.1, 6), "el_cane", T((hx + 0.03, -0.07, 0.36), (0, 90, 0)), bone="Arm.R")
    k.add(sphere(0.022, 6, 4), "el_frame", T((hx + 0.08, -0.07, 0.36)), smooth=True, bone="Arm.R")


def worker(k):
    """Broad quarry/kiln hand: flat cap, kerchief, leather apron with a hammer, rolled sleeves, work gloves."""
    N.body(k, {"shoe": "wk_boot", "pants": "wk_pants", "top": "wk_shirt", "sleeve": "wk_shirt"}, sleeves="rolled", hands="wk_glove")
    N.hair_cap(k, "wk_hair", tilt=-15)
    k.add(sphere(0.255, 14, 8), "wk_cap", T((0, 0.02, 0.95), (-8, 0, 0), (1.02, 1.04, 0.55)), smooth=True, bone="Head")
    k.add(cyl(0.16, 0.16, 0.02, 12), "wk_cap", T((0, -0.17, 0.97), (-10, 0, 0), (1.0, 0.55, 1)), smooth=True, bone="Head")   # peak
    k.add(cyl(0.135, 0.125, 0.05, 12, 0.012), "wk_scarf", T((0, 0.0, 0.625)), smooth=True, bone="Torso")
    k.add(prism([(-0.07, 0), (0.07, 0), (0, -0.09)], 0.02, 0.005), "wk_scarf", T((0, -0.152, 0.62)), bone="Torso")
    k.add(box(0.23, 0.03, 0.2, 0.01), "wk_apron", T((0, -0.125, 0.5)), bone="Torso")
    k.add(box(0.31, 0.03, 0.2, 0.01), "wk_apron", T((0, -0.118, 0.3)), bone="Hips")
    for s in (1, -1):
        k.add(box(0.03, 0.2, 0.03), "wk_apron", T((s * 0.085, -0.03, 0.62)), bone="Torso")
    k.add(box(0.33, 0.23, 0.04, 0.012), "leather", T((0, 0, 0.39)), bone="Hips")
    k.add(box(0.025, 0.025, 0.16), "wood_mid", T((-0.16, -0.05, 0.3)), bone="Hips")                     # hammer
    k.add(box(0.03, 0.09, 0.04, 0.008), "iron", T((-0.16, -0.05, 0.39)), bone="Hips")


def sailor(k):
    """Weathered sailor: striped knit jumper, yellow brimmed rain hat, grey beard, a coil of rope over the shoulder."""
    N.body(k, {"shoe": "sa_boot", "pants": "sa_pants", "top": "sa_jumper", "sleeve": "sa_jumper"})
    N.hair_cap(k, "sa_hair", tilt=-15)
    k.add(sphere(0.15, 12, 6), "sa_beard", T((0, -0.12, 0.72), scale=(1.25, 0.75, 0.75)), smooth=True, bone="Head")
    k.add(sphere(0.045, 8, 4), "sa_beard", T((0, -0.215, 0.77), scale=(1.6, 0.5, 0.55)), smooth=True, bone="Head")
    k.add(cyl(0.34, 0.31, 0.03, 18, 0.008), "sa_hat", T((0, 0.03, 1.0), (8, 0, 0)), smooth=True, bone="Head")
    k.add(cyl(0.21, 0.19, 0.14, 16, 0.02), "sa_hat", T((0, 0.02, 1.07), (-4, 0, 0)), smooth=True, bone="Head")
    for z in (0.44, 0.52, 0.6):  # jumper stripes (the torso widens from 0.135 at z 0.38 to 0.16 at 0.62)
        r = 0.135 + (0.16 - 0.135) * (z - 0.38) / 0.24 + 0.006
        k.add(cyl(r, r, 0.03, 12), "sa_stripe", T((0, 0, z), scale=(1.05, 0.8, 1)), smooth=True, bone="Torso")
    for s, b in ((1, "Arm.L"), (-1, "Arm.R")):
        for dz in (-0.04, 0.03):
            k.add(cyl(0.053, 0.053, 0.025, 8), "sa_stripe", T((s * 0.2, 0, 0.47), (0, s * 8, 0)) @ T((0, 0, dz)), smooth=True, bone=b)
    k.add(torus(0.17, 0.028, 12, 5), "rope", T((0.02, 0.0, 0.52), (0, 62, 0), (1, 0.75, 1)), bone="Torso")


def scholar(k):
    """Scholar: long open burgundy coat, round glasses, a long mustard scarf, a map roll under the arm, a satchel."""
    N.body(k, {"shoe": "sc_shoe", "pants": "sc_pants", "top": "sc_vest", "sleeve": "sc_coat"})
    N.hair_cap(k, "sc_hair", tilt=-22)
    for s in (1, -1):
        k.add(sphere(0.12, 10, 6), "sc_hair", T((s * 0.17, 0.03, 0.78), scale=(0.55, 1.2, 1.15)), smooth=True, bone="Head")
        k.add(N.ring(0.064, 0.05), "sc_frame", T((s * 0.085, -0.228, 0.82)), bone="Head")
        k.add(box(0.012, 0.16, 0.012), "sc_frame", T((s * 0.15, -0.15, 0.83)), bone="Head")
    k.add(box(0.05, 0.012, 0.012), "sc_frame", T((0, -0.229, 0.83)), bone="Head")
    N.skirt(k, "sc_coat", 0.17, 0.21, 0.08, 0.4, 14, open_front=True)
    for s in (1, -1):  # open coat fronts over the vest
        k.add(box(0.07, 0.03, 0.25, 0.01), "sc_coat", T((s * 0.1, -0.115, 0.5), (0, 0, s * -6)), bone="Torso")
    k.add(cyl(0.15, 0.14, 0.07, 14, 0.02), "sc_scarf", T((0, 0.0, 0.62)), smooth=True, bone="Torso")
    for x, L in ((0.05, 0.24), (-0.04, 0.3)):
        k.add(box(0.06, 0.03, L, 0.012), "sc_scarf", T((x, -0.135, 0.6 - L / 2), (0, 4, 0)), bone="Torso")
    k.add(cyl(0.035, 0.035, 0.34, 10), "paper", T((0.27, -0.02, 0.43), (60, 0, 0)), smooth=True, bone="Arm.L")
    k.add(cyl(0.037, 0.037, 0.02, 10), "wax", T((0.27, -0.02, 0.43), (60, 0, 0)), bone="Arm.L")
    k.add(box(0.06, 0.16, 0.13, 0.015), "sc_bag", T((-0.17, 0.0, 0.33)), bone="Hips")
    k.add(box(0.02, 0.03, 0.34), "sc_bag", T((0.0, -0.13, 0.48), (0, -38, 0)), bone="Torso")


STOOP = {"Torso": ((0, 0, 0), (10, 0, 0)), "Head": ((0, 0, 0), (-6, 0, 0)), "Arm.R": ((0, 0, 0), (-10, 0, -4))}
NPCS_V2 = {  # as build_npcs.NPCS: (builder, scale, head_scale, leg, idle pose offsets, walk arm swing)
    "npc_elder": (elder, 1.2, 0.93, 1.12, STOOP, 0.35),
    "npc_worker": (worker, 1.32, 0.94, 1.22, {"Arm.L": ((0, 0, 0), (0, 0, 6)), "Arm.R": ((0, 0, 0), (0, 0, -6))}, 1.0),
    "npc_sailor": (sailor, 1.28, 0.95, 1.14, None, 0.8),
    "npc_scholar": (scholar, 1.24, 1.0, 1.12, {"Head": ((0, 0, 0), (5, 0, 0))}, 0.6),
}
TAGS_V2 = {
    "npc_elder": {"primary": "#8f6fa0", "secondary": "#e8d9b0", "hair": "#eeeae6", "accent": "#3f6b5a"},
    "npc_worker": {"primary": "#c86a3a", "secondary": "#8a6040", "hair": "#4a3326", "accent": "#2f7f86"},
    "npc_sailor": {"primary": "#2f4a7a", "secondary": "#f1e7d2", "hair": "#b9b4ae", "accent": "#f0b830"},
    "npc_scholar": {"primary": "#7a2f3a", "secondary": "#e0a13a", "hair": "#2f2420", "accent": "#e9dcc0"},
}


# ---------------------------------------------------------------- build, export, checks, sheets


SLOTS = {  # Blender (x, y) in the PropsV2 scene; footprints never overlap
    "oven": (-9, 6), "printing_press": (-6, 6), "type_case": (-3.5, 6), "bookcase": (-1, 6), "bed": (2, 6),
    "workbench": (5, 6), "light_box": (8, 6), "net_locker": (10.5, 6),
    "hanging_sign": (-9.5, 3), "curio_cabinet": (-7, 3), "waymark": (-4.5, 3), "seal_stone": (-2, 2.5),
    "cave_mouth": (1, 3), "climb_wall": (3.5, 3), "inn_sign": (5.5, 3),
    "curio": (-9, 0), "brass_thimble": (-8, 0), "heron_feather": (-7, 0), "message_bottle": (-6, 0),
    "fossil_fern": (-5, 0), "tin_hex": (-4, 0), "lead_type": (-3, 0),
    "npc_elder": (0, 0), "npc_worker": (1.2, 0), "npc_sailor": (2.4, 0), "npc_scholar": (3.6, 0),
}
CURIO_Z = 0.4  # curios are centred on the origin; lift them in the scene so they don't sit in the ground


def build_all(scn, only):
    built = {}
    for name, fn in PROPS.items():
        if only and name not in only:
            continue
        coll = bpy.data.collections.new(B.PREFIX + name)
        scn.collection.children.link(coll)
        kit = B.Kit("curio_" + name if name in CURIOS and name != "curio" else name)  # (no node may start with "hero")
        after = fn(kit)
        ob = kit.build(coll)
        built[name] = [ob] + (after(ob, coll) if after else [])
    for name, (fn, S, head, leg, pose, swing) in NPCS_V2.items():
        if only and name not in only:
            continue
        coll = bpy.data.collections.new(B.PREFIX + name)
        scn.collection.children.link(coll)
        kit = B.Kit(name)
        fn(kit)
        built[name] = B.rig_character(kit, coll, name, S, head_scale=head, leg=leg, clips=B.NPC_CLIPS, pose=pose, arm_swing=swing)
    for name, objs in built.items():
        x, y = SLOTS[name]
        z = CURIO_Z if name in CURIOS else 0
        bpy.data.collections[B.PREFIX + name].instance_offset = (x, y, z)
        for o in objs:
            if o.parent is None:
                o.location = (x, y, z)
    return built


def import_glb(path, loc):
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=path)
    new = [o for o in bpy.data.objects if o not in before]
    for o in new:
        if o.parent is None:
            o.location = Vector(loc) + o.location
    return new


def stage(scn, ground=True):
    """Sun, sky and a grass plane like the area previews."""
    sun = bpy.data.objects.new("pv2_sun", bpy.data.lights.new(B.PREFIX + "pv2_sun", "SUN"))
    sun.data.energy = 3.5
    sun.rotation_euler = (math.radians(40), math.radians(20), math.radians(-30))
    scn.collection.objects.link(sun)
    world = bpy.data.worlds.new(B.PREFIX + "pv2_world")
    world.use_nodes = True
    bg = next(n for n in world.node_tree.nodes if n.type == "BACKGROUND")
    bg.inputs["Color"].default_value = (0.55, 0.62, 0.7, 1)
    bg.inputs["Strength"].default_value = 0.9
    scn.world = world
    if ground:
        g = B.Kit("pv2_ground")
        g.add(box(30, 18, 0.02), "grass", T((0, 2, -0.012)))
        g.build(scn.collection)
    scn.view_settings.view_transform = "Standard"
    scn.render.image_settings.file_format = "PNG"
    scn.render.engine = "BLENDER_EEVEE"
    cam = bpy.data.objects.new("pv2_cam", bpy.data.cameras.new(B.PREFIX + "pv2_cam"))
    scn.collection.objects.link(cam)
    scn.camera = cam
    return cam


def preview(scn, path, assets):
    """Game camera (50 deg pitch) over every new model, with the house, cottage, stall and an NPC for scale."""
    for name, loc in (("house.glb", (-2, 11.5, 0)), ("cottage.glb", (5, 11, 0)), ("stall.glb", (10, 3, 0)),
                      ("npc/npc_shopkeeper.glb", (-1.2, 0, 0)), ("hero.glb", (4.8, 0, 0))):
        p = os.path.join(assets, name)
        if os.path.exists(p):
            import_glb(p, loc)
    cliff = os.path.join(assets, "cliff.glb")  # the cave mouth and the climb wall stand against a cliff (row behind)
    if os.path.exists(cliff):
        for x, z in ((0.5, 0), (1.5, 0), (0.5, 1), (1.5, 1), (2.5, 0), (3.5, 0), (4.5, 0)):
            import_glb(cliff, (x, 4, z))
    cam = stage(scn)
    cam.data.lens = 30
    pitch, (ax, ay, dist) = 50, (0.5, 5.0, 20.0)
    cam.rotation_euler = (math.radians(90 - pitch), 0, 0)
    cam.location = (ax, ay - dist * math.cos(math.radians(pitch)), dist * math.sin(math.radians(pitch)))
    scn.render.resolution_x, scn.render.resolution_y = 2000, 1300
    scn.render.filepath = path
    bpy.ops.render.render(write_still=True, scene=scn.name)
    print("[sixfold] props v2 preview ->", path)


def sheet(scn, built, path, tile=300):
    """One tile per model and view (front 3/4 from the game pitch, then from behind), ortho, fitted."""
    import numpy as np
    cam = stage(scn, ground=False)
    cam.data.type = "ORTHO"
    scn.render.resolution_x = scn.render.resolution_y = tile
    scn.render.film_transparent = False
    tmp = os.path.join(os.path.dirname(os.path.abspath(path)), "_pv2_tile.png")
    rows, row = [], []
    names = list(built)
    for name in names:
        objs = built[name]
        pts = []
        for o in objs:
            if o.type == "MESH":
                pts += [o.matrix_world @ Vector(c) for c in o.bound_box]
        lo = Vector([min(p[i] for p in pts) for i in range(3)])
        hi = Vector([max(p[i] for p in pts) for i in range(3)])
        c, size = (lo + hi) / 2, (hi - lo).length
        hidden = [o for o in scn.objects if o.type == "MESH" and o not in objs]
        for o in hidden:
            o.hide_render = True
        for d in ((0.55, -0.75, 0.6), (-0.55, 0.75, 0.6)):
            d = Vector(d).normalized()
            cam.data.ortho_scale = size * 1.05
            cam.location = c + d * 30
            cam.rotation_euler = (-d).to_track_quat("-Z", "Y").to_euler()
            scn.render.filepath = tmp
            bpy.ops.render.render(write_still=True, scene=scn.name)
            img = bpy.data.images.load(tmp)
            row.append(np.array(img.pixels[:], dtype=np.float32).reshape(tile, tile, 4))
            bpy.data.images.remove(img)
            if len(row) == 8:
                rows.append(row)
                row = []
        for o in hidden:
            o.hide_render = False
    if row:
        row += [np.ones((tile, tile, 4), np.float32)] * (8 - len(row))
        rows.append(row)
    os.remove(tmp)
    px = np.concatenate([np.concatenate(r, axis=1) for r in reversed(rows)], axis=0)
    img = bpy.data.images.new("pv2_sheet", px.shape[1], px.shape[0])
    img.pixels.foreach_set(px.ravel())
    img.filepath_raw = path
    img.file_format = "PNG"
    img.save()
    print("[sixfold] props v2 sheet ->", path, "order:", ", ".join(names))


def check(scn):
    import check_coplanar
    g = B.Kit("pv2_check_ground")  # the game's ground plane: nothing may be coplanar with it
    g.add(box(30, 18, 0.0001), "grass", T((0, 2, -0.00005)))
    g.build(scn.collection)
    return check_coplanar.check()


def main(argv):
    arg = lambda k: argv[argv.index(k) + 1] if k in argv else None  # noqa: E731
    out, pv, sh, only = arg("--out"), arg("--preview"), arg("--sheet"), arg("--only")
    only = set(only.split(",")) if only else None
    scn = B.fresh_scene("PropsV2")
    with B.use_scene(scn):
        built = build_all(scn, only)
        if out:
            out = os.path.abspath(out)
            for name, objs in built.items():
                for e in objs[1:]:  # Blender suffixes repeated names (light_origin.001); the game wants the plain one
                    if e.type != "EMPTY":
                        continue
                    base = e.name.split(".")[0]
                    other = bpy.data.objects.get(base)
                    if other and other is not e:
                        other.name = base + "_parked"
                    e.name = base
                B.export({name: objs}, out)
            people = [n for n in built if n in NPCS_V2]
            if people:
                pal = os.path.join(out, "npc", "npc_palettes.json")
                tags = json.load(open(pal)) if os.path.exists(pal) else {}
                tags.update({n: TAGS_V2[n] for n in people})
                with open(pal, "w") as f:
                    json.dump(tags, f, indent=2)
                    f.write("\n")
        if "--check" in argv:
            check(scn)
        if sh:
            sheet(scn, built, os.path.abspath(sh))
        if pv:
            preview(scn, os.path.abspath(pv), os.path.abspath(arg("--assets") or out or "web/world/assets"))
    B.save_if_file()


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
