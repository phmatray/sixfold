"""Dressing props, land (#233): a windmill for the skyline, a tree in blossom, leaves for the ground to walk over.
Built by build_dressing.py (`--group land`). Glow part: dl_window_glow (the mill's windows).
"""
import math
import random

import bmesh

import build_dressing as D
from v13_kit import (T, along, arch_pts, beam, box, clamp01, contact, cyl, ico, jitter, lathe, prism, ribbon,  # noqa: F401
                     tube, FRONT)

PALETTE = {
    # the mill: whitewash on a stone foot, a shingled timber cap, painted teal woodwork
    "dl_wash": "#f2ecdc", "dl_wash_dk": "#ddd2bb",
    "dl_stone": "#aaa08e", "dl_stone_dk": "#8a806f", "dl_stone_lt": "#c6bca8",
    "dl_cap": "#a9573b", "dl_cap_dk": "#86412c", "dl_cap_lt": "#c46d4a",
    "dl_wood": "#9a6a42", "dl_wood_dk": "#6c4a2e", "dl_wood_lt": "#c29462",
    "dl_door": "#3f7d78", "dl_door_dk": "#2f605c", "dl_iron": "#3a383d", "dl_hole": "#2a221d",
    "dl_cloth": "#f4e9cf", "dl_window_glow": "#f6cf7c",
    "dl_sack": "#d9c290", "dl_sack_dk": "#b79c68", "dl_flower": "#e0544a", "dl_flower_b": "#f2c94c", "dl_green": "#5f9a3e",
    # the blossom tree
    "dl_bark": "#6e4c40", "dl_bark_dk": "#563a31", "dl_bark_lt": "#8a6452",
    "dl_bloom_lt": "#fdeef2", "dl_bloom": "#f7d1dc", "dl_bloom_md": "#efb2c6", "dl_bloom_dk": "#d9849f",
    # leaf litter
    "dl_ochre": "#e6a41c", "dl_orange": "#e8701a", "dl_rust": "#c94c1e", "dl_red": "#a02c1f", "dl_brown": "#8d5530",
    "dl_twig": "#6b4a32", "dl_pebble": "#aaa396", "dl_pebble_dk": "#8d877b", "dl_pebble_lt": "#c4beb1",
}

C8 = math.cos(math.radians(22.5))   # octagon apothem / radius
S8 = math.sin(math.radians(22.5))


def face_m(theta, z, prof, out=0.0):
    """A matrix on the face of an octagonal frustum (lathe, 8 segments, phase 22.5) whose outward normal points at
    `theta` degrees (270 = front, -Y), at height z, `out` metres off it: local X along the face, local Z up the
    (leaning) face, local -Y outward. prof = (r0, z0, r1, z1), the frustum's corner radii."""
    r0, z0, r1, z1 = prof
    lean = math.degrees(math.atan((r0 - r1) * C8 / (z1 - z0)))
    a = (r0 + (r1 - r0) * (z - z0) / (z1 - z0)) * C8
    return T(rot=(0, 0, theta - 270)) @ T((0, -a - out, z), (-lean, 0, 0))


def half_w(prof, z):
    r0, z0, r1, z1 = prof
    return (r0 + (r1 - r0) * (z - z0) / (z1 - z0)) * S8


# ---------------------------------------------------------------- windmill

PLINTH = (1.22, -0.02, 1.19, 0.34)
TOWER = (1.12, 0.30, 0.80, 5.30)
HUB = (0, -1.29, 5.72)   # the sails' pivot, ahead of the cap's front


