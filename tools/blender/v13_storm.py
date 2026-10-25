"""Storm: The Galewold (Lightning). High heath on sea cliffs, dry-stone walls, chime posts thrumming in the wind,
copper and verdigris, weather always moving.

Vocabulary: grey granite with a turf lip (cliffs, walls), copper gone green (the observatory dome, the Rodspire,
pylon coils), tall thin masts with stays and cables, purple heather, pennants that all point downwind (+X).
Glow parts: coil_glow (pylon coil, lift cage lamp, Rodspire orb: electric cyan), lamp_glow (observatory slit,
windows, the lift landings).
"""
import math
import random

import bmesh

from v13_kit import (B, T, beam, box, contact, cyl, empty, ico, jitter, prism, ribbon, shadow, sphere,  # noqa: F401
                     torus, tube, curve, half_sphere, arch_pts, arc, stratum, FRONT, clamp01, chain, open_part, lathe)

PALETTE = {
    "st_granite": "#9c9c96", "st_granite_dk": "#6f716e", "st_granite_lt": "#b8b6ad", "st_granite_warm": "#a99f90",
    "st_lichen": "#b3ad86", "st_turf": "#76804e", "st_turf_dk": "#5f6a40",
    "st_heath_leaf": "#55603c", "st_heath_leaf_lt": "#6c7748", "st_heather": "#a65ca0", "st_heather_lt": "#cf8fc6",
    "st_heather_dk": "#7c4380",
    "st_verdigris": "#5fae96", "st_verdigris_lt": "#7cc4ab", "st_verdigris_dk": "#3f8572",
    "st_whitewash": "#ece5d6", "st_whitewash_dk": "#d3cab8", "st_door": "#2f4a6b",
    "st_porcelain": "#f2efe6", "st_pennant": "#d9573f", "st_pennant_lt": "#f2c14a",
    "st_steel": "#59626c", "st_steel_dk": "#3f464f",
    "coil_glow": "#9feaff", "st_bottle": "#cfe4ec",
    "copper": "#c27a43", "copper_dk": "#9a5a2e",   # same values as v13_ember (shared keys)
}

WIND = {
    "chime_post": lambda x, y, z, r: clamp01((z - 0.9) / 1.4) * 0.55,
    "heather": lambda x, y, z, r: clamp01(z / 0.5) * 0.8,
}

FOOT = {"observatory": (5, 5), "lift_tower": (2, 1), "rodspire": (3, 3), "telescope": (2, 2)}
ANIM = {"pylon"}

LIGHT = dict(sun=("#f2f5ff", 2.3), from_hero=(8, 18, 6), hemi=("#dfe8f2", "#4f5b3c", 1.5), bg="#b9c6cf", fog=(14, 32))


def drop_bottom(bm):
    """Remove a box's bottom face (hidden; keeps stacked bottoms from sharing a plane)."""
    import bmesh
    zmin = min(v.co.z for v in bm.verts)
    bmesh.ops.delete(bm, geom=[f for f in bm.faces if f.normal.z < -0.9 and all(abs(v.co.z - zmin) < 1e-5 for v in f.verts)], context="FACES")
    return bm


def stone(rng, w, d, h, amt=0.006):
    return jitter(box(w, d, h), rng, amt)


# ---------------------------------------------------------------- terrain: granite with a turf lip


def build_cliff_heath(k, seed=401, tufted=False):
    """1 x 1 x 1 weathered granite: blocky grey strata to the tile edges, a band of turf on top, flat at 1.0."""
    rng = random.Random(seed)
    for z0, z1, m, b in ((-0.02, 0.34, "st_granite_dk", 0.03), (0.34, 0.62, "st_granite", 0.035),
                         (0.62, 0.86, "st_granite_lt", 0.025)):
        k.add(stratum(rng, z0, z1, b), m)
    k.add(stratum(rng, 0.86, 1.0, 0.0, inward_top=True), "st_turf")
    for _ in range(3):  # lichen dabs on the top
        r = rng.uniform(0.03, 0.05)
        k.add(jitter(ico(r, 1), rng, r * 0.2), "st_lichen", T((rng.uniform(-0.35, 0.35), rng.uniform(-0.35, 0.35), 1.0), scale=(1.2, 1, 0.2)))
    if tufted:  # a heather clump and a loose boulder on the top
        for x, y, r in ((-0.18, 0.12, 0.17), (0.02, 0.22, 0.13), (-0.25, -0.1, 0.12)):
            k.add(jitter(ico(r, 1), rng, r * 0.15), rng.choice(["st_heath_leaf", "st_heath_leaf_lt"]), T((x, y, 1.0 + r * 0.35), scale=(1, 1, 0.7)), smooth=True)
        for _ in range(9):
            a, d = rng.uniform(0, math.tau), rng.uniform(0, 0.22)
            h = rng.uniform(0.1, 0.18)
            k.add(cyl(0.022, 0.0, h, 3), rng.choice(["st_heather", "st_heather_lt"]), T((-0.13 + d * math.cos(a), 0.08 + d * math.sin(a), 1.08 + rng.uniform(0, 0.06) + h / 2)))
        k.add(jitter(ico(0.15, 1), rng, 0.03), "st_granite_lt", T((0.24, -0.2, 1.06), scale=(1.2, 1, 0.7)))
    else:
        for x, y, r in ((0.22, 0.24, 0.1), (-0.28, -0.2, 0.08)):
            k.add(jitter(ico(r, 1), rng, r * 0.15), "st_turf_dk", T((x, y, 1.0 + r * 0.3), scale=(1.2, 1, 0.6)), smooth=True)
        for _ in range(3):
            r = rng.uniform(0.02, 0.04)
            k.add(jitter(ico(r, 1), rng, r * 0.2), "pebble", T((rng.uniform(-0.38, 0.38), rng.uniform(-0.38, 0.38), 1.0), scale=(1, 0.8, 0.3)))


# ---------------------------------------------------------------- dry-stone walls (auto-tile set, as the fences)

WALL_COURSES = (0.24, 0.22, 0.19, 0.16)   # course heights from the ground; capstones on top (~0.95 m)


def wall_t(z):
    """Battered wall: 0.56 m thick at the foot, 0.38 m under the capstones."""
    return 0.56 - 0.2 * z / 0.81


