# Card illustrations

`web/cards/<slug>.webp` holds one painted illustration per card (110). The slug is the card's
English identity name, lowercased, with every run of non-letters/digits turned into `-`
(`Trail Compass` → `trail-compass.webp`), the same rule as `slug` in `web/src/art.rs`.
`web/build.rs` lists and hashes whatever is there; a card without a file falls back to its
procedural SVG.

## How they are made

- **Model:** FLUX 3 Image Klein (distilled), the pre-quantized 4-bit MLX build
  [`mflux-community/flux-3-image-klein-mflux-q4`](https://huggingface.co/mflux-community/flux-3-image-klein-mflux-q4)
  (~5 GB, downloaded to `~/.cache/huggingface` on first run), run locally with
  [mflux](https://github.com/filipstrand/mflux) 0.23 on Apple Silicon.
- **Settings:** 16 steps, float16 compute, 800×560 (the 10:7 art box), one fixed seed per card.
  About 55 s per image on an M1-class Mac.
- **Prompts:** `prompts.json` holds a full hand-written caption per card (the long, descriptive
  form FLUX 3 is trained on): the shared style lead (painterly gouache-and-watercolour storybook
  art on cold-press paper, warm light from the upper left, one subject, a soft wash in the card's
  colour behind it), then the subject. Evolution lines repeat the same palette and markings so a
  Basic, its Stage 1 and its Stage 2 read as one species growing. `_groups` orders the cards for
  the contact sheets.
- **Output:** the PNG goes to `tools/art/out/` (git-ignored); the WebP (best quality ≤ 60 KB)
  goes straight to `web/cards/`.

## Licence notes

- The weights are **not** in this repository. FLUX 3 Image Klein is published by Black Forest
  Labs under its community licence, which allows using the generated images in this free,
  non-commercial project. Re-check the licence before any commercial use.
- Every creature, object and caption is original. Captions never name or imitate an existing
  franchise, character or artist, and images are reviewed and redrawn if one looks like a known
  creature (see `CLAUDE.md`, "Legal scope").

## Redraw one card

```bash
uv tool install mflux==0.23.*          # once; gives ~/.local/share/uv/tools/mflux/bin/python
PY=~/.local/share/uv/tools/mflux/bin/python

# edit the card's "caption" and/or bump its "seed" in tools/art/prompts.json, then:
$PY tools/art/gen.py --cards "Trail Compass"

$PY tools/art/gen.py             # draws every card that has no PNG in tools/art/out/ yet
$PY tools/art/gen.py --contact   # rebuilds tools/art/out/contact-<group>.png only
```

Look at the result (the contact sheets help) before committing the new `web/cards/*.webp`: no
extra limbs, no lettering, one subject, same style as its neighbours.