def build_sails(k):
    """Four lattice sails round a hub, in the XZ plane (they turn about Y): a whip, three hemlaths and eight sail
    bars each, a cloth spread on the inner two thirds behind the lattice. Rest pose: a + (bars square to the axes, so
    no two parallel bars share a bounding box). Depths (Y): whip +-0.045, hemlaths +-0.025, bars +-0.012, cloth behind.
    S scales the sails in their plane only (3.0 m from the hub to the tips), so those depth gaps hold."""
    S = 1.32
    k.add(cyl(0.22, 0.22, 0.22, 8), "dl_wood_dk", T(rot=(90, 0, 0)))
    k.add(cyl(0.17, 0.06, 0.13, 8), "dl_iron", T((0, -0.145, 0), (90, 0, 0)))
    for i in range(4):
        R = T(rot=(0, 90 * i, 0))   # the arm runs along local +X, its lattice toward local +Z
        k.add(box(2.15 * S, 0.09, 0.09 * S), "dl_wood_dk", R @ T((0.12 + 2.15 * S / 2, 0, 0)))
        for v in (-0.1, 0.16, 0.42):
            k.add(box(1.7 * S, 0.05, 0.04 * S), "dl_wood", R @ T((1.4 * S, 0, v * S)))
        for j in range(8):
            k.add(box(0.035 * S, 0.024, 0.52 * S), "dl_wood", R @ T(((0.6 + j * 1.6 / 7) * S, 0, 0.16 * S)))
        k.add(box(1.2 * S, 0.012, 0.5 * S), "dl_cloth", R @ T((1.22 * S, 0.068, 0.17 * S)))


def window(k, theta, z, w=0.36, h=0.56, box_under=False):
    F = face_m(theta, z, TOWER)
    # faces off the wall (out): pane -0.025..0.012, frame -0.045..0.04, sill -0.03..0.11
    k.add(prism(arch_pts(w, h, 6), 0.037), "dl_window_glow", F @ T((0, 0.0065, 0)))
    rim = arch_pts(w + 0.05, h + 0.025, 6)   # its inner edge 1.5 cm over the pane, its legs 2 cm into the sill
    rim[0], rim[-1] = (rim[0][0], -0.06), (rim[-1][0], -0.06)
    k.add(ribbon(rim, 0.08, 0.085), "dl_door", F @ T((0, 0.045, 0), FRONT))
    k.add(box(w + 0.16, 0.14, 0.05), "dl_stone_lt", F @ T((0, -0.04, -0.065)))
    if box_under:  # a flower box under the sill
        k.add(box(w + 0.1, 0.14, 0.12), "dl_wood", F @ T((0, -0.05, -0.175)))
        rng = random.Random(int(z * 100))
        for i in range(6):
            x = -w / 2 + (i + 0.5) * w / 6
            k.add(jitter(ico(0.05, 1), rng, 0.008), "dl_green" if i % 2 else "dl_flower" if i % 4 == 0 else "dl_flower_b",
                  F @ T((x, -0.05 + rng.uniform(-0.03, 0.03), -0.095 + rng.uniform(0, 0.02))), smooth=True)


