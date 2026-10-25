"""Ember: The Emberlands (Fire). Black columnar basalt and ash, orange-lit kilns, glass glinting everywhere.

Vocabulary: hexagonal basalt columns (cliffs, stairs, rocks, the climb wall), brick bottle shapes (the Great
Kiln, the glass furnace), coloured glass (racks, cullet, the lava-glass pool), scorched pines, steam.
Glow parts: kiln_glow (kiln and furnace mouths), ember_glow (vents, cracks, the pool).
"""
import math
import random

from v13_kit import (A, B, T, beam, box, contact, cyl, empty, facet_block, ico, jitter, lathe, prism, ribbon,  # noqa: F401
                     rock, shadow, sphere, torus, tube, arch_pts, FRONT, clamp01, chain)

PALETTE = {
    "basalt": "#45434d", "basalt_dk": "#36343d", "basalt_lt": "#57545f", "basalt_top": "#7c7876", "basalt_joint": "#5b5756",
    "ash_dust": "#b3ada6", "char": "#33292a", "char_lt": "#4a3c3a", "ember_glow": "#ff7a2e",
    "ash_needle": "#7b8068", "ash_needle_dk": "#616653", "ash_needle_lt": "#979980",
    "kiln_brick": "#a8573d", "kiln_brick_dk": "#8a4430", "kiln_brick_lt": "#bf6a48", "soot": "#2a211c",
    "kiln_glow": "#ffa046", "fireclay": "#d9b48a", "copper": "#c27a43", "copper_dk": "#9a5a2e",
    "glass_teal": "#5fd0c4", "glass_amber": "#f0a73a", "glass_green": "#8fd06a", "glass_rose": "#e98a9a",
    "glass_blue": "#6fa8e8", "sulphur": "#e2c84a", "steam": "#f2eee8", "steam_dk": "#d9d4cc",
    "obsidian": "#2a2433", "obsidian_lt": "#4a3f5a", "glass_crust": "#5a3a2e", "kiln_stone": "#b9a58c",
    "kiln_stone_dk": "#9a8670", "timber_ember": "#6e4b33", "slag": "#3a3036",
}

WIND = {
    "ash_pine": lambda x, y, z, r: clamp01((z - 1.2) / 2.4) * 0.45,   # stiff: sways little
}

FOOT = {"kiln": (4, 4), "glass_furnace": (2, 2), "glass_rack": (2, 1), "lava_glass": (2, 2)}
FOOT_OTHER = {"cave_mouth": (2, 1), "seal_stone": (2, 2), "house": (7, 5), "cottage": (5, 4), "stall": (2, 1), "stall_b": (2, 1)}
ANIM = set()

LIGHT = dict(sun=("#ffd9a8", 2.8), from_hero=(8, 18, 6), hemi=("#ffe2c4", "#5a2f1f", 1.5), bg="#e3b98a", fog=(16, 34))


# ---------------------------------------------------------------- terrain: columnar basalt


def drop_bottom(bm):
    """Remove a prism's bottom cap (hidden inside the block; keeps bottoms from stacking up coplanar)."""
    import bmesh
    zmin = min(v.co.z for v in bm.verts)
    bmesh.ops.delete(bm, geom=[f for f in bm.faces if f.normal.z < -0.9 and all(abs(v.co.z - zmin) < 1e-5 for v in f.verts)], context="FACES")


