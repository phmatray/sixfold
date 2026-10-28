"""Ambient life (#115, #189): the critters, the perch bird and the docking fishing boat a map's `life` spawns, as .glb.

Headless:
    blender -b --factory-startup -P tools/blender/build_critters.py -- --out web/world/assets [--only critter_crab,fishing_boat]

From a live Blender (the MCP): exec this file with a global `ARGS = ["--out", ...]`.

Reuses the helpers of build_assets.py and only ever writes its own files: `critter_<kind>.glb` (rabbit, squirrel,
hedgehog, duck, crab, lizard, goat, glowbeetle, bird) and `fishing_boat.glb`. Same conventions: Blender Z-up, front
facing -Y (glTF +Z), origin on the ground under the feet, 1 tile = 1 m, at the size the game draws them (storybook-sized
next to the hero: a rabbit is ~0.4 m). Flat colours only and no contact shadow (they hop and fly): life.js merges each
model into one vertex-coloured mesh and draws every critter of a kind in one instanced draw, its hop, peck and flight
procedural. The boat is long along +X (bow +X), like boat.glb, waterline at 0.

Each region keeps its palette: grove browns, tide reds, ember charcoal with a glowing seam, stone creams, dream dusk
blues with a lantern of a belly, downs greys.
"""
import math
import os
import random
import sys

import bpy
from mathutils import Matrix, Vector

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import build_assets as B  # noqa: E402
from build_assets import T, box, cyl, ico, jitter, prism, sphere  # noqa: E402

B.PALETTE.update({
    # grove (Fernhollow, Rootholm, the Elder Ring): rabbit, squirrel, hedgehog
    "cr_rabbit": "#9b8a76", "cr_rabbit_dk": "#7f705f", "cr_rabbit_lt": "#e6d8c2", "cr_tail": "#f6f1e8", "cr_pink": "#e3a69c",
    "cr_squirrel": "#b0623a", "cr_squirrel_lt": "#d08452", "cr_cream": "#efdcb8",
    "cr_spine": "#5a4636", "cr_spine_lt": "#7d6650", "cr_face": "#d6b896",
    "cr_eye": "#241d26", "cr_nose": "#3a2a2a",
    # tide (Gullhaven, Bellflats, the seaside): crab, duck
    "cr_crab": "#d8553a", "cr_crab_dk": "#a83a2a", "cr_crab_lt": "#f08a5a",
    "cr_duck": "#8a6a4a", "cr_duck_dk": "#5e4632", "cr_duck_head": "#2f6b45", "cr_bill": "#e8a23a", "cr_speculum": "#3d6fb0",
    # ember (Cinderwick, Kilnmouth, the quarry): an ash-dark lizard with a glowing seam
    "cr_lizard": "#4b4542", "cr_lizard_dk": "#35302e", "cr_ember": "#ec7a2c", "cr_amber": "#f2c14a",
    # stone (the Bastion, Scree Pass): a cream goat with dark horns
    "cr_goat": "#ebe4d6", "cr_goat_dk": "#cfc4b0", "cr_horn": "#6b5a48", "cr_hoof": "#3a322c",
    # dream (Lanternmere, Moonwell, Hush Lane): a dusk-blue beetle with a lantern belly (the game adds its glow)
    "cr_shell": "#3a4370", "cr_shell_lt": "#5664a0", "cr_glow": "#d9ff8a",
    # downs (Thrumming Downs): a little chat on the stones
    "cr_bird": "#626b74", "cr_bird_dk": "#3c3f45", "cr_breast": "#c4703c", "cr_belly": "#ece2cf",
    # the fishing boat docking at a pier
    "fb_hull": "#2f6f7a", "fb_hull_dk": "#245760", "fb_stripe": "#f1e2c2", "fb_deck": "#b8875a", "fb_mast": "#8a6440",
    "fb_sail": "#efe3c8", "fb_rope": "#c9a46a", "fb_crate": "#a8754a", "fb_fish": "#9fb8c4", "fb_lamp": "#ffd98a",
})
BUDGET = {"fishing_boat": 2000}  # triangles; every critter <= 1200 (instanced: at most 12 in view, life_schedule.js)


def pair(fn):
    for s in (-1, 1):
        fn(s)


def eyes(k, x, y, z, r=0.016):
    pair(lambda s: k.add(sphere(r, 8, 6), "cr_eye", T((s * x, y, z)), smooth=True))


