"""Sixfold card illustrations: FLUX 3 Image Klein (4-bit MLX build) through mflux, on Apple Silicon.

Run with the mflux tool's Python (see README.md):
  PY=~/.local/share/uv/tools/mflux/bin/python
  $PY tools/art/gen.py                      # every card in prompts.json that has no picture yet
  $PY tools/art/gen.py --cards "Trail Compass"   # redraw one card (change its seed/caption first)
  $PY tools/art/gen.py --contact            # contact sheets only, no generation

Each card's full caption lives in prompts.json, keyed by its English name. The PNG goes to
tools/art/out/ (git-ignored), the WebP the game loads to web/cards/<slug>.webp.
"""
import argparse, io, json, time
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
OUT, CARDS = HERE / "out", REPO / "web" / "cards"
MODEL = "mflux-community/flux-3-image-klein-mflux-q4"
W, H, STEPS = 800, 560, 16  # the card art box is 10:7; FLUX needs multiples of 16


def slug(name):
    """Same as web/src/art.rs `slug`: ASCII letters/digits lowercased, every other run -> one '-'."""
    s = "".join(c.lower() if c.isascii() and c.isalnum() else "-" for c in name)
    return "-".join(w for w in s.split("-") if w)


def webp(png, dst, limit=60_000):
    """Highest quality WebP (step 5) that fits the size limit."""
    img = Image.open(png).convert("RGB")
    for q in range(90, 25, -5):
        buf = io.BytesIO(); img.save(buf, "WEBP", quality=q, method=6)
        if buf.tell() <= limit:
            break
    dst.write_bytes(buf.getvalue())
    return q, buf.tell()


def contact(names, dst, cols=4, tw=400, th=280, label=30):
    sheet = Image.new("RGB", (cols * tw, -(-len(names) // cols) * (th + label)), "white")
    d = ImageDraw.Draw(sheet)
    try:
        font = ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial Bold.ttf", 18)
    except OSError:
        font = ImageFont.load_default()
    for i, n in enumerate(names):
        x, y = (i % cols) * tw, (i // cols) * (th + label)
        p = OUT / f"{slug(n)}.png"
        if p.exists():
            sheet.paste(Image.open(p).convert("RGB").resize((tw, th), Image.LANCZOS), (x, y))
        d.text((x + 8, y + th + 5), n, fill="black", font=font)
    sheet.save(dst)


def sheets(cfg):
    """One sheet per group in prompts.json (`_groups`: colour lines in evolution order, Gear cards, energy)."""
    for group, names in cfg["_groups"].items():
        contact(names, OUT / f"contact-{group}.png")
        print(f"contact: {OUT / f'contact-{group}.png'}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--cards", nargs="+", help="redraw these cards even if they have a picture")
    ap.add_argument("--contact", action="store_true", help="contact sheets only")
    a = ap.parse_args()
    OUT.mkdir(exist_ok=True)
    read = lambda: json.loads((HERE / "prompts.json").read_text())
    if a.contact:
        return sheets(read())
    import mlx.core as mx
    from mflux.models.common.config import ModelConfig
    from mflux.models.flux3.variants import Flux3Klein
    flux = Flux3Klein(model_config=ModelConfig.flux3_image_klein(), model_path=MODEL,
                      compute_precision=mx.float16)
    todo = list(a.cards or [])
    while True:
        # Re-read every round so captions added (or fixed) while this runs are picked up.
        cards = read()["cards"]
        if a.cards:
            if not todo:
                break
            name = todo.pop(0)
        else:
            name = next((n for n in cards if not (OUT / f"{slug(n)}.png").exists()), None)
            if name is None:
                break
        card, png = cards[name], OUT / f"{slug(name)}.png"
        t = time.time()
        flux.generate_image(seed=card["seed"], prompt=card["caption"], num_inference_steps=STEPS,
                            width=W, height=H).save(str(png))
        q, size = webp(png, CARDS / f"{slug(name)}.webp")
        print(f"{name}: seed {card['seed']}, {time.time() - t:.0f}s, webp q{q} {size / 1000:.1f} KB", flush=True)
    sheets(read())


if __name__ == "__main__":
    assert slug("Trail Compass") == "trail-compass" and slug(" Bark  Shield's ") == "bark-shield-s"
    assert slug("Énergie Feu") == "nergie-feu"  # non-ASCII is not alphanumeric in art.rs either
    main()
