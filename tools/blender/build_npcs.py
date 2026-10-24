"""NPC variants on the hero's rig and clips (Idle, Walk, Fidget), scene "NPCs".

    blender -b art/Sixfold.blend -P tools/blender/build_npcs.py -- --out web/world/assets

Writes web/world/assets/npc/npc_<name>.glb (validated like every export) and npc_palettes.json.
The hero itself is frozen in build_assets.py; NPCs share only rig_character() (bones, clips).
All designs are original. Model space matches the hero's (about 1.2 m before scaling, front = -Y).
"""
import importlib
import json
import math
import os
import sys

import bmesh
import bpy

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import build_assets as ba  # noqa: E402

importlib.reload(ba)
from build_assets import Kit, T, box, cyl, sphere  # noqa: E402

SCENE = "NPCs"


def ring(r_out, r_in, seg=14):
    """Flat annulus facing -Y (glasses rims)."""
    bm = bmesh.new()
    pts = [(math.cos(a), math.sin(a)) for a in (i / seg * math.tau for i in range(seg))]
    o = [bm.verts.new((r_out * x, 0, r_out * z)) for x, z in pts]
    i = [bm.verts.new((r_in * x, 0, r_in * z)) for x, z in pts]
    for n in range(seg):
        m = (n + 1) % seg
        bm.faces.new((o[n], o[m], i[m], i[n]))
    for f in bm.faces:
        f.normal_update()
        if f.normal.y > 0:
            f.normal_flip()
    return bm


def body(k, c, pants="long", sleeves="long", hands="skin"):
    """Shared chibi body in the hero's proportions; c = colour keys for this NPC."""
    for s, b in ((1, "Leg.L"), (-1, "Leg.R")):
        x = s * 0.085
        k.add(box(0.13, 0.2, 0.09, 0.035, 2), c["shoe"], T((x, -0.025, 0.055)), bone=b)
        k.add(box(0.14, 0.21, 0.035, 0.01), c.get("sole", "sole"), T((x, -0.025, 0.0075)), bone=b)
        k.add(cyl(0.05, 0.055, 0.2, 8), c["pants"] if pants == "long" else "skin", T((x, 0, 0.18)), smooth=True, bone=b)
        k.add(cyl(0.075, 0.08, 0.1, 8), c["pants"], T((x, 0, 0.29)), smooth=True, bone=b)
    k.add(box(0.31, 0.2, 0.13, 0.05, 2), c["pants"], T((0, 0, 0.36)), bone="Hips")
    k.add(cyl(0.135, 0.16, 0.24, 10, 0.03), c["top"], T((0, 0, 0.5), scale=(1.05, 0.8, 1)), smooth=True, bone="Torso")
    for s, b in ((1, "Arm.L"), (-1, "Arm.R")):
        x = s * 0.2
        if sleeves == "long":
            k.add(cyl(0.05, 0.045, 0.2, 8), c["sleeve"], T((x, 0, 0.47), (0, s * 8, 0)), smooth=True, bone=b)
        else:  # rolled sleeves: short sleeve, cuff, bare forearm
            k.add(cyl(0.052, 0.05, 0.09, 8), c["sleeve"], T((x, 0, 0.52), (0, s * 8, 0)), smooth=True, bone=b)
            k.add(cyl(0.058, 0.058, 0.035, 8), c["sleeve"], T((x + s * 0.008, 0, 0.47), (0, s * 8, 0)), smooth=True, bone=b)
            k.add(cyl(0.042, 0.04, 0.11, 8), "skin", T((x + s * 0.012, 0, 0.41), (0, s * 8, 0)), smooth=True, bone=b)
        k.add(sphere(0.05, 8, 5), hands, T((x + s * 0.015, 0, 0.35)), smooth=True, bone=b)
    k.add(sphere(0.23, 14, 9), "skin", T((0, 0, 0.84), scale=(1, 0.95, 0.95)), smooth=True, bone="Head")
    for s, eb in ((1, "Eye.L"), (-1, "Eye.R")):
        k.add(sphere(0.048, 8, 6), "eye", T((s * 0.085, -0.203, 0.82), scale=(0.8, 0.5, 1.2)), smooth=True, bone=eb)
        k.add(sphere(0.016, 6, 4), "white", T((s * 0.085 + 0.013, -0.226, 0.85)), smooth=True, bone=eb)
        k.add(sphere(0.03, 8, 4), "blush", T((s * 0.14, -0.18, 0.76), scale=(1.2, 0.4, 0.6)), smooth=True, bone="Head")
    k.add(sphere(0.022, 8, 4), "eye", T((0, -0.221, 0.735), scale=(1.5, 0.4, 0.55)), smooth=True, bone="Head")


