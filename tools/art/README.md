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
  form FLUX 3 is trained on). `_groups` orders the cards for the contact sheets.
  - **Hexons** (the 78 creatures) are drawn from their species sheets in `docs/bestiary/`
    (see `docs/BESTIARY.md`): each caption is written from the sheet's morphology, colours, habitat
    and visual key. The shared lead asks for a fantasy creature-design painting in gouache and
    watercolour with ink linework, an invented species and not a real animal; then a composition
    sentence keeps the whole Hexon in the safe area (below); then the subject (pose, anatomy with
    exact counts, type material, the line's invented organ) and the light. Awaken 2, rares and promos
    get a low angle and backlight. An Awaken line repeats colours, material and signature organ so
    it reads as one species awakening. No Hexon carries a hexagon mark or a mark shared across species
    (no common chest disc or emblem). Colorless Hexons say "no fur, no feathers and no skin" and describe a
    body sewn from patchwork felt with bold stitched seams, so the felt is structural, not a texture on a real animal.
  - **Trainers and energy** keep the storybook still-life lead: painterly gouache and watercolour on
    cold-press paper, warm light from the upper left, one subject, a soft wash in the card's colour.
- **Output:** the PNG goes to `tools/art/out/` (git-ignored); the WebP (best quality ≤ 60 KB)
  goes straight to `web/cards/`.

## Safe area: where the picture is cropped

The art is 800×560 (10:7). How much of it shows depends on the card's size (measured at 1440×900
and 390×844 with the stylesheet of the boosters release):

| Where | Card width | Window (wide:tall) | What shows |
|---|---|---|---|
| Full card: binder, inspector, pack reveal | ≥ 140 px | fixed 10:7 (1.43) | **the whole picture** |
| Compact card: hand, Active and Bench on desktop, the Active on a phone | 48–139 px | 0.84–1.03 | x 15–85 % (energy, which has no stage line, x 21–79 %), full height |
| Picture only: Bench on a phone, the discard's top card | ≤ 47 px (the discard: any size) | fixed 1:1 | x 15–85 %, full height |

Every window is centred (`object-position: 50% 50%`), and none crops the top or bottom.

**Safe area: the whole subject** (body, tail, wings, antlers, the object's outline) stays inside
**the central 50 % of the width and 90 % of the height**, that is x 200–600 and y 28–532 on the 800×560
image (insets of 25 % left and right, 5 % top and bottom). That is the area the Hexon art is composed
in. It shows on every card, with at least 10 % to spare on each side, even in the narrowest window
(compact energy). Outside it go the background wash and the ground only: they show on full cards and
are cropped away on small ones.

## Booster key art

`web/packs/<theme>.webp` (fire, water, grass, lightning, fighting, psychic, wild) is the painting in
each booster wrapper's window: same model, settings and style lead, 512×816 (about the wrapper's 5:8),
text-free, a six-sided crystal in each region's landscape. Captions and seeds are in `packs.json`;
`gen_packs.py` draws them (WebP ≤ 80 KB, precached with the app shell):

```bash
$PY tools/art/gen_packs.py             # every pack without a PNG in tools/art/out/
$PY tools/art/gen_packs.py fire wild   # redraw these
```

The window is nearly square, so it shows the full width and about two thirds of the height: roughly
19 % is cut from the top (the calm wash) and 14 % from the bottom (`object-position: 50% 58%`). Keep
the crystal within y 25–80 %.

**Hexons aim tighter:** the whole Hexon (ears, tails, crests, veils included) sits in x 200–600,
y 28–532, so it survives even the picture-only crop. That means seated, perched, coiled or front
three-quarter poses, tails curled up, wings folded. Side-on quadrupeds, crouches, running poses and
spread wings or veils always spill out; redraw them.

## Licence notes

- The weights are **not** in this repository. FLUX 3 Image Klein is published by Black Forest
  Labs under its community licence, which allows using the generated images in this free,
  non-commercial project. Re-check the licence before any commercial use.
- Every Hexon, object and caption is original. Captions never name or imitate an existing
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
extra limbs, no lettering, one subject, same style as its neighbours, the whole Hexon inside the
safe area, nothing that reads as a real animal, a human figure, a hexagon or a shared emblem.
