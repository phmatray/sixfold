"""Storm: The Galewold (Lightning). High heath on sea cliffs, dry-stone walls, chime posts thrumming in the wind,
copper and verdigris, weather always moving.

Vocabulary: grey granite with a turf lip (cliffs, walls), copper gone green (the observatory dome, the Rodspire,
pylon coils), tall thin masts with stays and cables, purple heather, pennants that all point downwind (+X).
Glow parts: coil_glow (pylon coil, lift lamp, Rodspire orb: electric cyan), lamp_glow (observatory slit, windows).
"""
import math
import random

from v13_kit import (B, T, beam, box, contact, cyl, empty, ico, jitter, prism, ribbon, shadow, sphere,  # noqa: F401
                     torus, tube, curve, half_sphere, arch_pts, arc, stratum, FRONT, clamp01, chain, open_part)

PALETTE = {
    "st_granite": "#9c9c96", "st_granite_dk": "#6f716e", "st_granite_lt": "#b8b6ad", "st_granite_warm": "#a99f90",
    "st_lichen": "#b3ad86", "st_turf": "#76804e", "st_turf_dk": "#5f6a40",
    "st_heath_leaf": "#55603c", "st_heath_leaf_lt": "#6c7748", "st_heather": "#a65ca0", "st_heather_lt": "#cf8fc6",
    "st_heather_dk": "#7c4380",
    "st_verdigris": "#5fae96", "st_verdigris_lt": "#7cc4ab", "st_verdigris_dk": "#3f8572",
    "st_whitewash": "#ece5d6", "st_whitewash_dk": "#d3cab8", "st_door": "#2f4a6b",
    "st_porcelain": "#f2efe6", "st_pennant": "#d9573f", "st_pennant_lt": "#f2c14a",
    "st_steel": "#59626c", "st_steel_dk": "#3f464f",
    "coil_glow": "#9feaff",
    "copper": "#c27a43", "copper_dk": "#9a5a2e",   # same values as v13_ember (shared keys)
}

WIND = {
    "chime_post": lambda x, y, z, r: clamp01((z - 0.9) / 1.4) * 0.55,
    "heather": lambda x, y, z, r: clamp01(z / 0.5) * 0.8,
}

FOOT = {"observatory": (5, 5), "lift": (2, 2), "rodspire": (3, 3)}
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


