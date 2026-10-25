"""Dream: The Hushlands (Psychic). Willow country in a soft twilight tint, a lake of floating paper lanterns, moths,
fog and hush; busier at night than by day.

Vocabulary: weeping curtains (willows, wisteria), round things (round lake houses, the moonwell's ring of stones,
paper lanterns, lotus leaves), crescent moons in wrought iron, lavender stone and mauve wood, warm paper glow
against cool teal leaves. Glow parts: dr_paper_glow / dr_rose_glow (paper lanterns), dr_window_glow (lake house
windows and cupola), dr_moon_glow (moonflower petals), dr_moonpool_glow (the moonwell's pool).
"""
import math
import random

from v13_kit import (B, T, arc, arch_pts, beam, box, chain, clamp01, contact, curve, cyl, empty, ico, jitter,  # noqa: F401
                     lathe, open_part, prism, ribbon, rock, shadow, sphere, torus, tube, FRONT)

PALETTE = {
    # willow and leaves (cool teal greens that sit in the lilac light)
    "dr_willow": "#7fa898", "dr_willow_dk": "#5d8a80", "dr_willow_lt": "#a6c9ae", "dr_willow_xdk": "#4a7270",
    "dr_bark": "#5e4f60", "dr_bark_dk": "#4a3e4e",
    "dr_leaf": "#4f7469", "dr_leaf_lt": "#6f9585", "dr_leaf_dk": "#3c5d58",
    # paper lanterns and lights
    "dr_paper_glow": "#ffc27a", "dr_rose_glow": "#ff9db4", "dr_window_glow": "#ffd48e",
    "dr_moon_glow": "#eef0ff", "dr_moonpool_glow": "#9fb6ff", "dr_pollen": "#f3dc8e", "dr_bud": "#d8dcef",
    "dr_lotus": "#5f8f74", "dr_lotus_petal": "#f2b9cf",
    # stone, wood, roofs
    "dr_stone": "#b5afc6", "dr_stone_lt": "#d3cede", "dr_stone_dk": "#918ba6", "dr_stone_xdk": "#6c6784",
    "dr_deep": "#1f1c33", "dr_deep_b": "#2f2b4a", "dr_moss": "#6f9481",
    "dr_wood": "#8d6f72", "dr_wood_dk": "#644b56", "dr_wood_lt": "#ab8b88",
    "dr_plaster": "#f0e8f0", "dr_roof": "#8a84b8", "dr_roof_dk": "#6c679b", "dr_roof_lt": "#c4bfe0",
    "dr_iron": "#3b3848", "dr_brass": "#e3c06a", "dr_foam": "#d6e6ee",
    # wisteria and the wishing tree
    "dr_wisteria": "#b38ade", "dr_wisteria_lt": "#d3b4f2", "dr_wisteria_dk": "#8d68c6",
    "dr_blossom": "#eaa6c6", "dr_blossom_lt": "#f6c8db", "dr_blossom_dk": "#cc84ad",
    "dr_ribbon_a": "#e85d75", "dr_ribbon_b": "#f2c14e", "dr_ribbon_c": "#5fb3c9", "dr_ribbon_d": "#9d7be0",
    "dr_tag": "#f4ecd8",
}

WIND = {
    # the curtain sways from its hem; the crown barely
    "willow": lambda x, y, z, r: (clamp01((r - 0.35) / 0.75) * 0.85 + clamp01((z - 1.5) / 2.0) * 0.15) if z > 1.2 or r > 0.6 else 0.0,
    "wishing_tree": lambda x, y, z, r: clamp01((z - 1.2) / 2.2) * 0.6 + (0.35 if 1.0 < z < 2.1 and r > 0.4 else 0.0),
    "wisteria_arch": lambda x, y, z, r: clamp01((2.3 - z) / 0.6) if (abs(x) < 0.8 and z > 1.55) else 0.0,
}

FOOT = {"willow": (2, 2), "moonwell": (3, 3), "lake_house": (4, 4), "wisteria_arch": (2, 1), "wishing_tree": (2, 2)}
FOOT_OTHER = {"seal_stone": (2, 2)}
ANIM = {"moonflower"}

LIGHT = dict(sun=("#ffe9f2", 2.0), from_hero=(8, 18, 6), hemi=("#efe3ff", "#3d4a6b", 1.5), bg="#b9b2dc", fog=(12, 30),
             night=dict(sun=("#9fb4ff", 0.455), hemi=("#2a3557", "#10141f", 0.78)))  # light_night: 30% brighter (the lanterns)


def blade(w, L, kink=0.25):
    """A tapered vertical leaf blade hanging down from z = 0 to z = -L, w wide (X), thin (Y)."""
    return prism([(-w / 2, 0), (w / 2, 0), (w * 0.42, -L * 0.55), (w * 0.12, -L), (-w * 0.12, -L * 0.96), (-w * 0.45, -L * 0.5)], 0.02)


