"""Cave light: bioluminescent mushrooms for caves, mines and wild places at night (docs/WORLD_RULES.md §2: nothing
underground could power a street lamp). Shared by every region: the caps and gills are one `mushroom_glow`
material, near white, that a map tints per prop with `glow` (teal in the Saltreach caves, violet in the Hushlands,
amber in the Stonebacks), and build.js gives each a glimmer (a small tinted halo and pool) after dark and in caves.

  glow_mushroom       a low clump: five caps on a mossy stone, ~0.35 m (paths and junctions)
  glow_mushroom_tall  three tall stems over two small ones, ~0.9 m (landmarks, a turn you shouldn't miss)
  glow_shelf          bracket fungus stepping up a broken stump, ~0.6 m (walls, timber, dead wood)

All 1 x 1, solid, static (instanced), well under 1k triangles each.
"""
import math
import random

from v13_kit import B, T, contact, cyl, lathe, rock  # noqa: F401

PALETTE = {
    "mushroom_glow": "#eafff8",           # tinted per prop (`glow`); near white so any hue reads
    "cv_stem": "#e8e0cf", "cv_stem_dk": "#c9bda5",
    "cv_moss": "#3f6b3a", "cv_moss_dk": "#2e5230",
    "cv_stone": "#6f6c66", "cv_stone_dk": "#55524d",
    "cv_wood": "#5d4632", "cv_wood_dk": "#46331f", "cv_heart": "#8a6a48",
}
FOOT = {}
LIGHT = dict(sun=("#cfd8ff", 1.2), from_hero=(8, 18, 6), hemi=("#a8b4d8", "#1a1c26", 1.2), bg="#2b2f45", fog=(12, 28))


def cap(k, rng, x, y, z, r, h, tilt=8.0):
    """One mushroom cap: a domed top and its gills underneath, both glowing; tilted a little at random."""
    rot = (rng.uniform(-tilt, tilt), rng.uniform(-tilt, tilt), 0)
    m = T((x, y, z), rot)
    k.add(lathe([(r, 0.0), (r * 0.92, h * 0.45), (r * 0.55, h * 0.9), (0.0, h)], 8, rng.uniform(0, 45)), "mushroom_glow", m, smooth=True)
    # the gills: a shallow cone under the rim, so the underside glows too (seen from the 50 degree camera)
    k.add(lathe([(r * 0.18, -h * 0.28), (r * 0.97, -0.004)], 8), "mushroom_glow", m)


def stem(k, rng, x, y, h, r):
    lean = (rng.uniform(-6, 6), rng.uniform(-6, 6), 0)
    k.add(cyl(r * 1.25, r, h, 6), rng.choice(["cv_stem", "cv_stem", "cv_stem_dk"]), T((x, y, h / 2), lean))


def build_glow_mushroom(k):
    """A low clump on a mossy stone."""
    rng = random.Random(901)
    k.add(rock(rng, 0.2, 1, 0.1), "cv_stone", T((0.08, -0.06, 0.05), scale=(1.2, 1.0, 0.45)))
    k.add(cyl(0.3, 0.26, 0.04, 10), "cv_moss", T((0, 0, 0.02)))
    for x, y, h, r in [(-0.14, 0.08, 0.2, 0.09), (0.0, 0.16, 0.14, 0.07), (0.15, 0.1, 0.24, 0.1), (-0.05, -0.12, 0.11, 0.06), (0.2, -0.1, 0.16, 0.07)]:
        stem(k, rng, x, y, h, r * 0.24)
        cap(k, rng, x, y, h, r, r * 0.75)
    contact(k, 0.36)


def build_glow_mushroom_tall(k):
    """Three tall stems over two small ones: a beacon you see from a screen away."""
    rng = random.Random(902)
    k.add(cyl(0.34, 0.3, 0.05, 10), "cv_moss_dk", T((0, 0, 0.025)))
    k.add(rock(rng, 0.16, 1, 0.12), "cv_stone_dk", T((-0.18, -0.12, 0.05), scale=(1.0, 1.1, 0.6)))
    for x, y, h, r in [(-0.08, 0.04, 0.78, 0.17), (0.14, -0.04, 0.6, 0.14), (0.02, 0.18, 0.44, 0.12), (0.22, 0.17, 0.18, 0.07), (-0.2, 0.16, 0.14, 0.06)]:
        stem(k, rng, x, y, h, max(0.022, r * 0.2))
        cap(k, rng, x, y, h, r, r * 0.8, tilt=10)
    contact(k, 0.4)


def build_glow_shelf(k):
    """Bracket fungus stepping up a broken stump (or a mine's rotten prop)."""
    rng = random.Random(903)
    k.add(cyl(0.2, 0.17, 0.56, 8), "cv_wood", T((0, -0.04, 0.28)))
    # the broken top: a jagged cone sunk into the trunk (its flat base hidden inside), the pale heart showing
    k.add(lathe([(0.168, -0.02), (0.12, 0.05), (0.06, 0.02), (0.0, 0.06)], 8, 20), "cv_wood_dk", T((0, -0.04, 0.56)))
    k.add(lathe([(0.07, -0.01), (0.0, 0.035)], 8, 10), "cv_heart", T((0, -0.04, 0.57)))
    for a, z, r in [(-60, 0.16, 0.13), (0, 0.3, 0.15), (55, 0.44, 0.12), (130, 0.24, 0.1), (-150, 0.38, 0.09)]:
        rad = math.radians(a)
        px, py = 0.17 * math.sin(rad), -0.04 + 0.17 * math.cos(rad)
        # a half disc sticking out of the bark, glowing top and gills
        m = T((px, py, z), (rng.uniform(-6, 6), 0, -a))
        k.add(lathe([(r, 0.0), (r * 0.7, r * 0.22), (0.0, r * 0.3)], 8), "mushroom_glow", m @ T((0, r * 0.05, 0), scale=(1.0, 0.75, 1.0)), smooth=True)
        k.add(lathe([(r * 0.2, -r * 0.18), (r * 0.95, -0.003)], 8), "mushroom_glow", m @ T((0, r * 0.05, 0), scale=(1.0, 0.75, 1.0)))
    k.add(cyl(0.3, 0.27, 0.03, 10), "cv_moss", T((0, 0, 0.015)))
    contact(k, 0.34)


PROPS = {"glow_mushroom": build_glow_mushroom, "glow_mushroom_tall": build_glow_mushroom_tall, "glow_shelf": build_glow_shelf}
