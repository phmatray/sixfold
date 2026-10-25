"""Bake seamless, painterly base-colour textures for the splat ground (headless only).

    blender -b --factory-startup -P tools/blender/bake_ground_textures.py -- --out web/world/textures

    blender -b --factory-startup -P tools/blender/bake_ground_textures.py -- --out /tmp/tex --only sand,stone

grass.png, dirt.png, wood_floor.png, sand.png, stone.png (cobbles), and for 1.3 ash, heath, wet_sand,
dusk_grass, moss, slate (flags), cave: 1024 x 1024, tileable (all noise is periodic, every stroke
wraps around the edges), low contrast, hand-painted look (soft tonal patches + brush strokes).
"""
import math
import os
import sys

import bpy
import numpy as np

N = 1024


def hexrgb(h):
    return np.array([int(h[i:i + 2], 16) / 255 for i in (1, 3, 5)], dtype=np.float32)


def pnoise(rng, scale, aniso=(1.0, 1.0)):
    """Periodic smooth noise in [0, 1]: white noise low-passed in Fourier space (tiles exactly)."""
    f = np.fft.fftfreq(N)
    fy, fx = np.meshgrid(f, f, indexing="ij")
    filt = np.exp(-((fx * aniso[0]) ** 2 + (fy * aniso[1]) ** 2) * (N / scale) ** 2)
    out = np.real(np.fft.ifft2(np.fft.fft2(rng.normal(size=(N, N))) * filt))
    return (out - out.min()) / (out.max() - out.min())


def spread(x, amount=0.22):
    """Re-centre a noise mix around 0.5 with a fixed spread (sums of noises flatten toward grey)."""
    return 0.5 + amount * (x - x.mean()) / x.std()


def ramp(t, cols):
    """Map t in [0, 1] through evenly spaced colours."""
    cols = [hexrgb(c) for c in cols]
    t = np.clip(t, 0, 1) * (len(cols) - 1)
    i = np.minimum(t.astype(int), len(cols) - 2)
    f = (t - i)[..., None]
    a = np.stack(cols)[i]
    b = np.stack(cols)[i + 1]
    return a * (1 - f) + b * f


def stamp(img, rng, count, length, width, angle, angle_jit, colours, alpha):
    """Soft elongated brush dabs, wrapped around the edges so the texture stays seamless."""
    r = int(max(length, width)) + 2
    yy, xx = np.mgrid[-r:r + 1, -r:r + 1].astype(np.float32)
    for _ in range(count):
        a = angle + rng.uniform(-angle_jit, angle_jit)
        ca, sa = math.cos(a), math.sin(a)
        u = (xx * ca + yy * sa) / (length * rng.uniform(0.7, 1.3))
        v = (-xx * sa + yy * ca) / (width * rng.uniform(0.7, 1.3))
        k = np.clip(1 - (u * u + v * v), 0, 1) ** 1.5 * alpha
        y0, x0 = rng.integers(0, N, 2)
        rows = (np.arange(-r, r + 1) + y0) % N
        cols = (np.arange(-r, r + 1) + x0) % N
        col = hexrgb(colours[rng.integers(len(colours))])
        patch = img[np.ix_(rows, cols)]
        img[np.ix_(rows, cols)] = patch * (1 - k[..., None]) + col * k[..., None]


def grass(rng):
    img = ramp(spread(pnoise(rng, 220) * 0.55 + pnoise(rng, 70) * 0.3 + pnoise(rng, 20) * 0.15),
               ["#357f2b", "#45993a", "#56ad41", "#6abd4b"])
    stamp(img, rng, 2500, 30, 9, 0.3, 3.0, ["#3d8c30", "#5fb646", "#4a9f3a"], 0.22)       # broad soft dabs
    stamp(img, rng, 9000, 12, 2.2, -math.pi / 2, 0.45, ["#3a8a2c", "#6cc24a", "#57ad40", "#86d462"], 0.5)  # blades
    for _ in range(140):  # clover hints: three round leaves
        cy, cx = rng.integers(0, N, 2)
        for k in range(3):
            a = k * math.tau / 3 + rng.uniform(0, 1)
            yy, xx = np.mgrid[-5:6, -5:6]
            d = np.clip(1 - ((yy - 4 * math.sin(a)) ** 2 + (xx - 4 * math.cos(a)) ** 2) / 14, 0, 1) * 0.6
            rows = (np.arange(-5, 6) + cy) % N
            cols = (np.arange(-5, 6) + cx) % N
            img[np.ix_(rows, cols)] = img[np.ix_(rows, cols)] * (1 - d[..., None]) + hexrgb("#7fcf63") * d[..., None]
    return img