def flat_leaf(L, w):
    """A diamond leaf/petal lying in the XY plane along +X (thin in Z)."""
    return prism([(0, 0), (L * 0.4, w / 2), (L, 0), (L * 0.4, -w / 2)], 0.012), (90, 0, 0)


def paper_lantern(k, M, r=0.13, h=0.26, mat="dr_paper_glow", seg=8):
    """A round paper lantern centred on M (wood rings top and bottom)."""
    prof = [(r * 0.45, -h / 2), (r * 0.85, -h * 0.36), (r, -h * 0.1), (r * 0.98, h * 0.12), (r * 0.82, h * 0.36), (r * 0.45, h / 2)]
    k.add(lathe(prof, seg), mat, M, smooth=True)
    k.add(cyl(r * 0.5, r * 0.5, 0.025, seg), "dr_wood_dk", M @ T((0, 0, h / 2 + 0.008)))
    k.add(cyl(r * 0.5, r * 0.5, 0.025, seg), "dr_wood_dk", M @ T((0, 0, -h / 2 - 0.008)))


# ---------------------------------------------------------------- trees


def build_willow(k):
    """2 x 2 weeping willow (~3.6 m): a stout leaning trunk forking into three limbs under a soft crown, and a
    curtain of long hanging blades all round, hem about 0.6 m off the ground. The Hushlands' ring tree."""
    rng = random.Random(412)
    k.add(cyl(0.27, 0.19, 1.35, 8, 0.02), "dr_bark", T((0, 0, 0.66), (3, 2, 0)), smooth=True)
    for a in range(0, 360, 90):
        k.add(cyl(0.12, 0.03, 0.5, 5), "dr_bark_dk", T((0.2 * math.cos(math.radians(a + 20)), 0.2 * math.sin(math.radians(a + 20)), 0.1), (0, 62, a + 20)), smooth=True)
    for a in (30, 150, 270):
        ca, sa = math.cos(math.radians(a)), math.sin(math.radians(a))
        beam(k, (0.05 * ca, 0.05 * sa, 1.2), (0.55 * ca, 0.55 * sa, 2.35), 0.13, 0.13, "dr_bark")
    for loc, r, m in (((0, 0, 3.0), 0.8, "dr_willow"), ((0.55, -0.3, 2.72), 0.6, "dr_willow_dk"), ((-0.55, 0.28, 2.75), 0.6, "dr_willow_dk"),
                      ((0.1, 0.6, 2.8), 0.58, "dr_willow_dk"), ((-0.25, -0.55, 2.85), 0.55, "dr_willow"), ((0.62, 0.35, 2.85), 0.5, "dr_willow"),
                      ((-0.15, -0.2, 3.4), 0.48, "dr_willow_lt"), ((0.3, 0.15, 3.42), 0.4, "dr_willow_lt")):
        k.add(jitter(ico(r, 1), rng, r * 0.08), m, T(loc, scale=(1, 1, 0.82)), smooth=True)
    # the curtain: blades hang from under the dome's rim, longer at the sides, a ragged hem 0.4-0.9 m up
    radii = [0.95 + 0.0033 * j for j in range(70)]  # every blade on its own radius: no two share a plane
    rng.shuffle(radii)
    for c in range(14):
        a0 = c / 14 * math.tau + rng.uniform(-0.1, 0.1)
        for i in range(5):
            a = a0 + rng.uniform(-0.18, 0.18)
            rr = radii[c * 5 + i]
            top = rng.uniform(2.45, 2.8)
            hem = rng.uniform(0.4, 0.9) + (0.25 if math.sin(a) > 0.6 else 0.0)
            m = rng.choice(["dr_willow", "dr_willow_dk", "dr_willow_lt", "dr_willow_xdk", "dr_willow_dk"])
            k.add(blade(rng.uniform(0.14, 0.2), top - hem), m,
                  T((rr * math.cos(a), rr * math.sin(a), top), (-rng.uniform(3, 9), 0, math.degrees(a) + 90)))
    for c in range(18):  # a shorter, lighter layer higher on the dome hides where the curtain meets it
        a = c / 18 * math.tau + rng.uniform(-0.12, 0.12)
        rr = 0.78 + 0.0077 * ((c * 7) % 18)
        top = rng.uniform(3.0, 3.25)
        k.add(blade(rng.uniform(0.16, 0.22), rng.uniform(0.9, 1.4)), rng.choice(["dr_willow_lt", "dr_willow"]),
              T((rr * math.cos(a), rr * math.sin(a), top), (-rng.uniform(16, 24), 0, math.degrees(a) + 90)))
    contact(k, 0.95)