def hair_cap(k, mat, tilt=-22, z=0.87, cut=-0.02):
    cap = sphere(0.25, 14, 9)
    bmesh.ops.bisect_plane(cap, geom=list(cap.verts) + list(cap.edges) + list(cap.faces),
                           plane_co=(0, 0, cut), plane_no=(0, 0, -1), clear_outer=True)
    k.add(cap, mat, T((0, 0.03, z), (tilt, 0, 0)), smooth=True, bone="Head")
    k.add(sphere(0.21, 12, 7), mat, T((0, 0.07, 0.79), scale=(1.12, 0.95, 1.05)), smooth=True, bone="Head")


# ---------------------------------------------------------------- the five NPCs

def shopkeeper(k):
    c = {"shoe": "sk_shoe", "pants": "sk_pants", "top": "sk_shirt", "sleeve": "sk_shirt"}
    body(k, c, sleeves="rolled")
    hair_cap(k, "sk_hair", tilt=-42, cut=0.02)  # receding, swept back
    for s in (1, -1):  # silver at the temples: fuller, lighter sides above the ears
        k.add(sphere(0.1, 10, 6), "sk_streak", T((s * 0.19, 0.04, 0.82), scale=(0.5, 1.1, 0.9)), smooth=True, bone="Head")
    k.add(sphere(0.05, 8, 5), "sk_hair", T((0, -0.225, 0.765), scale=(1.4, 0.45, 0.5)), smooth=True, bone="Head")  # moustache
    for s in (1, -1):  # glasses
        k.add(ring(0.062, 0.05), "sk_frame", T((s * 0.085, -0.228, 0.82)), bone="Head")
    k.add(box(0.06, 0.012, 0.012), "sk_frame", T((0, -0.229, 0.83)), bone="Head")
    for s in (1, -1):
        k.add(box(0.012, 0.16, 0.012), "sk_frame", T((s * 0.15, -0.15, 0.83)), bone="Head")
    # apron: bib on the torso, skirt on the hips, neck strap, hex emblem
    k.add(box(0.22, 0.03, 0.22, 0.01), "sk_apron", T((0, -0.125, 0.51)), bone="Torso")
    k.add(box(0.3, 0.03, 0.2, 0.01), "sk_apron", T((0, -0.115, 0.3)), bone="Hips")
    k.add(box(0.03, 0.2, 0.03), "sk_apron", T((0.08, -0.03, 0.63)), bone="Torso")
    k.add(box(0.03, 0.2, 0.03), "sk_apron", T((-0.08, -0.03, 0.63)), bone="Torso")
    k.add(cyl(0.045, 0.045, 0.012, 6), "gold", T((0, -0.145, 0.52), (90, 30, 0)), bone="Torso")
    k.add(box(0.12, 0.012, 0.06), "sk_pocket", T((0, -0.132, 0.31)), bone="Hips")


def gardener(k):
    c = {"shoe": "ga_boot", "pants": "ga_denim", "top": "ga_shirt", "sleeve": "ga_shirt"}
    body(k, c, hands="ga_glove")
    hair_cap(k, "ga_hair", tilt=-15)
    for s in (1, -1):
        k.add(sphere(0.12, 10, 6), "ga_hair", T((s * 0.17, 0.03, 0.76), scale=(0.55, 1.2, 1.2)), smooth=True, bone="Head")
    # straw sun hat
    k.add(cyl(0.38, 0.36, 0.03, 20, 0.008), "ga_straw", T((0, 0.0, 1.02), (-6, 0, 0)), smooth=True, bone="Head")
    k.add(cyl(0.2, 0.18, 0.14, 16, 0.02), "ga_straw", T((0, 0.01, 1.1), (-6, 0, 0)), smooth=True, bone="Head")
    k.add(cyl(0.205, 0.205, 0.04, 16), "ga_band", T((0, 0.01, 1.055), (-6, 0, 0)), smooth=True, bone="Head")
    # overalls bib and straps over the shirt
    k.add(box(0.2, 0.03, 0.17, 0.01), "ga_denim", T((0, -0.125, 0.47)), bone="Torso")
    for s in (1, -1):
        k.add(box(0.035, 0.22, 0.03), "ga_denim", T((s * 0.08, -0.02, 0.59)), bone="Torso")
        k.add(cyl(0.015, 0.015, 0.01, 8), "gold", T((s * 0.08, -0.142, 0.54), (90, 0, 0)), bone="Torso")
    # watering can held in the right hand
    hx = -0.23
    k.add(cyl(0.075, 0.07, 0.13, 12, 0.01), "ga_can", T((hx - 0.02, -0.02, 0.22)), smooth=True, bone="Arm.R")
    k.add(box(0.025, 0.11, 0.025), "ga_can", T((hx - 0.02, -0.02, 0.32)), bone="Arm.R")          # handle
    k.add(cyl(0.015, 0.012, 0.2, 6), "ga_can", T((hx - 0.02, -0.15, 0.27), (55, 0, 0)), smooth=True, bone="Arm.R")
    k.add(cyl(0.03, 0.02, 0.03, 8), "ga_can", T((hx - 0.02, -0.23, 0.33), (55, 0, 0)), smooth=True, bone="Arm.R")


