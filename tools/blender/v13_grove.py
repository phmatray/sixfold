"""Grove: The Deepgrove (Grass). Old forest grown huge: a fallen elder whose roots make a village, walkways in
the canopy, moss everywhere, a hedge maze at the heart, bees.

Vocabulary: thick gnarled bark tubes (roots, the fallen trunk, the elder trees), moss blankets and ferns on every
top, rough log carpentry with rope lashings (treehouse, ladder, canopy deck, rope bridge), clipped hedges.
Glow parts: gv_window_glow (treehouse windows), lamp_glow (the lanterns), gv_shroom_glow (pale ring mushrooms).
"""
import math
import random

from v13_kit import (A, B, T, arch_pts, beam, box, contact, curve, cyl, empty, ico, jitter, lathe, prism,  # noqa: F401
                     ribbon, shadow, sphere, torus, tube, clamp01, chain, FRONT)

PALETTE = {
    "gv_bark": "#6e4c36", "gv_bark_dk": "#563a2a", "gv_bark_lt": "#8a6444", "gv_heart": "#c9a06a", "gv_ring": "#a87d50",
    "gv_moss": "#5f9c3a", "gv_moss_lt": "#82bd4c", "gv_moss_dk": "#457a2d", "gv_fern": "#4f9a3e", "gv_fern_lt": "#74b54a",
    "gv_hedge": "#3d8a3c", "gv_hedge_lt": "#58a647", "gv_hedge_dk": "#2c6930",
    "gv_elder": "#337a46", "gv_elder_dk": "#25613a", "gv_elder_lt": "#5ea34b", "gv_elder_top": "#9cc455",
    "gv_wood": "#b07d4c", "gv_wood_dk": "#7c5636", "gv_wood_lt": "#c99a63",
    "gv_shingle": "#a85a3b", "gv_shingle_dk": "#7e412b", "gv_door": "#8a3f4c", "gv_wall": "#d9b98a",
    "gv_window_glow": "#ffd98a", "gv_earth": "#6b4a32", "gv_earth_dk": "#4e3524", "gv_pebble": "#a69b88",
    "gv_straw": "#dbb45c", "gv_straw_dk": "#b8913f", "gv_honey": "#eaa22a", "gv_bee": "#f2c230", "gv_bee_dk": "#2a221c",
    "gv_cap": "#e0654a", "gv_cap_dk": "#b9493a", "gv_spot": "#fbefd8", "gv_stem": "#efe4cc", "gv_shroom_glow": "#a6ecd8",
    "gv_fungus": "#d9a35a", "gv_cord": "#c0503a", "gv_ribbon": "#f1e2c2", "gv_blossom": "#f6e7f0", "gv_blossom_b": "#f5c7d6",
    "gv_hedge_mid": "#4a9a40",
}

WIND = {
    "standing_tree": lambda x, y, z, r: clamp01((z - 3.45) / 2.4 * 0.6 + r / 2.4 * 0.4) if z > 3.45 else 0.0,   # canopy plates only
}

FOOT = {"great_root": (4, 2), "treehouse": (4, 4), "standing_tree": (3, 3), "fallen_trunk": (4, 3), "fallen_trunk_end": (4, 3)}
ANIM = set()

LIGHT = dict(sun=("#f7ffd6", 2.2), from_hero=(8, 18, 6), hemi=("#f3ffe0", "#2f5a24", 1.5), bg="#8fbf7a", fog=(14, 30))


# ---------------------------------------------------------------- small dressing


def fern(k, rng, x, y, z, s=1.0, n=6, tilt=55):
    """A rosette of fern fronds."""
    leaf = [(0, 0), (0.06, 0.1), (0.055, 0.3), (0, 0.44), (-0.055, 0.3), (-0.06, 0.1)]
    for i in range(n):
        a = i / n * 360 + rng.uniform(-15, 15)
        k.add(prism(leaf, 0.012), "gv_fern" if i % 2 else "gv_fern_lt",
              T((x, y, z), (0, 0, a)) @ T(rot=(tilt + rng.uniform(-10, 10), 0, 0), scale=(s, s, s)))


def moss_pad(k, rng, x, y, z, r, mat=None, squash=0.32):
    k.add(jitter(ico(r, 1), rng, r * 0.18), mat or rng.choice(["gv_moss", "gv_moss_lt", "gv_moss_dk"]),
          T((x, y, z), (0, 0, rng.uniform(0, 90)), (1.2, 1.0, squash)), smooth=True)


def toadstool(k, rng, x, y, s, cap="gv_cap", spots=True):
    h = 0.16 * s
    k.add(cyl(0.025 * s, 0.032 * s, h, 6), "gv_stem", T((x, y, h / 2 - 0.01)))
    k.add(lathe([(0.085 * s, h - 0.01), (0.075 * s, h + 0.04 * s), (0.0, h + 0.08 * s)], 6, rng.uniform(0, 50)),
          cap, T((0, 0, 0), (rng.uniform(-8, 8), rng.uniform(-8, 8), 0)) @ T((x, y, 0)), smooth=True)
    if spots:
        a = rng.uniform(0, math.tau)
        k.add(sphere(0.016 * s, 5, 3), "gv_spot", T((x + 0.035 * s * math.cos(a), y + 0.035 * s * math.sin(a), h + 0.062 * s), scale=(1, 1, 0.5)))


def bark_tube(k, pts, r, taper, mat="gv_bark", sides=7):
    k.add(tube(pts, r, sides, taper), mat, smooth=True)


# ---------------------------------------------------------------- hedges (auto-tile, arms as the fence)


HW = 0.64   # hedge crown width


def crown_profile(w, z0, z1, r=0.085):
    """Cross-section of the clipped crown: straight sides, rounded top edges."""
    arc_ = [(w / 2 - r + r * math.cos(math.radians(a)), z1 - r + r * math.sin(math.radians(a))) for a in (0, 30, 60, 90)]
    right = arc_
    left = [(-x, z) for x, z in reversed(arc_)]
    return [(-w / 2, z0), (w / 2, z0)] + right + left


