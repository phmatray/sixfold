"""Shared kit for the 1.3 region props (see build_regions_v13.py): shape helpers, export, and the review renders.

Conventions are those of build_assets.py / build_area_props.py / build_props_v2.py (imported or copied here):
bmesh primitives, flat palette colours, Blender Z-up with the front facing -Y (glTF +Z), origin at the
footprint centre on the ground, bases sunk 2 cm, nothing coplanar with y = 0. Materials whose name contains
`glow` are the night-emissive parts. Wind weights go in COLOR_0 (red: 0 at the base, 1 at the tips).

The review renders imitate the game's look rather than Blender's: every material is swapped for an emission
shader computing three.js MeshToonMaterial lighting (the 4-step gradient map 90/160/215/255 on N.L, shadowed
direct sun, a hemisphere light, linear fog toward the background), from a map's `light` block.
"""
import math
import os
import random
import sys

import bmesh
import bpy
from mathutils import Matrix, Vector

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import build_area_props as A  # noqa: E402  (imports build_assets and extends its palette)
import build_assets as B  # noqa: E402
from build_assets import T, box, contact, cyl, ico, jitter, prism, sphere  # noqa: E402,F401

shadow, empty, beam, stratum, loft, planks, door_leaf = A.shadow, A.empty, A.beam, A.stratum, A.loft, A.planks, A.door_leaf
clamp01 = B.clamp01

# Shared colours: the seal colours (props-v2) double as the region colours for the waymark crystal.
B.PALETTE.update({
    "contact_in": "#1c3216", "contact_out": "#1c3216",
    "seal_grass": "#3f9a52", "seal_fire": "#e1572f", "seal_water": "#2f7fd0", "seal_lightning": "#e2ad19",
    "seal_fighting": "#a8642f", "seal_psychic": "#8a4fc9",
    "rope": "#c8a46a", "vine": "#4f8a3a", "vine_lt": "#78b04e",
})

# ---------------------------------------------------------------- shape helpers (first five as in build_props_v2.py)


def ribbon(pts, w, d, closed=False):
    """A strip w wide following 2D points (x, y) in the XY plane, from z = 0 to z = d (arches, rims, letters)."""
    bm = bmesh.new()
    n = len(pts)
    rows = []
    for i, (x, y) in enumerate(pts):
        a = pts[i - 1] if (i > 0 or closed) else pts[i]
        b = pts[(i + 1) % n] if (i < n - 1 or closed) else pts[i]
        tx, ty = b[0] - a[0], b[1] - a[1]
        L = math.hypot(tx, ty) or 1.0
        nx, ny = -ty / L, tx / L
        rows.append([bm.verts.new((x + s * nx * w / 2, y + s * ny * w / 2, z)) for s, z in ((-1, 0), (1, 0), (1, d), (-1, d))])
    for i in range(n if closed else n - 1):
        r0, r1 = rows[i], rows[(i + 1) % n]
        for j in range(4):
            q = (j + 1) % 4
            bm.faces.new((r0[j], r1[j], r1[q], r0[q]))
    if not closed:
        bm.faces.new(rows[0][::-1])
        bm.faces.new(rows[-1])
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return bm


def arc(cx, cy, r, a0, a1, n):
    return [(cx + r * math.cos(math.radians(a0 + (a1 - a0) * i / n)), cy + r * math.sin(math.radians(a0 + (a1 - a0) * i / n)))
            for i in range(n + 1)]


def arch_pts(w, h, n=8):
    """Outline of a round-topped arch, w wide, h tall to the crown, base on y = 0."""
    r = w / 2
    return [(-r, 0.0)] + arc(0, h - r, r, 180, 0, n) + [(r, 0.0)]


