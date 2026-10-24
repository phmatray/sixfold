# Sixfold

A clean-room creature card duel with the old handheld rules, written in **Rust**,
compiled to **WebAssembly** and played in the browser (Leptos, client-side only).
Collect cards from booster packs, build your own decks and duel an AI opponent.

**Play it:** https://phmatray.github.io/sixfold/

## Status

Playable from start to finish:

- **Rules engine** (`engine/`): the whole duel, from setup and mulligans through
  attacks, conditions, Powers, trainers, knock-outs, prizes and sudden death.
  Pure Rust, no browser needed; 273 unit tests.
- **Card set**: 34 original cards and two 60-card decks, Ember Rush
  (Fire / Fighting aggro) and Tide Mind (Water / Psychic control). In
  AI-against-AI play, Ember Rush wins about 45% of games and Tide Mind about 55%. A test
  checks that each deck wins between 35% and 65%.
- **AI opponent** (`engine/src/ai.rs`): Normal or Easy (Easy skips a trainer or
  Power play one time in four). It sees only what its player may see.
- **Web app** (`web/`): deck select, the table, animated coins, damage and
  knock-outs, a battle log, a card inspector, and the game-over screen with a
  rematch. Works on desktop and on phones, in portrait and landscape.
- **Collection and deck building**: a binder of every card, booster packs
  (Ember, Tide, Wild) revealed card by card, saved decks you create, copy,
  rename and edit, and a Wild Pack for every duel you win. New players start
  with both prebuilt decks and three packs. Progress is kept in the browser
  (localStorage) and can be reset from Settings.

Not done yet: a second human player, sound.

## How to play

Prerequisites: [rustup](https://rustup.rs) with the `wasm32-unknown-unknown`
target, and [Trunk](https://trunkrs.dev).

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk

cd web && trunk serve            # http://localhost:8080 (first build takes ~2 min)
cd web && trunk build --release  # static site in web/dist

cargo test -p sixfold-engine     # rules, card set and AI tests
```

In the app, pick your deck and the AI's, then press **Start the duel**.

1. **Setup.** Choose a Basic creature as your Active, put more Basics on your
   Bench, then press **Done**.
2. **Your turn.** Click a card in your hand to see what it can do: place it,
   attach energy (once per turn), evolve, or play a trainer.
3. **Act.** The action bar has your attacks (with the damage they would do
   now), Retreat, Powers and End turn. An attack ends your turn.
4. **Win.** Take all your prizes, leave the opponent with no creature in play,
   or outlast a deck that runs out of cards.

The seed on the title screen fixes the shuffles and coin flips, so a seed
replays the same duel if both players make the same moves.

## Rules

The rules follow the original Game Boy game, not the modern card game. The
first player draws and may attack on turn 0, there is no Burned condition,
confusion hurts the attacker for 20, weakness doubles damage, resistance takes
off 30, a mulligan gives the opponent no extra cards, the Bench holds 5, and a
tie goes to sudden death with one prize each.

[`docs/RULES.md`](docs/RULES.md) lists every rule with an ID (for example
`R-STATUS-07`), its exact values, the evidence for it, and the engine test that
covers it. Each test names its rule ID. Where the ledger was unsure, the code
has a `// RULES:` comment naming the rule.
[`docs/ENGINE_API.md`](docs/ENGINE_API.md) describes how the UI and the AI use
the engine.

## Scope and legal

- Only the game's **rules and mechanics** are implemented. Those can't be
  copyrighted.
- Nothing from Nintendo, Game Freak or The Pokémon Company is included or
  shipped: no card art, names, attack names, flavor text, card stats, sprites
  or data tables. Every card name, number and rules text here is original, and
  the art is drawn by code.
- The code and the app say "creature", not the trademarked name.
- A reference ROM may sit in the repository root for local study only. It is
  git-ignored (`*.gbc`) and must never be committed or shipped.
