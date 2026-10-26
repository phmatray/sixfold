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
| 1.9.0 | Accessibility, menus and a world to live in | #94 `feat(a11y): readable, high-contrast, screen-reader-friendly` (#70) · #96 `feat(ui): screen transitions and one visual grammar` (#60) · #101 `feat(ui): a title that shows the world, and game-like menus` (#100, with the painted kit) · #103 `feat(decks): a deck builder with an auto mode, suggestions and unlockable decks` (#102) · #106 `feat(world): books you can take from the shelves` (#104) · #107 `feat(world): sit on benches and chairs` · #108 `feat(world): a challenge panel that stays in the world` · #109 `fix(world): the hero climbs porch steps` · #126 `feat(world): doors that open on every building, and model fixes` · #128 `feat: a logo for Sixfold` · #138 `fix: 1.9 design audit and stable browser tests`. Gamepad (#69) shipped in 1.7 and the soundscape (#71) in 1.5 |
| 1.10.0 | World foundations and seal keepers | #110 `feat(tools): check-maps v2 — blocking and advisory level-design checks` · #111 `feat(tools): world debug overlays` · #112 `feat(world): Fernhollow Trail, the reference map` (after #149) · the seal keepers, #140 `feat(world): seal keepers you have to earn, and an epic seal duel`: #149 `the six seal keepers stand at their seal sites` · #150 `a seal keeper accepts only once you've beaten enough of their region's duelists` · #151 `a seal duel looks like one` · #152 `an arrival and an entrance at every seal site` · #153 `the seal is won on stage, and a defeat is met with respect` · #154 `the Warden's duel is the grandest` · #155 `the seal-duel staging, polished to an independent critique` |
| 1.11.0 | A living world | #113 `feat(world): NPC routines` · #114 `feat(world): reactions and persistence` · #115 `feat(world): ambient life and micro-events` · #148 `feat(world): three camera angles to choose from` (40°, 50° and 60°) |
| 1.12.0 | Secrets and connections | #116 `feat(world): secrets` · #117 `feat(world): loops, shortcuts and an honest world map` · #118 `feat(world): biome transitions on every border` |
| 1.13.0 | Regions to the Fernhollow standard (1) | #119 `feat(world): the meadow and Grove to the Fernhollow standard` · #120 `feat(world): Ember and Stone to the Fernhollow standard` · #121 `feat(world): Tide and Storm to the Fernhollow standard` |
| 1.14.0 | Regions (2) and validation | #122 `feat(world): Dream to the Fernhollow standard` · #123 `feat(world): discovery staging for major places` · #124 `fix(world): the five-pass playtest and its fixes` |
| 1.15.0 | Base-set quick wins | #129 `feat(cards): eight new cards for the base set` · #130 `fix(cards): resistance errata and real prices for dormant mechanics` · #131 `feat(cards): set marks and Rare Basics in packs` |
| 1.16.0 | Crossroads | #132 `feat(engine): the Crossroads mechanics and rules` · #133 `feat(cards): the 64 Crossroads cards with lore, bestiary and art` · #134 `feat(world): Crossroads packs, unlocks and rival lists for the Open campaign` |
| 1.17.0 | Deep Seals | #135 `feat(engine): the Deep Seals mechanics, one-per-deck rule and AI for locks` · #136 `feat(cards): the 62 Deep Seals cards with lore, bestiary and art` · #137 `feat(world): the Sleepers quest, Deep Seals packs, relic finds and rematch lists` |
| 2.0.0 | Release | #72 `chore: release readiness` (a checklist: blind playtests, real-phone performance, a full EN/FR read-through, a rights audit, press material, a complete audit, every platform, a full run); the fixes it finds are its PRs |

**1.10 to 1.14 are about the world.** They follow the owner's level-design guide, `docs/LEVEL_DESIGN.md`:
first the tools and a reference map (Fernhollow Trail), then life, then secrets and connections, then every
region brought to the reference's standard, and finally a whole-world five-pass playtest.
`docs/WORLD_RULES.md` stays the source of truth for placement, and check-maps v2 enforces the new rules map by
map as each region opts in. 1.10 also brings the seal keepers to their seal sites (#140); #149 lands before
#112, since it takes Bramble and Lumen off Fernhollow Trail.

**1.15 to 1.17 are about the cards** (`docs/SET_DESIGN.md`): quick wins on the base set, then the two
expansions, Crossroads and Deep Seals, so both ship before 2.0.

Each version is also a GitHub milestone of the same name.

After 2.0: a second human player (two people on one device, or over the network).

Constraints that hold for every phase: only the rules and mechanics of the classic handheld game are
reproduced; every name, card, character, place and asset is original (see `CLAUDE.md`).