def basalt_block(k, rng, top_cracks=False):
    """1 x 1 x 1 columnar basalt: a core to the tile edges, half columns standing proud along the sides,
    a honeycomb of column caps on top (lighter, dusted with ash) with dark joints; flat top at 1.0."""
    core = facet_block(rng, 1.0, 1.0, -0.02, 0.965, bulge=0.01)
    k.add(core, "basalt_joint")
    # half columns along the four sides, each its own height so neighbours never share a face
    for side in range(4):
        a = math.radians(90 * side)
        nx, ny = round(math.cos(a)), round(math.sin(a))
        for i in range(4):
            u = -0.375 + 0.25 * i + rng.uniform(-0.03, 0.03)
            r = rng.uniform(0.1, 0.13)
            top = rng.uniform(0.86, 0.95)
            out = 0.5 - r * 0.55 + rng.uniform(0.0, 0.035)
            x, y = nx * out + (-ny) * u, ny * out + nx * u
            bm = cyl(r, r * rng.uniform(0.96, 1.02), top + 0.02, 6)
            drop_bottom(bm)
            k.add(bm, rng.choice(["basalt_dk", "basalt", "basalt", "basalt_lt"]), T((x, y, (top - 0.02) / 2), (0, 0, rng.uniform(0, 60))))
    # a dense, slightly jumbled honeycomb of column caps inside the tile (tops 0.985-1.0, never shared)
    R = 0.105
    ox, oy = rng.uniform(-R, R), rng.uniform(-R, R)
    for j in range(-4, 5):
        for i in range(-4, 5):
            cx = ox + i * R * 1.76 + (R * 0.88 if j % 2 else 0)
            cy = oy + j * R * 1.52
            r = R * rng.uniform(0.86, 0.95)
            if abs(cx) > 0.5 - r or abs(cy) > 0.5 - r * 0.9:
                continue
            if top_cracks and math.hypot(cx - 0.06, cy - 0.05) < 0.17:
                continue
            q = i - (j - (j & 1)) // 2      # axial hex coords: (q - j) mod 3 never repeats between neighbours
            h = (1.0, 0.993, 0.986)[(q - j) % 3]
            bm = cyl(r, r, h - 0.93, 6)
            drop_bottom(bm)
            k.add(bm, "ash_dust" if rng.random() < 0.12 else "basalt_top",
                  T((cx, cy, (h + 0.93) / 2), (0, 0, 30)))
    if top_cracks:  # a cooled vent among the columns: a glowing crack and a drift of ash
        k.add(prism([(-0.15, 0), (-0.04, 0.025), (0.06, -0.02), (0.16, 0.01), (0.05, 0.045), (-0.05, 0.05)], 0.06),
              "ember_glow", T((0.06, 0.05, 0.955), (90, 0, 35)))
        k.add(jitter(ico(0.07, 1), rng, 0.01), "ash_dust", T((-0.04, 0.14, 0.97), scale=(1.3, 1.0, 0.35)), smooth=True)


def build_cliff_basalt(k):
    basalt_block(k, random.Random(301))


def build_cliff_basalt_b(k):
    basalt_block(k, random.Random(302), top_cracks=True)


# ---------------------------------------------------------------- nature


def build_ash_pine(k):
    """1 x 1 scorched pine (~3.3 m): a charred, leaning trunk snapped off in a jagged point, bare stubs all the way
    up, and a few lopsided clumps of ash-green needles that survived on the lee side. The Emberlands' ring tree."""
    rng = random.Random(311)
    lean = T((0, 0, 0), (3, -2, 0))
    k.add(cyl(0.16, 0.075, 2.75, 7, 0.02), "char", lean @ T((0, 0, 1.35)), smooth=True)
    k.add(prism([(-0.075, 0), (0.075, 0), (0.06, 0.18), (0.01, 0.1), (-0.02, 0.42), (-0.05, 0.14)], 0.12), "char_lt", lean @ T((0.0, 0.0, 2.7)))
    for a in range(0, 360, 72):  # root flares
        k.add(cyl(0.1, 0.02, 0.42, 5), "char", T((0.15 * math.cos(math.radians(a)), 0.15 * math.sin(math.radians(a)), 0.08), (0, 62, a)))
    for i, z in enumerate((0.75, 1.05, 1.35, 1.6, 1.85, 2.1, 2.35, 2.55)):  # bare stubs, angled up, some snapped short
        a = i * 137 + rng.uniform(-20, 20)
        L = (0.62 - z * 0.14) * rng.choice((1.0, 1.0, 0.55))
        ca, sa = math.cos(math.radians(a)), math.sin(math.radians(a))
        beam(k, (0.04 * ca, 0.04 * sa, z), (L * ca, L * sa, z + L * 0.5), 0.05, 0.05, "char")
    # the survivors: a few clumps, all on the lee (+X) side, so the silhouette is lopsided and the trunk shows
    for zb, r, h, m, a in ((1.0, 0.5, 0.5, "ash_needle_dk", 20), (1.25, 0.42, 0.45, "ash_needle", -50), (1.7, 0.4, 0.42, "ash_needle", 40),
                           (2.05, 0.3, 0.38, "ash_needle_lt", -20), (2.4, 0.22, 0.32, "ash_needle_lt", 60)):
        bm = cyl(r, 0.0, h, 6)
        for v in bm.verts:
            if v.co.z < 0:
                v.co.x *= rng.uniform(0.8, 1.15)
                v.co.y *= rng.uniform(0.8, 1.15)
                v.co.z += rng.uniform(-0.05, 0.05)
        d = r * 0.75
        k.add(bm, m, lean @ T((d * math.cos(math.radians(a)), d * math.sin(math.radians(a)), zb + h / 2), (rng.uniform(-10, 10), rng.uniform(-10, 10), rng.uniform(0, 60))), smooth=True)
    contact(k, 0.42)