def wanderer(k):
    c = {"shoe": "wa_boot", "pants": "wa_pants", "top": "wa_coat", "sleeve": "wa_coat"}
    body(k, c)
    hair_cap(k, "wa_hair", tilt=-10)
    for s in (1, -1):  # ginger tufts poking out under the cap
        k.add(sphere(0.09, 8, 5), "wa_hair", T((s * 0.19, -0.02, 0.8), (0, 0, s * 20), (0.6, 1.0, 1.1)), smooth=True, bone="Head")
    # knit cap with cuff and pompom
    k.add(sphere(0.255, 14, 8), "wa_cap", T((0, 0.02, 0.92), (-12, 0, 0), (1.0, 1.0, 0.8)), smooth=True, bone="Head")
    k.add(cyl(0.255, 0.255, 0.07, 16, 0.02), "wa_cuff", T((0, 0.0, 0.93), (-12, 0, 0)), smooth=True, bone="Head")
    k.add(sphere(0.07, 8, 6), "wa_cuff", T((0, 0.08, 1.15)), smooth=True, bone="Head")
    # scarf: collar wrap and a hanging tail
    k.add(cyl(0.15, 0.14, 0.08, 14, 0.02), "wa_scarf", T((0, 0.0, 0.62)), smooth=True, bone="Torso")
    k.add(box(0.07, 0.03, 0.2, 0.012), "wa_scarf", T((0.06, -0.135, 0.5), (0, 8, 0)), bone="Torso")
    k.add(box(0.072, 0.032, 0.03), "wa_cuff", T((0.065, -0.136, 0.42), (0, 8, 0)), bone="Torso")
    # long coat skirt over the hips, coat buttons
    k.add(cyl(0.17, 0.2, 0.18, 12, 0.02), "wa_coat", T((0, 0, 0.31), scale=(1.05, 0.85, 1)), smooth=True, bone="Hips")
    for z in (0.44, 0.52):
        k.add(sphere(0.014, 6, 4), "wa_cuff", T((-0.03, -0.128, z)), smooth=True, bone="Torso")


def kid(k):
    c = {"shoe": "ki_shoe", "pants": "ki_shorts", "top": "ki_hoodie", "sleeve": "ki_hoodie"}
    body(k, c, pants="shorts")
    hair_cap(k, "ki_hair", tilt=-18)
    for i, (x, y) in enumerate(((-0.12, -0.12), (0.0, -0.16), (0.12, -0.12), (0.06, 0.05), (-0.08, 0.06))):
        k.add(sphere(0.07, 8, 5), "ki_hair", T((x, y, 1.03), (25, 0, i * 30), (0.9, 0.7, 0.9)), smooth=True, bone="Head")
    # hoodie: hood down behind the neck, kangaroo pocket, drawstrings
    k.add(sphere(0.14, 10, 6), "ki_hoodie", T((0, 0.09, 0.61), scale=(1.15, 0.8, 0.6)), smooth=True, bone="Torso")
    k.add(box(0.17, 0.03, 0.08, 0.012), "ki_pocket", T((0, -0.13, 0.43)), bone="Torso")
    for s in (1, -1):
        k.add(box(0.01, 0.01, 0.08), "cream", T((s * 0.03, -0.135, 0.56)), bone="Torso")
    # card binder held against the left side
    k.add(box(0.035, 0.2, 0.25, 0.01), "ki_binder", T((0.27, -0.02, 0.36)), bone="Arm.L")
    k.add(cyl(0.04, 0.04, 0.01, 6), "gold", T((0.29, -0.02, 0.36), (0, 90, 0)), bone="Arm.L")


def rival(k):
    c = {"shoe": "ri_shoe", "pants": "ri_pants", "top": "ri_jacket", "sleeve": "ri_jacket"}
    body(k, c)
    hair_cap(k, "ri_hair", tilt=-28, cut=0.03)
    # distinctive silhouette: sleek volume on top and a long asymmetric fringe swept to one side,
    # with a single red streak (smooth shapes, no spikes)
    k.add(sphere(0.17, 12, 7), "ri_hair", T((0.04, 0.0, 1.07), (-10, 0, -12), (1.25, 1.35, 0.75)), smooth=True, bone="Head")
    k.add(sphere(0.13, 12, 7), "ri_hair", T((-0.09, -0.16, 0.97), (25, 0, 28), (1.55, 0.6, 0.6)), smooth=True, bone="Head")
    k.add(sphere(0.06, 10, 6), "ri_tip", T((-0.07, -0.235, 1.0), (25, 0, 28), (2.4, 0.45, 0.35)), smooth=True, bone="Head")
    # track jacket: black collar, zip, black stripes down the sleeves
    k.add(cyl(0.12, 0.11, 0.06, 12, 0.015), "ri_black", T((0, 0.0, 0.625)), smooth=True, bone="Torso")
    k.add(box(0.015, 0.02, 0.22), "ri_white", T((0, -0.13, 0.5)), bone="Torso")
    for s, b in ((1, "Arm.L"), (-1, "Arm.R")):
        k.add(box(0.02, 0.02, 0.2), "ri_black", T((s * 0.245, 0, 0.47), (0, s * 8, 0)), bone=b)
    for s, b in ((1, "Leg.L"), (-1, "Leg.R")):  # white stripe down the trousers
        k.add(box(0.012, 0.02, 0.2), "ri_white", T((s * 0.135, 0, 0.2)), bone=b)
    k.add(box(0.1, 0.06, 0.08, 0.015), "ri_black", T((0.15, -0.06, 0.36)), bone="Hips")  # deck holster
    k.add(cyl(0.025, 0.025, 0.01, 6), "ri_tip", T((0.15, -0.093, 0.37), (90, 30, 0)), bone="Hips")


