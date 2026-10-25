# Roadmap

Releases are cut by release-please. **A new minor version ships after every three merged `feat:` or `fix:`
pull requests** (`docs:`, `chore:` and `ci:` don't count), so each version below is three issues and three PRs.
Every PR is opened as a draft as soon as its branch has work. Before each release, an `/impeccable` audit covers
what its three PRs touched; 1.4 and 2.0 get a complete audit of every screen and the world.

## Released

| Version | Phase |
|---|---|
| 0.1.0 | Card duel against the AI: full rules engine, original card set, two prebuilt decks, match UI |
| 0.2.0 | The world: tile-grid overworld, enterable duel hall, NPCs and dialogue, running shoes, saved progress |
| 0.3.0 | Collection and deck builder: rarities, original booster packs, the player's collection and saved decks |
| 0.4.0 | A larger original card set (about a hundred cards), more prebuilt decks and AI deck variety |
| 0.5.0 | Sound and feel: original music and sound effects, settings |
| 0.6.0 | Campaign: six rival duelists and the Hex Warden in the world, duels started from dialogue, seals, pack rewards and a champion's room |
| 0.7.0 | A bigger world: five outdoor areas joined by their edges, cliffs and decks, more buildings and NPCs, two small quests |
| 0.8.0 | Saves and slots with export / import, offline play (PWA), a Hard AI that looks ahead, an accessibility pass, a smaller wasm |
| 0.9.0 | Content complete: credits and the champion's title, the Gauntlet and a daily duel, seven promo cards, a balance pass over every deck, a full campaign playthrough test |
| 1.0.0 | Final polish and release: a last audit of every screen, accessibility and console fixes, a Low Tide balance nudge |
| 1.1.0 | French: the whole game in French or English, original French card names, an EN / FR switch |
| 1.2.0 | The world comes alive: Facets, shops, route duelists, finds, houses and quests (`docs/WORLD_DESIGN.md`) |
| 1.3.0 | Six regions: the hub and its ring of six regions, day and night, gates and field items, fast travel; the world re-laid to the world rules. Shipped 2026-10-25 |

## Planned (1.4 to 2.0)

The story ("The Seventh Face", `docs/WORLD_DESIGN.md` §8) was planned as 1.4. It moves to **1.7**, after the
reliability and game-feel releases, and duel juice moves to 1.6.

| Version | Theme | Issues / PRs |
|---|---|---|
| 1.4.0 | Finish 1.3 | #48 `feat(duel): a redesigned duel screen` · #55 `fix: 1.3 design audit and loose ends` · #56 `feat(ui): an in-game menu` |
| 1.5.0 | Reliability | #57 `feat: play fully offline from a fresh install, with a real loading screen` · #58 `feat(saves): survive corruption and test every migration` · #59 `test: end-to-end browser tests of the main journeys` |
| 1.6.0 | Game feel | #60 `feat(ui): screen transitions and one visual grammar` · #61 `feat(duel): juice` · #62 `feat(world): dialogue polish, current objective and clear unlocks` |
| 1.7.0 | Story | #63 `feat(world): cutscene scripts` · #64 `feat(story): Rook and the seventh face` · #65 `feat: stats and achievements` |
| 1.8.0 | Presentation | #66 `feat: a studio splash and a living title screen` (Atypical Play → Sixfold → press to start) · #67 `feat: a final victory sequence and staged credits` · #68 `feat(cards): a binder worth completing and better pack openings` |
| 1.9.0 | Controls and accessibility | #69 `feat: gamepad support` · #70 `feat(a11y): readable, high-contrast, screen-reader-friendly` · #71 `feat(audio): a soundscape per region and music for rivals and bosses` |
| 2.0.0 | Release | #72 `chore: release readiness` (a checklist: blind playtests, real-phone performance, a full EN/FR read-through, a rights audit, press material, a complete audit, every platform, a full run); the fixes it finds are its PRs |

Each version is also a GitHub milestone of the same name.

After 2.0: a second human player (two people on one device, or over the network).

Constraints that hold for every phase: only the rules and mechanics of the classic handheld game are
reproduced; every name, card, character, place and asset is original (see `CLAUDE.md`).