# ---------------------------------------------------------------- landmarks


def glow_arch(k, w, h, depth, M, frame="kiln_stone", glow="kiln_glow", fw=0.12):
    """A glowing arched opening: a stone arch frame proud of the face, the hot recess behind it."""
    k.add(ribbon(arch_pts(w + fw, h + fw / 2, 8), fw, depth * 0.6), frame, M @ T((0, -depth * 0.3, 0), FRONT))
    k.add(prism(arch_pts(w, h, 8), depth), glow, M @ T((0, 0, -0.03)))


def build_kiln(k):
    """4 x 4 the Great Kiln (~4.6 m): a brick bottle kiln on a stone plinth, iron hoops, a big glowing arched
    mouth at the front and four little firemouths round the foot, soot at the neck; a log stack, a blowpipe rack
    and a water butt in the corners. smoke_origin at the top. Kiln and firemouths glow at night."""
    rng = random.Random(321)
    k.add(cyl(1.92, 1.95, 0.32, 8, 0.03), "kiln_stone_dk", T((0, 0.05, 0.13), (0, 0, 22.5)))
    k.add(cyl(1.82, 1.82, 0.08, 8, 0.02), "kiln_stone", T((0, 0.05, 0.32), (0, 0, 22.5)))
    prof = [(1.58, 0.3), (1.68, 0.8), (1.66, 1.5), (1.48, 2.2), (1.15, 2.85), (0.8, 3.35)]
    k.add(lathe(prof, 16), "kiln_brick", T((0, 0.05, 0)), smooth=True)
    k.add(lathe([(0.8, 3.35), (0.62, 3.75), (0.56, 4.2), (0.58, 4.45)], 16), "soot", T((0, 0.05, 0)), smooth=True)
    k.add(torus(0.6, 0.075, 16, 4), "kiln_brick_dk", T((0, 0.05, 4.47)))                            # rolled lip
    k.add(lathe([(0.56, 4.3), (0.5, 4.32), (0.0, 4.33)], 16), "slag", T((0, 0.05, 0)))              # the dark throat, inside
    for z, m in ((0.62, "kiln_brick_lt"), (1.95, "kiln_brick_dk")):                                # brick courses
        r = 1.68 if z < 1 else 1.55
        k.add(lathe([(r + 0.012, z), (r + 0.012, z + 0.14)], 16), m, T((0, 0.05, 0)))
    for z, r in ((1.12, 1.69), (2.55, 1.33), (3.1, 0.98)):                                         # iron hoops
        k.add(torus(r + 0.03, 0.035, 14, 4), "iron", T((0, 0.05, z)))
    # the great mouth: arched, deep, glowing; an iron door hung open beside it
    glow_arch(k, 0.92, 1.12, 0.36, T((0, -1.57, 0.36)), fw=0.16)
    k.add(box(1.3, 0.3, 0.08, 0.02), "kiln_stone_dk", T((0, -1.72, 0.38)))                         # hearth sill
    k.add(prism(arch_pts(0.62, 0.84, 6), 0.04), "iron", T((0.78, -1.78, 0.38), (0, 0, -55)))
    k.add(box(0.03, 0.06, 0.06), "iron", T((0.5, -1.8, 0.9)))
    for a in (45, 135, 225, 315):                                                                  # little firemouths
        ca, sa = math.cos(math.radians(a)), math.sin(math.radians(a))
        M = T((1.6 * ca, 0.05 + 1.6 * sa, 0.36), (0, 0, a + 90))
        glow_arch(k, 0.34, 0.42, 0.3, M, fw=0.1)
    # corners: a log stack (front right), blowpipes in a rack (front left), a water butt (back left)
    for i in range(9):
        row, col = divmod(i, 3)
        r = rng.uniform(0.07, 0.085)
        k.add(cyl(r, r, 0.55, 7), "timber_ember", T((1.55 - 0.16 * col + 0.08 * row, -1.5, 0.095 + 0.15 * row), (0, 90, rng.uniform(0, 30)), (1, 1, 1)))
        k.add(cyl(r * 0.78, r * 0.78, 0.01, 7), "fireclay", T((1.55 - 0.16 * col + 0.08 * row + 0.276, -1.5, 0.095 + 0.15 * row), (0, 90, 0)))
    for x in (-1.75, -1.35):
        k.add(box(0.07, 0.07, 1.0, 0.01), "timber_ember", T((x, -1.55, 0.48)))
    k.add(box(0.5, 0.06, 0.06, 0.01), "timber_ember", T((-1.55, -1.55, 0.92)))
    for i in range(4):
        x = -1.68 + 0.1 * i
        k.add(cyl(0.015, 0.015, 1.5, 6), "iron", T((x, -1.48, 0.73), (8, 0, 0)))
        if i % 2 == 0:
            k.add(sphere(0.05, 8, 5), "glass_amber" if i else "glass_teal", T((x, -1.38, 1.47)), smooth=True)
    k.add(cyl(0.28, 0.25, 0.7, 10, 0.02), "timber_ember", T((-1.55, 1.55, 0.34)), smooth=True)
    for z in (0.15, 0.55):
        k.add(cyl(0.29, 0.29, 0.04, 10), "iron", T((-1.55, 1.55, z)))
    k.add(cyl(0.24, 0.24, 0.02, 10), "glass_blue", T((-1.55, 1.55, 0.688)))
    shadow(k, 2.05)
    # smoke from the neck; light from the great mouth (it lights the plaza at night)
    return chain(empty("smoke_origin", (0, 0.05, 4.62)), empty("light_origin", (0, -1.95, 0.95)))


