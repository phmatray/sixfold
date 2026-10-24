# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/). Entries from 0.1.0 on are written by
release-please from Conventional Commits.

## [1.0.0](https://github.com/phmatray/sixfold/compare/v0.9.0...v1.0.0) (2026-10-24)


### Added

* final polish for the 1.0 release ([#18](https://github.com/phmatray/sixfold/issues/18)) ([2c7eac5](https://github.com/phmatray/sixfold/commit/2c7eac5afe9700f1a46ce878aa1cc78093bdb11d))

## [0.9.0](https://github.com/phmatray/sixfold/compare/v0.8.0...v0.9.0) (2026-10-24)


### Added

* **endgame:** credits, a post-game Gauntlet, daily duels, promo cards and a balance pass ([#16](https://github.com/phmatray/sixfold/issues/16)) ([55e95e1](https://github.com/phmatray/sixfold/commit/55e95e10bc39eeb23d7662092b0fb09e18465c68))

## [0.8.0](https://github.com/phmatray/sixfold/compare/v0.7.0...v0.8.0) (2026-10-24)


### Added

* **polish:** save slots, offline play, a Hard AI and an accessibility pass ([#14](https://github.com/phmatray/sixfold/issues/14)) ([f8285c2](https://github.com/phmatray/sixfold/commit/f8285c2c47f0961d5dfe1a8536ccc375549b2ce5))

## [0.7.0](https://github.com/phmatray/sixfold/compare/v0.6.0...v0.7.0) (2026-10-24)


### Added

* **world:** open four themed areas around the meadow with side quests ([#12](https://github.com/phmatray/sixfold/issues/12)) ([850d5e8](https://github.com/phmatray/sixfold/commit/850d5e8eb7b995e063931f905bb493b35a2527a6))

## [0.6.0](https://github.com/phmatray/sixfold/compare/v0.5.0...v0.6.0) (2026-10-24)


### Added

* **campaign:** duel six rivals and the Hex Warden from the world for seals and packs ([#10](https://github.com/phmatray/sixfold/issues/10)) ([b9bc313](https://github.com/phmatray/sixfold/commit/b9bc313a847285e406a953cf306d137ca59a12dd))

## [0.5.0](https://github.com/phmatray/sixfold/compare/v0.4.0...v0.5.0) (2026-10-24)


### Added

* **audio:** add original synthesised music and sound effects ([#8](https://github.com/phmatray/sixfold/issues/8)) ([cbaeee1](https://github.com/phmatray/sixfold/commit/cbaeee14f0e8d03fb898972e02e584055f07d306))

## [0.4.0](https://github.com/phmatray/sixfold/compare/v0.3.0...v0.4.0) (2026-10-24)


### Added

* **cards:** grow the set to 103 original cards with four new decks and a starter ([#6](https://github.com/phmatray/sixfold/issues/6)) ([17c4313](https://github.com/phmatray/sixfold/commit/17c43130d667db61c3384727fc920157f5e028e2))

## [0.3.0](https://github.com/phmatray/sixfold/compare/v0.2.0...v0.3.0) (2026-10-24)


### Added

* collect cards, open booster packs and build your own decks ([#4](https://github.com/phmatray/sixfold/issues/4)) ([ac39bf9](https://github.com/phmatray/sixfold/commit/ac39bf99319a8273e516349f9e48ae1621f617bc))

## [0.2.0](https://github.com/phmatray/sixfold/compare/v0.1.0...v0.2.0) (2026-10-24)


### Added

* **world:** explore a 3D overworld with an enterable duel hall, NPCs and running shoes ([#2](https://github.com/phmatray/sixfold/issues/2)) ([b7782af](https://github.com/phmatray/sixfold/commit/b7782afd0edc888b570aa906c03966218fb5c99d))

## [0.1.0](https://github.com/phmatray/sixfold/releases/tag/v0.1.0) (2026-10-24)

### Added

- Rules engine in pure Rust: setup and mulligans, turn structure, energy, retreat, special
  conditions, damage with weakness and resistance, evolution, knock-outs, prizes and sudden death,
  trainers, powers and the effect framework, deck validation. Every rule of `docs/RULES.md` is
  covered by a named unit test (241 tests).
- An original card set and two balanced prebuilt decks.
- An AI opponent (Easy and Normal).
- A Leptos/WebAssembly match UI: board, hand, interactive turn, conditions and prizes, coin flips,
  battle log, card inspector, animations; desktop and phone layouts.
