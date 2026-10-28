# Sixfold's visual grammar

One game, one table: a felt playmat at night, card stock, and brass for whatever is yours to act on.
The duel screen (#48) set this language; the menus speak it in the world's own materials (#100,
"the painted kit", below). The tokens live in `:root`
in `web/style.css`, and `tools/check-tokens.mjs` (in CI) fails when a rule outside a token
definition uses a raw colour or font size that a token covers.

## Three meanings, never mixed

| Meaning | Looks like | Token |
|---|---|---|
| **Act** (the primary action, the one thing to press) | Brass fill, dark ink | `--brass`, `--on-brass` |
| **Selected** (a tab, a chip, a chosen deck, a toggle that is on) | Brass wash inside a 1.5 px brass line, brass-light text | `--sel-bg`, `--sel-line`, `--sel-ink` |
| **Focus** (where the keyboard or pad is) | A painted hand pointing at the control (menus); a cream ring on the duel's board, the cards and the world's scene | `.focus-hand`, `--focus` |

A selected tab never looks like a button to press, and focus never looks like selection, so a pad
or keyboard player always knows both where they are and what is chosen.

## The painted kit (#100)

The menus are made of the world's materials: **wood** for what you act on and for headers, **paper**
for what you read, and **one toon outline** round both, like the world's cel-shaded models. The
kit is the last section of `web/style.css`; its materials are tokens:

| Material | Tokens | Used for |
|---|---|---|
| Wood | `--wood`, `--wood-hi`, `--wood-lo`, `--grain-wood`, `--plank`, `--plank-lit` | the title's sign and menu planks, page headers (`.page-bar`, the world menu's top), the chosen tab |
| Brass plank | `--plank-brass`, `--brass-deep`, `--brass-ink` | the act button (`.btn.primary`), the title's first entry |
| Paper | `--paper`, `--paper-hi`, `--paper-lo`, `--paper-ink-soft`, `--paper-muted` | sheets, dialogs, panels, rows, plain buttons, fields |
| Outline and paint | `--toon`, `--toon-w`, `--toon-soft`, `--paint`, `--painted-text`, `--edge`, `--edge-soft` | the outline (3 px on planks and sheets, soft 2 px on rows), painted titles, a button's thickness |
| Signals | `--sprout` (a toggle that is on), `--danger-paint`, `--danger-on-paper`, `--heal-on-paper` | |

- **Paper turns the ink dark.** A paper surface (`.dialog`, `.drawer`, `.settings-body`, the world's
  sheets, and rows such as `.deck-row`, `.slot`, `.ach`, `.rival`, `.shelf-item`, `.quest`) remaps
  the ink tokens (`--ink`, `--muted`, `--text-*`, `--line`, `--panel*`, `--sel-*`, `--brass-hi`…)
  to their on-paper values, so a component placed on it reads without rules of its own. Add a new
  paper surface to that list rather than recolouring its children.
- **Dense screens stay calm.** Wood, grain and the thick outline are for the title, headers and the
  act button. Rows, lists and editors are plain paper with the soft outline: the content leads.
- **Buttons.** Plain `.btn`: a paper plate with the outline and a 3 px thickness it sinks into when
  pressed. `.primary`: the brass plank. `.ghost`: no plate, an outline in the ink of what it sits
  on. `.danger`: oxblood paint. Disabled: flat, no thickness, faded ink (still readable).
- **Choices on a tray** (tabs, segments, chips, the language and shelf switches): paper tabs; the
  chosen one is a plank with painted text.
- **Fields**: paper wells with the outline; sliders a paper groove filled with brass and a paint
  knob; toggles go `--sprout` green when on.
- **The title**: the real world behind (`web/world/title.js`), the name painted on a sign (no
  ropes), the menu as planks, each with its status on a paper tag that says its whole line (two
  lines when it needs them, never an ellipsis), the place's name on a paper card. On a phone the
  entries become a two-column grid. The tilted wood (planks, sign, card) sits over a live WebGL
  canvas: it keeps a compositing layer of its own (`will-change: transform`), or WebKit redraws
  its grain each time the canvases change and the boards blink.
- **The sheet** `?sheet` shows every control in each state (rest, hover, pressed, focus, disabled).

## What comes first

Every screen leads with what the player most often wants next: it is the first control in the
keys' and the pad's order, and the one brass act. The world is the game, so the title's first
plank is the world (Continue, or Explore the world on a new save), the guided duel after it; the
in-world menu opens on the Journal (what to do next), then the Map; the Campaign page leads with
going out into the world; a won Gauntlet rung with more to climb leads with the next one; a pack
shelf has one brass Open. An audit asks it of each screen: what is it for, what will the player
want next, and is that first and most prominent?

## Keys and pad, everywhere

Every menu is driven by the keyboard and by a pad (`gamepad.js` plays the pad as keys):

- **The hand** (`a11y.js`) points at the focused control when the keys or a pad moved focus, never
  after a pointer click or on touch. It lives in the top layer, so it shows over dialogs, and
  follows its control. Where it points, the ring is dropped: one marker.
- **Arrows / D-pad**: the nearest control that way (`sixfoldMoveFocus`), inside the open modal if
  any. In a set of choices (the decks on offer) the choice follows; the arrows lead out of it. Text
  fields, sliders, selects and tab lists keep their own arrows; the world walks; the duel's board
  has its own keys.
- **Enter / A** confirms. **Escape / B** folds the open panel, else closes the dialog, else goes
  Back. **L / R** switch tabs.
- A modal (`aria-modal`) keeps focus inside and gives it back to what opened it, unless another
  modal took over. The title's Settings and Save slots are modal sheets.

## Tokens

- **Colour.** Felt (`--felt`, `--felt-deep`, `--felt-hi`, `--deep`), panels (`--panel`, `--panel-2`,
  `--panel-hi`, `--sheet-bg`, `--chip`), ink (`--ink`, `--text-soft`, `--text-dim`, `--text-faint`,
  `--muted`), card stock (`--stock`, `--stock-ink`), brass (`--brass`, `--brass-hi`, `--brass-light`,
  `--brass-line`), the opponent's steel (`--steel`, `--steel-line`), and signal colours (`--damage`,
  `--hit`, `--heal`, `--danger-*`).
- **Type.** One ramp, `--fs-2xs` (11) to `--fs-5xl` (32), multiplied by `--ts`: the one text-size
  knob (Settings, 100 / 125 / 150 %, set on `<html>` by `store.rs`; `--text-scale` is its other
  name). The duel board's labels, which sit in fixed spots, use the board ramp `--fsb-*`,
  multiplied by `--tsb` (at most 1.2, and 1 on a phone). Card faces scale with the card instead.
  Each context has its own voice on top of the ramp:
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

Their look is the painted kit's (above). Underneath:

- **Button** `.btn`, at least 44 px tall: `.primary` (act), `.ghost`, `.danger` (leaving,
  deleting, resetting).
- **Sheet/dialog**: paper, rising in over `--dur-panel`.
- **Selector** (`select`): native underneath, so it reads and types as a select; where the browser
  allows it (`appearance: base-select`) its list is paper too.
- **Slider** (`input[type=range]`), **toggle** (`input[type=checkbox]`), **segmented choice**
  (`.seg`, `.segs`, `.lang-switch`, tabs, chips).
- **Scrolling**: no screen scrolls as a page. Each screen fills the viewport; only regions inside it
  (a menu screen's body, a panel, a log) scroll, with a thin wood scrollbar.

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
