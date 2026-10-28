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
cargo test -p sixfold-engine

# Run a single engine test by name
cargo test -p sixfold-engine draw_moves_card_from_deck_to_hand
```

## Architecture

Cargo workspace, three crates with one-way dependencies `web -> engine` and `desktop -> web/dist`:

- **`engine/`** — the rules core, **pure Rust with no WASM or UI dependencies**. This is the invariant that makes the game logic unit-testable on the host without a browser. Keep all game rules, state transitions, and card models here; never pull `leptos`, `wasm-bindgen`, or DOM concerns into this crate.
- **`web/`** — the Leptos CSR app. Depends on `engine` by path and renders its types. Built by Trunk, whose entry point is `web/index.html` (not `main.rs` directly) — the `data-trunk` link tags there pull in `style.css` and the wasm bundle.
- **`desktop/`** — the desktop app (Tauri v2): `web/dist` in a native window, no game code of its own. It needs a built `web/dist` (and WebKitGTK on Linux) to compile, so it is left out of `default-members` and of CI's workspace clippy; `.github/workflows/desktop.yml` builds and lints it. Run `cargo tauri dev` / `cargo tauri build` from the repo root.

When adding game logic, the model lives in `engine` with a unit test; the UI in `web` consumes it. A change that needs a browser to verify belongs in `web`; a change you can assert with plain Rust belongs in `engine`.

## Releases

Public repo `phmatray/sixfold`, deployed to https://phmatray.github.io/sixfold/ after CI passes on `main`.
Releases are cut by **release-please** from **Conventional Commits** (`feat:`, `fix:`, `docs:`, `ci:`, `chore:` …),
so commit messages and squash-merged PR titles must follow that form. A new minor version is cut after every **3 merged
`feat:`/`fix:` PRs** (`docs:`, `chore:`, `ci:` don't count); open every PR as a **draft** as soon as the branch has
work. The plan per version is in `docs/ROADMAP.md`. Renovate keeps dependencies current (`renovate.json`).
`scripts/check-no-game-data.sh` runs in CI and fails if a ROM, save, state, archive, `.blend` or any file over 2 MB is tracked.
