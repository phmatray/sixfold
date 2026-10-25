"""Tide: The Saltreach (Water). Harbour life on stilts, nets drying, painted boats, gulls, wide tidal flats.

Vocabulary: teal clapboard and red roofs on stilts with barnacled feet, navy-and-white hulls, cork-orange floats,
wave-worn blue-grey rock with barnacles, mussels and a green weed line, sea pinks on the cliff tops.
Glow parts: td_window_glow (harbour house and ferry windows), lamp_glow (the bell tower's night lamp, the ferry's
masthead light), td_buoy_glow (the buoy's lamp), stove_glow (The Dry Dock's firebox). td_pool_water is the tide pool's surface (rippling water shader).
"""
import math
import random

import bmesh
from mathutils import Matrix, Vector

from v13_kit import (A, B, T, arch_pts, beam, box, chain, contact, curve, cyl, empty, ico, jitter, lathe,  # noqa: F401
                     prism, ribbon, rock, shadow, sphere, stratum, torus, tube, FRONT)

PALETTE = {
    # harbour house
    "td_wall": "#4f9fb8", "td_wall_dk": "#4590a8", "td_trim": "#f4efe4", "td_roof": "#d4573f", "td_roof_dk": "#a8402e",
    "td_door": "#e8b33a", "td_shutter": "#2c4a6b", "td_window_glow": "#f7e6a8", "td_flower": "#e8607a",
    # ferry
    "td_hull": "#2c4a6b", "td_antifoul": "#b8443a", "td_cabin_roof": "#3f9aa8", "td_funnel": "#d4573f",
    "td_funnel_top": "#2a2d33", "td_tyre": "#33363c", "td_deck": "#c99a6a",
    # rock, weed, sea life
    "td_sc_wet": "#5d564c", "td_sc_ochre": "#b89c74", "td_sc_buff": "#cdb68e", "td_sc_pale": "#dccaa5",
    "td_sc_sand": "#e8d6a6", "td_marram": "#cdc98c", "td_marram_dk": "#a9ad6e", "td_weed": "#4f7a3c",
    "td_cliff_a": "#8a97a0", "td_cliff_b": "#76838c", "td_cliff_c": "#a2adb1", "td_cliff_d": "#66737c",
    "td_turf": "#9bb36a", "td_turf_dk": "#7f9a52", "td_thrift": "#ee8db4", "td_thrift_dk": "#c86a92",
    "td_algae": "#5f8a4a", "td_algae_dk": "#46703a", "td_barnacle": "#ece6d6", "td_mussel": "#2f3550",
    "td_stone": "#bdb8aa", "td_stone_dk": "#9a968a", "td_stone_lt": "#d2cdc0", "td_dark": "#2a2730",
    "td_spire": "#5a7590", "td_spire_dk": "#46607a", "td_bronze": "#c99440", "td_bronze_dk": "#94682a",
    "td_gull": "#f6f4ef", "td_gull_wing": "#a9b4bd", "td_beak": "#f0b030",
    # nets, pots, floats, buoy, pool
    "td_net": "#d6c49a", "td_net_line": "#8f7752", "td_net_dk": "#3c5a6e", "td_float": "#ef7a2f", "td_buoy_red": "#d0453a",
    "td_buoy_glow": "#ffd27a", "td_pool_water": "#4fb8c4", "td_star": "#ef8a3a", "td_anemone": "#e86a8a",
    "td_anemone_g": "#6fbf8a", "td_shell": "#f2e2cf", "td_rim": "#a9a49a", "td_rim_dk": "#8e8a82",
    # 1.3 PR 3: the clapper, the Ferry Pass, the Harbour Office desk, The Dry Dock's stove
    "td_verdigris": "#62a88c", "td_verdigris_dk": "#3f7f6a", "td_bronze_lt": "#f2c25e",
    "td_ticket": "#efd9a6", "td_ticket_dk": "#e2c88e", "td_ticket_line": "#c9a86a", "td_stamp_ink": "#4a5bb0",
    "td_stamp_red": "#c8403a", "td_cord": "#d24a3a", "td_cord_dk": "#f3e7d0", "td_stamp_knob": "#c98a50",
    "td_chart": "#f1e8cf", "td_chart_sea": "#a8d2e2", "td_chart_deep": "#7db4d0", "td_chart_coast": "#6b5a44",
    "td_ink": "#232838", "td_quill": "#f4f1ea", "td_steel": "#aab2ba",
    "td_castiron": "#4c515a", "td_iron_dk": "#33363d", "stove_glow": "#ff8a3a", "td_copper": "#c97c48",
    "td_copper_dk": "#9a5a32", "td_chowder": "#f2e2b8", "td_potato": "#e9c86a", "td_carrot": "#e8813a", "td_coal": "#2b2b31",
}

WIND = {}
FOOT = {"harbour_house": (5, 4), "ferry": (4, 2), "bell_tower": (3, 3), "net_rack": (2, 1), "chart_table": (2, 1)}
INTERIOR = {"chart_table", "chowder_stove"}   # exported to <out>/interior/
ANIM = set()

LIGHT = dict(sun=("#fff4e0", 2.7), from_hero=(8, 18, 6), hemi=("#f2fbff", "#6d8a7a", 1.6), bg="#bfe3e8", fog=(16, 34))


# ---------------------------------------------------------------- helpers