def build_wishing_tree(k):
    """2 x 2 wishing tree (~3.4 m): an old twisted trunk splitting low, a round crown of dusk-pink blossom, and
    ribbons with little wish tags hanging from the limbs under it; an offering bowl of coins at the roots."""
    rng = random.Random(402)
    for z0, z1, dx, r0, r1 in ((0.0, 0.7, 0.06, 0.34, 0.28), (0.65, 1.25, -0.05, 0.28, 0.24)):
        k.add(cyl(r0, r1, z1 - z0 + 0.04, 8, 0.02), "dr_bark", T((dx, 0, (z0 + z1) / 2), (0, dx * 120, 25 * z0)), smooth=True)
    for a in (0, 70, 150, 220, 290):
        k.add(cyl(0.15, 0.03, 0.62, 5), "dr_bark_dk", T((0.26 * math.cos(math.radians(a)), 0.26 * math.sin(math.radians(a)), 0.12), (0, 64, a)), smooth=True)
    limbs = []
    for a, L, up in ((20, 0.95, 1.05), (140, 0.9, 1.15), (250, 0.85, 1.0), (320, 0.65, 1.3), (80, 0.6, 1.25)):
        ca, sa = math.cos(math.radians(a)), math.sin(math.radians(a))
        p0, p1 = (0.05 * ca, 0.05 * sa, 1.2), (L * ca, L * sa, 1.2 + up)
        beam(k, p0, p1, 0.13, 0.12, "dr_bark")
        limbs.append((p0, p1))
    blobs = []
    for i in range(14):
        a = i * 137.5
        rr = 0.25 + 0.75 * ((i * 0.37) % 1)
        blobs.append(((rr * math.cos(math.radians(a)), rr * math.sin(math.radians(a)), 2.45 + 0.75 * (1 - rr) + rng.uniform(-0.1, 0.12)),
                      rng.uniform(0.38, 0.5), rng.choice(["dr_blossom", "dr_blossom", "dr_blossom_dk", "dr_blossom_lt"])))
    blobs += [((0, 0, 2.95), 0.72, "dr_blossom"), ((0.1, -0.1, 3.4), 0.45, "dr_blossom_lt"), ((0.75, 0.6, 2.35), 0.32, "dr_leaf_lt"), ((-0.8, -0.4, 2.3), 0.3, "dr_leaf")]
    for loc, r, m in blobs:
        k.add(jitter(ico(r, 1), rng, r * 0.1), m, T(loc, scale=(1, 1, 0.8)), smooth=True)
    # ribbons and tags hanging from the limbs below the crown
    cols = ["dr_ribbon_a", "dr_ribbon_b", "dr_ribbon_c", "dr_ribbon_d"]
    for i in range(20):
        p0, p1 = limbs[i % 5]
        t = rng.uniform(0.45, 0.8)
        x, y, z = (p0[j] + (p1[j] - p0[j]) * t for j in range(3))
        z = min(z, 2.1)
        L = rng.uniform(0.45, 0.8)
        yaw = rng.uniform(0, 180)
        k.add(box(0.055, 0.012, L), cols[i % 4], T((x, y, z - L / 2), (rng.uniform(-6, 6), 0, yaw)))
        if i % 2 == 0:
            k.add(box(0.1, 0.014, 0.13), "dr_tag", T((x, y, z - L - 0.065), (0, 0, yaw)))
    # offering bowl with coins and a ring of pebbles
    k.add(lathe([(0.12, -0.02), (0.2, 0.06), (0.24, 0.16), (0.2, 0.17), (0.17, 0.08), (0.0, 0.07)], 10), "dr_stone", T((0.55, -0.55, 0.0)))
    for i in range(5):
        k.add(cyl(0.035, 0.035, 0.012, 8), "dr_brass", T((0.55 + rng.uniform(-0.08, 0.08), -0.55 + rng.uniform(-0.08, 0.08), 0.085 + 0.013 * i), (rng.uniform(-12, 12), 0, 0)))
    for i in range(9):
        a = i / 9 * math.tau
        k.add(rock(rng, 0.07), "dr_stone_lt" if i % 2 else "dr_stone_dk", T((0.62 * math.cos(a), 0.62 * math.sin(a), 0.02), scale=(1, 1, 0.6)))
    contact(k, 0.7)


# ---------------------------------------------------------------- the lake


