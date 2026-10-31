"""Dressing props (#233): small things that make a place look lived in, exported as .glb. Three modules on the
1.3 kit (v13_kit.py), each with PALETTE, PROPS, FOOT, WIND and CLIPS:

    dressing_water.py    cliff_falls, water_lilies, well
    dressing_village.py  lantern_bunting, washing_line, flower_planter, vegetable_patch
    dressing_land.py     windmill, blossom_tree, leaf_litter

Headless only:
    blender -b --factory-startup -P tools/blender/build_dressing.py -- [--group water,village] [--only well] \
        [--out web/world/assets] [--sheets DIR] [--check]

  --group   water, village, land (comma separated) or all (default)
  --out     export each prop to <out>/<name>.glb (fails on > 3k triangles, > 2 MB, or clips other than CLIPS)
  --sheets  DIR/dressing_<group>_day.png and _night.png: every prop from seven angles (front, back, left,
            right, top, front 3/4, the game camera), toon-shaded like the game; look at both before shipping
  --check   z-fighting check (check_coplanar.py) on a y = 0 ground plane

Deterministic: every random choice is seeded per prop.
"""
import importlib
import math
import os
import sys

import bpy

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import v13_kit as K  # noqa: E402
from v13_kit import B  # noqa: E402

GROUPS = ("water", "village", "land")
LIGHT = dict(sun=("#fff4d6", 2.2), from_hero=(8, 18, 6), hemi=("#eaf4ff", "#3a5a2a", 1.5), bg="#9cc7e8", fog=(14, 30))
P = math.radians(50)   # the game camera's default pitch
VIEWS = (((0, -1, 0.12), "front"), ((0, 1, 0.12), "back"), ((-1, 0, 0.12), "left"), ((1, 0, 0.12), "right"),
         ((0, -0.3, 1), "top"), ((0.55, -0.75, 0.75), "front 3/4"), ((0, -math.cos(P), math.sin(P)), "game"))


def loop_part(build, pivot, axis, seconds, name):
    """after() hook: a separately built part parented at `pivot`, turning one full turn about its local `axis`
    ("X", "Y" or "Z") every `seconds`, as a clip named `Loop` (linear keys every quarter turn, seamless)."""
    def after(ob, coll):
        k = B.Kit(name)
        build(k)
        part = k.build(coll)
        part.parent = ob
        part.location = pivot
        part.rotation_mode = "XYZ"
        part.animation_data_create()
        act = bpy.data.actions.new("Loop")
        act.use_fake_user = True
        part.animation_data.action = act
        i = "XYZ".index(axis)
        fps = bpy.context.scene.render.fps
        for q in range(5):
            r = [0.0, 0.0, 0.0]
            r[i] = q * math.pi / 2
            part.rotation_euler = r
            part.keyframe_insert("rotation_euler", frame=1 + q * seconds * fps / 4)
        for fc in B.fcurves(act):
            for kp in fc.keyframe_points:
                kp.interpolation = "LINEAR"
        tr = part.animation_data.nla_tracks.new()
        tr.name = "Loop"
        tr.strips.new("Loop", 1, act)
        part.animation_data.action = None
        part.rotation_euler = (0, 0, 0)
        return [part]
    return after


def build_group(mod, group, only):
    scn = B.fresh_scene("Dressing_" + group)
    built, x = {}, 0.0
    with B.use_scene(scn):
        for name, fn in mod.PROPS.items():
            if only and name not in only:
                continue
            coll = bpy.data.collections.new(B.PREFIX + name)
            scn.collection.children.link(coll)
            kit = B.Kit(name)
            after = fn(kit)
            ob = kit.build(coll)
            if name in B.WIND:
                B.add_wind(ob, B.WIND[name])
            built[name] = [ob] + (after(ob, coll) if after else [])
            w = mod.FOOT.get(name, (1, 1))[0]
            x += w / 2
            ob.location = (x, 0, 0)
            coll.instance_offset = (x, 0, 0)
            x += w / 2 + 1.0
    return scn, built


def main(argv):
    arg = lambda k: argv[argv.index(k) + 1] if k in argv else None  # noqa: E731
    groups = arg("--group") or "all"
    groups = GROUPS if groups == "all" else groups.split(",")
    out, sheets, only = arg("--out"), arg("--sheets"), arg("--only")
    only = set(only.split(",")) if only else None
    out = os.path.abspath(out) if out else None
    report = []
    for group in groups:
        mod = importlib.import_module("dressing_" + group)
        B.PALETTE.update(mod.PALETTE)
        B.WIND.update(getattr(mod, "WIND", {}))
        B.ALPHA.update(getattr(mod, "ALPHA", {}))
        scn, built = build_group(mod, group, only)
        if not built:
            continue
        with B.use_scene(scn):
            stats = {}
            if out:
                for name, objs in built.items():
                    stats[name] = K.export(name, objs, out, clips=getattr(mod, "CLIPS", {}).get(name, []))
                    report.append((group, name, *stats[name]))
            if "--check" in argv:
                import check_coplanar
                g = B.Kit("dressing_check_ground")
                g.add(K.box(400, 40, 0.0001), "grass", K.T((150, 0, -0.00005)))
                g.build(scn.collection)
                check_coplanar.check()
                bpy.data.objects.remove(bpy.data.objects["dressing_check_ground"])
            if sheets:
                sheets = os.path.abspath(sheets)
                os.makedirs(sheets, exist_ok=True)
                info = {}
                for name, objs in built.items():
                    w, d = mod.FOOT.get(name, (1, 1))
                    t = stats[name][0] if name in stats else sum(B.tris(o) for o in objs if o.type == "MESH")
                    info[name] = f"{name}  {w}x{d}  {t} tris"
                for night in (False, True):
                    K.tile_sheet(scn, LIGHT, built, info, os.path.join(sheets, f"dressing_{group}_{'night' if night else 'day'}.png"),
                                 tile=320, cols=len(VIEWS), night=night, views=VIEWS)
    for group, name, t, size in report:
        print(f"[dressing] {group:7s} {name:16s} {t:5d} tris {size / 1024:7.1f} KB")


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