def build_glass_furnace(k):
    """2 x 2 glassblower's furnace (~2.6 m): a squat fireclay beehive on a stone hearth with three round glory
    holes glowing (front, left, right), a copper hood and flue, a blowpipe resting on its yoke with a hot gather."""
    rng = random.Random(322)
    k.add(cyl(0.95, 0.98, 0.24, 8, 0.02), "kiln_stone_dk", T((0, 0.08, 0.1), (0, 0, 22.5)))
    k.add(lathe([(0.78, 0.2), (0.8, 0.55), (0.72, 0.95), (0.5, 1.3), (0.2, 1.48), (0.0, 1.5)], 14), "fireclay", T((0, 0.1, 0)), smooth=True)
    k.add(torus(0.81, 0.03, 16, 4), "iron", T((0, 0.1, 0.6)))
    for a in (270, 200, 340):                                                        # glory holes
        ca, sa = math.cos(math.radians(a)), math.sin(math.radians(a))
        c = (0.78 * ca, 0.1 + 0.78 * sa, 0.72)
        k.add(torus(0.17, 0.045, 12, 4), "iron", T(c, (90, 0, a + 90)))
        k.add(cyl(0.15, 0.15, 0.12, 12), "kiln_glow", T((0.74 * ca, 0.1 + 0.74 * sa, 0.72), (90, 0, a + 90)))
    k.add(lathe([(0.42, 1.38), (0.26, 1.7), (0.16, 1.82)], 10), "copper", T((0, 0.25, 0)), smooth=True)  # hood
    k.add(cyl(0.12, 0.12, 0.8, 10), "copper_dk", T((0, 0.25, 2.2)), smooth=True)
    k.add(cyl(0.2, 0.0, 0.14, 10), "copper", T((0, 0.25, 2.68)))
    for z in (1.95, 2.35):
        k.add(torus(0.125, 0.018, 10, 4), "iron", T((0, 0.25, z)))
    # yoke and blowpipe with a glowing gather by the front hole
    k.add(box(0.05, 0.05, 0.62, 0.01), "iron", T((0.55, -0.85, 0.29)))
    k.add(box(0.14, 0.05, 0.03), "iron", T((0.55, -0.85, 0.63)))
    k.add(cyl(0.016, 0.016, 1.2, 6), "iron", T((0.28, -0.72, 0.68), (0, 84, 62)))
    k.add(sphere(0.06, 8, 6), "kiln_glow", T((0.0, -0.2, 0.71)), smooth=True)
    for i in range(3):                                                                # cullet in a tub
        k.add(jitter(ico(0.05, 0), rng, 0.012), ("glass_teal", "glass_amber", "glass_green")[i], T((-0.62 + 0.06 * i, -0.72, 0.3)))
    k.add(cyl(0.16, 0.13, 0.25, 10), "timber_ember", T((-0.62, -0.72, 0.115)))
    shadow(k, 1.0)
    return chain(empty("smoke_origin", (0, 0.25, 2.75)), empty("light_origin", (0, -0.95, 0.75)))