def build_floating_lantern(k):
    """1 x 1 paper lantern sitting in a lotus on a lily pad, floating on the lake (the game bobs it)."""
    rng = random.Random(403)
    pad = cyl(0.27, 0.27, 0.02, 12)
    k.add(pad, "dr_lotus", T((0, 0, 0.022)))
    k.add(prism([(0, 0), (0.29, 0.05), (0.29, -0.05)], 0.03), "dr_leaf_dk", T((0, 0, 0.031), (90, 0, 15)))  # the pad's notch, darker edge
    for i in range(7):
        a = i / 7 * 360 + rng.uniform(-8, 8)
        bm, rot = flat_leaf(0.2, 0.1)
        k.add(bm, "dr_lotus_petal", T((0, 0, 0.04), (90, -40, a)))
    paper_lantern(k, T((0, 0, 0.25)), r=0.17, h=0.34, mat="dr_paper_glow")
    k.add(cyl(0.012, 0.012, 0.08, 4), "dr_wood_dk", T((0, 0, 0.47)))
    k.add(torus(0.04, 0.009, 8, 3), "dr_wood_dk", T((0, 0, 0.53), (90, 0, 0)))


def build_lantern_raft(k):
    """1 x 1 deck, top exactly 0.1 m: five mauve planks lashed on two floats, a paper lantern on a short pole at
    the back corner. The night lantern path's open state, laid tile by tile across the lake."""
    rng = random.Random(404)
    for i in range(5):
        y = -0.36 + 0.18 * i
        k.add(box(0.92, 0.165, 0.04, 0.008), rng.choice(["dr_wood", "dr_wood_lt", "dr_wood"]), T((rng.uniform(-0.02, 0.02), y, 0.08)))
    for x in (-0.3, 0.3):
        k.add(cyl(0.055, 0.055, 0.9, 7), "dr_wood_dk", T((x, 0, 0.035), (90, 0, 0)))
    for x in (-0.3, 0.3):  # rope lashings over the planks' ends
        for y in (-0.4, 0.4):
            k.add(box(0.07, 0.03, 0.012), "rope", T((x, y, 0.104)))
    k.add(cyl(0.022, 0.018, 0.62, 6), "dr_wood_dk", T((0.36, 0.36, 0.41)))
    k.add(box(0.2, 0.02, 0.02), "dr_wood_dk", T((0.28, 0.36, 0.735)))
    k.add(cyl(0.004, 0.004, 0.07, 3), "dr_iron", T((0.2, 0.36, 0.69)))
    paper_lantern(k, T((0.2, 0.36, 0.57)), r=0.085, h=0.17, mat="dr_rose_glow", seg=8)


def build_ford_stones(k):
    """1 x 1 ford marker over water tiles: three flat stepping stones across the tile (along X), a foam ring round
    each, current streaks and a reed tuft. Low (tops <= 0.1 m); always shown on the boots gate."""
    rng = random.Random(405)
    for x, r, top in ((-0.3, 0.17, 0.09), (0.02, 0.2, 0.075), (0.33, 0.16, 0.085)):
        y = rng.uniform(-0.08, 0.08)
        st = jitter(cyl(r, r * 0.86, top + 0.04, 7), rng, 0.012)
        k.add(st, "dr_stone", T((x, y, (top - 0.04) / 2), (0, 0, rng.uniform(0, 50))))
        k.add(cyl(r * 0.7, r * 0.7, 0.01, 7), "dr_stone_lt", T((x, y, top + 0.003), (0, 0, rng.uniform(0, 50))))
        k.add(torus(r * 1.06, 0.01, 10, 3), "dr_foam", T((x, y, 0.014), scale=(1, 1, 0.5)))
    for i in range(5):  # current streaks, flowing along +Y (toward the camera at rot 0)
        k.add(box(0.022, rng.uniform(0.14, 0.26), 0.006), "dr_foam", T((rng.uniform(-0.42, 0.42), rng.uniform(-0.38, 0.38), 0.012 + 0.002 * i)))
    for i in range(7):
        h = rng.uniform(0.3, 0.55)
        k.add(cyl(0.02, 0.0, h, 3), rng.choice(["dr_leaf", "dr_leaf_lt"]),
              T((0.36 + rng.uniform(-0.06, 0.06), 0.36 + rng.uniform(-0.06, 0.06), h / 2), (rng.uniform(-12, 12), rng.uniform(-12, 12), rng.uniform(0, 120))))


# ---------------------------------------------------------------- flowers and arches


def moon_bloom(k):
    """An open moonflower at the origin (~0.2 m across): five broad, rounded, cupped petals round a pale-gold heart."""
    for i in range(5):
        a = i * 72
        k.add(sphere(0.05, 5, 3), "dr_moon_glow", T((0, 0, 0), (0, 0, a)) @ T((0.052, 0, 0.012), (0, -24, 0), (1.1, 0.85, 0.28)), smooth=True)
    k.add(sphere(0.03, 5, 3), "dr_pollen", T((0, 0, 0.026), scale=(1, 1, 0.75)), smooth=True)


def moon_bud(k):
    """A closed moonflower: a plump pointed teardrop of folded petals in a green cup."""
    k.add(lathe([(0.0, -0.01), (0.03, 0.012), (0.04, 0.034)], 5), "dr_leaf", T(), smooth=True)
    k.add(lathe([(0.04, 0.034), (0.043, 0.062), (0.032, 0.1), (0.012, 0.13), (0.0, 0.142)], 5, 36), "dr_bud", T(), smooth=True)