# ---------------------------------------------------------------- rivals (one per colour theme) and the champion

def skirt(k, mat, top=0.17, bottom=0.22, z0=0.12, z1=0.4, seg=12, open_front=False):
    """Coat tails / robe below the waist, on the hips bone (the legs swing underneath).
    open_front cuts the front away so it reads as an open coat, not a dress."""
    bm = cyl(top, bottom, z1 - z0, seg, 0.015)
    if open_front:
        bmesh.ops.bisect_plane(bm, geom=list(bm.verts) + list(bm.edges) + list(bm.faces),
                               plane_co=(0, -0.06, 0), plane_no=(0, -1, 0), clear_outer=True)
    k.add(bm, mat, T((0, 0, z0 + (z1 - z0) / 2), scale=(1.05, 0.85, 1)), smooth=True, bone="Hips")


def rival_grass(k):
    """Botanist: leafy cape over the shoulders and back, vine bracelet, flower pin."""
    body(k, {"shoe": "gr_boot", "pants": "gr_pants", "top": "gr_shirt", "sleeve": "gr_shirt"})
    hair_cap(k, "gr_hair", tilt=-18)
    for s in (1, -1):
        k.add(sphere(0.12, 10, 6), "gr_hair", T((s * 0.17, 0.04, 0.76), scale=(0.55, 1.2, 1.25)), smooth=True, bone="Head")
    k.add(sphere(0.045, 8, 5), "gr_flower", T((0.16, -0.12, 1.0), scale=(1, 0.6, 1)), smooth=True, bone="Head")
    k.add(sphere(0.02, 6, 4), "cream", T((0.16, -0.15, 1.0)), smooth=True, bone="Head")
    for i, (x, z, r) in enumerate(((0.0, 0.58, 0.0), (-0.13, 0.5, 20), (0.13, 0.5, -20), (-0.08, 0.36, 10),
                                   (0.08, 0.36, -10), (0.0, 0.3, 0.0), (-0.17, 0.62, 35), (0.17, 0.62, -35))):
        k.add(sphere(0.11, 8, 5), "gr_leaf" if i % 2 else "gr_leaf_dk", T((x, 0.15, z), (-15, 0, r), (0.75, 0.25, 1.3)),
              smooth=True, bone="Torso")  # cape of overlapping leaves
    k.add(cyl(0.16, 0.15, 0.05, 12, 0.012), "gr_leaf_dk", T((0, 0, 0.625)), smooth=True, bone="Torso")  # collar
    for sx in (1, -1):  # the cape's leaves wrap over the shoulders (reads from the front)
        k.add(sphere(0.1, 8, 5), "gr_leaf", T((sx * 0.17, -0.01, 0.61), (0, sx * 30, sx * 20), (1.0, 0.8, 0.3)), smooth=True, bone="Torso")
    k.add(cyl(0.056, 0.056, 0.03, 10), "gr_vine", T((0.205, 0, 0.39)), smooth=True, bone="Arm.L")       # vine bracelet
    for a in (0, 120, 240):
        k.add(sphere(0.022, 6, 4), "gr_leaf", T((0.205 + 0.05 * math.cos(math.radians(a)), 0.05 * math.sin(math.radians(a)), 0.39),
                                                  scale=(1, 0.5, 0.6)), smooth=True, bone="Arm.L")