# ---------------------------------------------------------------- grove


def build_rabbit(k):
    k.add(sphere(0.13, 12, 8), "cr_rabbit", T((0, 0.04, 0.13), scale=(0.95, 1.3, 0.95)), smooth=True)
    k.add(sphere(0.1, 10, 6), "cr_rabbit_lt", T((0, -0.02, 0.09), scale=(0.8, 1.1, 0.8)), smooth=True)   # belly
    pair(lambda s: k.add(sphere(0.075, 10, 6), "cr_rabbit", T((s * 0.085, 0.09, 0.085), scale=(0.7, 1.2, 1)), smooth=True))  # haunches
    pair(lambda s: k.add(box(0.05, 0.12, 0.03, 0.012), "cr_rabbit_dk", T((s * 0.08, 0.06, 0.015))))  # hind feet
    pair(lambda s: k.add(cyl(0.022, 0.02, 0.08, 6), "cr_rabbit_lt", T((s * 0.05, -0.11, 0.04))))   # forepaws
    k.add(sphere(0.085, 12, 8), "cr_rabbit", T((0, -0.14, 0.23), scale=(0.9, 1.1, 0.92)), smooth=True)
    k.add(sphere(0.045, 8, 6), "cr_rabbit_lt", T((0, -0.215, 0.205), scale=(1.1, 0.8, 0.8)), smooth=True)  # muzzle
    k.add(sphere(0.012, 6, 4), "cr_pink", T((0, -0.25, 0.22)))
    eyes(k, 0.055, -0.19, 0.25)
    for s in (-1, 1):  # long ears, tipped back and out
        k.add(box(0.045, 0.022, 0.17, 0.01), "cr_rabbit", T((s * 0.035, -0.1, 0.37), (-14, s * 10, 0)))
        k.add(box(0.026, 0.006, 0.12), "cr_pink", T((s * 0.035, -0.114, 0.37), (-14, s * 10, 0)))
    k.add(jitter(ico(0.045, 1), random.Random(11), 0.006), "cr_tail", T((0, 0.21, 0.16)), smooth=True)


def build_squirrel(k):
    k.add(sphere(0.085, 12, 8), "cr_squirrel", T((0, 0.0, 0.1), scale=(0.9, 1.25, 1.05)), smooth=True)
    k.add(sphere(0.065, 10, 6), "cr_cream", T((0, -0.045, 0.1), scale=(0.8, 0.9, 1.05)), smooth=True)   # chest
    k.add(sphere(0.062, 10, 8), "cr_squirrel", T((0, -0.1, 0.19), scale=(0.95, 1.15, 0.95)), smooth=True)
    k.add(sphere(0.03, 8, 6), "cr_cream", T((0, -0.155, 0.175)), smooth=True)
    k.add(sphere(0.01, 6, 4), "cr_nose", T((0, -0.18, 0.185)))
    eyes(k, 0.04, -0.14, 0.21, 0.013)
    pair(lambda s: k.add(cyl(0.022, 0.0, 0.05, 5), "cr_squirrel", T((s * 0.033, -0.09, 0.255), (10, s * 12, 0))))  # ears
    pair(lambda s: k.add(box(0.03, 0.08, 0.025, 0.008), "cr_squirrel", T((s * 0.05, 0.02, 0.013))))   # feet
    pair(lambda s: k.add(cyl(0.015, 0.013, 0.06, 6), "cr_cream", T((s * 0.03, -0.1, 0.11), (30, 0, 0))))  # paws held up
    # The tail: a plume of three puffs curling up the back, lighter at the tip.
    for (y, z, r, mat) in ((0.11, 0.1, 0.055, "cr_squirrel"), (0.16, 0.19, 0.068, "cr_squirrel"), (0.14, 0.3, 0.064, "cr_squirrel_lt"),
                           (0.09, 0.36, 0.045, "cr_squirrel_lt")):
        k.add(jitter(ico(r, 1), random.Random(int(z * 100)), r * 0.12), mat, T((0, y, z), scale=(0.8, 1, 1)), smooth=True)