def build_windmill(k):
    """3 x 3 tower mill (~8 m to the sail tips): a tapered whitewashed octagon on a stone foot, a shingled
    cap, a teal door up a stone step, lit windows, sacks and a spare millstone by the door, a tail pole down
    the back to a post; the lattice sails turn on the front (`sails`, Loop)."""
    rng = random.Random(811)
    # stone foot, its blocks proud of the faces (not on the front, where the door and step are)
    k.add(lathe([(PLINTH[0], PLINTH[1]), (PLINTH[2], PLINTH[3])], 8, 22.5), "dl_stone_dk")
    for j in range(1, 8):
        th = 270 + 45 * j
        for row, z in enumerate((0.10, 0.26)):
            hw = half_w(PLINTH, z) - 0.1
            x, n = -hw + rng.uniform(0, 0.06), 0
            while x < hw - 0.1:   # neighbours 3 cm up/down from each other: no shared top or bottom planes
                w = min(rng.uniform(0.2, 0.32), hw - x)
                k.add(box(w - 0.04, 0.05, 0.10), "dl_stone" if rng.random() < 0.6 else "dl_stone_lt",
                      face_m(th, z + (0.015 if (n + row + j) % 2 else -0.015), PLINTH) @ T((x + w / 2, 0, 0)))
                x, n = x + w, n + 1
    # the tower, a timber curb on top, and a few stones showing through the whitewash
    k.add(lathe([(TOWER[0], TOWER[1]), (TOWER[2], TOWER[3])], 8, 22.5), "dl_wash")
    k.add(lathe([(0.87, 5.22), (0.87, 5.36)], 8, 22.5), "dl_wood_dk")
    for th, z0 in ((225, 1.4), (315, 4.3), (0, 2.6), (180, 3.4), (90, 1.6)):
        for i in range(3):
            z = z0 + 0.14 * (i % 2)
            k.add(box(rng.uniform(0.13, 0.17), 0.04, rng.uniform(0.08, 0.1)), "dl_stone",
                  face_m(th, z, TOWER, 0.0) @ T(((i - 1) * 0.24 + rng.uniform(-0.03, 0.03), 0, 0), (0, rng.uniform(-6, 6), 0)))
    # the cap: three shingle courses, each lapping over the one below, and a knob
    for prof, m in (([(0.97, 5.33), (0.72, 5.86)], "dl_cap"), ([(0.80, 5.78), (0.50, 6.28)], "dl_cap_dk"),
                    ([(0.58, 6.20), (0.24, 6.62), (0.0, 6.74)], "dl_cap")):
        k.add(lathe(prof, 8, 22.5), m)
    k.add(cyl(0.035, 0.035, 0.16, 6), "dl_wood_dk", T((0, 0, 6.78)))
    k.add(lathe([(0, 6.80), (0.055, 6.82), (0.075, 6.88), (0.055, 6.94), (0, 6.96)], 8), "dl_wood_dk")   # (a UV sphere exports its triangles in a varying order)
    # the windshaft out of the cap's front to the hub
    k.add(cyl(0.09, 0.09, 0.8, 8), "dl_iron", T((0, -0.95, HUB[2]), (90, 0, 0)))
    # door: an arched leaf with two iron straps in a stone surround, on the foot's ledge
    F = face_m(270, 0.30, TOWER)
    k.add(prism(arch_pts(0.645, 1.695, 8), 0.07), "dl_door", F @ T((0, -0.005, 0)))
    for h in (0.45, 1.2):
        k.add(box(0.5, 0.04, 0.045), "dl_iron", F @ T((0, -0.04, h)))
    k.add(ico(0.03, 2), "dl_iron", F @ T((0.2, -0.07, 0.82)))
    rim = arch_pts(0.71, 1.725, 8)   # legs 4 cm into the foot, below the leaf's sill
    rim[0], rim[-1] = (rim[0][0], -0.04), (rim[-1][0], -0.04)
    k.add(ribbon(rim, 0.09, 0.115), "dl_stone_lt", F @ T((0, 0.045, 0), FRONT))
    k.add(box(0.13, 0.09, 0.14), "dl_stone", F @ T((0, -0.06, 1.74)))   # keystone
    k.add(box(1.0, 0.36, 0.19), "dl_stone_lt", T((0, -1.27, 0.06)))  # the step
    # windows (the lower front one with a flower box)
    window(k, 270, 2.5, box_under=True)
    window(k, 270, 3.95, 0.3, 0.48)
    window(k, 315, 3.0, 0.3, 0.48)
    window(k, 135, 2.8, 0.3, 0.48)   # back-left, clear of the tail pole
    # flour sacks by the door
    for (x, y, z), s, rot, m in (((-0.78, -1.1, 0.2), (1.0, 0.85, 1.2), (0, 0, 20), "dl_sack"),
                                 ((-1.0, -0.82, 0.17), (0.9, 0.8, 1.05), (8, -10, 50), "dl_sack_dk")):
        k.add(jitter(ico(0.2, 1), rng, 0.02), m, T((x, y, z), rot, s), smooth=True)
        k.add(cyl(0.05, 0.07, 0.08, 6), m, T((x, y, z), rot) @ T((0, 0, 0.24 * s[2])))
        k.add(cyl(0.055, 0.055, 0.025, 6), "dl_wood_dk", T((x, y, z), rot) @ T((0, 0, 0.24 * s[2])))
    # a spare millstone leaning on the foot
    n = (math.cos(math.radians(315)) * math.cos(math.radians(8)), math.sin(math.radians(315)) * math.cos(math.radians(8)),
         math.sin(math.radians(8)))
    c = (0.88, -0.88, 0.34)
    k.add(cyl(0.36, 0.36, 0.12, 12), "dl_stone_lt", along(c, n))
    k.add(cyl(0.07, 0.07, 0.15, 8), "dl_hole", along(c, n))
    # the tail pole down the back to a post
    beam(k, (0, 0.92, 5.38), (0, 1.40, 0.6), 0.11, 0.11, "dl_wood")
    k.add(cyl(0.075, 0.07, 0.8, 8), "dl_wood_dk", T((0, 1.36, 0.38)))
    k.add(cyl(0.085, 0.085, 0.09, 8), "dl_wood_lt", T((0, 1.36, 0.6)))   # a coil of rope round the post
    contact(k, 1.45)
    return D.loop_part(build_sails, HUB, "Y", 14, "sails")


