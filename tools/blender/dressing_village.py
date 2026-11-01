"""Dressing (#233), the village: lantern_bunting, washing_line, flower_planter, vegetable_patch. Built by
build_dressing.py on the 1.3 kit (Z-up, front toward -Y, origin at the footprint centre, bases sunk 2 cm).

Glow parts: lantern_glow / lantern_glow_b (the bunting's paper lanterns).
"""
import math
import random

import bmesh
from mathutils import Vector

from v13_kit import T, along, box, clamp01, contact, cyl, ico, jitter, lathe, prism, torus, tube


def sphere(r, u=10, v=6):
    """A UV sphere as a lathe: the kit's sphere (bmesh create_uvsphere) lists its faces in a different order from run
    to run, which makes the .glb bytes differ; this one is built face by face, so exports stay byte-identical."""
    return lathe([(r * math.sin(math.pi * i / v), -r * math.cos(math.pi * i / v)) for i in range(v + 1)], u)


PALETTE = {
    # bunting
    "dv_post": "#8a5d3b", "dv_post_dk": "#6a452c", "dv_footing": "#a39d90", "dv_rope": "#c8a46a", "dv_rib": "#7a4a30",
    "lantern_glow": "#ffd88a", "lantern_glow_b": "#ff9a78", "dv_tassel": "#c0443a",
    "dv_pennant_a": "#e8574a", "dv_pennant_b": "#f2c14e", "dv_pennant_c": "#3fa7a0",
    # washing line
    "dv_peg": "#e3c48f", "dv_line": "#efe6d2", "dv_shirt": "#9cc6e6", "dv_shirt_dk": "#6f9fc8", "dv_button": "#fbf6ea",
    "dv_towel_a": "#f6efe0", "dv_towel_b": "#e9796a", "dv_sheet": "#eef2e4", "dv_sheet_hem": "#b9a3dc",
    "dv_sock_a": "#e9a23b", "dv_sock_b": "#e58fae", "dv_wicker": "#c99a5a", "dv_wicker_dk": "#a07440",
    # planter
    "dv_plank": "#b9824e", "dv_plank_dk": "#7a5132", "dv_soil": "#5a3f2c", "dv_leaf": "#4f9a3e", "dv_leaf_lt": "#6db34a",
    "dv_leaf_dk": "#3c7a33", "dv_coral": "#f07a5a", "dv_butter": "#f6d25a", "dv_lilac": "#b28be0", "dv_white": "#f7f2e6",
    "dv_heart": "#f2b33a", "dv_heart_dk": "#7a4a2a",
    # vegetable patch
    "dv_bed": "#a8763f", "dv_bed_dk": "#7a5530", "dv_earth": "#6b4b33", "dv_earth_dk": "#553b28", "dv_path": "#9a7b5c",
    "dv_stone": "#b3ab9c", "dv_cabbage": "#8dbf72", "dv_cabbage_dk": "#5f9a62", "dv_lettuce": "#9ccf52",
    "dv_lettuce_lt": "#c4e27a", "dv_carrot": "#ef8a2e", "dv_top": "#5aa040", "dv_onion": "#f1ead2", "dv_cane": "#cdb57c",
    "dv_twine": "#d9c08a", "dv_vine": "#4f8a3a", "dv_pod": "#86b84c", "dv_bloom": "#e0563f", "dv_can": "#4f9db0",
    "dv_can_dk": "#3a7a8a", "dv_scare_shirt": "#d0604a", "dv_scare_patch": "#5b8fc4", "dv_sack": "#d8c49a",
    "dv_straw": "#f0d27a", "dv_straw_dk": "#d1ad4f", "dv_hat_band": "#3f6f8a", "dv_stitch": "#3a2a20",
}


def shadow_at(k, x, y, r, sx=1.0, sy=1.0):
    """B.contact moved to (x, y), squashed to an ellipse."""
    contact(k, r)
    for bm, *_ in k.parts[-2:]:
        bmesh.ops.scale(bm, vec=(sx, sy, 1), verts=bm.verts)
        bmesh.ops.translate(bm, vec=(x, y, 0), verts=bm.verts)