def build_hedgehog(k):
    rng = random.Random(23)
    k.add(jitter(ico(0.13, 2), rng, 0.012), "cr_spine", T((0, 0.03, 0.09), scale=(1.0, 1.25, 0.72)), smooth=True)
    for i in range(16):  # a coat of short spikes over the back
        a, e = rng.uniform(0, math.tau), rng.uniform(0.25, 1.2)
        x, y, z = 0.13 * math.cos(a) * math.cos(e), 0.16 * math.sin(a) * math.cos(e) + 0.03, 0.09 * math.sin(e) + 0.09
        if y < -0.06:
            continue
        aim = Vector((0, 0, 1)).rotation_difference(Vector((x, y - 0.06, z - 0.05)).normalized())  # out from the middle, raked back
        k.add(cyl(0.022, 0.0, 0.07, 4), "cr_spine_lt" if i % 3 else "cr_spine", Matrix.Translation((x, y, z)) @ aim.to_matrix().to_4x4())
    k.add(sphere(0.058, 10, 6), "cr_face", T((0, -0.12, 0.07), scale=(0.95, 1.35, 0.85)), smooth=True)
    k.add(sphere(0.016, 6, 4), "cr_nose", T((0, -0.2, 0.075)))
    eyes(k, 0.035, -0.15, 0.095, 0.012)
    pair(lambda s: k.add(sphere(0.018, 6, 4), "cr_face", T((s * 0.045, -0.1, 0.12))))   # ears
    for sx in (-1, 1):
        for sy in (-0.08, 0.1):
            k.add(cyl(0.02, 0.02, 0.04, 6), "cr_face", T((sx * 0.07, sy, 0.02)))


# ---------------------------------------------------------------- tide


def build_crab(k):
    k.add(sphere(0.11, 12, 8), "cr_crab", T((0, 0, 0.075), scale=(1.4, 1.0, 0.5)), smooth=True)
    k.add(sphere(0.1, 12, 6), "cr_crab_dk", T((0, 0.01, 0.06), scale=(1.42, 1.02, 0.35)))   # underside rim
    for s in (-1, 1):
        for i in range(3):  # walking legs, splayed and bent down
            y = 0.04 - i * 0.045
            k.add(box(0.12, 0.018, 0.018), "cr_crab_dk", T((s * 0.19, y, 0.06), (0, s * -30, s * (8 - i * 12))))
            k.add(box(0.016, 0.016, 0.07), "cr_crab_dk", T((s * 0.25, y - 0.01 * (i - 1), 0.03), (0, s * 15, 0)))
        k.add(cyl(0.022, 0.018, 0.1, 6), "cr_crab", T((s * 0.14, -0.1, 0.07), (90, 0, s * 40)))   # arm
        k.add(sphere(0.05, 10, 6), "cr_crab_lt", T((s * 0.18, -0.17, 0.07), scale=(0.9, 1.4, 0.75)), smooth=True)  # claw
        k.add(box(0.025, 0.06, 0.02, 0.006), "cr_crab_dk", T((s * 0.155, -0.235, 0.07), (0, 0, s * -12)))     # pincer
        k.add(cyl(0.008, 0.008, 0.06, 5), "cr_crab_dk", T((s * 0.035, -0.085, 0.13)))    # eye stalks
        k.add(sphere(0.016, 8, 6), "cr_eye", T((s * 0.035, -0.085, 0.165)), smooth=True)


def build_duck(k):
    k.add(sphere(0.12, 12, 8), "cr_duck", T((0, 0.02, 0.12), scale=(0.95, 1.4, 0.72)), smooth=True)
    k.add(sphere(0.1, 10, 6), "cr_duck_dk", T((0, 0.0, 0.1), scale=(0.98, 1.3, 0.55)))   # waterline shade
    k.add(cyl(0.06, 0.0, 0.1, 6), "cr_duck_dk", T((0, 0.19, 0.15), (-65, 0, 0)))        # upturned tail
    for s in (-1, 1):  # folded wings with a blue flash
        k.add(sphere(0.07, 10, 6), "cr_duck_dk", T((s * 0.085, 0.05, 0.14), scale=(0.35, 1.5, 0.6)), smooth=True)
        k.add(box(0.008, 0.05, 0.02), "cr_speculum", T((s * 0.112, 0.06, 0.15)))
    k.add(cyl(0.04, 0.045, 0.06, 8), "cr_duck", T((0, -0.12, 0.2)))
    k.add(cyl(0.043, 0.043, 0.012, 8), "cr_tail", T((0, -0.12, 0.215)))                   # white collar
    k.add(sphere(0.065, 12, 8), "cr_duck_head", T((0, -0.14, 0.27), scale=(0.85, 1.1, 0.9)), smooth=True)
    k.add(box(0.045, 0.07, 0.018, 0.008), "cr_bill", T((0, -0.215, 0.255), (-8, 0, 0)))
    eyes(k, 0.045, -0.165, 0.29, 0.012)
    pair(lambda s: k.add(box(0.04, 0.05, 0.008), "cr_bill", T((s * 0.04, 0.0, 0.004))))  # webbed feet