def dirt(rng):
    img = ramp(spread(pnoise(rng, 200) * 0.55 + pnoise(rng, 60) * 0.3 + pnoise(rng, 18) * 0.15),
               ["#ad7746", "#c18c57", "#d3a46e", "#e0b885"])
    stamp(img, rng, 1500, 40, 7, 0.0, 0.5, ["#c08d55", "#e3b981", "#a96f40"], 0.3)    # worn strokes
    stamp(img, rng, 900, 14, 10, 0.0, 3.0, ["#9c6a3c", "#d9ae78"], 0.25)              # soft blotches
    yy, xx = np.mgrid[-12:13, -12:13].astype(np.float32)
    for _ in range(220):  # pebbles: lit from the top-left, soft shadow
        cy, cx = rng.integers(0, N, 2)
        r = rng.uniform(3.5, 9)
        body = np.clip(1 - (xx ** 2 + yy ** 2) / r ** 2, 0, 1)
        shadow = np.clip(1 - ((xx - 1.5) ** 2 + (yy + 1.5) ** 2) / (r + 1) ** 2, 0, 1) * 0.35
        light = np.clip(1 - ((xx + r * 0.3) ** 2 + (yy - r * 0.3) ** 2) / (r * 0.5) ** 2, 0, 1) * 0.5
        rows, cols = (np.arange(-12, 13) + cy) % N, (np.arange(-12, 13) + cx) % N
        p = img[np.ix_(rows, cols)]
        p = p * (1 - shadow[..., None]) + hexrgb("#7a5534") * shadow[..., None]
        p = p * (1 - body[..., None]) + hexrgb(["#a9a294", "#968f82", "#bdb5a5"][rng.integers(3)]) * body[..., None]
        p = p * (1 - light[..., None]) + hexrgb("#e8dcc4") * light[..., None]
        img[np.ix_(rows, cols)] = p
    return img


def wood_floor(rng):
    planks, img = 8, np.zeros((N, N, 3), np.float32)
    pw = N // planks
    grain = pnoise(rng, 30, aniso=(0.05, 1.0)) * 0.7 + pnoise(rng, 8, aniso=(0.05, 1.0)) * 0.3  # along the planks
    tones = ["#a8703f", "#b67d4a", "#9e6739", "#c08752"]
    for p in range(planks):
        x0 = p * pw
        first = int(rng.integers(0, N))
        seams = [first, first + int(rng.integers(380, 640))]  # two boards per column, both >= 380 px
        for s0, s1 in ((seams[0], seams[1]), (seams[1], seams[0] + N)):  # two boards per column, wrapping
            tone = hexrgb(tones[rng.integers(len(tones))]) * rng.uniform(0.95, 1.05)
            rows = np.arange(s0, s1) % N
            g = grain[np.ix_(rows, np.arange(x0, x0 + pw))][..., None]
            img[np.ix_(rows, np.arange(x0, x0 + pw))] = tone * (0.8 + 0.32 * g)
            img[s0 % N, x0:x0 + pw] *= 0.55  # board end seam
        img[:, x0:x0 + 2] *= 0.6  # plank side seam
    return np.clip(img, 0, 1)


def sand(rng):
    img = ramp(spread(pnoise(rng, 240) * 0.55 + pnoise(rng, 80) * 0.3 + pnoise(rng, 24) * 0.15, 0.18),
               ["#d6bb88", "#e2c999", "#ecd6a8", "#f4e3b9"])
    stamp(img, rng, 1800, 46, 5, 0.15, 0.25, ["#d9bf8c", "#f1deb2", "#cdb07c"], 0.22)   # wind ripples, one way
    stamp(img, rng, 700, 16, 12, 0.0, 3.0, ["#c9ab78", "#f6e8c4"], 0.18)               # soft blotches
    stamp(img, rng, 2600, 2.2, 1.6, 0.0, 3.0, ["#a88e66", "#fff6de", "#b9a07a"], 0.6)  # grains and shell bits
    return img


