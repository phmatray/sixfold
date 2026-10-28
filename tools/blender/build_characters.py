"""Every person in the world: the hero, the townsfolk archetypes and the seven rivals (headless only).

    blender -b --factory-startup -P tools/blender/build_characters.py -- --out web/world/assets [--only hero,npc_kid]

Writes web/world/assets/hero.glb and web/world/assets/npc/{npc,rival}_*.glb. All designs are original.

The look ("tabletop miniature"): every garment is one sculpted surface per bone (primitives fused by a voxel
remesh, smoothed, decimated), faceted on hair, hats and coats, smooth on skin. Big hands, boots, hats and
props, so each trade reads from its silhouette at about 70 CSS px. Faces are painted decals (sclera, iris,
highlight) wrapped on an ellipsoid head.

How a model is made:
- Each character is authored in the hero's canonical body (neck at 0.95 m) and then mapped to its own
  build: `bz` scales the body's height, `bw` its width, `hs` the head (about the neck), `leg` the legs.
- One palette texture per character (4 px swatches on an 8 x 8 grid, nearest filtering): every face's UVs
  sit in its swatch, so the whole body is one mesh, one material, one draw. The material's extras carry the
  swatch table (`sx_palette`), which the game uses to recolour clothes (`outfit`) and hair (`hair`).
  Swatch names follow the material rules of web/world/actors.js: skin, eye, white, blush, sole, gold,
  cream, hair, trim and glow keep their colour under `outfit`.
- Glowing parts (a lantern) are a second, flat material named `glow`: the only second draw.
- Accessories for route duelists (`acc_cap`, `acc_glasses`, `acc_headband`, `acc_scarf`, `acc_bag`) are extra
  meshes on the same skeleton and palette, hidden unless the map asks for them (`wear`).
- Rig: Root, Hips, Torso, Head, Eye.L/R (blink), Leg.L/R + Shin.L/R (knees), Arm.L/R + Fore.L/R (elbows),
  rigid weights. Clips: Idle (4 s: breaths, weight shift, a blink), Fidget (2.5 s look-around), Walk (0.5 s,
  two 0.25 s steps, one tile each), Run (hero only; two 0.17 s steps, the game's run speed), Sit (8 s seated
  loop: benches and chairs, web/world/seat.js), SitDown and StandUp (hero only; 0.6 s each way), Reach,
  HoldBook and PutBack (hero only: a book off a shelf, read, put back).
"""
import json
import math
import os
import sys

import bmesh
import bpy
from mathutils import Euler, Matrix, Vector

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import build_assets as ba  # noqa: E402
from build_assets import T, box, cyl, sphere  # noqa: E402

FPS = ba.FPS
CELL, GRID = 4, 8  # palette swatch size (px) and grid (cells per side)

# ---------------------------------------------------------------- canonical body (the hero's proportions)
HIP, KNEE, SH, EL, NECK, SX, HX = 0.54, 0.28, 0.86, 0.66, 0.95, 0.19, 0.085
HEAD = (0, 0, 1.17, 0.2, 0.185, 0.225)  # ellipsoid: centre, radii
V = 0.011  # voxel size for fused parts


# ---------------------------------------------------------------- shapes