def torus(R, r, seg=12, sides=5):
    bm = bmesh.new()
    rows = [[bm.verts.new(((R + r * math.cos(b)) * math.cos(a), (R + r * math.cos(b)) * math.sin(a), r * math.sin(b)))
             for b in (j / sides * math.tau for j in range(sides))] for a in (i / seg * math.tau for i in range(seg))]
    for i in range(seg):
        for j in range(sides):
            bm.faces.new((rows[i][j], rows[(i + 1) % seg][j], rows[(i + 1) % seg][(j + 1) % sides], rows[i][(j + 1) % sides]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return bm


def half_sphere(r, u=16, v=10):
    bm = sphere(r, u, v)
    bmesh.ops.bisect_plane(bm, geom=list(bm.verts) + list(bm.edges) + list(bm.faces),
                           plane_co=(0, 0, 0), plane_no=(0, 0, -1), clear_outer=True)
    return bm


FRONT = (90, 0, 0)        # XY-plane shapes onto a -Y (front) face, raised toward -Y
BACK = (90, 0, 180)       # ... onto a +Y (back) face, raised toward +Y


def lathe(profile, seg=12, phase=0.0):
    """Surface of revolution about Z from (r, z) points, bottom to top; an r = 0 end closes to a point,
    any other end gets a flat cap. phase turns the ring (degrees)."""
    bm = bmesh.new()
    rings = []
    for r, z in profile:
        if r <= 1e-6:
            rings.append([bm.verts.new((0, 0, z))])
        else:
            rings.append([bm.verts.new((r * math.cos(math.radians(phase) + a), r * math.sin(math.radians(phase) + a), z))
                          for a in (i / seg * math.tau for i in range(seg))])
    for a, b in zip(rings, rings[1:]):
        for i in range(seg):
            j = (i + 1) % seg
            if len(a) == 1:
                bm.faces.new((a[0], b[i], b[j]))
            elif len(b) == 1:
                bm.faces.new((a[i], b[0], a[j]))
            else:
                bm.faces.new((a[i], a[j], b[j], b[i]))
    if len(rings[0]) > 1:
        bm.faces.new(rings[0][::-1])
    if len(rings[-1]) > 1:
        bm.faces.new(rings[-1])
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return bm


def tube(pts, r, sides=5, taper=1.0, cap=True):
    """A tube along 3D points (ropes, cables, vines, roots, fronds); radius goes from r to r * taper."""
    bm = bmesh.new()
    pts = [Vector(p) for p in pts]
    rows = []
    n = len(pts)
    up = Vector((0, 0, 1))
    for i, p in enumerate(pts):
        t = (pts[min(i + 1, n - 1)] - pts[max(i - 1, 0)]).normalized()
        side = t.cross(up)
        if side.length < 1e-4:
            side = t.cross(Vector((1, 0, 0)))
        side.normalize()
        nrm = side.cross(t).normalized()
        rr = r * (1 + (taper - 1) * i / max(n - 1, 1))
        rows.append([bm.verts.new(p + (side * math.cos(a) + nrm * math.sin(a)) * rr)
                     for a in (j / sides * math.tau for j in range(sides))])
    for a, b in zip(rows, rows[1:]):
        for j in range(sides):
            k = (j + 1) % sides
            bm.faces.new((a[j], a[k], b[k], b[j]))
    if cap:
        bm.faces.new(rows[0][::-1])
        bm.faces.new(rows[-1])
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return bm


def curve(a, b, sag, n=6):
    """Points from a to b hanging `sag` metres at the middle (ropes, cables, garlands)."""
    a, b = Vector(a), Vector(b)
    return [a.lerp(b, i / n) - Vector((0, 0, sag * 4 * (i / n) * (1 - i / n))) for i in range(n + 1)]


def rock(rng, r, sub=1, amt=0.12):
    """A faceted boulder: a jittered icosphere."""
    return jitter(ico(r, sub), rng, r * amt)


def ring_pts(r, n, phase=0.0, z=0.0):
    return [(r * math.cos(math.radians(phase) + i / n * math.tau), r * math.sin(math.radians(phase) + i / n * math.tau), z) for i in range(n)]


def facet_block(rng, w, d, z0, z1, bulge=0.03, cuts=1, top_in=False):
    """A w x d slab from z0 to z1 with its side verts pushed in/out a little (basalt, slate, dry stone)."""
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)
    if cuts:
        bmesh.ops.subdivide_edges(bm, edges=bm.edges[:], cuts=cuts, use_grid_fill=True)
    for v in bm.verts:
        v.co.z = z0 + (v.co.z + 0.5) * (z1 - z0)
        top = abs(v.co.z - z1) < 1e-4
        for ax, size in ((0, w), (1, d)):
            edge = abs(abs(v.co[ax]) - 0.5) < 1e-4
            v.co[ax] *= size
            if edge and not (top and top_in):
                v.co[ax] += math.copysign(rng.uniform(-0.4, 1.0) * bulge, v.co[ax])
            elif edge:
                v.co[ax] -= math.copysign(rng.uniform(0, 0.02), v.co[ax])
    bottom = [f for f in bm.faces if f.normal.z < -0.9 and all(abs(v.co.z - z0) < 1e-4 for v in f.verts)]
    bmesh.ops.delete(bm, geom=bottom, context="FACES")
    return bm


def open_part(build, pivot, keys, name):
    """after() hook: a separately built moving part, parented at `pivot`, with an `Open` clip.
    keys: [(frame, rot_deg (x, y, z), scale (x, y, z)), ...]; the first key is the rest (closed) pose."""
    def after(ob, coll):
        k = B.Kit(name)
        build(k)
        part = k.build(coll)
        part.parent = ob
        part.location = pivot
        part.rotation_mode = "XYZ"
        part.animation_data_create()
        act = bpy.data.actions.new("Open")
        act.use_fake_user = True
        part.animation_data.action = act
        for f, rot, sc in keys:
            part.rotation_euler = [math.radians(a) for a in rot]
            part.scale = sc
            part.keyframe_insert("rotation_euler", frame=f)
            part.keyframe_insert("scale", frame=f)
        tr = part.animation_data.nla_tracks.new()
        tr.name = "Open"  # glTF clip names come from NLA track names; same-named tracks merge into one clip
        tr.strips.new("Open", 0, act)
        part.animation_data.action = None
        _, rot, sc = keys[0]
        part.rotation_euler = [math.radians(a) for a in rot]
        part.scale = sc
        return [part]
    return after


def along(p, n):
    """A matrix at p with its local +Z along n (things stuck on a curved surface, logs, handles)."""
    return Matrix.Translation(p) @ Vector(n).normalized().to_track_quat("Z", "Y").to_matrix().to_4x4()


def quad(pts, normal):
    """One flat face through 3D points, turned to face `normal` (chalk strokes, marks laid just off a surface)."""
    bm = bmesh.new()
    f = bm.faces.new([bm.verts.new(p) for p in pts])
    f.normal_update()
    if f.normal.dot(Vector(normal)) < 0:
        f.normal_flip()
    return bm


def centre(k):
    """Move every part so the bounding box centre sits on the origin (item popups)."""
    vs = [v.co for bm, *_ in k.parts for v in bm.verts]
    c = Vector([(min(v[i] for v in vs) + max(v[i] for v in vs)) / 2 for i in range(3)])
    for bm, *_ in k.parts:
        bmesh.ops.translate(bm, vec=-c, verts=bm.verts)


def chain(*hooks):
    """Several after() hooks as one."""
    def after(ob, coll):
        out = []
        for h in hooks:
            out += h(ob, coll)
        return out
    return after


# ---------------------------------------------------------------- export


def export(name, objs, out_dir, animated=False, clips=None):
    """One prop -> <out_dir>/<name>.glb (Y-up, front +Z, roots at the origin). Returns (tris, bytes).
    clips: the clip names the file must carry (default ["Open"] when animated, else none)."""
    want = clips if clips is not None else (["Open"] if animated else [])
    animated = bool(want)
    import json
    import struct
    for o in bpy.context.view_layer.objects:
        o.select_set(False)
    for o in objs:
        o.select_set(True)
    bpy.context.view_layer.objects.active = objs[0]
    roots = [(o, o.location.copy()) for o in objs if o.parent is None]
    for o, _ in roots:
        o.location = (0, 0, 0)
    for e in objs[1:]:  # Blender suffixes repeated names (smoke_origin.001, door.001); the game wants the plain one
        base = e.name.split(".")[0]
        other = bpy.data.objects.get(base)
        if other and other is not e:
            other.name = base + "_parked"
        e.name = base
    path = os.path.join(out_dir, name + ".glb")
    os.makedirs(out_dir, exist_ok=True)
    bpy.ops.export_scene.gltf(
        filepath=path, export_format="GLB", use_selection=True, use_active_scene=True, export_yup=True,
        export_apply=True, export_animations=animated, export_animation_mode="NLA_TRACKS",
        export_vertex_color="ACTIVE" if name in B.WIND else "NONE", export_skins=False,
        export_cameras=False, export_lights=False, export_image_format="JPEG", export_jpeg_quality=85)
    for o, loc in roots:
        o.location = loc
    with open(path, "rb") as f:
        data = f.read()
    j = json.loads(data[20:20 + struct.unpack("<I", data[12:16])[0]])
    clips = [a["name"] for a in j.get("animations", [])]
    problems = []
    if len(j.get("scenes", [])) != 1:
        problems.append(f"{len(j.get('scenes', []))} scenes")
    if sorted(clips) != sorted(want):
        problems.append(f"clips {clips}")
    if any(n.get("name", "").startswith("hero") for n in j["nodes"]) or j.get("skins"):
        problems.append("hero data or skin in a prop")
    tris = sum(B.tris(o) for o in objs if o.type == "MESH")
    size = os.path.getsize(path)
    if tris > 3000:
        problems.append(f"{tris} triangles (> 3k)")
    if size > 2 * 1024 * 1024:
        problems.append(f"{size} bytes (> 2 MB)")
    if problems:
        raise RuntimeError(f"[v13] bad export {name}.glb: " + "; ".join(problems))
    return tris, size


# ---------------------------------------------------------------- review renders (game look)


def hexlin(h):
    return B.lin(h)[:3]


NIGHT = dict(sun=("#9fb4ff", 0.35), from_hero=(6, 18, -6), hemi=("#2a3557", "#10141f", 0.6), bg="#1b2236", fog=(12, 28))


def base_factor(bsdf):
    """The flat base colour of a BSDF, also when an imported glTF routes it through a vertex-colour mix
    (COLOR_0 is a wind weight, never a tint): the first unlinked colour input upstream."""
    sock = bsdf.inputs["Base Color"]
    if not sock.links:
        return tuple(sock.default_value)
    todo, seen = [sock.links[0].from_node], set()
    while todo:
        n = todo.pop(0)
        if n.name in seen:
            continue
        seen.add(n.name)
        if n.type == "RGB":
            return tuple(n.outputs[0].default_value)
        for i in n.inputs:
            if i.type == "RGBA":
                if i.links:
                    todo.append(i.links[0].from_node)
                else:
                    return tuple(i.default_value)
    return (1, 1, 1, 1)


def toon_material(src, L, glow=0.0, tint=None):
    """An emission material computing the game's toon lighting for the base colour of `src`."""
    m = bpy.data.materials.new("toon_" + src.name)
    m.use_nodes = True
    nt = m.node_tree
    nt.nodes.clear()
    bsdf = next((n for n in src.node_tree.nodes if n.type == "BSDF_PRINCIPLED"), None) if src.use_nodes else None
    N = lambda t, **kw: nt.nodes.new(t)  # noqa: E731
    link = nt.links.new
    # base colour: the source's colour, or its image texture
    tex_src = bsdf.inputs["Base Color"].links[0].from_node if bsdf and bsdf.inputs["Base Color"].links else None
    if tex_src is not None and tex_src.type == "TEX_IMAGE":
        tex = N("ShaderNodeTexImage")
        tex.image = tex_src.image
        uvs = tex_src.inputs["Vector"].links
        if uvs and uvs[0].from_node.type == "MAPPING":
            mp = N("ShaderNodeMapping")
            for k in ("Location", "Rotation", "Scale"):
                mp.inputs[k].default_value = uvs[0].from_node.inputs[k].default_value
            tc = N("ShaderNodeTexCoord")
            link(tc.outputs["UV"], mp.inputs["Vector"])
            link(mp.outputs["Vector"], tex.inputs["Vector"])
        base = tex.outputs["Color"]
    else:
        col = N("ShaderNodeRGB")
        c = tint or (base_factor(bsdf) if bsdf else tuple(src.diffuse_color))
        col.outputs[0].default_value = (*c[:3], 1)
        base = col.outputs[0]
    alpha = bsdf.inputs["Alpha"].default_value if bsdf else 1.0
    fx, fy, fz = L["from_hero"]
    ldir = Vector((fx, -fz, fy)).normalized()  # three (x, y, z) -> Blender (x, -z, y)
    geo = N("ShaderNodeNewGeometry")
    dot = N("ShaderNodeVectorMath")
    dot.operation = "DOT_PRODUCT"
    dot.inputs[1].default_value = ldir
    link(geo.outputs["Normal"], dot.inputs[0])
    coord = N("ShaderNodeMath")
    coord.operation = "MULTIPLY_ADD"
    coord.inputs[1].default_value = 0.5
    coord.inputs[2].default_value = 0.5
    link(dot.outputs["Value"], coord.inputs[0])
    ramp = N("ShaderNodeValToRGB")
    ramp.color_ramp.interpolation = "CONSTANT"
    els = ramp.color_ramp.elements
    els[0].position, els[0].color = 0.0, (90 / 255,) * 3 + (1,)
    els[1].position, els[1].color = 0.25, (160 / 255,) * 3 + (1,)
    for p, v in ((0.5, 215), (0.75, 255)):
        e = els.new(p)
        e.color = (v / 255,) * 3 + (1,)
    link(coord.outputs[0], ramp.inputs["Fac"])
    # shadow: a white diffuse lit only by the sun (strength pi -> N.L x visibility), divided by N.L
    dif = N("ShaderNodeBsdfDiffuse")
    dif.inputs["Color"].default_value = (1, 1, 1, 1)
    s2r = N("ShaderNodeShaderToRGB")
    link(dif.outputs[0], s2r.inputs[0])
    bw = N("ShaderNodeRGBToBW")
    link(s2r.outputs["Color"], bw.inputs[0])
    dmax = N("ShaderNodeMath")
    dmax.operation = "MAXIMUM"
    dmax.inputs[1].default_value = 0.03
    link(dot.outputs["Value"], dmax.inputs[0])
    vis = N("ShaderNodeMath")
    vis.operation = "DIVIDE"
    link(bw.outputs[0], vis.inputs[0])
    link(dmax.outputs[0], vis.inputs[1])
    vis2 = N("ShaderNodeMapRange")
    vis2.inputs["From Min"].default_value, vis2.inputs["From Max"].default_value = 0.35, 0.65
    vis2.clamp = True
    link(vis.outputs[0], vis2.inputs["Value"])
    sun_c, sun_i = L["sun"]
    direct = N("ShaderNodeMixRGB")
    direct.blend_type = "MULTIPLY"
    direct.inputs["Fac"].default_value = 1.0
    link(ramp.outputs["Color"], direct.inputs[1])
    direct.inputs[2].default_value = (*[c * sun_i / math.pi for c in hexlin(sun_c)], 1)
    dshadow = N("ShaderNodeVectorMath")
    dshadow.operation = "SCALE"
    link(direct.outputs[0], dshadow.inputs[0])
    link(vis2.outputs[0], dshadow.inputs["Scale"])
    # hemisphere
    sky, gnd, hi = L["hemi"]
    sep = N("ShaderNodeSeparateXYZ")
    link(geo.outputs["Normal"], sep.inputs[0])
    hz = N("ShaderNodeMath")
    hz.operation = "MULTIPLY_ADD"
    hz.inputs[1].default_value = 0.5
    hz.inputs[2].default_value = 0.5
    link(sep.outputs["Z"], hz.inputs[0])
    hemi = N("ShaderNodeMixRGB")
    link(hz.outputs[0], hemi.inputs["Fac"])
    hemi.inputs[1].default_value = (*[c * hi / math.pi for c in hexlin(gnd)], 1)
    hemi.inputs[2].default_value = (*[c * hi / math.pi for c in hexlin(sky)], 1)
    light = N("ShaderNodeVectorMath")
    light.operation = "ADD"
    link(dshadow.outputs[0], light.inputs[0])
    link(hemi.outputs[0], light.inputs[1])
    if glow:
        g = N("ShaderNodeVectorMath")
        g.operation = "ADD"
        link(light.outputs[0], g.inputs[0])
        g.inputs[1].default_value = (glow, glow, glow)
        light = g
    lit = N("ShaderNodeVectorMath")
    lit.operation = "MULTIPLY"
    link(base, lit.inputs[0])
    link(light.outputs[0], lit.inputs[1])
    # linear fog toward the background, on view depth
    cam = N("ShaderNodeCameraData")
    near, far = L["fog"]
    fog = N("ShaderNodeMapRange")
    fog.inputs["From Min"].default_value, fog.inputs["From Max"].default_value = near, far
    fog.clamp = True
    link(cam.outputs["View Z Depth"], fog.inputs["Value"])
    mix = N("ShaderNodeMixRGB")
    link(fog.outputs[0], mix.inputs["Fac"])
    link(lit.outputs[0], mix.inputs[1])
    mix.inputs[2].default_value = (*hexlin(L["bg"]), 1)
    em = N("ShaderNodeEmission")
    link(mix.outputs[0], em.inputs["Color"])
    out = N("ShaderNodeOutputMaterial")
    if alpha < 0.999:
        tr = N("ShaderNodeBsdfTransparent")
        ms = N("ShaderNodeMixShader")
        ms.inputs["Fac"].default_value = alpha
        link(tr.outputs[0], ms.inputs[1])
        link(em.outputs[0], ms.inputs[2])
        link(ms.outputs[0], out.inputs["Surface"])
        for attr, val in (("surface_render_method", "BLENDED"), ("blend_method", "BLEND")):
            try:
                setattr(m, attr, val)
            except (AttributeError, TypeError):
                pass
    else:
        link(em.outputs[0], out.inputs["Surface"])
    return m


_ORIG = {}


def toonify(L, night=False, tints=None):
    """Swap every material for its toon twin (glow parts emissive at night). Always maps from the source
    materials (remembered on the first call), so it can run again with other light."""
    for m in [m for m in bpy.data.materials if m.name.startswith("toon_")]:
        m.name = "old_" + m.name
    cache = {}
    for ob in bpy.data.objects:
        if ob.type != "MESH":
            continue
        for i, slot in enumerate(ob.material_slots):
            src = _ORIG.setdefault((ob.name, i), slot.material)
            if src is None or src.name.startswith("label_"):
                continue
            if src.name not in cache:
                tint = None
                for k, v in (tints or {}).items():
                    if k in src.name:
                        tint = hexlin(v)
                cache[src.name] = toon_material(src, L, glow=(1.6 if night and "glow" in src.name else 0.0), tint=tint)
            slot.material = cache[src.name]


def stage(scn, L, res=(1800, 1200)):
    """Sun (only for the shadow term), background, EEVEE settings; returns the camera."""
    fx, fy, fz = L["from_hero"]
    d = Vector((fx, -fz, fy)).normalized()
    sun = bpy.data.objects.new("v13_sun", bpy.data.lights.new("v13_sun", "SUN"))
    sun.data.energy = math.pi
    sun.data.angle = math.radians(0.5)
    sun.rotation_euler = (-d).to_track_quat("-Z", "Y").to_euler()
    scn.collection.objects.link(sun)
    world = bpy.data.worlds.new("v13_world")
    world.use_nodes = True
    bg = next(n for n in world.node_tree.nodes if n.type == "BACKGROUND")
    bg.inputs["Color"].default_value = (*hexlin(L["bg"]), 1)
    bg.inputs["Strength"].default_value = 1.0
    scn.world = world
    scn.view_settings.view_transform = "Standard"
    scn.view_settings.look = "None"
    scn.render.image_settings.file_format = "PNG"
    for eng in ("BLENDER_EEVEE", "BLENDER_EEVEE_NEXT"):
        try:
            scn.render.engine = eng
            break
        except TypeError:
            continue
    try:
        scn.eevee.taa_render_samples = 16
        scn.eevee.use_shadows = True
    except AttributeError:
        pass
    scn.render.resolution_x, scn.render.resolution_y = res
    cam = bpy.data.objects.new("v13_cam", bpy.data.cameras.new("v13_cam"))
    scn.collection.objects.link(cam)
    scn.camera = cam
    return cam


def game_camera(cam, aim, dist, pitch=50, lens=30):
    ax, ay, az = aim
    cam.data.type = "PERSP"
    cam.data.lens = lens
    cam.data.clip_end = 400
    cam.rotation_euler = (math.radians(90 - pitch), 0, 0)
    cam.location = (ax, ay - dist * math.cos(math.radians(pitch)), az + dist * math.sin(math.radians(pitch)))


def import_glb(path):
    """Import a .glb once into its own (unlinked) collection, for collection instances."""
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=path)
    new = [o for o in bpy.data.objects if o not in before]
    for o in [o for o in new if o.type == "MESH" and o.name.startswith("Icosphere")]:  # importer bone shapes
        new.remove(o)
        bpy.data.objects.remove(o)
    coll = bpy.data.collections.new("glb_" + os.path.splitext(os.path.basename(path))[0])
    for o in new:
        for c in list(o.users_collection):
            c.objects.unlink(o)
        coll.objects.link(o)
    return coll