def build_wall_dry(k, arms, seed):
    """Dry-stone wall through the tile centre: a pier and an arm to each joined edge (B.ARM_DIR, like the fence
    pieces), four courses of rough granite with dark joints, a row of upright capstones."""
    rng = random.Random(seed)
    stones = ["st_granite", "st_granite", "st_granite", "st_granite_lt", "st_granite_warm", "st_granite_warm"]
    z = -0.02
    for ch in WALL_COURSES:
        t = wall_t(z + ch / 2)
        h = ch - 0.012
        zc = z + ch / 2
        # pier: one or two stones filling the centre square
        if rng.random() < 0.5:
            k.add(stone(rng, t, t, h), rng.choice(stones), T((0, 0, zc), (0, 0, rng.uniform(-3, 3))))
        else:
            for s in (-1, 1):
                k.add(stone(rng, t / 2 - 0.01, t, h), rng.choice(stones), T((s * t / 4, 0, zc)))
        for arm in arms:
            dx, dy = B.ARM_DIR[arm]
            a = t / 2 + 0.004
            while a < 0.49:
                L = min(rng.uniform(0.17, 0.3), 0.5 - a)
                if 0.5 - (a + L) < 0.08:  # no sliver at the tile edge
                    L = 0.5 - a
                c = a + L / 2
                sx, sy = (L - 0.004, t) if dx else (t, L - 0.004)
                k.add(stone(rng, sx, sy, h * rng.uniform(0.94, 1.0)), rng.choice(stones), T((dx * c, dy * c, zc)))
                a += L
        z += ch
    # joints: a dark core just inside the stones (pier, then each arm from the pier's face to the edge)
    core_h = z - 0.03
    tt = wall_t(0.81) - 0.05   # narrower than the thinnest course, so it only shows through the joints
    k.add(drop_bottom(box(tt, tt, core_h)), "st_granite_dk", T((0, 0, core_h / 2 - 0.02)))
    for arm in arms:
        dx, dy = B.ARM_DIR[arm]
        L = 0.5 - tt / 2
        k.add(drop_bottom(box(L if dx else tt - 0.004, tt - 0.004 if dx else L, core_h - 0.004)), "st_granite_dk",
              T((dx * (tt / 2 + L / 2), dy * (tt / 2 + L / 2), core_h / 2 - 0.022)))
    # coping: a regular row of upright stones every 10 cm, all leaning toward +X (or +Y), tall and short in
    # turn (cock and hen). Positions sit on a fixed grid so runs carry on seamlessly across tiles.
    top = z
    tw = wall_t(0.81) + 0.02
    half = tw / 2
    axes = {"x": [B.ARM_DIR[a][0] for a in arms if B.ARM_DIR[a][0]], "y": [B.ARM_DIR[a][1] for a in arms if B.ARM_DIR[a][1]]}
    owner = "x" if axes["x"] or not axes["y"] else "y"
    for ax, signs in axes.items():
        if ax == owner:
            lo = -0.45 if -1 in signs else -half + 0.05
            hi = 0.45 if 1 in signs else half - 0.05
        else:
            lo = -0.45 if -1 in signs else half + 0.05
            hi = 0.45 if 1 in signs else -half - 0.05
        for i in range(10):
            u = -0.45 + 0.1 * i
            if not (lo - 1e-6 <= u <= hi + 1e-6) or (ax != owner and abs(u) < half + 0.04):
                continue
            hh = 0.17 if i % 2 == 0 else 0.13
            if ax == "x":
                k.add(stone(rng, 0.075, tw, hh, 0.005), rng.choice(stones), T((u, 0, top + hh / 2 - 0.02), (0, 14, 0)))
            else:
                k.add(stone(rng, tw, 0.075, hh, 0.005), rng.choice(stones), T((0, u, top + hh / 2 - 0.02), (-14, 0, 0)))


# ---------------------------------------------------------------- nature


def build_heather(k):
    """1 x 1 heather (~0.5 m): low, wide olive cushions, each crowned with a lilac bloom and sprigs of purple beads."""
    rng = random.Random(411)
    for (x, y), r, m in (((0.02, 0.04), 0.3, "st_heath_leaf"), ((0.24, -0.13), 0.22, "st_heath_leaf_lt"),
                         ((-0.24, -0.1), 0.24, "st_heath_leaf")):
        k.add(jitter(ico(r, 1), rng, r * 0.1), m, T((x, y, r * 0.42), scale=(1.15, 1.1, 0.62)), smooth=True)
        k.add(jitter(ico(r * 0.8, 0), rng, r * 0.06), "st_heather" if r > 0.23 else "st_heather_dk",
              T((x, y, r * 0.42 + r * 0.5), scale=(1.1, 1.05, 0.42)), smooth=True)
    for x, y, z, m in ((0.16, -0.02, 0.36, "st_heather"), (-0.18, 0.1, 0.37, "st_heather_dk"), (0.1, 0.12, 0.42, "st_heather_lt"), (-0.08, -0.06, 0.43, "st_heather_lt"), (0.24, -0.06, 0.32, "st_heather"),
                       (-0.3, -0.04, 0.33, "st_heather_lt"), (-0.02, 0.24, 0.38, "st_heather_dk"), (0.3, -0.2, 0.28, "st_heather_lt")):
        k.add(sphere(0.058, 6, 4), m, T((x, y, z), (0, 0, rng.uniform(0, 70)), (1, 1, 1.15)), smooth=True)
    k.add(drop_bottom(cyl(0.44, 0.44, 0.002, 12)), "contact_in", T((0, 0, 0.004)))


# ---------------------------------------------------------------- the Downs and the relay line


def build_chime_post(k):
    """1 x 1 chime post (~2.3 m): a weathered post on a granite foot, a crossbar hung with copper, brass and
    verdigris tubes, a wind sail under the striker, a copper cap and a pennant streaming downwind."""
    rng = random.Random(421)
    k.add(stone(rng, 0.36, 0.36, 0.16, 0.02), "st_granite", T((0, 0, 0.06)))
    k.add(box(0.12, 0.12, 2.0, 0.015), "plank_grey", T((0, 0, 1.06)))
    for z in (0.5, 1.4):  # iron straps
        k.add(box(0.135, 0.135, 0.04), "iron", T((0, 0, z)))
    k.add(box(0.9, 0.1, 0.09, 0.012), "plank_grey_dk", T((0, 0, 1.92)))
    beam(k, (0, 0.01, 1.55), (-0.3, 0.01, 1.875), 0.05, 0.04, "plank_grey_dk")
    beam(k, (0, -0.012, 1.55), (0.3, -0.012, 1.875), 0.05, 0.04, "plank_grey_dk")
    k.add(cyl(0.12, 0.0, 0.16, 4), "copper", T((0, 0, 2.14), (0, 0, 45)))
    k.add(box(0.15, 0.15, 0.03), "copper_dk", T((0, 0, 2.065)))
    # pennant on a thin staff, streaming toward +X
    k.add(cyl(0.012, 0.012, 0.35, 4), "iron", T((0, 0, 2.38)))
    k.add(prism([(0, 0), (0.34, -0.05), (0.3, -0.07), (0.36, -0.11), (0, -0.16)], 0.012), "st_pennant", T((0.01, 0, 2.55)))
    k.add(prism([(0, -0.055), (0.2, -0.075), (0, -0.1)], 0.022), "st_pennant_lt", T((0.012, 0.001, 2.55)))
    # chime tubes from both arms
    for sx in (-1, 1):
        for i, (L, m) in enumerate(((0.52, "copper"), (0.66, "brass"), (0.42, "st_verdigris"))):
            x = sx * (0.18 + 0.1 * i)
            y = (i - 1) * 0.035
            k.add(box(0.008, 0.008, 0.08), "rope", T((x, y, 1.84)))
            k.add(cyl(0.03, 0.03, L, 6), m, T((x, y, 1.8 - L / 2)), smooth=True)
            k.add(cyl(0.034, 0.034, 0.024, 6), "copper_dk", T((x, y, 1.8 + 0.004)))
        # striker disc and a little wind sail hanging under the tubes
        k.add(box(0.006, 0.006, 0.5), "rope", T((sx * 0.28, 0.06, 1.6)))
        k.add(cyl(0.07, 0.07, 0.025, 8), "wood", T((sx * 0.28, 0.06, 1.47)))
        k.add(box(0.12, 0.012, 0.18), "st_porcelain", T((sx * 0.28, 0.06, 1.25), (0, 0, 15 * sx)))
    contact(k, 0.3)