def build_hedge_piece(k, arms, seed, top=1.2):
    """1.2 m clipped hedge through the tile centre, arms to the tile edges (fence rules). A darker woody base band,
    the leafy body, a crown with rounded top edges; leafy clumps in three greens break the edges and the faces, a few
    small blossoms. One crown runs along X through the centre (every piece has the W arm), the S arm's crown stops
    at its side, so no two top faces overlap; hedge_end sits 1 cm higher for the 4-way tile (t + end)."""
    rng = random.Random(seed)
    for z0, z1, m, th in ((-0.02, 0.2, "gv_hedge_dk", 0.6), (0.2, 1.02, "gv_hedge", 0.62)):
        k.add(box(th, th, z1 - z0), m, T((0, 0, (z0 + z1) / 2)))
        for arm in arms:
            dx, dy = B.ARM_DIR[arm]
            ln, mid = 0.5 - th / 2, (0.5 + th / 2) / 2
            k.add(box(ln if dx else th, th if dx else ln, z1 - z0), m, T((dx * mid, dy * mid, (z0 + z1) / 2)))
    prof = crown_profile(HW, 1.02, top)   # sits on the body; its ends never share a plane with the body's
    x0, x1 = -0.5, (0.5 if "E" in arms else HW / 2)
    k.add(prism(prof, x1 - x0), "gv_hedge_lt", T(((x0 + x1) / 2, 0, 0), (0, 0, 90)))
    if "S" in arms:
        k.add(prism(prof, 0.5 - HW / 2), "gv_hedge_lt", T((0, -(0.5 + HW / 2) / 2, 0)))
    # leafy clumps: along the crown edges and on the faces of every run, and on the free ends
    runs = []
    for arm in arms:
        dx, dy = B.ARM_DIR[arm]
        runs.append((dx, dy, 0.12, 0.44))
    for arm, (dx, dy) in B.ARM_DIR.items():
        if arm not in arms:
            runs.append((dx, dy, 0.0, 0.0))
    greens = ["gv_hedge", "gv_hedge_mid", "gv_hedge_lt"]
    for dx, dy, u0, u1 in runs:
        px, py = -dy, dx        # across the run
        for side in (-1, 1):
            for j in range(2 if u1 else 1):
                u = rng.uniform(u0, u1) if u1 else 0.0
                base = (dx * (u if u1 else HW / 2 - 0.04), dy * (u if u1 else HW / 2 - 0.04))
                off = side * (HW / 2 - 0.02) if u1 else side * rng.uniform(0.05, 0.2)
                x, y = base[0] + px * off, base[1] + py * off
                r = rng.uniform(0.09, 0.13)
                k.add(jitter(ico(r, 1), rng, r * 0.2), rng.choice(greens), T((x, y, top - 0.03), scale=(1.1, 1.1, 0.8)), smooth=True)
                if u1:
                    r = rng.uniform(0.08, 0.11)
                    k.add(jitter(ico(r, 1), rng, r * 0.2), rng.choice(greens),
                          T((x * 0.98, y * 0.98, rng.uniform(0.4, 0.85)), scale=(1, 1, 1.2)), smooth=True)
        if u1 and rng.random() < 0.7:   # a few blossoms on one face
            side = rng.choice((-1, 1))
            u = rng.uniform(u0, u1)
            for i in range(3):
                x = dx * (u + rng.uniform(-0.06, 0.06)) + px * side * (HW / 2 - 0.01)
                y = dy * (u + rng.uniform(-0.06, 0.06)) + py * side * (HW / 2 - 0.01)
                k.add(ico(0.03, 1), rng.choice(["gv_blossom", "gv_blossom_b"]), T((x, y, rng.uniform(0.55, 1.05))))


# ---------------------------------------------------------------- trees and roots


def canopy_plate(k, rng, cx, cy, z, r, mats):
    """A broad flat plate of foliage: a lumpy disc much wider than tall, darker underneath, lit lumps on its rim."""
    body = lathe([(0.0, 0.0), (r * 0.7, -0.02), (r, 0.18), (r * 0.94, 0.38), (r * 0.62, 0.55), (0.0, 0.6)], 14, rng.uniform(0, 30))
    for v in body.verts:
        if math.hypot(v.co.x, v.co.y) > r * 0.5:
            f = rng.uniform(0.9, 1.1)
            v.co.x *= f
            v.co.y *= f
            v.co.z += rng.uniform(-0.05, 0.05)
    k.add(body, mats[1], T((cx, cy, z), scale=(1.0, 0.92, 1.0)), smooth=True)
    k.add(cyl(r * 0.78, r * 0.6, 0.12, 12), mats[0], T((cx, cy, z - 0.02), scale=(1.0, 0.92, 1.0)), smooth=True)
    n = max(6, int(r * 5))
    for i in range(n):
        a = i / n * math.tau + rng.uniform(-0.15, 0.15)
        rr = rng.uniform(0.32, 0.45) * (1 if r > 1.2 else 0.8)
        k.add(jitter(ico(rr, 1), rng, rr * 0.15), mats[2 if i % 3 else 1],
              T((cx + r * 0.86 * math.cos(a), cy + r * 0.8 * math.sin(a), z + 0.3), scale=(1.15, 1.15, 0.6)), smooth=True)
    return body


