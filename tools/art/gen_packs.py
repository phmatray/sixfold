"""Sixfold booster key art: FLUX 3 Image Klein through mflux, like gen.py (see README.md).

  $PY tools/art/gen_packs.py             # every pack in packs.json without a picture
  $PY tools/art/gen_packs.py fire wild   # redraw these (change their seed/caption first)

The PNG goes to tools/art/out/pack-<theme>.png, the WebP (<= 80 KB) to web/packs/<theme>.webp.
"""
import sys, json, time
from gen import HERE, REPO, OUT, MODEL, STEPS, webp

W, H = 512, 816  # the pack wrapper is about 5:8
PACKS = REPO / "web" / "packs"

if __name__ == "__main__":
    import mlx.core as mx
    from mflux.models.common.config import ModelConfig
    from mflux.models.flux3.variants import Flux3Klein
    packs = json.loads((HERE / "packs.json").read_text())
    todo = sys.argv[1:] or [k for k in packs if not (OUT / f"pack-{k}.png").exists()]
    OUT.mkdir(exist_ok=True); PACKS.mkdir(exist_ok=True)
    flux = Flux3Klein(model_config=ModelConfig.flux3_image_klein(), model_path=MODEL, compute_precision=mx.float16)
    for k in todo:
        p, png = packs[k], OUT / f"pack-{k}.png"
        t = time.time()
        flux.generate_image(seed=p["seed"], prompt=p["caption"], num_inference_steps=STEPS, width=W, height=H).save(str(png))
        q, size = webp(png, PACKS / f"{k}.webp", limit=80_000)
        print(f"{k}: {time.time() - t:.0f}s, webp q{q} {size / 1000:.1f} KB", flush=True)