def build_pylon(k):
    """1 x 1 relay pylon (~3 m): a four-legged timber mast braced with iron, porcelain insulators, a copper-wound
    coil (coil_glow) under a copper ball, a cable down to a switch box whose knife lever is the `Open` clip."""
    rng = random.Random(431)
    leg = lambda sx, sy, z: (sx * (0.34 - 0.24 * z / 2.2), sy * (0.34 - 0.24 * z / 2.2), z)  # noqa: E731
    for sx in (-1, 1):
        for sy in (-1, 1):
            k.add(stone(rng, 0.2, 0.2, 0.12, 0.01), "st_granite", T((sx * 0.34, sy * 0.34, 0.04)))
            beam(k, leg(sx, sy, 0.0), leg(sx, sy, 2.24), 0.08, 0.08, "wood_dk")
    for z0, z1 in ((0.25, 0.95), (0.95, 1.6), (1.6, 2.15)):  # X braces on each face
        for f in range(4):
            a, b = [(-1, -1), (1, -1), (1, 1), (-1, 1)][f], [(-1, -1), (1, -1), (1, 1), (-1, 1)][(f + 1) % 4]
            pa0, pb1 = leg(*a, z0), leg(*b, z1)
            pa1, pb0 = leg(*a, z1), leg(*b, z0)
            off = lambda p: (p[0] * 1.08, p[1] * 1.08, p[2])  # noqa: E731  (just outside the legs)
            beam(k, off(pa0), off(pb1), 0.03, 0.02, "iron")
            beam(k, off(pb0), off(pa1), 0.03, 0.02, "iron")
    k.add(box(0.34, 0.34, 0.06, 0.01), "st_steel", T((0, 0, 2.25)))
    for i, z in enumerate((2.33, 2.41, 2.49)):  # insulator stack
        k.add(cyl(0.11 - 0.015 * i, 0.11 - 0.015 * i, 0.05, 10), "st_porcelain", T((0, 0, z)), smooth=True)
        k.add(cyl(0.05, 0.05, 0.03, 8), "st_porcelain", T((0, 0, z + 0.04)))
    k.add(cyl(0.085, 0.085, 0.42, 10), "coil_glow", T((0, 0, 2.76)))
    for i in range(6):  # copper windings
        k.add(torus(0.1, 0.018, 12, 4), "copper", T((0, 0, 2.6 + 0.065 * i)))
    k.add(cyl(0.11, 0.11, 0.03, 10), "copper_dk", T((0, 0, 2.985)))
    k.add(sphere(0.12, 12, 8), "copper", T((0, 0, 3.08)), smooth=True)
    # cable from the coil down the front-right leg to the switch box
    k.add(tube([(0.1, -0.05, 2.62), (0.22, -0.16, 2.35)] + [(lambda p: (p[0] + 0.05, p[1] - 0.04, p[2]))(leg(1, -1, z)) for z in (2.0, 1.4, 0.95)] + [(0.13, -0.31, 0.82)],
               0.014, 4), "st_steel_dk")
    # switch box on the front legs, its lever pivot on the front face
    k.add(box(0.36, 0.14, 0.42, 0.015), "st_steel", T((0, -0.3, 0.7)))
    k.add(box(0.3, 0.02, 0.34), "st_steel_dk", T((0, -0.375, 0.7)))
    k.add(cyl(0.05, 0.05, 0.03, 8), "brass", T((0, -0.39, 0.7), (90, 0, 0)))
    for x in (-0.11, 0.11):  # the two contacts the blade swings between
        k.add(box(0.04, 0.04, 0.06), "copper", T((x * 1.6, -0.395, 0.86)))
    k.add(box(0.44, 0.18, 0.04, 0.01), "st_steel_dk", T((0, -0.3, 0.93)))   # little rain hood
    contact(k, 0.5)

    def lever(lk):
        lk.add(box(0.035, 0.02, 0.26), "copper", T((0, 0, 0.13)))
        lk.add(cyl(0.012, 0.012, 0.16, 6), "iron", T((0, -0.02, 0.27), (0, 90, 0)))
        lk.add(sphere(0.04, 8, 6), "st_pennant", T((0, -0.06, 0.27)), smooth=True)
        lk.add(cyl(0.02, 0.02, 0.07, 6), "iron", T((0, -0.04, 0.27), (90, 0, 0)))
    # rest: blade toward the left contact (off); Open: over to the right (on)
    return open_part(lever, (0, -0.41, 0.7), [(0, (0, -40, 0), (1, 1, 1)), (12, (0, 40, 0), (1, 1, 1))], "pylon_lever")


# ---------------------------------------------------------------- Stormwatch


