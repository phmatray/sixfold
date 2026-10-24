"""Assemble the overworld from web/world/maps/overworld.json and the exported .glb files.

Live Blender (scene "World", viewport left looking through the game camera):
    import runpy; runpy.run_path("tools/blender/build_world_scene.py", run_name="__main__")
Headless, in the shared file (rebuilds only the "World" scene, then saves the file):
    blender -b art/Sixfold.blend -P tools/blender/build_world_scene.py -- \
        [--check] [--render out/world.png] [--save copy.blend]

Coordinates: map tile (x, z) covers [x, x+1] x [z, z+1] in glTF space (Y up, row 0 far).
The glTF importer maps glTF (x, y, z) to Blender (x, -z, y), so a model at glTF
(px, 0, pz) sits at Blender (px, -pz, 0), and a turn about glTF +Y is a turn about Blender +Z.
"""
import json
import math
import os
import random
import re
import sys

import bpy
from mathutils import Vector

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import importlib  # noqa: E402

import build_assets  # noqa: E402  (one source for the fence auto-tiling)
import check_coplanar  # noqa: E402

importlib.reload(build_assets)  # live re-runs pick up edits
importlib.reload(check_coplanar)
fcurves, fence_pick = build_assets.fcurves, build_assets.fence_pick

REPO = os.path.abspath(os.path.join(HERE, "..", ".."))
WORLD_DIR = os.path.join(REPO, "web", "world")
FOOT = {"house": (7, 5), "tree": (2, 2)}  # everything else is 1 x 1
FACING = {"s": 0, "e": 90, "n": 180, "w": 270}  # model front is +Z (south, toward the camera)
LIB = "wl_"
LOOP = 288  # timeline loop: 12 s at 24 fps = 3 Idle cycles = 3 smoke cycles
SWAY = {"tree": (3.0, 60), "bush": (5.0, 40), "flowers": (7.0, 30)}  # degrees of noise, frames per wobble


def lin(hexstr):
    out = []
    for i in (0, 2, 4):
        c = int(hexstr.lstrip("#")[i:i + 2], 16) / 255
        out.append(c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4)
    return (*out, 1.0)


def to_blender(px, pz):
    return Vector((px, -pz, 0))


def fresh_scene():
    """The "World" scene of the current file, emptied; other scenes are never touched."""
    scn = bpy.data.scenes.get("World") or bpy.data.scenes.new("World")
    for extra in [x for x in bpy.data.scenes if re.fullmatch(r"(World|Preview|Assets|Gen)\.\d{3}", x.name)
                  and not x.objects]:
        bpy.data.scenes.remove(extra)  # strays from earlier headless imports
    build_assets.clear_scene(scn)
    for c in [c for c in bpy.data.collections if c.name.startswith(LIB)]:  # model libraries
        for o in list(c.objects):
            bpy.data.objects.remove(o)
        bpy.data.collections.remove(c)
    for a in [a for a in bpy.data.actions if a.users == 0 and not a.use_fake_user]:
        bpy.data.actions.remove(a)  # leftovers of earlier builds (sway, puffs, imported clips)
    return scn


def new_coll(scn, name, link=True):
    """Collection names are global in a .blend: scope them by scene (World keeps the plain names)."""
    name = name if scn.name == "World" else f"{scn.name}.{name}"
    c = bpy.data.collections.get(name) or bpy.data.collections.new(name)
    if link and c.name not in scn.collection.children:
        scn.collection.children.link(c)
    return c


def import_glb(scn, name, coll, subdir=""):
    """Import web/world/assets/<name>.glb into coll; returns the new objects."""
    vl = scn.view_layers[0]
    lc = vl.layer_collection.children[coll.name]
    vl.active_layer_collection = lc
    before, scenes = set(bpy.data.objects), set(bpy.data.scenes)
    bpy.ops.import_scene.gltf(filepath=os.path.join(WORLD_DIR, "assets", subdir, name + ".glb"))
    for extra in [x for x in bpy.data.scenes if x not in scenes and not x.objects]:
        bpy.data.scenes.remove(extra)  # headless, the importer adds an empty scene per file
    return [o for o in bpy.data.objects if o not in before]