# ---------------------------------------------------------------- ember


def build_lizard(k):
    k.add(sphere(0.07, 12, 6), "cr_lizard", T((0, 0.0, 0.045), scale=(0.72, 1.9, 0.45)), smooth=True)
    k.add(sphere(0.05, 10, 6), "cr_lizard", T((0, -0.17, 0.05), scale=(0.82, 1.3, 0.6)), smooth=True)
    k.add(cyl(0.035, 0.004, 0.3, 6), "cr_lizard_dk", T((0, 0.25, 0.035), (90, 0, 0), scale=(1, 0.6, 1)))   # tail, flattened
    for i, y in enumerate((-0.09, -0.03, 0.03, 0.09, 0.15)):   # a glowing seam of ember spots down the back
        k.add(box(0.03 - i * 0.003, 0.03, 0.01), "cr_ember", T((0, y, 0.077 - abs(y) * 0.06)))
    for sx in (-1, 1):
        for sy in (-0.07, 0.08):  # splayed legs and toes
            k.add(box(0.07, 0.022, 0.018), "cr_lizard_dk", T((sx * 0.07, sy, 0.025), (0, sx * -25, sx * (-20 if sy < 0 else 20))))
            k.add(box(0.03, 0.035, 0.008), "cr_lizard_dk", T((sx * 0.105, sy - 0.01, 0.004)))
    eyes(k, 0.03, -0.19, 0.07, 0.011)
    pair(lambda s: k.add(sphere(0.006, 6, 4), "cr_amber", T((s * 0.037, -0.198, 0.072))))   # amber glints


# ---------------------------------------------------------------- stone


def build_goat(k):
    k.add(box(0.3, 0.56, 0.28, 0.07, 2), "cr_goat", T((0, 0.0, 0.47)))
    k.add(box(0.24, 0.2, 0.2, 0.05), "cr_goat_dk", T((0, 0.12, 0.5), scale=(1.28, 1, 1.2)))   # a grey saddle patch
    for sx in (-1, 1):
        for sy in (-0.19, 0.19):
            k.add(cyl(0.035, 0.03, 0.32, 6), "cr_goat_dk", T((sx * 0.1, sy, 0.19)))
            k.add(cyl(0.034, 0.036, 0.05, 6), "cr_hoof", T((sx * 0.1, sy, 0.025)))
    k.add(box(0.11, 0.12, 0.2, 0.04), "cr_goat", T((0, -0.3, 0.62), (-25, 0, 0)))                # neck
    k.add(box(0.14, 0.24, 0.15, 0.05, 2), "cr_goat", T((0, -0.4, 0.72), (18, 0, 0)))            # head, muzzle down
    k.add(cyl(0.03, 0.005, 0.1, 5), "cr_goat_dk", T((0, -0.47, 0.61), (180, 0, 0)))             # beard
    for s in (-1, 1):
        k.add(cyl(0.028, 0.004, 0.17, 6), "cr_horn", T((s * 0.04, -0.33, 0.83), (-55, s * 8, 0)))
        k.add(box(0.1, 0.03, 0.045, 0.012), "cr_goat", T((s * 0.1, -0.35, 0.74), (0, s * 25, s * -10)))   # ears
    eyes(k, 0.068, -0.43, 0.75, 0.015)
    k.add(cyl(0.03, 0.0, 0.1, 5), "cr_goat", T((0, 0.3, 0.6), (-125, 0, 0)))                    # tail, flicked up


# ---------------------------------------------------------------- dream


