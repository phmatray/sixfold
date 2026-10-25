# World assets and maps (contract for the Blender side)

`world.js` (with `map.js` and `build.js`) builds each map from `web/world/maps/<name>.json` (the game starts in
`overworld`, Sixfold Meadow) and loads the models below. Only the current map is built;
once it is up, the maps, models and ground textures next door (its exits and warps) are
fetched one at a time while the browser is idle, so the next area opens quickly. A missing model is fine: the scene draws a
placeholder of the same footprint, so models can land one at a time. Reload the page
to see a new file.

## The grid

- 1 tile = 1 m = 1 world unit. Tile (x, z) covers [x, x+1] × [z, z+1]; its centre
  is (x + 0.5, z + 0.5). A map spans [0, width] × [0, height].
- −Z is "north", away from the camera (the top of the screen); +Z faces the camera.
- The hero stands on tile centres and moves one tile per step: 0.25 s, or 0.125 s
  running (once it owns the running shoes).
- Collision is per tile: a tile is blocked if a prop's footprint covers it (except
  the walkable `rug` and `exit_mat`, and the decks below), an NPC stands on it, or its
  tile character is `#`, `f`, `W`, `V` or `~`. Outside the map is blocked, except where an
  edge exit leads on.
- **Height:** every tile has a floor height: 0, a cliff digit (`1`..`9` m), a deck (`=` 0.1,
  `H` 0.3, or a deck prop). The hero and NPCs stand at it, easing between two heights on a
  step. A step may rise or drop at most 0.6 m, so a cliff edge (1 m) blocks and stairs
  (0.5 m) climb it. Taps on raised ground find the right tile.
- Press G in the game to show the grid with a legend (blocked, look or use, door, person).

## Models

Outdoor models live in `assets/`, interior ones in `assets/interior/`, NPCs in
`assets/npc/`.