def lathe(profile, seg=12, phase=0.0, sx=1.0, sy=1.0):
    """Surface of revolution about Z from (r, z) points bottom to top; r = 0 closes to a point."""
    bm = bmesh.new()
    rings = []
    for r, z in profile:
        if r <= 1e-6:
            rings.append([bm.verts.new((0, 0, z))])
        else:
            rings.append([bm.verts.new((sx * r * math.cos(math.radians(phase) + a), sy * r * math.sin(math.radians(phase) + a), z))
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


def tube(pts, r, sides=6, taper=1.0, cap=True):
    bm = bmesh.new()
    pts = [Vector(p) for p in pts]
    rows, n, up = [], len(pts), Vector((0, 0, 1))
    for i, p in enumerate(pts):
        t = (pts[min(i + 1, n - 1)] - pts[max(i - 1, 0)]).normalized()
        side = t.cross(up)
        if side.length < 1e-4:
            side = t.cross(Vector((1, 0, 0)))
        side.normalize()
        nrm = side.cross(t).normalized()
        rr = r * (1 + (taper - 1) * i / max(n - 1, 1))
        rows.append([bm.verts.new(p + (side * math.cos(a) + nrm * math.sin(a)) * rr) for a in (j / sides * math.tau for j in range(sides))])
    for a, b in zip(rows, rows[1:]):
        for j in range(sides):
            k = (j + 1) % sides
            bm.faces.new((a[j], a[k], b[k], b[j]))
    if cap:
        bm.faces.new(rows[0][::-1])
        bm.faces.new(rows[-1])
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return bm


def torus(R, r, seg=14, sides=6):
    bm = bmesh.new()
    rows = [[bm.verts.new(((R + r * math.cos(b)) * math.cos(a), (R + r * math.cos(b)) * math.sin(a), r * math.sin(b)))
             for b in (j / sides * math.tau for j in range(sides))] for a in (i / seg * math.tau for i in range(seg))]
    for i in range(seg):
        for j in range(sides):
            bm.faces.new((rows[i][j], rows[(i + 1) % seg][j], rows[(i + 1) % seg][(j + 1) % sides], rows[i][(j + 1) % sides]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return bm


def cut(bm, co, no):
    """Drop the side of the plane that `no` points to."""
    bmesh.ops.bisect_plane(bm, geom=list(bm.verts) + list(bm.edges) + list(bm.faces), plane_co=co, plane_no=no, clear_outer=True)
    return bm


def decal(w, h, seg=10):
    """A flat ellipse in the XZ plane facing -Y (eyes, cheeks)."""
    bm = bmesh.new()
    c = bm.verts.new((0, 0, 0))
    ring = [bm.verts.new((w / 2 * math.cos(a), 0, h / 2 * math.sin(a))) for a in (i / seg * math.tau for i in range(seg))]
    for i in range(seg):
        bm.faces.new((c, ring[(i + 1) % seg], ring[i]))
    _face_front(bm)
    return bm


def poly_decal(pts_xz):
    """A flat polygon in the XZ plane facing -Y (brows, mouths, lids)."""
    bm = bmesh.new()
    bm.faces.new([bm.verts.new((x, 0, z)) for x, z in pts_xz])
    bmesh.ops.triangulate(bm, faces=bm.faces[:])
    _face_front(bm)
    return bm


def _face_front(bm):
    for f in bm.faces:
        f.normal_update()
        if f.normal.y > 0:
            f.normal_flip()


def paint(bm, x, z, layer=0.0, head=HEAD):
    """Move a decal to (x, z) and wrap it onto the front of the head ellipsoid, 3 mm proud (+layer)."""
    cx, cy, cz, rx, ry, rz = head
    for v in bm.verts:
        px, pz = v.co.x + x, v.co.z + z
        k = 1 - ((px - cx) / rx) ** 2 - ((pz - cz) / rz) ** 2
        v.co = Vector((px, cy - ry * math.sqrt(max(k, 0.0)) - 0.003 - layer, pz))
    return bm


def _decimate(bm, ratio):
    me = bpy.data.meshes.new("dec")
    bm.to_mesh(me)
    ob = bpy.data.objects.new("dec", me)
    bpy.context.scene.collection.objects.link(ob)
    ob.modifiers.new("d", "DECIMATE").ratio = ratio
    me2 = bpy.data.meshes.new_from_object(ob.evaluated_get(bpy.context.evaluated_depsgraph_get()))
    out = bmesh.new()
    out.from_mesh(me2)
    bpy.data.objects.remove(ob)
    bpy.data.meshes.remove(me)
    bpy.data.meshes.remove(me2)
    bm.free()
    return out


def fuse(parts, target=300, voxel=V, smooth=4):
    """Primitives -> one sculpted surface: voxel remesh (a union), smooth, decimate to `target` triangles.
    parts: [(bmesh, matrix or None)]."""
    out = bmesh.new()
    for bm, M in parts:
        if M is not None:
            bmesh.ops.transform(bm, matrix=M, verts=bm.verts)
        me = bpy.data.meshes.new("p")
        bm.to_mesh(me)
        out.from_mesh(me)
        bpy.data.meshes.remove(me)
        bm.free()
    me = bpy.data.meshes.new("fuse")
    out.to_mesh(me)
    out.free()
    ob = bpy.data.objects.new("fuse", me)
    bpy.context.scene.collection.objects.link(ob)
    r = ob.modifiers.new("r", "REMESH")
    r.mode, r.voxel_size, r.adaptivity = "VOXEL", voxel, 0.0
    if smooth:
        s = ob.modifiers.new("s", "SMOOTH")
        s.iterations, s.factor = smooth, 0.6
    me2 = bpy.data.meshes.new_from_object(ob.evaluated_get(bpy.context.evaluated_depsgraph_get()))
    res = bmesh.new()
    res.from_mesh(me2)
    bpy.data.objects.remove(ob)
    bpy.data.meshes.remove(me)
    bpy.data.meshes.remove(me2)
    bmesh.ops.triangulate(res, faces=res.faces[:])
    if len(res.faces) > target:
        res = _decimate(res, target / len(res.faces))
    return res


def solid(bm, thickness=0.02):
    """Give an open surface (a hood, a brim) a thickness, so it shows from both sides."""
    me = bpy.data.meshes.new("sol")
    bm.to_mesh(me)
    bm.free()
    ob = bpy.data.objects.new("sol", me)
    bpy.context.scene.collection.objects.link(ob)
    m = ob.modifiers.new("s", "SOLIDIFY")
    m.thickness, m.offset = thickness, -1.0
    me2 = bpy.data.meshes.new_from_object(ob.evaluated_get(bpy.context.evaluated_depsgraph_get()))
    out = bmesh.new()
    out.from_mesh(me2)
    bpy.data.objects.remove(ob)
    bpy.data.meshes.remove(me)
    bpy.data.meshes.remove(me2)
    return out


def sp(r, loc, scale=(1, 1, 1), rot=(0, 0, 0)):
    return (sphere(r, 14, 10), T(loc, rot, scale))


def cy(r_bottom, r_top, z0, z1, x=0.0, y=0.0, seg=14):
    return (cyl(r_bottom, r_top, z1 - z0, seg), T((x, y, (z0 + z1) / 2)))


def limb(a, b, r1, r2, seg=12):
    """A tapered capsule from a to b."""
    a, b = Vector(a), Vector(b)
    return [(tube([a.lerp(b, t / 4) for t in range(5)], r1, seg, r2 / r1), None), (sphere(r1, seg, 8), T(tuple(a))), (sphere(r2, seg, 8), T(tuple(b)))]


# ---------------------------------------------------------------- the kit: parts, palette, one mesh per group

class Char:
    """Collects parts (bmesh, palette key, bone, smooth, group) in canonical space, then maps them to the
    character's build and makes one mesh per group (`body`, `acc_*`) on a shared palette material."""

    def __init__(self, name, pal, bz=1.0, bw=1.0, hs=1.0, leg=1.0):
        self.name, self.pal, self.parts = name, dict(pal), []
        self.bz, self.bw, self.hs, self.leg = bz, bw, hs, leg
        self.group = "body"

    def add(self, bm, key, bone, M=None, smooth=True):
        if M is not None:
            bmesh.ops.transform(bm, matrix=M, verts=bm.verts)
        if key not in self.pal:
            raise KeyError(f"{self.name}: no colour for {key}")
        self.parts.append((bm, key, bone, smooth, self.group))

    def fuse(self, key, bone, parts, target=300, smooth=True, voxel=V):
        self.add(fuse(parts, target, voxel), key, bone, smooth=smooth)

    # canonical -> this character
    def z(self, z):
        return z * self.bz * self.leg if z <= HIP else z * self.bz + HIP * self.bz * (self.leg - 1)

    def body_pt(self, v):
        return Vector((v.x * self.bw, v.y * self.bw, self.z(v.z)))

    def head_pt(self, v):
        return Vector((0, 0, self.z(NECK))) + (v - Vector((0, 0, NECK))) * self.hs

    def skeleton(self):
        z = self.z
        ex, ey, ez = 0.078, -0.17, 1.165
        e = self.head_pt(Vector((ex, ey, ez)))
        return {
            "Root": ((0, 0, 0), (0, 0, 0.12), None),
            "Hips": ((0, 0, z(HIP)), (0, 0, z(HIP) + 0.1), "Root"),
            "Torso": ((0, 0, z(HIP) + 0.04), (0, 0, z(NECK)), "Hips"),
            "Head": ((0, 0, z(NECK)), (0, 0, z(NECK) + 0.3), "Torso"),
            "Leg.L": ((HX * self.bw, 0, z(HIP) - 0.02), (HX * self.bw, 0, z(HIP) + 0.08), "Hips"),
            "Leg.R": ((-HX * self.bw, 0, z(HIP) - 0.02), (-HX * self.bw, 0, z(HIP) + 0.08), "Hips"),
            "Shin.L": ((HX * self.bw, 0, z(KNEE)), (HX * self.bw, 0, z(KNEE) + 0.1), "Leg.L"),
            "Shin.R": ((-HX * self.bw, 0, z(KNEE)), (-HX * self.bw, 0, z(KNEE) + 0.1), "Leg.R"),
            "Arm.L": ((SX * self.bw, 0, z(SH)), (SX * self.bw, 0, z(SH) + 0.1), "Torso"),
            "Arm.R": ((-SX * self.bw, 0, z(SH)), (-SX * self.bw, 0, z(SH) + 0.1), "Torso"),
            "Fore.L": ((SX * self.bw, 0, z(EL)), (SX * self.bw, 0, z(EL) + 0.1), "Arm.L"),
            "Fore.R": ((-SX * self.bw, 0, z(EL)), (-SX * self.bw, 0, z(EL) + 0.1), "Arm.R"),
            "Eye.L": ((e.x, e.y, e.z), (e.x, e.y, e.z + 0.05), "Head"),
            "Eye.R": ((-e.x, e.y, e.z), (-e.x, e.y, e.z + 0.05), "Head"),
        }

    def build(self, coll):
        keys = []
        for _, key, _, _, _ in self.parts:
            if key not in keys and "glow" not in key:
                keys.append(key)
        if len(keys) > GRID * GRID:
            raise ValueError(f"{self.name}: {len(keys)} colours, the palette holds {GRID * GRID}")
        pal_mat = palette_material(self.name, keys, self.pal)
        glows = {}
        groups = {}
        for bm, key, bone, smooth, group in self.parts:
            head = bone in ("Head", "Eye.L", "Eye.R")
            for v in bm.verts:
                v.co = self.head_pt(v.co) if head else self.body_pt(v.co)
            groups.setdefault(group, []).append((bm, key, bone, smooth))
        objs = {}
        for group, parts in groups.items():
            out = bmesh.new()
            dl = out.verts.layers.deform.verify()
            uvl = out.loops.layers.uv.verify()
            mats, bones = [pal_mat], []
            for bm, key, bone, smooth in parts:
                if "glow" in key:
                    if key not in glows:
                        glows[key] = flat_material(key, self.pal[key])
                    if glows[key] not in mats:
                        mats.append(glows[key])
                    mi, uv = mats.index(glows[key]), (0.5, 0.5)
                else:
                    i = keys.index(key)
                    mi, uv = 0, ((i % GRID + 0.5) / GRID, (i // GRID + 0.5) / GRID)
                if bone not in bones:
                    bones.append(bone)
                vmap = {}
                for v in bm.verts:
                    nv = out.verts.new(v.co)
                    nv[dl][bones.index(bone)] = 1.0
                    vmap[v] = nv
                for f in bm.faces:
                    try:
                        nf = out.faces.new([vmap[v] for v in f.verts])
                    except ValueError:  # a part that repeats a face (a degenerate lathe end): keep one
                        continue
                    nf.material_index = mi
                    nf.smooth = smooth
                    for lp in nf.loops:
                        lp[uvl].uv = uv
                bm.free()
            bmesh.ops.triangulate(out, faces=out.faces[:])
            me = bpy.data.meshes.new(ba.PREFIX + self.name + ("" if group == "body" else "_" + group))
            out.to_mesh(me)
            out.free()
            for m in mats:
                me.materials.append(m)
            ob = bpy.data.objects.new(self.name if group == "body" else group, me)
            for b in bones:
                ob.vertex_groups.new(name=b)
            coll.objects.link(ob)
            objs[group] = ob
        return objs


def srgb(hexstr):
    return tuple(int(hexstr.lstrip("#")[i:i + 2], 16) / 255 for i in (0, 2, 4))


def palette_material(name, keys, pal):
    """One material with a GRID x GRID swatch texture (sRGB, nearest), swatch table in its extras."""
    size = CELL * GRID
    img = bpy.data.images.new(ba.PREFIX + name + "_palette", size, size, alpha=False)
    px = [0.0] * (size * size * 4)
    for i, key in enumerate(keys):
        r, g, b = srgb(pal[key])
        c0, r0 = (i % GRID) * CELL, (i // GRID) * CELL
        for y in range(r0, r0 + CELL):
            for x in range(c0, c0 + CELL):
                j = (y * size + x) * 4
                px[j:j + 4] = (r, g, b, 1.0)
    img.pixels = px
    img.pack()
    m = bpy.data.materials.new(ba.PREFIX + name)
    m.use_nodes = True
    nt = m.node_tree
    bsdf = next(n for n in nt.nodes if n.type == "BSDF_PRINCIPLED")
    bsdf.inputs["Roughness"].default_value = 0.9
    tex = nt.nodes.new("ShaderNodeTexImage")
    tex.image, tex.interpolation = img, "Closest"
    nt.links.new(tex.outputs["Color"], bsdf.inputs["Base Color"])
    m["sx_palette"] = json.dumps({"grid": GRID, "cell": CELL, "keys": {k: [i, pal[k]] for i, k in enumerate(keys)}}, separators=(",", ":"))
    return m


def flat_material(key, hexstr):
    m = bpy.data.materials.new(ba.PREFIX + key)
    m.use_nodes = True
    bsdf = next(n for n in m.node_tree.nodes if n.type == "BSDF_PRINCIPLED")
    bsdf.inputs["Base Color"].default_value = ba.lin(hexstr)
    return m


# ---------------------------------------------------------------- rig and clips

def rig(c, objs, coll, clips, pose=None, hold=None, arm_swing=1.0, stride=1.0):
    """Armature for character c, its meshes skinned to it; `pose` offsets Idle and Fidget, `hold` offsets every
    clip (an arm carrying something keeps carrying it while walking)."""
    arm_data = bpy.data.armatures.new(ba.PREFIX + c.name + "_rig")
    rg = bpy.data.objects.new(c.name + "_rig", arm_data)
    coll.objects.link(rg)
    bpy.context.view_layer.objects.active = rg
    bpy.ops.object.mode_set(mode="EDIT")
    for bname, (h, t, par) in c.skeleton().items():
        eb = arm_data.edit_bones.new(bname)
        eb.head, eb.tail, eb.roll = Vector(h), Vector(t), 0
        if par:
            eb.parent = arm_data.edit_bones[par]
    bpy.ops.object.mode_set(mode="OBJECT")
    for b in arm_data.bones:
        b.use_deform = b.name != "Root"
    for ob in objs.values():
        ob.parent = rg
        ob.modifiers.new("Armature", "ARMATURE").object = rg
    rg.animation_data_create()
    pb = rg.pose.bones
    for b in pb:
        b.rotation_mode = "XYZ"
    hold = hold or {}

    def key(frame, poses, offset):
        for bname, bpose in poses.items():
            b = pb[bname]
            dl, dr = (offset or {}).get(bname, ((0, 0, 0), (0, 0, 0)))
            hl, hr = hold.get(bname, ((0, 0, 0), (0, 0, 0)))
            b.location = Vector(bpose[0]) + Vector(dl) + Vector(hl)
            b.rotation_euler = [math.radians(a + d + h) for a, d, h in zip(bpose[1], dr, hr)]
            b.keyframe_insert("location", frame=frame)
            b.keyframe_insert("rotation_euler", frame=frame)
            if len(bpose) > 2:
                b.scale = bpose[2]
                b.keyframe_insert("scale", frame=frame)

    def make(clip, keys, offset=None):
        if clip not in clips:
            return
        act = bpy.data.actions.new(clip)
        act.use_fake_user = True
        rg.animation_data.action = act
        for frame, poses in keys:
            key(frame, poses, offset)
        track = rg.animation_data.nla_tracks.new()
        track.name = clip
        track.strips.new(clip, int(keys[0][0]), act)
        track.mute = clip != "Idle"
        rg.animation_data.action = None
        for b in pb:
            b.location, b.rotation_euler, b.scale = (0, 0, 0), (0, 0, 0), (1, 1, 1)

    z = ((0, 0, 0), (0, 0, 0))
    idle = []
    for f in sorted(set(range(0, 97, 6)) | {58, 60, 62}):
        br = (1 - math.cos(math.tau * f / 48)) / 2
        ws = math.sin(math.tau * f / 96)
        blink = 0.1 if f == 60 else 1.0
        idle.append((f, {"Hips": ((0, 0, 0), (0, 0, 2 * ws)),
                         "Leg.L": ((0, 0, 0), (0, 0, -2 * ws)), "Leg.R": ((0, 0, 0), (0, 0, -2 * ws)),
                         "Shin.L": z, "Shin.R": z,
                         "Torso": ((0, 0.006 * br, 0), (-1.5 * br, 0, -1.5 * ws)),
                         "Head": ((0, 0.004 * br, 0), (2.5 * br - 1, 4 * ws, ws)),
                         "Arm.L": ((0, 0.008 * br, 0), (0, 0, 3 + 2 * br)),
                         "Arm.R": ((0, 0.008 * br, 0), (0, 0, -3 - 2 * br)),
                         "Fore.L": ((0, 0, 0), (-8, 0, 0)), "Fore.R": ((0, 0, 0), (-8, 0, 0)),
                         "Eye.L": ((0, 0, 0), (0, 0, 0), (1, blink, 1)),
                         "Eye.R": ((0, 0, 0), (0, 0, 0), (1, blink, 1))}))
    make("Idle", idle, pose)
    fidget = []
    for f, turn in ((0, 0), (12, 32), (26, 32), (38, -32), (50, -32), (60, 0)):
        fidget.append((f, {"Head": ((0, 0, 0), (-3 if turn else 0, turn, 0)),
                           "Torso": ((0, 0, 0), (0, turn * 0.25, 0)),
                           "Hips": z, "Leg.L": z, "Leg.R": z, "Shin.L": z, "Shin.R": z,
                           "Arm.L": ((0, 0, 0), (0, 0, 4)), "Arm.R": ((0, 0, 0), (0, 0, -4)),
                           "Fore.L": ((0, 0, 0), (-10, 0, 0)), "Fore.R": ((0, 0, 0), (-10, 0, 0))}))
    make("Fidget", fidget, pose)

    # an arm that carries something keeps still while walking (only its hold pose)
    sw_l = 0.0 if ("Arm.L" in hold or "Fore.L" in hold) else 1.0
    sw_r = 0.0 if ("Arm.R" in hold or "Fore.R" in hold) else 1.0

    def gait(frames, thigh, knee, arm, elbow, lean, bob, roll):
        out = []
        for f, sw, b, passing in frames:  # passing: 1 while the left leg swings through (its knee bends), -1 the right
            kl = knee if passing > 0 else knee * 0.15
            kr = knee if passing < 0 else knee * 0.15
            out.append((f, {"Hips": ((0, bob * b, 0), (0, 0, roll * sw)),
                            "Torso": ((0, 0, 0), (lean, 0, -1.6 * roll * sw)),
                            "Head": ((0, 0, 0), (-lean * 0.7, 0, roll * sw)),
                            "Leg.L": ((0, 0, 0), (-thigh * sw * stride, 0, 0)), "Leg.R": ((0, 0, 0), (thigh * sw * stride, 0, 0)),
                            "Shin.L": ((0, 0, 0), (kl, 0, 0)), "Shin.R": ((0, 0, 0), (kr, 0, 0)),
                            "Arm.L": ((0, 0, 0), (arm * sw * arm_swing * sw_l, 0, 4 * sw_l)),
                            "Arm.R": ((0, 0, 0), (-arm * sw * arm_swing * sw_r, 0, -4 * sw_r)),
                            "Fore.L": ((0, 0, 0), ((-elbow - max(0, -sw) * elbow * 0.6) * sw_l, 0, 0)),
                            "Fore.R": ((0, 0, 0), ((-elbow - max(0, sw) * elbow * 0.6) * sw_r, 0, 0))}))
        return out
    make("Walk", gait(((0, 1, -1, 0), (3, 0, 0, -1), (6, -1, -1, 0), (9, 0, 0, 1), (12, 1, -1, 0)),
                      thigh=26, knee=34, arm=30, elbow=14, lean=4, bob=0.03, roll=5))
    run_step = 0.17 * FPS  # world.js RUN_STEP: one tile per 0.17 s
    make("Run", gait(((0, 1, -1, 0), (run_step / 2, 0, 1, -1), (run_step, -1, -1, 0), (run_step * 1.5, 0, 1, 1), (run_step * 2, 1, -1, 0)),
                     thigh=40, knee=62, arm=50, elbow=50, lean=12, bob=0.03, roll=7))

    # Seated (web/world/seat.js): thighs level, shins hanging a little forward (the feet dangle off a 0.47 m seat),
    # hands on the knees. Sit is the calm loop (8 s: breaths, a slow look round, the feet swinging, a blink);
    # SitDown (0.6 s) bends from standing into it, leaning forward as the body lowers; StandUp is SitDown backwards.
    def seated(br=0.0, look=0.0, kick=0.0, blink=1.0):
        return {"Hips": z, "Torso": ((0, 0.006 * br, 0), (-1.5 * br - 2, look * 0.2, 0)),
                "Head": ((0, 0.004 * br, 0), (2 * br - 2, look, 0)),
                "Leg.L": ((0, 0, 0), (-88, 0, 3)), "Leg.R": ((0, 0, 0), (-88, 0, -3)),
                "Shin.L": ((0, 0, 0), (74 + 8 * kick, 0, 0)), "Shin.R": ((0, 0, 0), (74 - 8 * kick, 0, 0)),
                "Arm.L": ((0, 0.006 * br, 0), (-34, 0, -6)), "Arm.R": ((0, 0.006 * br, 0), (-34, 0, 6)),
                "Fore.L": ((0, 0, 0), (-36, 0, 0)), "Fore.R": ((0, 0, 0), (-36, 0, 0)),
                "Eye.L": ((0, 0, 0), (0, 0, 0), (1, blink, 1)), "Eye.R": ((0, 0, 0), (0, 0, 0), (1, blink, 1))}
    sit = []
    for f in sorted(set(range(0, 193, 8)) | {118, 120, 122}):
        br = (1 - math.cos(math.tau * f / 64)) / 2  # a slower breath than standing
        look = 26 * math.sin(math.tau * f / 192) ** 3  # a long look one way, then the other
        sit.append((f, seated(br, look, math.sin(math.tau * f / 96), 0.1 if f == 120 else 1.0)))
    make("Sit", sit, pose)
    rest = {"Hips": z, "Torso": z, "Head": z, "Leg.L": z, "Leg.R": z, "Shin.L": z, "Shin.R": z,
            "Arm.L": ((0, 0, 0), (0, 0, 3)), "Arm.R": ((0, 0, 0), (0, 0, -3)),
            "Fore.L": ((0, 0, 0), (-8, 0, 0)), "Fore.R": ((0, 0, 0), (-8, 0, 0)),
            "Eye.L": ((0, 0, 0), (0, 0, 0), (1, 1, 1)), "Eye.R": ((0, 0, 0), (0, 0, 0), (1, 1, 1))}
    bend = dict(seated(), **{"Hips": ((0, -0.05, 0), (0, 0, 0)), "Torso": ((0, 0, 0), (18, 0, 0)), "Head": ((0, 0, 0), (-10, 0, 0)),
                             "Leg.L": ((0, 0, 0), (-50, 0, 2)), "Leg.R": ((0, 0, 0), (-50, 0, -2)),
                             "Shin.L": ((0, 0, 0), (56, 0, 0)), "Shin.R": ((0, 0, 0), (56, 0, 0)),
                             "Arm.L": ((0, 0, 0), (-24, 0, -2)), "Arm.R": ((0, 0, 0), (-24, 0, 2)),
                             "Fore.L": ((0, 0, 0), (-26, 0, 0)), "Fore.R": ((0, 0, 0), (-26, 0, 0))})
    down = [(0, rest), (6, bend), (11, dict(seated(), Torso=((0, 0, 0), (4, 0, 0)))), (14, seated())]
    make("SitDown", down, pose)
    make("StandUp", [(14 - f, p) for f, p in reversed(down)], pose)

    # A book off a shelf (hero only; the readable books): Reach (0.5 s) lifts the right hand up and forward to a
    # shelf (the hand tops out about 1.3 m up on tiptoe, just in front of the face), grips and pulls the book down to the
    # chest; HoldBook (4 s loop) holds it there in both hands, head bowed to read; PutBack (0.5 s) reaches up
    # again and lets go, back to standing. The book itself is the game's, carried by the right hand (Fore.R).
    held_pose = {"Torso": ((0, 0, 0), (3, 0, 0)), "Head": ((0, 0, 0), (12, 0, 0)), "Hips": z,
                 "Leg.L": z, "Leg.R": z, "Shin.L": z, "Shin.R": z,
                 "Arm.L": ((0, 0, 0), (-38, 0, -10)), "Arm.R": ((0, 0, 0), (-38, 0, 10)),
                 "Fore.L": ((0, 0, 0), (-80, 0, 0)), "Fore.R": ((0, 0, 0), (-80, 0, 0))}
    up = dict(rest, **{"Hips": ((0, 0.06, 0), (0, 0, 0)), "Torso": ((0, 0, 0), (-6, 0, 3)), "Head": ((0, 0, 0), (-14, 0, 0)),
                       "Arm.R": ((0, 0, 0), (-165, 0, 8)), "Fore.R": ((0, 0, 0), (-8, 0, 0)),
                       "Arm.L": ((0, 0, 0), (-10, 0, 6)), "Fore.L": ((0, 0, 0), (-20, 0, 0))})
    grip = dict(up, **{"Fore.R": ((0, 0, 0), (-24, 0, 0))})
    make("Reach", [(0, rest), (5, up), (7, grip), (12, dict(rest, **held_pose))])
    book = []
    for f in sorted(set(range(0, 97, 8)) | {58, 60, 62}):
        br = (1 - math.cos(math.tau * f / 48)) / 2
        p = dict(held_pose)
        p["Torso"] = ((0, 0.006 * br, 0), (3 - 1.5 * br, 0, 0))
        p["Head"] = ((0, 0.004 * br, 0), (12 + 2 * br, 3 * math.sin(math.tau * f / 96), 0))  # eyes along the lines
        p["Eye.L"] = p["Eye.R"] = ((0, 0, 0), (0, 0, 0), (1, 0.1 if f == 60 else 0.55, 1))  # lids half down, reading
        book.append((f, p))
    make("HoldBook", book)
    make("PutBack", [(0, dict(rest, **held_pose)), (5, grip), (7, up), (12, rest)])
    return rg


# ---------------------------------------------------------------- shared parts (canonical space)

def boots(c, key, tall=False, big=1.0, sole="sole"):
    for s, shin in ((1, "Shin.L"), (-1, "Shin.R")):
        x = s * HX
        top = 0.36 if tall else 0.17
        parts = [sp(0.08 * big, (x, -0.035, 0.07), (0.95, 1.45, 0.85)), cy(0.064 * big, 0.06 * big, 0.06, top, x)]
        if tall:
            parts.append((torus(0.07, 0.02, 14, 6), T((x, 0, top))))
        c.fuse(key, shin, parts, 230 if tall else 190)
        c.add(box(0.13 * big, 0.25 * big, 0.03, 0.012), sole, shin, T((x, -0.035, 0.015)), smooth=False)


def legs(c, key, style="long", skin="skin"):
    """long / rolled (a cuff above the boot) / shorts (bare shins) / none (under a robe)."""
    for s, leg, shin in ((1, "Leg.L", "Shin.L"), (-1, "Leg.R", "Shin.R")):
        x = s * HX
        if style == "shorts":
            c.fuse(skin, shin, [cy(0.045, 0.05, 0.15, 0.3, x)], 90)
            c.fuse(key, leg, limb((x, 0, 0.36), (x * 0.95, 0, 0.53), 0.07, 0.08), 160)
            continue
        if style != "none":
            parts = [cy(0.056, 0.06, 0.17, 0.29, x)]
            if style == "rolled":
                parts.append((torus(0.062, 0.02, 14, 6), T((x, 0, 0.19))))
            c.fuse(key, shin, parts, 160)
        c.fuse(key, leg, limb((x, 0, 0.3), (x * 0.95, 0, 0.53), 0.064, 0.078), 160)
    if style != "none":
        c.fuse(key, "Hips", [sp(0.16, (0, 0, 0.53), (1.0, 0.75, 0.55))], 120)


def torso(c, key, bulk=1.0, belly=0.0, target=320, smooth=True):
    parts = [sp(0.17 * bulk, (0, 0, 0.71), (1.0, 0.78, 1.05)), sp(0.15 * bulk, (0, 0, 0.83), (1.22, 0.8, 0.62)), cy(0.16 * bulk, 0.17 * bulk, 0.56, 0.64)]
    if belly:
        parts.append(sp(0.15 * belly, (0, -0.05, 0.64), (1.05, 1.0, 1.0)))
    c.fuse(key, "Torso", parts, target, smooth=smooth)


def skirt(c, key, z0=0.4, z1=0.62, r0=0.205, r1=0.17, bone="Hips", target=220, open_front=False, smooth=True):
    """Coat skirts, dresses and robes from z0 (hem) up to z1, flaring to r0 at the hem."""
    bm = cyl(r0, r1, z1 - z0, 18)
    bmesh.ops.transform(bm, matrix=T((0, 0, (z0 + z1) / 2), scale=(1.0, 0.86, 1.0)), verts=bm.verts)
    parts = [(bm, None), sp(r1, (0, 0, z1), (1.05, 0.85, 0.3))]
    res = fuse(parts, target)
    if open_front:
        cut(res, (0, -0.1, 0), (0, -1, 0))
    c.add(res, key, bone, smooth=smooth)


def arms(c, key, skin="skin", cuff=None, rolled=False, hand=0.056, bulk=1.0, target=150, smooth=True):
    for s, arm, fore in ((1, "Arm.L", "Fore.L"), (-1, "Arm.R", "Fore.R")):
        x = s * SX
        c.fuse(key, arm, limb((x, 0, SH), (x + s * 0.01, 0, EL + 0.01), 0.068 * bulk, 0.058 * bulk), target, smooth=smooth)
        if rolled:
            c.fuse(skin, fore, limb((x + s * 0.01, 0, EL - 0.02), (x + s * 0.012, 0, 0.56), 0.045, 0.042), 100)
            c.fuse(cuff or key, fore, [(torus(0.064 * bulk, 0.024, 14, 6), T((x + s * 0.01, 0, EL - 0.01)))], 90)
        else:
            parts = limb((x + s * 0.01, 0, EL), (x + s * 0.012, 0, 0.57), 0.056 * bulk, 0.054 * bulk)
            c.fuse(key, fore, parts, target, smooth=smooth)
            c.fuse(cuff or key, fore, [(torus(0.058 * bulk, 0.022, 14, 6), T((x + s * 0.012, 0, 0.57)))], 80)
        hand_at(c, x + s * 0.012, 0.5, s, skin, fore, hand)


def hand_at(c, x, z, s, key, bone, r=0.056):
    c.fuse(key, bone, [sp(r, (x + s * 0.006, -0.006, z), (0.82, 0.95, 1.12)), sp(r * 0.42, (x - s * r * 0.5, -r * 0.6, z + r * 0.3))], 110, voxel=0.009)


def head(c, skin="skin", nose=0.026, ears=True):
    cx, cy_, cz, rx, ry, rz = HEAD
    c.add(cyl(0.05, 0.055, 0.1, 10), skin, "Head", T((0, 0, 0.97)))
    c.add(sphere(1, 18, 12), skin, "Head", T((cx, cy_, cz), scale=(rx, ry, rz)))
    if ears:
        for s in (1, -1):
            c.add(sphere(0.04, 8, 6), skin, "Head", T((s * rx * 0.97, 0.01, cz - 0.01), scale=(0.5, 0.9, 1.15)))
    if nose:
        nz = cz - 0.035
        y = cy_ - ry * math.sqrt(max(1 - ((nz - cz) / rz) ** 2, 0))
        c.add(sphere(nose, 10, 7), skin, "Head", T((0, y - nose * 0.2, nz), scale=(1.1, 0.9, 0.9)))


def eyes(c, w=0.075, h=0.092, x=0.078, z=1.165, iris=0.62, lid=None, closed=False, look=0.0):
    for s, eb in ((1, "Eye.L"), (-1, "Eye.R")):
        if closed:  # smiling arcs
            pts = [(-w * 0.55, -h * 0.05), (-w * 0.28, h * 0.22), (0, h * 0.3), (w * 0.28, h * 0.22), (w * 0.55, -h * 0.05),
                   (w * 0.3, h * 0.08), (0, h * 0.13), (-w * 0.3, h * 0.08)]
            c.add(paint(poly_decal(pts), s * x, z), "eye", eb)
            continue
        c.add(paint(decal(w, h, 14), s * x, z), "white", eb)
        c.add(paint(decal(w * iris, h * 0.78, 12), s * x - s * w * 0.08 + look, z - h * 0.06, 0.0012), "eye", eb)
        c.add(paint(decal(w * 0.22, h * 0.2, 8), s * x + w * 0.05, z + h * 0.12, 0.0024), "white", eb)
        if lid:  # a heavy upper lid in the skin's shade or the hair colour
            key, drop = lid
            pts = [(-w * 0.62, h * 0.18), (-w * 0.3, h * 0.5), (w * 0.3, h * 0.52), (w * 0.62, h * 0.25), (w * 0.4, h * (0.4 - drop)), (-w * 0.4, h * (0.35 - drop))]
            c.add(paint(poly_decal(pts), s * x, z, 0.0034), key, eb)


def brows(c, key="hair", w=0.07, thick=0.016, tilt=0.006, x=0.082, z=1.225):
    for s in (1, -1):
        pts = [(-w / 2, 0), (0, thick * 0.5 + tilt * 0.3), (w / 2, tilt), (w / 2, tilt + thick), (0, thick * 1.3 + tilt * 0.3), (-w / 2, thick * 0.9)]
        c.add(paint(poly_decal([(s * px, pz) for px, pz in pts]), s * x, z), key, "Head")


def mouth(c, kind="smile", w=0.05, z=1.08):
    pts = {"smile": [(-w / 2, 0.004), (-w * 0.3, -w * 0.28), (0, -w * 0.38), (w * 0.3, -w * 0.28), (w / 2, 0.004), (0, -0.002)],
           "flat": [(-w / 2, 0.003), (0, -w * 0.12), (w / 2, 0.003), (0, -w * 0.04)],
           "grin": [(-w / 2, 0.006), (-w * 0.35, -w * 0.32), (0, -w * 0.45), (w * 0.35, -w * 0.32), (w / 2, 0.006), (0, -w * 0.08)],
           "smirk": [(-w * 0.45, -0.002), (0, -w * 0.12), (w * 0.5, w * 0.12), (w * 0.3, w * 0.02), (0, -w * 0.04)]}[kind]
    c.add(paint(poly_decal(pts), 0, z), "lip_blush", "Head")


def cheeks(c, x=0.13, z=1.1):
    for s in (1, -1):
        c.add(paint(decal(0.055, 0.03, 10), s * x, z), "blush", "Head")


def face(c, kind="smile", lid=None, closed=False, brow=(0.07, 0.016, 0.006), blush=True, look=0.0, eye=(0.075, 0.092)):
    eyes(c, eye[0], eye[1], lid=lid, closed=closed, look=look)
    if brow:
        brows(c, "hair", *brow)
    if kind:
        mouth(c, kind)
    if blush:
        cheeks(c)


# hair and hats (Head bone; faceted)

def hair_cap(c, key="hair", z=1.3, y=0.04, sq=0.78, extra=(), target=420):
    c.fuse(key, "Head", [sp(0.215, (0, y, z), (1.05, 1.0, sq))] + list(extra), target, smooth=False, voxel=0.01)


def knit_cap(c, key, cuff=None, pompom=None, z=1.36):
    c.fuse(key, "Head", [sp(0.215, (0, 0.03, z), (1.04, 1.04, 0.8))], 260, smooth=False)
    c.fuse(cuff or key, "Head", [(torus(0.205, 0.042, 18, 6), T((0, 0.02, z - 0.05), (-6, 0, 0)))], 220, smooth=False)
    if pompom:
        c.fuse(pompom, "Head", [sp(0.065, (0, 0.05, z + 0.17))], 120, smooth=False)


def brim_hat(c, key, band=None, r=0.38, crown=0.18, z=1.36, tilt=-6, droop=0.0, target=320):
    brim = lathe([(crown, 0.0), (r, -0.015 - droop), (r - 0.01, -0.035 - droop), (crown, -0.02)], seg=20)
    c.add(brim, key, "Head", T((0, 0.0, z), (tilt, 0, 0)), smooth=False)
    c.fuse(key, "Head", [sp(crown, (0, 0.0, z + 0.05), (1.0, 1.0, 0.85))], target // 2, smooth=False)
    if band:
        c.add(lathe([(crown + 0.006, 0.0), (crown + 0.004, 0.045)], seg=16), band, "Head", T((0, 0, z), (tilt, 0, 0)), smooth=False)


def peak_cap(c, key, peak=None, z=1.34, flat=False):
    c.fuse(key, "Head", [sp(0.215, (0, 0.025, z), (1.06, 1.06, 0.55 if flat else 0.7))], 220, smooth=False)
    c.add(lathe([(0.0, 0.0), (0.13, 0.0), (0.12, 0.02), (0.0, 0.02)], seg=12, sx=1.25, sy=0.7), peak or key, "Head",
          T((0, -0.19, z - 0.03), (-10, 0, 0)), smooth=False)


# props (each on a hand bone; canonical hand at x = +-0.202, z = 0.5)

def staff(c, key, s=-1, top=1.55, crook=False, bone=None, x=None, y=-0.03):
    x = s * (SX + 0.012) if x is None else x
    pts = [(x, y, 0.02), (x, y, top)]
    if crook:
        pts += [(x + s * -0.02, y, top + 0.07), (x + s * -0.08, y, top + 0.1), (x + s * -0.13, y, top + 0.05)]
    c.add(tube(pts, 0.016, 6), key, bone or ("Fore.R" if s < 0 else "Fore.L"), smooth=False)


def round_glasses(c, key="trim_frame", r=0.034, z=1.17):
    for s in (1, -1):
        c.add(torus(r, 0.007, 14, 4), key, "Head", T((s * 0.078, -0.186 + abs(0.078) * 0.15, z), (90, 0, 0)))
    c.add(box(0.06, 0.008, 0.008), key, "Head", T((0, -0.19, z + 0.005)))


# ---------------------------------------------------------------- accessories for route duelists

ACC = {}


def accessory(fn):
    ACC[fn.__name__.removeprefix("acc_")] = fn
    return fn


@accessory
def acc_cap(c):
    peak_cap(c, "acc_accent", "acc_trim", z=1.36)


@accessory
def acc_glasses(c):
    round_glasses(c, "acc_trim", 0.04, 1.168)


@accessory
def acc_headband(c):
    c.fuse("acc_accent", "Head", [(torus(0.205, 0.024, 20, 6), T((0, 0.0, 1.28), (-14, 0, 0), (1.0, 0.95, 1.0)))], 200, smooth=False)


@accessory
def acc_scarf(c):
    c.fuse("acc_accent", "Torso", [(torus(0.1, 0.045, 18, 8), T((0, 0, 0.92), (-8, 0, 0))), *limb((0.07, -0.12, 0.88), (0.09, -0.16, 0.7), 0.042, 0.036)], 320, smooth=False)


@accessory
def acc_bag(c):
    c.fuse("acc_accent", "Hips", [sp(0.08, (-0.21, -0.01, 0.5), (0.55, 1.15, 1.0))], 120, smooth=False)
    c.add(tube([(0.15, -0.11, 0.85), (0.0, -0.17, 0.7), (-0.19, -0.1, 0.56)], 0.013, 5), "acc_trim", "Torso", smooth=False)
    c.add(tube([(0.15, 0.11, 0.85), (0.0, 0.16, 0.7), (-0.19, 0.1, 0.56)], 0.013, 5), "acc_trim", "Torso", smooth=False)


def wearables(c, names):
    for n in names:
        c.group = "acc_" + n
        ACC[n](c)
    c.group = "body"


# ---------------------------------------------------------------- the cast

def palette(**kv):
    base = {"skin": "#f6cfae", "eye": "#2a1f30", "white": "#ffffff", "blush": "#f39a8e", "lip_blush": "#8a3440",
            "sole": "#f1e4c8", "gold": "#e9b949", "acc_accent": "#d6402f", "acc_trim": "#2b2433", "trim_frame": "#3a2c25"}
    base.update(kv)
    return base


CAST = {}


def person(name, pal, bz=1.0, bw=1.0, hs=1.0, leg=1.0, pose=None, hold=None, swing=1.0, wear=()):
    def deco(fn):
        CAST[name] = dict(fn=fn, pal=pal, bz=bz, bw=bw, hs=hs, leg=leg, pose=pose, hold=hold, swing=swing, wear=wear)
        return fn
    return deco


STOOP = {"Torso": ((0, 0, 0), (10, 0, 0)), "Head": ((0, 0, 0), (-8, 0, 0)), "Arm.R": ((0, 0, 0), (-2, 0, -4))}
AKIMBO = {"Arm.L": ((0, 0, 0), (0, 10, 38)), "Arm.R": ((0, 0, 0), (0, -10, -38)),
          "Fore.L": ((0, 0, 0), (0, 0, -78)), "Fore.R": ((0, 0, 0), (0, 0, 78)), "Torso": ((0, 0, 0), (-4, 0, 0))}


@person("hero", palette(hair="#5a3c8e", coat="#ef7426", scarf="#14b3a0", trousers="#26407a", boot="#7a4a2c", case="#8a5a3a"))
def hero(c):
    boots(c, "boot")
    legs(c, "trousers", "rolled")
    torso(c, "coat")
    skirt(c, "coat", 0.4, 0.62, 0.21, 0.17, target=200)
    c.add(box(0.05, 0.02, 0.36, 0.008), "sole", "Torso", T((0, -0.138, 0.66)), smooth=False)  # shirt in the coat's opening
    for s in (1, -1):
        c.add(box(0.075, 0.02, 0.06, 0.008), "case", "Hips", T((s * 0.11, -0.163, 0.47), (0, 0, s * -6)), smooth=False)
    # chunky scarf wound twice, a tail down the front and one flying behind
    c.fuse("scarf", "Torso", [(torus(0.105, 0.05, 18, 8), T((0, 0, 0.92), (-8, 0, 0), (1.0, 0.92, 1.0))),
                              (torus(0.1, 0.045, 18, 8), T((0, 0.01, 0.87), (6, 0, 0), (1.05, 0.95, 1.0))),
                              *limb((0.07, -0.12, 0.88), (0.09, -0.16, 0.66), 0.045, 0.04),
                              *limb((-0.05, 0.12, 0.9), (-0.13, 0.27, 0.78), 0.044, 0.036)], 420)
    c.fuse("case", "Hips", [sp(0.075, (-0.21, -0.02, 0.5), (0.6, 1.05, 1.0))], 100, smooth=False)
    c.add(tube([(0.15, -0.1, 0.84), (0.0, -0.165, 0.7), (-0.18, -0.1, 0.56)], 0.012, 5), "case", "Torso", smooth=False)
    c.add(cyl(0.03, 0.03, 0.012, 6), "gold", "Hips", T((-0.255, -0.02, 0.51), (0, 90, 30)), smooth=False)
    arms(c, "coat", cuff="coat")
    head(c)
    face(c)
    hair_cap(c, extra=[sp(0.09, (-0.11, -0.12, 1.31), (1.2, 0.7, 0.7), (0, -20, 25)), sp(0.085, (-0.02, -0.15, 1.33), (1.2, 0.7, 0.7), (0, 0, 15)),
                       sp(0.08, (0.08, -0.14, 1.32), (1.15, 0.7, 0.7), (0, 15, -10)), sp(0.07, (0.16, -0.08, 1.27), (0.8, 0.8, 1.1), (0, 20, -20)),
                       sp(0.08, (-0.18, -0.02, 1.2), (0.6, 0.9, 1.25)), sp(0.08, (0.19, 0.0, 1.2), (0.6, 0.9, 1.2)),
                       sp(0.11, (0, 0.13, 1.18), (1.4, 0.8, 1.0)), sp(0.05, (0.04, 0.0, 1.47), (0.6, 0.9, 1.4), (-30, 20, 0))], target=560)


# props held in a posed hand: model them where they show in the idle pose, then take the pose back off

def _bone_rot(e):
    if not e:
        return Matrix.Identity(3)
    Mb = Matrix(((1, 0, 0), (0, 0, -1), (0, 1, 0)))  # bone axes (x, up, front) in world columns
    return Mb @ Euler([math.radians(a) for a in e[1]], "XYZ").to_matrix() @ Mb.transposed()


def held(bm, side, hold):
    """bm modelled in the posed (idle) position of the `side` hand ('L' or 'R') -> rest pose of Fore.<side>."""
    s = 1 if side == "L" else -1
    sh, el = Vector((s * SX, 0, SH)), Vector((s * SX, 0, EL))
    ra, rf = _bone_rot((hold or {}).get("Arm." + side)), _bone_rot((hold or {}).get("Fore." + side))
    ra_i, rf_i = ra.inverted(), rf.inverted()
    for v in bm.verts:
        q = sh + ra_i @ (v.co - sh)
        v.co = el + rf_i @ (q - el)
    return bm


def hand_pos(side, hold):
    """Where the hand sits in the posed (idle) frame."""
    s = 1 if side == "L" else -1
    sh, el = Vector((s * SX, 0, SH)), Vector((s * SX, 0, EL))
    ra, rf = _bone_rot((hold or {}).get("Arm." + side)), _bone_rot((hold or {}).get("Fore." + side))
    p = el + rf @ (Vector((s * (SX + 0.012), -0.006, 0.5)) - el)
    return sh + ra @ (p - sh)


def prop(c, side, hold, key, bm, M=None, smooth=False):
    if M is not None:
        bmesh.ops.transform(bm, matrix=M, verts=bm.verts)
    c.add(held(bm, side, hold), key, "Fore." + side, smooth=smooth)


def shoulder_hair(c, z=1.2, key="hair"):
    """Hair under a hat: sides and back only."""
    c.fuse(key, "Head", [sp(0.08, (0.18, 0.0, z), (0.6, 0.9, 1.2)), sp(0.08, (-0.18, 0.0, z), (0.6, 0.9, 1.2)),
                         sp(0.12, (0, 0.13, z - 0.02), (1.4, 0.8, 1.0))], 260, smooth=False)


# ---------------------------------------------------------------- townsfolk

NET = {"Arm.R": ((0, 0, 0), (-22, 0, -8)), "Fore.R": ((0, 0, 0), (-38, 0, 0))}


@person("npc_kid", palette(hair="#6a3a22", hoodie="#f4b521", hoodie_accent="#2f6fd6", shorts="#d6402f", shoe_trim="#2fb5a0",
                           pole_trim="#9a6a3c", net_trim="#f4f1ea"), bz=0.74, bw=0.95, hs=0.98, leg=0.95, hold=NET, swing=1.1,
        wear=("cap", "glasses", "headband", "bag"))
def kid(c):
    boots(c, "shoe_trim", big=0.95)
    legs(c, "shorts", "shorts")
    torso(c, "hoodie", 1.04)
    skirt(c, "hoodie", 0.5, 0.62, 0.19, 0.17, target=120)
    c.fuse("hoodie", "Torso", [sp(0.13, (0, 0.12, 0.9), (1.3, 0.7, 0.62))], 140)  # hood down
    c.add(box(0.17, 0.03, 0.08, 0.012), "hoodie_accent", "Torso", T((0, -0.15, 0.6)), smooth=False)  # pocket
    arms(c, "hoodie", cuff="hoodie_accent", target=120)
    head(c)
    face(c, "grin", brow=(0.07, 0.016, 0.01))
    hair_cap(c, extra=[sp(0.07, (x, y, 1.38), (0.8, 0.8, 1.1), (r, 0, 0)) for x, y, r in ((-0.1, -0.05, -20), (0.0, -0.08, -25), (0.1, -0.04, -15), (0.05, 0.08, 20), (-0.07, 0.07, 25))], target=380)
    hp = hand_pos("R", NET)  # bug net: a pole up from the right hand, a hoop and a bag of netting
    top = hp + Vector((-0.12, 0.06, 0.95))
    prop(c, "R", NET, "pole_trim", tube([hp + Vector((0.02, 0, -0.18)), top], 0.014, 5))
    prop(c, "R", NET, "pole_trim", torus(0.13, 0.012, 16, 4), T(tuple(top + Vector((-0.05, 0, 0.11))), (0, 70, 0)))
    prop(c, "R", NET, "net_trim", lathe([(0.0, -0.24), (0.07, -0.16), (0.12, -0.03), (0.125, 0.0)], seg=10), T(tuple(top + Vector((-0.05, 0, 0.11))), (0, -20, 0)))


@person("npc_gardener", palette(hair="#6b4a2b", overalls="#2f6fc0", shirt_accent="#7fbf3a", boot_trim="#5a3a22",
                                straw_trim="#ecc66a", band="#2f8f4a", can_trim="#3fa3a0"), bz=1.12, bw=1.12, hs=0.92, swing=0.8,
        wear=("glasses", "scarf", "bag"))
def gardener(c):
    boots(c, "boot_trim", big=1.05)
    legs(c, "overalls")
    torso(c, "shirt_accent")
    c.fuse("overalls", "Torso", [(box(0.25, 0.06, 0.2, 0.03, 2), T((0, -0.13, 0.66)))], 120)  # bib
    for s in (1, -1):
        c.add(tube([(s * 0.09, -0.15, 0.75), (s * 0.12, -0.08, 0.88), (s * 0.1, 0.1, 0.86), (s * 0.08, 0.15, 0.62)], 0.017, 5), "overalls", "Torso", smooth=False)
        c.add(cyl(0.017, 0.017, 0.01, 8), "gold", "Torso", T((s * 0.09, -0.163, 0.75), (90, 0, 0)), smooth=False)
    skirt(c, "overalls", 0.46, 0.6, 0.18, 0.165, target=120)
    arms(c, "shirt_accent", rolled=True, cuff="shirt_accent", target=120)
    head(c)
    face(c, "smile")
    shoulder_hair(c)
    brim_hat(c, "straw_trim", band="band", r=0.44, crown=0.19, z=1.33, tilt=-8, droop=0.035)
    hp = hand_pos("R", None)  # watering can hanging from the right hand
    can = Vector((hp.x - 0.03, -0.03, hp.z - 0.12))
    c.fuse("can_trim", "Fore.R", [cy(0.085, 0.075, can.z - 0.08, can.z + 0.07, can.x, can.y), (tube([can + Vector((0, -0.05, -0.02)), can + Vector((0, -0.22, 0.12))], 0.017, 6, 0.8), None),
                                  (torus(0.06, 0.012, 12, 4), T(tuple(can + Vector((0, 0.0, 0.11))), (0, 90, 0)))], 200)


@person("npc_elder", palette(hair="#ece8e2", dress="#8a3f8f", cardigan="#3f8f86", shawl_accent="#e8a630", shoe_trim="#4a3326",
                             cane_trim="#6b4a35", trim_frame="#e9b949"), bz=1.06, bw=1.05, hs=0.94, pose=STOOP, swing=0.3,
        wear=("scarf", "bag"))
def elder(c):
    boots(c, "shoe_trim", big=0.9)
    legs(c, "dress", "none")
    skirt(c, "dress", 0.06, 0.62, 0.25, 0.18, target=260)
    torso(c, "cardigan")
    for z in (0.62, 0.7, 0.78):
        c.add(sphere(0.016, 6, 5), "gold", "Torso", T((0, -0.165, z)), smooth=False)
    shawl = lathe([(0.1, 0.94), (0.24, 0.8), (0.27, 0.68), (0.25, 0.675), (0.22, 0.79), (0.09, 0.9)], seg=16, sy=0.82)
    c.add(shawl, "shawl_accent", "Torso", smooth=False)
    c.add(lathe([(0.0, 0.48), (0.16, 0.72), (0.0, 0.72)], seg=3, phase=90, sx=1.5, sy=0.25), "shawl_accent", "Torso", T((0, 0.18, 0)), smooth=False)
    c.add(sphere(0.035, 8, 6), "shawl_accent", "Torso", T((0, -0.155, 0.84)), smooth=False)
    arms(c, "cardigan", target=120)
    head(c, nose=0.032)
    face(c, "smile", closed=True, brow=(0.06, 0.012, -0.004))
    round_glasses(c)
    hair_cap(c, sq=0.8, extra=[sp(0.07, (0.17, 0.0, 1.23), (0.55, 1.0, 1.0)), sp(0.07, (-0.17, 0.0, 1.23), (0.55, 1.0, 1.0))], target=320)
    c.fuse("hair", "Head", [sp(0.1, (0, 0.08, 1.46), (1.0, 1.0, 0.85))], 160, smooth=False)  # bun
    c.add(tube([(-0.09, 0.07, 1.47), (0.1, 0.1, 1.5)], 0.008, 4), "gold", "Head", smooth=False)  # hairpin
    hp = hand_pos("R", STOOP)
    c.add(tube([(hp.x - 0.02, -0.05, 0.02), (hp.x, -0.01, hp.z + 0.04), (hp.x + 0.03, -0.01, hp.z + 0.1), (hp.x + 0.08, -0.01, hp.z + 0.08)], 0.015, 6),
          "cane_trim", "Fore.R", smooth=False)


@person("npc_sailor", palette(hair="#c8642e", oil="#f2c230", jumper_cream="#efe8d8", stripe_trim="#26407a", cap_accent="#d6402f",
                              trousers="#2b3e66", boot_trim="#3f7a4a", rope_trim="#c9a46a"), bz=1.14, bw=1.18, hs=0.9, swing=0.9,
        wear=("glasses", "scarf", "bag"))
def sailor(c):
    boots(c, "boot_trim", tall=True, big=1.08)
    legs(c, "trousers")
    torso(c, "jumper_cream", 1.05)
    for z in (0.6, 0.7, 0.8):
        c.add(torus(0.18 - abs(z - 0.7) * 0.2, 0.016, 20, 5), "stripe_trim", "Torso", T((0, 0, z), scale=(1, 0.8, 1)), smooth=False)
    c.fuse("oil", "Torso", [(box(0.26, 0.05, 0.28, 0.04, 2), T((0, -0.15, 0.7)))], 140)
    skirt(c, "oil", 0.3, 0.6, 0.235, 0.18, target=200)
    for s in (1, -1):
        c.add(tube([(s * 0.1, -0.17, 0.83), (s * 0.12, -0.08, 0.92), (s * 0.11, 0.08, 0.9), (s * 0.08, 0.16, 0.72)], 0.016, 5), "oil", "Torso", smooth=False)
    c.add(torus(0.15, 0.032, 18, 6), "rope_trim", "Torso", T((0.0, 0.2, 0.72), (78, 0, -25)), smooth=False)  # rope coil on the back
    c.add(tube([(0.15, -0.12, 0.87), (0.02, -0.18, 0.72), (-0.14, -0.15, 0.58)], 0.014, 5), "rope_trim", "Torso", smooth=False)
    arms(c, "jumper_cream", cuff="stripe_trim", hand=0.062, target=120)
    head(c, nose=0.03)
    face(c, "flat", brow=(0.065, 0.014, -0.004))
    hair_cap(c, sq=0.75, extra=[sp(0.07, (0.17, -0.03, 1.2), (0.55, 1.0, 1.2)), sp(0.07, (-0.17, -0.03, 1.2), (0.55, 1.0, 1.2))]
             + [sp(0.05 - i * 0.004, (0, 0.18 + i * 0.012, 1.14 - i * 0.07)) for i in range(5)], target=300)
    knit_cap(c, "cap_accent", z=1.37)


TRAY = {"Fore.L": ((0, 0, 0), (-82, 0, 0)), "Arm.L": ((0, 0, 0), (0, 0, 8))}


@person("npc_shopkeeper", palette(hair="#4a2e1e", shirt="#2fa58a", apron_cream="#f6efe0", kerchief_accent="#d6402f", trousers="#3a3f55",
                                  shoe_trim="#4a3326", tray_trim="#b0763c", bread_trim="#e8a650", mug_accent="#2f6fd6"),
        bz=1.1, bw=1.32, hs=0.93, hold=TRAY, swing=0.7, wear=("cap", "glasses", "headband"))
def shopkeeper(c):
    boots(c, "shoe_trim")
    legs(c, "trousers")
    torso(c, "shirt", 1.06, belly=1.0)
    bib = lathe([(0.19, 0.56), (0.2, 0.66), (0.18, 0.8)], seg=24, sy=0.95)
    cut(bib, (0, -0.05, 0), (0, 1, 0))  # the front half only
    c.add(solid(bib, 0.012), "apron_cream", "Torso", T((0, -0.045, 0)))
    skirt_front = lathe([(0.25, 0.26), (0.23, 0.42), (0.2, 0.56)], seg=24, sy=0.85)
    cut(skirt_front, (0, -0.02, 0), (0, 1, 0))
    c.add(solid(skirt_front, 0.012), "apron_cream", "Hips")
    c.add(torus(0.16, 0.012, 18, 4), "apron_cream", "Torso", T((0, 0, 0.58), scale=(1.05, 0.95, 1)), smooth=False)
    c.fuse("kerchief_accent", "Torso", [(torus(0.095, 0.03, 16, 6), T((0, 0, 0.92))), sp(0.04, (0.03, -0.11, 0.88))], 160)
    arms(c, "shirt", rolled=True, cuff="shirt", target=120)
    head(c, nose=0.03)
    face(c, "smile", brow=(0.068, 0.016, 0.0))
    hair_cap(c, sq=0.72, target=300)
    c.fuse("kerchief_accent", "Head", [(torus(0.21, 0.03, 20, 6), T((0, 0.01, 1.27), (-14, 0, 0)))], 180, smooth=False)
    hp = hand_pos("L", TRAY)
    prop(c, "L", TRAY, "tray_trim", cyl(0.17, 0.17, 0.02, 16), T(tuple(hp + Vector((-0.06, -0.06, 0.05)))))
    prop(c, "L", TRAY, "bread_trim", sphere(0.07, 10, 6), T(tuple(hp + Vector((-0.1, -0.03, 0.09))), scale=(1.4, 0.8, 0.55)))
    prop(c, "L", TRAY, "mug_accent", cyl(0.035, 0.035, 0.08, 10), T(tuple(hp + Vector((0.0, -0.12, 0.1)))))


BOOKS = {"Arm.L": ((0, 0, 0), (-12, 0, 12)), "Fore.L": ((0, 0, 0), (-92, 0, 0))}


@person("npc_scholar", palette(hair="#3a2a20", coat="#8f2f3f", scarf_accent="#e8a630", trousers="#2b2a35", shoe_trim="#2b211c",
                               book_trim="#2f6f9f", book2_trim="#e8d29a", trim_frame="#2b2433"), bz=1.18, bw=0.96, hs=0.9, hold=BOOKS, swing=0.6,
        wear=("cap", "headband"))
def scholar(c):
    boots(c, "shoe_trim", big=0.95)
    legs(c, "trousers")
    torso(c, "coat")
    skirt(c, "coat", 0.2, 0.6, 0.22, 0.17, target=220, open_front=True)
    c.add(box(0.06, 0.02, 0.34, 0.006), "book2_trim", "Torso", T((0, -0.14, 0.7)), smooth=False)  # shirt front
    c.fuse("scarf_accent", "Torso", [(torus(0.1, 0.04, 18, 6), T((0, 0, 0.92))), *limb((0.06, -0.12, 0.88), (0.08, -0.15, 0.48), 0.035, 0.03),
                                     *limb((-0.06, -0.12, 0.88), (-0.07, -0.15, 0.55), 0.035, 0.03)], 300)
    arms(c, "coat", cuff="coat", target=120)
    head(c)
    face(c, "flat", brow=(0.07, 0.014, 0.008), look=0.004)
    round_glasses(c, r=0.04)
    hair_cap(c, sq=0.74, extra=[sp(0.09, (0.1, -0.12, 1.32), (1.3, 0.7, 0.6), (0, 0, -20))], target=320)
    hp = hand_pos("L", BOOKS)
    for i, (k, w) in enumerate((("book_trim", 0.2), ("book2_trim", 0.18), ("book_trim", 0.21))):
        prop(c, "L", BOOKS, k, box(w, 0.06, 0.15, 0.006), T(tuple(hp + Vector((-0.1, -0.05 - 0.062 * i, 0.08))), (0, 0, 8 * (i - 1))))


MALLET = {"Arm.R": ((0, 0, 0), (-18, 0, -14)), "Fore.R": ((0, 0, 0), (-140, 0, 0))}


@person("npc_worker", palette(hair="#4a3326", shirt="#d6542f", apron_trim="#7a4f33", trousers="#3a4a5e", boot_trim="#3a2a20",
                              cap_accent="#2f6fd6", glove_trim="#c9a46a", wood_trim="#a8754a", iron_trim="#6e7378"),
        bz=1.16, bw=1.25, hs=0.9, hold=MALLET, swing=0.9, wear=("glasses", "scarf", "bag"))
def worker(c):
    boots(c, "boot_trim", big=1.12)
    legs(c, "trousers")
    torso(c, "shirt", 1.06)
    c.fuse("apron_trim", "Torso", [(box(0.24, 0.05, 0.22, 0.03, 2), T((0, -0.16, 0.68)))], 100)
    skirt(c, "apron_trim", 0.3, 0.6, 0.2, 0.175, target=160, open_front=False)
    for s in (1, -1):
        c.add(tube([(s * 0.09, -0.17, 0.78), (s * 0.12, 0.0, 0.92), (s * 0.06, 0.15, 0.62)], 0.014, 5), "apron_trim", "Torso", smooth=False)
    arms(c, "shirt", skin="glove_trim", rolled=True, cuff="shirt", hand=0.062, target=120)
    head(c, nose=0.032)
    face(c, "flat", brow=(0.075, 0.02, -0.006))
    shoulder_hair(c, 1.2)
    peak_cap(c, "cap_accent", flat=True, z=1.33)
    hp = hand_pos("R", MALLET)  # a big mallet over the right shoulder, head behind
    top = hp + Vector((0.04, 0.42, 0.18))
    prop(c, "R", MALLET, "wood_trim", tube([hp + Vector((0, -0.06, -0.03)), top], 0.017, 6))
    prop(c, "R", MALLET, "iron_trim", cyl(0.06, 0.06, 0.2, 8), T(tuple(top), (0, 90, 0)))


STAFF_L = {"Arm.L": ((0, 0, 0), (-10, 0, 4)), "Fore.L": ((0, 0, 0), (-28, 0, 0))}


@person("npc_wanderer", palette(hair="#c8642e", jacket="#3f8f4a", trousers="#5a4a3a", boot_trim="#4a3326", pack_accent="#d6402f",
                                roll_trim="#e8d29a", beanie_accent="#f2c230", pompom_trim="#f4f1ea", staff_trim="#8a5a34"),
        bz=1.1, bw=1.06, hs=0.93, hold=STAFF_L, swing=0.8, wear=("glasses", "scarf"))
def wanderer(c):
    boots(c, "boot_trim", big=1.08)
    legs(c, "trousers", "rolled")
    torso(c, "jacket")
    skirt(c, "jacket", 0.42, 0.6, 0.2, 0.17, target=140)
    # a tall pack with a bedroll on top: the wanderer's back reads from across the map
    c.fuse("pack_accent", "Torso", [(box(0.28, 0.17, 0.4, 0.06, 2), T((0, 0.22, 0.74))), sp(0.08, (0, 0.31, 0.62), (1.4, 0.6, 1.0))], 220, smooth=False)
    c.fuse("roll_trim", "Torso", [(cyl(0.075, 0.075, 0.42, 10), T((0, 0.22, 1.0), (0, 90, 0)))], 120, smooth=False)
    for s in (1, -1):
        c.add(tube([(s * 0.1, 0.06, 0.92), (s * 0.12, -0.12, 0.85), (s * 0.12, -0.15, 0.62), (s * 0.12, 0.08, 0.56)], 0.016, 5), "pack_accent", "Torso", smooth=False)
    arms(c, "jacket", cuff="jacket", target=120)
    head(c)
    face(c, "smile")
    shoulder_hair(c, 1.21)
    knit_cap(c, "beanie_accent", cuff="beanie_accent", pompom="pompom_trim", z=1.36)
    hp = hand_pos("L", STAFF_L)
    prop(c, "L", STAFF_L, "staff_trim", tube([Vector((hp.x + 0.02, hp.y - 0.02, 0.03)), hp + Vector((0.0, 0.0, 1.05))], 0.017, 6))


@person("npc_rival", palette(hair="#1f1f2a", hair_tip="#e8302f", jacket="#e8302f", stripe_trim="#f4f1ea", trousers="#26262e",
                             shoe_trim="#f4f1ea", glove_trim="#26262e"), bz=1.04, bw=1.06, hs=0.98, pose=AKIMBO, swing=1.0,
        wear=("glasses", "headband", "scarf", "bag"))
def rival(c):
    boots(c, "shoe_trim", big=1.02, sole="stripe_trim")
    legs(c, "trousers")
    torso(c, "jacket")
    skirt(c, "jacket", 0.5, 0.6, 0.18, 0.17, target=100)
    c.fuse("jacket", "Torso", [(lathe([(0.1, 0.86), (0.12, 0.93), (0.115, 0.98), (0.09, 0.98), (0.09, 0.88)], seg=14), None)], 120)  # high collar
    c.add(box(0.014, 0.02, 0.32), "stripe_trim", "Torso", T((0, -0.15, 0.72)), smooth=False)
    arms(c, "jacket", skin="glove_trim", cuff="stripe_trim", target=120)
    for s, b in ((1, "Arm.L"), (-1, "Arm.R")):
        c.add(box(0.016, 0.02, 0.2), "stripe_trim", b, T((s * 0.258, 0, 0.77), (0, s * 6, 0)), smooth=False)
    head(c)
    face(c, "smirk", brow=(0.075, 0.018, 0.016), lid=("skin", 0.12))
    spikes = [sp(0.06, (x, y, 1.42 + h), (0.7, 0.7, 2.2), (rx, 0, rz)) for x, y, h, rx, rz in
              ((-0.12, -0.04, 0.02, -35, 30), (-0.04, -0.06, 0.06, -40, 8), (0.05, -0.04, 0.05, -40, -12), (0.13, 0.0, 0.0, -35, -32),
               (-0.07, 0.08, 0.03, -60, 18), (0.07, 0.09, 0.02, -60, -18), (0.0, 0.14, -0.02, -75, 0))]
    hair_cap(c, sq=0.72, extra=spikes, target=460)
    c.fuse("hair_tip", "Head", [sp(0.05, (-0.1, -0.2, 1.37), (2.0, 0.6, 0.5), (35, 0, 25))], 80, smooth=False)


# ---------------------------------------------------------------- the seven rivals

SHEARS = {"Arm.R": ((0, 0, 0), (-30, 0, -6)), "Fore.R": ((0, 0, 0), (-55, 0, 0)), "Arm.L": ((0, 0, 0), (-30, 0, 6)), "Fore.L": ((0, 0, 0), (-60, 0, 0))}


@person("rival_grass", palette(hair="#6b4a2b", vest="#3f9a3a", shirt_cream="#f1e7d0", trousers="#7a5a36", boot="#4a3326",
                               leaf="#5fbf4a", leaf_dk="#2f7f35", hat="#d8b45a", blade_trim="#c9ced6", wood_trim="#8a5a34"),
        bz=1.14, bw=1.16, hs=0.92, hold=SHEARS, swing=0.6)
def bramble(c):
    """Bramble, hedge tender: a broad hat ringed with leaves, a leaf mantle, big hedge shears held ready."""
    boots(c, "boot", big=1.1)
    legs(c, "trousers", "rolled")
    torso(c, "shirt_cream")
    c.fuse("vest", "Torso", [sp(0.172, (0, 0.0, 0.7), (1.02, 0.8, 0.9))], 160)
    skirt(c, "vest", 0.44, 0.6, 0.19, 0.17, target=120)
    leaves = [sp(0.1, (x, y, z), (0.9, 0.35, 1.25), (rx, 0, rz)) for x, y, z, rx, rz in
              ((0.0, 0.16, 0.74, -15, 0), (-0.12, 0.13, 0.78, -10, 25), (0.12, 0.13, 0.78, -10, -25), (-0.08, 0.15, 0.6, -15, 12),
               (0.08, 0.15, 0.6, -15, -12), (0.0, 0.16, 0.55, -15, 0))]
    c.fuse("leaf_dk", "Torso", leaves, 220, smooth=False)
    c.fuse("leaf", "Torso", [sp(0.11, (s * 0.17, 0.0, 0.85), (1.0, 0.8, 0.35), (0, s * 25, s * 25)) for s in (1, -1)]
           + [sp(0.08, (s * 0.07, -0.12, 0.88), (0.9, 0.4, 0.6), (20, 0, s * 30)) for s in (1, -1)], 200, smooth=False)
    arms(c, "shirt_cream", rolled=True, cuff="vest", hand=0.06, target=110)
    head(c, nose=0.034)
    face(c, "smile", brow=(0.07, 0.02, -0.004))
    c.fuse("hair", "Head", [sp(0.13, (0, -0.1, 1.06), (1.2, 0.8, 0.75))], 160, smooth=False)  # a short beard
    shoulder_hair(c, 1.2)
    brim_hat(c, "hat", r=0.4, crown=0.19, z=1.34, tilt=-6, droop=0.02)
    for i in range(10):
        a = i / 10 * math.tau
        c.add(sphere(0.06, 6, 4), "leaf" if i % 2 else "leaf_dk", "Head",
              T((0.36 * math.cos(a), 0.36 * math.sin(a) * 0.95, 1.33 - 0.03), (0, 0, math.degrees(a)), (1.5, 0.6, 0.3)), smooth=False)
    hr, hl = hand_pos("R", SHEARS), hand_pos("L", SHEARS)  # shears: two long blades up and to the left from the hands
    tip = (hr + hl) / 2 + Vector((0.12, -0.25, 0.42))
    for side, h in (("R", hr), ("L", hl)):
        prop(c, side, SHEARS, "wood_trim", tube([h + Vector((0, 0, -0.04)), h + (tip - h) * 0.3], 0.016, 5))
        prop(c, side, SHEARS, "blade_trim", tube([h + (tip - h) * 0.3, tip + Vector((0.02 if side == "R" else -0.02, 0, 0))], 0.022, 4, 0.4))


POLE = {"Arm.L": ((0, 0, 0), (-8, 0, 4)), "Fore.L": ((0, 0, 0), (-22, 0, 0))}


@person("rival_water", palette(hair="#2f8fc0", coat="#24365e", toggle_trim="#e9d29a", hat="#f2c230", boot="#2a2a33", trousers="#e8e2d0",
                               pole_red="#d6402f", pole_white="#f4f1ea"), bz=1.16, bw=1.1, hs=0.92, hold=POLE, swing=0.7)
def marlo(c):
    """Marlo, tide reader: a sou'wester, a long navy oilcoat with toggles, a striped tide pole taller than him."""
    boots(c, "boot", tall=True)
    legs(c, "trousers")
    torso(c, "coat", 1.04)
    skirt(c, "coat", 0.16, 0.6, 0.24, 0.175, target=240)
    c.fuse("coat", "Torso", [(torus(0.12, 0.04, 16, 6), T((0, 0.02, 0.92), (-12, 0, 0)))], 140)  # turned-up collar
    for z in (0.32, 0.5, 0.66, 0.8):
        c.add(box(0.06, 0.02, 0.018, 0.005), "toggle_trim", "Hips" if z < 0.55 else "Torso", T((0, -0.2 if z < 0.55 else -0.165, z)), smooth=False)
    arms(c, "coat", cuff="coat", target=120)
    head(c)
    face(c, "flat", brow=(0.07, 0.014, -0.004), lid=("skin", 0.08))
    shoulder_hair(c, 1.2)
    c.fuse("hat", "Head", [sp(0.205, (0, 0.02, 1.36), (1.0, 1.0, 0.75))], 160, smooth=False)
    brim = lathe([(0.19, 0.04), (0.33, -0.04), (0.325, -0.06), (0.18, 0.02)], seg=22)
    for v in brim.verts:
        if v.co.y > 0:
            v.co.y *= 1.3
            v.co.z -= 0.08 * (v.co.y / 0.42)
    c.add(brim, "hat", "Head", T((0, 0.02, 1.33)), smooth=False)
    hp = hand_pos("L", POLE)
    base, top = Vector((hp.x + 0.03, hp.y - 0.03, 0.02)), Vector((hp.x + 0.03, hp.y - 0.03, 1.95))
    for i in range(8):
        a, b = base.lerp(top, i / 8), base.lerp(top, (i + 1) / 8)
        prop(c, "L", POLE, "pole_red" if i % 2 else "pole_white", tube([a, b], 0.02, 6))


LEAN = {"Torso": ((0, 0, 0), (8, 0, 0)), "Head": ((0, 0, 0), (-6, 0, 0)),
        "Arm.L": ((0, 0, 0), (-25, 0, 10)), "Fore.L": ((0, 0, 0), (-60, 0, 0)), "Arm.R": ((0, 0, 0), (-25, 0, -10)), "Fore.R": ((0, 0, 0), (-60, 0, 0)),
        "Leg.L": ((0, 0, 0), (-10, 0, 0)), "Shin.L": ((0, 0, 0), (12, 0, 0)), "Leg.R": ((0, 0, 0), (6, 0, 0))}


@person("rival_fire", palette(hair="#e8402a", hair_tip="#ffc23a", jacket="#ff6a1f", glove="#3a2a26", leggings="#2a2226", shoe="#ffc23a",
                              goggle_trim="#3a3f4a", lens_trim="#7fd4ff"), bz=1.04, bw=1.04, hs=0.98, pose=LEAN, swing=1.3)
def cinza(c):
    """Cinza, kiln runner: a flame of a ponytail swept back, goggles up, huge kiln gauntlets, already leaning in."""
    boots(c, "shoe", big=1.02)
    legs(c, "leggings")
    torso(c, "jacket")
    skirt(c, "jacket", 0.52, 0.62, 0.18, 0.17, target=100)
    c.fuse("jacket", "Torso", [(torus(0.11, 0.04, 16, 6), T((0, 0.01, 0.9)))], 120)
    for s, arm, fore in ((1, "Arm.L", "Fore.L"), (-1, "Arm.R", "Fore.R")):
        x = s * SX
        c.fuse("jacket", arm, limb((x, 0, SH), (x + s * 0.01, 0, EL + 0.01), 0.066, 0.058), 110)
        c.fuse("glove", fore, limb((x + s * 0.01, 0, EL), (x + s * 0.012, 0, 0.56), 0.07, 0.08) + [sp(0.085, (x + s * 0.012, -0.01, 0.49), (0.95, 1.0, 1.1))], 220)
    head(c)
    face(c, "grin", brow=(0.075, 0.02, 0.018))
    hair_cap(c, sq=0.72, target=300)
    flame = [sp(0.1 - i * 0.011, (0.0, 0.06 + i * 0.075, 1.46 + i * 0.05 - i * i * 0.014), (0.9, 1.3, 0.75), (-20 - i * 12, 0, 0)) for i in range(6)]
    flame += [sp(0.06, (s * 0.08, 0.02, 1.5), (0.7, 0.8, 1.6), (-25, 0, s * -25)) for s in (1, -1)]
    c.fuse("hair", "Head", flame, 360, smooth=False)
    c.fuse("hair_tip", "Head", [sp(0.05, (0.0, 0.48, 1.38), (0.8, 1.6, 0.5), (-60, 0, 0))] + [sp(0.035, (s * 0.1, 0.03, 1.6), (0.7, 0.8, 1.4), (-25, 0, s * -25)) for s in (1, -1)], 140, smooth=False)
    c.fuse("goggle_trim", "Head", [(torus(0.212, 0.016, 22, 5), T((0, 0.0, 1.32), (-16, 0, 0)))], 160, smooth=False)
    for s in (1, -1):
        c.add(cyl(0.045, 0.045, 0.04, 10), "goggle_trim", "Head", T((s * 0.08, -0.18, 1.35), (68, 0, 0)), smooth=False)
        c.add(cyl(0.034, 0.034, 0.01, 10), "lens_trim", "Head", T((s * 0.08, -0.2, 1.358), (68, 0, 0)), smooth=False)


VANE = {"Arm.R": ((0, 0, 0), (-6, 0, -4)), "Fore.R": ((0, 0, 0), (-20, 0, 0))}


@person("rival_lightning", palette(hair="#d9782a", coat="#f4c531", cape_trim="#3c3f47", boot="#3c3f47", trousers="#3c3f47",
                                   vane_trim="#b7bcc4", cup_trim="#7fd4ff", book_trim="#7a4f33", trim_frame="#3c3f47"),
        bz=1.14, bw=1.04, hs=0.92, hold=VANE, swing=0.7)
def quill(c):
    """Quill, weather watcher: a long yellow raincoat with a dark cape collar, goggles, an anemometer staff."""
    boots(c, "boot", tall=True)
    legs(c, "trousers")
    torso(c, "coat")
    skirt(c, "coat", 0.2, 0.6, 0.24, 0.175, target=220)
    c.add(lathe([(0.1, 0.95), (0.25, 0.8), (0.27, 0.66), (0.25, 0.655), (0.23, 0.78), (0.09, 0.91)], seg=16, sy=0.82), "cape_trim", "Torso", smooth=False)
    c.fuse("book_trim", "Hips", [(box(0.05, 0.14, 0.18, 0.01), T((0.22, 0.0, 0.48)))], 60)
    arms(c, "coat", cuff="cape_trim", target=120)
    head(c)
    face(c, "flat", brow=(0.07, 0.014, 0.004))
    round_glasses(c, r=0.042)
    hair_cap(c, sq=0.76, extra=[sp(0.07, (x, -0.1, 1.38), (1.0, 0.7, 0.9), (30, 0, x * 100)) for x in (-0.12, -0.04, 0.05, 0.13)], target=360)
    hp = hand_pos("R", VANE)
    top = Vector((hp.x - 0.02, hp.y - 0.02, 1.85))
    prop(c, "R", VANE, "vane_trim", tube([Vector((hp.x - 0.02, hp.y - 0.02, 0.03)), top], 0.016, 6))
    for i in range(3):  # three cups on arms
        a = i / 3 * math.tau
        tip = top + Vector((0.13 * math.cos(a), 0.13 * math.sin(a), 0.02))
        prop(c, "R", VANE, "vane_trim", tube([top + Vector((0, 0, 0.02)), tip], 0.008, 4))
        prop(c, "R", VANE, "cup_trim", sphere(0.04, 8, 5), T(tuple(tip), scale=(1.0, 1.0, 0.8)))
    prop(c, "R", VANE, "cup_trim", lathe([(0.0, 0.0), (0.03, 0.02), (0.0, 0.18)], seg=4), T(tuple(top + Vector((0, 0, 0.04)))))  # a lightning point


HAMMER = {"Arm.R": ((0, 0, 0), (-20, 0, -16)), "Fore.R": ((0, 0, 0), (-145, 0, 0))}


@person("rival_fighting", palette(hair="#2a2024", hat="#f2c230", vest="#ff7a1f", stripe_trim="#f4f1ea", shirt="#5a6a7a",
                                  trousers="#6b4a35", boot="#3a2a20", wood_trim="#a8754a", iron_trim="#5a5f66", lamp_trim="#fff2b0"),
        bz=1.18, bw=1.36, hs=0.9, hold=HAMMER, swing=0.9)
def orla(c):
    """Orla, quarry foreman: a hard hat with a lamp, broad shoulders in a high-vis vest, a sledgehammer on the shoulder."""
    boots(c, "boot", big=1.18)
    legs(c, "trousers", "rolled")
    torso(c, "shirt", 1.1)
    c.fuse("vest", "Torso", [sp(0.2, (0, 0, 0.7), (1.03, 0.82, 0.92)), cy(0.19, 0.2, 0.55, 0.63)], 180)
    for z in (0.62, 0.74):
        c.add(torus(0.205, 0.013, 20, 4), "stripe_trim", "Torso", T((0, 0, z), scale=(1.02, 0.84, 1)), smooth=False)
    arms(c, "shirt", rolled=True, cuff="shirt", hand=0.066, bulk=1.12, target=120)
    head(c, nose=0.032)
    face(c, "smile", brow=(0.075, 0.022, -0.004))
    shoulder_hair(c, 1.2)
    c.fuse("hat", "Head", [sp(0.215, (0, 0.02, 1.33), (1.05, 1.08, 0.72))], 200, smooth=False)
    c.add(lathe([(0.2, 0.0), (0.3, -0.02), (0.295, -0.035), (0.2, -0.02)], seg=18, sy=1.1), "hat", "Head", T((0, 0.02, 1.3)), smooth=False)
    c.add(cyl(0.035, 0.04, 0.04, 10), "lamp_trim", "Head", T((0, -0.21, 1.38), (75, 0, 0)), smooth=False)
    hp = hand_pos("R", HAMMER)
    top = hp + Vector((0.05, 0.5, 0.22))
    prop(c, "R", HAMMER, "wood_trim", tube([hp + Vector((0, -0.08, -0.04)), top], 0.02, 6))
    prop(c, "R", HAMMER, "iron_trim", box(0.12, 0.12, 0.26, 0.02), T(tuple(top), (0, 90, 0)))


LANTERN = {"Arm.L": ((0, 0, 0), (-38, 0, 8)), "Fore.L": ((0, 0, 0), (-30, 0, 0))}


@person("rival_psychic", palette(hair="#e8e2f6", cloak="#6b3fae", cloak_dk="#4a2a80", trim_moon="#e9d29a", staff_trim="#3b3848",
                                 glow="#ffd27a"), bz=1.12, bw=1.0, hs=0.94, hold=LANTERN, swing=0.4)
def lumen(c):
    """Lumen, night gardener: a deep hooded cloak to the ground, pale hair, a moon lantern held out on a crook."""
    boots(c, "cloak_dk", big=0.9)
    legs(c, "cloak", "none")
    skirt(c, "cloak", 0.04, 0.62, 0.27, 0.18, target=260)
    torso(c, "cloak")
    c.add(lathe([(0.1, 0.95), (0.26, 0.78), (0.29, 0.6), (0.27, 0.595), (0.24, 0.76), (0.09, 0.91)], seg=16, sy=0.82), "cloak_dk", "Torso", smooth=False)
    c.add(cyl(0.035, 0.035, 0.01, 6), "trim_moon", "Torso", T((0, -0.16, 0.78), (90, 30, 0)), smooth=False)
    arms(c, "cloak", cuff="cloak_dk", bulk=1.1, target=120)
    head(c, nose=0.024)
    face(c, "flat", brow=(0.06, 0.012, 0.004), lid=("hair", 0.16))
    c.fuse("hair", "Head", [sp(0.19, (0, 0.07, 1.31), (1.05, 1.0, 0.72))] + [sp(0.05, (s * 0.07, -0.15, 1.31), (1.3, 0.6, 0.6), (0, 0, s * -30)) for s in (1, -1)]
           + [sp(0.055, (s * 0.16, -0.08, 1.07), (0.7, 0.6, 2.4)) for s in (1, -1)], 300, smooth=False)  # parted in the middle
    hood = sphere(0.26, 16, 10)
    cut(hood, (0, -0.16, 0), (0, -1, 0.25))
    c.add(solid(hood, 0.03), "cloak_dk", "Head", T((0, 0.04, 1.2), (-4, 0, 0), (1.0, 1.0, 1.1)), smooth=False)
    c.fuse("cloak_dk", "Head", [sp(0.09, (0, 0.26, 1.02), (1.1, 0.9, 1.5))], 120, smooth=False)
    hp = hand_pos("L", LANTERN)
    top = hp + Vector((0.0, -0.05, 0.45))
    prop(c, "L", LANTERN, "staff_trim", tube([hp + Vector((0, 0, -0.35)), top, top + Vector((0.06, -0.08, 0.06)), top + Vector((0.1, -0.16, 0.0))], 0.014, 5))
    lamp = top + Vector((0.1, -0.16, -0.12))
    prop(c, "L", LANTERN, "staff_trim", cyl(0.003, 0.003, 0.08, 4), T(tuple(lamp + Vector((0, 0, 0.08)))))
    prop(c, "L", LANTERN, "glow", sphere(0.06, 10, 7), T(tuple(lamp), scale=(1.0, 1.0, 1.15)))


HEXSTAFF = {"Arm.R": ((0, 0, 0), (-8, 0, -4)), "Fore.R": ((0, 0, 0), (-24, 0, 0))}


@person("rival_champion", palette(hair="#2b2638", coat="#232a44", mantle="#7a3fb0", boot="#1c1c24", trousers="#2b2f3d", glove_cream="#e9e4d6",
                                  staff_trim="#2b2433"), bz=1.38, bw=1.06, hs=0.88, hold=HEXSTAFF, swing=0.6)
def sable(c):
    """Sable, the Hex Warden: tall, a long dark coat edged in gold, a round mantle and high collar, a hex-headed staff."""
    boots(c, "boot", tall=True)
    legs(c, "trousers")
    torso(c, "coat")
    skirt(c, "coat", 0.06, 0.6, 0.26, 0.175, target=240)
    c.add(box(0.03, 0.02, 0.54), "gold", "Hips", T((0, -0.2, 0.33), (-7, 0, 0)), smooth=False)
    c.add(lathe([(0.1, 0.94), (0.29, 0.8), (0.33, 0.6), (0.31, 0.595), (0.27, 0.78), (0.09, 0.9)], seg=18, sy=0.84), "mantle", "Torso", smooth=False)
    c.add(lathe([(0.33, 0.595), (0.332, 0.62), (0.312, 0.62), (0.31, 0.595)], seg=18, sy=0.84), "gold", "Torso", smooth=False)
    c.fuse("coat", "Torso", [cy(0.12, 0.13, 0.9, 1.04)], 120)  # high collar
    c.add(cyl(0.05, 0.05, 0.012, 6), "gold", "Torso", T((0, -0.15, 0.74), (90, 30, 0)), smooth=False)
    arms(c, "coat", skin="glove_cream", cuff="gold", target=120)
    head(c, nose=0.026)
    face(c, "flat", brow=(0.07, 0.016, 0.01), lid=("hair", 0.14))
    hair_cap(c, sq=0.78, extra=[sp(0.08, (s * 0.17, 0.02, 1.18), (0.55, 1.0, 1.3)) for s in (1, -1)]
             + [sp(0.07 - i * 0.006, (0, 0.2 + i * 0.015, 1.12 - i * 0.1)) for i in range(5)], target=400)
    c.add(cyl(0.035, 0.035, 0.04, 6), "gold", "Head", T((0, 0.2, 1.17), (70, 0, 30)), smooth=False)
    hp = hand_pos("R", HEXSTAFF)
    top = Vector((hp.x - 0.02, hp.y - 0.03, 1.85))
    prop(c, "R", HEXSTAFF, "staff_trim", tube([Vector((hp.x - 0.02, hp.y - 0.03, 0.03)), top], 0.018, 6))
    prop(c, "R", HEXSTAFF, "gold", torus(0.1, 0.018, 6, 4), T(tuple(top + Vector((0, 0, 0.1))), (90, 0, 0)))

# ---------------------------------------------------------------- Rook, the story rival (1.7)

SLOUCH = {"Torso": ((0, 0, 0), (6, 0, 0)), "Head": ((0, 0, 0), (-4, 0, 6)),
          "Arm.L": ((0, 0, 0), (0, 4, 8)), "Arm.R": ((0, 0, 0), (0, -4, -8))}


@person("rival_colorless", palette(hair="#3a3a40", hair_tip="#e9e7e2", hoodie="#6f6d69", hood_dk="#53514d", patch="#f4f2ec",
                                   tally="#f4f2ec", trousers="#4a4a50", shoe="#e9e7e2", sole="#bdbab3"),
        bz=1.12, bw=0.98, hs=0.94, leg=1.04, pose=SLOUCH, swing=0.8)
def rook(c):
    """Rook, The Blank: a grey hoodie with the hood down, a blank white hexagon patch where a colour would go, a tally
    of wins chalked on the left sleeve, untidy dark hair with one pale streak, scuffed white sneakers."""
    boots(c, "shoe", sole="sole")
    legs(c, "trousers")
    torso(c, "hoodie")
    skirt(c, "hoodie", 0.5, 0.62, 0.19, 0.17, target=110)
    c.fuse("hood_dk", "Torso", [sp(0.13, (0, 0.11, 0.94), (1.25, 0.9, 0.55))], 140)  # the hood, down on the shoulders
    c.add(cyl(0.05, 0.05, 0.01, 6), "patch", "Torso", T((0, -0.168, 0.76), (90, 30, 0)), smooth=False)  # the blank hexagon
    arms(c, "hoodie", cuff="hood_dk", target=120)
    for i in range(4):  # four chalk strokes and one across, on the left sleeve
        c.add(box(0.008, 0.012, 0.07), "tally", "Arm.L", T((0.262, -0.035 + i * 0.022, 0.8)), smooth=False)
    c.add(box(0.008, 0.1, 0.01), "tally", "Arm.L", T((0.264, -0.002, 0.8), (18, 0, 0)), smooth=False)
    head(c)
    face(c, "smirk", brow=(0.072, 0.016, 0.012), lid=("skin", 0.1))
    tufts = [sp(0.06, (x, y, 1.42 + h), (0.7, 0.7, 1.9), (rx, 0, rz)) for x, y, h, rx, rz in
             ((-0.12, -0.08, 0.0, -30, 25), (-0.03, -0.11, 0.03, -35, 5), (0.07, -0.09, 0.02, -30, -15),
              (0.14, -0.02, -0.02, -20, -35), (0.0, 0.1, 0.0, -60, 0))]
    hair_cap(c, sq=0.74, extra=tufts, target=420)
    c.fuse("hair_tip", "Head", [sp(0.05, (0.06, -0.16, 1.43), (0.8, 1.8, 0.7), (-40, 0, -15))], 80, smooth=False)  # the pale streak


# ---------------------------------------------------------------- build and export

def export(name, objs, out_dir):
    for o in bpy.context.view_layer.objects:
        o.select_set(False)
    for o in objs:
        o.select_set(True)
    bpy.context.view_layer.objects.active = objs[0]
    path = os.path.join(out_dir, "" if name == "hero" else "npc", name + ".glb")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    bpy.ops.export_scene.gltf(
        filepath=path, export_format="GLB", use_selection=True, use_active_scene=True, export_yup=True,
        export_apply=True, export_animations=True, export_animation_mode="NLA_TRACKS", export_skins=True,
        export_def_bones=True, export_extras=True, export_vertex_color="NONE", export_cameras=False,
        export_lights=False, export_image_format="AUTO")
    ba.validate_glb(path, name)
    return path


def build(names, out_dir):
    scn = ba.fresh_scene("Characters")
    report = []
    with ba.use_scene(scn):
        for i, name in enumerate(names):
            spec = CAST[name]
            coll = bpy.data.collections.new(ba.PREFIX + name)
            scn.collection.children.link(coll)
            c = Char(name, spec["pal"], spec["bz"], spec["bw"], spec["hs"], spec["leg"])
            spec["fn"](c)
            wearables(c, spec["wear"])
            objs = c.build(coll)
            clips = ("Idle", "Fidget", "Walk", "Run", "Sit", "SitDown", "StandUp", "Reach", "HoldBook", "PutBack") if name == "hero" else ("Idle", "Fidget", "Walk", "Sit")
            rg = rig(c, objs, coll, clips, spec["pose"], spec["hold"], spec["swing"])
            body = ba.tris(objs["body"])
            accs = {g: ba.tris(o) for g, o in objs.items() if g != "body"}
            if out_dir:
                path = export(name, [rg, *objs.values()], out_dir)
                report.append((name, body, accs, os.path.getsize(path)))
            for g, o in objs.items():  # free the plain accessory names (acc_cap ...) for the next person
                if g != "body":
                    o.name = f"{name}.{g}"
            rg.location.x = objs["body"].location.x = i * 1.2
    for name, body, accs, size in report:
        extra = " ".join(f"{g[4:]} {t}" for g, t in accs.items())
        print(f"[characters] {name:16s} {body:5d} tris  {size / 1024:6.1f} KB  {extra}")
        limit = 5000 if name == "hero" else 6000
        if body > limit:
            raise RuntimeError(f"[characters] {name}: {body} triangles, over {limit}")


def main(argv):
    arg = lambda k: argv[argv.index(k) + 1] if k in argv else None  # noqa: E731
    out = os.path.abspath(arg("--out")) if arg("--out") else None
    names = arg("--only").split(",") if arg("--only") else list(CAST)
    build(names, out)


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