def build_moonflower(k):
    """1 x 1 moonflower bed: a low cushion of broad teal leaves and seven stems of varied height. Each head is two
    parts sharing an `Open` clip: at rest (day) plump closed buds; at the end (dusk) the buds shrink away as broad
    pale blooms twist open in their place. Petals are `dr_moon_glow`."""
    rng = random.Random(416)
    k.add(jitter(ico(0.3, 1), rng, 0.03), "dr_leaf_dk", T((0, 0, 0.03), scale=(1.3, 1.25, 0.32)), smooth=True)
    for i in range(9):
        bm, _ = flat_leaf(0.29, 0.16)
        k.add(bm, ("dr_leaf", "dr_leaf_lt", "dr_leaf")[i % 3], T((0, 0, 0.075 + 0.004 * (i % 3)), (90, -14 - 6 * (i % 2), i * 40 + rng.uniform(-8, 8))))
    heads = [(-0.22, -0.16, 0.36), (0.2, -0.17, 0.3), (0.02, 0.06, 0.52), (-0.24, 0.18, 0.44), (0.26, 0.15, 0.4),
             (-0.02, -0.28, 0.24), (0.08, 0.3, 0.33)]
    hooks = []
    for i, (x, y, z) in enumerate(heads):
        k.add(tube([(x * 0.25, y * 0.25, 0.06), (x * 0.75 + 0.02, y * 0.75, z * 0.55), (x, y, z - 0.01)], 0.011, 3), "dr_leaf", None)
        hooks.append(open_part(moon_bud, (x, y, z), [(0, (0, 0, 0), (1, 1, 1)), (16, (0, 0, 20), (0.01, 0.01, 0.01))], f"moonflower_bud{i}"))
        hooks.append(open_part(moon_bloom, (x, y, z), [(0, (0, 0, 0), (0.05, 0.05, 0.05)), (24, (0, 0, 40), (1, 1, 1))], f"moonflower_bloom{i}"))
    contact(k, 0.42)
    return chain(*hooks)


def build_wisteria_arch(k):
    """2 x 1 walk-under arch over a path (walkable; posts on the footprint's outer edges): two posts wound with
    vines, a slatted canopy at 2.3 m under a leafy top, and purple racemes hanging to ~1.7 m."""
    rng = random.Random(407)
    for sx in (-1, 1):
        x = sx * 0.9
        k.add(box(0.26, 0.26, 0.12, 0.03), "dr_stone_dk", T((x, 0, 0.045)))
        k.add(box(0.13, 0.13, 2.34, 0.02), "dr_wood", T((x, 0, 1.15)))
        k.add(tube([(x + 0.09 * math.cos(t * 1.9), 0.09 * math.sin(t * 1.9), 0.1 + t * 0.19) for t in range(12)], 0.018, 4), "vine", None)
        for t in range(1, 12, 2):
            a = t * 1.9
            k.add(sphere(0.06, 6, 3), "dr_leaf_lt" if t % 4 == 1 else "dr_leaf", T((x + 0.12 * math.cos(a), 0.12 * math.sin(a), 0.1 + t * 0.19), scale=(1, 0.6, 0.6)), smooth=True)
    for y in (-0.28, 0.28):
        k.add(box(2.14, 0.1, 0.12, 0.015), "dr_wood_dk", T((0, y, 2.28)))
    for i in range(6):
        k.add(box(0.07, 0.86, 0.05, 0.01), "dr_wood_lt", T((-0.88 + i * 0.352, 0, 2.375)))
    for loc, r, m in (((-0.7, 0.0, 2.5), 0.36, "dr_leaf"), ((-0.25, 0.08, 2.55), 0.38, "dr_leaf_dk"), ((0.25, -0.05, 2.55), 0.38, "dr_leaf"),
                      ((0.7, 0.05, 2.5), 0.35, "dr_leaf_dk")):
        k.add(jitter(ico(r, 1), rng, r * 0.1), m, T(loc, scale=(1.4, 1.15, 0.42)), smooth=True)
    for i in range(9):  # blossom showing on top of the leaves
        k.add(jitter(ico(0.12, 1), rng, 0.015), rng.choice(["dr_wisteria", "dr_wisteria_lt"]),
              T((rng.uniform(-0.85, 0.85), rng.uniform(-0.3, 0.3), rng.uniform(2.55, 2.66)), scale=(1.3, 1.1, 0.6)), smooth=True)
    cols = ["dr_wisteria", "dr_wisteria_lt", "dr_wisteria_dk"]
    for i in range(34):
        x = rng.uniform(-1.0, 1.0)
        y = rng.uniform(-0.45, 0.45)
        L = rng.uniform(0.3, 0.55) if abs(x) < 0.7 else rng.uniform(0.55, 0.95)
        top = 2.3 - 0.0016 * i  # distinct caps
        k.add(cyl(0.006, rng.uniform(0.07, 0.1), L, 5), rng.choice(cols), T((x, y, top - L / 2)), smooth=True)
    shadow(k, 1.0, 1.0, 0.42)


