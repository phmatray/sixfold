# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/). Entries from 0.1.0 on are written by
release-please from Conventional Commits.

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