def label(text, size, mat_hex="#2a2238"):
    cu = bpy.data.curves.new("label", "FONT")
    cu.body = text
    cu.size = size
    ob = bpy.data.objects.new("label", cu)
    m = bpy.data.materials.new("label_mat")
    m.use_nodes = True
    nt = m.node_tree
    nt.nodes.clear()
    em = nt.nodes.new("ShaderNodeEmission")
    em.inputs["Color"].default_value = (*hexlin(mat_hex), 1)
    o = nt.nodes.new("ShaderNodeOutputMaterial")
    nt.links.new(em.outputs[0], o.inputs["Surface"])
    cu.materials.append(m)
    return ob


def render(scn, path):
    scn.render.filepath = path
    bpy.ops.render.render(write_still=True, scene=scn.name)
    print("[v13] render ->", path)


FRONT_BACK = (((0.55, -0.75, 0.75), "front"), ((-0.55, 0.75, 0.75), "back"))


def tile_sheet(scn, L, built, info, path, tile=360, cols=6, night=False, views=FRONT_BACK):
    """One labelled tile per model and view (default: front 3/4 from the game pitch, then from behind), ortho,
    fitted. views: [(direction toward the camera, tag)]; the first tile of each model carries its info line,
    the others their tag. night: the 1.3 night preset with glow parts lit."""
    import numpy as np
    for ob in [o for o in scn.objects if o.type == "LIGHT" or o.name.startswith("v13_cam")]:
        bpy.data.objects.remove(ob)
    if night:
        L = dict(L, **NIGHT)
        L.update(L.get("night", {}))
    L = dict(L, fog=(1e4, 2e4))  # the ortho camera stands far off: no fog on the tiles
    cam = stage(scn, L, (tile, tile))
    cam.data.type = "ORTHO"
    toonify(L, night=night)
    tmp = os.path.join(os.path.dirname(os.path.abspath(path)), f"_v13_tile_{os.getpid()}.png")  # several runs may share a folder
    tiles = []
    meshes = [o for o in scn.objects if o.type == "MESH"]
    for name, objs in built.items():
        pts = []
        for o in objs:
            if o.type == "MESH":
                pts += [o.matrix_world @ Vector(c) for c in o.bound_box]
        lo = Vector([min(p[i] for p in pts) for i in range(3)])
        hi = Vector([max(p[i] for p in pts) for i in range(3)])
        c, size = (lo + hi) / 2, (hi - lo).length
        mine = {o for o in objs}
        for o in meshes:
            o.hide_render = o not in mine
        for vi, (d, tag) in enumerate(views):
            d = Vector(d).normalized()
            cam.data.ortho_scale = size * 1.08
            cam.location = c + d * 40
            cam.rotation_euler = (-d).to_track_quat("-Z", "Y").to_euler()
            cam.data.clip_end = 100
            lab = None
            if vi == 0 or views is not FRONT_BACK:
                lab = label(info[name] if vi == 0 else tag, cam.data.ortho_scale * 0.05, "#d8dcef" if night else "#2a2238")
                scn.collection.objects.link(lab)
                lab.parent = cam
                lab.matrix_parent_inverse = Matrix.Identity(4)
                s = cam.data.ortho_scale
                lab.location = (-s / 2 + s * 0.03, -s / 2 + s * 0.03, -1.0)
            render(scn, tmp)
            if lab:
                bpy.data.objects.remove(lab)
            img = bpy.data.images.load(tmp)
            tiles.append(np.array(img.pixels[:], dtype=np.float32).reshape(tile, tile, 4))
            bpy.data.images.remove(img)
    for o in meshes:
        o.hide_render = False
    os.remove(tmp)
    rows = [tiles[i:i + cols] for i in range(0, len(tiles), cols)]
    if rows and len(rows[-1]) < cols:
        rows[-1] += [np.ones((tile, tile, 4), np.float32)] * (cols - len(rows[-1]))
    px = np.concatenate([np.concatenate(r, axis=1) for r in reversed(rows)], axis=0)
    img = bpy.data.images.new("v13_sheet", px.shape[1], px.shape[0])
    img.pixels.foreach_set(px.ravel())
    img.filepath_raw = path
    img.file_format = "PNG"
    img.save()
    print("[v13] sheet ->", path)