def library(scn, name, prefix=LIB, subdir=""):
    """A collection holding one imported model, used through collection instances."""
    coll = new_coll(scn, prefix + name)
    for o in import_glb(scn, name, coll, subdir):
        if o.animation_data:  # e.g. the house door's "Open": keep it closed in the world
            o.animation_data.action = None
            for t in o.animation_data.nla_tracks:
                t.mute = True
            if o.type != "ARMATURE":  # back to the rest pose (the importer uses quaternions)
                o.rotation_euler = (0, 0, 0)
                o.rotation_quaternion = (1, 0, 0, 0)
    scn.collection.children.unlink(coll)
    return coll


def place(scn, coll, parent, name, loc, rot_deg):
    e = bpy.data.objects.new(name, None)
    e.instance_type = "COLLECTION"
    e.instance_collection = coll
    e.location = loc
    e.rotation_euler = (0, 0, math.radians(rot_deg))
    parent.objects.link(e)
    return e


def grass_material():
    m = bpy.data.materials.get(LIB + "grass") or bpy.data.materials.new(LIB + "grass")
    m.use_nodes = True
    nt = m.node_tree
    for n in [n for n in nt.nodes if n.type != "OUTPUT_MATERIAL"]:
        nt.nodes.remove(n)
    out = next(n for n in nt.nodes if n.type == "OUTPUT_MATERIAL")
    bsdf = nt.nodes.new("ShaderNodeBsdfPrincipled")
    bsdf.inputs["Roughness"].default_value = 1.0
    coord = nt.nodes.new("ShaderNodeTexCoord")
    nt.links.new(bsdf.outputs[0], out.inputs["Surface"])
    tex_path = os.path.join(WORLD_DIR, "textures", "grass.png")
    if os.path.exists(tex_path):  # the baked painterly grass, one repeat every 4 m
        tex = nt.nodes.new("ShaderNodeTexImage")
        tex.image = bpy.data.images.load(tex_path, check_existing=True)
        mapping = nt.nodes.new("ShaderNodeMapping")
        mapping.inputs["Scale"].default_value = (0.25, 0.25, 1)
        nt.links.new(coord.outputs["Object"], mapping.inputs["Vector"])
        nt.links.new(mapping.outputs["Vector"], tex.inputs["Vector"])
        nt.links.new(tex.outputs["Color"], bsdf.inputs["Base Color"])
        return m
    noise = nt.nodes.new("ShaderNodeTexNoise")
    noise.inputs["Scale"].default_value = 0.35
    noise.inputs["Detail"].default_value = 4.0
    ramp = nt.nodes.new("ShaderNodeValToRGB")
    els = ramp.color_ramp.elements
    els[0].position, els[0].color = 0.35, lin("#3a8a2c")
    els[1].position, els[1].color = 0.68, lin("#6cc24a")
    mid = els.new(0.5)
    mid.color = lin("#4fa83a")
    nt.links.new(coord.outputs["Object"], noise.inputs["Vector"])
    nt.links.new(noise.outputs["Fac"], ramp.inputs["Fac"])
    nt.links.new(ramp.outputs["Color"], bsdf.inputs["Base Color"])
    nt.links.new(bsdf.outputs[0], out.inputs["Surface"])
    return m


def flat_material(name, hexstr):
    m = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    m.use_nodes = True
    bsdf = next(n for n in m.node_tree.nodes if n.type == "BSDF_PRINCIPLED")
    bsdf.inputs["Base Color"].default_value = lin(hexstr)
    bsdf.inputs["Emission Color"].default_value = lin(hexstr)
    bsdf.inputs["Emission Strength"].default_value = 0.4
    return m


def quad_mesh(name, quads):
    me = bpy.data.meshes.new(name)
    verts, faces = [], []
    for q in quads:
        faces.append(tuple(range(len(verts), len(verts) + 4)))
        verts += q
    me.from_pydata(verts, [], faces)
    return me


def ground(scn, W, H):
    margin = 8
    me = quad_mesh(LIB + "ground", [[(-margin, margin, 0), (W + margin, margin, 0),
                                     (W + margin, -H - margin, 0), (-margin, -H - margin, 0)]])
    ob = bpy.data.objects.new("Ground", me)
    me.materials.append(grass_material())
    scn.collection.objects.link(ob)