def stone(rng, cells=16, gap=0.07):
    """Cobbles: a jittered grid of periodic Voronoi cells, each a rounded stone, mortar between."""
    pts = [((i + rng.uniform(0.15, 0.85)) * N / cells, (j + rng.uniform(0.15, 0.85)) * N / cells)
           for i in range(cells) for j in range(cells)]
    yy, xx = np.mgrid[0:N, 0:N].astype(np.float32)
    d1 = np.full((N, N), 1e9, np.float32)
    d2 = np.full((N, N), 1e9, np.float32)
    idx = np.zeros((N, N), np.int32)
    for k, (py, px) in enumerate(pts):
        dy = np.abs(yy - py)
        dx = np.abs(xx - px)
        d = np.sqrt(np.minimum(dy, N - dy) ** 2 + np.minimum(dx, N - dx) ** 2)
        closer = d < d1
        d2 = np.where(closer, d1, np.minimum(d2, d))
        idx = np.where(closer, k, idx)
        d1 = np.where(closer, d, d1)
    cell = N / cells
    edge = (d2 - d1) / cell                          # 0 on the seams
    tones = np.stack([hexrgb(["#b9b1a3", "#a9a194", "#c7beae", "#b3a58f", "#9f988c", "#c2b49a"][rng.integers(6)])
                      * rng.uniform(0.93, 1.06) for _ in pts])
    body = tones[idx]
    dome = np.clip(1 - d1 / (cell * 0.7), 0, 1)[..., None]       # lighter crowns
    grain = (pnoise(rng, 30) * 0.6 + pnoise(rng, 8) * 0.4)[..., None]
    img = body * (0.82 + 0.22 * dome + 0.12 * (grain - 0.5))
    mortar = np.clip((gap - edge) / 0.03, 0, 1)[..., None]       # soft, slightly dark seams
    img = img * (1 - mortar) + hexrgb("#7d7366") * mortar * (0.9 + 0.2 * grain)
    stamp(img, rng, 900, 10, 6, 0.0, 3.0, ["#d6cdbb", "#8f877a"], 0.15)  # painterly wear
    return np.clip(img, 0, 1)


# ---------------------------------------------------------------- 1.3 regions


def specks(img, rng, count, rmin, rmax, colours, alpha=1.0, light=None):
    """Small round bits (cinders, shells, petals): soft discs, wrapped; optional top-left highlight."""
    R = int(rmax) + 2
    yy, xx = np.mgrid[-R:R + 1, -R:R + 1].astype(np.float32)
    for _ in range(count):
        cy, cx = rng.integers(0, N, 2)
        r = rng.uniform(rmin, rmax)
        body = np.clip(1 - (xx ** 2 + yy ** 2) / r ** 2, 0, 1) ** 0.6 * alpha
        rows, cols = (np.arange(-R, R + 1) + cy) % N, (np.arange(-R, R + 1) + cx) % N
        p = img[np.ix_(rows, cols)]
        p = p * (1 - body[..., None]) + hexrgb(colours[rng.integers(len(colours))]) * body[..., None]
        if light:
            hl = np.clip(1 - ((xx + r * 0.3) ** 2 + (yy - r * 0.3) ** 2) / (r * 0.45) ** 2, 0, 1) * 0.45
            p = p * (1 - hl[..., None]) + hexrgb(light) * hl[..., None]
        img[np.ix_(rows, cols)] = p


def cells(rng, n_x, n_y):
    """Periodic jittered Voronoi: (nearest index, edge distance normalised by the cell size), cells n_x by n_y."""
    pts = [((j + rng.uniform(0.15, 0.85)) * N / n_y, (i + rng.uniform(0.15, 0.85)) * N / n_x) for i in range(n_x) for j in range(n_y)]
    yy, xx = np.mgrid[0:N, 0:N].astype(np.float32)
    sx, sy = n_x / N, n_y / N                     # measure distances in cell units (anisotropic cells)
    d1 = np.full((N, N), 1e9, np.float32)
    d2 = np.full((N, N), 1e9, np.float32)
    idx = np.zeros((N, N), np.int32)
    for k, (py, px) in enumerate(pts):
        dy = np.abs(yy - py)
        dx = np.abs(xx - px)
        d = np.sqrt((np.minimum(dx, N - dx) * sx) ** 2 + (np.minimum(dy, N - dy) * sy) ** 2)
        closer = d < d1
        d2 = np.where(closer, d1, np.minimum(d2, d))
        idx = np.where(closer, k, idx)
        d1 = np.where(closer, d, d1)
    return idx, d2 - d1, len(pts)