def build_lift(k):
    """2 x 2 cliff lift: an iron cage (floor top 0.1 m, open at the front) on rails up a steel lattice mast at the
    back, a winding wheel and cables at the head, a counterweight, a lamp that says the line is live."""
    rng = random.Random(451)
    k.add(stone(rng, 1.9, 1.9, 0.08, 0.008), "st_granite", T((0, 0, -0.005)))
    # cage floor and frame (front edge at y = -0.85)
    k.add(box(1.6, 1.5, 0.1, 0.012), "st_steel_dk", T((0, -0.1, 0.025)))
    for i in range(6):
        k.add(box(1.52, 0.2, 0.025), "plank_grey" if i % 2 else "plank_grey_lt", T((0, -0.73 + 0.25 * i, 0.0875)))
    H = 2.1
    for sx in (-1, 1):
        for sy in (-1, 1):
            k.add(box(0.07, 0.07, H, 0.01), "lh_red", T((sx * 0.77, -0.1 + sy * 0.72, 0.1 + H / 2)))
    k.add(box(1.62, 1.52, 0.08, 0.012), "lh_red", T((0, -0.1, 0.1 + H)))
    k.add(cyl(0.95, 0.4, 0.3, 4), "st_steel_dk", T((0, -0.1, 0.1 + H + 0.19), (0, 0, 45)))
    for side in ("l", "r", "b"):  # bars on three sides
        for i in range(7):
            u = -0.65 + 0.217 * i
            x, y = {"l": (-0.77, -0.1 + u), "r": (0.77, -0.1 + u), "b": (u, 0.62)}[side]
            k.add(box(0.022, 0.022, H - 0.1), "iron", T((x, y, 0.15 + (H - 0.1) / 2)))
        for z in (1.0, 1.55):
            if side == "b":
                k.add(box(1.5, 0.03, 0.04), "lh_red", T((0, 0.62, z)))
            else:
                k.add(box(0.03, 1.4, 0.04), "lh_red", T(((-0.77 if side == "l" else 0.77), -0.1, z)))
    # folded-open gate on the front left
    for i in range(4):
        k.add(box(0.022, 0.022, 1.6), "iron", T((-0.74 + 0.07 * i, -0.86 - 0.03 * i, 0.95)))
    k.add(box(0.26, 0.12, 0.04), "lh_red", T((-0.64, -0.9, 1.75), (0, 0, -25)))
    # mast at the back edge, rails, head wheel, cables
    MH = 4.6
    for sx in (-1, 1):
        k.add(box(0.1, 0.1, MH + 0.02, 0.012), "st_steel", T((sx * 0.45, 0.88, MH / 2 - 0.01)))
    for i in range(7):
        z0 = 0.3 + i * 0.6
        o = 0.012 * (i % 2)
        beam(k, (-0.45, 0.825 + o, z0), (0.45, 0.825 + o, z0 + 0.6), 0.03, 0.025, "st_steel_dk")
        beam(k, (0.45, 0.94 + o, z0), (-0.45, 0.94 + o, z0 + 0.6), 0.03, 0.025, "st_steel_dk")
    k.add(box(1.1, 0.4, 0.1, 0.012), "st_steel", T((0, 0.82, MH)))
    k.add(cyl(0.38, 0.38, 0.07, 14), "lh_red", T((0, 0.72, MH + 0.38), (0, 90, 0)))
    k.add(torus(0.38, 0.035, 14, 4), "st_steel_dk", T((0, 0.72, MH + 0.38), (0, 90, 0)))
    for i in range(6):
        a = i / 6 * math.tau
        x = 0.005 * (i - 2.5)
        beam(k, (x, 0.72, MH + 0.38), (x, 0.72 + 0.34 * math.cos(a), MH + 0.38 + 0.34 * math.sin(a)), 0.03, 0.03, "st_steel_dk")
    k.add(cyl(0.06, 0.06, 0.2, 8), "brass", T((0, 0.72, MH + 0.38), (0, 90, 0)))
    k.add(tube([(0, 0.35, MH + 0.38), (0, -0.05, 2.5)], 0.014, 4), "st_steel_dk")
    k.add(tube([(0, 1.08, MH + 0.38), (0, 1.12, 1.6)], 0.014, 4), "st_steel_dk")
    k.add(box(0.32, 0.18, 0.5, 0.015), "st_granite_dk", T((0, 1.1, 1.35)))   # counterweight
    # live lamp on the mast
    k.add(box(0.12, 0.08, 0.12), "st_steel_dk", T((0.52, 0.8, 2.6)))
    k.add(sphere(0.07, 8, 6), "coil_glow", T((0.52, 0.74, 2.6)), smooth=True)
    shadow(k, 1.0, 1.0, 1.0)


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


PROPS = {
    "cliff_heath": build_cliff_heath, "cliff_heath_b": lambda k: build_cliff_heath(k, 402, tufted=True),
    "wall_dry": lambda k: build_wall_dry(k, "WE", 471), "wall_dry_corner": lambda k: build_wall_dry(k, "WS", 472),
    "wall_dry_end": lambda k: build_wall_dry(k, "W", 473), "wall_dry_t": lambda k: build_wall_dry(k, "WES", 474),
    "heather": build_heather, "chime_post": build_chime_post, "pylon": build_pylon,
    "observatory": build_observatory, "lift": build_lift, "rodspire": build_rodspire,
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
    props=[("observatory", 7, 0), ("lift", 12, 2), ("rodspire", 15, 1, 0, 3)] + _STACK + [
        ("chime_post", 1, 8), ("chime_post", 3, 8), ("chime_post", 0, 10), ("chime_post", 4, 10), ("chime_post", 1, 11),
        ("pylon", 10, 7), ("pylon", 12, 10, 90),
        ("heather", 6, 9), ("heather", 7, 11), ("heather", 11, 5), ("heather", 13, 12), ("heather", 6, 4), ("heather", 3, 5),
        ("heather", 0, 0, 0, 3), ("heather", 4, 1, 0, 3), ("rock", 9, 12), ("rock", 5, 3),
        ("waymark", 9, 8), ("hero", 8, 9),
    ],
    aim=(9.5, -7.2, 0), dist=16, lens=28,
)
FOOT_OTHER = {}