# ---------------------------------------------------------------- blossom tree


def build_blossom_tree(k):
    """2 x 2 tree in blossom (~3.6 m): a gnarled trunk forking into three limbs, two of them forking again, under
    clustered blobs of pink and white blossom (white on top, deeper pink beneath), a birdhouse on the trunk and
    fallen petals on the ground."""
    rng = random.Random(733)
    trunk = [(0, 0, -0.03), (0.06, 0.02, 0.4), (-0.03, 0.07, 0.8), (0.04, 0.03, 1.15)]
    k.add(jitter(tube(trunk, 0.19, 7, taper=0.72), rng, 0.015), "dl_bark", smooth=True)
    for a in (15, 100, 190, 275):   # root flares, mostly buried
        k.add(cyl(0.1, 0.035, 0.36, 5), "dl_bark_dk", T((0.15 * math.cos(math.radians(a)), 0.15 * math.sin(math.radians(a)), 0.05), (0, 70, a)), smooth=True)
    fork = (0.04, 0.03, 1.1)
    for mid, tip, sub in (((0.28, -0.16, 1.55), (0.5, -0.36, 2.0), (0.62, 0.08, 2.05)),
                          ((-0.26, 0.1, 1.6), (-0.55, 0.26, 2.05), (-0.45, -0.34, 1.95)),
                          ((0.1, 0.3, 1.75), (0.12, 0.5, 2.35), None)):
        k.add(tube([fork, mid, tip], 0.12, 6, taper=0.45), "dl_bark", smooth=True)
        if sub:
            k.add(tube([mid, ((mid[0] + sub[0]) / 2, (mid[1] + sub[1]) / 2, (mid[2] + sub[2]) / 2 - 0.05), sub], 0.06, 5, taper=0.5),
                  "dl_bark_lt", smooth=True)
    # blossom: a full dome of clustered blobs, white on top, pink round the middle, deeper pink under the rim
    blobs = [((0.02, 0.05, 2.7), 0.62, "dl_bloom"), ((0.04, 0.0, 3.2), 0.36, "dl_bloom_lt")]
    for i in range(7):
        a = i / 7 * math.tau + rng.uniform(-0.15, 0.15)
        blobs.append(((0.56 * math.cos(a), 0.56 * math.sin(a), 2.25 + rng.uniform(-0.08, 0.1)), rng.uniform(0.38, 0.44),
                      ("dl_bloom_md", "dl_bloom", "dl_bloom_lt")[i % 3] if i % 3 else "dl_bloom_md"))
    for i in range(5):
        a = i / 5 * math.tau + 0.5 + rng.uniform(-0.15, 0.15)
        blobs.append(((0.4 * math.cos(a), 0.4 * math.sin(a), 2.88 + rng.uniform(-0.06, 0.08)), rng.uniform(0.32, 0.38), "dl_bloom" if i % 2 else "dl_bloom_lt"))
    for i in range(6):
        a = (i + 0.5) / 6 * math.tau + rng.uniform(-0.2, 0.2)
        blobs.append(((0.62 * math.cos(a), 0.62 * math.sin(a), 1.98 + rng.uniform(0, 0.1)), rng.uniform(0.2, 0.25), "dl_bloom_dk"))
    for (x, y, z), r, m in blobs:
        rc = math.hypot(x, y)
        if rc + r > 0.98:   # keep the crown inside the 2 x 2 footprint
            x, y = x * (0.98 - r) / rc, y * (0.98 - r) / rc
        k.add(jitter(ico(r, 2), rng, r * 0.08), m, T((x, y, z), scale=(1, 1, 0.8)), smooth=True)
    # a birdhouse on the trunk, facing front
    bh = T((-0.02, -0.15, 0.92))
    k.add(box(0.16, 0.14, 0.2), "dl_wood_lt", bh)
    k.add(prism([(-0.115, 0), (0.115, 0), (0, 0.09)], 0.2), "dl_cap", bh @ T((0, -0.005, 0.07)))
    k.add(cyl(0.03, 0.03, 0.04, 8), "dl_hole", bh @ T((0, -0.065, 0.03), (90, 0, 0)))
    k.add(cyl(0.008, 0.008, 0.08, 5), "dl_wood_dk", bh @ T((0, -0.09, -0.04), (90, 0, 0)))
    # fallen petals: tiny tilted discs, none touching another, at staggered heights just off the ground
    pts = []
    while len(pts) < 30:
        a, rr = rng.uniform(0, math.tau), 0.25 + 0.7 * math.sqrt(rng.random())
        p = (rr * math.cos(a), rr * math.sin(a))
        if all(math.hypot(p[0] - q[0], p[1] - q[1]) > 0.1 for q in pts):
            pts.append(p)
    for i, (x, y) in enumerate(pts):
        r = rng.uniform(0.028, 0.04)
        k.add(cyl(r, r * 0.8, 0.003, 5), ("dl_bloom_lt", "dl_bloom", "dl_bloom_md")[i % 3],
              T((x, y, 0.006 + 0.003 * i / 29), (rng.choice((-1, 1)) * rng.uniform(3, 6), rng.uniform(-3, 3), rng.uniform(0, 360))))
    contact(k, 0.85)