def grid(scn, W, H):
    coll = new_coll(scn, "Grid")
    w, z = 0.01, 0.06  # 3 cm above the path tops; hide the Grid collection to toggle
    quads = [[(x - w, 0, z), (x + w, 0, z), (x + w, -H, z), (x - w, -H, z)] for x in range(W + 1)]
    z2 = z + 0.003  # rows sit 3 mm above columns: no coplanar overlap at the crossings
    quads += [[(0, -y + w, z2), (W, -y + w, z2), (W, -y - w, z2), (0, -y - w, z2)] for y in range(H + 1)]
    ob = bpy.data.objects.new("Grid1m", quad_mesh(LIB + "grid", quads))
    ob.data.materials.append(flat_material(LIB + "grid", "#d8ecb0"))
    ob.hide_select = True
    coll.objects.link(ob)


def lights(scn, data, hero_loc):
    light = data["light"]
    sun_d = bpy.data.lights.new(LIB + "sun", "SUN")
    sun_d.color = lin(light["sun"]["color"])[:3]
    sun_d.energy = light["sun"]["intensity"] * 1.5
    sun_d.angle = math.radians(3)
    sun = bpy.data.objects.new("Sun", sun_d)
    fx, fy, fz = light["sun"]["fromHero"]           # glTF offset hero -> sun
    to_sun = Vector((fx, -fz, fy)).normalized()
    sun.rotation_euler = to_sun.to_track_quat("Z", "Y").to_euler()  # light shines along local -Z
    sun.location = hero_loc + to_sun * 10
    scn.collection.objects.link(sun)

    hemi = light["hemisphere"]
    world = bpy.data.worlds.get(LIB + "world") or bpy.data.worlds.new(LIB + "world")
    world.use_nodes = True
    nt = world.node_tree
    for n in [n for n in nt.nodes if n.type != "OUTPUT_WORLD"]:
        nt.nodes.remove(n)
    out = next(n for n in nt.nodes if n.type == "OUTPUT_WORLD")
    bg = nt.nodes.new("ShaderNodeBackground")
    bg.inputs["Strength"].default_value = hemi["intensity"] * 0.55
    coord = nt.nodes.new("ShaderNodeTexCoord")
    sep = nt.nodes.new("ShaderNodeSeparateXYZ")
    ramp = nt.nodes.new("ShaderNodeValToRGB")  # hemisphere: ground colour below, sky above
    els = ramp.color_ramp.elements
    els[0].position, els[0].color = 0.0, lin(hemi["ground"])
    els[1].position, els[1].color = 1.0, lin(hemi["sky"])
    rng = nt.nodes.new("ShaderNodeMapRange")
    rng.inputs["From Min"].default_value = -1.0
    nt.links.new(coord.outputs["Generated"], sep.inputs[0])
    nt.links.new(sep.outputs["Z"], rng.inputs["Value"])
    nt.links.new(rng.outputs["Result"], ramp.inputs["Fac"])
    nt.links.new(ramp.outputs["Color"], bg.inputs["Color"])
    nt.links.new(bg.outputs[0], out.inputs["Surface"])
    scn.world = world


def camera(scn, data, hero_loc):
    cam_cfg = data["camera"]
    pitch = math.radians(cam_cfg["pitch"])
    d = cam_cfg["distance"]
    ox, oy, oz = cam_cfg["lookAtOffset"]
    look = hero_loc + Vector((ox, -oz, oy))
    cd = bpy.data.cameras.new(LIB + "camera")
    cd.sensor_fit = "VERTICAL"
    cd.angle_y = math.radians(cam_cfg["fov"])
    cd.clip_start, cd.clip_end = 0.5, 200  # a tiny clip_start costs depth precision (flicker)
    cam = bpy.data.objects.new("GameCamera", cd)
    cam.location = look + Vector((0, -d * math.cos(pitch), d * math.sin(pitch)))
    cam.rotation_euler = (math.pi / 2 - pitch, 0, 0)  # looks toward +Y (glTF -Z), pitched down
    scn.collection.objects.link(cam)
    scn.camera = cam


