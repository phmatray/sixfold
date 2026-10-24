"""List z-fighting candidates in the current scene: faces of different parts that lie in the
same plane (within TOL) and overlap. Covers collection instances (props, paths, fences).

Headless, on the assembled world:
    blender -b --factory-startup -P tools/blender/build_world_scene.py -- --check
Live: runpy.run_path("tools/blender/check_coplanar.py", run_name="__main__")

"same" = both faces point the same way (visible flicker). "back-to-back" = opposite normals,
only visible when back faces are drawn and nothing else covers them.
Exit code / return value: number of same-facing pairs.
"""
import math
from collections import defaultdict

import bpy

TOL = 0.01      # plane distance considered coplanar (m)
EXACT = 0.001   # closer than this: real z-fighting at any distance; between EXACT and TOL: "near"
MIN_OVERLAP = 0.005


def components(me):
    parent = list(range(len(me.vertices)))

    def find(i):
        while parent[i] != i:
            parent[i] = parent[parent[i]]
            i = parent[i]
        return i
    for e in me.edges:
        a, b = find(e.vertices[0]), find(e.vertices[1])
        if a != b:
            parent[a] = b
    return [find(p.vertices[0]) for p in me.polygons]


def gather(depsgraph):
    faces = []
    for inst in depsgraph.object_instances:
        ob = inst.object
        if ob.type != "MESH":
            continue
        owner = inst.parent.name if inst.is_instance and inst.parent else ob.name
        label = (inst.parent.instance_collection.name if inst.is_instance and inst.parent
                 and inst.parent.instance_collection else ob.name)
        M = inst.matrix_world.copy()
        N = M.to_3x3().inverted_safe().transposed()
        me = ob.data
        comp = components(me)
        co = [M @ v.co for v in me.vertices]
        for p in me.polygons:
            n = (N @ p.normal)
            if n.length < 1e-9:
                continue
            n.normalize()
            vs = [co[i] for i in p.vertices]
            lo = [min(v[k] for v in vs) for k in range(3)]
            hi = [max(v[k] for v in vs) for k in range(3)]
            c = sum(vs, vs[0] * 0) / len(vs)
            mat = me.materials[p.material_index].name if me.materials else ""
            faces.append((owner, comp[p.index], label, ob.name, n, n.dot(c), lo, hi, mat))
    return faces


def overlap(f, g, axis):
    ov = [min(f[7][k], g[7][k]) - max(f[6][k], g[6][k]) for k in range(3)]
    return all(ov[k] > MIN_OVERLAP for k in range(3) if k != axis) and ov[axis] > -TOL


def check(verbose=True, only_exact=False):
    faces = gather(bpy.context.evaluated_depsgraph_get())
    buckets = defaultdict(list)
    for i, f in enumerate(faces):
        n = f[4]
        sign = 1 if max(n, key=abs) > 0 else -1  # canonical direction: dominant axis positive
        cn = n * sign
        key = (round(cn.x, 2), round(cn.y, 2), round(cn.z, 2))
        buckets[key + (math.floor(f[5] * sign / TOL),)].append((i, sign))
    seen, report = set(), defaultdict(int)
    for key, items in buckets.items():
        near = items + buckets.get(key[:3] + (key[3] + 1,), [])
        for a, sa in items:
            fa = faces[a]
            for b, sb in near:
                pk = (min(a, b), max(a, b))
                if a == b or pk in seen:
                    continue
                fb = faces[b]
                if fa[0] == fb[0] and fa[1] == fb[1] and fa[3] == fb[3]:
                    continue  # same part of the same object: neighbouring faces
                gap = abs(fa[5] * sa - fb[5] * sb)
                if gap >= (EXACT if only_exact else TOL):
                    continue
                axis = max(range(3), key=lambda k: abs(fa[4][k]))
                if not overlap(fa, fb, axis):
                    continue
                seen.add(pk)
                kind = ("same" if sa == sb else "back-to-back") + ("/exact" if gap < EXACT else "/near")
                h = round((fa[6][2] + fa[7][2]) / 2, 3) if axis == 2 else None
                where = f"z={h}" if h is not None else f"plane d={round(fa[5], 2)} axis {'xyz'[axis]}"
                pair = tuple(sorted((f"{fa[2]}[{fa[8]}]", f"{fb[2]}[{fb[8]}]")))
                report[(kind, pair, where)] += 1
    same = sum(v for (k, _, _), v in report.items() if k == "same/exact")
    if verbose:
        for (kind, pair, where), cnt in sorted(report.items()):
            print(f"[coplanar] {kind:19s} {pair[0]} <-> {pair[1]}  {where}  x{cnt}")
        print(f"[coplanar] {len(faces)} faces checked, {same} same-facing exact pairs, "
              f"{sum(report.values()) - same} other pairs")
    return same


if __name__ == "__main__":
    check()