# ---------------------------------------------------------------- landmarks


def build_moonwell(k):
    """3 x 3 moonwell: a ring of lavender stone blocks on a round plinth, open at the front, steps winding down the
    inside wall into the dark to a pale glowing pool; a wrought-iron arch with a crescent moon and a paper lantern
    over it. The Moonwell Garden's centrepiece; the well's stair is a door warp (lantern) at its front."""
    rng = random.Random(408)
    k.add(cyl(1.47, 1.5, 0.16, 16, 0.02), "dr_stone_dk", T((0, 0, 0.06), (0, 0, 11.25)))
    k.add(cyl(1.0, 1.0, 0.02, 16), "dr_deep_b", T((0, 0, 0.156), (0, 0, 11.25)))
    k.add(cyl(0.66, 0.66, 0.02, 14), "dr_deep", T((0, 0, 0.165)))
    k.add(cyl(0.4, 0.4, 0.02, 12), "dr_moonpool_glow", T((0, 0, 0.18)))
    gap = 26  # half-width (degrees) of the opening at the front (-Y)
    n = 16
    for course, (z0, z1, off) in enumerate(((0.13, 0.4, 0.0), (0.4, 0.64, 0.5))):
        for i in range(n):
            a = (i + off) / n * 360
            if abs(((a - 270) + 180) % 360 - 180) < gap + 9:
                continue
            ra = math.radians(a)
            L = math.tau * 1.15 / n * 0.92
            m = rng.choice(["dr_stone", "dr_stone", "dr_stone_lt", "dr_stone_dk"])
            dz = 0.005 * (i % 2)  # neighbours never share a plane
            k.add(box(L, 0.34, z1 - z0 - 0.015, 0.02), m, T((1.15 * math.cos(ra), 1.15 * math.sin(ra), (z0 + z1) / 2 + dz), (0, 0, a + 90)))
    for i in range(n):  # coping
        a = (i + 0.25) / n * 360
        if abs(((a - 270) + 180) % 360 - 180) < gap + 9:
            continue
        ra = math.radians(a)
        k.add(box(math.tau * 1.16 / n * 0.97, 0.42, 0.08), "dr_stone_lt", T((1.16 * math.cos(ra), 1.16 * math.sin(ra), 0.68 + 0.006 * (i % 2)), (0, 0, a + 90)))
    for sx in (-1, 1):  # gate stones flanking the opening, with a moss cap
        a = math.radians(270 + sx * (gap + 6))
        k.add(box(0.3, 0.42, 0.86, 0.03), "dr_stone_dk", T((1.18 * math.cos(a), 1.18 * math.sin(a), 0.5), (0, 0, math.degrees(a) + 90)))
        k.add(jitter(ico(0.15, 1), rng, 0.02), "dr_moss", T((1.18 * math.cos(a), 1.18 * math.sin(a), 0.94), scale=(1.2, 1.2, 0.45)), smooth=True)
    # steps down the inner wall: from the left of the opening, round the back, darkening as they go down
    steps = ["dr_stone_lt", "dr_stone_lt", "dr_stone", "dr_stone", "dr_stone_dk", "dr_stone_dk", "dr_stone_xdk", "dr_stone_xdk", "dr_deep_b"]
    for i, m in enumerate(steps):
        a = 270 - gap - 14 - i * 30
        ra = math.radians(a)
        top = 0.6 - i * 0.052
        b0 = 0.12 + 0.004 * i
        k.add(box(0.3, 0.3, top - b0), m, T((0.82 * math.cos(ra), 0.82 * math.sin(ra), (top + b0) / 2), (0, 0, a + 90)))
    for x, y in ((-0.5, 0.95), (0.85, 0.55), (-0.95, -0.4)):  # moss on the blocks
        k.add(jitter(ico(0.1, 1), rng, 0.02), "dr_moss", T((x, y, 0.72), scale=(1.4, 1.0, 0.35)), smooth=True)
    # wrought-iron arch over the well, crescent moon on top, a lantern hanging over the pool
    for sx in (-1, 1):
        k.add(box(0.09, 0.09, 1.95, 0.01), "dr_iron", T((sx * 1.2, 0.0, 1.1)))
        k.add(sphere(0.06, 6, 4), "dr_brass", T((sx * 1.2, 0.0, 2.1)))
    k.add(ribbon(arc(0, 1.95, 1.2, 180, 0, 12), 0.07, 0.07), "dr_iron", T((0, 0.035, 0), FRONT))
    outer = arc(0, 0, 0.26, 60, 300, 12)
    inner = arc(0.12, 0, 0.22, 300, 60, 10)[1:-1]
    k.add(prism([(x - 0.04, z + 3.45) for x, z in outer + inner], 0.05), "dr_brass", T())
    k.add(cyl(0.012, 0.012, 0.38, 4), "dr_iron", T((0, 0, 2.97)))
    paper_lantern(k, T((0, 0, 2.66)), r=0.13, h=0.24, mat="dr_paper_glow")
    contact(k, 1.5)
    return empty("light_origin", (0, 0, 2.66))