def build_standing_tree(k):
    """3 x 3 ancient elder (~6 m), the Elder Ring's landmark: a massive gnarled trunk on great buttress roots that
    spill over the footprint, a hollow and moss on the front, a revered cord tied round it with two ribbon tails,
    three heavy low limbs reaching sideways, and a canopy of three broad flat layered plates (wider than tall)
    trailing moss strands. Wind moves only the plates."""
    rng = random.Random(501)
    trunk = lathe([(1.1, -0.02), (0.92, 0.25), (0.78, 0.7), (0.7, 1.3), (0.68, 1.9), (0.74, 2.5), (0.9, 2.95), (0.6, 3.4)], 12)
    for v in trunk.verts:  # twist and gnarl
        a = v.co.z * 0.3
        x, y = v.co.x, v.co.y
        v.co.x, v.co.y = x * math.cos(a) - y * math.sin(a), x * math.sin(a) + y * math.cos(a)
        if 0.2 < v.co.z < 3.2:
            v.co.x *= rng.uniform(0.88, 1.12)
            v.co.y *= rng.uniform(0.88, 1.12)
    k.add(trunk, "gv_bark", smooth=True)
    for i in range(6):  # buttress roots: tall fins from high on the trunk, spilling out over the 3 x 3
        a = math.radians(i * 60 + 20 + rng.uniform(-10, 10))
        ca, sa = math.cos(a), math.sin(a)
        L = rng.uniform(1.45, 1.75)
        fin = tube([(0.45 * ca, 0.45 * sa, 1.5), (0.95 * ca, 0.95 * sa, 0.7), (1.3 * ca, 1.3 * sa, 0.22), (L * ca, L * sa, -0.02),
                    ((L + 0.2) * ca, (L + 0.2) * sa, -0.15)], 0.32, 6, 0.25)
        for v in fin.verts:   # flatten across into a fin
            d = v.co.x * -sa + v.co.y * ca
            v.co.x -= -sa * d * 0.45
            v.co.y -= ca * d * 0.45
        k.add(fin, "gv_bark_dk" if i % 2 else "gv_bark", smooth=True)
        moss_pad(k, rng, 1.0 * ca, 1.0 * sa, 0.62, 0.2, squash=0.45)
        sub = math.radians(i * 60 + 50)   # a lesser root between the fins
        bark_tube(k, [(0.75 * math.cos(sub), 0.75 * math.sin(sub), 0.3), (1.25 * math.cos(sub), 1.25 * math.sin(sub), 0.05),
                      (1.45 * math.cos(sub), 1.45 * math.sin(sub), -0.1)], 0.12, 0.4, "gv_bark", 5)
    # hollow on the front, moss, a shelf fungus
    k.add(prism([(math.cos(t) * 0.22, 0.1 + math.sin(t) * 0.34) for t in (i / 10 * math.tau for i in range(10))], 0.1), "gv_bark_dk", T((0.0, -0.7, 1.15)))
    k.add(prism([(math.cos(t) * 0.16, 0.1 + math.sin(t) * 0.27) for t in (i / 10 * math.tau for i in range(10))], 0.1), "gv_earth_dk", T((0.0, -0.74, 1.15)))
    for x, z in ((0.45, 2.1), (-0.5, 1.7), (0.2, 2.8)):
        moss_pad(k, rng, x, -0.72, z, 0.2, "gv_moss_lt", 0.5)
    k.add(cyl(0.2, 0.15, 0.06, 7), "gv_fungus", T((0.68, -0.35, 1.0), scale=(1, 0.7, 1)))
    # the revered cord with two ribbon tails
    k.add(torus(0.78, 0.045, 16, 4), "gv_cord", T((0, 0, 1.72), scale=(1.0, 0.98, 1)))
    k.add(sphere(0.07, 6, 4), "gv_cord", T((0.25, -0.76, 1.72)))
    for dx, tilt in ((0.17, 8), (0.31, -10)):
        k.add(box(0.07, 0.015, 0.42), "gv_ribbon", T((dx, -0.8, 1.48), (0, tilt, 0)))
    # heavy low limbs reaching sideways, each carrying a plate
    for a, L, z1 in ((200, 2.0, 3.25), (340, 1.9, 3.55), (95, 1.6, 4.25)):
        ca, sa = math.cos(math.radians(a)), math.sin(math.radians(a))
        bark_tube(k, [(0.3 * ca, 0.3 * sa, 2.55), (0.9 * ca, 0.9 * sa, 2.75), (L * 0.75 * ca, L * 0.75 * sa, z1 - 0.15), (L * ca, L * sa, z1)],
                  0.24, 0.45, "gv_bark", 7)
    # three broad flat plates, stepped up and turned so they read as layers
    canopy_plate(k, rng, -0.55, -0.25, 3.5, 1.9, ("gv_elder_dk", "gv_elder", "gv_elder_lt"))
    canopy_plate(k, rng, 0.65, -0.15, 4.25, 1.55, ("gv_elder_dk", "gv_elder", "gv_elder_lt"))
    canopy_plate(k, rng, -0.1, 0.45, 5.1, 1.05, ("gv_elder", "gv_elder_lt", "gv_elder_top"))
    bark_tube(k, [(0.2, 0.1, 3.3), (0.0, 0.3, 4.4), (-0.1, 0.45, 5.15)], 0.2, 0.5, "gv_bark", 6)   # the leader to the top plate
    for i in range(9):  # moss strands hanging under the plate rims
        cx, cy, z, r = ((-0.55, -0.25, 3.5, 1.9), (0.65, -0.15, 4.25, 1.55))[i % 2]
        a = rng.uniform(0, math.tau)
        x, y = cx + r * 0.8 * math.cos(a), cy + r * 0.75 * math.sin(a)
        L = rng.uniform(0.5, 0.95)
        k.add(tube([(x, y, z + 0.02), (x + 0.03, y, z - L / 2), (x, y + 0.02, z - L)], 0.035, 4, 0.4), "gv_moss_lt", smooth=True)
    for i in range(3):
        fern(k, rng, 1.25 * math.cos(i * 2.1 + 0.9), 1.25 * math.sin(i * 2.1 + 0.9), 0.0, 1.1, 5)
    contact(k, 1.5)