def slab(pts, z0, z1):
    """A polygon (x, y) in plan, extruded from z0 to z1."""
    bm = bmesh.new()
    lo = [bm.verts.new((x, y, z0)) for x, y in pts]
    hi = [bm.verts.new((x, y, z1)) for x, y in pts]
    bm.faces.new(lo[::-1])
    bm.faces.new(hi)
    for i in range(len(pts)):
        j = (i + 1) % len(pts)
        bm.faces.new((lo[i], lo[j], hi[j], hi[i]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return bm


def drape(f, nu, nv, t=0.012):
    """A thin solid cloth: a grid of points f(u, v) (u, v in [0, 1]) given a little thickness (nets)."""
    bm = bmesh.new()
    vs = [[bm.verts.new(f(i / nu, j / nv)) for i in range(nu + 1)] for j in range(nv + 1)]
    for j in range(nv):
        for i in range(nu):
            bm.faces.new((vs[j][i], vs[j][i + 1], vs[j + 1][i + 1], vs[j + 1][i]))
    bmesh.ops.solidify(bm, geom=bm.faces[:], thickness=t)
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return bm


def netting(k, f, nu, nv, off, r=0.011):
    """Knotted lines over a drape's front: nv + 1 rows along u, nu + 1 columns along v, pushed `off` toward -Y."""
    o = Vector((0, -off, 0))
    for j in range(nv + 1):
        k.add(tube([f(0.015 + 0.97 * i / 8, j / nv) + o for i in range(9)], r, 3), "td_net_line")
    for i in range(nu + 1):
        k.add(tube([f(0.03 + 0.94 * i / nu, j / 3) + o for j in range(4)], r, 3), "td_net_line")


def orient_out(bm, ref):
    """Flip faces of an open surface so they point away from ref(centre) (hull bands)."""
    for f in bm.faces:
        c = f.calc_center_median()
        if f.normal.dot(c - ref(c)) < 0:
            f.normal_flip()
    return bm


def barnacles(k, rng, pts, r=0.022):
    for p in pts:
        k.add(cyl(r, r * 0.45, r * 0.9, 4), "td_barnacle", T(p, (rng.uniform(-30, 30), rng.uniform(-30, 30), 0)))


def gull(k, M):
    """A gull perched facing -Y (about 0.3 m long)."""
    k.add(sphere(0.07, 8, 5), "td_gull", M @ T((0, 0, 0.1), scale=(0.75, 1.5, 0.8)), smooth=True)
    k.add(sphere(0.05, 8, 5), "td_gull", M @ T((0, -0.1, 0.17)), smooth=True)
    k.add(cyl(0.016, 0.0, 0.06, 4), "td_beak", M @ T((0, -0.17, 0.165), (90, 0, 0)))
    for s in (-1, 1):
        k.add(prism([(0, 0), (0.05, -0.02), (0.02, 0.17)], 0.012), "td_gull_wing", M @ T((s * 0.05, 0.02, 0.12), (90, 0, 0)) @ T((0, 0, 0), (0, s * 12, 0)))
        k.add(cyl(0.008, 0.008, 0.06, 4), "td_beak", M @ T((s * 0.025, 0, 0.03)))
    k.add(prism([(-0.04, 0), (0.04, 0), (0, 0.08)], 0.012), "td_gull_wing", M @ T((0, 0.12, 0.11), (100, 0, 0)))


def floats(k, pts, r=0.07):
    for p in pts:
        k.add(sphere(r, 6, 4), "td_float", T(p, scale=(1, 1, 0.8)), smooth=True)


# ---------------------------------------------------------------- terrain: the sea cliff


def pillow(rng, z0, z1, bulge, rim_in=0.03, flat_top=False):
    """A 1 x 1 stratum with rounded, wave-polished sides: the side mid-line pushed out, the rims tucked in, so
    stacked layers meet in soft grooves. With flat_top the top face stays a flat plane (at z1)."""
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)
    bmesh.ops.subdivide_edges(bm, edges=bm.edges[:], cuts=1, use_grid_fill=True)
    for v in bm.verts:
        t = v.co.z + 0.5
        v.co.z = z0 + t * (z1 - z0)
        for ax in (0, 1):
            if abs(abs(v.co[ax]) - 0.5) > 1e-4:
                continue
            if abs(t - 0.5) < 1e-4:
                v.co[ax] *= 1 + bulge * rng.uniform(0.7, 1.3)
            elif t > 0.5:
                v.co[ax] *= 1 - rim_in * (1.4 if flat_top else 1.0)
            else:
                v.co[ax] *= 1 - rim_in * 0.7
    bmesh.ops.delete(bm, geom=[f for f in bm.faces if f.normal.z < -0.9 and all(abs(v.co.z - z0) < 1e-4 for v in f.verts)], context="FACES")
    return bm


def sea_block(k, rng, top_variant=False):
    """1 x 1 x 1 Saltreach cliff: warm buff and ochre strata, rounded by the waves; a dark wet foot crusted with
    barnacles and mussels under a fringe of hanging weed; a sandy top with marram tufts and sea pinks.
    Sides reach the tile edges (the soft bulges a little past); flat top at exactly 1.0."""
    k.add(pillow(rng, -0.02, 0.3, 0.05, 0.04), "td_sc_wet")
    k.add(pillow(rng, 0.3, 0.56, 0.045), "td_sc_ochre")
    k.add(pillow(rng, 0.56, 0.8, 0.05), "td_sc_buff")
    k.add(pillow(rng, 0.8, 1.0, 0.03, 0.02, flat_top=True), "td_sc_sand")
    for side in range(4):
        a = math.radians(90 * side)
        nx, ny = round(math.cos(a)), round(math.sin(a))
        at = lambda u, z, out=0.535: (nx * out - ny * u, ny * out + nx * u, z)  # noqa: E731
        # weed fringe hanging from the top of the wet band: a jagged strip
        pts = [(-0.46 + 0.153 * i, 0.0 if i % 2 == 0 else -rng.uniform(0.07, 0.12)) for i in range(7)]
        k.add(prism(pts + [(0.46, 0.035), (-0.46, 0.035)], 0.02), "td_weed", T(at(0, 0.31, 0.54), (0, 0, 90 if nx else 0)))
        u = rng.uniform(-0.3, 0.3)  # a barnacle crust and a few mussels on the wet foot
        for i in range(2):
            k.add(cyl(0.024, 0.01, 0.022, 4), "td_barnacle", T(at(u + 0.06 * i, 0.12 + 0.07 * (i % 2), 0.545), (90 * ny, -90 * nx, 0)))
        if side % 2 == 0:
            for i in range(2):
                k.add(ico(0.035, 0), "td_mussel", T(at(-u + 0.05 * i, 0.07 + 0.04 * i, 0.54), (0, 0, 90 * side), (0.55, 1.0, 0.6)))
    tufts = [(-0.25, -0.2), (0.2, 0.18), (0.28, -0.26)] + ([(-0.2, 0.26), (0.0, -0.05)] if top_variant else [])
    for i, (x, y) in enumerate(tufts):  # marram tufts: a fan of pale blades
        for j in range(4):
            ang = j * 90 + rng.uniform(-25, 25)
            h = rng.uniform(0.14, 0.24)
            k.add(cyl(0.016, 0.0, h, 3), "td_marram" if j % 2 else "td_marram_dk",
                  T((x, y, 1.0 + h / 2 - 0.01), (rng.uniform(10, 24) * math.cos(math.radians(ang)), rng.uniform(10, 24) * math.sin(math.radians(ang)), 0)) )
    pinks = [(0.05, 0.3), (-0.32, 0.05)] + ([(0.32, 0.02)] if top_variant else [])
    for x, y in pinks:  # sea pinks
        k.add(ico(0.05, 0), "td_turf_dk", T((x, y, 1.0), scale=(1.2, 1.2, 0.45)))
        k.add(ico(0.036, 0), "td_thrift", T((x, y, 1.08)))


def build_cliff_sea(k):
    sea_block(k, random.Random(501))


def build_cliff_sea_b(k):
    sea_block(k, random.Random(502), top_variant=True)


def build_rock_sea(k):
    """1 x 1 barnacled boulder (~0.6 m) with a weed skirt and mussels, for shores and the tree ring."""
    rng = random.Random(503)
    k.add(rock(rng, 0.46, 2, 0.14), "td_cliff_a", T((0.02, 0.06, 0.24), scale=(1.0, 0.85, 0.66)))
    k.add(rock(rng, 0.22, 1, 0.15), "td_cliff_b", T((-0.3, -0.22, 0.1), scale=(1, 0.9, 0.62)))
    k.add(jitter(ico(0.44, 1), rng, 0.03), "td_algae", T((0.02, 0.05, 0.05), scale=(1.05, 0.92, 0.18)), smooth=True)
    barnacles(k, rng, [(rng.uniform(-0.3, 0.3), -0.33, rng.uniform(0.12, 0.3)) for _ in range(6)])
    for i in range(4):
        k.add(sphere(0.04, 6, 4), "td_mussel", T((0.28 + 0.04 * i, -0.22 + 0.03 * i, 0.12), (0, 0, 40), (0.55, 1, 0.6)), smooth=True)
    contact(k, 0.5)


# ---------------------------------------------------------------- harbour house (5 x 4, on stilts)


def build_harbour_house(k):
    """5 x 4 harbour house on barnacled stilts: teal clapboard, white trim, a red gable facing the quay, a
    sunflower-yellow door on the centre column of the front row reached by three steps, shuttered windows with
    flower boxes, a net drying on the porch rail, a string of floats, a stovepipe (smoke_origin)."""
    rng = random.Random(510)
    fz = 0.68                                                    # porch and floor level
    for x in (-2.2, -0.75, 0.75, 2.2):                           # stilts with a weed line and barnacles
        for y in (-1.3, 0.25, 1.75):
            k.add(cyl(0.085, 0.08, 0.62, 6), "post", T((x, y, 0.29)), smooth=True)
            k.add(cyl(0.1, 0.1, 0.16, 6), "td_algae", T((x, y, 0.07)), smooth=True)
    for x0, x1 in ((-2.2, -0.75), (0.75, 2.2)):                  # cross braces under the front
        beam(k, (x0, -1.32, 0.08), (x1, -1.32, fz - 0.08), 0.05, 0.05, "post")
        beam(k, (x0, -1.37, fz - 0.08), (x1, -1.37, 0.08), 0.05, 0.05, "post")
    barnacles(k, rng, [(x + 0.08 * math.cos(a), y + 0.08 * math.sin(a), 0.2) for x, y, a in
                       ((-2.2, -1.3, -100), (-0.75, -1.3, -80), (0.75, -1.3, -110), (2.2, -1.3, -70), (2.2, 0.25, -20))])
    # platform: joists and planks along X, the front edge at y = -1.45
    k.add(box(4.9, 3.35, 0.1), "post", T((0, 0.25, fz - 0.11)))
    for i in range(9):
        y = -1.45 + 0.186 + 0.372 * i
        k.add(box(4.92, 0.36, 0.05), ("plank", "plank_lt", "plank_dk")[i % 3], T((0, y - 0.186, fz - 0.025)))
    # steps down the front row on the centre column
    for i, top in enumerate((0.51, 0.34, 0.17)):
        k.add(box(0.95, 0.2, 0.05), "plank_lt", T((0, -1.55 - 0.18 * i, top - 0.025)))
        k.add(box(0.9, 0.04, top - 0.03), "post", T((0, -1.48 - 0.18 * i, (top - 0.07) / 2)))
    for sx in (-1, 1):
        beam(k, (sx * 0.5, -1.42, fz), (sx * 0.5, -1.98, 0.02), 0.06, 0.06, "post")
    # walls: a teal box with clapboard strips on the front and the sides
    x0, x1, y0, y1, top = -2.1, 2.1, -0.95, 1.85, fz + 2.07
    k.add(box(x1 - x0, y1 - y0, top - fz), "td_wall_dk", T((0, (y0 + y1) / 2, (top + fz) / 2)))
    for j in range(7):
        z = fz + 0.14 + 0.285 * j
        k.add(box(4.22, 0.03, 0.23), "td_wall", T((0, y0 - 0.012, z), (-6, 0, 0)))
        for sx in (-1, 1):
            k.add(box(0.03, 2.82, 0.23), "td_wall", T((sx * 2.112, (y0 + y1) / 2, z), (0, sx * 6, 0)))
    for sx in (-1, 1):
        for sy in (y0, y1):
            k.add(box(0.13, 0.13, top - fz + 0.02), "td_trim", T((sx * 2.1, sy, (top + fz) / 2)))
    k.add(box(4.36, 2.96, 0.1), "td_trim", T((0, (y0 + y1) / 2, top)))
    # front gable (wall colour) under two red roof slabs, ridge along Y
    gh, yc, d = 1.3, (y0 + y1) / 2, y1 - y0
    k.add(prism([(-2.12, 0), (2.12, 0), (0, gh)], d - 0.02), "td_wall", T((0, yc, top + 0.05)))
    L = math.hypot(2.12, gh)
    ang = math.degrees(math.atan2(gh, 2.12))
    for s in (-1, 1):
        u = Vector((-s * 2.12, gh)) / L
        n = Vector((s * gh, 2.12)) / L
        mid = Vector((s * 1.06, top + 0.05 + gh / 2)) - u * 0.18 + n * 0.07
        k.add(box(L + 0.36, d + 0.5, 0.12, 0.02), "td_roof", T((mid.x, yc, mid.y), (0, s * ang, 0)))
        k.add(box(L + 0.36, 0.08, 0.16), "td_roof_dk", T((mid.x - n.x * 0.01, y0 - 0.27 - 0.006 * (s > 0), mid.y - 0.02), (0, s * ang, 0)))
    k.add(box(0.2, d + 0.56, 0.12), "td_roof_dk", T((0, yc, top + 0.05 + gh + 0.1)))
    # round window in the gable, door, two shuttered windows with flower boxes
    gy = y0 - 0.02
    k.add(torus(0.22, 0.05, 10, 4), "td_trim", T((0, gy - 0.02, top + 0.6), (90, 0, 0)))
    k.add(cyl(0.2, 0.2, 0.03, 12), "td_window_glow", T((0, gy, top + 0.6), (90, 0, 0)))
    k.add(box(1.02, 0.08, 1.95), "td_trim", T((0, gy - 0.03, fz + 0.96)))
    k.add(box(0.8, 0.06, 1.78), "td_door", T((0, gy - 0.06, fz + 0.89)))
    for z in (fz + 0.45, fz + 1.35):
        k.add(box(0.62, 0.02, 0.5), "td_trim", T((0, gy - 0.095, z)))
    k.add(sphere(0.04, 6, 4), "knob", T((0.28, gy - 0.11, fz + 0.9)))
    k.add(box(1.3, 0.5, 0.06), "td_roof", T((0, gy - 0.25, fz + 2.03), (-14, 0, 0)))
    for sx in (-1, 1):
        x = sx * 1.35
        k.add(box(0.86, 0.08, 0.86), "td_trim", T((x, gy - 0.03, fz + 1.18)))
        k.add(box(0.66, 0.04, 0.66), "td_window_glow", T((x, gy - 0.06, fz + 1.18)))
        k.add(box(0.05, 0.05, 0.62), "td_trim", T((x, gy - 0.08, fz + 1.18)))
        k.add(box(0.62, 0.05, 0.05), "td_trim", T((x, gy - 0.088, fz + 1.18)))
        for s2 in (-1, 1):
            k.add(box(0.28, 0.04, 0.84), "td_shutter", T((x + s2 * 0.6, gy - 0.05, fz + 1.18)))
        k.add(box(0.78, 0.2, 0.14), "wood_dk", T((x, gy - 0.14, fz + 0.7)))
        for i in range(4):
            k.add(ico(0.065, 0), "td_flower" if i % 2 else "leaf", T((x - 0.27 + 0.18 * i, gy - 0.15, fz + 0.8)))
    for sx in (-1, 1):  # side windows
        k.add(box(0.08, 0.7, 0.7), "td_trim", T((sx * 2.14, 0.5, fz + 1.2)))
        k.add(box(0.04, 0.54, 0.54), "td_window_glow", T((sx * 2.17, 0.5, fz + 1.2)))
    # porch rail along the front edge, gap for the steps
    for xa, xb in ((-2.4, -0.55), (0.55, 2.4)):
        for x in (xa, (xa + xb) / 2, xb):
            k.add(box(0.07, 0.07, 0.62), "td_trim", T((x, -1.4, fz + 0.3)))
        k.add(box(abs(xb - xa) + 0.09, 0.08, 0.06), "td_trim", T(((xa + xb) / 2, -1.4, fz + 0.62)))
        k.add(box(abs(xb - xa), 0.04, 0.04), "td_trim", T(((xa + xb) / 2, -1.4, fz + 0.3)))
    # a net drying over the left rail, scalloped hem, floats along its top
    def net(u, v):
        x = -2.3 + 1.6 * u
        return Vector((x, -1.47 - 0.05 * v, fz + 0.64 - 0.55 * v - 0.08 * math.sin(u * math.pi * 3) * v - 0.06 * math.sin(u * math.pi)))
    k.add(drape(net, 8, 3), "td_net")
    netting(k, net, 6, 2, 0.012)
    floats(k, [(-2.2 + 0.36 * i, -1.47, fz + 0.66) for i in range(5)], 0.055)
    # a string of floats and a life ring on the right wall
    for p in curve((1.75, y0 - 0.06, top - 0.15), (2.08, y0 - 0.06, fz + 0.8), 0.1, 4)[1:]:
        floats(k, [p], 0.06)
    k.add(torus(0.17, 0.05, 10, 4), "td_buoy_red", T((-2.18, 0.5, fz + 1.0), (0, 90, 0)))
    for a in (0, 90, 180, 270):
        k.add(box(0.03, 0.06, 0.08), "td_trim", T((-2.24, 0.5 + 0.17 * math.cos(math.radians(a)), fz + 1.0 + 0.17 * math.sin(math.radians(a)))))
    # lantern by the door
    k.add(box(0.03, 0.18, 0.03), "iron", T((0.65, gy - 0.09, fz + 1.8)))
    k.add(box(0.12, 0.12, 0.16), "lamp_glow", T((0.65, gy - 0.18, fz + 1.7)))
    k.add(cyl(0.1, 0.0, 0.07, 4), "iron", T((0.65, gy - 0.18, fz + 1.81), (0, 0, 45)))
    # stovepipe on the right slope
    sx, sy = 1.25, 1.2
    roof_z = top + 0.05 + gh * (1 - sx / 2.12) + 0.1
    k.add(cyl(0.09, 0.09, 0.8, 8), "iron", T((sx, sy, roof_z + 0.2)))
    k.add(cyl(0.14, 0.0, 0.12, 8), "iron", T((sx, sy, roof_z + 0.68)))
    return empty("smoke_origin", (sx, sy, roof_z + 0.75))


# ---------------------------------------------------------------- ferry (4 x 2, bobs)


def build_ferry(k):
    """4 x 2 harbour ferry, bow toward +X: navy hull with a white stripe over red antifouling, a planked open
    deck (0.44 m) with benches forward, a white wheelhouse with teal roof, a red funnel, a masthead light."""
    xs = [-1.85, -1.6, -0.9, 0.0, 0.8, 1.4, 1.9]
    ws = [0.7, 0.76, 0.8, 0.8, 0.7, 0.45, 0.05]
    deck = 0.44

    def sec(x, w):
        f = (1.0, 0.985, 0.95, 0.72, 0.25)
        zz = (deck, 0.2, 0.08, -0.12, -0.28)
        side = [(x, -w * a, z) for a, z in zip(f, zz)]
        return side + [(x, -p[1], p[2]) for p in reversed(side)]

    st = [sec(x, w) for x, w in zip(xs, ws)]
    ref = lambda c: Vector((c.x, 0, 0.1))  # noqa: E731
    for idx, m in (([0, 1], "td_hull"), ([8, 9], "td_hull"), ([1, 2], "td_trim"), ([7, 8], "td_trim"),
                   ([2, 3, 4, 5, 6, 7], "td_antifoul")):
        k.add(orient_out(A.loft(st, idx), ref), m)
    bm = bmesh.new()  # transom
    f = bm.faces.new([bm.verts.new(p) for p in st[0]])
    if f.normal.x > 0:
        f.normal_flip()
    k.add(bm, "td_hull")
    outline = [(x, -w) for x, w in zip(xs, ws)] + [(x, w) for x, w in reversed(list(zip(xs, ws)))]
    k.add(slab(outline, deck - 0.05, deck), "td_deck")
    k.add(ribbon(outline, 0.07, 0.2, closed=True), "td_hull", T((0, 0, deck - 0.01)))
    k.add(ribbon(outline, 0.09, 0.04, closed=True), "td_trim", T((0, 0, deck + 0.19)))
    for i in range(1, 5):  # rail posts and top rail along the open deck
        for s in (-1, 1):
            x = -0.3 + 0.4 * i
            w = ws[3] if x < 0.8 else ws[3] + (ws[4] - ws[3]) * (x - 0.0) / 0.8
            k.add(cyl(0.02, 0.02, 0.4, 5), "td_trim", T((x, s * (w - 0.04), deck + 0.4)))
    for s in (-1, 1):
        k.add(tube([(-0.35, s * 0.76, deck + 0.6), (0.5, s * 0.76, deck + 0.6), (1.0, s * 0.66, deck + 0.6), (1.35, s * 0.44, deck + 0.6)], 0.022, 5),
              "td_trim")
    # benches forward
    for s in (-1, 1):
        k.add(box(0.95, 0.24, 0.05, 0.01), "wood", T((0.4, s * 0.48, deck + 0.38)))
        k.add(box(0.95, 0.04, 0.22, 0.01), "wood", T((0.4, s * 0.6, deck + 0.5)))
        for x in (0.0, 0.8):
            k.add(box(0.05, 0.2, 0.36), "iron", T((x, s * 0.48, deck + 0.18)))
    # wheelhouse: white walls, glow windows, teal roof
    wx0, wx1, wy, wz = -1.5, -0.45, 0.55, deck + 1.0
    k.add(box(wx1 - wx0, 2 * wy, wz - deck, 0.02), "td_trim", T(((wx0 + wx1) / 2, 0, (wz + deck) / 2)))
    k.add(box(0.05, 0.9, 0.38), "td_window_glow", T((wx1 + 0.01, 0, deck + 0.72)))
    for x in (wx1 + 0.02,):
        for y in (-0.15, 0.15):
            k.add(box(0.04, 0.05, 0.4), "td_hull", T((x + 0.01, y, deck + 0.72)))
    for s in (-1, 1):
        for x in (-1.25, -0.75):
            k.add(box(0.34, 0.05, 0.32), "td_window_glow", T((x, s * (wy + 0.01), deck + 0.72)))
    k.add(box(0.36, 0.04, 0.72), "td_hull", T((-1.0 + 0.25, -(wy + 0.02), deck + 0.38)))   # door, port side
    k.add(box(wx1 - wx0 + 0.3, 2 * wy + 0.24, 0.08, 0.02), "td_cabin_roof", T(((wx0 + wx1) / 2 + 0.05, 0, wz + 0.04)))
    k.add(box(wx1 - wx0 + 0.36, 0.06, 0.08), "td_cabin_roof", T(((wx0 + wx1) / 2 + 0.05, 0, wz - 0.02)))
    # funnel and the life ring
    k.add(cyl(0.15, 0.13, 0.5, 10), "td_funnel", T((-1.25, 0, wz + 0.3)), smooth=True)
    k.add(cyl(0.155, 0.155, 0.08, 10), "td_trim", T((-1.25, 0, wz + 0.36)))
    k.add(cyl(0.135, 0.135, 0.08, 10), "td_funnel_top", T((-1.25, 0, wz + 0.58)))
    k.add(cyl(0.06, 0.06, 0.09, 6), "hex", T((-1.25, -0.155, wz + 0.36), (90, 30, 0)))
    k.add(torus(0.15, 0.045, 12, 5), "td_buoy_red", T((-0.95, wy + 0.06, deck + 0.55), (90, 0, 0)))
    for a in (0, 90, 180, 270):
        k.add(box(0.07, 0.03, 0.04), "td_trim", T((-0.95 + 0.15 * math.cos(math.radians(a)), wy + 0.1, deck + 0.55 + 0.15 * math.sin(math.radians(a))), (0, -a, 0)))
    # mast with a pennant and the masthead light at the bow
    k.add(cyl(0.025, 0.02, 1.3, 6), "wood_dk", T((1.45, 0, deck + 0.65)))
    k.add(prism([(0, 0), (0.32, -0.06), (0, -0.14)], 0.01), "td_cabin_roof", T((1.47, 0, deck + 1.28)))
    k.add(box(0.1, 0.1, 0.12), "lamp_glow", T((1.45, 0, deck + 1.05)))
    k.add(cyl(0.08, 0.0, 0.06, 4), "iron", T((1.45, 0, deck + 1.14), (0, 0, 45)))
    # tyre fenders over the sides
    for x in (-1.2, -0.2, 0.7):
        for s in (-1, 1):
            k.add(torus(0.1, 0.04, 8, 4), "td_tyre", T((x, s * (ws[2] + 0.06), 0.24), (90, 0, 0)))
    return empty("light_origin", (1.45, 0, deck + 1.05))


# ---------------------------------------------------------------- bell tower (3 x 3, leaning, half drowned)


def build_bell_tower(k):
    """3 x 3 drowned bell tower on its islet: weed-lined rocks, a leaning stone shaft with a sunken doorway and a
    green tide mark, an open belfry with a bronze bell, a slate-blue pyramid roof with a fish vane, an iron lantern
    on a bracket (lamp_glow, light_origin), two gulls."""
    rng = random.Random(520)
    k.add(jitter(ico(1.3, 2), rng, 0.08), "td_cliff_b", T((0, 0, 0.0), scale=(1.1, 1.0, 0.2)))
    for i in range(9):
        a = i / 9 * math.tau + rng.uniform(-0.2, 0.2)
        r = rng.uniform(1.0, 1.25)
        s = rng.uniform(0.22, 0.38)
        k.add(rock(rng, s, 1, 0.15), rng.choice(["td_cliff_a", "td_cliff_c", "td_cliff_b"]),
              T((r * math.cos(a), r * math.sin(a), s * 0.35), scale=(1.2, 1.0, 0.8)))
        k.add(jitter(ico(s * 1.05, 1), rng, 0.02), "td_algae", T((r * math.cos(a), r * math.sin(a), 0.03), scale=(1.25, 1.05, 0.25)), smooth=True)
    for i in range(5):  # weed strands draped over the rocks
        a = rng.uniform(0, math.tau)
        x, y = 1.1 * math.cos(a), 1.1 * math.sin(a)
        k.add(tube([(x, y, 0.3), (x * 1.1, y * 1.1, 0.18), (x * 1.18, y * 1.18, 0.02)], 0.03, 4, 0.5), "td_algae_dk")
    lean = T((0, 0, 0.1), (-3, 7, 0))
    Lm = lambda M: lean @ M  # noqa: E731
    k.add(box(1.5, 1.5, 3.7, 0.02), "td_stone", Lm(T((0, 0, 1.8))))
    k.add(box(1.54, 1.54, 0.55, 0.02), "td_algae", Lm(T((0, 0, 0.27))))
    for z in (1.3, 2.55, 3.66):
        k.add(box(1.6, 1.6, 0.1, 0.02), "td_stone_dk", Lm(T((0, 0, z))))
    for z, w in ((0.9, 0.4), (1.95, 0.3), (3.1, 0.35)):  # a few proud blocks for texture
        k.add(box(w, 0.04, 0.22), "td_stone_lt", Lm(T((rng.uniform(-0.4, 0.4), -0.772, z))))
        k.add(box(0.04, w, 0.22), "td_stone_lt", Lm(T((0.772, rng.uniform(-0.4, 0.4), z + 0.35))))
    k.add(prism(arch_pts(0.6, 1.0), 0.04), "td_dark", Lm(T((0, -0.758, 0.2))))           # the sunken door
    k.add(ribbon(arch_pts(0.72, 1.06, 6), 0.08, 0.05), "td_stone_dk", Lm(T((0, -0.77, 0.2), FRONT)))
    for x, z in ((0.0, 2.05), (0.0, 3.1)):
        k.add(box(0.14, 0.04, 0.4), "td_dark", Lm(T((x, -0.758, z))))
    k.add(box(0.04, 0.14, 0.4), "td_dark", Lm(T((0.758, 0, 2.6))))
    barnacles(k, rng, [Lm(T((rng.uniform(-0.6, 0.6), -0.79, rng.uniform(0.1, 0.5)))).to_translation() for _ in range(7)])
    # belfry: piers, arches on all faces, the bell
    bz0, bz1 = 3.72, 4.95
    k.add(box(1.46, 1.46, 0.08), "td_stone_dk", Lm(T((0, 0, bz0 + 0.02))))
    for sx in (-1, 1):
        for sy in (-1, 1):
            k.add(box(0.3, 0.3, bz1 - bz0, 0.015), "td_stone", Lm(T((sx * 0.62, sy * 0.62, (bz0 + bz1) / 2))))
    for rot in (0, 90, 180, 270):
        k.add(ribbon(arch_pts(0.9, 1.15, 8), 0.12, 0.14), "td_stone_lt", Lm(T((0, 0, 0), (0, 0, rot)) @ T((0, -0.68, bz0 + 0.065), FRONT)))
    k.add(box(1.3, 0.1, 0.1), "wood_dk", Lm(T((0, 0, bz1 - 0.12))))
    k.add(lathe([(0.0, bz1 - 0.16), (0.11, bz1 - 0.16), (0.16, bz1 - 0.28), (0.19, bz1 - 0.5), (0.27, bz1 - 0.68), (0.29, bz1 - 0.73), (0.0, bz1 - 0.73)], 12),
          "td_bronze", Lm(T()), smooth=True)
    k.add(torus(0.27, 0.025, 12, 4), "td_bronze_dk", Lm(T((0, 0, bz1 - 0.7))))
    k.add(sphere(0.06, 6, 4), "td_bronze_dk", Lm(T((0, 0, bz1 - 0.78))))
    k.add(box(1.72, 1.72, 0.12, 0.02), "td_stone_dk", Lm(T((0, 0, bz1 + 0.04))))
    k.add(cyl(1.22, 0.0, 1.35, 4), "td_spire", Lm(T((0, 0, bz1 + 0.8), (0, 0, 45))))
    vz = bz1 + 1.4
    k.add(cyl(0.015, 0.015, 0.45, 4), "iron", Lm(T((0, 0, vz + 0.2))))
    k.add(prism([(-0.16, 0), (0.06, 0.06), (0.16, 0.0), (0.06, -0.06), (-0.16, 0), (-0.22, 0.06), (-0.22, -0.06)], 0.015), "iron", Lm(T((0.02, 0, vz + 0.36))))
    # the night lamp: an iron bracket and lantern on the front-right corner
    lz = 3.15
    k.add(box(0.05, 0.4, 0.05), "iron", Lm(T((0.55, -0.92, lz + 0.35))))
    beam(k, Lm(T((0.55, -0.76, lz + 0.05))).to_translation(), Lm(T((0.55, -1.08, lz + 0.35))).to_translation(), 0.04, 0.04, "iron")
    k.add(box(0.18, 0.18, 0.24), "lamp_glow", Lm(T((0.55, -1.1, lz + 0.12))))
    for sx in (-1, 1):
        for sy in (-1, 1):
            k.add(box(0.025, 0.025, 0.27), "iron", Lm(T((0.55 + sx * 0.095, -1.1 + sy * 0.095, lz + 0.12))))
    k.add(cyl(0.15, 0.0, 0.1, 4), "iron", Lm(T((0.55, -1.1, lz + 0.29), (0, 0, 45))))
    gull(k, T((-1.05, -0.55, 0.42), (0, 0, 25)))
    gull(k, Lm(T((-0.6, -0.72, bz1 + 0.12), (0, 0, -10))))
    return empty("light_origin", Lm(T((0.55, -1.1, lz + 0.12))).to_translation())


# ---------------------------------------------------------------- small harbour props


def lobster_pot(k, M):
    """One slatted pot: a plank base, three hoops, slats over them, blue net ends."""
    k.add(box(0.5, 0.34, 0.04), "wood_dk", M @ T((0, 0, 0.02)))
    for x in (-0.2, 0.2):
        k.add(torus(0.17, 0.016, 6, 3), "wood", M @ T((x, 0, 0.04), (0, 90, 0)))
    for i in range(3):
        a = math.radians(30 + 60 * i)
        k.add(box(0.5, 0.035, 0.02), "wood_lt", M @ T((0, 0.17 * math.cos(a), 0.04 + 0.17 * math.sin(a)), (90 - math.degrees(a) + 90, 0, 0)))
    for sx in (-1, 1):
        k.add(half_disc(0.16), "td_net_dk", M @ T((sx * 0.232, 0, 0.04), (0, 0, 90)))


def half_disc(r, n=5, t=0.012):
    pts = [(r * math.cos(math.pi * i / n), r * math.sin(math.pi * i / n)) for i in range(n + 1)]
    return prism(pts, t)


def build_crab_pots(k):
    """1 x 1 stack of three slatted pots, a coil of rope and a cork-orange float."""
    lobster_pot(k, T((-0.1, -0.16, -0.015), (0, 0, 8)))
    lobster_pot(k, T((0.05, 0.2, -0.019), (0, 0, -6)))
    lobster_pot(k, T((-0.02, 0.03, 0.22), (0, 0, 80)))
    k.add(torus(0.11, 0.035, 7, 3), "rope", T((0.3, -0.3, 0.035)))
    floats(k, [(-0.33, -0.32, 0.08)], 0.08)
    k.add(tube(curve((-0.33, -0.32, 0.12), (-0.15, -0.12, 0.2), 0.05, 2), 0.012, 3), "rope")
    shadow(k, 0.5, 1.0, 0.9)


def build_buoy(k):
    """1 x 1 channel buoy (bobs): a red float with a white band, a lattice cage and a little lamp on top."""
    k.add(lathe([(0.0, -0.15), (0.42, -0.15), (0.46, 0.05), (0.44, 0.22), (0.3, 0.3), (0.0, 0.32)], 12), "td_buoy_red", T(), smooth=True)
    k.add(cyl(0.465, 0.465, 0.1, 12), "td_trim", T((0, 0, 0.07)))
    k.add(cyl(0.26, 0.24, 0.08, 8), "td_trim", T((0, 0, 0.34)))
    for i in range(4):  # cage legs and a ring
        a = math.radians(45 + 90 * i)
        beam(k, (0.22 * math.cos(a), 0.22 * math.sin(a), 0.36), (0.1 * math.cos(a), 0.1 * math.sin(a), 1.1), 0.04, 0.04, "td_buoy_red")
    k.add(torus(0.16, 0.025, 8, 4), "td_trim", T((0, 0, 0.72)))
    k.add(cyl(0.14, 0.14, 0.05, 8), "td_buoy_red", T((0, 0, 1.12)))
    k.add(cyl(0.08, 0.08, 0.14, 8), "td_buoy_glow", T((0, 0, 1.22)))
    k.add(cyl(0.1, 0.0, 0.08, 8), "td_buoy_red", T((0, 0, 1.33)))
    k.add(prism([(0, 0), (0.18, 0.07), (0, 0.14)], 0.012), "td_buoy_red", T((0.0, 0, 0.82)))   # a little pennant
    return empty("light_origin", (0, 0, 1.22))


def build_net_rack(k):
    """2 x 1 net rack: two A-frame trestles and a pole, a net draped over it (scalloped hem, floats along the top),
    a basket and a pair of oars."""
    rng = random.Random(530)
    h = 1.35
    for x in (-0.82, 0.82):
        for s in (-1, 1):
            beam(k, (x + 0.012 * (s > 0), s * 0.32, -0.02), (x + 0.012 * (s > 0), s * 0.03, h + 0.04), 0.07, 0.07, "post")
        k.add(box(0.06, 0.5, 0.05), "post", T((x, 0, 0.45)))
    k.add(cyl(0.045, 0.045, 1.95, 7), "wood", T((0, 0, h), (0, 90, 0)), smooth=True)
    for side in (-1, 1):  # both falls of the net
        def f(u, v, side=side):
            x = -0.85 + 1.7 * u
            hem = 0.12 * math.sin(u * math.pi * 4) ** 2
            drop = (0.95 + hem) * v
            return Vector((x, side * (0.06 + 0.16 * v), h + 0.03 - drop - 0.05 * math.sin(u * math.pi)))
        k.add(drape(f, 10, 3), "td_net")
        if side < 0:
            netting(k, f, 8, 3, 0.012)
    for i in range(6):
        floats(k, [(-0.78 + 0.31 * i, 0, h + 0.06)], 0.055)
    # basket of floats and the oars
    k.add(cyl(0.2, 0.17, 0.3, 10), "wood_lt", T((0.55, -0.32, 0.14)), smooth=True)
    k.add(torus(0.2, 0.02, 10, 4), "wood_dk", T((0.55, -0.32, 0.29)))
    floats(k, [(0.5, -0.36, 0.32), (0.62, -0.28, 0.33), (0.58, -0.4, 0.3)], 0.06)
    for y, tilt, yaw in ((-0.42, 12, -15), (-0.47, 18, -9)):
        k.add(cyl(0.018, 0.018, 1.25, 6), "oar", T((-0.6, y, 0.6), (tilt, yaw, 0)))
        k.add(box(0.12, 0.02, 0.3), "oar", T((-0.6 - 0.15, y - 0.13, 0.14), (tilt, yaw, 0)))
    _ = rng
    shadow(k, 0.95, 1.0, 0.45)


def build_tidepool(k):
    """1 x 1 tide pool, walkable: a ring of low flat rocks (under 8 cm) round a shallow pool of water
    (td_pool_water, 12 mm up), a starfish, anemones, shells and pebbles."""
    rng = random.Random(540)
    pts = [(0.4 * (1 + rng.uniform(-0.12, 0.08)) * math.cos(a), 0.36 * (1 + rng.uniform(-0.12, 0.08)) * math.sin(a))
           for a in (i / 12 * math.tau for i in range(12))]
    k.add(slab([(x * 0.98, y * 0.98) for x, y in pts], 0.004, 0.012), "td_pool_water")
    for i, (x, y) in enumerate(pts):
        r = rng.uniform(0.07, 0.11)
        k.add(jitter(ico(r, 1), rng, r * 0.15), "td_rim" if i % 3 else "td_rim_dk", T((x * 1.05, y * 1.05, 0.0), scale=(1.3, 1.1, 0.5)))
        if i % 4 == 1:
            k.add(jitter(ico(r * 0.9, 1), rng, 0.01), "td_algae", T((x * 1.15, y * 1.15, -0.005), scale=(1.4, 1.2, 0.28)), smooth=True)
    star = [((0.07 if i % 2 == 0 else 0.028) * math.cos(math.radians(90 + 36 * i)), (0.07 if i % 2 == 0 else 0.028) * math.sin(math.radians(90 + 36 * i)))
            for i in range(10)]
    k.add(slab(star, 0.01, 0.03), "td_star", T((-0.1, 0.06, 0), (0, 0, 20)))
    for x, y, m in ((0.14, -0.08, "td_anemone"), (0.2, 0.08, "td_anemone_g"), (-0.18, -0.14, "td_anemone")):
        k.add(cyl(0.035, 0.03, 0.03, 6), m, T((x, y, 0.025)))
        for j in range(5):
            a = j / 5 * math.tau
            k.add(cyl(0.008, 0.004, 0.035, 3), m, T((x + 0.025 * math.cos(a), y + 0.025 * math.sin(a), 0.05), (25 * math.sin(a), -25 * math.cos(a), 0)))
    for x, y in ((0.32, -0.3), (-0.35, 0.3), (0.0, -0.36)):
        k.add(sphere(0.03, 6, 4), "td_shell", T((x, y, 0.012), scale=(1, 0.8, 0.45)), smooth=True)
    for _ in range(5):
        r = rng.uniform(0.015, 0.03)
        k.add(jitter(ico(r, 1), rng, r * 0.2), "pebble", T((rng.uniform(-0.25, 0.25), rng.uniform(-0.2, 0.2), 0.012), scale=(1, 0.8, 0.4)))


def build_bollard(k):
    """1 x 1 iron mooring bollard with a rope looped round it, on a stone block."""
    k.add(box(0.5, 0.5, 0.1, 0.02), "td_stone_dk", T((0, 0, 0.03)))
    k.add(lathe([(0.0, 0.08), (0.17, 0.08), (0.13, 0.2), (0.12, 0.42), (0.18, 0.5), (0.18, 0.56), (0.0, 0.58)], 10), "td_funnel_top", T(), smooth=True)
    k.add(torus(0.15, 0.03, 10, 4), "rope", T((0, 0, 0.32)))
    k.add(tube([(0.1, -0.12, 0.3), (0.17, -0.2, 0.15), (0.2, -0.22, 0.1), (0.12, -0.24, 0.1), (0.0, -0.22, 0.1)], 0.026, 5), "rope")
    contact(k, 0.34)


# ---------------------------------------------------------------- 1.3 PR 3: curios, the Ferry Pass, Gullhaven interiors


def along(p, n):
    """A matrix at p with its local +Z along n (things stuck on a curved surface)."""
    return Matrix.Translation(p) @ Vector(n).normalized().to_track_quat("Z", "Y").to_matrix().to_4x4()


def build_bell_clapper(k):
    """Curio (the popup, `t_bell`): the drowned bell's clapper. A verdigris bronze shank hung from an eye, a heavy
    ball worn bright in a band where it struck the bell, a short tail; a loop of old rope through the eye with a
    knot and two frayed ends; barnacles on the ball. Centred on the origin."""
    k.add(lathe([(0.0, -0.215), (0.02, -0.19), (0.026, -0.165), (0.05, -0.15), (0.068, -0.12), (0.068, -0.09),
                 (0.05, -0.062), (0.03, -0.04), (0.022, 0.04), (0.016, 0.1), (0.0, 0.122)], 10), "td_verdigris", T(), smooth=True)
    k.add(lathe([(0.071, -0.128), (0.074, -0.105), (0.071, -0.082)], 10), "td_bronze_lt", T(), smooth=True)
    k.add(torus(0.034, 0.012, 10, 4), "td_verdigris_dk", T((0, 0, 0.15), (90, 0, 0)))          # the eye
    k.add(torus(0.05, 0.011, 10, 4), "rope", T((0, 0, 0.196), (0, 90, 0)))                   # rope loop through it
    k.add(sphere(0.019, 6, 4), "rope", T((0, 0, 0.246), scale=(1.2, 1, 1)), smooth=True)     # knot
    for dx, L in ((0.012, 1.0), (-0.004, 0.75)):                                             # two frayed tails
        k.add(tube([(0, 0, 0.25), (dx + 0.02, -0.01, 0.245), (dx + 0.032, -0.016, 0.25 - 0.05 * L), (dx + 0.036, -0.018, 0.25 - 0.09 * L)],
                   0.009, 4, 0.6), "rope")
    for p, sz in (((0.045, -0.04, -0.074), 0.014), ((-0.03, -0.05, -0.07), 0.011), ((0.012, 0.055, -0.072), 0.012)):
        n = Vector(p) - Vector((0, 0, -0.105))
        k.add(cyl(sz, sz * 0.4, sz * 0.9, 5), "td_barnacle", along(Vector(p) + n.normalized() * 0.003, n))


def card(xs, top, bot, t):
    """A ticket-like slab across columns xs (in XZ), top(x) / bot(x) its edges, t thick (Y), with end caps."""
    bm = bmesh.new()
    cols = [[bm.verts.new((x, s * t / 2, z)) for s, z in ((-1, bot(x)), (-1, top(x)), (1, top(x)), (1, bot(x)))] for x in xs]
    for a, b in zip(cols, cols[1:]):
        for j in range(4):
            q = (j + 1) % 4
            bm.faces.new((a[j], b[j], b[q], a[q]))
    bm.faces.new(cols[0])
    bm.faces.new(cols[-1][::-1])
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return bm


def build_ferry_pass(k):
    """Field item (the popup): the Ferry Pass. A buff pasteboard ticket, slightly bent, a teal stub band with a
    brass grommet and a short twisted cord loop, a big round inked stamp (a ring round a wave line, no letters), the
    far corner bitten out by the conductor's punch. Stands upright facing the front, centred on the origin."""
    hw, hh, t, bite = 0.13, 0.075, 0.006, 0.03
    n0 = len(k.parts)
    xs = [-hw + (2 * hw - bite) * i / 10 for i in range(11)] + [hw - bite * 0.62, hw - bite * 0.24, hw]

    def top(x):
        d = x - (hw - bite)
        return hh if d <= 0 else hh - math.sqrt(max(bite * bite - (hw - x) ** 2, 0.0))
    k.add(card(xs, top, lambda x: -hh, t), "td_ticket")
    x0, x1 = -hw + 0.004, -hw + 0.056                                                        # teal stub band, both faces
    k.add(card([x0 + (x1 - x0) * i / 3 for i in range(4)], lambda x: hh - 0.005, lambda x: -hh + 0.005, t + 0.003), "td_cabin_roof")
    k.add(card([x1 + 0.006, x1 + 0.012], lambda x: hh - 0.005, lambda x: -hh + 0.005, t + 0.003), "td_ticket_line")
    # the stamp: a ring and a wave inside it, tilted, 1.5 mm proud of the front
    cx, cz, R, tilt = 0.042, -0.004, 0.047, math.radians(-12)
    rot = lambda x, z: (cx + x * math.cos(tilt) - z * math.sin(tilt), cz + x * math.sin(tilt) + z * math.cos(tilt))  # noqa: E731
    M = T((0, -t / 2 + 0.0005, 0), FRONT)                                                  # 2.5 mm proud
    k.add(ribbon([rot(R * math.cos(a), R * math.sin(a)) for a in (i / 12 * math.tau for i in range(12))], 0.008, 0.003, closed=True), "td_stamp_ink", M)
    k.add(ribbon([rot(-0.03 + 0.01 * i, 0.009 * math.sin(i * math.pi / 2)) for i in range(7)], 0.007, 0.003), "td_stamp_ink", M)
    # grommet, its hole, and a two-ply cord loop out to the upper left
    gx, gz = -hw + 0.026, 0.038
    k.add(torus(0.011, 0.006, 8, 3), "brass", T((gx, 0, gz), (90, 0, 0)))
    k.add(cyl(0.007, 0.007, t + 0.006, 8), "td_dark", T((gx, 0, gz), (90, 0, 0)))
    u, Y = Vector((-1.0, 0.0, 0.75)).normalized(), Vector((0, 1, 0))
    nrm = u.cross(Y)
    for ph in (0.0, math.pi):
        pts = []
        for i in range(11):
            th = -0.35 + (math.tau + 0.7) * i / 10
            c = Vector((gx, 0, gz)) + u * 0.034 * (1 - math.cos(th)) + Y * 0.017 * math.sin(th)
            tang = (u * 0.034 * math.sin(th) + Y * 0.017 * math.cos(th)).normalized()
            b = tang.cross(nrm).normalized()
            w = 3 * th + ph
            pts.append(c + (nrm * math.cos(w) + b * math.sin(w)) * 0.0026)
        k.add(tube(pts, 0.0032, 3), "td_cord" if ph else "td_cord_dk")
    for bm, *_ in k.parts[n0:]:                                                              # the bend: ends curl back
        for v in bm.verts:
            v.co.y += 0.016 * (v.co.x / hw) ** 2 + 0.004 * v.co.x / hw


def turned_leg(k, x, y, h, mat, seg=6):
    """A turned table leg standing on the floor (bun foot, a bead, a long taper, a block under the top)."""
    k.add(lathe([(0.04, -0.01), (0.048, 0.035), (0.03, 0.08), (0.042, 0.13), (0.026, 0.2), (0.032, h - 0.32), (0.044, h - 0.24),
                 (0.03, h - 0.18), (0.04, h - 0.16), (0.04, h)], seg, 30), mat, T((x, y, 0)), smooth=True)


def build_chart_table(k):
    """2 x 1 (interior) the Gullhaven Harbour Office desk, customer side to the front: a carcass with two drawers on
    turned legs; on the left a sloping chart table (front edge 0.9 m, back 1.04 m) with a pale sea chart (blue
    water, a deeper channel, coast lines, an island, a compass star, a red course line; no text), its far edge rolled,
    brass dividers on it and an inkwell with a quill on the flat back rail; on the right a flat counter with the big
    wooden-handled stamp, a red stamp pad in an open tin, a stack of stamped tickets. Rolled charts on the stretcher shelf."""
    rng = random.Random(580)
    for sx in (-1, 1):
        for sy in (-1, 1):
            turned_leg(k, sx * 0.87, sy * 0.34, 0.8, "wood_dk")
    k.add(box(1.86, 0.78, 0.2), "wood_mid", T((0, 0, 0.76)))                                    # frieze
    for sx in (-1, 1):                                                                       # two drawers, customer side
        k.add(box(0.7, 0.02, 0.13), "wood_lt", T((sx * 0.46, -0.395, 0.76)))
        k.add(sphere(0.022, 6, 4), "brass", T((sx * 0.46, -0.41, 0.76)), smooth=True)
    k.add(box(1.72, 0.62, 0.03), "wood_mid", T((0, 0, 0.16)))                                    # stretcher shelf
    for i, (m, y) in enumerate((("td_chart", -0.16), ("td_ticket", 0.0), ("td_chart", 0.16))):
        k.add(cyl(0.05, 0.05, 0.9 - 0.12 * i, 8), m, T((-0.3 + 0.12 * i, y, 0.225), (0, 90, 0)))
    k.add(torus(0.052, 0.008, 8, 3), "td_stamp_red", T((-0.2, -0.16, 0.225), (0, 90, 0)))
    # left: the sloping chart table (a wedge on the frieze, a board, a pencil lip, a flat back rail)
    xl0, xl1, yf, yb, zf, zb = -0.95, 0.25, -0.43, 0.24, 0.9, 1.04
    xc, wl = (xl0 + xl1) / 2, xl1 - xl0
    k.add(prism([(yf, 0.85), (0.43, 0.85), (0.43, zb), (yb, zb), (yf, zf)], wl), "wood_mid", T((xc, 0, 0), (0, 0, 90)))
    th = math.degrees(math.atan2(zb - zf, yb - yf))
    S = T((xc, (yf + yb) / 2, (zf + zb) / 2), (th, 0, 0))                                     # the slope's top plane
    L = math.hypot(yb - yf, zb - zf)
    k.add(box(wl + 0.04, L + 0.02, 0.03), "wood_lt", S @ T((0, 0, 0.0)))
    k.add(box(wl + 0.03, 0.03, 0.04), "wood_dk", S @ T((0, -L / 2 - 0.005, 0.03)))            # pencil lip
    k.add(box(wl + 0.02, 0.2, 0.03), "wood_lt", T((xc, 0.34, zb)))                             # flat back rail
    # the chart, its far edge rolled
    C = S @ T((-0.02, 0.0, 0.018))
    cw, cd = 1.02, 0.5
    k.add(box(cw, cd, 0.004), "td_chart", C)
    k.add(cyl(0.022, 0.022, cw, 8), "td_chart", C @ T((0, cd / 2 + 0.01, 0.02), (0, 90, 0)))
    sea = [(-0.12, -0.25), (0.51, -0.25), (0.51, 0.25), (0.2, 0.25), (0.12, 0.14), (0.16, 0.05), (0.04, -0.03), (-0.04, -0.13)]
    k.add(slab([(x * 0.98, y * 0.98) for x, y in sea], 0.0, 0.0035), "td_chart_sea", C)
    k.add(slab([(0.2, -0.22), (0.46, -0.22), (0.46, 0.2), (0.33, 0.12), (0.26, -0.02), (0.14, -0.1)], 0.0028, 0.0055), "td_chart_deep", C)
    k.add(ribbon([(x * 0.98, y * 0.98) for x, y in sea[3:] + sea[:1]], 0.012, 0.006), "td_chart_coast", C @ T((0, 0, 0.0013)))
    isle = [(0.3, 0.02), (0.36, 0.05), (0.4, 0.0), (0.38, -0.06), (0.31, -0.07), (0.27, -0.03)]
    k.add(slab(isle, 0.004, 0.0105), "td_chart", C)
    k.add(ribbon(isle, 0.01, 0.0065, closed=True), "td_chart_coast", C @ T((0, 0, 0.0055)))
    star = [((0.06 if i % 2 == 0 else 0.018) * math.cos(math.radians(90 * i / 2)), (0.06 if i % 2 == 0 else 0.018) * math.sin(math.radians(90 * i / 2)))
            for i in range(8)]
    k.add(slab([(x - 0.33, y + 0.13) for x, y in star], 0.001, 0.005), "td_stamp_red", C)
    k.add(ribbon([(-0.04, -0.16), (0.14, -0.1), (0.25, -0.05)], 0.008, 0.0048), "td_stamp_red", C @ T((0, 0, 0.0042)))
    hinge = C @ T((-0.2, 0.02, 0.012), (0, 0, 0))                                             # brass dividers, open
    k.add(cyl(0.016, 0.016, 0.012, 8), "brass", hinge)
    for a in (-104, -76):
        e = Vector((0.2 * math.cos(math.radians(a)), 0.2 * math.sin(math.radians(a)), 0))
        beam(k, (hinge @ Vector((0, 0, 0))), (hinge @ e), 0.012, 0.01, "brass")
    k.add(cyl(0.045, 0.05, 0.06, 8), "td_ink", T((-0.75, 0.34, zb + 0.045)))                     # inkwell and quill
    k.add(cyl(0.024, 0.024, 0.02, 8), "brass", T((-0.75, 0.34, zb + 0.08)))
    k.add(prism([(0, 0), (0.012, 0.06), (0.02, 0.2), (0.005, 0.24), (-0.012, 0.12)], 0.006), "td_quill", T((-0.75, 0.34, zb + 0.07), (0, -22, 25)))
    # right: the flat counter with the stamp, the pad, the tickets
    k.add(box(0.7, 0.86, 0.05), "wood_lt", T((0.62, 0, 0.885)))
    k.add(box(0.66, 0.03, 0.07), "wood_dk", T((0.62, 0.41, 0.94)))                             # back gallery
    zt = 0.91
    sx, sy = 0.5, -0.12                                                                     # the big stamp
    k.add(box(0.15, 0.1, 0.022), "td_dark", T((sx, sy, zt + 0.011), (0, 0, 8)))
    k.add(box(0.15, 0.1, 0.04), "wood_mid", T((sx, sy, zt + 0.042), (0, 0, 8)))
    k.add(cyl(0.02, 0.026, 0.07, 8), "wood_lt", T((sx, sy, zt + 0.095)))
    k.add(lathe([(0.026, zt + 0.12), (0.05, zt + 0.15), (0.058, zt + 0.19), (0.045, zt + 0.225), (0.0, zt + 0.24)], 8), "td_stamp_knob", T((sx, sy, 0)), smooth=True)
    k.add(box(0.2, 0.13, 0.024), "td_steel", T((0.78, -0.2, zt + 0.012), (0, 0, -6)))         # pad in an open tin
    k.add(box(0.17, 0.1, 0.006), "td_stamp_red", T((0.78, -0.2, zt + 0.026), (0, 0, -6)))
    k.add(box(0.2, 0.012, 0.13), "td_steel", T((0.785, -0.13, zt + 0.07), (-15, 0, -6)))
    for i in range(6):                                                                       # stamped tickets
        k.add(box(0.2, 0.11, 0.006), "td_ticket" if i % 3 else "td_ticket_dk", T((0.72, 0.18, zt + 0.004 + 0.007 * i), (0, 0, rng.uniform(-12, 12))))
    k.add(torus(0.028, 0.005, 10, 3), "td_stamp_ink", T((0.74, 0.18, zt + 0.045), scale=(1, 1, 0.3)))
    k.add(box(0.2, 0.11, 0.006), "td_ticket", T((0.45, 0.24, zt + 0.004), (0, 0, 30)))       # a stray one
    A.shadow(k, 1.0, 1.0, 0.5)


def build_chowder_stove(k):
    """1 x 1 (interior) The Dry Dock's stove: a squat cast-iron pot-belly on four little legs and a stone hearth,
    a firebox door with a glowing grate (stove_glow), a copper pot of chowder on top with a ladle in it, a flue pipe
    up the back that turns into the wall, a brass coal scuttle at the front right."""
    rng = random.Random(590)
    cx, cy = -0.06, 0.05
    k.add(box(0.94, 0.94, 0.05, 0.01), "td_stone", T((0, 0, 0.0)))
    for i in range(4):                                                                       # legs
        a = math.radians(45 + 90 * i)
        beam(k, (cx + 0.17 * math.cos(a), cy + 0.17 * math.sin(a), 0.2), (cx + 0.24 * math.cos(a), cy + 0.24 * math.sin(a), 0.02), 0.05, 0.05, "td_iron_dk")
    k.add(cyl(0.2, 0.22, 0.1, 12), "td_iron_dk", T((cx, cy, 0.22)))                             # ash pan
    k.add(lathe([(0.2, 0.26), (0.27, 0.32), (0.3, 0.41), (0.29, 0.51), (0.25, 0.59), (0.21, 0.63)], 12, 15), "td_castiron", T((cx, cy, 0)), smooth=True)
    k.add(torus(0.215, 0.022, 12, 3), "brass", T((cx, cy, 0.63)))
    k.add(cyl(0.27, 0.25, 0.05, 12), "td_iron_dk", T((cx, cy, 0.665)))                          # top plate
    # firebox door: a frame, the glowing grate behind three bars, a brass knob; an ash door under it
    fy = cy - 0.3
    k.add(box(0.26, 0.06, 0.22, 0.01), "td_iron_dk", T((cx, fy + 0.01, 0.42)))
    k.add(box(0.2, 0.012, 0.16), "stove_glow", T((cx, fy - 0.024, 0.42)))
    for z in (0.375, 0.42, 0.465):
        k.add(box(0.21, 0.016, 0.014), "td_iron_dk", T((cx, fy - 0.034, z)))
    k.add(sphere(0.02, 6, 4), "brass", T((cx + 0.15, fy - 0.03, 0.42)), smooth=True)
    k.add(box(0.16, 0.03, 0.06), "td_iron_dk", T((cx, cy - 0.215, 0.22)))
    k.add(box(0.12, 0.012, 0.012), "brass", T((cx, cy - 0.235, 0.215)))
    # the copper pot, the chowder, a few bits, the ladle
    py = cy - 0.06
    k.add(lathe([(0.0, 0.69), (0.15, 0.69), (0.185, 0.72), (0.195, 0.85), (0.205, 0.9), (0.18, 0.9), (0.174, 0.86)], 12), "td_copper", T((cx, py, 0)), smooth=True)
    k.add(cyl(0.174, 0.174, 0.012, 12), "td_chowder", T((cx, py, 0.877)))
    for i, m in enumerate(("td_potato", "td_carrot", "td_potato", "td_turf", "td_carrot")):
        a = i * 1.3 + 0.4
        r = 0.05 + 0.025 * i
        k.add(jitter(ico(0.017, 0), rng, 0.004), m, T((cx + r * math.cos(a), py + r * math.sin(a), 0.885), scale=(1, 1, 0.6)))
    for s in (-1, 1):
        k.add(torus(0.04, 0.01, 8, 3), "td_copper_dk", T((cx + s * 0.212, py, 0.84), (90, 0, 0)))
    beam(k, (cx + 0.04, py - 0.02, 0.86), (cx + 0.18, py - 0.16, 1.12), 0.024, 0.014, "td_steel")
    k.add(torus(0.016, 0.005, 6, 3), "td_steel", T((cx + 0.185, py - 0.165, 1.14), (90, 0, 45)))
    # flue up the back with a damper key, a collar and a little rain hat
    fx, fyb = cx, cy + 0.2
    k.add(cyl(0.06, 0.06, 0.74, 8), "td_castiron", T((fx, fyb, 1.05)))
    k.add(cyl(0.072, 0.072, 0.04, 8), "td_iron_dk", T((fx, fyb, 0.71)))
    k.add(cyl(0.072, 0.072, 0.04, 8), "td_iron_dk", T((fx, fyb, 1.2)))
    k.add(box(0.2, 0.02, 0.02), "brass", T((fx + 0.07, fyb, 0.98)))
    k.add(cyl(0.07, 0.07, 0.03, 8), "td_iron_dk", T((fx, fyb, 1.42)))
    for z in (1.445, 1.47):
        k.add(cyl(0.012, 0.012, 0.025, 4), "td_iron_dk", T((fx + 0.045, fyb, z)))
    k.add(cyl(0.12, 0.0, 0.07, 8), "td_castiron", T((fx, fyb, 1.5)))
    # brass coal scuttle and coal at the front right
    sx, sy = 0.31, -0.3
    k.add(lathe([(0.0, 0.025), (0.09, 0.025), (0.11, 0.1), (0.125, 0.2)], 10), "brass", T((sx, sy, 0)), smooth=True)
    k.add(jitter(ico(0.115, 1), rng, 0.012), "td_coal", T((sx, sy, 0.19), scale=(1, 1, 0.45)))
    for i in range(3):
        a = i * 2.1
        k.add(jitter(ico(0.03, 0), rng, 0.008), "td_coal", T((sx + 0.07 * math.cos(a), sy + 0.07 * math.sin(a), 0.235)))
    k.add(torus(0.1, 0.012, 10, 3), "td_iron_dk", T((sx, sy, 0.21), (90, 0, 0)))
    for bm, *_ in k.parts[-1:]:                                                              # handle: the top half only
        bmesh.ops.delete(bm, geom=[v for v in bm.verts if v.co.z < 0.205], context="VERTS")


PROPS = {
    "harbour_house": build_harbour_house, "ferry": build_ferry, "bell_tower": build_bell_tower,
    "crab_pots": build_crab_pots, "buoy": build_buoy, "net_rack": build_net_rack, "cliff_sea": build_cliff_sea,
    "cliff_sea_b": build_cliff_sea_b, "tidepool": build_tidepool, "rock_sea": build_rock_sea, "bollard": build_bollard,
    "bell_clapper": build_bell_clapper, "ferry_pass": build_ferry_pass, "chart_table": build_chart_table,
    "chowder_stove": build_chowder_stove,
}

VIGNETTE = dict(
    ground=("sand", "wood_floor", "#e2c999"),
    overlays={"w": "wet_sand"},
    water="#5cbdc2",
    cliff=("cliff_sea", "cliff_sea_b"),
    ring=["rock_sea", "reeds", "rock"],
    sea_beyond="e",
    tiles=[
        "33.......~~~~~~~~~~~",
        "32.......~~~~~~~~~~~",
        "2........~~~~~~~~~~~",
        "1........~~~~~~~~~~~",
        "..========HHHH~~~~~~",
        "www.......~~~~~~~~~~",
        "wwww.....~~~~~~~~~~~",
        "wwwww..~~~~~~~~~~~~~",
        "wwwwww~~~~~~~~~~~~~~",
        "wwww~~~~~~~~~~~~~~~~",
        "w~~~~~~~~~~~~~~~~~~~",
        "~~~~~~~~~~~~~~~~~~~~",
        "~~~~~~~~~~~~~~~~~~~~",
        "~~~~~~~~~~~~~~~~~~~~",
    ],
    props=[("harbour_house", 3, 0), ("net_rack", 8, 3), ("crab_pots", 2, 3), ("crab_pots", 8, 2),
           ("ferry", 10, 5), ("bollard", 13, 4, 0, 0.3), ("bollard", 10, 4, 0, 0.3), ("bell_tower", 15, 1),
           ("buoy", 17, 6), ("buoy", 9, 9), ("rock_sea", 6, 6), ("rock_sea", 5, 9), ("rock_sea", 13, 10),
           ("tidepool", 1, 6), ("tidepool", 3, 7), ("tidepool", 0, 9), ("hero", 7, 4, 0, 0.1)],
    aim=(10.0, -5.8, 0), dist=19,
)
FOOT_OTHER = {}
