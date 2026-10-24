# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A from-scratch, clean-room trading card game written in **Rust**, compiled to **WebAssembly** and run in the browser via **Leptos** (CSR), with a modern UI/UX.

## Legal scope (hard constraint)

Only the game's **rules and mechanics** are implemented here — those are not copyrightable. Do **not** add Nintendo / Game Freak / The Pokémon Company assets (card art, flavor text, official data tables) to the codebase or any shipped build. A reference `.gbc` ROM may sit in the repo root for local study only; it is git-ignored (`*.gbc`) and must never be committed. All card names/content in the app are original or placeholder.

## Commands

Toolchain is **rustup** (not Homebrew rust). rustup is keg-only at `/opt/homebrew/opt/rustup/bin`; if a shell can't find `cargo`/`rustc`, that path (plus `$HOME/.cargo/bin`) is missing from PATH — it's appended in `~/.zshrc`.

```bash
# Run the web app (first build is slow ~2min; serves on http://localhost:8080)
cd web && trunk serve

# Production build -> web/dist
cd web && trunk build --release

# Test the rules engine (host-native, fast)
cargo test -p pkmntcg-engine

# Run a single engine test by name
cargo test -p pkmntcg-engine draw_moves_card_from_deck_to_hand
```

## Architecture

Cargo workspace, two crates with a one-way dependency `web -> engine`:

- **`engine/`** — the rules core, **pure Rust with no WASM or UI dependencies**. This is the invariant that makes the game logic unit-testable on the host without a browser. Keep all game rules, state transitions, and card models here; never pull `leptos`, `wasm-bindgen`, or DOM concerns into this crate.
- **`web/`** — the Leptos CSR app. Depends on `engine` by path and renders its types. Built by Trunk, whose entry point is `web/index.html` (not `main.rs` directly) — the `data-trunk` link tags there pull in `style.css` and the wasm bundle.

When adding game logic, the model lives in `engine` with a unit test; the UI in `web` consumes it. A change that needs a browser to verify belongs in `web`; a change you can assert with plain Rust belongs in `engine`.