def build_great_root(k):
    """4 x 2 root arch (~2.7 m): a huge mossy root heaving out of the ground and diving back in, thicker at the
    west end where the elder lies, with lesser roots twined round it, ferns and toadstools at its feet and a
    lantern hung from the crown."""
    rng = random.Random(502)
    pts = [(-1.95, 0.15, -0.35), (-1.6, 0.1, 0.9), (-1.0, 0.0, 2.0), (-0.2, -0.05, 2.45), (0.6, 0.0, 2.35),
           (1.25, 0.05, 1.7), (1.65, 0.1, 0.8), (1.9, 0.15, -0.3)]
    main = tube(pts, 0.6, 8, 0.55)
    for v in main.verts:
        v.co.x += rng.uniform(-0.04, 0.04)
        v.co.y += rng.uniform(-0.05, 0.05)
    k.add(main, "gv_bark", smooth=True)
    for off, r, m in ((0.36, 0.16, "gv_bark_dk"), (-0.38, 0.13, "gv_bark_lt")):  # lesser roots twining round
        sub = [(x + 0.08 * math.sin(i * 1.7), y + off * math.cos(i * 1.3), z + off * 0.6 * math.sin(i * 1.3)) for i, (x, y, z) in enumerate(pts)]
        bark_tube(k, sub, r, 0.6, m, 5)
    for sx in (-1, 1):  # root feet splaying into the ground
        for j in range(3):
            a = math.radians(rng.uniform(-70, 70) + (180 if sx < 0 else 0))
            x0 = sx * 1.75
            bark_tube(k, [(x0, 0.1, 0.35), (x0 + 0.35 * math.cos(a), 0.1 + 0.45 * math.sin(a), 0.1),
                          (x0 + 0.6 * math.cos(a), 0.1 + 0.75 * math.sin(a), -0.08)], 0.12, 0.35, "gv_bark_dk", 5)
    for t in (0.18, 0.3, 0.42, 0.55, 0.68, 0.8):  # a moss blanket along the top of the arch
        i = t * (len(pts) - 1)
        a, b = pts[int(i)], pts[min(int(i) + 1, len(pts) - 1)]
        f = i - int(i)
        x, y, z = (a[n] + (b[n] - a[n]) * f for n in range(3))
        moss_pad(k, rng, x, y, z + 0.42, rng.uniform(0.32, 0.4), squash=0.4)
    for x, y in ((-1.4, -0.6), (1.35, -0.55), (-0.9, 0.65), (1.5, 0.6)):
        fern(k, rng, x, y, 0.0, rng.uniform(0.9, 1.2))
    for x, y, s in ((-1.1, -0.55, 1.0), (-1.25, -0.4, 0.7), (1.05, -0.62, 0.85)):
        toadstool(k, rng, x, y, s)
    # lantern from the crown
    k.add(cyl(0.008, 0.008, 0.35, 4), "rope", T((0.1, -0.12, 1.95)))
    k.add(box(0.14, 0.14, 0.18), "lamp_glow", T((0.1, -0.12, 1.68)))
    for sx in (-1, 1):
        for sy in (-1, 1):
            k.add(box(0.02, 0.02, 0.2), "iron", T((0.1 + sx * 0.075, -0.12 + sy * 0.075, 1.68)))
    k.add(cyl(0.12, 0.0, 0.08, 4), "iron", T((0.1, -0.12, 1.81), (0, 0, 45)))
    shadow(k, 1.0, 2.0, 0.85)
    return empty("light_origin", (0.1, -0.12, 1.68))


TRUNK_R, TRUNK_Z = 1.48, 1.4   # the fallen elder: radius, axis height (top at ~2.9 m, bottom sunk 8 cm)
RIDGE = (1.0, 1.05, 0.97, 1.03, 1.06, 0.98, 1.02, 0.96, 1.04, 1.0, 0.97, 1.05, 1.01, 0.98, 1.04, 0.99)