# ---------------------------------------------------------------- vignette: a tiny map built like build.js does

# ground.js REPEAT (metres per texture tile); the 1.3 textures suggest their own (see ASSETS_V13.md)
REPEAT = {"grass": 6, "dirt": 3, "wood_floor": 2.4, "sand": 5, "stone": 2.5,
          "ash": 5, "heath": 6, "wet_sand": 5, "dusk_grass": 6, "moss": 5, "slate": 3, "cave": 4}
AUTOTILE = [("", 0), ("_end", 270), ("_end", 180), ("_corner", 180), ("_end", 90), ("", 90), ("_corner", 90), ("_t", 90),
            ("_end", 0), ("_corner", 270), ("", 0), ("_t", 180), ("_corner", 0), ("_t", 270), ("_t", 0), ("_t", 0)]


def vignette(scn, L, V, assets_dirs, foot, path_night=None, path_day=None, tints=None):
    """Build V (tiles, props, cliff set, fence set, ring) from exported .glb files and render it day and night.
    Tile chars as in the maps: . base, p path, s sand, ~ water, = boardwalk, 1-9 cliff stack, f fence set."""
    rng = random.Random(13)
    tex = os.path.join(HERE, "..", "..", "web", "world", "textures")
    cache = {}

    def coll(name):
        if name not in cache:
            for d in assets_dirs:
                p = os.path.join(d, name + ".glb")
                if os.path.exists(p):
                    cache[name] = import_glb(p)
                    break
            else:
                print("[v13] vignette: no", name)
                cache[name] = None
        return cache[name]

    host = bpy.data.collections.new("v13_vignette")
    scn.collection.children.link(host)

    def place(name, cx, cz, rot=0.0, y=0.0, s=1.0):
        c = coll(name)
        if c is None:
            return
        e = bpy.data.objects.new("i_" + name, None)
        e.instance_type = "COLLECTION"
        e.instance_collection = c
        e.location = (cx, -cz, y)
        e.rotation_euler = (0, 0, math.radians(rot))
        e.scale = (s, s, s)
        host.objects.link(e)

    tiles = V["tiles"]
    H, W = len(tiles), len(tiles[0])

    def ground_mat(name, fallback):
        m = bpy.data.materials.new("gnd_" + name)
        m.use_nodes = True
        bsdf = next(n for n in m.node_tree.nodes if n.type == "BSDF_PRINCIPLED")
        p = None
        for d in V.get("texdirs", []) + [tex]:
            for ext in (".png", ".webp"):
                if p is None and os.path.exists(os.path.join(d, name + ext)):
                    p = os.path.join(d, name + ext)
        if p:
            t = m.node_tree.nodes.new("ShaderNodeTexImage")
            t.image = bpy.data.images.load(p)
            mp = m.node_tree.nodes.new("ShaderNodeMapping")
            rep = REPEAT.get(name, 4)
            mp.inputs["Scale"].default_value = (1 / rep, 1 / rep, 1)   # metres per repeat, as ground.js
            tc = m.node_tree.nodes.new("ShaderNodeTexCoord")
            m.node_tree.links.new(tc.outputs["UV"], mp.inputs["Vector"])
            m.node_tree.links.new(mp.outputs["Vector"], t.inputs["Vector"])
            m.node_tree.links.new(t.outputs["Color"], bsdf.inputs["Base Color"])
        else:
            bsdf.inputs["Base Color"].default_value = B.lin(fallback)
        return m

    def quad(x0, z0, x1, z1, h, mat):
        me = bpy.data.meshes.new("q")
        vs = [(x0, -z0, h), (x1, -z0, h), (x1, -z1, h), (x0, -z1, h)]
        me.from_pydata(vs, [], [(0, 3, 2, 1)])
        me.uv_layers.new()
        for lp in me.loops:
            co = me.vertices[lp.vertex_index].co
            me.uv_layers[0].data[lp.index].uv = (co.x, co.y)
        me.materials.append(mat)
        ob = bpy.data.objects.new("q", me)
        host.objects.link(ob)
        return ob

    m = V.get("margin", 10)
    base = ground_mat(V["ground"][0], V["ground"][2] if len(V["ground"]) > 2 else "#8a8a7a")
    quad(-m, -m, W + m, H + m, 0.0, base)
    pathm = ground_mat(V["ground"][1], "#b9a98c")
    extra = {k: ground_mat(v, "#c9b48a") for k, v in V.get("overlays", {}).items()}
    water = bpy.data.materials.new("water_v13")
    water.use_nodes = True
    next(n for n in water.node_tree.nodes if n.type == "BSDF_PRINCIPLED").inputs["Base Color"].default_value = B.lin(V.get("water", "#4fb3c4"))
    cliff, cliff_b = V.get("cliff", ("cliff", "cliff_b"))
    fence = V.get("fence", "fence")
    for z, row in enumerate(tiles):
        for x, ch in enumerate(row):
            if ch == "p":
                quad(x, z, x + 1, z + 1, 0.003, pathm)
            elif ch in extra:
                quad(x, z, x + 1, z + 1, 0.002, extra[ch])
            elif ch in "~H":
                quad(x, z, x + 1, z + 1, 0.004, water)
                if ch == "H":
                    place("pier", x + 0.5, z + 0.5)
            elif ch == "=":
                place("boardwalk", x + 0.5, z + 0.5)
            elif ch.isdigit():
                for k in range(int(ch)):
                    n = cliff_b if k == int(ch) - 1 and rng.random() < 0.35 else cliff
                    place(n, x + 0.5, z + 0.5, 90 * rng.randrange(4), k)
            elif ch == "f":
                at = lambda xx, zz: 0 <= zz < H and 0 <= xx < W and tiles[zz][xx] == "f"  # noqa: E731
                mask = sum(1 << i for i, (dx, dz) in enumerate(((0, -1), (1, 0), (0, 1), (-1, 0))) if at(x + dx, z + dz))
                for sfx, deg in [AUTOTILE[mask]] + ([("_end", 270)] if mask == 15 else []):
                    place(fence + sfx, x + 0.5, z + 0.5, deg)
    if any(ch in "~H" for row in tiles for ch in row) and V.get("sea_beyond"):  # the sea carries on past the edge
        side = V["sea_beyond"]
        if side == "s":
            quad(-m, H, W + m, H + m, 0.004, water)
        elif side == "e":
            quad(W, -m, W + m, H + m, 0.004, water)
        elif side == "n":
            quad(-m, -m, W + m, 0, 0.004, water)
    for name, x, z, *rest in V["props"]:
        rot = rest[0] if rest else 0
        y = rest[1] if len(rest) > 1 else 0
        fw, fd = foot.get(name, (1, 1))
        if rot % 180 == 90:
            fw, fd = fd, fw
        place(name, x + fw / 2, z + fd / 2, rot, y)
    ring = V.get("ring", [])
    if ring:
        xx = -3.0
        while xx < W + 3:
            place(rng.choice(ring), xx, -1.6 - rng.random() * 2.0, rng.random() * 360, 0, 1.05 + rng.random() * 0.4)
            xx += 1.4 + rng.random() * 0.6
        for side_x in (-1.8, W + 1.8):
            zz = -1.0
            while zz < H + 2:
                place(rng.choice(ring), side_x + rng.uniform(-0.6, 0.6), zz, rng.random() * 360, 0, 1.0 + rng.random() * 0.4)
                zz += 1.5 + rng.random() * 0.6
    aim, dist, lens = V.get("aim", (W / 2, -H / 2 - 1, 0)), V.get("dist", 22), V.get("lens", 30)
    night_light = dict(L, **NIGHT)
    night_light.update(L.get("night", {}))  # a region's own light_night (e.g. Dream's brighter nights)
    for p, LL, night in ((path_day, L, False), (path_night, night_light, True)):
        if not p:
            continue
        for ob in [o for o in scn.objects if o.type == "LIGHT" or o.name.startswith("v13_cam")]:
            bpy.data.objects.remove(ob)
        cam = stage(scn, LL)
        game_camera(cam, aim, dist, lens=lens)
        toonify(LL, night=night, tints=tints)
        render(scn, p)