def rival_fire(k):
    """Showman: flame-orange tailcoat with yellow lining, goggles pushed up, rounded crimson pompadour."""
    body(k, {"shoe": "fi_shoe", "pants": "fi_pants", "top": "fi_coat", "sleeve": "fi_coat"})
    hair_cap(k, "fi_hair", tilt=-25)
    k.add(sphere(0.15, 12, 7), "fi_hair", T((0, -0.1, 1.04), (-30, 0, 0), (1.1, 1.3, 0.8)), smooth=True, bone="Head")  # pompadour
    k.add(cyl(0.255, 0.255, 0.035, 18), "fi_strap", T((0, 0.0, 0.97), (-20, 0, 0)), smooth=True, bone="Head")       # goggle strap
    for s in (1, -1):
        k.add(cyl(0.06, 0.06, 0.05, 12, 0.01), "gold", T((s * 0.08, -0.2, 1.02), (70, 0, 0)), smooth=True, bone="Head")
        k.add(cyl(0.045, 0.045, 0.012, 12), "fi_lens", T((s * 0.08, -0.226, 1.03), (70, 0, 0)), smooth=True, bone="Head")
    skirt(k, "fi_coat", 0.17, 0.2, 0.12, 0.4, open_front=True)  # tailcoat
    k.add(box(0.06, 0.02, 0.24), "fi_lining", T((0, -0.13, 0.5)), bone="Torso")                    # yellow placket
    for z in (0.44, 0.52, 0.6):
        k.add(sphere(0.014, 6, 4), "gold", T((0.045, -0.14, z)), smooth=True, bone="Torso")
    for s in (1, -1):
        k.add(cyl(0.058, 0.058, 0.04, 8), "fi_lining", T((s * 0.215, 0, 0.39), (0, s * 8, 0)), smooth=True, bone="Arm.L" if s > 0 else "Arm.R")


def rival_water(k):
    """Sailor-scholar: navy double-breasted pea coat, white sailor collar, a book under the arm."""
    body(k, {"shoe": "wt_shoe", "pants": "wt_pants", "top": "wt_coat", "sleeve": "wt_coat"})
    hair_cap(k, "wt_hair", tilt=-20)
    for s in (1, -1):
        k.add(sphere(0.11, 10, 6), "wt_hair", T((s * 0.17, 0.03, 0.78), scale=(0.55, 1.2, 1.15)), smooth=True, bone="Head")
    k.add(box(0.3, 0.16, 0.03, 0.01), "white", T((0, 0.08, 0.63), (-12, 0, 0)), bone="Torso")      # sailor collar
    k.add(box(0.12, 0.02, 0.07), "white", T((0, -0.125, 0.58)), bone="Torso")
    skirt(k, "wt_coat", 0.17, 0.19, 0.22, 0.4)
    for x in (-0.05, 0.05):  # double row of gold buttons
        for z in (0.42, 0.5):
            k.add(sphere(0.014, 6, 4), "gold", T((x, -0.14, z)), smooth=True, bone="Torso")
    k.add(box(0.05, 0.2, 0.26, 0.01), "wt_book", T((0.26, 0.0, 0.42)), bone="Arm.L")               # book
    k.add(box(0.045, 0.19, 0.25), "cream", T((0.266, 0.0, 0.42)), bone="Arm.L")                    # pages
    k.add(cyl(0.03, 0.03, 0.006, 6), "gold", T((0.29, 0.0, 0.46), (0, 90, 0)), bone="Arm.L")


def rival_lightning(k):
    """Tinkerer: yellow jumpsuit, tool belt with pouches and a wrench, headband with two antennae."""
    body(k, {"shoe": "li_boot", "pants": "li_suit", "top": "li_suit", "sleeve": "li_suit"}, hands="li_glove")
    hair_cap(k, "li_hair", tilt=-15)
    for i, x in enumerate((-0.13, -0.04, 0.05, 0.14)):
        k.add(sphere(0.065, 8, 5), "li_hair", T((x, -0.17, 0.98 - abs(x) * 0.3), (30, 0, x * 100), (1, 0.6, 0.8)), smooth=True, bone="Head")
    k.add(cyl(0.26, 0.26, 0.04, 18), "li_band", T((0, 0.0, 0.95), (-15, 0, 0)), smooth=True, bone="Head")   # headband
    for s in (1, -1):  # zig-zag antennae with ball tips
        k.add(cyl(0.012, 0.012, 0.14, 5), "li_band", T((s * 0.14, 0.02, 1.12), (0, s * 20, 0)), bone="Head")
        k.add(cyl(0.012, 0.012, 0.1, 5), "li_band", T((s * 0.18, 0.02, 1.23), (0, -s * 25, 0)), bone="Head")
        k.add(sphere(0.035, 8, 5), "li_tip", T((s * 0.16, 0.02, 1.3)), smooth=True, bone="Head")
    k.add(box(0.33, 0.23, 0.05, 0.015), "li_belt", T((0, 0, 0.4)), bone="Hips")                    # tool belt
    for x in (-0.13, 0.13):
        k.add(box(0.07, 0.05, 0.08, 0.015), "li_belt", T((x, -0.12, 0.36)), bone="Hips")          # pouches
    k.add(box(0.025, 0.012, 0.14), "li_metal", T((0.0, -0.125, 0.33), (0, 25, 0)), bone="Hips")    # wrench
    k.add(box(0.012, 0.025, 0.25), "li_zip", T((0, -0.13, 0.5)), bone="Torso")