def build_lake_house(k):
    """4 x 4 round house on piles over the lake: a ring deck at 0.3 m with a rail (open at the front), plaster walls
    with round glowing windows, a flared indigo roof with a lantern cupola, and a string of paper lanterns under the
    eaves. Door on the front, on column 2 (origin + (2, 3) in tiles)."""
    rng = random.Random(409)
    for i in range(10):  # piles
        a = i / 10 * math.tau
        k.add(cyl(0.08, 0.07, 0.36, 6), "dr_wood_dk", T((1.7 * math.cos(a), 1.7 * math.sin(a), 0.14)))
    k.add(cyl(0.1, 0.1, 0.36, 6), "dr_wood_dk", T((0, 0, 0.14)))
    k.add(cyl(1.95, 1.95, 0.08, 16), "dr_wood", T((0, 0, 0.26)))
    k.add(cyl(1.98, 1.98, 0.05, 16), "dr_wood_dk", T((0, 0, 0.205)))
    # rail round the deck, open at the front
    posts = [a for a in range(0, 360, 30) if abs(a - 270) > 25]
    for a in posts:
        ra = math.radians(a)
        k.add(box(0.07, 0.07, 0.55), "dr_wood_dk", T((1.85 * math.cos(ra), 1.85 * math.sin(ra), 0.57)))
    for i, (a, b) in enumerate(zip(posts, posts[1:])):
        if b - a > 30:
            continue
        z = 0.8 + 0.012 * (i % 2)
        pa = (1.85 * math.cos(math.radians(a)), 1.85 * math.sin(math.radians(a)), z)
        pb = (1.85 * math.cos(math.radians(b)), 1.85 * math.sin(math.radians(b)), z)
        beam(k, pa, pb, 0.06, 0.05, "dr_wood_lt")
    # walls
    k.add(cyl(1.35, 1.35, 1.95, 16), "dr_plaster", T((0, 0, 1.27)))
    k.add(cyl(1.38, 1.38, 0.14, 16), "dr_wood_dk", T((0, 0, 0.36)))
    k.add(cyl(1.4, 1.4, 0.12, 16), "dr_wood_dk", T((0, 0, 2.2)))
    for i in range(8):
        a = i / 8 * 360 + 22.5
        ra = math.radians(a)
        k.add(box(0.1, 0.06, 1.8), "dr_wood", T((1.37 * math.cos(ra), 1.37 * math.sin(ra), 1.27), (0, 0, a + 90)))
    # door on column 2: the round wall at x = 0.5, turned to face out
    dy = -math.sqrt(1.35 ** 2 - 0.5 ** 2)
    phi = math.degrees(math.atan2(0.5 / 1.35, -dy / 1.35))
    D = T((0.5, dy, 0.31), (0, 0, phi))
    k.add(prism(arch_pts(0.78, 1.55, 8), 0.08), "dr_wood_dk", D @ T((0, -0.02, 0.0)))
    k.add(ribbon(arch_pts(0.86, 1.6, 8), 0.07, 0.06), "dr_wood_lt", D @ T((0, -0.03, -0.012), FRONT))
    k.add(sphere(0.035, 6, 4), "dr_brass", D @ T((0.22, -0.08, 0.75)))
    k.add(box(0.95, 0.4, 0.04), "dr_wood_lt", D @ T((0, -0.3, 0.006)))
    # round windows (glow at night), front-left, left, right, back
    for a in (232, 180, 330, 0):
        ra = math.radians(a)
        M = T((1.36 * math.cos(ra), 1.36 * math.sin(ra), 1.5), (0, 0, a + 90))
        k.add(cyl(0.24, 0.24, 0.05, 10), "dr_window_glow", M @ T((0, 0, 0), (90, 0, 0)))
        k.add(torus(0.25, 0.04, 10, 3), "dr_wood_lt", M @ T((0, -0.03, 0), (90, 0, 0)))
        k.add(box(0.48, 0.03, 0.03), "dr_wood_lt", M @ T((0, -0.04, 0)))
        k.add(box(0.03, 0.03, 0.48), "dr_wood_lt", M @ T((0, -0.047, 0)))
        k.add(box(0.5, 0.16, 0.06, 0.01), "dr_wood", M @ T((0, -0.08, -0.29)))
    # flared roof, cupola with a lantern, crescent finial, stovepipe
    k.add(lathe([(1.98, 2.22), (1.72, 2.42), (1.2, 2.88), (0.55, 3.4), (0.28, 3.62)], 16), "dr_roof", T(), smooth=False)
    k.add(lathe([(2.02, 2.17), (2.02, 2.25), (1.95, 2.29)], 16), "dr_roof_dk", T())
    for z, r in ((2.6, 1.55), (3.05, 1.0)):
        k.add(cyl(r + 0.02, r - 0.05, 0.06, 16), "dr_roof_lt", T((0, 0, z)))
    k.add(cyl(0.26, 0.26, 0.36, 8), "dr_window_glow", T((0, 0, 3.75)))
    for i in range(8):
        a = math.radians(i * 45 + 22.5)
        k.add(box(0.05, 0.05, 0.38), "dr_wood_dk", T((0.27 * math.cos(a), 0.27 * math.sin(a), 3.75)))
    k.add(cyl(0.4, 0.0, 0.42, 8), "dr_roof_dk", T((0, 0, 4.13)))
    outer = arc(0, 0, 0.13, 60, 300, 8)
    inner = arc(0.06, 0, 0.11, 300, 60, 6)[1:-1]
    k.add(prism([(x, z + 4.5) for x, z in outer + inner], 0.03), "dr_brass", T())
    k.add(cyl(0.07, 0.07, 0.6, 6), "dr_iron", T((-0.9, 0.55, 3.0)))
    k.add(cyl(0.11, 0.11, 0.06, 6), "dr_iron", T((-0.9, 0.55, 3.32)))
    # a string of paper lanterns round the eaves
    n = 9
    pts = [(2.05 * math.cos(math.radians(a)), 2.05 * math.sin(math.radians(a)), 2.18) for a in (i / n * 360 + 10 for i in range(n))]
    for i in range(n):
        a, b = pts[i], pts[(i + 1) % n]
        k.add(tube(curve(a, b, 0.14, 4), 0.012, 3), "rope", None)
        mid = [(a[j] + b[j]) / 2 for j in range(3)]
        mid[2] -= 0.3
        k.add(cyl(0.006, 0.006, 0.12, 3), "dr_iron", T((mid[0], mid[1], mid[2] + 0.16)))
        k.add(sphere(0.13, 6, 4), ("dr_paper_glow", "dr_rose_glow")[i % 2], T(mid, scale=(1, 1, 1.15)), smooth=True)
    _ = rng
    return empty("smoke_origin", (-0.9, 0.55, 3.35))