def build_glowbeetle(k):
    k.add(sphere(0.045, 10, 8), "cr_glow", T((0, 0.035, 0.035), scale=(0.95, 1.35, 0.8)), smooth=True)   # the lantern belly
    for s in (-1, 1):  # wing cases, parted over the glow
        k.add(sphere(0.042, 10, 6), "cr_shell", T((s * 0.024, -0.01, 0.05), scale=(0.6, 1.25, 0.55), rot=(0, s * -12, s * -6)), smooth=True)
        for sy in (-0.035, 0.0, 0.03):
            k.add(box(0.05, 0.006, 0.006), "cr_shell", T((s * 0.04, sy, 0.012), (0, 0, s * (sy * 400))))   # legs
        k.add(cyl(0.003, 0.003, 0.05, 4), "cr_shell_lt", T((s * 0.018, -0.085, 0.06), (-55, s * 20, 0)))  # antennae
    k.add(sphere(0.022, 8, 6), "cr_shell_lt", T((0, -0.06, 0.035)), smooth=True)


# ---------------------------------------------------------------- downs


def build_bird(k):
    k.add(sphere(0.065, 12, 8), "cr_bird", T((0, 0.01, 0.11), scale=(0.9, 1.35, 0.92)), smooth=True)
    k.add(sphere(0.055, 10, 6), "cr_breast", T((0, -0.035, 0.1), scale=(0.88, 0.9, 0.95)), smooth=True)
    k.add(sphere(0.045, 10, 6), "cr_belly", T((0, 0.0, 0.075), scale=(0.85, 1.1, 0.7)), smooth=True)
    k.add(sphere(0.046, 10, 8), "cr_bird_dk", T((0, -0.075, 0.175)), smooth=True)               # dark cap
    k.add(cyl(0.012, 0.0, 0.035, 5), "cr_nose", T((0, -0.13, 0.172), (90, 0, 0)))              # beak
    pair(lambda s: k.add(sphere(0.012, 6, 4), "cr_tail", T((s * 0.036, -0.095, 0.176))))      # white cheek
    eyes(k, 0.03, -0.11, 0.185, 0.01)
    for s in (-1, 1):  # folded wings, tips crossing over the tail
        k.add(sphere(0.05, 8, 6), "cr_bird_dk", T((s * 0.05, 0.03, 0.12), scale=(0.35, 1.55, 0.6), rot=(-10, 0, s * 6)), smooth=True)
        k.add(cyl(0.006, 0.006, 0.06, 4), "cr_nose", T((s * 0.02, 0.0, 0.03)))                  # legs
    k.add(box(0.05, 0.11, 0.012, 0.004), "cr_bird_dk", T((0, 0.12, 0.11), (-20, 0, 0)))         # tail


# ---------------------------------------------------------------- the boat


