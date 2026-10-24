"""Assemble the shop interior from web/world/maps/shop.json into the scene "Interior".

Headless, in the shared file (rebuilds only "Interior", then saves):
    blender -b art/Sixfold.blend -P tools/blender/build_interior_scene.py -- [--render out.png]
Live: runpy.run_path("tools/blender/build_interior_scene.py", run_name="__main__")

Tiles: "W" wall (auto-tiled with fence_pick; the south row, on the camera side, uses the 0.6 m
low pieces), "w" wood floor. Props, hero spawn, camera and lights come from the map file.
"""
import importlib
import json
import math
import os
import sys

import bpy
from mathutils import Vector

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import build_assets  # noqa: E402
import build_world_scene as bws  # noqa: E402

importlib.reload(build_assets)
importlib.reload(bws)

SCENE = "Interior"
LIB = "il_"
FOOT = {"counter": (3, 1), "shelf": (2, 1), "duel_table": (2, 1), "rug": (3, 2)}
# straight wall tiles that get a window (the map only says "W"); the duel table sits by the west one
WINDOWS = {(0, 4), (8, 0), (10, 5)}
PIECE = {"fence": "wall", "fence_corner": "wall_corner", "fence_end": "wall_end", "fence_t": "wall_t"}


def fresh_scene():
    scn = bpy.data.scenes.get(SCENE) or bpy.data.scenes.new(SCENE)
    build_assets.clear_scene(scn)
    for c in [c for c in bpy.data.collections if c.name.startswith(SCENE + ".")]:
        for o in list(c.objects):
            bpy.data.objects.remove(o)
        bpy.data.collections.remove(c)
    for a in [a for a in bpy.data.actions if a.users == 0 and not a.use_fake_user]:
        bpy.data.actions.remove(a)
    return scn


def floor(scn, W, H):
    mat = bpy.data.materials.get(LIB + "floor") or bpy.data.materials.new(LIB + "floor")
    mat.use_nodes = True
    nt = mat.node_tree
    bsdf = next(n for n in nt.nodes if n.type == "BSDF_PRINCIPLED")
    tex_path = os.path.join(bws.WORLD_DIR, "textures", "wood_floor.png")
    if os.path.exists(tex_path) and not bsdf.inputs["Base Color"].links:
        tex = nt.nodes.new("ShaderNodeTexImage")
        tex.image = bpy.data.images.load(tex_path, check_existing=True)
        mapping = nt.nodes.new("ShaderNodeMapping")
        mapping.inputs["Scale"].default_value = (0.5, 0.5, 1)  # texture spans 2 m
        coord = nt.nodes.new("ShaderNodeTexCoord")
        nt.links.new(coord.outputs["Object"], mapping.inputs["Vector"])
        nt.links.new(mapping.outputs["Vector"], tex.inputs["Vector"])
        nt.links.new(tex.outputs["Color"], bsdf.inputs["Base Color"])
    else:
        bsdf.inputs["Base Color"].default_value = bws.lin("#b07a4c")
    me = bws.quad_mesh(LIB + "floor", [[(0, 0, 0), (W, 0, 0), (W, -H, 0), (0, -H, 0)]])
    me.materials.append(mat)
    ob = bpy.data.objects.new("Floor", me)
    scn.collection.objects.link(ob)


def build(data):
    scn = fresh_scene()
    with build_assets.use_scene(scn):
        scn.render.fps = 24
        W, H = data["width"], data["height"]
        floor(scn, W, H)
        walls = {(x, z) for z, row in enumerate(data["tiles"]) for x, ch in enumerate(row) if ch == "W"}
        names = {p["asset"] for p in data["props"]} | set(PIECE.values()) \
            | {"wall_low", "wall_low_corner", "wall_low_end", "wall_low_t", "wall_window"}
        libs = {n: bws.library(scn, n, LIB, "interior") for n in sorted(names)
                if os.path.exists(os.path.join(bws.WORLD_DIR, "assets", "interior", n + ".glb"))}

        wcoll = bws.new_coll(scn, "Walls")
        for x, z in sorted(walls):
            nb = {d for d, (dx, dz) in {"N": (0, -1), "E": (1, 0), "S": (0, 1), "W": (-1, 0)}.items()
                  if (x + dx, z + dz) in walls}
            for piece, rot in build_assets.fence_pick(nb):
                name = PIECE[piece]
                if z == H - 1:  # camera-facing south row: cutaway
                    name = name.replace("wall", "wall_low", 1)
                elif name == "wall" and (x, z) in WINDOWS:
                    name = "wall_window"
                bws.place(scn, libs[name], wcoll, f"{name}_{x}_{z}", bws.to_blender(x + 0.5, z + 0.5), rot)

        props = bws.new_coll(scn, "Props")
        for p in data["props"]:
            if p["asset"] not in libs:
                continue
            w, d = FOOT.get(p["asset"], (1, 1))
            if p["rot"] % 180:
                w, d = d, w
            e = bws.place(scn, libs[p["asset"]], props, f"{p['asset']}_{p['x']}_{p['z']}",
                          bws.to_blender(p["x"] + w / 2, p["z"] + d / 2), p["rot"])
            if p["asset"] == "lamp":  # a warm point light on the lamp's light_origin
                origin = next((o for o in libs["lamp"].all_objects if o.name.startswith("light_origin")), None)
                if origin:
                    lc = data["light"].get("lamp", {})
                    ld = bpy.data.lights.new(LIB + "lamp", "POINT")
                    ld.color = bws.lin(lc.get("color", "#ffb257"))[:3]
                    ld.energy = lc.get("intensity", 5) * 40
                    ld.shadow_soft_size = 0.2
                    lo = bpy.data.objects.new(f"lamp_light_{p['x']}_{p['z']}", ld)
                    lo.location = e.matrix_world @ origin.matrix_world.translation
                    props.objects.link(lo)

        hero_loc = bws.hero(scn, data)
        helper = scn.view_layers[0].layer_collection.children.get(f"{SCENE}.glTF_not_exported") \
            or scn.view_layers[0].layer_collection.children.get("glTF_not_exported")
        if helper:
            helper.exclude = True
        bws.grid(scn, W, H)
        centre = Vector((W / 2, -H / 2, 0))
        bws.lights(scn, data, centre)  # indoors the sun is the window light, from the room centre
        bws.camera(scn, data, hero_loc)
        scn.render.resolution_x, scn.render.resolution_y = 1600, 1000
        scn.view_settings.view_transform = "Standard"
        try:
            scn.render.engine = "BLENDER_EEVEE"
        except TypeError:
            pass
        scn.frame_start, scn.frame_end = 0, bws.LOOP - 1
        scn.frame_set(0)
    return scn


def main(argv):
    with open(os.path.join(bws.WORLD_DIR, "maps", "shop.json")) as f:
        data = json.load(f)
    scn = build(data)
    if "--render" in argv:
        path = os.path.abspath(argv[argv.index("--render") + 1])
        scn.render.filepath = path
        scn.render.image_settings.file_format = "PNG"
        bpy.ops.render.render(write_still=True, scene=scn.name)
        print("[interior] render ->", path)
    build_assets.save_if_file()


if __name__ == "__main__":
    args = globals().get("ARGS")
    if args is None:
        args = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    main(args)