PROPS = {
    "willow": build_willow, "wishing_tree": build_wishing_tree, "floating_lantern": build_floating_lantern,
    "lantern_raft": build_lantern_raft, "ford_stones": build_ford_stones, "moonflower": build_moonflower,
    "wisteria_arch": build_wisteria_arch, "moonwell": build_moonwell, "lake_house": build_lake_house,
}

VIGNETTE = dict(
    ground=("dusk_grass", "stone", "#6f8e88"),
    ring=["willow", "willow", "bush_dark"],
    water="#5d7fb8",
    sea_beyond="e",
    tiles=[
        ".........p.......",
        ".........pp...~~~",
        "..........p..~~~~",
        "..........pp=~~~~",
        "...........=~~~~~",
        "..........~~~~~~~",
        "~~.......~~~~~~~~",
        "~~.....pp~~~~~~~~",
        "~~...ppp~~~~~~~~~",
        "~~pppp..~~~~~~~~~",
        "~~p....~~~~~~~~~~",
    ],
    props=[
        ("moonwell", 2, 0), ("seal_stone", 6, 1), ("waymark", 7, 3),
        ("moonflower", 1, 3), ("moonflower", 2, 3), ("moonflower", 3, 3), ("moonflower", 5, 0), ("moonflower", 1, 4),
        ("moonflower", 5, 3), ("moonflower", 6, 4),
        ("wisteria_arch", 9, 1), ("willow", 3, 5), ("wishing_tree", 6, 7),
        ("lake_house", 13, 1),
        ("lantern_raft", 11, 5), ("lantern_raft", 12, 6), ("lantern_raft", 13, 7), ("lantern_raft", 14, 8), ("lantern_raft", 15, 9),
        ("floating_lantern", 10, 8), ("floating_lantern", 11, 10), ("floating_lantern", 16, 6), ("floating_lantern", 9, 10),
        ("ford_stones", 0, 7), ("ford_stones", 1, 7), ("ford_stones", 0, 8), ("ford_stones", 1, 8),
        ("hero", 8, 5),
    ],
    aim=(8.5, -5.8, 0), dist=16.5,
)