def rival_fighting(k):
    """Martial artist: cream wrap top, dark wrap belt with long ends, bandaged forearms, topknot."""
    body(k, {"shoe": "fg_shoe", "pants": "fg_pants", "top": "fg_top", "sleeve": "fg_top"}, sleeves="rolled", hands="fg_wrap")
    hair_cap(k, "fg_hair", tilt=-25)
    k.add(sphere(0.08, 10, 6), "fg_hair", T((0, 0.08, 1.11)), smooth=True, bone="Head")               # topknot
    k.add(cyl(0.05, 0.05, 0.03, 10), "fg_belt", T((0, 0.07, 1.06), (-20, 0, 0)), smooth=True, bone="Head")
    k.add(box(0.03, 0.02, 0.26), "fg_trim", T((0.03, -0.128, 0.5), (0, 28, 0)), bone="Torso")        # crossover lapel
    k.add(box(0.33, 0.23, 0.06, 0.015), "fg_belt", T((0, 0, 0.43)), bone="Hips")                    # wrap belt
    for x, r in ((0.06, 8), (0.1, -6)):
        k.add(box(0.04, 0.015, 0.16), "fg_belt", T((x, -0.12, 0.33), (0, r, 0)), bone="Hips")      # belt ends
    for s, b in ((1, "Arm.L"), (-1, "Arm.R")):
        for z in (0.38, 0.41, 0.44):  # bandage turns on the forearms
            k.add(cyl(0.047, 0.047, 0.018, 8), "fg_wrap", T((s * 0.212, 0, z), (0, s * 8, 0)), smooth=True, bone=b)


def rival_psychic(k):
    """Mystic: hooded violet robe to the ankles, silver fringe, a gold hex charm floating before the hand."""
    body(k, {"shoe": "ps_shoe", "pants": "ps_robe", "top": "ps_robe", "sleeve": "ps_robe"})
    hair_cap(k, "ps_hair", tilt=-10)
    hood = sphere(0.285, 16, 10)
    bmesh.ops.bisect_plane(hood, geom=list(hood.verts) + list(hood.edges) + list(hood.faces),
                           plane_co=(0, -0.12, 0), plane_no=(0, -1, 0), clear_outer=True)  # open at the front
    k.add(hood, "ps_robe", T((0, 0.02, 0.86), (-8, 0, 0)), smooth=True, bone="Head")
    k.add(cyl(0.2, 0.17, 0.08, 14), "ps_robe", T((0, 0.04, 0.63)), smooth=True, bone="Torso")       # hood drape
    skirt(k, "ps_robe", 0.17, 0.26, 0.02, 0.4, 14)
    k.add(box(0.05, 0.02, 0.36), "ps_trim", T((0, -0.13, 0.42)), bone="Torso")
    for s, b in ((1, "Arm.L"), (-1, "Arm.R")):  # wide sleeves
        k.add(cyl(0.05, 0.08, 0.12, 10), "ps_robe", T((s * 0.205, 0, 0.4), (0, s * 8, 0)), smooth=True, bone=b)
    k.add(cyl(0.07, 0.07, 0.02, 6, 0.005), "gold", T((-0.22, -0.16, 0.42), (90, 30, 0)), bone="Arm.R")   # floating charm
    k.add(cyl(0.035, 0.035, 0.024, 6), "ps_gem", T((-0.22, -0.165, 0.42), (90, 30, 0)), bone="Arm.R")


def rival_champion(k):
    """The Hex Warden: long dark open coat with gold hex trim, round shoulder mantle, high collar,
    long dark hair tied back with a gold clasp."""
    body(k, {"shoe": "ch_boot", "pants": "ch_pants", "top": "ch_coat", "sleeve": "ch_coat"}, hands="ch_glove")
    hair_cap(k, "ch_hair", tilt=-38)
    for sx in (1, -1):
        k.add(sphere(0.11, 10, 6), "ch_hair", T((sx * 0.18, 0.05, 0.8), scale=(0.5, 1.2, 1.1)), smooth=True, bone="Head")
    k.add(sphere(0.09, 10, 7), "ch_hair", T((0, 0.26, 0.7), (-10, 0, 0), (0.8, 0.9, 2.0)), smooth=True, bone="Head")  # long tail
    k.add(cyl(0.05, 0.05, 0.04, 6), "gold", T((0, 0.24, 0.84), (70, 0, 30)), bone="Head")                              # clasp
    skirt(k, "ch_coat", 0.17, 0.21, 0.04, 0.4, 14, open_front=True)                                                   # long open coat
    for sx in (1, -1):
        k.add(box(0.025, 0.02, 0.36), "gold", T((sx * 0.13, -0.075, 0.22), (0, 0, sx * -8)), bone="Hips")            # front edges
    k.add(cyl(0.15, 0.14, 0.1, 14, 0.015), "ch_coat", T((0, 0.01, 0.65)), smooth=True, bone="Torso")                   # high collar
    k.add(cyl(0.155, 0.155, 0.02, 14), "gold", T((0, 0.01, 0.705)), smooth=True, bone="Torso")
    k.add(cyl(0.16, 0.27, 0.11, 16, 0.02), "ch_mantle", T((0, 0.02, 0.57)), smooth=True, bone="Torso")                 # mantle
    k.add(cyl(0.275, 0.275, 0.02, 16), "gold", T((0, 0.02, 0.51)), smooth=True, bone="Torso")
    k.add(box(0.03, 0.02, 0.2), "gold", T((0, -0.13, 0.45)), bone="Torso")                                              # front trim
    k.add(cyl(0.05, 0.05, 0.012, 6), "gold", T((0, -0.145, 0.56), (90, 30, 0)), bone="Torso")                          # hex on the chest
    k.add(cyl(0.06, 0.06, 0.012, 6), "gold", T((0, 0.26, 0.56), (90, 30, 0)), bone="Torso")                            # hex on the back


