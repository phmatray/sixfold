"""Bake seamless, painterly base-colour textures for the splat ground (headless only).

    blender -b --factory-startup -P tools/blender/bake_ground_textures.py -- --out web/world/textures

grass.png, dirt.png, wood_floor.png: 1024 x 1024, tileable (all noise is periodic, every stroke
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
    os.makedirs(out, exist_ok=True)
    for name, fn, seed in (("grass", grass, 1), ("dirt", dirt, 2), ("wood_floor", wood_floor, 3)):
        save(fn(np.random.default_rng(seed)), os.path.join(out, name + ".png"))


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
