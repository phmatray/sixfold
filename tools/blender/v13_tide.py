"""Tide: The Saltreach (Water). Harbour life on stilts, nets drying, painted boats, gulls, wide tidal flats.

Vocabulary: teal clapboard and red roofs on stilts with barnacled feet, navy-and-white hulls, cork-orange floats,
wave-worn blue-grey rock with barnacles, mussels and a green weed line, sea pinks on the cliff tops.
Glow parts: td_window_glow (harbour house and ferry windows), lamp_glow (the bell tower's night lamp, the ferry's
masthead light), td_buoy_glow (the buoy's lamp). td_pool_water is the tide pool's surface (rippling water shader).
"""
import math
import random

import bmesh
from mathutils import Vector

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
}

WIND = {}
FOOT = {"harbour_house": (5, 4), "ferry": (4, 2), "bell_tower": (3, 3), "net_rack": (2, 1)}
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


PROPS = {
    "harbour_house": build_harbour_house, "ferry": build_ferry, "bell_tower": build_bell_tower,
    "crab_pots": build_crab_pots, "buoy": build_buoy, "net_rack": build_net_rack, "cliff_sea": build_cliff_sea,
    "cliff_sea_b": build_cliff_sea_b, "tidepool": build_tidepool, "rock_sea": build_rock_sea, "bollard": build_bollard,
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