def hero(scn, data):
    coll = new_coll(scn, "Hero")
    old_actions = set(bpy.data.actions)
    objs = import_glb(scn, "hero", coll)
    sp = data["spawn"]
    loc = to_blender(sp["x"] + 0.5, sp["z"] + 0.5)
    root = next(o for o in objs if o.parent is None)
    root.location = loc
    root.rotation_mode = "XYZ"
    root.rotation_euler = (0, 0, math.radians(FACING.get(sp.get("facing", "s"), 0)))
    rig = next((o for o in objs if o.type == "ARMATURE"), None)
    idle = next((a for a in bpy.data.actions if a not in old_actions and a.name.lower().startswith("idle")), None)
    if rig and idle:
        ad = rig.animation_data or rig.animation_data_create()
        ad.action = None
        for t in ad.nla_tracks:
            t.mute = True
        track = ad.nla_tracks.new()
        track.name = "IdleLoop"
        strip = track.strips.new("Idle", 0, idle)
        s, e = idle.frame_range  # last key == first, so repeats join seamlessly
        strip.repeat = LOOP / (e - s)
    return loc


def build(data):
    scn = fresh_scene()
    with build_assets.use_scene(scn):
        return build_in(scn, data)


def build_in(scn, data):
    scn.render.fps = 24
    W, H = data["width"], data["height"]
    ground(scn, W, H)

    fences = {(x, z) for z, row in enumerate(data["tiles"]) for x, ch in enumerate(row) if ch == "f"}
    names = sorted({p["asset"] for p in data["props"]} | {"path", "path_b", "path_c"}
                   | ({"fence", "fence_corner", "fence_end", "fence_t"} if fences else set()))
    libs = {n: library(scn, n) for n in names
            if os.path.exists(os.path.join(WORLD_DIR, "assets", n + ".glb"))}
    paths = [n for n in ("path", "path_b", "path_c") if n in libs]

    pcoll = new_coll(scn, "Paths")
    for z, row in enumerate(data["tiles"]):
        for x, ch in enumerate(row):
            if ch == "p":
                h = (x * 73856093) ^ (z * 19349663)
                place(scn, libs[paths[h % len(paths)]], pcoll, f"path_{x}_{z}",
                      to_blender(x + 0.5, z + 0.5), 90 * ((h >> 3) % 4))

    fcoll = new_coll(scn, "Fences")
    for x, z in sorted(fences):
        nb = {d for d, (dx, dz) in {"N": (0, -1), "E": (1, 0), "S": (0, 1), "W": (-1, 0)}.items()
              if (x + dx, z + dz) in fences}
        for piece, rot in fence_pick(nb):
            if piece in libs:
                place(scn, libs[piece], fcoll, f"{piece}_{x}_{z}", to_blender(x + 0.5, z + 0.5), rot)

    props = new_coll(scn, "Props")
    for p in data["props"]:
        if p["asset"] not in libs:
            continue
        w, d = FOOT.get(p["asset"], (1, 1))
        if p["rot"] % 180:
            w, d = d, w
        place(scn, libs[p["asset"]], props, f"{p['asset']}_{p['x']}_{p['z']}",
              to_blender(p["x"] + w / 2, p["z"] + d / 2), p["rot"])

    hero_loc = hero(scn, data)
    helper = scn.view_layers[0].layer_collection.children.get("glTF_not_exported")
    if helper:  # bone-display shapes the importer adds
        helper.exclude = True
    grid(scn, W, H)
    lights(scn, data, hero_loc)
    camera(scn, data, hero_loc)

    scn.render.resolution_x, scn.render.resolution_y = 1600, 1000
    scn.view_settings.view_transform = "Standard"
    for eng in ("BLENDER_EEVEE", "BLENDER_EEVEE_NEXT"):
        try:
            scn.render.engine = eng
            break
        except TypeError:
            continue
    sway(scn, props)
    smoke(scn, props)
    scn.frame_start, scn.frame_end = 0, LOOP - 1
    scn.frame_set(0)
    return scn


