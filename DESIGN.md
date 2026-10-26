# Sixfold's visual grammar

One game, one table: a felt playmat at night, card stock, and brass for whatever is yours to act on.
The duel screen (#48) set this language; every other screen speaks it. The tokens live in `:root`
in `web/style.css`, and `tools/check-tokens.mjs` (in CI) fails when a rule outside a token
definition uses a raw colour or font size that a token covers.

## Three meanings, never mixed

| Meaning | Looks like | Token |
|---|---|---|
| **Act** (the primary action, the one thing to press) | Brass fill, dark ink | `--brass`, `--on-brass` |
| **Selected** (a tab, a chip, a chosen deck, a toggle that is on) | Brass wash inside a 1.5 px brass line, brass-light text | `--sel-bg`, `--sel-line`, `--sel-ink` |
| **Focus** (where the keyboard or pad is) | Cream ring held 3 px off the control; it closes in from 7 px | `--focus` |

A selected tab never looks like a button to press, and focus never looks like selection, so a pad
or keyboard player always knows both where they are and what is chosen.

## Tokens

- **Colour.** Felt (`--felt`, `--felt-deep`, `--felt-hi`, `--deep`), panels (`--panel`, `--panel-2`,
  `--panel-hi`, `--sheet-bg`, `--chip`), ink (`--ink`, `--text-soft`, `--text-dim`, `--text-faint`,
  `--muted`), card stock (`--stock`, `--stock-ink`), brass (`--brass`, `--brass-hi`, `--brass-light`,
  `--brass-line`), the opponent's steel (`--steel`, `--steel-line`), and signal colours (`--damage`,
  `--hit`, `--heal`, `--danger-*`).
- **Type.** One ramp, `--fs-2xs` (11) to `--fs-5xl` (32), multiplied by `--text-scale` (1 for now:
  the one knob a reading-size setting turns, #70). Each context has its own voice on top of it:
  - *Menus*: `--type-menu-title` 22, `-section` 20, `-label` 18, `-body` 15, `-meta` 13.
  - *Duel*: `--type-duel-fine` 12, `-body` 14, `-label` 16, `-title` 20, `-big` 28.
  - *World*: `--type-world-hud` 14, `-name` 14, `-line` 18 (a spoken line reads larger), `-sheet` 22.
  - Display face (Bricolage Grotesque) for names, titles and numbers; Atkinson Hyperlegible for
    everything read.
- **Space.** A 4 px step: `--space-1` 4 … `--space-7` 48.
- **Radii.** `--radius-sm` 6 (chips, keys), `--radius-md` 12 (controls, tiles, plates),
  `--radius-lg` 18 (sheets, dialogs), `--radius-pill`.
- **Elevation.** `--elev-1` resting, `--elev-2` lifted (popovers, toasts), `--elev-3` floating (dialogs).
- **Motion.** `--dur-quick` 140 ms (state), `--dur-panel` 220 ms (a panel), `--dur-screen` 380 ms (a
  screen), one ease-out (`--ease-out`).

## Components

- **Button** `.btn`: panel fill, hairline, 12 px radius, 44 px tall. `.primary` brass (act),
  `.ghost` no fill, `.danger` oxblood (leaving, deleting, resetting).
- **Panel**: `--panel` on the felt with a hairline; a **sheet/dialog** is `--sheet-bg` with a brass
  hairline, `--radius-lg` and `--elev-3`, rising in over `--dur-panel`.
- **Selector** (`select`): a button-shaped field with a brass chevron; where the browser allows it
  (`appearance: base-select`) its list is a panel too. Native underneath, so it reads and types as
  a select.
- **Slider** (`input[type=range]`): a felt groove filled with brass up to the value, a brass hex
  thumb.
- **Toggle** (`input[type=checkbox]`): a switch, brass when on.
- **Segmented choice** (`.seg`, `.lang-switch`, tabs, chips): options on one tray, the chosen one
  in the selected style.
- **Scrolling**: no screen scrolls as a page. Each screen fills the viewport; only regions inside it
  (a menu screen's body, a panel, a log) scroll, with a thin brass scrollbar.

## Screen transitions

Every screen change goes through `crate::go(Screen)` (a view transition where the browser has them):

- **Between menus**: the old screen sinks and fades, the new one rises 12 px into place (≤ 380 ms).
- **Into a duel**: the table opens out of a hexagon from the centre, the old screen dims behind it.
- **Back from a duel**: the duel folds into a hexagon and the world (or menu) comes up behind it.
- **Reduced motion**: an instant cut. Focus lands on the new screen's heading or its first
  meaningful control once the swap is done.

## The world's HUD

Only what is glanceable stays over the scene: the time of day, the Facets held and the Menu
button. The Journal, the map, the seals and sound are in the menu (and on their keys); the controls
hint shows until the first steps, then fades, and lives in the menu's Options.