def bottle(k, x, y, z, h, r, mat, neck=0.4):
    k.add(lathe([(r * 0.8, 0), (r, h * 0.12), (r, h * (1 - neck)), (r * 0.35, h * (1 - neck * 0.5)), (r * 0.3, h), (r * 0.4, h * 1.04)], 8),
          mat, T((x, y, z)), smooth=True)


def build_glass_rack(k):
    """2 x 1 cooling rack: a timber frame with three slatted shelves of glassware (bottles, bowls, vases,
    floats) in every glass colour, a little shingled roof against the ash, a cloth on the top shelf."""
    rng = random.Random(323)
    cols = ["glass_teal", "glass_amber", "glass_green", "glass_rose", "glass_blue"]
    for x in (-0.86, 0.86):
        for y in (-0.22, 0.22):
            k.add(box(0.07, 0.07, 1.62, 0.01), "timber_ember", T((x, y, 0.79)))
    for z in (0.32, 0.78, 1.24):
        for i in range(4):
            k.add(box(1.76, 0.1, 0.03, 0.005), "wood", T((0, -0.18 + 0.12 * i, z)))
    k.add(prism([(-0.36, 0), (0.36, 0), (0, 0.22)], 1.95), "kiln_brick_dk", T((0, 0, 1.62), (0, 0, 90)))
    for z, items in ((0.335, 6), (0.795, 7), (1.255, 5)):
        x = -0.72
        for i in range(items):
            m = cols[rng.randrange(len(cols))]
            kind = rng.random()
            if kind < 0.55:
                h = rng.uniform(0.18, 0.3)
                bottle(k, x, rng.uniform(-0.08, 0.06), z + 0.015, h, rng.uniform(0.045, 0.065), m)
            elif kind < 0.8:
                k.add(lathe([(0.04, 0), (0.1, 0.03), (0.13, 0.09)], 10), m, T((x, rng.uniform(-0.06, 0.04), z + 0.015)), smooth=True)
            else:
                k.add(sphere(0.07, 10, 6), m, T((x, 0.0, z + 0.085)), smooth=True)
            x += 1.44 / (items - 1) * rng.uniform(0.85, 1.0)
    k.add(box(0.5, 0.36, 0.015, 0.004), "glass_crust", T((0.45, 0.02, 1.27), (0, 4, 0)))
    shadow(k, 0.95, 1.0, 0.4)


def build_ember_vent(k):
    """1 x 1 steam vent (a gate's prop_closed): a cracked basalt mound round a glowing throat, sulphur crust,
    and a tall billow of steam that clearly blocks the way. smoke_origin at the top for the game's puffs."""
    rng = random.Random(324)
    for i in range(7):  # the mound: column stubs leaning out round the throat
        a = i / 7 * math.tau + rng.uniform(-0.2, 0.2)
        r = rng.uniform(0.1, 0.13)
        k.add(cyl(r, r, rng.uniform(0.22, 0.42), 6), rng.choice(["basalt", "basalt_lt", "basalt_dk"]),
              T((0.3 * math.cos(a), 0.3 * math.sin(a), 0.1), (math.degrees(math.sin(a)) * -14, math.degrees(math.cos(a)) * 14, rng.uniform(0, 60))))
    k.add(rock(rng, 0.36, 1, 0.1), "basalt_dk", T((0, 0, 0.0), scale=(1.1, 1.1, 0.42)))
    k.add(cyl(0.15, 0.1, 0.06, 8), "ember_glow", T((0, 0, 0.15)))
    for i in range(9):
        a = rng.uniform(0, math.tau)
        k.add(jitter(ico(0.035, 0), rng, 0.01), "sulphur", T((0.2 * math.cos(a), 0.2 * math.sin(a), 0.2), scale=(1, 1, 0.5)))
    # the billow: puffs of every size, swelling and leaning downwind as they rise
    z, x, y = 0.3, 0.0, 0.0
    for i in range(9):
        r = (0.17, 0.24, 0.3, 0.27, 0.33, 0.26, 0.22, 0.18, 0.12)[i]
        for j in range(2 if 1 <= i <= 6 else 1):
            ox = rng.uniform(-0.14, 0.14) if j else 0
            oy = rng.uniform(-0.1, 0.1) if j else 0
            rr = r * (0.7 if j else 1.0)
            k.add(jitter(ico(rr, 1), rng, rr * 0.12), "steam" if (i + j) % 3 else "steam_dk",
                  T((x + ox, y + oy, z + rr * 0.5), scale=(1.05, 1.0, 0.82)), smooth=True)
        z += r * 0.62
        x += 0.035 * i
        y += 0.012 * i
    contact(k, 0.48)
    return empty("smoke_origin", (x, y, z + 0.1))


