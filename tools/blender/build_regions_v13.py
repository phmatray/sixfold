"""Region props for Sixfold 1.3 "Six regions", exported as .glb. One module per region (v13_<region>.py), the
shared helpers and review renders in v13_kit.py. Contract: web/world/ASSETS.md; what each prop is for and how a
map uses it: web/world/ASSETS_V13.md.

Headless only:
    blender -b --factory-startup -P tools/blender/build_regions_v13.py -- --region ember,storm \
        [--out web/world/assets] [--only kiln,ash_pine] [--sheets DIR] [--check] [--assets-extra DIR]

  --region  ember, storm, tide, dream, grove, stone (comma separated) or all
  --out     export each prop to <out>/<name>.glb (fails on > 3k triangles or > 2 MB, or a stray clip)
  --sheets  DIR/<region>_tiles.png (every prop, front and back 3/4, labelled with footprint and triangles) and
            DIR/<region>_day.png, DIR/<region>_night.png: a small map of the region built from the exported
            files like build.js builds one (cliff stacks, auto-tiled walls, ring trees), at the game camera,
            with toon lighting from the region's light block (night: the 1.3 night preset, glow parts lit)
  --check   z-fighting check (check_coplanar.py) over the new props on a y = 0 ground plane
  --assets-extra  more .glb folders the sheets may borrow from (the 1.3 shared kit before it is merged)
  --textures      folder of not-yet-converted ground PNGs for the sheets (bake_ground_textures.py output)

Deterministic: every random choice is seeded per prop. It only ever writes the files its regions list.
"""
import importlib
import os
import sys

import bpy

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import v13_kit as K  # noqa: E402
from v13_kit import B  # noqa: E402

REGIONS = ("ember", "storm", "tide", "dream", "grove", "stone")
SEAL = {"ember": "#e1572f", "storm": "#e2ad19", "tide": "#2f7fd0", "dream": "#8a4fc9", "grove": "#3f9a52", "stone": "#a8642f"}


def build_region(mod, region, only):
    scn = B.fresh_scene("V13_" + region)
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
    regions = arg("--region") or "all"
    regions = REGIONS if regions == "all" else regions.split(",")
    out, sheets, only = arg("--out"), arg("--sheets"), arg("--only")
    only = set(only.split(",")) if only else None
    extra = [os.path.abspath(d) for d in (arg("--assets-extra") or "").split(",") if d]
    texdirs = [os.path.abspath(d) for d in (arg("--textures") or "").split(",") if d]
    out = os.path.abspath(out) if out else None
    report = []
    for region in regions:
        mod = importlib.import_module("v13_" + region)
        B.PALETTE.update(mod.PALETTE)
        B.WIND.update(getattr(mod, "WIND", {}))
        B.ALPHA.update(getattr(mod, "ALPHA", {}))
        scn, built = build_region(mod, region, only)
        with B.use_scene(scn):
            stats = {}
            if out:
                for name, objs in built.items():
                    stats[name] = K.export(name, objs, out, animated=name in getattr(mod, "ANIM", ()))
                    report.append((region, name, *stats[name]))
            if "--check" in argv:
                import check_coplanar
                g = B.Kit("v13_check_ground")
                g.add(K.box(400, 40, 0.0001), "grass", K.T((150, 0, -0.00005)))
                g.build(scn.collection)
                check_coplanar.check()
                bpy.data.objects.remove(bpy.data.objects["v13_check_ground"])
            if sheets:
                sheets = os.path.abspath(sheets)
                os.makedirs(sheets, exist_ok=True)
                info = {}
                for name, objs in built.items():
                    w, d = mod.FOOT.get(name, (1, 1))
                    t = stats[name][0] if name in stats else sum(B.tris(o) for o in objs if o.type == "MESH")
                    info[name] = f"{name}  {w}x{d}  {t} tris"
                if built:
                    K.tile_sheet(scn, mod.LIGHT, built, info, os.path.join(sheets, f"{region}_tiles.png"))
        if sheets and getattr(mod, "VIGNETTE", None) and not only:
            vs = B.fresh_scene("V13_" + region + "_map")
            with B.use_scene(vs):
                V = dict(mod.VIGNETTE, texdirs=texdirs)
                dirs = ([out] if out else []) + [os.path.join(HERE, "..", "..", "web", "world", "assets")] + extra
                K.vignette(vs, mod.LIGHT, V, dirs, dict(mod.FOOT, **getattr(mod, "FOOT_OTHER", {})),
                           path_day=os.path.join(sheets, f"{region}_day.png"),
                           path_night=os.path.join(sheets, f"{region}_night.png"),
                           tints={"crystal_glow": SEAL[region]})
    for region, name, t, size in report:
        print(f"[v13] {region:6s} {name:18s} {t:5d} tris {size / 1024:7.1f} KB")


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