def ash(rng):
    """Ember ground: warm grey ash in soft drifts, dark cinders, a few glass glints."""
    img = ramp(spread(pnoise(rng, 230) * 0.55 + pnoise(rng, 70) * 0.3 + pnoise(rng, 20) * 0.15, 0.2),
               ["#7f7a75", "#918b85", "#a39d96", "#b4aea6"])
    stamp(img, rng, 1400, 44, 9, 0.35, 0.4, ["#9a948d", "#b9b3ab", "#857f79"], 0.25)      # drifts, one wind
    stamp(img, rng, 700, 15, 11, 0.0, 3.0, ["#77716c", "#c2bcb3"], 0.2)                   # soft blotches
    specks(img, rng, 900, 2.0, 5.5, ["#4a4549", "#3d383c", "#5a5458"], 0.85, light="#8a8488")  # cinders
    specks(img, rng, 70, 1.2, 2.2, ["#e88a3a", "#7fd3c8", "#f0b24a"], 0.9)                 # ember and glass glints
    return img


def heath(rng):
    """Storm ground: olive heath and straw-gold grass, broken by soft clumps of purple heather."""
    img = ramp(spread(pnoise(rng, 220) * 0.55 + pnoise(rng, 60) * 0.3 + pnoise(rng, 18) * 0.15),
               ["#56653a", "#657444", "#74834c", "#879358"])
    stamp(img, rng, 7000, 12, 2.0, -math.pi / 2, 0.5, ["#5d6c3c", "#8c9858", "#b2a560", "#77864a"], 0.45)  # blades
    patch = spread(pnoise(rng, 110) * 0.7 + pnoise(rng, 20) * 0.3, 0.2)
    purple = np.clip((patch - 0.6) * 6, 0, 1)[..., None] * 0.85
    img = img * (1 - purple) + ramp(pnoise(rng, 10), ["#6a476e", "#875a86", "#a5739f"]) * purple
    specks(img, rng, 2600, 1.4, 2.8, ["#b383b3", "#9a6aa0", "#c99ac4", "#6f7d45"], 0.8)     # heather bells
    return img


def wet_sand(rng):
    """Tide flats: cool, darker sand, ripple marks, shining films of water, shells."""
    img = ramp(spread(pnoise(rng, 240) * 0.55 + pnoise(rng, 80) * 0.3 + pnoise(rng, 24) * 0.15, 0.18),
               ["#9c8a6a", "#ab9877", "#b9a684", "#c6b391"])
    yy, xx = np.mgrid[0:N, 0:N].astype(np.float32)
    warp = pnoise(rng, 120) * 40
    rip = (np.sin((yy + warp) * math.tau / (N / 32)) * 0.5 + 0.5)[..., None]   # 32 ripples per repeat: tiles
    img = img * (0.92 + 0.1 * rip)
    film = np.clip((pnoise(rng, 70) - 0.62) * 5, 0, 1)[..., None] * 0.45
    img = img * (1 - film) + hexrgb("#c9d6cf") * film                         # puddle shine
    specks(img, rng, 500, 1.5, 3.5, ["#f2e8d8", "#e9d2c0", "#8a7a66"], 0.85, light="#ffffff")  # shells, grit
    return img


def dusk_grass(rng):
    """Dream ground: blue-tinted twilight grass with pale lavender flecks."""
    img = ramp(spread(pnoise(rng, 220) * 0.55 + pnoise(rng, 70) * 0.3 + pnoise(rng, 20) * 0.15),
               ["#335c55", "#3f6e64", "#4d7f71", "#5f9182"])
    stamp(img, rng, 2400, 30, 9, 0.3, 3.0, ["#3a655d", "#62958a", "#4a776e", "#55688a"], 0.26)   # broad dabs, a few blue
    stamp(img, rng, 11000, 13, 2.2, -math.pi / 2, 0.45, ["#2e544f", "#6c9f92", "#507f74", "#86b3a5"], 0.55)
    for _ in range(2):  # clover-like clusters of pale lavender flowers
        specks(img, rng, 420, 1.6, 3.2, ["#c3b2e6", "#a996d6", "#e5dcf5"], 0.85)
    return img