def build_lava_glass(k):
    """2 x 2 the Kilnmouth's lava-glass pool (blocked): a rim of glassy obsidian and basalt columns round a
    molten surface, plates of cooling crust drifting on it. The surface glows at night."""
    rng = random.Random(325)
    k.add(cyl(0.82, 0.82, 0.06, 14), "ember_glow", T((0, 0, 0.09)))
    for i in range(6):  # crust plates on the melt
        a = rng.uniform(0, math.tau)
        d = rng.uniform(0.15, 0.55)
        pts = [(0.14 * math.cos(t) * rng.uniform(0.7, 1.2), 0.1 * math.sin(t) * rng.uniform(0.7, 1.2)) for t in (j / 6 * math.tau for j in range(6))]
        k.add(prism(pts, 0.025), "glass_crust", T((d * math.cos(a), d * math.sin(a), 0.125 + 0.002 * i), (90, 0, rng.uniform(0, 180))))
    for i in range(16):  # the rim
        a = i / 16 * math.tau + rng.uniform(-0.08, 0.08)
        r = rng.uniform(0.14, 0.2)
        if i % 3 == 0:
            k.add(cyl(r * 0.8, r * 0.8, rng.uniform(0.25, 0.45), 6), "basalt", T((0.9 * math.cos(a), 0.9 * math.sin(a), 0.12), (0, 0, rng.uniform(0, 60))))
        else:
            k.add(rock(rng, r, 1, 0.15), rng.choice(["obsidian", "obsidian_lt", "basalt_dk"]), T((0.88 * math.cos(a), 0.88 * math.sin(a), 0.06), scale=(1.2, 1.0, 0.7)))
    for i in range(5):  # glass shards catching the light on the rim
        a = rng.uniform(0, math.tau)
        k.add(jitter(ico(0.05, 0), rng, 0.015), rng.choice(["glass_teal", "glass_amber", "obsidian_lt"]), T((0.98 * math.cos(a), 0.98 * math.sin(a), 0.2), scale=(0.6, 0.6, 1.4)))
    shadow(k, 1.05)


# ---------------------------------------------------------------- terrain pieces and scatter


def build_stairs_basalt(k):
    """1 x 1 basalt steps, same contract as stairs: up from the front (-Y) toward the back, tread tops
    0.125 / 0.375 / 0.625 / 0.875, floor 0.5; hex column cheeks rising toward the back."""
    rng = random.Random(326)
    for i in range(4):
        y0 = -0.5 + 0.25 * i
        topz = 0.125 + 0.25 * i
        d = 0.25 if i < 3 else 0.25
        k.add(box(0.72 - 0.006 * i, d, topz + 0.02, 0.012), "basalt_dk", T((0, y0 + d / 2, (topz - 0.02) / 2)))
        k.add(box(0.74, 0.25, 0.04, 0.012), "basalt_top", T((0, y0 + 0.113, topz - 0.015)))
    for sx in (-1, 1):
        for i in range(4):
            r = rng.uniform(0.1, 0.12)
            h = 0.3 + 0.25 * i + rng.uniform(-0.02, 0.06)
            y = -0.38 + 0.25 * i + rng.uniform(-0.015, 0.015)
            col = cyl(r, r, h + 0.02 + 0.007 * i, 6)
            drop_bottom(col)
            k.add(col, rng.choice(["basalt", "basalt_lt"]), T((sx * (0.5 - r * 0.8), y, (h - 0.02 - 0.007 * i) / 2), (0, 0, rng.uniform(0, 60))))
            k.add(cyl(r * 0.92, r * 0.92, 0.02, 6), "basalt_top", T((sx * (0.5 - r * 0.8), y, h + 0.005), (0, 0, rng.uniform(0, 60))))
    contact(k, 0.5)


