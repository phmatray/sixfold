"""Dressing props by the water (#233): a waterfall off a cliff ledge, lily pads, a village well.

Materials: fall_water is the falling sheet, water the pool's surface (rippling water shader); dw_foam is the
splash (never "water", so it stays opaque and merged); well_dark is the shaft's dark bottom (the well holds no
animated water, so it stays instanced).
"""
import math
import random

import bmesh
from mathutils import Vector

from v13_kit import B, T, beam, box, contact, curve, cyl, ico, jitter, lathe, prism, rock, shadow, torus, tube  # noqa: F401

PALETTE = {
    # wet rock, moss, ferns
    "dw_rock": "#8e959b", "dw_rock_lt": "#aab0b3", "dw_rock_dk": "#6e7780", "dw_rock_wet": "#58626c",
    "dw_moss": "#5f9a45", "dw_moss_dk": "#477d36", "dw_fern": "#76b450",
    # falling water, splash, pool
    "fall_water": "#c4ecf0", "dw_foam": "#f6fcfc", "dw_foam_sh": "#d9eff2", "water": "#5fb9c9",
    # lilies
    "dw_pad": "#55a043", "dw_pad_dk": "#3d8034", "dw_petal": "#fbf8f2", "dw_petal_pk": "#f3bfcd",
    "dw_lily_eye": "#f2c43a", "dw_bud": "#e98fab", "dw_stem": "#6aa64a",
    # the well
    "dw_stone": "#b8b0a2", "dw_stone_lt": "#cec7b8", "dw_stone_dk": "#9a9286", "dw_mortar": "#6c655d",
    "dw_cap": "#d8d1c2", "well_dark": "#131820", "dw_timber": "#8d5d39", "dw_timber_dk": "#6b4429",
    "dw_shingle": "#b8643f", "dw_shingle_lt": "#cc7a4f", "dw_shingle_dk": "#82432e", "dw_iron": "#4a4f57",
    "dw_bucket": "#b07a48", "dw_bucket_in": "#7a5232", "dw_grass": "#68a948",
}

FOOT = {"cliff_falls": (2, 1), "water_lilies": (1, 1), "well": (2, 2)}
WIND = {}
CLIPS = {}


# ---------------------------------------------------------------- helpers


def slab(pts, z0, z1, bottom=True):
    """A polygon (x, y) in plan, extruded from z0 to z1 (bottom=False leaves the underside open)."""
    bm = bmesh.new()
    lo = [bm.verts.new((x, y, z0)) for x, y in pts]
    hi = [bm.verts.new((x, y, z1)) for x, y in pts]
    if bottom:
        bm.faces.new(lo[::-1])
    bm.faces.new(hi)
    for i in range(len(pts)):   # pts counter-clockwise seen from above: these face outward
        j = (i + 1) % len(pts)
        bm.faces.new((lo[i], lo[j], hi[j], hi[i]))
    return bm


