"""Turn a raw text-to-3D download into a game-ready prop (reproducible, headless).

    blender -b --factory-startup -P tools/blender/postprocess_generated.py -- \
        --raw tools/blender/generated/tree_raw.glb --asset tree --out web/world/assets/tree_gen.glb \
        [--rot-z 0] [--sat 1.0] [--val 1.0]

Steps: join meshes, turn so the front faces -Y (glTF +Z), decimate to the triangle budget,
scale to the asset's grid footprint (origin at the footprint centre, base sunk 2 cm),
keep only the base-colour texture (<= 1024 px, optional saturation/value grade toward the
procedural palette), add wind weights (COLOR_0) for foliage, export GLB.
"""
import math
import os
import sys

import bmesh
import bpy
import numpy as np
from mathutils import Matrix, Vector

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import build_assets  # noqa: E402  (wind weights, tri counting)

# footprint (x, y) the model must fill, its max size, and the triangle budget
SPEC = {
    "house": {"fit": (7.0, 5.0), "tris": 3000},
    "tree": {"fit": (2.5, 2.5), "tris": 3000},   # 2 x 2 tiles + 0.25 m canopy overhang
    "rock": {"fit": (0.95, 0.95), "tris": 1500},
    "bush": {"fit": (1.1, 1.1), "tris": 1500},
}


def arg(argv, name, default=None, cast=str):
    return cast(argv[argv.index(name) + 1]) if name in argv else default


def join_meshes():
    meshes = [o for o in bpy.context.scene.objects if o.type == "MESH"]
    for o in bpy.context.scene.objects:
        o.select_set(o in meshes)
    bpy.context.view_layer.objects.active = meshes[0]
    if len(meshes) > 1:
        bpy.ops.object.join()
    ob = bpy.context.view_layer.objects.active
    for o in [o for o in bpy.context.scene.objects if o is not ob]:
        bpy.data.objects.remove(o)  # empties / armature roots from the importer
    ob.parent = None
    bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
    return ob


def decimate(ob, tris):
    now = build_assets.tris(ob)
    if now > tris:
        m = ob.modifiers.new("dec", "DECIMATE")
        m.ratio = tris / now * 0.97
        bpy.ops.object.modifier_apply(modifier=m.name)
    return build_assets.tris(ob)


def fit(ob, rot_z, size):
    me = ob.data
    me.transform(Matrix.Rotation(math.radians(rot_z), 4, "Z"))
    xs, ys, zs = zip(*(v.co for v in me.vertices))
    s = min(size[0] / (max(xs) - min(xs)), size[1] / (max(ys) - min(ys)))
    c = Vector(((max(xs) + min(xs)) / 2, (max(ys) + min(ys)) / 2, min(zs)))
    me.transform(Matrix.Scale(s, 4) @ Matrix.Translation(-c))
    me.transform(Matrix.Translation((0, 0, -0.02)))  # sink 2 cm: never coplanar with the ground
    me.update()


def base_colour_only(ob, sat, val):
    for mat in ob.data.materials:
        if not mat or not mat.use_nodes:
            continue
        nt = mat.node_tree
        bsdf = next((n for n in nt.nodes if n.type == "BSDF_PRINCIPLED"), None)
        if not bsdf:
            continue
        for sock in bsdf.inputs:  # drop every texture except base colour
            if sock.name != "Base Color":
                for link in list(sock.links):
                    nt.links.remove(link)
        bsdf.inputs["Roughness"].default_value = 0.9
        bsdf.inputs["Metallic"].default_value = 0.0
        for n in [n for n in nt.nodes if n.type == "NORMAL_MAP"]:
            nt.nodes.remove(n)
        for n in [n for n in nt.nodes if n.type == "TEX_IMAGE" and not n.outputs[0].links]:
            nt.nodes.remove(n)
        for n in [n for n in nt.nodes if n.type == "TEX_IMAGE" and n.image]:
            img = n.image
            if max(img.size) > 1024:
                img.scale(*(int(d * 1024 / max(img.size)) for d in img.size))
            if sat != 1.0 or val != 1.0:
                px = np.empty(len(img.pixels), dtype=np.float32)
                img.pixels.foreach_get(px)
                rgba = px.reshape(-1, 4)
                grey = rgba[:, :3].mean(axis=1, keepdims=True)
                rgba[:, :3] = np.clip((grey + (rgba[:, :3] - grey) * sat) * val, 0, 1)
                img.pixels.foreach_set(rgba.ravel())
            img.update()
            img.pack()


def main(argv):
    raw, asset, out = arg(argv, "--raw"), arg(argv, "--asset"), arg(argv, "--out")
    for o in list(bpy.data.objects):
        bpy.data.objects.remove(o)
    bpy.ops.import_scene.gltf(filepath=os.path.abspath(raw))
    ob = join_meshes()
    ob.name = asset
    spec = SPEC[asset]
    raw_tris = build_assets.tris(ob)
    fit(ob, arg(argv, "--rot-z", 0.0, float), spec["fit"])
    t = decimate(ob, spec["tris"])
    base_colour_only(ob, arg(argv, "--sat", 1.0, float), arg(argv, "--val", 1.0, float))
    if asset in build_assets.WIND:
        build_assets.add_wind(ob, build_assets.WIND[asset])
    for o in bpy.context.scene.objects:
        o.select_set(o is ob)
    os.makedirs(os.path.dirname(os.path.abspath(out)), exist_ok=True)
    bpy.ops.export_scene.gltf(filepath=os.path.abspath(out), export_format="GLB", use_selection=True,
                              export_yup=True, export_apply=True, export_animations=False,
                              export_vertex_color="ACTIVE" if asset in build_assets.WIND else "NONE",
                              export_image_format="JPEG", export_jpeg_quality=85)
    dims = [round(d, 2) for d in ob.dimensions]
    print(f"[gen] {asset}: {raw_tris} -> {t} tris, {dims} m, {os.path.getsize(out) // 1024} KB -> {out}")


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
