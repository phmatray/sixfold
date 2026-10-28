---
name: readme-refresh
description: Bring Sixfold's README up to date after a release — what the game is now, fresh screenshots of the live build, the GitHub social card, every number checked against the code — and open a `docs:` PR. Use whenever a version has shipped and the README lags, or the user asks to review, refresh, update or rewrite the README, its screenshots, its banner or the repo's social image, including « mets à jour le README », « revois le readme pour la 1.11 », « refais les captures d'écran », « la bannière n'est plus à jour ». Not for CHANGELOG.md (release-please writes it) or docs/ROADMAP.md.
---

# README refresh

The README is the game's shop window: a visitor should see what Sixfold *is today* and want to
press Play. It drifts because each release adds features and numbers (cards, maps, tests) without
touching it. A refresh brings four things back in line: the text, the screenshots, the banner, and
the facts behind both.

Work on a branch `docs/readme-<version>` and finish with one PR titled
`docs: refresh the README for <version>`. A `docs:` PR doesn't count toward the next release.

## 1. Find what changed

- Current version: `grep '^version' Cargo.toml`.
- The last refresh: `git log --format='%h %s' -- README.md | head`.
- What shipped since: the `### Added` / `### Fixed` lines of `CHANGELOG.md` for each version after
  it, and the version's row in `docs/ROADMAP.md`.

Decide which changes a player would notice. A new region, mode, screen or control belongs in the
README; a tooling or audit PR doesn't.

## 2. Check every fact against the code

The README states numbers and claims; each one must still be true. Re-derive them, never copy them
from the old README or the changelog's wording:

| Claim | Source |
|---|---|
| Engine test count ("450+") | `cargo test -p sixfold-engine 2>&1 \| grep '^test result'`, first line; round down to a tens |
| Cards: total, Hexons, Gear cards, energies, promos | `engine/src/set.rs`; the Collection screen's "of N cards" and its per-kind progress |
| Outdoor maps, waymarks, regions | `web/world/atlas.json` (`maps`, `waymarks`, `regions`) |
| Route duelists | distinct `"t_…"` ids in `engine/src/duelists.rs` |
| Seal keepers, Warden, unlock thresholds | `engine/src/campaign.rs`, `docs/WORLD_DESIGN.md` |
| Achievements, books | the title menu ("0 of N achievements"), the Journal's Library |
| Controls | `web/gamepad.js` header, the world's key hint line |
| Rules deviations | `docs/RULES.md` |
| How the art is made | `tools/art/README.md` |

When the README and the code disagree, the code wins; mention the correction in the PR body. The
legal section must stay exact: rules and mechanics only, nothing from Nintendo / Game Freak / The
Pokémon Company, art original and how it's made, the ROM git-ignored.

## 3. Retake the screenshots

`e2e/readme-shots.mjs` drives the installed Google Chrome (in a visible window, since headless has
no GPU and draws the 3D world blank) through each scene as a new player and writes
`docs/screenshots/<scene>.jpg`:

```bash
cd e2e && npm ci && node readme-shots.mjs                          # the live site (after the release deployed)
cd e2e && node serve.mjs 4173 & node readme-shots.mjs http://127.0.0.1:4173/   # a local `trunk build --release`
```

Prefer the live site once the release's Pages deploy is green: the screenshots then show exactly
what a visitor gets.

Open every JPEG and look. A scene can fail silently: a cutscene or tutorial that now sits on top,
a menu opened by a stray key, a duel caught mid-animation, a renamed button that made the script
throw (`FAIL <scene>`, exit code 1). Fix the scene in the script (labels come from the accessible
names, the way `e2e/helpers.ts` finds them) rather than hand-editing images, so the next refresh
inherits the fix. When the release's headline is visual (a new region, a seal duel, a new screen),
add a scene for it and a cell to the README's screenshot table. Keep each file well under 2 MB
(CI fails over that); the JPEGs run 120–270 KB.

## 4. Refresh the social card

The banner at the top of the README is also the repo's GitHub social preview. It's drawn with the
`social-card` skill; its source is `~/repo/social-previews/cards/phmatray/sixfold.html` (FIG. 1:
the duel table in plan view). Run that skill as a **refresh**: update the promise, chips or legend
only when the game changed shape, then render.

Differences from that skill's step 4: copy the render to `.github/banner.png` in *this* branch
instead of running its `publish.sh`, which would open a second PR. Still do its step 3 (upload in
GitHub settings through Claude in Chrome, then the GraphQL check). If the Chrome extension isn't
connected, say so and leave the upload to the user with the PNG's path.

`web/social-preview.png` is a different image: the game's own Open Graph card for the Pages site
(logo on navy). Leave it unless the logo or tagline changed.

## 5. Write the README

Keep the structure, which reads top to bottom as: banner → logo, play link, badges → pitch →
title screenshot → screenshot table → what's in the game → how to play (with the controls table)
→ rules → build from source and the project layout → scope and legal → licence.

- Describe the game as it is now, by area (duel, cards, collection and decks, world, campaign,
  for everyone). No "What's new in 1.x" sections: the CHANGELOG holds history, and per-version
  lists are what made the README go stale.
- Plain, concrete sentences in the voice of the existing text: say what the player does and sees,
  with real names (regions, places, characters) from the code.
- Alt text on every screenshot says what's in the picture.
- Everything in English (the game is bilingual; the README isn't).

## 6. Check and ship

```bash
bash scripts/check-no-game-data.sh     # no ROM, no file over 2 MB
```

Commit (Conventional Commit, `docs:`), push, and open the PR with a short body: what changed, which
screenshots, any fact the code corrected, and whether the social preview upload was done. Then
report the PR link to the user.