CONFIDENT = {"Torso": ((0, 0, 0), (-5, 0, 0)), "Head": ((0, 0, 0), (-5, 6, 0)),
             "Leg.L": ((0, 0, 0), (0, 0, 4)), "Leg.R": ((0, 0, 0), (0, 0, -4)),
             "Arm.L": ((0, 0, 0), (10, 0, 14)), "Arm.R": ((0, 0, 0), (10, 0, -14))}
GUARD = {"Arm.L": ((0, 0, 0), (-45, 0, 10)), "Arm.R": ((0, 0, 0), (-45, 0, -10)),          # fists up
         "Leg.L": ((0, 0, 0), (-6, 0, 6)), "Leg.R": ((0, 0, 0), (6, 0, -6)), "Torso": ((0, 0, 0), (4, 0, 0))}
CHARM = {"Arm.R": ((0, 0, 0), (-25, 0, -6)), "Head": ((0, 0, 0), (4, 0, 0))}

NPCS = {  # name: (builder, scale, head_scale, leg, idle pose offsets, walk arm swing)
    "npc_shopkeeper": (shopkeeper, 1.33, 0.9, 1.3, None, 1.0),  # older adult: taller, smaller head
    "npc_gardener": (gardener, 1.2, 1.1, 1.0, None, 0.8),
    "npc_wanderer": (wanderer, 1.2, 1.1, 1.05,
                     {"Arm.L": ((0, 0, 0), (12, 0, -7)), "Arm.R": ((0, 0, 0), (12, 0, 7))}, 0.25),  # hands in pockets
    "npc_kid": (kid, 1.0, 1.18, 0.95, None, 1.1),
    "npc_rival": (rival, 1.24, 1.05, 1.1,
                  {"Torso": ((0, 0, 0), (-5, 0, 0)), "Head": ((0, 0, 0), (-5, 8, 0)),       # chest out, chin up
                   "Leg.L": ((0, 0, 0), (0, 0, 5)), "Leg.R": ((0, 0, 0), (0, 0, -5)),       # wide stance
                   "Arm.L": ((0, 0, 0), (10, 0, 16)), "Arm.R": ((0, 0, 0), (10, 0, -16))}, 1.0),
    # campaign rivals, one per colour theme, and the champion
    "rival_grass": (rival_grass, 1.22, 1.08, 1.05, None, 0.9),
    "rival_fire": (rival_fire, 1.24, 1.05, 1.08, CONFIDENT, 1.2),
    "rival_water": (rival_water, 1.22, 1.08, 1.05, None, 0.7),
    "rival_lightning": (rival_lightning, 1.2, 1.1, 1.0, None, 1.2),
    "rival_fighting": (rival_fighting, 1.25, 1.02, 1.1, GUARD, 1.0),
    "rival_psychic": (rival_psychic, 1.22, 1.06, 1.05, CHARM, 0.4),
    "rival_champion": (rival_champion, 1.4, 0.92, 1.4, CONFIDENT, 0.8),  # taller adult
}