def grid(f, nu, nv, t):
    """A sheet of points f(u, v) (u, v in [0, 1]), given thickness t toward du x dv (bmesh's solidify is not
    deterministic: its face order changes run to run). Triangulated with fixed diagonals for the same reason."""
    bm = bmesh.new()
    e = 1e-3
    P = [[Vector(f(i / nu, j / nv)) for i in range(nu + 1)] for j in range(nv + 1)]
    N = [[(Vector(f(min(i / nu + e, 1), j / nv)) - Vector(f(max(i / nu - e, 0), j / nv))).cross(
          Vector(f(i / nu, min(j / nv + e, 1))) - Vector(f(i / nu, max(j / nv - e, 0)))).normalized()
          for i in range(nu + 1)] for j in range(nv + 1)]
    fr = [[bm.verts.new(P[j][i]) for i in range(nu + 1)] for j in range(nv + 1)]
    bk = [[bm.verts.new(P[j][i] + N[j][i] * t) for i in range(nu + 1)] for j in range(nv + 1)]
    for j in range(nv):
        for i in range(nu):
            bm.faces.new((fr[j][i], fr[j + 1][i], fr[j + 1][i + 1], fr[j][i + 1]))
            bm.faces.new((bk[j][i], bk[j][i + 1], bk[j + 1][i + 1], bk[j + 1][i]))
    rim = [(j, 0) for j in range(nv)] + [(nv, i) for i in range(nu)] + [(j, nu) for j in range(nv, 0, -1)] + [(0, i) for i in range(nu, 0, -1)]
    for k, (j, i) in enumerate(rim):
        j2, i2 = rim[(k + 1) % len(rim)]
        bm.faces.new((fr[j][i], bk[j][i], bk[j2][i2], fr[j2][i2]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bmesh.ops.triangulate(bm, faces=bm.faces[:], quad_method="FIXED", ngon_method="EAR_CLIP")
    return bm


def sector(r0, r1, a0, a1, z0, z1, n=2):
    """An annular wedge (capstones) from angle a0 to a1 (degrees)."""
    arc = lambda r: [(r * math.cos(math.radians(a0 + (a1 - a0) * i / n)), r * math.sin(math.radians(a0 + (a1 - a0) * i / n)))  # noqa: E731
                     for i in range(n + 1)]
    return slab(arc(r1) + arc(r0)[::-1], z0, z1)


def petal(L, w, lift):
    """A cupped petal from the origin along +X, its tip raised `lift` (6 triangles)."""
    bm = bmesh.new()
    b = bm.verts.new((0, 0, 0))
    le = bm.verts.new((L * 0.5, -w / 2, lift * 0.35))
    ri = bm.verts.new((L * 0.5, w / 2, lift * 0.35))
    tip = bm.verts.new((L, 0, lift))
    keel = bm.verts.new((L * 0.5, 0, lift * 0.35 - 0.01))
    for f in ((b, le, tip), (b, tip, ri), (b, keel, le), (b, ri, keel), (le, keel, tip), (keel, ri, tip)):
        bm.faces.new(f)
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return bm


def flower(k, x, y, z, rng, size=1.0):
    """A water lily bloom: white outer petals, pale pink inner ones, a yellow eye (~8 cm tall)."""
    s = size
    n = 7 if s >= 1 else 6
    for i in range(n):
        a = i / n * 360 + rng.uniform(-6, 6)
        k.add(petal(0.06 * s, 0.035 * s, 0.03 * s), "dw_petal", T((x, y, z), (0, 0, a)))
    for i in range(n - 2):
        a = i / (n - 2) * 360 + 25 + rng.uniform(-6, 6)
        k.add(petal(0.042 * s, 0.03 * s, 0.06 * s), "dw_petal_pk", T((x, y, z + 0.006), (0, 0, a)))
    k.add(cyl(0.016 * s, 0.012 * s, 0.022 * s, 5), "dw_lily_eye", T((x, y, z + 0.011 + 0.011 * s)))


# ---------------------------------------------------------------- cliff_falls


def build_cliff_falls(k):
    """2 x 1: water pouring off a 2 m ledge (the game's raised ground behind, at +Y) into a rock-ringed pool."""
    rng = random.Random(2331)
    # the falling sheet: over the lip at the top, then a parabola forward, narrowing, rippled across
    rows = [(0.5, 2.035), (0.42, 2.03), (0.35, 2.012)]
    n = 9
    for i in range(1, n + 1):
        d = 1.964 * (i / n) ** 1.4   # ends at 3.6 cm, inside the pool's water slab (2 to 5 cm)
        rows.append((0.33 - 0.22 * math.sqrt(d), 2.0 - d))

    def fall(u, v):
        j = v * (len(rows) - 1)
        j0 = min(int(j), len(rows) - 2)
        t = j - j0
        y = rows[j0][0] * (1 - t) + rows[j0 + 1][0] * t
        z = rows[j0][1] * (1 - t) + rows[j0 + 1][1] * t
        half = 0.56 - 0.1 * v
        x = (u - 0.5) * 2 * half
        # constant down the sheet, so each strip keeps one normal: clean vertical tonal bands in the toon light
        y -= 0.05 * math.sin(math.pi * u)            # bulges forward in the middle
        y += 0.03 * math.sin(u * math.pi * 4)        # ripples
        return Vector((x, y, z))
    k.add(grid(fall, 16, len(rows) - 1, 0.03), "fall_water")

    # the lip it pours over, and rocks flanking it at the top
    k.add(jitter(ico(0.3, 1), rng, 0.03), "dw_rock_wet", T((0, 0.37, 1.82), scale=(2.1, 0.42, 0.42)))
    for x, s, m in ((-0.74, 0.21, "dw_rock"), (0.73, 0.22, "dw_rock_dk"), (-0.6, 0.14, "dw_rock_dk"), (0.59, 0.13, "dw_rock")):
        k.add(rock(rng, s, 1), m, T((x, 0.47 - 0.6 * s * 1.1, 2.0 - s * 0.2), scale=(1.1, 0.6, 0.8)))
    k.add(jitter(ico(0.15, 1), rng, 0.015), "dw_moss", T((-0.74, 0.37, 2.12), scale=(1.1, 0.7, 0.3)), smooth=True)

    # the pool: an irregular basin of rocks round a water surface at 5 cm
    cy = -0.03
    pts = [(0.72 * (1 + rng.uniform(-0.08, 0.04)) * math.cos(a), cy + 0.33 * (1 + rng.uniform(-0.08, 0.04)) * math.sin(a))
           for a in (i / 16 * math.tau for i in range(16))]
    k.add(slab(pts, 0.02, 0.05), "water")
    for i, (x, y) in enumerate(pts):
        back = y > cy + 0.15
        r = rng.uniform(0.09, 0.13) * (1.5 if back else 1.0)
        rx, ry = x * 1.04, cy + (y - cy) * 1.08
        ry = min(max(ry, -0.35), 0.5 - r * 1.1)
        m = "dw_rock_wet" if back else rng.choice(["dw_rock", "dw_rock", "dw_rock_lt", "dw_rock_dk"])
        k.add(rock(rng, r, 0 if i % 2 else 1, 0.15), m, T((rx, ry, 0.0), scale=(1.35, 1.0, 0.55 if not back else 0.9)))
        if i % 5 == 2 and not back:
            k.add(jitter(ico(r * 0.8, 1), rng, 0.01), "dw_moss", T((rx, ry, 0.035), scale=(1.3, 1.0, 0.35)), smooth=True)
    # wet boulders at the foot of the wall, either side of the fall
    for x, s, m in ((-0.68, 0.26, "dw_rock_wet"), (0.7, 0.24, "dw_rock_dk"), (-0.5, 0.15, "dw_rock"), (0.52, 0.16, "dw_rock_wet")):
        k.add(rock(rng, s, 1), m, T((x, 0.5 - 0.7 * s * 1.12, s * 0.45), scale=(1.0, 0.7, 1.15)))
    k.add(jitter(ico(0.12, 1), rng, 0.01), "dw_moss_dk", T((0.7, 0.3, 0.5), scale=(1.2, 0.8, 0.35)), smooth=True)
    for x, y, z, yaw in ((-0.68, 0.3, 0.5, 30), (0.73, 0.4, 2.1, 0)):   # fern tufts
        for j in range(5):
            k.add(prism([(0, 0), (0.02, 0.1), (0.0, 0.21), (-0.02, 0.1)], 0.01), "dw_fern",
                  T((x, y, z), (0, 0, yaw + j * 72)) @ T((0, 0, 0), (0, -50, 0)))

    # foam where it lands, and a few flecks drifting forward
    for x, y, r in ((-0.3, 0.02, 0.13), (0.0, -0.02, 0.16), (0.3, 0.03, 0.13), (-0.15, -0.12, 0.1), (0.17, -0.11, 0.1),
                    (-0.45, -0.02, 0.08), (0.45, -0.01, 0.08)):
        k.add(jitter(ico(r, 1), rng, r * 0.18), rng.choice(["dw_foam", "dw_foam", "dw_foam_sh"]), T((x, y, 0.06), scale=(1.3, 1.0, 0.6)))
    for x, y in ((-0.35, -0.25), (0.1, -0.3), (0.42, -0.22), (-0.05, -0.2)):
        k.add(jitter(ico(0.05, 0), rng, 0.008), "dw_foam", T((x, y, 0.055), scale=(1.4, 1.0, 0.3)))
    shadow(k, 0.95, 1.04, 0.5)


# ---------------------------------------------------------------- water_lilies


def build_water_lilies(k):
    """1 x 1 on water: five notched pads in two greens, staggered in height, two blooms and a bud."""
    rng = random.Random(2332)
    pads = [(-0.18, -0.12, 0.17, 30), (0.2, -0.16, 0.14, 150), (0.05, 0.2, 0.15, 260), (-0.27, 0.22, 0.1, 340), (0.3, 0.17, 0.09, 80)]
    for i, (x, y, r, notch) in enumerate(pads):
        arc = [(x + r * math.cos(math.radians(notch + 16 + 328 * j / 9)), y + r * math.sin(math.radians(notch + 16 + 328 * j / 9)))
               for j in range(10)]
        z = 0.012 + 0.003 * i
        k.add(slab([(x, y)] + arc, z, z + 0.005, bottom=False), "dw_pad" if i % 2 == 0 else "dw_pad_dk")
    flower(k, -0.16, -0.1, 0.017, rng)
    flower(k, 0.06, 0.18, 0.023, rng, 0.8)
    k.add(cyl(0.007, 0.007, 0.07, 4), "dw_stem", T((0.33, -0.33, 0.015)))
    k.add(lathe([(0.0, 0.04), (0.02, 0.06), (0.016, 0.09), (0.0, 0.11)], 5), "dw_bud", T((0.33, -0.33, 0.0)))


# ---------------------------------------------------------------- well


def build_well(k):
    """2 x 2 village well: a drum of coursed stones under a capstone rim, two posts, a shingled roof, a winch with a
    crank and rope to a bucket resting on the rim; a dark shaft inside."""
    rng = random.Random(2333)
    k.add(lathe([(0.56, -0.02), (0.56, 0.8), (0.46, 0.8), (0.46, 0.1)], 16), "dw_mortar")
    k.add(cyl(0.44, 0.44, 0.01, 16), "well_dark", T((0, 0, 0.3)))
    nb = 11
    for c in range(4):                       # coursed stones, staggered
        z = 0.085 + c * 0.195
        for i in range(nb):
            a = (i + 0.5 * (c % 2)) / nb * 360 + rng.uniform(-3, 3)
            w = rng.uniform(0.28, 0.33)
            h = rng.uniform(0.16, 0.18)
            bm = jitter(box(w, 0.14, h), rng, 0.008)
            m = rng.choice(["dw_stone", "dw_stone", "dw_stone_lt", "dw_stone_dk"])
            k.add(bm, m, T((0, 0, 0), (0, 0, a - 90)) @ T((0, -(0.585 + rng.uniform(-0.01, 0.01)), z - (0.01 if c == 0 else 0))))
    cap_top = []
    for i in range(9):                       # capstone rim, heights stepped so neighbours never share a plane
        a0 = i / 9 * 360 + 0.8
        z0 = 0.765 + 0.012 * (i % 3)
        cap_top.append(z0 + 0.11)
        k.add(sector(0.43, 0.71 + rng.uniform(-0.01, 0.01), a0, a0 + 40 - 1.6, z0, z0 + 0.11), "dw_cap" if i % 3 else "dw_stone_lt")
    # posts on footing stones, the ridge beam, rafters
    for x in (-0.84, 0.84):
        k.add(box(0.13, 0.13, 2.12), "dw_timber", T((x, 0, 1.04)))
        k.add(rock(rng, 0.13, 1), "dw_stone_dk", T((x, 0, 0.02), scale=(1.4, 1.4, 0.55)))
        for s in (-1, 1):   # rafters, let 1 cm into the roof boards; the two of a pair differ in width
            beam(k, (x, 0, 2.1175), (x, s * 0.6, 2.1175 - 0.6 * math.tan(math.radians(31))), 0.07 if s > 0 else 0.04, 0.07, "dw_timber_dk")
    k.add(box(1.9, 0.09, 0.11), "dw_timber_dk", T((0, 0, 2.1)))
    # the roof: two plank slopes with staggered shingle rows
    pitch = 31
    for s in (-1, 1):
        R = T((0, 0, 2.17), (s * pitch, 0, 0))
        k.add(box(1.96 if s > 0 else 1.93, 0.8, 0.05), "dw_shingle_dk", R @ T((0, s * -0.4, 0)))
        for row in range(4):   # courses from the eave up, every other one offset half a shingle; each course's lower edge
            # is let ~1 cm into the course below it (resting exactly on it would z-fight)
            yy = s * -(0.71 - row * 0.2)
            lim = 0.95 - 0.012 * row
            for i in range(8):
                x = -0.98 + (0.14 if row % 2 == 0 else 0.0) + i * 0.28
                x0, x1 = max(x - 0.13, -lim), min(x + 0.13, lim)
                if x1 - x0 < 0.08:
                    continue
                m = rng.choice(["dw_shingle", "dw_shingle", "dw_shingle_lt"])
                t = (i + row % 2) % 3   # three tilts: no shingle lies parallel to one it overlaps or touches
                k.add(box(x1 - x0, 0.26, 0.03), m,
                      R @ T(((x0 + x1) / 2, yy, 0.05 - 0.006 * row + 0.003 * t), (s * -(3.5 + 2 * t), 0, rng.uniform(-2, 2))))
    k.add(box(2.0, 0.12, 0.07), "dw_shingle_dk", T((0, 0, 2.27), (45, 0, 0), (1, 1, 1)))
    # the winch: axle through the posts, a rope drum, a crank on +X
    k.add(cyl(0.04, 0.04, 1.91, 8), "dw_timber_dk", T((0.02, 0, 1.35), (0, 90, 0)))   # pegs out 3 cm past the post
    k.add(cyl(0.075, 0.075, 0.5, 10), "rope", T((0, 0, 1.35), (0, 90, 0)))
    for x in (-0.25, 0.25):
        k.add(cyl(0.1, 0.1, 0.03, 10), "dw_timber", T((x, 0, 1.35), (0, 90, 0)))
    k.add(box(0.035, 0.05, 0.3), "dw_iron", T((0.94, -0.06, 1.24), (-25, 0, 0)))
    k.add(cyl(0.022, 0.022, 0.1, 6), "dw_timber", T((0.95, -0.12, 1.11), (0, 90, 0)))
    # bucket resting on the rim, front right, roped up to the drum
    bx, by = 0.22, -0.53
    bz = cap_top[int((math.degrees(math.atan2(by, bx)) % 360 - 0.8) // 40)] - 0.012   # sunk 12 mm into its capstone
    k.add(lathe([(0.11, 0.0), (0.135, 0.2), (0.115, 0.2), (0.095, 0.03)], 10), "dw_bucket", T((bx, by, bz)))
    k.add(cyl(0.1, 0.1, 0.01, 10), "dw_bucket_in", T((bx, by, bz + 0.05)))
    for z, r in ((0.045, 0.116), (0.155, 0.13)):
        k.add(torus(r, 0.008, 10, 3), "dw_iron", T((bx, by, bz + z)))
    k.add(tube([(bx - 0.13 * math.cos(math.radians(a)), by, bz + 0.19 + 0.12 * math.sin(math.radians(a))) for a in range(0, 181, 30)],
               0.007, 4), "dw_iron")
    k.add(tube(curve((0.12, -0.07, 1.29), (bx, by, bz + 0.31), 0.05, 5), 0.012, 4), "rope")
    # tufts and pebbles round the foot
    for x, y, yaw in ((-0.55, -0.48, 10), (0.66, 0.4, 50), (-0.3, 0.68, -20)):
        for j in range(4):
            k.add(prism([(0, 0), (0.02, 0.0), (0.004, 0.2)], 0.008), "dw_grass", T((x, y, -0.01), (0, 0, yaw + j * 45)) @ T((0, 0, 0), (0, -18, 0)))
    for x, y in ((0.7, -0.45), (-0.72, 0.3), (0.25, 0.75)):
        k.add(rock(rng, 0.05, 0), "dw_stone_dk", T((x, y, 0.0), scale=(1.3, 1.0, 0.6)))
    contact(k, 0.95)


PROPS = {"cliff_falls": build_cliff_falls, "water_lilies": build_water_lilies, "well": build_well}