def sway(scn, props):
    """Noise on each tree/bush/flower instance's tilt, phase-shifted per object."""
    rng = random.Random(7)
    for e in props.objects:
        kind = e.instance_collection.name[len(LIB):] if e.instance_collection else ""
        if kind not in SWAY:
            continue
        deg, period = SWAY[kind]
        e.keyframe_insert("rotation_euler", index=0, frame=0)
        e.keyframe_insert("rotation_euler", index=1, frame=0)
        for fc in fcurves(e.animation_data.action):
            m = fc.modifiers.new("NOISE")
            m.scale = period * rng.uniform(0.8, 1.2)
            m.strength = math.radians(deg)
            m.phase = rng.uniform(0, 100)
            m.offset = rng.uniform(0, 500)


def smoke(scn, props):
    """Three puffs rising from the chimney's smoke_origin, looping every 96 frames."""
    lib = bpy.data.collections.get(LIB + "house")
    origin = next((o for o in lib.all_objects if o.name.startswith("smoke_origin")), None) if lib else None
    houses = [e for e in props.objects if e.instance_collection == lib]
    if not origin or not houses:
        return
    coll = new_coll(scn, "Smoke")
    mat = flat_material(LIB + "smoke", "#f4f1ea")
    base = houses[0].matrix_world @ origin.matrix_world.translation
    for i in range(3):
        bpy_mesh = bpy.data.meshes.get(LIB + "puff")
        if bpy_mesh is None:
            import bmesh
            bm = bmesh.new()
            bmesh.ops.create_icosphere(bm, subdivisions=2, radius=1.0)
            bpy_mesh = bpy.data.meshes.new(LIB + "puff")
            bm.to_mesh(bpy_mesh)
            bm.free()
            for p in bpy_mesh.polygons:
                p.use_smooth = True
            bpy_mesh.materials.append(mat)
        puff = bpy.data.objects.new(f"puff_{i}", bpy_mesh)
        coll.objects.link(puff)
        for f, (dx, dz, sc) in ((0, (0, 0, 0.05)), (40, (0.15, 0.7, 0.32)), (96, (0.5, 1.8, 0.0))):
            puff.location = base + Vector((dx, 0, dz))
            puff.scale = (sc, sc, sc)
            puff.keyframe_insert("location", frame=f + 32 * i)
            puff.keyframe_insert("scale", frame=f + 32 * i)
        for fc in fcurves(puff.animation_data.action):
            fc.modifiers.new("CYCLES")
        puff.location = base


def show_in_viewport(scn):
    for area in bpy.context.window.screen.areas:
        if area.type == "VIEW_3D":
            sp = area.spaces.active
            sp.shading.type = "MATERIAL"
            sp.region_3d.view_perspective = "CAMERA"
            sp.overlay.show_floor = False
            sp.clip_start = 0.1  # depth precision: no flicker at distance
            sp.clip_end = 300
    with bpy.context.temp_override(window=bpy.context.window):
        if not bpy.context.screen.is_animation_playing:
            bpy.ops.screen.animation_play()


def main(argv):
    with open(os.path.join(WORLD_DIR, "maps", "overworld.json")) as f:
        data = json.load(f)
    scn = build(data)
    if not bpy.app.background:
        show_in_viewport(scn)
    else:  # make World the scene the file opens on
        for win in bpy.data.window_managers[0].windows:
            win.scene = scn
    if "--check" in argv:
        with build_assets.use_scene(scn):
            check_coplanar.check()
    if "--render" in argv:
        path = os.path.abspath(argv[argv.index("--render") + 1])
        os.makedirs(os.path.dirname(path), exist_ok=True)
        scn.render.filepath = path
        scn.render.image_settings.file_format = "PNG"
        bpy.ops.render.render(write_still=True, scene=scn.name)
        print("[world] render ->", path)
    if "--save" in argv:
        path = os.path.abspath(argv[argv.index("--save") + 1])
        os.makedirs(os.path.dirname(path), exist_ok=True)
        bpy.ops.wm.save_as_mainfile(filepath=path, copy=True)
        print("[world] saved copy ->", path)
    build_assets.save_if_file()


if __name__ == "__main__":
    args = globals().get("ARGS")
    if args is None:
        args = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    main(args)
