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
- **Raised ground** (`terrain.js`): digit tiles are part of the ground mesh, one height field per
  map: plateau tops carry the ground texture (and `paint`), walls stand in the lower tile's outer
  quarter, convex corners are rounded and concave ones filled. Next to a built prop (stairs, a climb
  wall, a cave mouth, a house; not trees, rocks or flowers) the wall stays straight on the tile edge.
- Press G in the game to show the grid with a legend (blocked, look or use, door, person).

## Models

Outdoor models live in `assets/`, interior ones in `assets/interior/`, NPCs in
`assets/npc/`.

| File | What | Footprint (X × Z) | Height guide |
|---|---|---|---|
| `house.glb` | The duel hall, door on the front (+Z) edge | 7 × 5 | ~5 m |
| `cottage.glb` | Small house (Hob's bakery), door on column 2 of the front row, shingle courses on the roof, `smoke_origin` on the chimney | 5 × 4 | ~4.5 m |
| `workshop.glb` | Shed (boathouse, letter office), double door (`door`, `door_r`) on column 1 of the front row | 3 × 3 | ~3.8 m |
| `lighthouse.glb` | Striped tower, `light_origin` in the lantern room (the game adds a pulsing glow) | 2 × 2 | ~7 m |
| `fountain.glb` | Hexagonal basin; meshes whose material name contains `water` get rippling water | 3 × 3 | ~1.6 m |
| `pine.glb`, `birch.glb` | Woodland trees, varied like `tree` | 1 × 1 | ~3.8, ~3.6 m |
| `reeds.glb` | Shoreline reeds and cattails | 1 × 1 | ~0.9 m |
| `boardwalk.glb` | Deck tile for `=`, top at 0.10 m, planks along X, chains on all sides | 1 × 1 | 0.1 m |
| `pier.glb` | Deck tile for `H` (over water), top at 0.30 m, posts into the water | 1 × 1 | 0.3 m |
| `bridge.glb` | Footbridge spanning its Z length; floor 0.12 / 0.24 / 0.24 / 0.12 m by row | 2 × 4 | rails ~1.1 m |
| `stairs.glb` | Stone steps climbing toward −Z from 0 to 1 m (floor 0.5 m; `y` stacks them on a cliff) | 1 × 1 | 1 m |
| `cliff.glb` (+ `cliff_b`, mossy) | A rock block (no longer stacked on digit tiles: `terrain.js` builds raised ground; its colours are the sandstone wall's) | 1 × 1 | 1 m |
| `boat.glb` | Rowboat on the water (bobs in the game) | 2 × 1 | ~0.4 m |
| `fishing_boat.glb` | The boat a map's `life` docks at a pier (`life.boats`): bow +X like `boat`, waterline at 0, a furled sail, `glow` lamp on the mast; ≤ 2k tris (built by `build_critters.py`) | 2 × 1 | ~1.8 m |
| `critter_<kind>.glb` | Ambient life (`life.critters`, `life.perch`): `rabbit`, `squirrel`, `hedgehog`, `duck`, `crab`, `lizard`, `goat`, `glowbeetle` and the perch `bird`, at the size the game draws them, feet on the origin, front +Z, flat colours only, no contact shadow (they hop and fly); the game merges each into one vertex-coloured mesh, all of a kind in one instanced draw (glowbeetles glow on their own); ≤ 1.2k tris (built by `build_critters.py`) | — | 0.07–0.85 m |
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
| `hero.glb` | The player character (built by `build_characters.py`) | — | ~1.47 m |
| `shoes.glb` | The running shoes, shown turning in the item popup | — | any (fitted) |
| `interior/wall*.glb` | Wall pieces, see Auto-tiling; `wall_window` is a straight wall with a window | 1 × 1 | ~2.6 m |
| `interior/wall_low*.glb` | Low cutaway walls for the south (camera-facing) row | 1 × 1 | ~0.45 m |
| `interior/counter.glb` | Shop counter, customer side +Z | 3 × 1 | ~1 m |
| `interior/shelf.glb` | Shelf against a wall | 2 × 1 | ~2 m |
| `interior/duel_table.glb` | Card table | 2 × 1 | ~0.8 m |
| `interior/chair.glb`, `plant.glb`, `display_case.glb`, `lamp.glb` | Furniture | 1 × 1 | — |
| `interior/rug.glb` | Rug, ≥ 1 cm thick (walkable) | 3 × 2 | — |
| `interior/exit_mat.glb` | Door mat on the exit tile, ≥ 1 cm thick (walkable) | 1 × 1 | — |
| `npc/<model>.glb` | A person (`Idle`, `Walk`, `Fidget`, `Sit`), one draw on a palette texture: the townsfolk archetypes `npc_kid`, `npc_gardener`, `npc_elder`, `npc_sailor`, `npc_shopkeeper`, `npc_scholar`, `npc_worker`, `npc_wanderer`, `npc_rival`, the campaign rivals `rival_<colour>.glb` and `rival_champion.glb`, and Rook, the story rival, `rival_colorless.glb` (1.7) | — | ~1.1–1.8 m |
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
| `seal_stone_carved.glb` | The duel hall's seal stone after the story's epilogue (1.7): the same stone, its top cut with a hexagon outline, still blank | 2 × 2 | ~1.9 m |
| `cave_mouth.glb` | A rough arch of boulders round a black throat in a sandstone face, timber bracing, a lantern on the lintel (`lamp_glow`), roots and vines over the top, crystals glinting inside (`crystal_glow`); back flush with the north edge: put it against a cliff on the row behind (≥ 2 m); the door warp is its tile | 2 × 1 | ~2.1 m |
| `climb_wall.glb` | Two rough steps (tops 0.5 m front, 1.0 m back) with painted holds, chalk, a knotted rope from an iron stake and vines; climbs toward −Z, floor 0.5 m, stacks with `y` like `stairs` | 1 × 1 | 1 m |
| `inn_sign.glb` | Two posts, a board with a crescent moon and stars (both faces), a lantern (`lamp_glow`) | 1 × 1 | ~2.35 m |
| `stump.glb` | Hollow stump: flared roots, a cut rim round a dark hollow, moss, two mushrooms (a find's host) | 1 × 1 | ~0.55 m |
| `log.glb` | Fallen mossy log along X, a hollow end on +X, bracket fungus and mushrooms (a find's host) | 2 × 1 | ~0.55 m |
| `cliff_falls.glb` | A waterfall from a 2 m ledge into a rock pool (#233): the sheet is `sx_fall_water` (streaks run down it), the pool `sx_water` (it ripples); its back is flush with the north edge, so it stands against a 2 m cliff on the row behind | 2 × 1 | ~2 m |
| `lantern_bunting.glb` | Paper lanterns (`lantern_glow`, lit at night) and pennants on a sagging line between two posts at the ends; walked under (its lowest point is above 2 m), sways | 3 × 1 | ~2.7 m |
| `washing_line.glb` | Two posts and a line of pegged cloths that move in the wind | 3 × 1 | ~1.9 m |
| `water_lilies.glb` | Lily pads and flowers, on a water tile | 1 × 1 | ~0.1 m |
| `leaf_litter.glb` | Fallen leaves, twigs and pebbles; walked over, never above 6 cm | 1 × 1 | ~0.05 m |
| `windmill.glb` | A tower mill: the sails are a node `sails` turned by a `Loop` clip (one turn in 14 s) | 3 × 3 | ~7.5 m |
| `vegetable_patch.glb` | Two raised beds, a bean wigwam and a scarecrow | 3 × 2 | ~1.7 m |
| `flower_planter.glb` | A wooden planter of flowers for a doorstep | 1 × 1 | ~0.75 m |
| `blossom_tree.glb` | A tree in pale pink blossom, fallen petals under it; varied like `tree` | 2 × 2 | ~3.6 m |
| `well.glb` | A stone well with a little roof, winch and bucket (a dark disc inside, no animated water, so it stays instanced) | 2 × 2 | ~2.3 m |
| `portraits/<rival id>.webp` | 512² bust portraits of the campaign rivals for the duel screen (`bramble`, `marlo`, `cinza`, `quill`, `orla`, `lumen`, `warden`), 3/4 view to each rival's own side and tilt, a rim light and a background motif in the rival's colour, ≤ 60 KB | — | — |

The house is 7 wide so the door sits on the centre column: door tile = footprint
origin + (3, 4), the house's warp tile.
A model with a door warp on its front row gets a `STOOP` entry in `props.js`: how far past the front edge the
hero walks (to the door's face, or through it when it opens) and its step heights at 0, 0.25 and 0.5 m in.

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
  after ~6 s idle). People also have `Sit` (a seated loop: thighs level, hands on the knees; the seat sits 0.87 x
  the hips' height under them); the hero also `SitDown` and `StandUp` (0.6 s each, played once), and for a book off a shelf `Reach` (0.5 s: the right
  hand up beside the face, about 1.3 m, grips and pulls down to the chest), `HoldBook` (4 s loop: both hands at the
  chest, head bowed, reading) and `PutBack` (0.5 s: up again, let go, back to standing); a held book rides on `Fore.R`. The first clip
  of each name wins. The game adds the step bounce.
- **Draw calls:** static props (trees, bushes, rocks, decks, cliffs, stalls, lanterns …) are
  instanced, and the flat-coloured opaque parts of each model are merged into one mesh with
  vertex colours, so a model costs one draw (plus one shadow draw) however many there are.
  Keep textured or see-through parts to a minimum; each one is a draw of its own.
  People are skinned, so they are not merged: each is one mesh on one palette texture (one draw, plus
  one shadow draw), with its swatch table in the material's extras (`sx_palette`) for recolours.
- **Wind:** trees, bushes, flowers, pine, birch, reeds and grass_edge sway in the vertex shader, weighted by a
  `COLOR_0` vertex colour (red: 0 at the base, 1 at the tips; never shown as a tint).
  Without it, the game uses height above the ground.
- **Shadows:** bake a soft contact-shadow disc into prop bases; the game adds one only
  under placeholders and characters.
- **Doors:** every model with a door warp has its leaf as a node named `door`, origin on the hinge (the leaf's -X
  edge, at the sill); a double door's right leaf is `door_r`, hinged on its +X edge. The game swings it 80° inward
  about +Y (`door_r` the other way) while the hero faces or takes the door, and shut after. Behind the leaf, a thin
  `doorway` panel (near black) is the dark doorway it reveals. The leaf is merged on its own (one more draw), the rest
  of the model as usual. The hall's old `Open` clip is ignored. Chimneys: an empty `smoke_origin` at the top.
- **Lamp:** an empty `light_origin` where the warm point light sits.
- **Loop:** a clip named `Loop` (the windmill's sails) plays all the time, from a different point for each prop so
  two never turn in step; it stops with reduced motion. Make it seamless (linear keys, the last pose the first).
- **Water:** a material whose name contains `water` gets drifting ripples (the fountain's pool); `fall_water`
  (a waterfall's sheet) gets streaks running down it instead. Both freeze with reduced motion. A prop with
  either is never instanced, so keep them to the props that need them.
- **Glow:** a material whose name contains `glow` (`oven_glow`, `lamp_glow`, `crystal_glow`, `lightbox_glow`) is
  the part that turns emissive at dusk and night; keep it out of the merged vertex-colour mesh so its name survives.
- **Legal:** original designs only. No logos, characters, buildings or text from any
  existing game. The only emblem is the Sixfold hexagon.

## The map editor (debug builds)

`cd web && trunk serve`, then open the world with `?edit` in the URL (`http://localhost:8080/?edit`). A
panel top left picks a tool: **Walk** (play), **Paint** (the chosen tile character), **Prop** (place the
chosen model at `y`; R turns the next one, or the last one placed), **Scatter** (click two corners: the
prop fills that rectangle at the density, on free tiles), **Person** (an id starting `t_` makes a route
duelist), **Find** and **Erase**. Every change rebuilds the scene at once; route duelists' sight lines are
drawn on the floor. **Ctrl+S** downloads the edited map JSON (keys in their file order); copy it over
`web/world/maps/<map>.json`. Text and dialogue (the "…" placeholders) are still written by hand, with their
`_fr` twins. Walking through a door or an edge edits the next map.

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
  "paint": ["....ppp..", ...],          // optional, same size as `tiles`: "p" / "s" / "r" paint that layer on
      // any tile, a raised one too (a path across a plateau); "." leaves the tile as it is
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
  Seats (`SEAT` in props.js: `bench`, `chair`) can be sat on (Sit, E): each footprint tile is a place,
  facing the prop's front, reached from the tile in front of it or beside it, never from behind.
  `seat: false` makes one scenery; check-maps fails a seat with no such tile the hero can reach.
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
  names); until it exists the hero model is used, lerped toward `tint` and scaled by `scale`. On the NPC's
  own model, `outfit` (a colour) moves the clothes to that hue, keeping each garment's lightness (accent
  garments take the opposite hue), and `hair` recolours the hair: two people on one model then look like two
  people. `wear` (a list of `cap`, `glasses`, `headband`, `scarf`, `bag`, whichever the model carries) shows
  accessories; a route duelist without `wear` puts on one of them, picked by its id. `look` (directions) turns them
  between those ways every 2.5 s. `if` (a condition): they are only there while it holds. `sit: true` on a
  bench or chair tile: they sit there (their `Sit` clip), facing the way the seat faces, and are spoken to from in front or beside.
  `route` (optional) is a loop of one-tile steps. Speaking (E or tap while facing them, also
  across a counter or a duel table) turns them to the hero and runs `dialogue`: the first
  branch whose `if` holds plays its `lines`, then applies its `do` effects. Conditions and
  effects are the engine's progression language (`engine/src/overworld.rs`):
  - conditions (negate with `!`, join with `&`): `item:x`, `flag:x`, `seals:n` (at least n seals),
    `beat:<id>`, `open:<rival>` (enough seals to duel them), `won:<id>` / `lost:<id>` (the last
    duel's result, until that rival or duelist is next talked to), `coins:n` (at least n Facets),
    `count:<prefix>:n` (at least n flags and items start with the prefix);
  - effects: `give:x`, `take:x`, `flag:x`, `duel:<id>` (after the lines, offer a duel with a deck
    picker), `pack:<booster>` (`ember`, `tide`, `wild`, `grove`, `storm`, `stone`, `dream`),
    `card:<slug>` (one card, shown as found), `coins:n`, `pay:n` (short: the rest is skipped and
    "Not enough Facets" shows), `show:<panel>`, `emote:<actor>:<bang|what|dots|heart|note>` (a
    bubble over that NPC's head, or `hero`'s, once the dialog closes).
  A campaign rival is an NPC whose `id` is the rival's id in `engine/src/campaign.rs`; its
  branches go `won:` / `lost:` (post-duel lines), `!open:` (locked), `beat:` (rematch, with
  `duel:`), then the first meeting (with `duel:`). Talking also sets `flag:talked_<id>`.
  Progress, and where the hero stands (`map,x,z,facing`), are saved in localStorage; the
  world resumes there, and after a world duel the hero, back on its tile, meets the rival or
  duelist again so the post-duel line plays.
- **route duelists:** an NPC whose `id` is a duelist in `engine/src/duelists.rs` (`t_<name>`) with a
  `duelist` block (WORLD_DESIGN §4.1). `web/world/duelist.js` runs it; `map.js` expands it to dialogue.

  ```jsonc
  { "id": "t_hazel", "name": "Hazel", "model": "npc_gardener", "outfit": "#e8c33a", "x": 22, "z": 8, "facing": "w",
    "duelist": {
      "sight": 4,                 // tiles watched, 1..5 (default 3); 0: talk to duel, never watches
      "look": ["w", "s"],         // optional: turns between these every 2.5 s, watching each way
      "challenge": ["…"], "challenge_fr": ["…"],   // when it walks up: ends with the duel offer
      "win": ["…"], "win_fr": ["…"],               // the hero won (won:t_hazel)
      "lose": ["…"], "lose_fr": ["…"],             // the hero lost (lost:t_hazel)
      "after": ["…"], "after_fr": ["…"]            // beaten, any later talk (beat:t_hazel)
    },
    "dialogue": [ … ] }           // optional, played between won/lost and after (night lines, quests)
  ```

  While unbeaten it watches a straight line from its tile along its facing, up to `sight` tiles, for as
  long as each tile is one the hero could step on (no prop, no person, at most a 0.6 m climb: a duelist on
  a ledge can't see the floor). The hero standing on that line, after a step or as the map comes up: input
  locks, "!" pops over the duelist, it walks over until it stands by the hero, they face each other and its
  challenge plays, then the duel sheet (deck picker, Duel / Not now). Not now, or after the post-duel line,
  it walks back to its post and stops watching until the hero leaves the map. Beaten, it only chats.
  `tools/check-maps.mjs` refuses a sight line that covers an exit's edge tile, the map's spawn or a tile a
  warp lands on. `"sight": 0` makes a talk-to-duel duelist that never watches (towns and interiors, as
  WORLD_DESIGN §4.1 asks).
- **finds** (`kind: "find"` interactions): something to look in, once.

  ```jsonc
  { "kind": "find", "id": "m_stump", "x": 2, "z": 2, "give": "coins:40", "hidden": false,
    "text": "Tucked in the hollow stump: …", "text_fr": "Cachée dans la souche creuse : …",
    "if": "flag:seeds_asked" }   // optional: only there while it holds
  ```

  `give` is one effect (`coins:n`, `card:<slug>`, `give:<item>`, `pack:<booster>`). The first look plays
  `text` and applies it (flag `found_<id>`, saved); later looks say "Nothing else here". Unless `hidden`, a
  glint marks it until found. Put it on a solid tile, a prop worth looking in (`stump`, `log`, `crates`, a
  bookcase…): check-maps refuses a find on an open tile. Ids are unique across all maps
  (`engine/tests/world_data.rs`).
- **prop `glow`**: a colour for the prop's `*glow*` materials (a waymark's crystal in its region's colour).
- **prop `recolor`**: `{ "<material name>": "#rrggbb" | null }`, a cheap variant of a shared model: each named
  material gets that colour, and `null` hides its parts. The Deck Clinic is the bakery's `cottage` with a slate
  roof, a teal awning and no bread sign. A recoloured prop is never instanced.
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

## World rules: `kind`, `wild`, `why` (docs/WORLD_RULES.md)

- **map `kind`** (outdoor maps): `village`, `town`, `hub`, `route`, `site` or `cave`; sets the minimum size in screens.
- **map `wild`**: `[[x0, z0, x1, z1], …]`, wilderness rects on a town or hub map (lanterns there need a `why`).
- **prop `why`** (a `lantern`): why a lamp stands out of town (`"the boathouse door"`); routes, sites and caves need one per lamp.
- Placement (one free tile before a post, nothing tall in front of what you use, exits 3 wide with a signpost, a closed
  border…) is in WORLD_RULES; `node tools/check-maps.mjs web/world --strict` checks it.

## 1.3 additions: time of day, gates, waymarks, regions (WORLD_DESIGN §7)

- **Outdoor map keys** (check-maps requires the first three outdoors): `title_fr`, `region` (an id in
  `atlas.json`: `hub`, `ember`, `storm`, `tide`, `dream`, `grove`, `stone`), `music` (a track:
  `overworld`, `ember`, `storm`, `tide`, `dream`, `grove`, `stone`, `cave`; after dark the world plays
  `<track>_night`), and optionally:
  - `cliff: ["cliff_basalt", "cliff_basalt_b"]`: the rock of the raised ground's walls (strata, foot and
    worn lip, `ROCK` in `terrain.js`; default `cliff`, sandstone);
  - `fence: "wall_dry" | "hedge"`: the auto-tile set for `f` tiles (default `fence`; same mask table);
  - `dark: true`: a cave. No sun at any hour, a dim sky, and the hero's lantern lights the way; music `cave`;
  - `light_dawn` / `light_dusk` / `light_night`: any part of a `light` block, overriding that phase's
    derived look (`web/world/daylight.js`).
- **The clock** (`engine/src/clock.rs`): a game minute a real second, saved as `t:<minute>`. Dawn 05:00,
  day 08:00, dusk 18:00, night 21:00. Conditions `time:dawn|day|dusk|night` ("dark" is
  `!time:day & !time:dawn`); the effect `clock:<phase>` jumps to the start of a phase (the inn's panel
  uses it). Everything with an `if` (people, spots, finds, props, gates) is re-checked at each phase
  change, so **schedules are data**: duplicate a townsperson with `"if": "!time:night"` and
  `"if": "time:night"` on two tiles. A night duelist (`night: true` in `duelists.rs`) must carry
  `time:night` in its `if` (the data test checks). A night-only find is a find with `"if": "time:night"`.
- **Light at night:** materials named `*glow*` shine brighter after dusk, and `*glass*` (windows) turns
  warm and lit outdoors, dark from inside. Every `light_origin` empty gets a halo and, below 3.2 m, a pool
  of light on the ground; the nearest three get a real light, plus the hero's lantern (never more than six
  live lights, WORLD_DESIGN P1). New lamps need nothing but a `light_origin`.
- **prop `if`** (a condition): the prop is only there, and only blocks its footprint, while it holds (a
  "closed" sign at night, the hub's seal dressing). **prop `open`** (a condition): a prop whose model has an
  `Open` clip plays it while the condition holds, backwards when it stops (`pylon` with
  `"open": "flag:pylon_1"`, `moonflower` with `"open": "!time:day & !time:dawn"`). Afloat props (`boat`,
  `ferry`, `buoy`, `floating_lantern`, `lantern_raft`) bob. A `ladder` joins its front and back tiles
  whatever the height (its rows run along its own Z).
- **gates** (WORLD_DESIGN §7.1): tiles walkable only while a condition holds, whatever the tile.

  ```jsonc
  "gates": [{
    "id": "hush_ford",
    "tiles": [[0, 24], [1, 24], [2, 24, 0]],  // [x, z] or [x, z, y]: y stacks the props (default: the tile's own height)
    "if": "item:wading_boots",                 // any condition: item, time, flag, seals
    "floor": 0.1,                              // optional floor while open (>= 0; default: the open prop's deck top, else the ground)
    "prop": "ford_stones",                     // optional: shown always, one per tile
    "prop_closed": "rubble", "prop_open": "lantern_raft", "rot": 0,  // optional: shown per state
    "text": "The ford runs fast and knee-deep. Boots would help.", "text_fr": "…"   // faced while shut
  }]
  ```

  Shut, the tiles block and facing one shows `text` (titled "No way through"). A gate that changes while
  the hero is on the map plays it: the going props sink, the coming ones rise, with dust and a sound. **A
  gate never closes under the hero**: it stays open while the hero is on any of its tiles and re-checks
  as they step off (`Progress::gate_open`). Recipes: ember vent `"if": "!time:day & !time:dawn"`,
  `prop_closed: "ember_vent"`; rubble `"if": "flag:rubble_1"`, `prop_closed: "rubble"`; ford
  `"if": "item:wading_boots"`, `prop: "ford_stones"`; climb `"if": "item:grip_gloves"`,
  `prop: "climb_wall"` (or `climb_wall_basalt`) on the tile below the ledge; lantern path
  `"if": "time:night"`, `prop_open: "lantern_raft"` on water. A **ferry** is a person, not a gate: a branch
  `"if": "item:ferry_pass"` with `"do": ["warp:<map>:<x>:<z>:<facing>"]` (the `warp:` cue fades and loads
  that tile), and a branch without the pass. A dark cave is a door warp with `"if": "item:kiln_lantern"`
  and a `locked` text, into a `dark: true` map. check-maps flood-fills every map with all gates shut (the
  spawn must still reach an exit or a warp) and all open (every person, spot, warp and waymark must be
  reachable).
- **waymarks** (WORLD_DESIGN §7.3): the post is a prop plus a spot; the chart lists it.

  ```jsonc
  { "asset": "waymark", "x": 16, "z": 12, "rot": 0, "glow": "#d6ac4a" }   // props: crystal in the region's colour
  { "kind": "waymark", "id": "meadow", "x": 16, "z": 12 }                // interactions: same tile
  ```

  and in `atlas.json` `waymarks`: `{ "id": "meadow", "map": "overworld", "x": 16, "z": 13, "facing": "n",
  "name": "…", "name_fr": "…" }`, the tile the hero stands on, facing the post. The first touch lights it
  (flag `way_<id>`) and opens the world map; later touches open it to travel. The crystal is dim until lit.
  Entering an outdoor map sets `seen_<map>`: the chart inks its region and node.
- **atlas.json** (`web/world/atlas.json`, read by `web/src/worldmap.rs`): `regions` (id, name/name_fr, colour,
  centre `x`/`y` and radius `r` on a 12 × 9 chart), `maps` (every outdoor map: id, region, `x`/`y`, name and
  name_fr equal to the map's title), `links` (roads `a`–`b`, drawn once one end is seen; a gated one has
  `if` and `icon`: `lantern`, `ferry`, `boots` or `gloves`), `waymarks` (above). Posts must sit at least
  1.7 squares apart on the chart (44 px targets on a phone).
- **inns:** an innkeeper's branch with `"do": ["show:inn"]` opens the rest panel (rest until dawn, morning,
  dusk or night; free). The hub inn is `inn_hub` in Brightwell Square (Hollis).
- **1.3 PR 2 additions** (Ember and Storm):
  - `lightning: true` (map): far lightning at night, a short double flash of the sky every 20–40 s (off with reduced
    motion). **prop `pulse`** (a condition): an arc of light at the prop's `light_origin` every few seconds while it
    holds (the Rodspire with its pylons on).
  - **Smoke** rises from the first chimney and from every `kiln`, `glass_furnace` and `ember_vent` (`SMOKY` in
    `build.js`, eight sources at most); a gate's vents steam only while it is shut.
  - **prop `recolor`** works on any prop now: a recoloured prop is drawn on its own instead of instanced.
  - Static models that are not instanced (houses, kilns, the observatory…) are **merged** like the instanced ones:
    one draw for their flat-coloured parts; their named empties survive. Raised ground is one mesh with the ground
    (`terrain.js`), plus one shadow-only copy of its raised part.
  - A **door warp to the same map** carries check-maps' flood fill on to where it lands.
  - **`lifts`** (map, lift.js): a cage the hero rides between two heights of one map, the camera following.
    `[{ "id", "x", "z", "top", "bottom"?, "head"?, "cage"?, "rot"?, "if"?, "locked"? }]`: `x, z` the shaft tile
    (walkable even under a tower prop's footprint), `top` / `bottom` the landing heights (bottom: the tile's own),
    `if` the power (`locked` is said while it fails), `head` where the cables end (default top + 3.1), `cage` the
    model (`lift_cage`). The landings are the shaft's neighbours at either height that a gate of the cage faces.
    Inside: "Ride the lift" (E, the prompt, a tap on the cage); on a landing it isn't at: "Call the lift". Input
    is locked during a ride (gate shuts, an eased climb, a little bob, gate opens, the hero steps off); with
    reduced motion it's a fade. Nothing is saved mid-ride (the tile doesn't change); a reload on the shaft starts
    at the bottom. A cage model: `gate*` nodes hinged on their +X edge (they swing out), `cable*` nodes 1 m tall
    from their origin (stretched up to `head`). check-maps joins the shaft and the landings at both heights.
  - The effect **`sfx:<name>`** plays one of `engine::overworld::SOUNDS` (`spark`, `chime`, `jingle`, `gate`, `seal`).
  - Coloured glassware (`glass_teal`, `glass_amber` … materials) keeps its colour at night and catches the light
    softly; only plain `*glass*` (windows) turns warm.
  - `tidepool` is walkable (WALKABLE); `stairs_basalt` and `stairs_heath` are decks.
  - A regional inn (an `inn_<region>` interior listed in `engine::shop::INNS`) opens its counter under the rest panel
    of `show:inn`.
- **1.3 PR 3 additions** (Tide and Dream):
  - **gate `tide: true`**: the sea covers the gate's tiles in the ground shader while it is shut and drains while it is open,
    over five seconds (at once on a fresh build or with reduced motion); `flood: [[x, z], …]` adds tiles that flood with it
    but are not the gate (under its boardwalks and props). The Bellflats' `flats` and both ends of the causeway.
  - **map `gulls: n`**: n gulls glide high over the hero's part of the map by day and roost at night.
  - **moths**: after dark, eight moths flutter round the lamps (and glimmers) nearest the hero, on every map.
  - **glimmers**: `floating_lantern`, `lantern_raft` (a gate's `prop_open`) and `moonflower` (while its `open` holds) get a
    small tinted halo and a soft pool of light at their `glow` parts after dusk. Not real lights: one point cloud per size and
    one instanced pool mesh for all of them.
  - **moonflowers with `open`** are instanced as two sets, closed and posed open (the end of their `Open` clip), cross-faded
    when the condition changes: a whole bed costs two draws.
  - **lantern rafts** are instanced (a little turned and apart, so the path reads as rafts drifting together).
  - **map `fence_tint`** (a colour): multiplies the map's `f` pieces (the Moonwell's dusky hedges).
  - Props: `harbour_house` (5×4), `bell_tower` (3×3), `net_rack` (2×1), `willow`, `wishing_tree` (2×2, sway), `moonwell` (3×3),
    `lake_house` (4×4), `wisteria_arch` (2×1, walked under), and the interior `chart_table`, `cloth_rack` (2×1), `dress_form`,
    `paper_lanterns`, `chowder_stove`; `tidepool` and the hedge set are batched.
  - **check-maps** also turns the clock: for every phase, once the `time:` gates of that phase have shut, every tile the hero
    can stand on must still reach an exit, a door or a spot that warps (it found a dead-end pocket on the Kilnmouth).
- **Debug hooks** on the world's host element (`.world-host`): `sixfoldClock(minute)` sets the time,
  `sixfoldWarp(map, x, z, facing)` goes anywhere, `sixfoldStats()` reads draw calls, the area and the hour, and `sixfoldCam({ distance: 40, pitch: 65 })` pulls the
  camera back (no fog) to review a whole map until the next one loads.