PALETTE = {
    "sk_hair": "#8e8a90", "sk_streak": "#d9d6dc", "sk_frame": "#3a2c25", "sk_shirt": "#e9dcc0", "sk_pants": "#6b4a35",
    "sk_shoe": "#4a3326", "sk_apron": "#2f7f86", "sk_pocket": "#256a70",
    "ga_hair": "#6b4a2b", "ga_straw": "#e6c87e", "ga_band": "#5d9b4a", "ga_shirt": "#f2d26b", "ga_denim": "#4a6fa5",
    "ga_glove": "#6aa84f", "ga_boot": "#7a5a3a", "ga_can": "#5f9ea0",
    "wa_hair": "#d9783a", "wa_cap": "#e0a13a", "wa_cuff": "#f3e3c3", "wa_scarf": "#c8473d", "wa_coat": "#6b7d4a",
    "wa_pants": "#4c4f58", "wa_boot": "#6e4b33",
    "ki_hair": "#5a3a28", "ki_hoodie": "#7fb6ff", "ki_pocket": "#5f98e6", "ki_shorts": "#3f4a6e", "ki_shoe": "#e0503c",
    "ki_binder": "#7b5ea7",
    "ri_hair": "#1f1f26", "ri_tip": "#e0303a", "ri_jacket": "#c8303a", "ri_black": "#1f1f26", "ri_white": "#f4f1ea",
    "ri_pants": "#26262e", "ri_shoe": "#f4f1ea",
    "gr_hair": "#7a4a2a", "gr_flower": "#f08ab0", "gr_shirt": "#e8e2c8", "gr_pants": "#8a6b45", "gr_boot": "#5a3f2a",
    "gr_leaf": "#5fb04a", "gr_leaf_dk": "#3a8a35", "gr_vine": "#4f7f2e",
    "fi_hair": "#b0282e", "fi_coat": "#f07a2a", "fi_lining": "#ffd23f", "fi_pants": "#2a2226", "fi_shoe": "#2a2226",
    "fi_strap": "#5a3a26", "fi_lens": "#3a3f4a",
    "wt_hair": "#3d8fb0", "wt_coat": "#24365e", "wt_pants": "#e9e4d6", "wt_shoe": "#3a2c25", "wt_book": "#2f7f86",
    "li_hair": "#c9772a", "li_suit": "#f4c531", "li_glove": "#5a5f6a", "li_boot": "#3c3f47", "li_band": "#3c3f47",
    "li_tip": "#7fd4ff", "li_belt": "#7a5236", "li_metal": "#b7bcc4", "li_zip": "#3c3f47",
    "fg_hair": "#2a2024", "fg_top": "#f1e7d2", "fg_trim": "#8c2f2f", "fg_pants": "#3a3340", "fg_shoe": "#3a3340",
    "fg_belt": "#8c2f2f", "fg_wrap": "#efe6d6",
    "ps_hair": "#d9d6ef", "ps_robe": "#6b4a9b", "ps_trim": "#e9b949", "ps_shoe": "#3a2c4a", "ps_gem": "#c08cff",
    "ch_hair": "#2b2638", "ch_coat": "#22283c", "ch_mantle": "#353d58", "ch_pants": "#2b2f3d", "ch_boot": "#1c1c24",
    "ch_glove": "#e9e4d6",
}
TAGS = {  # main colours for UI name tags
    "npc_shopkeeper": {"primary": "#2f7f86", "secondary": "#e9dcc0", "hair": "#8e8a90", "accent": "#e9b949"},
    "npc_gardener": {"primary": "#4a6fa5", "secondary": "#f2d26b", "hair": "#6b4a2b", "accent": "#e6c87e"},
    "npc_wanderer": {"primary": "#6b7d4a", "secondary": "#c8473d", "hair": "#d9783a", "accent": "#e0a13a"},
    "npc_kid": {"primary": "#7fb6ff", "secondary": "#3f4a6e", "hair": "#5a3a28", "accent": "#7b5ea7"},
    "npc_rival": {"primary": "#c8303a", "secondary": "#1f1f26", "hair": "#1f1f26", "accent": "#e0303a"},
    "rival_grass": {"primary": "#3a8a35", "secondary": "#e8e2c8", "hair": "#7a4a2a", "accent": "#f08ab0"},
    "rival_fire": {"primary": "#f07a2a", "secondary": "#ffd23f", "hair": "#b0282e", "accent": "#9fe0f0"},
    "rival_water": {"primary": "#24365e", "secondary": "#f4f1ea", "hair": "#3d8fb0", "accent": "#e9b949"},
    "rival_lightning": {"primary": "#f4c531", "secondary": "#3c3f47", "hair": "#c9772a", "accent": "#7fd4ff"},
    "rival_fighting": {"primary": "#f1e7d2", "secondary": "#8c2f2f", "hair": "#2a2024", "accent": "#efe6d6"},
    "rival_psychic": {"primary": "#6b4a9b", "secondary": "#e9b949", "hair": "#d9d6ef", "accent": "#c08cff"},
    "rival_champion": {"primary": "#22283c", "secondary": "#e9b949", "hair": "#2b2638", "accent": "#353d58"},
}


def main(argv):
    out = os.path.abspath(argv[argv.index("--out") + 1]) if "--out" in argv else None
    ba.PALETTE.update(PALETTE)
    scn = ba.fresh_scene(SCENE)
    built = {}
    with ba.use_scene(scn):
        for i, (name, (fn, S, head, leg, pose, swing)) in enumerate(NPCS.items()):
            coll = bpy.data.collections.new(ba.PREFIX + name)
            scn.collection.children.link(coll)
            kit = Kit(name)
            fn(kit)
            objs = ba.rig_character(kit, coll, name, S, head_scale=head, leg=leg,
                                    clips=ba.NPC_CLIPS, pose=pose, arm_swing=swing)
            x = -4 + 2 * i
            coll.instance_offset = (x, 0, 0)
            for o in objs:
                if o.parent is None:
                    o.location = (x, 0, 0)
            built[name] = objs
        if out:
            ba.export(built, out)
            with open(os.path.join(out, "npc", "npc_palettes.json"), "w") as f:
                json.dump(TAGS, f, indent=2)
                f.write("\n")
    ba.save_if_file()


if __name__ == "__main__":
    args = globals().get("ARGS")
    if args is None:
        args = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    main(args)