| File | What | Footprint (X × Z) | Height guide |
|---|---|---|---|
| `house.glb` | The duel hall, door on the front (+Z) edge | 7 × 5 | ~5 m |
| `cottage.glb` | Small house (Hob's bakery), door on column 2 of the front row, `smoke_origin` on the chimney | 5 × 4 | ~4.5 m |
| `workshop.glb` | Shed (boathouse, letter office), door on column 1 of the front row | 3 × 3 | ~3.8 m |
| `lighthouse.glb` | Striped tower, `light_origin` in the lantern room (the game adds a pulsing glow) | 2 × 2 | ~7 m |
| `fountain.glb` | Hexagonal basin; meshes whose material name contains `water` get rippling water | 3 × 3 | ~1.6 m |
| `pine.glb`, `birch.glb` | Woodland trees, varied like `tree` | 1 × 1 | ~3.8, ~3.6 m |
| `reeds.glb` | Shoreline reeds and cattails | 1 × 1 | ~0.9 m |
| `boardwalk.glb` | Deck tile for `=`, top at 0.10 m, planks along X, chains on all sides | 1 × 1 | 0.1 m |
| `pier.glb` | Deck tile for `H` (over water), top at 0.30 m, posts into the water | 1 × 1 | 0.3 m |
| `bridge.glb` | Footbridge spanning its Z length; floor 0.12 / 0.24 / 0.24 / 0.12 m by row | 2 × 4 | rails ~1.1 m |
| `stairs.glb` | Stone steps climbing toward −Z from 0 to 1 m (floor 0.5 m; `y` stacks them on a cliff) | 1 × 1 | 1 m |
| `cliff.glb` (+ `cliff_b`, mossy) | Stackable rock block for the digit tiles: sides to the tile edges, flat top at exactly 1 m, random 90° turns | 1 × 1 | 1 m |
| `boat.glb` | Rowboat on the water (bobs in the game) | 2 × 1 | ~0.4 m |
| `stall.glb`, `stall_b.glb` | Market stalls, counter on the front; a person can be talked to across one | 2 × 1 | ~2.3 m |
| `lantern.glb` | Street lantern, `light_origin` at the glass | 1 × 1 | ~2.2 m |
| `crates.glb`, `bench.glb` | Crates and a barrel; a park bench facing +Z | 1 × 1, 2 × 1 | — |
| `signpost.glb` | Direction arrows on a post, read like `sign` | 1 × 1 | ~1.6 m |
| `lens.glb`, `letter.glb` | Quest items, shown turning in the item popup | — | any (fitted) |
| `tree.glb` (+ `tree_b`, `tree_c`) | Round-canopy trees, picked at random | 2 × 2 | ~3.5 m |
| `bush.glb` | Leafy bush | 1 × 1 | ~0.7 m |
| `bush_dark.glb` | Darker filler for the band of trees outside the map | free | ~1 m |
| `flowers.glb` | Flower clump | 1 × 1 | ~0.3 m |
| `rock.glb` | Boulder | 1 × 1 | ~0.6 m |
| `sign.glb` | Signboard on one post, face toward +Z | 1 × 1 | ~1.3 m |
| `grass_edge.glb` | A few blades, scattered along path borders | free | ~0.35 m |
| `fence*.glb` | Fence pieces, see Auto-tiling | 1 × 1 | ~0.9 m |
| `hero.glb` | Chibi player character | — | ~1.45 m |
| `shoes.glb` | The running shoes, shown turning in the item popup | — | any (fitted) |
| `interior/wall*.glb` | Wall pieces, see Auto-tiling; `wall_window` is a straight wall with a window | 1 × 1 | ~2.6 m |
| `interior/wall_low*.glb` | Low cutaway walls for the south (camera-facing) row | 1 × 1 | ~0.45 m |
| `interior/counter.glb` | Shop counter, customer side +Z | 3 × 1 | ~1 m |
| `interior/shelf.glb` | Shelf against a wall | 2 × 1 | ~2 m |
| `interior/duel_table.glb` | Card table | 2 × 1 | ~0.8 m |
| `interior/chair.glb`, `plant.glb`, `display_case.glb`, `lamp.glb` | Furniture | 1 × 1 | — |
| `interior/rug.glb` | Rug, ≥ 1 cm thick (walkable) | 3 × 2 | — |
| `interior/exit_mat.glb` | Door mat on the exit tile, ≥ 1 cm thick (walkable) | 1 × 1 | — |
| `npc/<model>.glb` | NPC on the hero rig (`Idle`, `Walk`, `Fidget`); campaign rivals are `rival_<colour>.glb` and `rival_champion.glb` | — | ~1.45 m |
| `interior/oven.glb` | Brick bread oven against the north wall, glowing mouth (`oven_glow`), flue, a ledge with loaves and the tip jar on its right | 2 × 1 | ~2.45 m |
| `interior/printing_press.glb` | Hand press: frame, screw and bar, bed on rails along X, raised tympan on the left, ink table on the right | 2 × 1 | ~1.8 m |
| `interior/type_case.glb` | Compositor's cabinet: drawers (one pulled out), a sloping case of compartments | 1 × 1 | ~1.35 m |
| `interior/bookcase.glb` | Tall bookcase against the north wall, cupboard below | 2 × 1 | ~2.3 m |
| `interior/bed.glb` | Bed along X, head on +X, patchwork quilt | 2 × 1 | ~1.1 m |
| `interior/workbench.glb` | Workbench with a vise and a pegboard of tools against the north wall | 2 × 1 | ~1.8 m |
| `interior/light_box.glb` | The deck doctor's table: a glowing panel (`lightbox_glow`) with cards on it, a magnifier on an arm | 2 × 1 | ~1.25 m |
| `interior/net_locker.glb` | Slatted locker with a net, floats, rope and an oilskin | 1 × 1 | ~1.95 m |
| `hanging_sign.glb` | The swap shop's sign: a board with two chasing arrows on a braced arm (both faces) | 1 × 1 | ~2.5 m |
| `curio_cabinet.glb` | Stall variant for the curio collector: counter, glass vitrine, drawer towers, canopy, a lamp (`lamp_glow`); spoken to across like a stall | 2 × 1 | ~2.3 m |
| `curio.glb` | Fallback curio (a drawstring pouch), item popup | — | any (fitted) |
| `brass_thimble.glb`, `heron_feather.glb`, `message_bottle.glb`, `fossil_fern.glb`, `tin_hex.glb`, `lead_type.glb` | The six 1.2 curios, named by curio id, centred on the origin (≤ 600 tris) | — | any (fitted) |
| `waymark.glb` | Hex stone post holding a crystal (`crystal_glow`, near white: tint it with the region colour) | 1 × 1 | ~1.9 m |
| `seal_stone.glb` | Hex seal stone on a stepped base, one coloured face per colour (`seal_<colour>`: front fire, then counter-clockwise seen from above water, lightning, psychic, fighting, grass), a blank pale top (`seal_top`) | 2 × 2 | ~1.9 m |
| `cave_mouth.glb` | Sandstone opening with a timber frame and a dark recess, back flush with the north edge: put it against a cliff on the row behind (≥ 2 m); the door warp is its tile | 2 × 1 | ~2.1 m |
| `climb_wall.glb` | Two rough steps (tops 0.5 m front, 1.0 m back) with handholds, pegs and vines; climbs toward −Z, floor 0.5 m, stacks with `y` like `stairs` | 1 × 1 | 1 m |
| `inn_sign.glb` | Two posts, a board with a crescent moon and stars (both faces), a lantern (`lamp_glow`) | 1 × 1 | ~2.35 m |
| `npc/npc_elder.glb`, `npc_worker`, `npc_sailor`, `npc_scholar` | 1.2 / 1.3 townsfolk on the hero rig (stooped elder with a cane; worker in cap and apron; sailor in a striped jumper and rain hat; scholar in a long coat with a map roll) | — | ~1.4–1.6 m |
| `portraits/<rival id>.webp` | 512² bust portraits of the campaign rivals for the duel screen (`bramble`, `marlo`, `cinza`, `quill`, `orla`, `lumen`, `warden`), 3/4 view on a gradient in the rival's colour, ≤ 60 KB | — | — |

The house is 7 wide so the door sits on the centre column: door tile = footprint
origin + (3, 4), the house's warp tile.

Rules for every model:

- **Format:** glTF binary (`.glb`), one scene per file, ≤ 2 MB.
- **Axes and units:** glTF Y-up, 1 unit = 1 m. Models load at 1:1, never rescaled.
- **Origin:** at the **centre of the footprint**, on the ground (y = 0).
- **Facing:** the front faces **+Z** (door, sign face, a person's eyes).
- **Materials:** flat colours or a baked painterly base-colour texture (≤ 1024 px). The
  game re-shades with a toon material, keeping base colour, its texture and alpha.
- **Budget:** ≤ 3k triangles per prop, ≤ 6k for a character.
- **Characters:** `Idle` (breathing, blink) and `Walk` (an in-place loop, one leg forward
  per 0.25 s tile), optional `Run` (else Walk plays twice as fast) and `Fidget` (played once
  after ~6 s idle). The first clip of each name wins. The game adds the step bounce.
- **Draw calls:** static props (trees, bushes, rocks, decks, cliffs, stalls, lanterns …) are
  instanced, and the flat-coloured opaque parts of each model are merged into one mesh with
  vertex colours, so a model costs one draw (plus one shadow draw) however many there are.
  Keep textured or see-through parts to a minimum; each one is a draw of its own.
- **Wind:** trees, bushes, flowers, pine, birch, reeds and grass_edge sway in the vertex shader, weighted by a
  `COLOR_0` vertex colour (red: 0 at the base, 1 at the tips; never shown as a tint).
  Without it, the game uses height above the ground.
- **Shadows:** bake a soft contact-shadow disc into prop bases; the game adds one only
  under placeholders and characters.
- **House:** an empty `smoke_origin` at the chimney top. The door opens with an `Open` clip
  (played forward to open, back to close); without one, a node named `door` swings about
  +Y on its own origin (put it on the hinge).
- **Lamp:** an empty `light_origin` where the warm point light sits.
- **Glow:** a material whose name contains `glow` (`oven_glow`, `lamp_glow`, `crystal_glow`, `lightbox_glow`) is
  the part that turns emissive at dusk and night; keep it out of the merged vertex-colour mesh so its name survives.
- **Legal:** original designs only. No logos, characters, buildings or text from any
  existing game. The only emblem is the Sixfold hexagon.

## Ground

One flat ground mesh per map at y = 0 (nothing else may be coplanar with it). Its
shader paints the base layer and up to four overlays through a per-tile RGBA mask with
noisy, painted borders: `p` the map's path layer, `s` (and `=`) sand, `r` stone, and `~`
(and `H`) water. Water is drawn by the same shader, no extra mesh: shallow to deep teal,
drifting ripples and glints, a foam line and wet sand along the shore (still with reduced
motion). Past the map edge the mask carries on from the edge tile, so a path or the sea
runs on into the distance. Textures: `textures/grass.webp`, `dirt.webp`, `wood_floor.webp`,
`sand.webp`, `stone.webp` (seamless, 1024²; convert from PNG with `cwebp -q 82`; only the
.webp files are committed). A procedural fallback is drawn when one is missing.

## Auto-tiling (fences and walls)

Fences (`f`) and walls (`W`, and `V` for a wall with a window) are tiles, not props.
Each picks a piece and a rotation from which of its 4 neighbours join it (a wall joins
`W` and `V`). Walls on the map's last row (south, facing the camera) use the
`wall_low*` pieces. A `V` tile uses `wall_window` where the run is straight and a normal
piece elsewhere. Every piece is 1 × 1, origin at the tile centre, arms reaching the tile
edges. At rot 0 (`fence` shown; `wall` and `wall_low` are the same):

- `fence`: straight, arms to −X and +X
- `fence_corner`: arms to −X and +Z
- `fence_end`: one arm to −X
- `fence_t`: arms to −X, +X and +Z

Mask bits: N (−Z) = 1, E (+X) = 2, S (+Z) = 4, W (−X) = 8. `rot` as for props below.
`tools/check-fence-table.mjs` checks this table against `props.js`.

| Mask | Neighbours | Piece | rot |   | Mask | Neighbours | Piece | rot |
|---|---|---|---|---|---|---|---|---|
| 0 | none | straight | 0 |   | 8 | W | end | 0 |
| 1 | N | end | 270 |   | 9 | N W | corner | 270 |
| 2 | E | end | 180 |   | 10 | E W | straight | 0 |
| 3 | N E | corner | 180 |   | 11 | N E W | t | 180 |
| 4 | S | end | 90 |   | 12 | S W | corner | 0 |
| 5 | N S | straight | 90 |   | 13 | N S W | t | 270 |
| 6 | E S | corner | 90 |   | 14 | E S W | t | 0 |
| 7 | N E S | t | 90 |   | 15 | all | t 0 + end 270 | — |

## Map files (`maps/<name>.json`)

```jsonc
{
  "width": 28, "height": 24,            // in tiles
  "title": "Sixfold Meadow",            // outdoors: shown (and announced) on the way in
  "tiles": ["....p....", ...],          // `height` rows of `width` chars; row 0 is z = 0 (far side)
      // "." base ground, "p" path, "s" sand, "r" stone, "w" wood floor (walkable)
      // "=" boardwalk, "H" pier over water, "1".."9" a raised cliff top that many metres up (walkable)
      // "f" fence, "W" wall, "V" wall with a window, "~" water, "#" blocked
  "props": [{ "asset": "house", "x": 10, "z": 6, "rot": 0 }, { "asset": "pine", "x": 3, "z": 2, "rot": 0, "y": 1 }, ...],
  "exits": [{ "side": "n", "span": [24, 25], "to": "quarry" }],
  "spawn": { "x": 13, "z": 14, "facing": "n" },
  "interactions": [{ "x": 16, "z": 11, "kind": "sign", "text": "..." }, ...],
  "warps": [{ "x": 13, "z": 10, "to": "shop", "label": "Go inside", "spawn": { "x": 5, "z": 7, "facing": "n" } }],
  "npcs": [{ "id": "wren", "name": "Wren", "model": "npc_wanderer", "tint": "#7fb6ff", "scale": 0.94,
             "x": 6, "z": 13, "facing": "e", "idle": "Idle", "route": ["e", "e", "s", ...],
             "dialogue": [{ "if": "!item:running_shoes", "lines": ["..."], "do": ["give:running_shoes"] },
                          { "lines": ["..."] }] }],
  "ground": { "base": "grass", "path": "dirt", "margin": 9, "tufts": 1800 },
  "ring": ["pine", "birch", "bush_dark"], "ringSides": "nesw",
  "camera": { ... },
  "light": { ... }
}
```

- **props:** `x`, `z` = the footprint's min-corner tile. `rot` ∈ 0, 90, 180, 270 degrees
  about +Y (counter-clockwise seen from above: rot 90 turns the +Z front toward +X); 90
  and 270 swap the footprint. The model sits at the footprint's centre, `y` metres up
  (optional: a tree or a rock on a cliff top, stairs from one ledge to the next). Decks
  (`boardwalk`, `pier`, `bridge`, `stairs`) are walkable, even over water, and set the floor
  height of their tiles.
- **exits:** walking off the map's `side` (`n`, `e`, `s`, `w`) on a tile whose coordinate
  along that edge is within `span` (inclusive) fades to map `to`. The hero comes in on the
  matching edge of `to`, at the same offset along the exit there that leads back (same
  `to`, opposite side), facing the way it walked; without one, on `to`'s spawn. Every exit
  needs its way back, and both spans' edge tiles must be walkable
  (`node tools/check-maps.mjs web/world` checks this, in CI). Put a signpost by each.
- **Directions:** `n` = −Z, `s` = +Z, `e` = +X, `w` = −X.
- **interactions:** the tile the hero must **face** to act with E / tap. `kind` `duel` offers a
  match; `sign` shows `text`. A spot with an `id`, a `name` and a `dialogue` (as for NPCs)
  talks instead, so a find can `give:` an item; with `glint` (a condition, e.g.
  `"!item:field_lens"`) a sparkle marks it while the condition holds.
- **warps:** a door warp (no `dir`) sits on a solid tile; facing it shows `label`, and stepping
  into it (or E) walks in, fades, and loads map `to` with the hero on `spawn`. A warp with
  `dir` (the exit mat) fires when the hero steps onto it, or pushes `dir` while standing on it.
  A door warp with `if` (a dialogue condition, e.g. `"seals:6"`) only opens while it holds;
  until then it reads like a sign showing `locked`. A `#` tile in a wall row leaves a gap: a
  dark doorway for such a door (the hall's sealed door to `warden`).
- **npcs:** block their tile (and stand at its floor height). `model` loads `assets/npc/<model>.glb` (`npc_*` and `rival_*`
  names); until it exists the hero model is used, lerped toward `tint` and scaled by `scale`.
  `route` (optional) is a loop of one-tile steps. Speaking (E or tap while facing them, also
  across a counter or a duel table) turns them to the hero and runs `dialogue`: the first
  branch whose `if` holds plays its `lines`, then applies its `do` effects. Conditions and
  effects are the engine's progression language (`engine/src/overworld.rs`):
  - conditions (negate with `!`): `item:x`, `flag:x`, `seals:n` (at least n seals),
    `beat:<rival>`, `open:<rival>` (enough seals to duel them), `won:<rival>` /
    `lost:<rival>` (the last campaign duel's result, until its rival is next talked to);
  - effects: `give:x`, `flag:x`, `duel:<rival>` (after the lines, offer a duel with a deck
    picker), `pack:<booster>` (`ember`, `tide`, `wild`, `grove`, `storm`, `stone`, `dream`).
  A campaign rival is an NPC whose `id` is the rival's id in `engine/src/campaign.rs`; its
  branches go `won:` / `lost:` (post-duel lines), `!open:` (locked), `beat:` (rematch, with
  `duel:`), then the first meeting (with `duel:`). Talking also sets `flag:talked_<id>`.
  Progress, and where the hero stands (`map,x,z,facing`), are saved in localStorage; the
  world resumes there, and after a campaign duel the hero, back on its tile, talks to the
  rival again so the post-duel line plays.
- **ground:** the base layer (`grass`, `dirt`, `sand` …), `path` under `p` tiles, how far
  the ground runs past the map edge, and `tufts` (grass blades scattered on open base
  tiles, 1800 by default; 0 on sand or dirt).
- **ring / ringSides:** the band of trees drawn outside an outdoor map: which models
  (default `tree`, its variants and `bush_dark`) and on which sides (default `nesw`; leave out
  the sea). It leaves a gap at each exit.
- **camera:** `fov` (vertical, degrees; `fovPortrait` when the view is taller than wide),
  `pitch` (degrees below horizontal), `distance` (`distancePortrait`), `lookAtOffset`, and
  `clamp` (indoors: keep the view inside the room). The camera looks at hero + lookAtOffset
  from hero + lookAtOffset + distance × (0, sin pitch, cos pitch), never rotating.
- **light:** `sun` colour/intensity and `fromHero` (outdoors the sun sits at hero + fromHero;
  indoors, with `indoor: true`, at the room centre + fromHero, as window light), `hemisphere`
  sky/ground colours, `background` (also the fog colour), optional `fog` range, and indoors
  `lamp` (a point light on every lamp). Intensities are three.js (physically based) units.
