"""Ansel's sheep for Thrumming Downs (1.3, WORLD_DESIGN §10.2): an NPC model on its own little rig of parts,
with the clips the game's actors play (Idle, Walk, Fidget). Object animation, no skin: a woolly body, a dark
face with floppy ears, four thin legs swinging in diagonal pairs.

Headless only:
    blender -b --factory-startup -P tools/blender/build_sheep.py -- --out web/world/assets/npc
"""
import math
import os
import random
import sys

import bpy

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import build_assets as B  # noqa: E402
from build_assets import T, box, cyl, ico, jitter, sphere  # noqa: E402

B.PALETTE.update({"wool": "#f1ead9", "wool_dk": "#ddd3be", "sheep_face": "#3b3532", "sheep_hoof": "#2a2522", "sheep_ear_in": "#c98f86"})
FPS = 24


def part(name, build, coll, parent=None, loc=(0, 0, 0)):
    k = B.Kit(name)
    build(k)
    ob = k.build(coll)
    ob.parent = parent
    ob.location = loc
    ob.rotation_mode = "XYZ"
    return ob


def body(k):
    rng = random.Random(901)
    k.add(jitter(ico(0.34, 2), rng, 0.02), "wool", T((0, 0, 0), scale=(0.95, 1.35, 0.9)), smooth=True)
    for i in range(14):  # the fleece: lumpy curls all over
        a, e = rng.uniform(0, math.tau), rng.uniform(-0.3, 1.1)
        x, y, z = 0.3 * math.cos(a) * math.cos(e), 0.42 * math.sin(a) * math.cos(e), 0.27 * math.sin(e)
        k.add(jitter(ico(rng.uniform(0.1, 0.15), 1), rng, 0.02), "wool" if i % 3 else "wool_dk", T((x, y, z)), smooth=True)
    k.add(jitter(ico(0.07, 1), rng, 0.01), "wool_dk", T((0, 0.48, 0.08)), smooth=True)   # tail (back is +Y)


def head(k):
    k.add(sphere(0.14, 10, 8), "sheep_face", T((0, -0.1, 0), scale=(0.85, 1.25, 0.95)), smooth=True)
    k.add(jitter(ico(0.12, 1), random.Random(902), 0.015), "wool", T((0, 0.02, 0.1)), smooth=True)   # topknot
    for sx in (-1, 1):
        k.add(box(0.05, 0.14, 0.025, 0.01), "sheep_face", T((sx * 0.14, -0.02, 0.03), (0, sx * 20, sx * 30)))
        k.add(box(0.03, 0.09, 0.01), "sheep_ear_in", T((sx * 0.145, -0.025, 0.045), (0, sx * 20, sx * 30)))
        k.add(sphere(0.022, 8, 6), "sheep_hoof", T((sx * 0.07, -0.2, 0.05)), smooth=True)              # eyes


def leg(k):
    k.add(cyl(0.035, 0.03, 0.34, 6), "sheep_face", T((0, 0, -0.17)))
    k.add(cyl(0.04, 0.04, 0.05, 6), "sheep_hoof", T((0, 0, -0.34)))


def keyframes(ob, track, keys, path="rotation_euler"):
    """keys: [(frame, value tuple)]; adds an NLA track named `track` (same names merge into one clip)."""
    ob.animation_data_create()
    act = bpy.data.actions.new(f"{track}_{ob.name}")
    act.use_fake_user = True
    ob.animation_data.action = act
    rest = tuple(getattr(ob, path))
    for f, v in keys:
        setattr(ob, path, [math.radians(a) for a in v] if path == "rotation_euler" else v)
        ob.keyframe_insert(path, frame=f)
    tr = ob.animation_data.nla_tracks.new()
    tr.name = track
    tr.strips.new(track, 0, act)
    ob.animation_data.action = None
    setattr(ob, path, rest)


def main(argv):
    out = os.path.abspath(argv[argv.index("--out") + 1]) if "--out" in argv else os.path.join(HERE, "../../web/world/assets/npc")
    scn = B.fresh_scene("SHEEP")
    with B.use_scene(scn):
        coll = bpy.data.collections.new("sheep")
        scn.collection.children.link(coll)
        root = bpy.data.objects.new("npc_sheep", None)
        coll.objects.link(root)
        trunk = part("sheep_body", body, coll, root, (0, 0, 0.62))
        skull = part("sheep_head", head, coll, trunk, (0, -0.46, 0.14))
        legs = [part(f"sheep_leg{i}", leg, coll, trunk, (sx * 0.17, sy * 0.28, -0.2)) for i, (sx, sy) in enumerate(((-1, -1), (1, -1), (-1, 1), (1, 1)))]
        # Idle (2 s): breathing, the head dips to graze and comes back up.
        keyframes(trunk, "Idle", [(0, (0, 0, 0)), (24, (1.5, 0, 0)), (48, (0, 0, 0))])
        keyframes(skull, "Idle", [(0, (0, 0, 0)), (12, (28, 0, 0)), (30, (30, 0, 4)), (40, (0, 0, 0)), (48, (0, 0, 0))])
        for l in legs:
            keyframes(l, "Idle", [(0, (0, 0, 0)), (48, (0, 0, 0))])
        # Walk (0.5 s, one step per 0.25 s tile): diagonal pairs swing, the body rocks.
        for i, l in enumerate(legs):
            s = 1 if i in (0, 3) else -1
            keyframes(l, "Walk", [(0, (s * 28, 0, 0)), (6, (-s * 28, 0, 0)), (12, (s * 28, 0, 0))])
        keyframes(trunk, "Walk", [(0, (0, 4, 0)), (3, (0, 0, 0)), (6, (0, -4, 0)), (9, (0, 0, 0)), (12, (0, 4, 0))])
        keyframes(skull, "Walk", [(0, (-6, 0, 0)), (6, (4, 0, 0)), (12, (-6, 0, 0))])
        # Fidget (1.5 s): an ear-flapping head shake.
        keyframes(skull, "Fidget", [(0, (0, 0, 0)), (6, (0, 0, 25)), (12, (0, 0, -25)), (18, (0, 0, 20)), (24, (0, 0, -10)), (36, (0, 0, 0))])
        for o in [trunk] + legs:
            keyframes(o, "Fidget", [(0, tuple(math.degrees(a) for a in o.rotation_euler)), (36, tuple(math.degrees(a) for a in o.rotation_euler))])
        scn.render.fps = FPS
        objs = [root, trunk, skull] + legs
        for o in bpy.context.view_layer.objects:
            o.select_set(False)
        for o in objs:
            o.select_set(True)
        bpy.context.view_layer.objects.active = root
        os.makedirs(out, exist_ok=True)
        path = os.path.join(out, "npc_sheep.glb")
        bpy.ops.export_scene.gltf(filepath=path, export_format="GLB", use_selection=True, use_active_scene=True, export_yup=True,
                                  export_apply=True, export_animations=True, export_animation_mode="NLA_TRACKS",
                                  export_vertex_color="NONE", export_skins=False, export_cameras=False, export_lights=False)
        tris = sum(B.tris(o) for o in objs if o.type == "MESH")
        print(f"[sheep] npc_sheep.glb {tris} tris {os.path.getsize(path) / 1024:.1f} KB")
        assert tris <= 6000


main(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
