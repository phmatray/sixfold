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

## Safe area: where the picture is cropped

The art is 800×560 (10:7). How much of it shows depends on the card's size (measured in 1.3.x at
1440×900 and 390×844):

| Where | Card width | Window | What shows |
|---|---|---|---|
| Full card: binder, inspector, pack reveal, shop | ≥ 140 px | fixed 10:7 | **the whole picture** |
| Compact card: hand, Active and Bench on desktop, shop thumbnails | 48–139 px | about 1:1 (0.84–1.03) | x 15–85 % (energy, which has no stage line, x 21–79 %), full height |
| Picture only: Bench on a phone, discard top, piles | ≤ 47 px | 5:7 (0.70) | x 25–75 %, full height |

The picture is centred (`object-position: 50% 50%`) and never cropped top or bottom. So, as
insets from each edge of the 800×560 frame:

- **Safe area — the whole subject** (body, tail, wings, the object's outline): **18 % left and
  right, 4 % top and bottom** (x 144–656, y 22–538). Everything inside shows on every card from the
  hand up.
- **Core — what must read even as a thumbnail** (the face, or the object's key part): **27 % left
  and right** (x 216–584), full height.
- Outside the safe area: background wash and ground only; it shows on full cards and is cropped
  away on small ones.

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