def trunk_section(x0, x1, rows=3):
    """The fallen trunk's bark from x0 to x1, open ends; the same ridged profile everywhere, so pieces chain."""
    import bmesh
    bm = bmesh.new()
    n = len(RIDGE)
    rings = []
    for i in range(rows + 1):
        x = x0 + (x1 - x0) * i / rows
        rings.append([bm.verts.new((x, TRUNK_R * RIDGE[j] * math.cos(j / n * math.tau), TRUNK_Z + TRUNK_R * RIDGE[j] * math.sin(j / n * math.tau)))
                      for j in range(n)])
    for a, b in zip(rings, rings[1:]):
        for j in range(n):
            q = (j + 1) % n
            bm.faces.new((a[j], b[j], b[q], a[q]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    for f in bm.faces:  # outward
        c = f.calc_center_median()
        if (f.normal.y * c.y + f.normal.z * (c.z - TRUNK_Z)) < 0:
            f.normal_flip()
    return bm


def trunk_dressing(k, rng, x0, x1):
    """Moss blanket along the top, bracket fungi and a knot on the front, ferns and saplings on top."""
    x = x0 + 0.35
    while x < x1 - 0.35:
        moss_pad(k, rng, x + rng.uniform(-0.1, 0.1), rng.uniform(-0.3, 0.3), TRUNK_Z + TRUNK_R * 0.93, rng.uniform(0.55, 0.7),
                 rng.choice(["gv_moss", "gv_moss_lt"]), squash=0.33)
        x += rng.uniform(0.55, 0.7)
    for xs, ys in ((x0 + 0.6, -0.65), (x1 - 0.8, 0.55)):  # moss spilling down the sides
        moss_pad(k, rng, xs, ys * 1.6, TRUNK_Z + 1.05, 0.38, "gv_moss_dk", 0.4)
    for xs in (x0 + 1.1, x0 + 1.35, x0 + 2.6):  # bracket fungi on the front
        if xs < x1 - 0.3:
            k.add(cyl(0.2, 0.16, 0.06, 8), "gv_fungus", T((xs, -TRUNK_R * 1.02, TRUNK_Z + rng.uniform(-0.3, 0.25)), scale=(1, 0.55, 1)))
    k.add(prism([(math.cos(t) * 0.2, math.sin(t) * 0.14) for t in (i / 10 * math.tau for i in range(10))], 0.1), "gv_bark_dk",
          T((x0 + 2.0 if x1 - x0 > 3 else x0 + 1.2, -TRUNK_R * 0.98, TRUNK_Z - 0.35)))
    for i in range(2):
        fern(k, rng, x0 + 0.9 + 1.6 * i, rng.uniform(-0.2, 0.2), TRUNK_Z + TRUNK_R + 0.04, 1.1, 6)
    xs = x0 + (x1 - x0) * 0.62  # a sapling taking root on the trunk
    k.add(cyl(0.025, 0.015, 0.55, 5), "gv_bark_lt", T((xs, 0.15, TRUNK_Z + TRUNK_R + 0.3)))
    for dz, r in ((0.55, 0.16), (0.7, 0.12)):
        k.add(jitter(ico(r, 1), rng, 0.02), "gv_moss_lt", T((xs, 0.15, TRUNK_Z + TRUNK_R + dz)), smooth=True)
    for _ in range(4):  # roots of the ground grass along the foot
        k.add(jitter(ico(0.12, 1), rng, 0.02), "gv_moss_dk", T((rng.uniform(x0 + 0.3, x1 - 0.3), rng.choice((-1, 1)) * 1.38, 0.05), scale=(1.4, 0.9, 0.5)), smooth=True)


def build_fallen_trunk(k):
    """4 x 3 segment of the colossal fallen elder lying along X (~2.9 m), open ends: chain segments along X."""
    rng = random.Random(503)
    k.add(trunk_section(-2.0, 2.0, 2), "gv_bark", smooth=True)
    for j in (2, 5, 9, 12, 14):  # dark grooves along the bark
        a = j / len(RIDGE) * math.tau + 0.2
        r = TRUNK_R * RIDGE[j] + 0.005
        k.add(box(4.0, 0.1, 0.06), "gv_bark_dk", T((0, r * math.cos(a), TRUNK_Z + r * math.sin(a)), (math.degrees(a) - 90, 0, 0)))
    trunk_dressing(k, rng, -2.0, 2.0)
    shadow(k, 1.0, 2.1, 1.65)


def build_fallen_trunk_end(k):
    """4 x 3 root end of the fallen elder: the trunk comes in from -X (chains with fallen_trunk) and ends in a
    great upturned root plate (~3.8 m) of earth and stones bristling with roots, grass on its crown."""
    rng = random.Random(504)
    k.add(trunk_section(-2.0, 0.55, 2), "gv_bark", smooth=True)
    for j in (2, 5, 9, 12, 14):
        a = j / len(RIDGE) * math.tau + 0.2
        r = TRUNK_R * RIDGE[j] + 0.005
        k.add(box(2.55, 0.1, 0.06), "gv_bark_dk", T((-0.725, r * math.cos(a), TRUNK_Z + r * math.sin(a)), (math.degrees(a) - 90, 0, 0)))
    trunk_dressing(k, rng, -2.0, 0.4)
    # root plate: a thick earthy ellipse standing across the trunk's end, rim bristling with curling roots
    plate = cyl(1.0, 1.0, 0.75, 14)
    for v in plate.verts:
        f = 1.0 + rng.uniform(-0.09, 0.09)
        v.co.x *= f
        v.co.y *= f
    k.add(plate, "gv_earth", T((1.0, 0, 1.75), (0, 90, 0), (2.15, 1.6, 1)))
    k.add(cyl(0.95, 0.95, 0.1, 14), "gv_earth_dk", T((1.4, 0, 1.75), (0, 90, 0), (1.95, 1.45, 1)))
    k.add(cyl(TRUNK_R * 0.9, TRUNK_R * 0.9, 0.06, 16), "gv_ring", T((0.6, 0, TRUNK_Z), (0, 90, 0)))   # where the trunk meets it
    for i in range(11):  # thick roots from the rim, curling outward and drooping
        a = i / 11 * math.tau + rng.uniform(-0.12, 0.12)
        ca, sa = math.cos(a), math.sin(a)
        y0, z0 = 1.5 * ca, 1.75 + 2.0 * sa
        if z0 < 0.3:
            continue
        L = rng.uniform(0.55, 0.9)
        bark_tube(k, [(0.9, y0 * 0.92, z0 * 0.96 + 0.07), (1.3, y0 + L * 0.45 * ca, z0 + L * 0.45 * sa),
                      (1.55, y0 + L * 0.8 * ca, z0 + L * 0.7 * sa - 0.15), (1.75, y0 + L * ca, z0 + L * 0.6 * sa - 0.45)],
                  rng.uniform(0.11, 0.15), 0.3, "gv_bark_dk" if i % 3 else "gv_bark", 5)
    for i in range(7):  # roots and stones sticking out of the plate face
        y, z = rng.uniform(-1.1, 1.1), rng.uniform(0.5, 3.0)
        if (y / 1.4) ** 2 + ((z - 1.75) / 1.9) ** 2 > 1:
            continue
        if i % 2:
            k.add(jitter(ico(0.17, 1), rng, 0.03), "gv_pebble", T((1.43, y, z), scale=(0.7, 1, 0.9)))
        else:
            bark_tube(k, [(1.4, y, z), (1.65, y + 0.1, z - 0.2), (1.8, y + 0.05, z - 0.55)], 0.08, 0.3, "gv_bark", 5)
    for y in (-0.7, -0.2, 0.35, 0.8):  # grass and moss on the crown of the plate
        moss_pad(k, rng, 1.0, y, 1.75 + 2.1 * math.sqrt(max(0, 1 - (y / 1.6) ** 2)) - 0.15, 0.38, "gv_moss_lt", squash=0.55)
    fern(k, rng, 1.0, -0.35, 3.65, 1.1, 5, 35)
    fern(k, rng, 1.0, 0.5, 3.55, 0.9, 5, 35)
    for y in (-1.1, 0.0, 1.0):  # a spill of earth at the foot
        k.add(jitter(ico(0.32, 1), rng, 0.06), "gv_earth", T((1.45, y, 0.02), scale=(1.2, 1.0, 0.45)), smooth=True)
    shadow(k, 1.0, 2.1, 1.65)


# ---------------------------------------------------------------- carpentry: treehouse, ladder, decks, bridge


def log(k, a, b, r, mat="gv_wood_dk", sides=7):
    k.add(tube([a, b], r, sides), mat, smooth=True)


def build_treehouse(k):
    """4 x 4 treehouse: a cabin on a log platform at 3 m (porch top 3.0), built against a living tree whose crown
    shelters the roof. Steep shingled gable to the front with a round window, a plum door on column 2 of the
    front (origin + (2, 3) in tiles; put a `ladder` on the tile in front, origin + (2, 4)), a porch rail, a lantern,
    a stovepipe. Windows glow at night."""
    rng = random.Random(505)
    P = 3.0
    # stilts and braces
    for sx in (-1, 1):
        for sy in (-1, 1):
            log(k, (sx * 1.55, sy * 1.45, -0.02), (sx * 1.5, sy * 1.4, P - 0.12), 0.13)
            k.add(cyl(0.2, 0.16, 0.12, 7), "gv_pebble", T((sx * 1.55, sy * 1.45, 0.03)))
    for sy in (-1, 1):
        beam(k, (-1.5, sy * 1.4, 0.6), (1.5, sy * 1.4, 2.6), 0.08, 0.08, "gv_wood_dk")
    for sx in (-1, 1):
        beam(k, (sx * 1.5, -1.4, 2.6), (sx * 1.5, 1.4, 0.6), 0.08, 0.08, "gv_wood_dk")
    # the living tree at the back left, through the platform
    log(k, (-1.0, 1.15, -0.02), (-1.05, 1.2, 6.4), 0.36, "gv_bark", 8)
    for a in (0, 120, 240):
        ca, sa = math.cos(math.radians(a)), math.sin(math.radians(a))
        bark_tube(k, [(-1.0 + 0.25 * ca, 1.15 + 0.25 * sa, 0.5), (-1.0 + 0.6 * ca, 1.15 + 0.6 * sa, 0.0), (-1.0 + 0.75 * ca, 1.15 + 0.75 * sa, -0.08)], 0.14, 0.35, "gv_bark", 5)
    # platform: frame and planks (top exactly at P)
    for sy in (-1, 1):
        k.add(box(3.7, 0.16, 0.18), "gv_wood_dk", T((0, sy * 1.42, P - 0.13)))
    for i in range(12):
        y = -1.85 + 0.3 * i
        k.add(box(3.75, 0.285, 0.05), rng.choice(["gv_wood", "gv_wood_lt", "gv_wood"]), T((rng.uniform(-0.03, 0.03), y + 0.15, P - 0.025)))
    # cabin
    cx, cy, cw, cd, ch = 0.3, 0.45, 2.7, 2.2, 1.9
    fy = cy - cd / 2      # front wall face y = -0.65
    k.add(box(cw, cd, ch), "gv_wall", T((cx, cy, P + ch / 2)))
    for i in range(10):   # vertical boards on the front
        x = cx - cw / 2 + (i + 0.5) * cw / 10
        if abs(x - 0.5) < 0.42:
            continue
        k.add(box(cw / 10 - 0.035, 0.03, ch - 0.05), rng.choice(["gv_wood", "gv_wood_lt"]), T((x, fy - 0.015, P + ch / 2)))
    for sx in (-1, 1):
        for sy in (-1, 1):
            k.add(box(0.14, 0.14, ch + 0.05), "gv_wood_dk", T((cx + sx * (cw / 2), cy + sy * (cd / 2), P + ch / 2)))
    # steep gable roof, ridge along Y (the gable faces the camera)
    rh, rw = 1.55, cw + 0.55
    k.add(prism([(-rw / 2, 0), (rw / 2, 0), (0, rh)], cd + 0.6), "gv_shingle", T((cx, cy, P + ch)))
    k.add(prism([(-cw / 2, 0), (cw / 2, 0), (0, rh * cw / rw)], 0.04), "gv_wall", T((cx, fy - 0.31 + 0.02, P + ch + 0.01)))
    for i in range(3):    # shingle courses on the slopes (front edge strips)
        z = P + ch + 0.25 + i * 0.42
        half = (rw / 2) * (1 - (z - P - ch) / rh)
        for sx in (-1, 1):
            beam(k, (cx + sx * (half + 0.02), fy - 0.32, z - 0.03), (cx + sx * (half + 0.02), cy + cd / 2 + 0.32, z - 0.03), 0.06, 0.05, "gv_shingle_dk")
    k.add(box(0.12, cd + 0.66, 0.1), "gv_shingle_dk", T((cx, cy, P + ch + rh + 0.02)))
    # round window in the gable, a square one on the right side, door with an arched top
    k.add(cyl(0.24, 0.24, 0.06, 12), "gv_wood_dk", T((cx, fy - 0.31, P + ch + 0.55), (90, 0, 0)))
    k.add(cyl(0.18, 0.18, 0.04, 12), "gv_window_glow", T((cx, fy - 0.33, P + ch + 0.55), (90, 0, 0)))
    k.add(box(0.36, 0.03, 0.03), "gv_wood_dk", T((cx, fy - 0.36, P + ch + 0.55)))
    k.add(box(0.03, 0.03, 0.36), "gv_wood_dk", T((cx, fy - 0.366, P + ch + 0.55)))
    k.add(box(0.08, 0.5, 0.5), "gv_wood_dk", T((cx + cw / 2 + 0.02, cy, P + 1.15)))
    k.add(box(0.04, 0.38, 0.38), "gv_window_glow", T((cx + cw / 2 + 0.05, cy, P + 1.15)))
    k.add(box(0.04, 0.42, 0.03), "gv_wood_dk", T((cx + cw / 2 + 0.07, cy, P + 1.15)))
    k.add(prism(arch_pts(0.82, 1.55), 0.06), "gv_wood_dk", T((0.5, fy - 0.036, P - 0.01)))
    k.add(prism(arch_pts(0.66, 1.42), 0.05), "gv_door", T((0.5, fy - 0.07, P)))
    k.add(sphere(0.04, 6, 4), "gv_honey", T((0.72, fy - 0.11, P + 0.75)))
    k.add(box(0.95, 0.4, 0.06), "gv_shingle_dk", T((0.5, fy - 0.2, P + 1.72), (-14, 0, 0)))
    # porch rail along the platform's edges, open where the ladder arrives (x 0 .. 1 at the front)
    posts = set()
    for j, ((x0, y0), (x1, y1)) in enumerate((((-1.85, -1.8), (-0.05, -1.8)), ((1.05, -1.8), (1.85, -1.8)), ((-1.85, -1.8), (-1.85, 1.0)),
                                              ((1.85, -1.8), (1.85, 1.75)))):
        n = max(1, round(math.hypot(x1 - x0, y1 - y0) / 0.6))
        for i in range(n + 1):
            posts.add((round(x0 + (x1 - x0) * i / n, 3), round(y0 + (y1 - y0) * i / n, 3)))
        beam(k, (x0, y0, P + 0.74 + 0.012 * j), (x1, y1, P + 0.74 + 0.012 * j), 0.08, 0.07, "gv_wood")
        beam(k, (x0, y0, P + 0.4 + 0.012 * j), (x1, y1, P + 0.4 + 0.012 * j), 0.05, 0.045, "gv_wood_lt")
    for x, y in sorted(posts):
        k.add(box(0.06, 0.06, 0.74), "gv_wood_dk", T((x, y, P + 0.36)))
    # lantern on the rail corner by the gap, stovepipe, a pulley with a bucket
    k.add(box(0.05, 0.05, 0.62), "gv_wood_dk", T((-0.05, -1.855, P + 0.98)))
    k.add(box(0.3, 0.045, 0.045), "gv_wood_dk", T((-0.15, -1.855, P + 1.27)))
    k.add(box(0.13, 0.13, 0.17), "lamp_glow", T((-0.25, -1.8, P + 1.08)))
    k.add(cyl(0.11, 0.0, 0.08, 4), "iron", T((-0.25, -1.8, P + 1.21), (0, 0, 45)))
    k.add(cyl(0.08, 0.08, 1.2, 8), "iron", T((1.25, 1.0, P + ch + 0.9)))
    k.add(cyl(0.11, 0.07, 0.12, 8), "iron", T((1.25, 1.0, P + ch + 1.55)))
    beam(k, (1.5, -1.4, P + 0.75), (2.05, -1.4, P + 1.2), 0.07, 0.07, "gv_wood_dk")
    k.add(cyl(0.006, 0.006, 2.4, 4), "rope", T((2.0, -1.4, P - 0.05)))
    k.add(cyl(0.12, 0.1, 0.18, 8), "gv_wood", T((2.0, -1.4, 1.75)))
    # the tree's crown over the roof
    for loc, r, m in (((-1.0, 1.2, 6.5), 1.05, "gv_elder"), ((-0.1, 1.4, 6.1), 0.8, "gv_elder_dk"), ((-1.6, 0.5, 6.0), 0.75, "gv_elder_dk"),
                      ((-0.6, 0.6, 6.95), 0.75, "gv_elder_lt"), ((-1.3, 1.6, 7.1), 0.6, "gv_elder_top"), ((0.5, 1.6, 6.7), 0.55, "gv_elder_lt")):
        k.add(jitter(ico(r, 2), rng, r * 0.08), m, T(loc, scale=(1, 1, 0.8)), smooth=True)
    fern(k, rng, 1.2, -1.0, 0.0, 1.1)
    fern(k, rng, -1.4, -1.2, 0.0, 0.9)
    shadow(k, 1.0, 2.0, 1.9)
    return chain(empty("light_origin", (-0.25, -1.8, P + 1.08)), empty("smoke_origin", (1.25, 1.0, P + ch + 1.65)))


LADDER_TOP = 3.1   # reaches a canopy deck or the treehouse porch (floor 3.1 / 3.0) from the ground


def build_ladder(k):
    """1 x 1 rope-lashed log ladder (3.1 m), foot at the front of the tile, leaning back (toward -Z, glTF) to hook
    over the edge of the 3 m deck behind it. A single piece: CLIMB (0.6 m per step) can't be met by stacking."""
    rng = random.Random(506)
    y0, y1 = -0.38, 0.47
    for sx in (-1, 1):
        log(k, (sx * 0.26, y0, -0.03), (sx * 0.26, y1, LADDER_TOP + 0.3), 0.045, "gv_wood_dk", 6)
    n = 9
    for i in range(n):
        t = (i + 0.6) / (n + 0.4)
        y, z = y0 + (y1 - y0) * t, -0.03 + (LADDER_TOP + 0.33) * t
        log(k, (-0.29, y, z), (0.29, y, z), 0.03, rng.choice(["gv_wood", "gv_wood_lt"]), 5)
        for sx in (-1, 1):
            k.add(cyl(0.055, 0.055, 0.04, 6), "rope", T((sx * 0.26, y, z), (math.degrees(math.atan2(y1 - y0, LADDER_TOP)), 0, 0)))
    k.add(cyl(0.12, 0.12, 0.05, 7), "gv_pebble", T((-0.26, y0, 0.0)))
    k.add(cyl(0.11, 0.11, 0.05, 7), "gv_pebble", T((0.26, y0, 0.0)))
    fern(k, rng, 0.33, -0.4, 0.0, 0.7, 5)
    shadow(k, 0.45, 1.0, 0.9)


def deck_planks(k, rng, top, n=5):
    for i in range(n):
        y = -0.4 + 0.2 * i
        k.add(box(1.0, 0.185, 0.045, 0.006), rng.choice(["gv_wood", "gv_wood_lt", "gv_wood"]), T((0, y, top - 0.0225)))


def build_canopy_deck(k):
    """1 x 1 walkway deck for the canopy (place at y 3; deck top 0.1 above its origin, planks along X like
    `boardwalk`): joists, one log post down to the ground (-3) with knee braces, a vine and a leaf tuft."""
    rng = random.Random(507)
    deck_planks(k, rng, 0.1)
    for x in (-0.32, 0.32):
        k.add(box(0.09, 1.0, 0.09), "gv_wood_dk", T((x, 0, 0.012)))
    log(k, (0.0, 0.0, -3.03), (0.0, 0.0, -0.06), 0.09, "gv_wood_dk")
    k.add(box(0.7, 0.12, 0.1), "gv_wood_dk", T((0, 0, -0.08)))
    for sx in (-1, 1):
        beam(k, (0, 0, -0.75), (sx * 0.32, 0, -0.1), 0.06 + 0.006 * sx, 0.06, "gv_wood_dk")
    k.add(cyl(0.16, 0.13, 0.1, 7), "gv_pebble", T((0, 0, -3.0)))
    vx = rng.choice((-0.45, 0.45))
    k.add(tube([(vx, -0.42, 0.06), (vx + 0.03, -0.44, -0.3), (vx - 0.02, -0.43, -0.65)], 0.018, 4, 0.5), "vine", smooth=True)
    for z in (-0.15, -0.45):
        k.add(sphere(0.045, 6, 4), "vine_lt", T((vx + 0.02, -0.46, z), scale=(1, 0.4, 0.7)), smooth=True)
    k.add(jitter(ico(0.08, 1), rng, 0.01), "gv_moss", T((rng.choice((-0.42, 0.42)), 0.42, 0.11), scale=(1.3, 1, 0.4)), smooth=True)
    contact(k, 0.25)
    for bm, *_ in k.parts[-2:]:  # the post's shadow on the ground, 3 m down
        import bmesh
        bmesh.ops.translate(bm, vec=(0, 0, -3.0), verts=bm.verts)


def build_rope_bridge(k):
    """1 x 1 rope bridge tile (place at y 3; deck top 0.1): slats across X with gaps, walked along Z (turn it 90
    for an east-west span); hand ropes on both sides sag between the tile's posts, so a run reads as one bridge."""
    rng = random.Random(508)
    for i in range(6):
        y = -0.42 + i * 0.168
        k.add(box(0.86, 0.13, 0.04, 0.006), rng.choice(["gv_wood", "gv_wood_lt", "gv_wood_dk"]),
              T((rng.uniform(-0.02, 0.02), y, 0.08), (0, 0, rng.uniform(-3, 3))))
    for sx in (-1, 1):
        k.add(cyl(0.012, 0.012, 1.0, 4), "rope", T((sx * 0.36, 0, 0.045), (90, 0, 0)))          # foot ropes under the slats
        k.add(box(0.06, 0.06, 0.95), "gv_wood_dk", T((sx * 0.45, -0.46, 0.5)))                  # one post per tile, front corners
        k.add(tube(curve((sx * 0.45, -0.5, 0.92), (sx * 0.45, 0.5, 0.92), 0.12, 6), 0.016, 4), "rope", smooth=True)
        k.add(tube(curve((sx * 0.45, -0.5, 0.52), (sx * 0.45, 0.5, 0.52), 0.07, 6), 0.012, 4), "rope", smooth=True)
        for j in range(1, 6, 2):  # drop ropes from the hand rope to the slats
            y = -0.5 + j / 6
            sag = 0.12 * 4 * (j / 6) * (1 - j / 6)
            k.add(cyl(0.008, 0.008, 0.82 - sag, 4), "rope", T((sx * 0.43, y, 0.1 + (0.82 - sag) / 2)))
    k.add(tube([(0.45, -0.46, 0.4), (0.47, -0.47, 0.1), (0.46, -0.45, -0.3)], 0.016, 4, 0.5), "vine", smooth=True)


def build_beehive(k):
    """1 x 1 straw skep on a stump: stepped coils of straw, an entrance with a honey drip, three bees."""
    rng = random.Random(509)
    k.add(cyl(0.2, 0.24, 0.3, 8), "gv_bark", T((0, 0, 0.13)))
    k.add(cyl(0.19, 0.19, 0.03, 8), "gv_ring", T((0, 0, 0.29)))
    prof, z = [], 0.3
    for r in (0.29, 0.29, 0.27, 0.235, 0.18, 0.1):
        prof += [(r, z), (r * 0.93, z + 0.085)]
        z += 0.09
    prof.append((0.0, z + 0.02))
    k.add(lathe(prof, 9), "gv_straw", smooth=False)
    k.add(cyl(0.25, 0.235, 0.03, 9), "gv_straw_dk", T((0, 0, 0.56)))   # a binding band
    k.add(prism(arch_pts(0.12, 0.09, 4), 0.06), "gv_bee_dk", T((0, -0.27, 0.31)))
    k.add(sphere(0.028, 6, 4), "gv_honey", T((0.07, -0.275, 0.305), scale=(1, 0.6, 1.4)))
    for i, (x, y, zz) in enumerate(((0.33, -0.22, 0.62), (-0.27, -0.35, 0.47), (0.12, -0.4, 0.86))):
        k.add(sphere(0.03, 6, 4), "gv_bee", T((x, y, zz), scale=(1.3, 0.9, 0.9)))
        k.add(sphere(0.018, 5, 3), "gv_bee_dk", T((x + 0.035, y, zz)))
        k.add(box(0.05, 0.004, 0.035), "gv_spot", T((x - 0.01, y, zz + 0.03), (0, 25, 0)))
    contact(k, 0.36)


def build_mushroom_ring(k):
    """1 x 1 fairy ring: chunky coral toadstools round a mossy circle, three small pale ones that glow at night."""
    rng = random.Random(510)
    k.add(cyl(0.4, 0.4, 0.02, 12), "gv_moss_dk", T((0, 0, 0.006)))
    n = 8
    for i in range(n):
        a = i / n * math.tau + rng.uniform(-0.12, 0.12)
        r = 0.34 + rng.uniform(-0.04, 0.04)
        glow = i in (1, 4, 6)
        toadstool(k, rng, r * math.cos(a), r * math.sin(a), rng.uniform(0.9, 1.15) if glow else rng.uniform(1.4, 2.1),
                  "gv_shroom_glow" if glow else rng.choice(["gv_cap", "gv_cap", "gv_cap_dk"]), spots=not glow)
    contact(k, 0.48)


PROPS = {
    "hedge": lambda k: build_hedge_piece(k, "WE", 521), "hedge_corner": lambda k: build_hedge_piece(k, "WS", 522),
    "hedge_end": lambda k: build_hedge_piece(k, "W", 523, top=1.21), "hedge_t": lambda k: build_hedge_piece(k, "WES", 524),
    "standing_tree": build_standing_tree, "great_root": build_great_root,
    "fallen_trunk": build_fallen_trunk, "fallen_trunk_end": build_fallen_trunk_end,
    "treehouse": build_treehouse, "ladder": build_ladder, "canopy_deck": build_canopy_deck, "rope_bridge": build_rope_bridge,
    "beehive": build_beehive, "mushroom_ring": build_mushroom_ring,
}

VIGNETTE = dict(
    ground=("moss", "dirt", "#5f9a45"),
    fence="hedge",
    ring=["pine", "birch", "pine", "bush_dark"],
    tiles=[
        "....................",
        "....................",
        "....................",
        "..............ffffff",
        "..............f..f..",
        "..............f.ffff",
        "..............f..f..",
        "...pppppppp...f..f.f",
        "..........p...ffff..",
        "..........ppppp.....",
        "....................",
        "....................",
    ],
    props=[
        ("fallen_trunk", 0, 0), ("fallen_trunk", 4, 0), ("fallen_trunk_end", 8, 0),
        ("great_root", 13, 0, 0),
        ("treehouse", 0, 3), ("ladder", 2, 7),
        ("canopy_deck", 4, 4, 0, 3), ("canopy_deck", 5, 4, 0, 3), ("rope_bridge", 6, 4, 90, 3), ("rope_bridge", 7, 4, 90, 3),
        ("canopy_deck", 8, 4, 0, 3), ("canopy_deck", 8, 5, 0, 3), ("ladder", 8, 6),
        ("standing_tree", 11, 4),
        ("beehive", 4, 9), ("beehive", 5, 10), ("mushroom_ring", 7, 9),
        ("hero", 9, 8), ("mushroom_ring", 12, 8), ("beehive", 3, 10),
    ],
    aim=(9.5, -6.0, 0), dist=18,
)
FOOT_OTHER = {}