def build_climb_wall_basalt(k):
    """1 x 1 climb (grip gloves), same contract as climb_wall: two basalt steps (tops 0.5 front, 1.0 back),
    floor 0.5, climbs toward -Z, stacks with y. Columns standing proud on each riser, copper staples hammered in
    as rungs, a chain hanging from the top."""
    rng = random.Random(327)
    for n, (y0, y1, top) in enumerate(((-0.5, 0.0, 0.5), (0.0, 0.5, 1.0))):
        k.add(facet_block(rng, 0.98, 0.42, -0.02, top - 0.03, bulge=0.006), "basalt_dk", T((0, (y0 + y1) / 2 + 0.04, 0)))
        for i in range(4):
            r = rng.uniform(0.115, 0.13)
            x = -0.375 + 0.25 * i
            h = top - 0.004 - 0.003 * i - 0.004 * n
            col = cyl(r, r, h + 0.02, 6)
            drop_bottom(col)
            k.add(col, ("basalt", "basalt_lt")[(i + n) % 2], T((x, y0 + 0.1, (h - 0.02) / 2), (0, 0, 30)))
            k.add(cyl(r * 0.9, r * 0.9, 0.03, 6), "basalt_top", T((x, (y0 + y1) / 2 + 0.12, top - 0.012 - 0.003 * i), (0, 0, 30)))
        for i in range(3):  # staples as rungs, between the columns
            x = -0.25 + 0.25 * i
            z = top - 0.36 + 0.14 * ((i + n) % 2)
            k.add(ribbon([(-0.07, 0), (-0.07, -0.07), (0.07, -0.07), (0.07, 0)], 0.03, 0.03), "copper",
                  T((x, y0 - 0.02, z), (90, 0, 0)))
    for i in range(7):
        k.add(torus(0.03, 0.01, 6, 3), "iron", T((0.37, 0.02 - 0.0 * i, 0.97 - 0.07 * i), (0, 90 * (i % 2), 0)))
    contact(k, 0.5)


def build_rock_basalt(k):
    """1 x 1 scatter: a toppled cluster of basalt columns with a boulder and a few glass glints."""
    rng = random.Random(328)
    k.add(rock(rng, 0.34, 2, 0.1), "basalt_dk", T((-0.12, 0.08, 0.16), scale=(1.1, 0.9, 0.62)))
    for i, (x, y, h, tilt) in enumerate(((0.12, -0.1, 0.62, 8), (0.28, 0.08, 0.44, -12), (-0.02, -0.26, 0.3, 24))):
        r = 0.11
        k.add(cyl(r, r, h, 6), "basalt" if i % 2 else "basalt_lt", T((x, y, h / 2 - 0.03), (tilt, tilt * 0.6, rng.uniform(0, 60))))
        k.add(cyl(r * 0.9, r * 0.9, 0.025, 6), "basalt_top", T((x, y, h - 0.02), (tilt, tilt * 0.6, 0)))
    for i in range(3):
        a = rng.uniform(0, math.tau)
        k.add(jitter(ico(0.04, 0), rng, 0.012), ("glass_teal", "glass_amber", "glass_green")[i], T((0.36 * math.cos(a), 0.36 * math.sin(a), 0.03), scale=(1, 0.7, 0.6)))
    contact(k, 0.5)


def build_cullet(k):
    """Quest item (the item popup): three chunky shards of cullet, teal, amber and green, centred on the origin."""
    rng = random.Random(329)
    for (x, y, z, s, m, rot) in ((-0.05, 0.0, 0.0, 0.09, "glass_teal", (20, 10, 0)), (0.06, 0.03, 0.01, 0.075, "glass_amber", (-15, 30, 40)),
                                 (0.0, -0.05, 0.04, 0.06, "glass_green", (40, -20, 70))):
        bm = jitter(ico(s, 0), rng, s * 0.25)
        k.add(bm, m, T((x, y, z), rot, (1.3, 0.8, 0.9)))
        k.add(jitter(ico(s * 0.5, 0), rng, s * 0.1), "white", T((x - s * 0.3, y - s * 0.4, z + s * 0.35), rot, (0.5, 0.3, 0.3)))