def build_observatory(k):
    """5 x 5 observatory: a whitewashed granite drum with a walkable gallery at its top, a verdigris copper dome
    with ribs and an open slit (lamp_glow) the brass telescope pokes out of, anemometer cups on the crown, an
    arched door on the front centre column (door tile = origin + (2, 4)), arched windows."""
    rng = random.Random(441)
    R, top = 2.0, 2.6
    k.add(cyl(R + 0.25, R + 0.3, 0.3, 16, 0.03), "st_granite", T((0, 0, 0.13)))
    k.add(cyl(R, R, top - 0.3, 16), "st_whitewash", T((0, 0, 0.28 + (top - 0.3) / 2)), smooth=True)
    for i in range(16):  # granite quoins round the foot of the drum
        a = (i + 0.5) / 16 * math.tau
        if abs(math.degrees(a) - 270) < 25:
            continue
        k.add(stone(rng, 0.34, 0.12, 0.26, 0.01), rng.choice(["st_granite_lt", "st_granite"]), T(((R + 0.02) * math.cos(a), (R + 0.02) * math.sin(a), 0.42), (0, 0, math.degrees(a) + 90)))
    k.add(cyl(R + 0.06, R + 0.06, 0.14, 16), "st_granite_lt", T((0, 0, top - 0.07)))
    # gallery: a ring deck round the dome, railing posts and a rail
    k.add(cyl(R + 0.3, R + 0.3, 0.08, 16), "st_granite", T((0, 0, top + 0.04)))
    for i in range(16):
        a = i / 16 * math.tau
        k.add(box(0.04, 0.04, 0.55), "iron", T(((R + 0.24) * math.cos(a), (R + 0.24) * math.sin(a), top + 0.35)))
    k.add(torus(R + 0.24, 0.025, 16, 4), "iron", T((0, 0, top + 0.62)))
    k.add(torus(R + 0.245, 0.015, 16, 4), "iron", T((0, 0, top + 0.37)))
    # dome, ribs, slit, telescope
    D = 1.75
    k.add(cyl(D + 0.05, D + 0.05, 0.16, 16), "copper_dk", T((0, 0, top + 0.16)))
    k.add(half_sphere(D, 16, 8), "st_verdigris", T((0, 0, top + 0.235)), smooth=True)
    slit = math.radians(-60)   # the slit faces front-right
    for i in range(8):
        az = slit + math.radians(22.5) + i * math.tau / 8
        pts = [(D * math.cos(e) * math.cos(az), D * math.cos(e) * math.sin(az), top + 0.24 + D * math.sin(e)) for e in (math.radians(v) for v in (0, 20, 40, 60, 80))]
        k.add(tube([(x * 1.012, y * 1.012, z) for x, y, z in pts], 0.03, 4), "st_verdigris_dk")
    for side in (-1, 1):  # shutter edges either side of the slit
        az = slit + side * math.radians(9)
        pts = [((D + 0.03) * math.cos(e) * math.cos(az), (D + 0.03) * math.cos(e) * math.sin(az), top + 0.24 + (D + 0.03) * math.sin(e)) for e in (math.radians(v) for v in (5, 25, 45, 65, 85))]
        k.add(tube(pts, 0.05, 4), "copper")
    for e0 in range(8, 84, 12):  # the slit itself: lit panels set into the dome
        e = math.radians(e0 + 6)
        r = D + 0.005
        k.add(box(0.36 - 0.02 * ((e0 // 12) % 2), 0.04, D * math.radians(12) * 0.95), "lamp_glow",
              T((r * math.cos(e) * math.cos(slit), r * math.cos(e) * math.sin(slit), top + 0.24 + r * math.sin(e)),
                (0, 0, math.degrees(slit) + 90)) @ T((0, 0, 0), (math.degrees(e) - 90, 0, 0)))
    e = math.radians(38)
    tdir = (math.cos(e) * math.cos(slit), math.cos(e) * math.sin(slit), math.sin(e))
    a = (tdir[0] * 1.0, tdir[1] * 1.0, top + 0.24 + tdir[2] * 1.0)
    b = (tdir[0] * 2.45, tdir[1] * 2.45, top + 0.24 + tdir[2] * 2.45)
    k.add(tube([a, b], 0.13, 10, taper=0.8), "brass")
    k.add(tube([b, (b[0] + tdir[0] * 0.08, b[1] + tdir[1] * 0.08, b[2] + tdir[2] * 0.08)], 0.12, 10), "copper_dk")
    # crown: finial, anemometer cups, lightning rod
    k.add(cyl(0.2, 0.12, 0.12, 8), "copper_dk", T((0, 0, top + 0.24 + D)))
    k.add(cyl(0.025, 0.025, 0.75, 6), "iron", T((0, 0, top + 0.24 + D + 0.4)))
    for i in range(3):
        a3 = i / 3 * math.tau + 0.4
        tip = (0.32 * math.cos(a3), 0.32 * math.sin(a3), top + 0.24 + D + 0.62)
        beam(k, (0, 0, tip[2] + 0.008 * i), (tip[0], tip[1], tip[2] + 0.008 * i), 0.018, 0.018, "iron")
        k.add(half_sphere(0.08, 8, 4), "copper", T(tip, (0, 90, math.degrees(a3) + 90)))
    k.add(cyl(0.02, 0.0, 0.3, 4), "copper", T((0, 0, top + 0.24 + D + 0.92)))
    # door: arched, recessed in a granite frame, two steps
    fy = -R
    k.add(ribbon(arch_pts(1.12, 2.0), 0.16, 0.18), "st_granite_lt", T((0, fy + 0.08, 0.27), FRONT))
    k.add(prism(arch_pts(0.92, 1.88), 0.1), "st_door", T((0, fy + 0.02, 0.27)))
    for x in (-0.2, 0.2):
        k.add(box(0.04, 0.03, 1.4), "st_steel_dk", T((x, fy - 0.04, 1.0)))
    k.add(sphere(0.045, 6, 4), "brass", T((0.3, fy - 0.06, 1.05)))
    k.add(box(1.4, 0.4, 0.16, 0.02), "st_granite", T((0, fy - 0.24, 0.065)))
    k.add(box(1.2, 0.3, 0.14, 0.02), "st_granite_lt", T((0, fy - 0.2, 0.2)))
    k.add(cyl(0.16, 0.16, 0.05, 6), "hex", T((0, fy - 0.07, 2.42), (90, 30, 0)))
    k.add(cyl(0.09, 0.09, 0.05, 6), "hex_in", T((0, fy - 0.09, 2.42), (90, 30, 0)))
    # arched windows, lit at night
    for az in (222, 318, 60, 120):
        a = math.radians(az)
        M = T(((R + 0.005) * math.cos(a), (R + 0.005) * math.sin(a), 1.25), (0, 0, az - 90 + 180))
        k.add(prism(arch_pts(0.42, 0.72), 0.06), "lamp_glow", M @ T((0, 0.0, 0)))
        k.add(ribbon(arch_pts(0.52, 0.78), 0.06, 0.085), "st_granite_lt", M @ T((0, 0.035, -0.03), FRONT))
        k.add(box(0.03, 0.08, 0.7), "st_steel_dk", M @ T((0, -0.03, 0.36)))
    contact(k, 2.45)


LIFT_TOP = 6.0     # the summit the lift serves (the Rodspire stack)
LIFT_HEAD = 9.1    # the head sheave's axle: where the cables end (lift.js: `head` = top + 3.1)


def build_lift_tower(k):
    """2 x 1 lift tower against a 6 m cliff (the cliff face on the tile edge behind it, +Y). East tile (x 0..1):
    the shaft the cage rides in, on two iron guide rails; west tile: a bay whose top carries the winch house.
    Weathered timber posts and X braces, anchored into the rock by beams with iron plates; a head frame over
    the shaft with a copper-rimmed sheave whose east rim drops the cage cable and whose west rim takes the winch
    cable; a plank landing with rails on the summit behind the shaft; lamps (lamp_glow) at both landings."""
    rng = random.Random(452)
    S, Wb, P = 0.5, -0.5, 0.11                       # shaft and bay centres (x), post section
    XS = (-0.94, 0.0, 1.0)                           # post lines
    YF, YB = -0.44, 0.42                             # front and back post rows
    wood = lambda: rng.choice(["plank_grey_dk", "plank_grey", "plank_grey_dk"])  # noqa: E731
    # footings and ground sills
    for x in XS:
        for y in (YF, YB):
            k.add(stone(rng, 0.24, 0.24, 0.14, 0.01), "st_granite", T((x, y, 0.05)))
    for y in (YF, YB):
        k.add(box(2.06, 0.12, 0.1), "plank_grey_dk", T((0.03, y, 0.13)))
    # posts: the shaft's four up to the head frame, the bay's two up to the winch floor
    for x in XS:
        for y in (YF, YB):
            top = LIFT_TOP + 0.02 if x < -0.5 else 8.52
            k.add(box(P, P, top - 0.08), wood(), T((x, y, 0.08 + (top - 0.08) / 2)))
    # girts: shaft sides at every level; front only clear of the doorway and the rider; back only below the summit
    for z in (1.4, 2.7, 4.0, 5.3, 6.6, 7.9):
        for x in (0.0, 1.0):
            k.add(box(0.09, YB - YF + 0.1, 0.1), wood(), T((x + (0.012 if x else -0.012), (YF + YB) / 2, z)))
    for z in (2.7, 4.3):
        k.add(box(1.1, 0.09, 0.1), wood(), T((S, YF - 0.03, z)))
    for z in (1.4, 2.7, 4.0, 5.3):
        k.add(box(1.1, 0.09, 0.1), wood(), T((S, YB + 0.05, z)))
    for z in (1.4, 2.7, 4.0, 5.3):                   # the bay's own girts, front, back and west side
        for y in (YF, YB):
            k.add(box(1.04, 0.09, 0.1), wood(), T((Wb - 0.02, y + (-0.012 if y < 0 else 0.012), z)))
        k.add(box(0.09, YB - YF + 0.1, 0.1), wood(), T((-0.952, (YF + YB) / 2, z)))
    # X braces (iron straps on the east side and over the bay's front and west side)
    def cross(a, b, c, d, off):
        beam(k, a, b, 0.035, 0.025, "iron")
        beam(k, tuple(v + o for v, o in zip(c, off)), tuple(v + o for v, o in zip(d, off)), 0.035, 0.025, "iron")
    lv = (0.2, 1.4, 2.7, 4.0, 5.3, 6.6, 7.9)
    for z0, z1 in zip(lv, lv[1:]):
        cross((1.07, YF, z0), (1.07, YB, z1), (1.07, YB, z0), (1.07, YF, z1), (0.02, 0, 0))
    for z0, z1 in zip(lv[:5], lv[1:6]):
        cross((-0.94, YF - 0.07, z0), (0.0, YF - 0.07, z1), (0.0, YF - 0.07, z0), (-0.94, YF - 0.07, z1), (0, -0.02, 0))
        cross((-1.01, YF, z0), (-1.01, YB, z1), (-1.01, YB, z0), (-1.01, YF, z1), (-0.02, 0, 0))
    # guide rails either side of the cage (tile centre line), on brackets to the girts
    for x in (0.07, 0.93):
        k.add(box(0.04, 0.07, 8.36), "st_steel", T((x, 0.0, 0.1 + 4.18)))
        for z in (1.4, 4.0, 6.6):
            k.add(box(0.07, 0.05, 0.06), "st_steel_dk", T((x + (-0.035 if x < 0.5 else 0.035), 0.0, z)))
    # anchored to the rock: beams from the back posts into the face, an iron plate and bolts where they enter
    for z in (1.4, 3.3, 5.3):
        for x in XS:
            k.add(box(0.1, 0.62, 0.1), "plank_grey_dk", T((x, 0.7, z - 0.13)))
            k.add(box(0.26, 0.03, 0.26), "st_steel_dk", T((x, 0.48, z - 0.13)))
            for dx in (-0.08, 0.08):
                k.add(cyl(0.022, 0.022, 0.03, 6), "brass", T((x + dx, 0.46, z - 0.13 + dx), (90, 0, 0)))
    # winch house on the bay at the summit's height: a plank floor, board walls, a copper roof
    k.add(box(1.04, 0.98, 0.07), "plank_grey", T((Wb - 0.02, 0.0, LIFT_TOP + 0.035)))
    k.add(box(0.8, 0.74, 1.12), "plank_grey_lt", T((Wb, 0.0, LIFT_TOP + 0.07 + 0.56)))
    for x in (Wb - 0.4, Wb + 0.4):
        k.add(box(0.06, 0.78, 1.14), "plank_grey_dk", T((x, 0.0, LIFT_TOP + 0.07 + 0.57)))
    k.add(box(0.3, 0.02, 0.24), "lamp_glow", T((Wb, -0.375, LIFT_TOP + 0.85)))       # a lit window on the sea side
    k.add(box(0.36, 0.03, 0.04), "plank_grey_dk", T((Wb, -0.385, LIFT_TOP + 0.99)))
    roof = prism([(-0.5, 0.0), (0.5, 0.0), (0.0, 0.38)], 0.92)
    k.add(roof, "copper", T((Wb, 0.0, LIFT_TOP + 1.17)))
    k.add(box(0.06, 0.96, 0.05), "st_verdigris_dk", T((Wb, 0.0, LIFT_TOP + 1.56)))   # ridge
    k.add(cyl(0.05, 0.05, 0.12, 8), "st_steel_dk", T((Wb, 0.0, LIFT_TOP + 1.6)))     # cable gland on the ridge
    k.add(box(0.03, 0.03, LIFT_HEAD - LIFT_TOP - 1.6), "st_steel_dk", T((Wb, 0.0, (LIFT_TOP + 1.6 + LIFT_HEAD) / 2)))
    # head frame: cap beams, two bearing pedestals, the axle along Y, the sheave in the XZ plane (faces the camera)
    for y in (YF, YB):
        k.add(box(1.14, 0.12, 0.12), "plank_grey_dk", T((S, y, 8.46)))
    for x in (0.0, 1.0):
        k.add(box(0.12, YB - YF + 0.12, 0.12), "plank_grey_dk", T((x, (YF + YB) / 2, 8.46)))
    for y in (YF, YB):
        beam(k, (-0.2, y, 8.52), (0.0, y, LIFT_HEAD), 0.07, 0.06, "st_steel")
        beam(k, (0.2, y, 8.52), (0.0, y, LIFT_HEAD), 0.07, 0.06, "st_steel")
        k.add(box(0.12, 0.08, 0.1), "st_steel_dk", T((0.0, y, LIFT_HEAD)))
    k.add(cyl(0.035, 0.035, YB - YF + 0.12, 8), "brass", T((0.0, (YF + YB) / 2, LIFT_HEAD), (90, 0, 0)))
    R = 0.5
    M = T((0.0, 0.0, LIFT_HEAD), (90, 0, 0))
    k.add(torus(R, 0.045, 16, 4), "copper", M)
    k.add(torus(R - 0.06, 0.02, 16, 4), "copper_dk", M @ T((0, 0, 0.03)))
    k.add(cyl(0.09, 0.09, 0.14, 10), "st_steel_dk", M)
    for i in range(6):
        a = i / 6 * math.tau + 0.2
        beam(k, (0.08 * math.cos(a), 0.0, LIFT_HEAD + 0.08 * math.sin(a)), ((R - 0.03) * math.cos(a), 0.0, LIFT_HEAD + (R - 0.03) * math.sin(a)),
             0.035, 0.03, "iron")
    k.add(tube([((R + 0.03) * math.cos(math.radians(a)), 0.0, LIFT_HEAD + (R + 0.03) * math.sin(math.radians(a)))
                for a in range(180, -1, -20)], 0.016, 4, cap=False), "st_steel_dk")  # the cable over the top
    # summit landing behind the shaft: planks over the lip, rails each side, a lamp on the east post
    for i in range(5):
        k.add(box(0.19, 1.06, 0.05), rng.choice(["plank_grey", "plank_grey_lt", "plank_grey_dk"]),
              T((0.1 + 0.2 * i, 0.98, LIFT_TOP - 0.01)))
    for x in (0.06, 0.94):  # short guard rails at the lip (the summit stays open east and west)
        for y in (0.56, 0.94):
            k.add(box(0.08, 0.08, 1.08), "plank_grey_dk", T((x, y, LIFT_TOP + 0.5)))
        for z in (0.55, 1.0):
            k.add(box(0.06, 0.46, 0.06), "plank_grey", T((x, 0.75, LIFT_TOP + z)))

    def lamp(x, y, z, arm=0.0):  # a little lantern on a post top, or hung from an arm off a post's face
        if arm:
            k.add(box(0.04, abs(arm), 0.04), "st_steel_dk", T((x, y + arm / 2, z + 0.22)))
            y += arm
        k.add(box(0.13, 0.13, 0.03), "st_steel_dk", T((x, y, z + 0.015)))
        k.add(box(0.1, 0.1, 0.16), "lamp_glow", T((x, y, z + 0.1)))
        k.add(box(0.13, 0.13, 0.03), "st_steel_dk", T((x, y, z + 0.195)))
        k.add(cyl(0.09, 0.02, 0.07, 4), "copper", T((x, y, z + 0.245), (0, 0, 45)))
        return (x, y, z + 0.1)
    top_lamp = lamp(0.94, 0.56, LIFT_TOP + 1.04)
    low_lamp = lamp(1.0, YF - 0.055, 2.0, -0.18)
    contact(k, 1.05)
    for bm, *_ in k.parts[-2:]:
        bmesh.ops.scale(bm, vec=(1.15, 0.6, 1), verts=bm.verts)
        bmesh.ops.translate(bm, vec=(0.03, 0, 0), verts=bm.verts)
    return chain(empty("light_origin", low_lamp), empty("light_origin", top_lamp))


def build_lift_cage(k):
    """The moving cage (lift.js carries it): 0.8 m square, floor top 0.1 m, copper frame, iron bar sides, open
    above two waist-high gates (four materials, so few draws). Nodes: gate_front (-Y) and gate_back (+Y), hinged on their +X edge, swing out;
    cable_cage (1 m tall from its origin, stretched up to the sheave), a coil_glow lamp under the roof."""
    H, h = 2.25, 0.375
    k.add(box(0.8, 0.8, 0.06), "iron", T((0, 0, 0.03)))
    for i in range(4):
        k.add(box(0.19, 0.76, 0.04), "plank_grey", T((-0.3 + 0.2 * i, 0, 0.08)))
    for sx in (-1, 1):
        for sy in (-1, 1):
            k.add(box(0.05, 0.05, H - 0.06), "copper", T((sx * h, sy * h, 0.06 + (H - 0.06) / 2)))
    k.add(box(0.82, 0.82, 0.05), "copper", T((0, 0, H + 0.025)))
    k.add(cyl(0.56, 0.12, 0.18, 4), "iron", T((0, 0, H + 0.14), (0, 0, 45)))
    k.add(box(0.05, 0.05, 0.12), "iron", T((0, 0, H + 0.27)))
    for sx in (-1, 1):  # bar sides (east, west), rails, guide shoes on the rails
        for i in range(4):
            k.add(box(0.02, 0.02, H - 0.12), "iron", T((sx * h, -0.225 + 0.15 * i, 0.06 + (H - 0.12) / 2)))
        for z in (0.12, 1.05, 2.0):
            k.add(box(0.03, 0.72, 0.04), "copper", T((sx * h, 0, z)))
        for z in (0.3, 2.05):
            k.add(box(0.04, 0.1, 0.12), "iron", T((sx * 0.405, 0, z)))
    for sy in (-1, 1):  # a lintel over each doorway
        k.add(box(0.72, 0.03, 0.05), "copper", T((0, sy * h, 2.0)))
    k.add(sphere(0.06, 8, 6), "coil_glow", T((0, -0.3, H - 0.08)), smooth=True)
    k.add(box(0.1, 0.06, 0.04), "iron", T((0, -0.33, H - 0.01)))

    def gate(gk):  # local: hinge at the origin, the leaf toward -X, 1 m tall
        for z in (0.04, 0.98):
            gk.add(box(0.7, 0.03, 0.04), "copper", T((-0.36, 0, z)))
        gk.add(box(0.03, 0.03, 0.98), "copper", T((-0.02, 0, 0.5)))
        gk.add(box(0.03, 0.03, 0.98), "copper", T((-0.7, 0, 0.5)))
        for i in range(5):
            gk.add(box(0.016, 0.016, 0.92), "iron", T((-0.13 - 0.11 * i, 0, 0.5)))
        gk.add(box(0.05, 0.05, 0.1), "copper", T((-0.69, -0.03, 0.62)))

    def cable(ck):
        ck.add(box(0.025, 0.025, 1.0), "iron", T((0, 0, 0.5)))

    def part(build, loc, name):
        def after(ob, coll):
            pk = B.Kit(name)
            build(pk)
            o = pk.build(coll)
            o.parent = ob
            o.location = loc
            return [o]
        return after
    return chain(part(gate, (0.35, -0.4, 0.1), "gate_front"), part(gate, (0.35, 0.4, 0.1), "gate_back"),
                 part(cable, (0.0, 0.0, H + 0.33), "cable_cage"))


def build_rodspire(k):
    """3 x 3, ~12 m: the Rodspire. A stepped granite plinth carrying a stout four-legged copper lattice tower that
    tapers in an Eiffel curve (X-braced bays, chunky verdigris collars at every level), a heavy copper rod up its
    core wound with two big coil drums (glowing bands), then a mast with two halo rings and a crown of fins round a
    large cyan orb (coil_glow) where the `light_origin` sits."""
    k.add(cyl(1.44, 1.48, 0.3, 8, 0.03), "st_granite_dk", T((0, 0, 0.13), (0, 0, 22.5)))
    k.add(cyl(1.2, 1.26, 0.36, 8, 0.03), "st_granite", T((0, 0, 0.44), (0, 0, 22.5)))
    k.add(cyl(0.55, 0.6, 0.3, 8, 0.02), "st_granite_lt", T((0, 0, 0.75), (0, 0, 22.5)))
    Z0, ZT = 0.62, 9.0
    R = lambda z: 0.36 + 0.68 * ((ZT - z) / (ZT - Z0)) ** 1.35  # noqa: E731  (leg distance from the axis)
    lv = [Z0, 2.3, 3.9, 5.4, 6.8, 8.0, ZT]
    corner = lambda z, i: (R(z) * math.cos(math.radians(45 + 90 * i)), R(z) * math.sin(math.radians(45 + 90 * i)), z)  # noqa: E731
    for i in range(4):  # legs, thick at the foot; granite foot blocks
        k.add(box(0.36, 0.36, 0.16, 0.02), "st_granite_lt", T((*corner(Z0, i)[:2], Z0 + 0.06), (0, 0, 45 + 90 * i)))
        for z0, z1 in zip(lv, lv[1:]):
            w = 0.075 + 0.07 * (ZT - z0) / (ZT - Z0)
            k.add(tube([corner(z0 + 0.02, i), corner(z1, i)], w, 4, cap=False), "copper")
    for z0, z1 in zip(lv[:-1], lv[1:-1]):  # X bracing in every bay but the top one
        for i in range(4):
            a, b = corner(z0, i), corner(z1, (i + 1) % 4)
            c, d = corner(z0, (i + 1) % 4), corner(z1, i)
            for p, q, f in ((a, b, 1.0), (c, d, 0.97)):   # the second diagonal a touch inside: no shared plane
                k.add(tube([(p[0] * f, p[1] * f, p[2]), (q[0] * f, q[1] * f, q[2])], 0.045, 4, cap=False), "copper_dk")
    for n, z in enumerate(lv[1:]):  # collars: a square frame ring with a verdigris band, one per level
        r = R(z) + 0.2
        k.add(cyl(r, r, 0.2, 4), "st_verdigris_dk" if n % 2 else "st_verdigris", T((0, 0, z), (0, 0, 45)))
        k.add(cyl(r * 0.8, r * 0.8, 0.27, 4), "copper_dk", T((0, 0, z), (0, 0, 45)))
    # the core rod and two coil drums with glowing bands
    core = lambda z: 0.3 - 0.14 * (z - 0.9) / (ZT - 0.9)  # noqa: E731
    k.add(cyl(core(0.9), core(ZT), ZT - 0.86, 8), "copper", T((0, 0, (0.84 + ZT - 0.02) / 2), (0, 0, 22.5)), smooth=True)
    for zc, h, rr in ((3.1, 1.2, 0.5), (6.1, 0.9, 0.38)):
        k.add(cyl(rr, rr, h, 10), "copper_dk", T((0, 0, zc)), smooth=True)
        for j in range(3):
            k.add(torus(rr + 0.03, 0.06, 8, 4), "copper", T((0, 0, zc - h / 2 + 0.12 + j * (h - 0.24) / 2)))
        for j in range(2):
            k.add(cyl(rr + 0.012, rr + 0.012, 0.15, 8), "coil_glow", T((0, 0, zc - h / 2 + 0.12 + (j + 0.5) * (h - 0.24) / 2)))
    # the mast above the tower, halo rings on spokes, crown of fins round the orb
    TOP = 11.2
    k.add(cyl(0.2, 0.11, TOP - ZT, 8), "copper_dk", T((0, 0, (ZT + TOP) / 2), (0, 0, 22.5)), smooth=True)
    for z, rr in ((9.55, 0.95), (10.45, 0.62)):
        k.add(torus(rr, 0.08, 14, 4), "copper", T((0, 0, z)))
        k.add(cyl(0.26, 0.26, 0.18, 8), "st_verdigris", T((0, 0, z), (0, 0, 22.5)))
        for i in range(4):
            a = math.radians(90 * i)
            k.add(tube([(0.2 * math.cos(a), 0.2 * math.sin(a), z), (rr * math.cos(a), rr * math.sin(a), z)], 0.045, 4), "st_verdigris_dk")
    k.add(cyl(0.34, 0.24, 0.16, 8), "st_verdigris", T((0, 0, TOP), (0, 0, 22.5)))
    for i in range(4):
        k.add(prism([(0.16, 0.0), (0.52, 0.1), (0.58, 0.5), (0.36, 1.0), (0.2, 0.62)], 0.07), "copper",
              T((0, 0, TOP), (0, 0, 45 + 90 * i)))
    k.add(sphere(0.4, 12, 8), "coil_glow", T((0, 0, TOP + 0.5)), smooth=True)
    k.add(cyl(0.06, 0.0, 0.9, 6), "copper", T((0, 0, TOP + 1.3)))
    contact(k, 1.5)
    return empty("light_origin", (0, 0, TOP + 0.5))


def build_stairs_heath(k):
    """1 x 1 granite steps, same contract as stairs: up from the front (-Y) toward the back, tread tops
    0.125 / 0.375 / 0.625 / 0.875, floor 0.5; rough granite cheeks with a lip of turf. Stormwatch's bluff stairs."""
    rng = random.Random(481)
    for i in range(4):
        y0 = -0.5 + 0.25 * i
        topz = 0.125 + 0.25 * i
        k.add(box(0.72 - 0.006 * i, 0.5 - y0, topz + 0.02, 0.014), "st_granite_dk", T((0, (y0 + 0.5) / 2, (topz - 0.02) / 2)))
        k.add(stone(rng, 0.74, 0.25, 0.04, 0.004), ("st_granite_lt", "st_granite")[i % 2], T((0, y0 + 0.115, topz - 0.015)))
    for sx in (-1, 1):
        pts = [(-0.5, -0.02), (0.5, -0.02), (0.5, 1.0), (0.22, 0.9), (-0.25, 0.32), (-0.5, 0.14)]
        k.add(jitter(prism(pts, 0.13, 0.015), rng, 0.01), "st_granite", T((sx * 0.44, 0, 0), (0, 0, 90)))
        k.add(jitter(prism([(0.18, 0.88), (0.5, 0.98), (0.5, 1.03), (0.16, 0.93)], 0.15, 0.01), rng, 0.008), "st_turf", T((sx * 0.44, 0, 0), (0, 0, 90)))
    contact(k, 0.5)


def build_telescope(k):
    """2 x 2 (interior) the observatory's great telescope: a brass tube on a tilted equatorial mount, a cast-iron
    pier on a round granite plinth, a finder scope, a little eyepiece chair. Front +Z (glTF), aimed up and back."""
    k.add(cyl(0.7, 0.75, 0.12, 14, 0.02), "st_granite", T((0, 0.1, 0.05)))
    k.add(cyl(0.16, 0.22, 0.9, 10), "st_steel_dk", T((0, 0.1, 0.55)), smooth=True)              # pier
    k.add(box(0.36, 0.36, 0.12, 0.02), "st_steel", T((0, 0.1, 1.05)))
    M = T((0, 0.1, 1.15), (-38, 0, 0))                                                          # polar axis tilt
    k.add(cyl(0.07, 0.07, 0.36, 8), "st_steel", M @ T((0, 0, 0.12)))
    k.add(box(0.22, 0.14, 0.14, 0.01), "brass", M @ T((0, 0, 0.32)))
    tube_m = M @ T((0, 0, 0.42), (70, 0, 0))                                                    # the tube, pointing up and back
    k.add(cyl(0.13, 0.15, 1.7, 14), "brass", tube_m @ T((0, 0, 0.3)), smooth=True)
    k.add(torus(0.155, 0.02, 14, 4), "copper_dk", tube_m @ T((0, 0, 1.15)))
    k.add(torus(0.15, 0.02, 14, 4), "copper_dk", tube_m @ T((0, 0, -0.35)))
    k.add(cyl(0.16, 0.16, 0.04, 14), "st_steel_dk", tube_m @ T((0, 0, 1.16)))
    k.add(cyl(0.035, 0.035, 0.42, 8), "brass", tube_m @ T((0.16, 0, 0.75)))                     # finder
    k.add(cyl(0.03, 0.04, 0.1, 8), "st_steel_dk", tube_m @ T((0, 0.14, -0.4), (90, 0, 0)))       # eyepiece
    k.add(cyl(0.18, 0.18, 0.05, 10), "wood", T((0.45, -0.55, 0.45)))                            # stool
    for a in (0, 120, 240):
        ca, sa = math.cos(math.radians(a)), math.sin(math.radians(a))
        beam(k, (0.45 + 0.13 * ca, -0.55 + 0.13 * sa, 0.0), (0.45 + 0.08 * ca, -0.55 + 0.08 * sa, 0.44), 0.03, 0.03, "wood")
    shadow(k, 0.8)


def build_bottled_spark(k):
    """Curio (the popup): a stoppered glass bottle with a crackle of blue lightning caught inside, a copper wire
    twisted round its neck. Centred on the origin."""
    rng = random.Random(482)
    k.add(lathe([(0.0, -0.11), (0.07, -0.105), (0.08, -0.06), (0.075, 0.03), (0.035, 0.07), (0.028, 0.1), (0.032, 0.11)], 12),
          "st_bottle", T((0, 0, 0)), smooth=True)
    k.add(cyl(0.024, 0.02, 0.04, 8), "wood", T((0, 0, 0.125)))
    k.add(torus(0.03, 0.006, 10, 4), "copper", T((0, 0, 0.085)))
    pts, z = [], -0.08
    for i in range(7):
        pts.append((rng.uniform(-0.04, 0.04), rng.uniform(-0.04, 0.04), z))
        z += 0.022
    k.add(tube(pts, 0.008, 4), "coil_glow")
    k.add(sphere(0.022, 8, 6), "coil_glow", T((0, 0, -0.02)), smooth=True)


PROPS = {
    "stairs_heath": build_stairs_heath, "telescope": build_telescope, "bottled_spark": build_bottled_spark,
    "cliff_heath": build_cliff_heath, "cliff_heath_b": lambda k: build_cliff_heath(k, 402, tufted=True),
    "wall_dry": lambda k: build_wall_dry(k, "WE", 471), "wall_dry_corner": lambda k: build_wall_dry(k, "WS", 472),
    "wall_dry_end": lambda k: build_wall_dry(k, "W", 473), "wall_dry_t": lambda k: build_wall_dry(k, "WES", 474),
    "heather": build_heather, "chime_post": build_chime_post, "pylon": build_pylon,
    "observatory": build_observatory, "lift_tower": build_lift_tower, "lift_cage": build_lift_cage,
    "rodspire": build_rodspire,
}

_STACK = [("cliff_basalt", x, z, 0, h) for x in (15, 16, 17) for z in (1, 2, 3) for h in range(3)]
VIGNETTE = dict(
    ground=("heath", "dirt", "#7d8a5a"),
    cliff=("cliff_heath", "cliff_heath_b"),
    fence="wall_dry",
    ring=["heather", "rock", "bush_dark"],
    water="#4f9fb4",
    tiles=[
        "333333........~~~~~~",
        "333333........~~~~~~",
        "22222.........~~~~~~",
        "222...........=~~~~~",
        "11............~~~~~~",
        "..............~~~~~~",
        "ffffff..pppppp~~~~~~",
        ".....f..p.....~~~~~~",
        ".....f..p......~~~~~",
        "........p.....~~~~~~",
        "..f.....p......~~~~~",
        "..f.....p.....~~~~~~",
        "..fff...p.....~~~~~~",
    ],
    props=[("observatory", 7, 0), ("lift_tower", 11, 2), ("lift_cage", 12, 2), ("rodspire", 15, 1, 0, 3)] + _STACK + [
        ("chime_post", 1, 8), ("chime_post", 3, 8), ("chime_post", 0, 10), ("chime_post", 4, 10), ("chime_post", 1, 11),
        ("pylon", 10, 7), ("pylon", 12, 10, 90),
        ("heather", 6, 9), ("heather", 7, 11), ("heather", 11, 5), ("heather", 13, 12), ("heather", 6, 4), ("heather", 3, 5),
        ("heather", 0, 0, 0, 3), ("heather", 4, 1, 0, 3), ("rock", 9, 12), ("rock", 5, 3),
        ("waymark", 9, 8), ("hero", 8, 9),
    ],
    aim=(9.5, -7.2, 0), dist=16, lens=28,
)
FOOT_OTHER = {}