def build_fishing_boat(k):
    L, W, WALL = 1.0, 0.42, 0.15  # half length (bow +X), half beam, bulwark height over the deck

    def deck(x):  # the deck rises a little to the bow
        return 0.2 + 0.12 * max(0.0, x / L) ** 2

    def width(x):  # plan taper: a pointed bow, a fuller transom
        t = x / L
        return W * (1 - max(0.0, t) ** 2 * 0.92 - max(0.0, -t) ** 4 * 0.35)

    xs = [x / 10 * L for x in range(-10, 11)]
    hull = prism([(x, deck(x)) for x in xs] + [(x * 0.94, -0.1 + 0.08 * (x / L) ** 4) for x in reversed(xs)], 2)
    for v in hull.verts:  # narrower at the keel
        v.co.y *= width(v.co.x) * (0.62 if v.co.z < 0.1 else 1)
    k.add(hull, "fb_hull")
    planks = prism([(x * 0.97, deck(x) + 0.008) for x in xs] + [(x * 0.97, deck(x) - 0.01) for x in reversed(xs)], 2)
    for v in planks.verts:
        v.co.y *= width(v.co.x) * 0.92
    k.add(planks, "fb_deck")
    for s in (-1, 1):  # bulwarks along the sheer, a cream stripe outside and a dark rail on top, in segments
        for i in range(10):
            x0, x1 = (i - 5) / 5 * L * 0.93, (i - 4) / 5 * L * 0.93
            xm, w0, w1 = (x0 + x1) / 2, width(x0), width(x1)
            ln, z, rot = math.hypot(x1 - x0, w1 - w0) + 0.02, deck(xm), (0, 0, math.degrees(math.atan2(s * (w1 - w0), x1 - x0)))
            wm = (w0 + w1) / 2
            k.add(box(ln, 0.03, WALL), "fb_hull", T((xm, s * (wm - 0.015), z + WALL / 2), rot))
            k.add(box(ln, 0.012, 0.05), "fb_stripe", T((xm, s * (wm + 0.003), z + WALL - 0.045), rot))
            k.add(box(ln, 0.05, 0.025), "fb_hull_dk", T((xm, s * (wm - 0.015), z + WALL + 0.012), rot))
    k.add(box(0.04, 2 * width(-L * 0.93) - 0.02, WALL + 0.02), "fb_hull_dk", T((-L * 0.93, 0, deck(0) + WALL / 2)))   # transom
    for x in (-0.5, 0.0):
        k.add(box(0.1, 2 * width(x) - 0.04, 0.03), "fb_deck", T((x, 0, deck(x) + 0.11)))                          # thwarts
    # A mast with a furled sail on its boom, and a lamp at the top.
    k.add(cyl(0.035, 0.028, 1.5, 8), "fb_mast", T((0.25, 0, 0.95)))
    k.add(cyl(0.022, 0.022, 0.95, 6), "fb_mast", T((-0.2, 0, 0.62), (0, 90, 0)))
    k.add(cyl(0.065, 0.05, 0.85, 8), "fb_sail", T((-0.2, 0, 0.68), (0, 90, 0), scale=(1.3, 1, 1)))
    for x in (-0.45, -0.15, 0.1):
        k.add(cyl(0.07, 0.07, 0.02, 8), "fb_rope", T((x, 0, 0.68), (0, 90, 0), scale=(1.3, 1, 1)))
    k.add(cyl(0.006, 0.006, 1.2, 4), "fb_rope", T((0.65, 0, 1.0), (0, -35, 0)))                  # forestay
    k.add(box(0.07, 0.07, 0.09, 0.01), "fb_lamp", T((0.25, 0, 1.68)))
    k.add(cyl(0.05, 0.0, 0.05, 6), "fb_hull_dk", T((0.25, 0, 1.75)))
    # The catch: a crate of fish and a coil of rope at the stern.
    k.add(box(0.28, 0.24, 0.14, 0.01), "fb_crate", T((-0.7, 0.09, 0.27)))
    for i in range(4):
        k.add(sphere(0.03, 6, 4), "fb_fish", T((-0.78 + i * 0.055, 0.05 + (i % 2) * 0.07, 0.35), scale=(2.2, 0.8, 0.6), rot=(0, 0, i * 25)))
    k.add(cyl(0.09, 0.09, 0.05, 10), "fb_rope", T((-0.68, -0.16, 0.225)))
    k.add(cyl(0.05, 0.05, 0.06, 8), "fb_hull_dk", T((-0.68, -0.16, 0.235)))


MODELS = {
    "critter_rabbit": build_rabbit, "critter_squirrel": build_squirrel, "critter_hedgehog": build_hedgehog,
    "critter_crab": build_crab, "critter_duck": build_duck, "critter_lizard": build_lizard, "critter_goat": build_goat,
    "critter_glowbeetle": build_glowbeetle, "critter_bird": build_bird, "fishing_boat": build_fishing_boat,
}


def main(argv):
    out = os.path.abspath(argv[argv.index("--out") + 1]) if "--out" in argv else os.path.join(HERE, "../../web/world/assets")
    only = set(argv[argv.index("--only") + 1].split(",")) if "--only" in argv else None
    scn = B.fresh_scene("Critters")
    with B.use_scene(scn):
        built = {}
        for i, (name, fn) in enumerate(MODELS.items()):
            if only and name not in only:
                continue
            coll = bpy.data.collections.new(B.PREFIX + name)
            scn.collection.children.link(coll)
            k = B.Kit(name)
            fn(k)
            ob = k.build(coll)
            ob.location = (i * 0.8 - 3.2 if name != "fishing_boat" else 0, 0 if name != "fishing_boat" else 1.6, 0)
            built[name] = [ob]
        bpy.context.view_layer.update()  # a re-run in a live Blender: drop the objects fresh_scene removed
        for name, t, size in B.export(built, out):
            assert t <= BUDGET.get(name, 1200), f"{name}: {t} tris over budget"
            assert size < 100 * 1024, f"{name}: {size} bytes"
    B.save_if_file()
    return built


if __name__ == "__main__":
    args = globals().get("ARGS")
    if args is None:
        args = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    main(args)