def build_kiln_lantern(k):
    """Item (the popup): the Kiln Lantern. A hand lantern blown at Cinderwick: a copper cap and ring, a bulb of
    amber glass round a glowing coal, a wire bail to carry it by. Centred on the origin."""
    rng = random.Random(330)
    k.add(cyl(0.075, 0.085, 0.03, 10), "copper_dk", T((0, 0, -0.1)))                       # foot
    k.add(lathe([(0.06, -0.085), (0.1, -0.04), (0.11, 0.02), (0.095, 0.07), (0.055, 0.1)], 12), "glass_amber", T((0, 0, 0)), smooth=True)
    k.add(sphere(0.045, 10, 7), "kiln_glow", T((0, 0, -0.01)), smooth=True)                  # the coal
    for i in range(4):                                                                       # sparks
        a = i * 1.6
        k.add(jitter(ico(0.008, 0), rng, 0.002), "kiln_glow", T((0.03 * math.cos(a), 0.03 * math.sin(a), 0.03 + 0.012 * i)))
    for a in range(0, 360, 90):                                                              # copper ribs
        ca, sa = math.cos(math.radians(a)), math.sin(math.radians(a))
        beam(k, (0.105 * ca, 0.105 * sa, -0.06), (0.06 * ca, 0.06 * sa, 0.1), 0.012, 0.012, "copper")
    k.add(cyl(0.065, 0.04, 0.05, 10), "copper", T((0, 0, 0.12)))                             # cap
    k.add(torus(0.035, 0.008, 10, 4), "copper_dk", T((0, 0, 0.15)))
    k.add(torus(0.075, 0.006, 14, 4), "iron", T((0, 0, 0.16), (90, 0, 0), (1, 1, 0.9)))       # bail


def build_glass_marble(k):
    """Curio (the popup): a glass marble as big as a plum, a twist of ember-orange and teal inside clear amber,
    on a little brass stand. Centred on the origin."""
    k.add(sphere(0.09, 14, 10), "glass_amber", T((0, 0, 0.03)), smooth=True)
    k.add(tube([(-0.05, -0.02, 0.0), (-0.02, 0.03, 0.03), (0.02, -0.03, 0.05), (0.05, 0.02, 0.07)], 0.014, 6), "ember_glow", T((0, 0, 0.0)))
    k.add(tube([(0.05, -0.03, -0.01), (0.01, 0.03, 0.02), (-0.03, -0.02, 0.05), (-0.05, 0.03, 0.07)], 0.012, 6), "glass_teal", T((0, 0, 0.0)))
    k.add(cyl(0.05, 0.06, 0.02, 10), "copper", T((0, 0, -0.07)))
    k.add(torus(0.045, 0.008, 12, 4), "copper_dk", T((0, 0, -0.055)))


PROPS = {
    "kiln_lantern": build_kiln_lantern, "glass_marble": build_glass_marble,
    "kiln": build_kiln, "glass_furnace": build_glass_furnace, "glass_rack": build_glass_rack, "ember_vent": build_ember_vent,
    "lava_glass": build_lava_glass, "ash_pine": build_ash_pine, "rock_basalt": build_rock_basalt,
    "cliff_basalt": build_cliff_basalt, "cliff_basalt_b": build_cliff_basalt_b, "stairs_basalt": build_stairs_basalt,
    "climb_wall_basalt": build_climb_wall_basalt, "cullet": build_cullet,
}

VIGNETTE = dict(
    ground=("ash", "stone", "#8f8a84"),
    cliff=("cliff_basalt", "cliff_basalt_b"),
    ring=["ash_pine", "ash_pine", "rock_basalt"],
    tiles=[
        "3333333333........",
        "3333333333........",
        "2222222222........",
        "2222222222........",
        "111111............",
        "1.1111....pppppp..",
        "..........p....p..",
        "..ppppppppp....p..",
        "..p............p..",
        "..p............p..",
        "..pppppppppppppp..",
        "..................",
    ],
    props=[
        ("kiln", 11, 6), ("glass_furnace", 3, 8), ("glass_rack", 6, 8), ("glass_rack", 16, 5, 270),
        ("ember_vent", 3, 2, 0, 2), ("ember_vent", 7, 3, 0, 2), ("lava_glass", 7, 0, 0, 3),
        ("stairs_basalt", 1, 5), ("stairs_basalt", 4, 4, 0, 1), ("climb_wall_basalt", 8, 2, 0, 2), ("waymark", 9, 9), ("rock_basalt", 16, 1), ("rock_basalt", 1, 11),
        ("ash_pine", 14, 2), ("ash_pine", 0, 8), ("ash_pine", 16, 10), ("hero", 10, 8),
    ],
    aim=(9, -6.0, 0), dist=17,
)