def moss(rng):
    """Grove ground: deep cushiony moss, lighter cushions, leaf litter."""
    img = ramp(spread(pnoise(rng, 200) * 0.5 + pnoise(rng, 50) * 0.3 + pnoise(rng, 14) * 0.2, 0.24),
               ["#335f24", "#3e6f2a", "#4b7f31", "#5b8f3a"])
    specks(img, rng, 1300, 5, 13, ["#5f973a", "#6ea544", "#4f8732"], 0.5)                   # cushions
    stamp(img, rng, 6000, 6, 1.6, 0.0, 3.0, ["#78ad4a", "#2f5a22", "#8cbf58"], 0.45)        # fronds
    specks(img, rng, 260, 2.0, 4.0, ["#8a6a3a", "#a5803f", "#6e5230"], 0.8)                 # leaf litter
    return img


def slate(rng):
    """Stone ground: blue-grey slate flags, angular and elongated, dark seams, faint rust."""
    idx, edge, n = cells(rng, 5, 8)
    tones = np.stack([hexrgb(["#6d7178", "#7a7e85", "#858990", "#686c72", "#7f8188"][rng.integers(5)])
                      * rng.uniform(0.93, 1.06) for _ in range(n)])
    grain = (pnoise(rng, 40, aniso=(0.25, 1.0)) * 0.6 + pnoise(rng, 8) * 0.4)[..., None]   # cleavage streaks
    img = tones[idx] * (0.86 + 0.24 * grain)
    seam = np.clip((0.05 - edge) / 0.03, 0, 1)[..., None] * 0.8
    img = img * (1 - seam) + hexrgb("#53565c") * seam
    img = img * 0.55 + ramp(spread(pnoise(rng, 160) * 0.6 + pnoise(rng, 30) * 0.4), ["#666a71", "#767a81", "#858990"]) * 0.45
    rust = np.clip((pnoise(rng, 60) - 0.7) * 4, 0, 1)[..., None] * 0.25
    img = img * (1 - rust) + hexrgb("#a8642f") * rust
    stamp(img, rng, 900, 18, 2.5, 0.0, 0.2, ["#9a9ea4", "#5c6066"], 0.18)                   # chisel scuffs
    specks(img, rng, 600, 1.5, 4, ["#5a5e64", "#9a9ea4", "#6f6a66"], 0.8, light="#b5b9be")  # chips and grit
    return np.clip(img, 0, 1)


def cave(rng):
    """Undermine floor: dark damp rock, grit and pebbles, wetter hollows."""
    img = ramp(spread(pnoise(rng, 180) * 0.55 + pnoise(rng, 50) * 0.3 + pnoise(rng, 14) * 0.15, 0.22),
               ["#38322c", "#433c35", "#4f473f", "#5c5349"])
    damp = np.clip((pnoise(rng, 90) - 0.55) * 4, 0, 1)[..., None] * 0.35
    img = img * (1 - damp) + hexrgb("#2a2622") * damp
    specks(img, rng, 700, 2.5, 7, ["#5e564d", "#6b6258", "#4a433c"], 0.9, light="#8a8074")
    specks(img, rng, 2500, 0.8, 1.6, ["#756c61", "#2c2722"], 0.7)
    return img


def save(img, path):
    rgba = np.concatenate([np.clip(img, 0, 1), np.ones((N, N, 1), np.float32)], axis=2)
    im = bpy.data.images.new(os.path.basename(path), N, N)
    im.pixels.foreach_set(rgba[::-1].astype(np.float32).ravel())  # Blender rows are bottom-up
    im.filepath_raw = path
    im.file_format = "PNG"
    im.save()
    print("[ground]", path)


def main(argv):
    out = os.path.abspath(argv[argv.index("--out") + 1]) if "--out" in argv else "web/world/textures"
    only = argv[argv.index("--only") + 1].split(",") if "--only" in argv else None
    os.makedirs(out, exist_ok=True)
    for name, fn, seed in (("grass", grass, 1), ("dirt", dirt, 2), ("wood_floor", wood_floor, 3),
                           ("sand", sand, 4), ("stone", stone, 5),
                           ("ash", ash, 6), ("heath", heath, 7), ("wet_sand", wet_sand, 8), ("dusk_grass", dusk_grass, 9),
                           ("moss", moss, 10), ("slate", slate, 11), ("cave", cave, 12)):
        if not only or name in only:
            save(fn(np.random.default_rng(seed)), os.path.join(out, name + ".png"))


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