# ---------------------------------------------------------------- leaf litter


def leaf(L, w, h):
    """A closed bent leaf along +X from its stalk (0) to its tip (L): a flat diamond underneath, the midrib raised
    `h`, the tip curling up."""
    bm = bmesh.new()
    V = lambda *p: bm.verts.new(p)  # noqa: E731
    base, tip = V(0, 0, 0), V(L, 0, h * 0.8)
    left, right = V(L * 0.42, w / 2, 0), V(L * 0.42, -w / 2, 0)
    m1, m2 = V(L * 0.25, 0, h), V(L * 0.62, 0, h)
    for f in ((base, m1, left), (m1, m2, left), (m2, tip, left), (base, right, m1), (m1, right, m2), (m2, right, tip),
              (base, left, tip), (base, tip, right)):
        bm.faces.new(f)
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return bm


def build_leaf_litter(k):
    """1 x 1 ground scatter to walk over: a loose drift of 36 fallen leaves in warm autumn tones (ochre, burnt orange,
    rust, a little dark red and brown), heaped toward the middle so it reads from the game camera, three twigs and
    five pebbles, all under 6 cm."""
    rng = random.Random(907)
    cols = ["dl_ochre"] * 6 + ["dl_orange"] * 5 + ["dl_rust"] * 5 + ["dl_red"] * 2 + ["dl_brown"] * 2
    leaves = []
    while len(leaves) < 36:   # clustered round the middle, a little longer across than deep
        L = rng.uniform(0.12, 0.17)
        cx, cy = rng.gauss(0, 0.17) * 1.15, rng.gauss(0, 0.17) * 0.85
        if max(abs(cx), abs(cy)) + 0.6 * L <= 0.48:
            leaves.append((L, cx, cy))
    # lowest points staggered 4 to 26 mm, the middle of the drift on top
    order = sorted(range(len(leaves)), key=lambda i: math.hypot(leaves[i][1], leaves[i][2]) + rng.uniform(0, 0.08), reverse=True)
    heights = {i: 0.004 + 0.022 * n / (len(leaves) - 1) for n, i in enumerate(order)}
    placed = []   # (unit normal, plane offset, bbox) of every leaf face so far

    def planes(bm):
        out = []
        for f in bm.faces:
            f.normal_update()
            n = f.normal.copy()
            if n.length > 0.5:
                vs = [v.co for v in f.verts]
                out.append((n, n.dot(f.calc_center_median()), [min(v[a] for v in vs) for a in range(3)], [max(v[a] for v in vs) for a in range(3)]))
        return out

    def clash(a, b):   # check_coplanar's test: parallel (either way) within 1 cm, boxes overlapping
        (na, da, la, ha), (nb, db, lb, hb) = a, b
        s = 1 if na.dot(nb) > 0 else -1
        if (na - nb * s).length > 0.03 or abs(da - db * s) >= 0.012:
            return False
        return all(min(ha[x], hb[x]) - max(la[x], lb[x]) > 0.004 for x in range(3) if x != max(range(3), key=lambda q: abs(na[q])))

    for i, (L, cx, cy) in enumerate(leaves):
        w = L * rng.uniform(0.45, 0.65)
        yaw = rng.uniform(0, 360)
        ca, sa = math.cos(math.radians(yaw)), math.sin(math.radians(yaw))
        h = rng.uniform(0.006, 0.01)
        for _ in range(40):   # re-roll the tilt until no face lies along one of an overlapping leaf's
            bm = leaf(L, w, h)
            M = T((cx - ca * L / 2, cy - sa * L / 2, 0), (rng.choice((-1, 1)) * rng.uniform(3, 8), rng.choice((-1, 1)) * rng.uniform(3, 7), yaw))
            bmesh.ops.transform(bm, matrix=M, verts=bm.verts)
            lo = min(v.co.z for v in bm.verts)
            bmesh.ops.translate(bm, vec=(0, 0, heights[i] - lo), verts=bm.verts)
            mine = planes(bm)
            if not any(clash(a, b) for a in mine for b in placed):
                break
            bm.free()
        else:
            raise RuntimeError(f"leaf_litter: leaf {i} keeps lying along another")
        placed += mine
        k.add(bm, rng.choice(cols))
    for pts, r in (([(-0.36, 0.22), (-0.24, 0.26), (-0.1, 0.24)], 0.01),
                   ([(0.14, -0.3), (0.24, -0.24), (0.36, -0.26)], 0.008),
                   ([(-0.28, -0.14), (-0.23, -0.06), (-0.2, 0.02)], 0.007)):
        k.add(tube([(x, y, r * 0.8 + 0.004 + 0.002 * j) for j, (x, y) in enumerate(pts)], r, 4, taper=0.6), "dl_twig")
    for (x, y), r, m in (((0.3, 0.3), 0.04, "dl_pebble"), ((0.36, 0.18), 0.025, "dl_pebble_dk"), ((-0.35, -0.36), 0.035, "dl_pebble_lt"),
                         ((0.05, -0.05), 0.03, "dl_pebble_dk"), ((-0.08, 0.4), 0.028, "dl_pebble")):
        k.add(jitter(ico(r, 1), rng, r * 0.15), m, T((x, y, r * 0.15), scale=(1.2, 1, 0.6)))


PROPS = {"windmill": build_windmill, "blossom_tree": build_blossom_tree, "leaf_litter": build_leaf_litter}
FOOT = {"windmill": (3, 3), "blossom_tree": (2, 2), "leaf_litter": (1, 1)}
WIND = {"blossom_tree": lambda x, y, z, r: clamp01((z - 1.3) / 2.2 * 0.7 + r / 1.0 * 0.3) if z > 1.3 else 0.0}
CLIPS = {"windmill": ["Loop"]}