def slab(fn, nu, nv, t=0.008, top=True, bottom=True, left=True, right=True):
    """A thin cloth: fn(u, v) -> (x, y, z) on its mid-surface (u across, v = 0 the top edge), t thick in Y. Edges can be
    left open where the cloth carries on in another colour, so no two faces ever lie on each other."""
    bm = bmesh.new()
    F, Bk = ([[bm.verts.new(Vector(fn(i / nu, j / nv)) + Vector((0, s * t / 2, 0))) for i in range(nu + 1)] for j in range(nv + 1)]
             for s in (-1, 1))
    for j in range(nv):
        for i in range(nu):
            bm.faces.new((F[j][i], F[j + 1][i], F[j + 1][i + 1], F[j][i + 1]))
            bm.faces.new((Bk[j][i], Bk[j][i + 1], Bk[j + 1][i + 1], Bk[j + 1][i]))
    for i, on in ((0, left), (nu, right)):
        if on:
            for j in range(nv):
                bm.faces.new((F[j][i], Bk[j][i], Bk[j + 1][i], F[j + 1][i]))
    for j, on in ((0, top), (nv, bottom)):
        if on:
            for i in range(nu):
                bm.faces.new((F[j][i], F[j][i + 1], Bk[j][i + 1], Bk[j][i]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    f0 = bm.faces[0]   # a front face: must look toward -Y
    f0.normal_update()
    if f0.normal.y > 0:
        for f in bm.faces:
            f.normal_flip()
    return bm


def frame(hx, hy, w, z0, z1):
    """A rectangular ring (one piece): outer half sizes hx, hy, board width w, from z0 to z1."""
    bm = bmesh.new()
    ring = lambda a, b, z: [bm.verts.new((sx * a, sy * b, z)) for sx, sy in ((-1, -1), (1, -1), (1, 1), (-1, 1))]  # noqa: E731
    oz0, oz1, iz0, iz1 = ring(hx, hy, z0), ring(hx, hy, z1), ring(hx - w, hy - w, z0), ring(hx - w, hy - w, z1)
    for i in range(4):
        j = (i + 1) % 4
        for a, b in ((oz0, oz1), (oz1, iz1), (iz1, iz0), (iz0, oz0)):
            bm.faces.new((a[i], a[j], b[j], b[i]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return bm


def cone_along(p, d, r, L, sides=4):
    """A cone from p (radius r) to a point L along d (straw tufts, carrot tops)."""
    return cyl(r, 0.0, L, sides), along(Vector(p) + Vector(d).normalized() * (L / 2), d)


# ---------------------------------------------------------------- lantern bunting (3 x 1, walked under)

BX, BZ, BSAG, BY = 1.40, 2.6, 0.42, 0.36   # post x, rope tie height, sag (lowest 2.18 m), offset toward the back


def bz(x):
    return BZ - BSAG * (1 - (x / BX) ** 2)


def paper_lantern(k, x, r, mat):
    h = 1.7 * r
    top = bz(x) - 0.07
    zc = top - h / 2
    k.add(tube([(x, 0, bz(x) + 0.004), (x, 0, top + 0.0125)], 0.005, 3), "dv_rope")
    prof = [(r * 0.42, -h / 2), (r * 0.82, -h * 0.38), (r, -h * 0.12), (r, h * 0.1), (r * 0.84, h * 0.34), (r * 0.42, h / 2)]
    k.add(lathe(prof, 10), mat, T((x, 0, zc)), smooth=True)
    for zz in (-h * 0.25, h * 0.22):   # bamboo ribs round the paper
        rr = r * (0.94 if zz < 0 else 0.95)
        k.add(torus(rr + 0.004, 0.006, 10, 3), "dv_rib", T((x, 0, zc + zz)))
    for s in (-1, 1):
        k.add(cyl(r * 0.46, r * 0.46, 0.05, 8), "dv_rib", T((x, 0, zc + s * h / 2)))
    k.add(cyl(0.032, 0.0, 0.09, 6), "dv_tassel", T((x, 0, zc - h / 2 - 0.05)))


def pennant(k, x, mat, rng):
    w, L = 0.062, rng.uniform(0.21, 0.24)
    tip = x + rng.uniform(-0.015, 0.015)
    k.add(prism([(x - w, bz(x - w) + 0.006), (tip, bz(x) - L), (x + w, bz(x + w) + 0.006)], 0.006), mat)


def build_lantern_bunting(k):
    """3 x 1, walked under: two slim timber posts on stone footings at the ends, a rope sagging between them (lowest
    2.18 m) hung with five round paper lanterns (amber and coral, bamboo ribs, red tassels) alternating with six
    pennants in three colours; ribbon tails where the rope is tied."""
    rng = random.Random(2331)
    for s in (-1, 1):
        x = s * BX
        k.add(box(0.18, 0.18, 0.13, 0.012), "dv_footing", T((x, 0, 0.045)))
        k.add(cyl(0.06, 0.05, 2.76, 8), "dv_post", T((x, 0, 1.34)), smooth=True)
        k.add(cyl(0.072, 0.072, 0.05, 8), "dv_post_dk", T((x, 0, 2.735)))
        k.add(sphere(0.05, 8, 5), "dv_post_dk", T((x, 0, 2.79)), smooth=True)
        k.add(torus(0.062, 0.014, 8, 4), "dv_rope", T((x, 0, BZ)))
        for i, m in enumerate(("dv_pennant_a", "dv_pennant_c")):   # ribbon tails from the tie
            dx = -s * 0.055
            k.add(tube([(x + dx, -0.03 + 0.06 * i, BZ - 0.01), (x + dx * 1.3, -0.035 + 0.07 * i, BZ - 0.16),
                        (x + dx * 1.1, -0.03 + 0.06 * i, BZ - 0.3 - 0.04 * i)], 0.012, 3), m)
        shadow_at(k, x, 0, 0.1)
    xs = [-BX + 0.06 + i * (2 * BX - 0.12) / 16 for i in range(17)]
    k.add(tube([(x, 0, bz(x)) for x in xs], 0.011, 6), "dv_rope", smooth=True)
    pen = ("dv_pennant_a", "dv_pennant_b", "dv_pennant_c")
    sizes = (0.13, 0.14, 0.15, 0.14, 0.13)
    for i in range(11):
        x = -BX + (i + 1) * 2 * BX / 12
        if i % 2:
            paper_lantern(k, x, sizes[i // 2], "lantern_glow" if (i // 2) % 2 == 0 else "lantern_glow_b")
        else:
            pennant(k, x, pen[(i // 2) % 3], rng)
    # The hero walks the row's middle (and along it, from tile to tile): the posts stand toward its back edge.
    for bm, *_ in k.parts:
        bmesh.ops.translate(bm, vec=(0, BY, 0), verts=bm.verts)


def bunting_wind(x, y, z, r):
    if z < 1.0:
        return 0.0
    return clamp01((BX - 0.08 - abs(x)) / 0.45) * (0.2 + 0.8 * clamp01((bz(x) - z) / 0.3))


# ---------------------------------------------------------------- washing line (3 x 1)

WX, WZ, WSAG, WY = 1.3, 1.80, 0.07, 0.17   # post x, tie height, sag, lines at y = -WY (front) and +WY


def wz(x):
    return WZ - WSAG * (1 - (x / WX) ** 2)


def hang(k, x0, x1, yl, h, bands, bow=0.05, nu=4, nv=3, hem=0.03, t=0.012):
    """A cloth pegged along a line: x0..x1, h deep, bellied toward the front; bands [(v0, v1, mat)] top to bottom."""
    hv = bands[-1][0]   # the hem wave stays in the last band, so the bands meet on a straight seam

    def fn(u, v):
        x = x0 + (x1 - x0) * u
        z = wz(x) + 0.006 - v * h - h * hem * math.sin(u * math.pi * 3) * max(0.0, (v - hv) / (1 - hv))
        y = yl - bow * math.sin(math.pi * u) * v + 0.01 * math.sin(u * math.pi * 2) * v
        return (x, y, z)
    for v0, v1, mat in bands:
        n = max(1, round(nv * (v1 - v0)))
        k.add(slab(lambda u, vv, v0=v0, v1=v1: fn(u, v0 + (v1 - v0) * vv), nu, n, t, top=v0 == 0, bottom=v1 == 1), mat)
    return fn


def pegs(k, yl, xs, rng):
    for x in xs:
        k.add(box(0.022, 0.038, 0.07, 0.004), "dv_peg", T((x, yl, wz(x) - 0.012), (0, rng.uniform(-8, 8), 0)))


def build_washing_line(k):
    """3 x 1: two forked clothes-prop poles, rough and leaning out a little, each splitting at the top into a Y of two
    tines (spread along Y, nothing crosswise anywhere); a line runs from tine to tine on each side, at ~1.75 m. On the
    front line a blue shirt hung by its shoulders, a pair of odd socks and a striped towel; on the back line a big sheet
    with a lilac hem. Wooden pegs; a wicker basket of folded washing by the right pole."""
    rng = random.Random(2332)
    for s in (-1, 1):
        x = s * WX
        k.add(tube([(s * 1.23, 0.01, -0.05), (s * 1.25, -0.012, 0.5), (s * 1.27, 0.012, 1.0), (x, 0, 1.46)], 0.05, 6, 0.75),
              "dv_post", smooth=True)
        k.add(sphere(0.042, 7, 4), "dv_post_dk", T((x, 0, 1.46)), smooth=True)
        for sy in (-1, 1):   # the fork's tines, each holding one line just under its tip
            k.add(tube([(x, sy * 0.015, 1.46), (x, sy * 0.1, 1.6), (x, sy * 0.17, 1.78), (x, sy * 0.19, 1.87)], 0.03, 5, 0.65),
                  "dv_post", smooth=True)
            k.add(sphere(0.02, 6, 4), "dv_line", T((x, sy * WY, WZ)), smooth=True)
        shadow_at(k, s * 1.23, 0, 0.1)
    xs = [-WX + i * 2 * WX / 14 for i in range(15)]
    for yl in (-WY, WY):
        k.add(tube([(x, yl, wz(x)) for x in xs], 0.007, 4), "dv_line")
    # front line: shirt (body + short sleeves), socks, towel
    yl = -WY
    bx0, bx1, top = -0.95, -0.55, lambda x: wz(x) + 0.006
    fn = hang(k, bx0, bx1, yl, 0.55, [(0, 0.86, "dv_shirt"), (0.86, 1, "dv_shirt_dk")], bow=0.04)
    for side in (-1, 1):
        ex = bx1 if side > 0 else bx0

        def sleeve(u, v, ex=ex, side=side):
            uu = u if side > 0 else 1 - u
            return (ex + side * 0.17 * uu, yl - 0.008 * v, top(ex) - 0.11 * uu - 0.17 * v)
        k.add(slab(sleeve, 2, 1, left=side < 0, right=side > 0), "dv_shirt")
    for v in (0.22, 0.44, 0.66):
        p = Vector(fn(0.5, v))
        k.add(sphere(0.011, 6, 4), "dv_button", T((p.x, p.y - 0.009, p.z)))
    pegs(k, yl, (bx0 + 0.03, bx1 - 0.03), rng)
    for xs_, m, flip in ((-0.27, "dv_sock_a", 1), (-0.09, "dv_sock_b", 1)):
        z0 = top(xs_)
        out = [(-0.045, 0), (0.045, 0), (0.045, -0.17), (0.12, -0.2), (0.125, -0.25), (0.06, -0.265), (0.0, -0.25), (-0.045, -0.21)]
        k.add(prism([(xs_ + flip * px, z0 + pz) for px, pz in out], 0.012), m, T((0, yl, 0)))
        pegs(k, yl, (xs_,), rng)
    hang(k, 0.12, 0.56, yl, 0.6, [(0, 0.14, "dv_towel_a"), (0.14, 0.26, "dv_towel_b"), (0.26, 0.42, "dv_towel_a"),
                                  (0.42, 0.58, "dv_towel_b"), (0.58, 0.74, "dv_towel_a"), (0.74, 0.86, "dv_towel_b"),
                                  (0.86, 1, "dv_towel_a")], bow=0.05, nv=7)
    pegs(k, yl, (0.15, 0.53), rng)
    pegs(k, yl, (0.78, 0.84), rng)   # two spare pegs
    # back line: the sheet
    hang(k, -0.55, 0.78, WY, 0.88, [(0, 0.88, "dv_sheet"), (0.88, 1, "dv_sheet_hem")], bow=0.07, nu=6, nv=4, hem=0.04, t=0.016)
    pegs(k, WY, (-0.52, 0.11, 0.75), rng)
    # basket of folded washing
    bxc, byc = 0.82, -0.1
    k.add(lathe([(0.17, -0.025), (0.215, 0.12), (0.235, 0.24), (0.21, 0.245), (0.19, 0.11), (0.0, 0.07)], 12),
          "dv_wicker", T((bxc, byc, 0), scale=(1, 0.72, 1)))
    for zz in (0.07, 0.17):
        k.add(torus(0.205 + zz * 0.12, 0.012, 12, 3), "dv_wicker_dk", T((bxc, byc, zz), scale=(1, 0.72, 1)))
    k.add(torus(0.225, 0.016, 12, 4), "dv_wicker_dk", T((bxc, byc, 0.245), scale=(1, 0.72, 1)))
    for s in (-1, 1):
        k.add(torus(0.05, 0.011, 8, 3), "dv_wicker_dk", T((bxc + s * 0.225, byc, 0.25), (90, 0, 90)))
    for i, (m, dz, rot) in enumerate((("dv_towel_b", 0.17, 8), ("dv_shirt", 0.205, -12), ("dv_sheet", 0.24, 20))):
        k.add(box(0.24 - i * 0.03, 0.17 - i * 0.02, 0.05, 0.01), m, T((bxc - 0.01 + i * 0.015, byc + 0.005, dz), (0, 0, rot)))
    shadow_at(k, bxc, byc, 0.3, 1.0, 0.75)


def washing_wind(x, y, z, r):
    if z < 0.6 or abs(x) > 1.2:
        return 0.0
    return clamp01((wz(x) - z) / 0.75) * clamp01((1.2 - abs(x)) / 0.1)


# ---------------------------------------------------------------- flower planter (1 x 1)

PW, PD, PH, PP = 0.8, 0.5, 0.4, 0.075   # box width (X), depth (Y), plank height, corner post size
SOIL = 0.37


def daisy(k, p, n, petal, heart, r=0.05):
    star = [((r if i % 2 == 0 else r * 0.45) * math.cos(i * math.pi / 6), (r if i % 2 == 0 else r * 0.45) * math.sin(i * math.pi / 6))
            for i in range(12)]
    M = along(p, n)
    k.add(prism(star, 0.012), petal, M @ T(rot=(90, 0, 0)))
    k.add(sphere(r * 0.38, 6, 4), heart, M @ T((0, 0, 0.008), scale=(1, 1, 0.7)), smooth=True)


def stem(k, a, b, r=0.007):
    k.add(tube([a, b], r, 4, 0.8, cap=False), "dv_leaf_dk")


def build_flower_planter(k):
    """1 x 1: a wooden planter (0.8 x 0.5 x 0.42 m) of light planks between darker corner posts, a dark rim, raised on
    the posts' feet; heaped with leafy mounds, coral and white daisies, butter-yellow pompoms and three lilac spires
    (tallest ~0.75 m), with trailing greenery spilling over the front edge."""
    rng = random.Random(2333)
    for sx in (-1, 1):
        for sy in (-1, 1):
            k.add(box(PP, PP, 0.445, 0.008), "dv_plank_dk", T((sx * (PW / 2 - PP / 2), sy * (PD / 2 - PP / 2), 0.2025)))
    t = 0.03
    for j in range(3):
        z0 = 0.05 + j * 0.12
        zc = z0 + 0.0525
        for sy in (-1, 1):
            k.add(box(PW - 2 * PP + 0.06, t, 0.105), "dv_plank", T((0, sy * (PD / 2 - 0.03), zc), (0, rng.uniform(-0.3, 0.3), 0)))
        for sx in (-1, 1):
            k.add(box(t, PD - 2 * PP + 0.06, 0.105), "dv_plank", T((sx * (PW / 2 - 0.03), 0, zc)))
    k.add(frame(PW / 2 + 0.012, PD / 2 + 0.012, 0.072, 0.41, 0.45), "dv_plank_dk")   # the rim, on the posts
    ix, iy = PW / 2 - 0.057, PD / 2 - 0.057
    k.add(box(2 * ix, 2 * iy, SOIL - 0.05), "dv_soil", T((0, 0, (SOIL + 0.05) / 2)))
    # leafy mounds
    for x, y, r in ((-0.26, 0.07, 0.11), (-0.06, 0.1, 0.12), (0.17, 0.08, 0.11), (0.28, -0.06, 0.09), (-0.2, -0.08, 0.09),
                    (0.05, -0.09, 0.1)):
        k.add(jitter(ico(r, 1), rng, r * 0.15), rng.choice(("dv_leaf", "dv_leaf_lt", "dv_leaf_dk")),
              T((x, y, SOIL + 0.03), (0, 0, rng.uniform(0, 90)), (1.15, 0.95, 0.62)), smooth=True)
    # three lilac spires at the back
    for x in (-0.22, 0.02, 0.24):
        h = rng.uniform(0.3, 0.38)
        base = Vector((x + rng.uniform(-0.02, 0.02), 0.12, SOIL))
        stem(k, base, base + Vector((0, 0, h - 0.1)))
        prof = [(0.0, 0.0)] + [((0.036 if i % 2 else 0.028) * (1 - i / 8), 0.012 + i * 0.022) for i in range(8)] + [(0.0, 0.2)]
        k.add(lathe(prof, 6, rng.uniform(0, 30)), "dv_lilac", T(base + Vector((0, 0, h - 0.17))), smooth=True)
    # daisies facing the front
    for x, y, h, petal in ((-0.3, -0.02, 0.2, "dv_white"), (-0.13, 0.03, 0.26, "dv_coral"), (0.08, 0.0, 0.24, "dv_white"),
                           (0.3, 0.04, 0.22, "dv_coral"), (-0.02, -0.12, 0.15, "dv_coral"), (0.2, -0.12, 0.16, "dv_white"),
                           (-0.22, -0.13, 0.14, "dv_white")):
        a = Vector((x, y, SOIL))
        b = Vector((x + rng.uniform(-0.03, 0.03), y - 0.03, SOIL + h))
        stem(k, a, b)
        daisy(k, b, (rng.uniform(-0.3, 0.3), -0.7, 0.75), petal, "dv_heart" if petal == "dv_white" else "dv_heart_dk",
              rng.uniform(0.045, 0.055))
    # butter-yellow pompoms
    for x, y, h in ((-0.33, 0.12, 0.19), (0.12, 0.13, 0.2), (0.33, -0.1, 0.12), (-0.08, -0.05, 0.18), (0.22, 0.12, 0.17)):
        a = Vector((x, y, SOIL))
        b = a + Vector((rng.uniform(-0.02, 0.02), 0, h))
        stem(k, a, b)
        k.add(jitter(sphere(0.038, 7, 5), rng, 0.004), "dv_butter", T(b), smooth=True)
    # trailing greenery over the front edge
    yf = -(PD / 2) - 0.032   # clear of the rim's front face
    for x, drop in ((-0.27, 0.22), (-0.09, 0.3), (0.1, 0.25), (0.28, 0.18)):
        pts = [(x, -0.15, SOIL + 0.02), (x + 0.01, -0.235, 0.468), (x + 0.02, yf, 0.43), (x + 0.04, yf - 0.004, 0.43 - drop * 0.5),
               (x + 0.03, yf - 0.01, 0.43 - drop)]
        k.add(tube(pts, 0.012, 4, 0.7), "dv_leaf_dk", smooth=True)
        for i, q in enumerate(pts[1:]):
            q = Vector(q)
            k.add(sphere(0.048 - i * 0.006, 6, 3), "dv_leaf_lt" if i % 2 else "dv_leaf",
                  T((q.x + (0.025 if i % 2 else -0.025), q.y - 0.012, q.z - 0.01), (0, 0, rng.uniform(0, 60)), (1.25, 0.55, 0.95)), smooth=True)
    for x, y in ((-0.17, -0.2), (0.19, -0.19)):   # leaves heaped over the front rim
        k.add(jitter(ico(0.08, 1), rng, 0.012), "dv_leaf_lt", T((x, y, SOIL + 0.065), (0, 0, rng.uniform(0, 90)), (1.3, 1.0, 0.62)), smooth=True)
    shadow_at(k, 0, 0, 0.5, 1.0, 0.72)


def planter_wind(x, y, z, r):
    return clamp01((z - 0.43) / 0.3)


# ---------------------------------------------------------------- vegetable patch (3 x 2)

VL = 1.43          # bed half length (X)
VY0, VY1 = 0.23, 0.93   # bed outer edges (|y|)
VT = 0.035         # board thickness
VS = 0.19          # soil top
SCX, SCY = -0.35, 0.64   # scarecrow
WGX, WGY = 1.08, 0.6     # bean wigwam


def raised_bed(k, sy, rng):
    """One bed edged with two stacked boards a side, corner stakes standing 1.5 cm proud; y from sy*VY0 to sy*VY1."""
    ya, yb = sorted((sy * VY0, sy * VY1))
    for z0, z1, m in ((-0.02, 0.1, "dv_bed"), (0.112, 0.225, "dv_bed")):
        zc, hz = (z0 + z1) / 2, z1 - z0
        for y in (ya + VT / 2, yb - VT / 2):
            k.add(box(2 * VL - 0.07, VT, hz), m, T((0, y, zc), (rng.uniform(-0.4, 0.4), 0, 0)))
        for x in (-VL + VT / 2, VL - VT / 2):
            k.add(box(VT, (yb - ya) - 0.07, hz), m, T((x, (ya + yb) / 2, zc)))
    for x in (-VL, VL):
        for y in (ya, yb):
            cx = x + (0.0225 if x < 0 else -0.0225)
            cy = y + (0.0225 if y == ya else -0.0225)
            k.add(box(0.075, 0.075, 0.29), "dv_bed_dk", T((cx, cy, 0.1)))
    g = VT + 0.012
    k.add(box(2 * (VL - g), (yb - ya) - 2 * g, VS + 0.032), "dv_earth", T((0, (ya + yb) / 2, (VS - 0.032) / 2)))
    for y in ((ya + yb) / 2,):   # a hoed furrow between the rows
        k.add(cyl(0.035, 0.035, 2 * (VL - g) - 0.04, 6), "dv_earth_dk", T((0, y, VS - 0.012), (0, 90, 0), (1, 1, 1)))


def cabbage(k, rng, x, y):
    k.add(jitter(sphere(0.12, 6, 4), rng, 0.008), "dv_cabbage", T((x, y, VS + 0.1), scale=(1, 1, 0.85)), smooth=True)
    for i in range(3):
        a = i * 120 + rng.uniform(0, 40)
        k.add(sphere(0.125, 5, 3), "dv_cabbage_dk",
              T((x, y, VS + 0.03), (0, 0, a)) @ T((0.105, 0, 0.04), (0, -55, 0), (1.0, 1.15, 0.32)), smooth=True)


def carrot(k, rng, x, y):
    k.add(cyl(0.024, 0.017, 0.045, 5), "dv_carrot", T((x, y, VS + 0.006)))
    for i in range(3):
        a = i * 120 + rng.uniform(-20, 20)
        h = rng.uniform(0.24, 0.3)
        k.add(prism([(-0.03, 0.0), (0.03, 0.0), (0.0, h)], 0.006), "dv_top",
              T((x, y, VS + 0.02), (0, 0, a)) @ T(rot=(0, 18, 0)))


def lettuce(k, rng, x, y):
    k.add(jitter(sphere(0.135, 7, 4), rng, 0.014), "dv_lettuce", T((x, y, VS + 0.04), (0, 0, rng.uniform(0, 45)), (1, 1, 0.55)), smooth=True)
    k.add(jitter(sphere(0.075, 5, 3), rng, 0.007), "dv_lettuce_lt", T((x, y, VS + 0.1), scale=(1, 1, 0.8)), smooth=True)


def onion(k, rng, x, y):
    k.add(sphere(0.03, 5, 3), "dv_onion", T((x, y, VS + 0.012)), smooth=True)
    for i in range(2):
        a = i * 180 + rng.uniform(-25, 25)
        h = rng.uniform(0.2, 0.28)
        k.add(prism([(-0.009, 0.0), (0.009, 0.0), (0.0, h)], 0.009), "dv_top", T((x, y, VS + 0.03), (0, 0, a)) @ T(rot=(0, 10, 0)))


def wigwam(k, rng):
    """Three canes lashed at the top, a bean vine climbing each with leaves, pods and red flowers."""
    apex = Vector((WGX, WGY, 1.42))
    for i, a in enumerate((100, 220, 340)):
        base = Vector((WGX + 0.2 * math.cos(math.radians(a)), WGY + 0.17 * math.sin(math.radians(a)), VS - 0.06))
        d = (apex - base).normalized()
        k.add(tube([base, apex + d * 0.12], 0.016, 5, 0.8), "dv_cane")
        side = d.cross(Vector((0, 0, 1))).normalized()
        nrm = side.cross(d).normalized()
        pts = []
        for j in range(7):
            t = 0.08 + j * 0.12
            ph = j * 1.7 + i
            pts.append(base.lerp(apex, t) + (side * math.cos(ph) + nrm * math.sin(ph)) * 0.028)
        k.add(tube(pts, 0.011, 3), "dv_vine")
        for j, q in enumerate(pts[1:]):
            out = (q - base.lerp(apex, 0.08 + (j + 1) * 0.12)).normalized() + Vector((0, 0, -0.3))
            k.add(prism([(0, 0), (0.06, 0.055), (0.14, 0), (0.06, -0.055)], 0.006), "dv_leaf" if j % 2 else "dv_leaf_lt",
                  along(q, out) @ T(rot=(0, 0, rng.uniform(0, 180))) @ T(rot=(0, -90, 0)))
            if j in (1, 3):
                k.add(cyl(0.011, 0.005, 0.1, 4), "dv_pod", T((q.x, q.y, q.z - 0.05)))
            elif j in (2, 4):
                k.add(sphere(0.018, 5, 3), "dv_bloom", T(q + out.normalized() * 0.02))
    k.add(sphere(0.032, 6, 4), "dv_twine", T(apex))


def scarecrow(k, rng):
    """A straw figure on a leaning pole: sack head with button eyes and a stitched smile under a straw hat, a patched
    red shirt over a straw skirt, one arm drooping and one bent up in a wave, straw at the cuffs."""
    M = T((SCX, SCY, 0), (0, 4, 0))
    P = lambda x, y, z: M @ Vector((x, y, z))  # noqa: E731
    k.add(cyl(0.024, 0.024, 1.12), "dv_cane", M @ T((0, 0, 0.66)))
    body = lathe([(0.13, 0.74), (0.17, 0.84), (0.18, 1.0), (0.16, 1.11), (0.07, 1.19)], 10)
    k.add(body, "dv_scare_shirt", M @ T(scale=(1, 0.65, 1)), smooth=True)
    yf = -0.18 * math.cos(math.radians(18)) * 0.65
    k.add(box(0.085, 0.024, 0.085), "dv_scare_patch", M @ T((0.06, yf - 0.0, 0.95), (0, 8, 0)))
    for i in range(8):   # straw skirt
        a = math.radians(i * 45 + 20)
        p = P(0.12 * math.cos(a), 0.08 * math.sin(a), 0.77)
        d = M.to_3x3() @ Vector((0.35 * math.cos(a), 0.25 * math.sin(a), -1))
        bm, Mc = cone_along(p, d, 0.032, rng.uniform(0.14, 0.19))
        k.add(bm, "dv_straw" if i % 2 else "dv_straw_dk", Mc)
    arms = ([(-0.13, 0, 1.09), (-0.3, -0.01, 1.0), (-0.47, -0.03, 0.87)],
            [(0.13, 0, 1.09), (0.29, -0.02, 0.99), (0.4, -0.05, 1.06), (0.45, -0.07, 1.2)])
    for pts in arms:
        wp = [P(*p) for p in pts]
        k.add(tube(wp, 0.05, 5, 0.8), "dv_scare_shirt", smooth=True)
        d = (wp[-1] - wp[-2]).normalized()
        side = d.cross(Vector((0, 1, 0))).normalized()
        for j in range(4):
            s = (j - 1.5) * 0.35
            bm, Mc = cone_along(wp[-1] - d * 0.01, d + side * s + Vector((0, -0.1 * (j % 2), 0)), 0.018, 0.11)
            k.add(bm, "dv_straw" if j % 2 else "dv_straw_dk", Mc)
    k.add(torus(0.07, 0.014, 8, 3), "dv_twine", M @ T((0, 0, 1.185)))
    hc, ha, hb, hcz = Vector((0, 0, 1.3)), 0.13, 0.12, 0.135
    k.add(sphere(1.0, 9, 6), "dv_sack", M @ T(hc, scale=(ha, hb, hcz)), smooth=True)

    def on_head(sx, sz, out=0.0):
        dd = Vector((sx, -1, sz)).normalized()
        n = Vector((dd.x / ha, dd.y / hb, dd.z / hcz)).normalized()
        return M @ (hc + Vector((dd.x * ha, dd.y * hb, dd.z * hcz)) + n * out), M.to_3x3() @ n
    for sx in (-0.38, 0.38):
        p, n = on_head(sx, 0.12)
        k.add(cyl(0.022, 0.022, 0.016, 8), "dv_stitch", along(p, n))
    mouth = [on_head(-0.42 + i * 0.12, -0.3 + 0.35 * (-0.42 + i * 0.12) ** 2, 0.003) for i in range(8)]
    k.add(tube([p for p, _ in mouth], 0.005, 3), "dv_stitch")
    for p, n in mouth[1:-1:2]:
        k.add(box(0.006, 0.034, 0.008), "dv_stitch", along(p, n))
    H = M @ T((0, 0, 1.4), (-5, -7, 0)) @ T((0, 0, -1.4))   # tilted about the brim, not the ground
    k.add(cyl(0.23, 0.22, 0.03, 10), "dv_straw", H @ T((0, 0, 1.40)))
    k.add(lathe([(0.12, 1.40), (0.115, 1.47), (0.095, 1.52), (0.0, 1.535)], 10), "dv_straw_dk", H)
    k.add(cyl(0.131, 0.131, 0.03, 10), "dv_hat_band", H @ T((0, 0, 1.445)))
    for sx in (-1, 1):   # straw poking out under the hat
        for j in range(2):
            bm, Mc = cone_along(P(sx * 0.11, 0.02 * j, 1.36 - 0.04 * j), Vector((sx, -0.2 + 0.3 * j, -0.6)), 0.016, 0.09)
            k.add(bm, "dv_straw", Mc)


def watering_can(k, x, y):
    k.add(lathe([(0.07, 0.012), (0.075, 0.15), (0.05, 0.175), (0.0, 0.18)], 8), "dv_can", T((x, y, 0)))
    k.add(tube([(x + 0.05, y, 0.05), (x + 0.12, y, 0.12), (x + 0.2, y, 0.2)], 0.013, 5, 0.8), "dv_can_dk")
    k.add(cyl(0.022, 0.03, 0.03, 8), "dv_can_dk", along(Vector((x + 0.2, y, 0.2)), (1, 0, 1)))
    k.add(tube([(x - 0.075, y, 0.12), (x - 0.06, y, 0.23), (x + 0.03, y, 0.22), (x + 0.04, y, 0.17)], 0.011, 4), "dv_can_dk")


def build_vegetable_patch(k):
    """3 x 2: two plank-edged raised beds along X with a trodden path and stepping stones between them. Front bed:
    cabbages, then carrot tops; back bed: lettuces, a row of spring onions, a scarecrow standing in it and a
    bean-cane wigwam with climbing beans in the back right corner. A watering can on the path."""
    rng = random.Random(2334)
    for sy in (-1, 1):
        raised_bed(k, sy, rng)
        shadow_at(k, 0, sy * 0.58, 1.0, 1.5, 0.42)
        k.parts.pop(-2)[0].free()   # the inner disc lies wholly under the bed: keep only the ring
    k.add(box(2 * VL + 0.02, 2 * VY0 - 0.06, 0.046), "dv_path", T((0, 0, 0.003)))
    for x in (-0.55, 0.35):
        k.add(jitter(cyl(0.1, 0.09, 0.03, 7), rng, 0.008, 0.2), "dv_stone", T((x, rng.uniform(-0.03, 0.03), 0.03), (0, 0, rng.uniform(0, 50))))
    for x in (-1.05, -0.35, 0.35, 1.05):
        cabbage(k, rng, x + rng.uniform(-0.03, 0.03), -0.69)
    for i in range(6):
        carrot(k, rng, -1.15 + i * 0.46 + rng.uniform(-0.03, 0.03), -0.43)
    for x in (-1.12, -0.6, -0.08, 0.44):
        lettuce(k, rng, x + rng.uniform(-0.03, 0.03), 0.42)
    for x in (-1.15, -0.95, 0.2):
        onion(k, rng, x, 0.75)
    wigwam(k, rng)
    scarecrow(k, rng)
    watering_can(k, -1.2, -0.02)


def patch_wind(x, y, z, r):
    if abs(y) < VY0 or z < 0.25 or (x > 0.8 and y > 0.3):
        return 0.0
    if abs(x - SCX) < 0.65 and abs(y - SCY) < 0.25 and z > 0.5:
        h = math.hypot(x - SCX, y - SCY)
        if z < 0.78:
            return 0.4 * clamp01((0.78 - z) / 0.15) * clamp01(h / 0.12)
        if z < 1.34:
            return 0.5 * clamp01((h - 0.19) / 0.3)
        return 0.0
    return 0.35 * clamp01((z - 0.25) / 0.2)


PROPS = {"lantern_bunting": build_lantern_bunting, "washing_line": build_washing_line,
         "flower_planter": build_flower_planter, "vegetable_patch": build_vegetable_patch}
FOOT = {"lantern_bunting": (3, 1), "washing_line": (3, 1), "flower_planter": (1, 1), "vegetable_patch": (3, 2)}
WIND = {"lantern_bunting": bunting_wind, "washing_line": washing_wind, "flower_planter": planter_wind,
        "vegetable_patch": patch_wind}
CLIPS = {}
