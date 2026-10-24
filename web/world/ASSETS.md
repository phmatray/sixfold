# World assets and maps (contract for the Blender side)

`world.js` builds each map from `web/world/maps/<name>.json` (the game starts in
`overworld`) and loads the models below. A missing model is fine: the scene draws a
placeholder of the same footprint, so models can land one at a time. Reload the page
to see a new file.

## The grid

- 1 tile = 1 m = 1 world unit. Tile (x, z) covers [x, x+1] × [z, z+1]; its centre
  is (x + 0.5, z + 0.5). A map spans [0, width] × [0, height].
- −Z is "north", away from the camera (the top of the screen); +Z faces the camera.
- The hero stands on tile centres and moves one tile per step: 0.25 s, or 0.125 s
  running (once it owns the running shoes).
- Collision is per tile: a tile is blocked if a prop's footprint covers it (except
  the walkable `rug` and `exit_mat`), an NPC stands on it, or its tile character is
  `#`, `f`, `W` or `V`. Outside the map is blocked.
- Press G in the game to show the grid with a legend (blocked, look or use, door, person).

## Models

Outdoor models live in `assets/`, interior ones in `assets/interior/`, NPCs in
`assets/npc/`.

| File | What | Footprint (X × Z) | Height guide |
|---|---|---|---|
| `house.glb` | The duel hall, door on the front (+Z) edge | 7 × 5 | ~5 m |
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
| `npc/<model>.glb` | NPC on the hero rig (`Idle`, `Walk`, `Fidget`) | — | ~1.45 m |

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
- **Wind:** trees, bushes, flowers and grass_edge sway in the vertex shader, weighted by a
  `COLOR_0` vertex colour (red: 0 at the base, 1 at the tips; never shown as a tint).
  Without it, the game uses height above the ground.
- **Shadows:** bake a soft contact-shadow disc into prop bases; the game adds one only
  under placeholders and characters.
- **House:** an empty `smoke_origin` at the chimney top. The door opens with an `Open` clip
  (played forward to open, back to close); without one, a node named `door` swings about
  +Y on its own origin (put it on the hinge).
- **Lamp:** an empty `light_origin` where the warm point light sits.
- **Legal:** original designs only. No logos, characters, buildings or text from any
  existing game. The only emblem is the Sixfold hexagon.

## Ground

One flat ground mesh per map at y = 0 (nothing else may be coplanar with it). Its
shader blends two layers through a per-tile mask (path tiles in the overworld) with
noisy, painted borders. Textures: `textures/grass.webp`, `dirt.webp`, `wood_floor.webp`
(seamless, 1024²; convert from PNG with `cwebp -q 82`; only the .webp files are
committed). A procedural fallback is drawn when one is missing.

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
  "tiles": ["....p....", ...],          // `height` rows of `width` chars; row 0 is z = 0 (far side)
      // "." grass, "p" dirt path, "w" wood floor (walkable)
      // "f" fence, "W" wall, "V" wall with a window, "#" blocked
  "props": [{ "asset": "house", "x": 10, "z": 6, "rot": 0 }, ...],
  "spawn": { "x": 13, "z": 14, "facing": "n" },
  "interactions": [{ "x": 16, "z": 11, "kind": "sign", "text": "..." }, ...],
  "warps": [{ "x": 13, "z": 10, "to": "shop", "label": "Go inside", "spawn": { "x": 5, "z": 7, "facing": "n" } }],
  "npcs": [{ "id": "wren", "name": "Wren", "model": "npc_wanderer", "tint": "#7fb6ff", "scale": 0.94,
             "x": 6, "z": 13, "facing": "e", "idle": "Idle", "route": ["e", "e", "s", ...],
             "dialogue": [{ "if": "!item:running_shoes", "lines": ["..."], "do": ["give:running_shoes"] },
                          { "lines": ["..."] }] }],
  "ground": { "base": "grass", "path": "dirt", "margin": 9 },
  "camera": { ... },
  "light": { ... }
}
```

- **props:** `x`, `z` = the footprint's min-corner tile. `rot` ∈ 0, 90, 180, 270 degrees
  about +Y (counter-clockwise seen from above: rot 90 turns the +Z front toward +X); 90
  and 270 swap the footprint. The model sits at the footprint's centre.
- **Directions:** `n` = −Z, `s` = +Z, `e` = +X, `w` = −X.
- **interactions:** the tile the hero must **face** to act with E / tap. `kind` `duel` offers a
  match; `sign` shows `text`.
- **warps:** a door warp (no `dir`) sits on a solid tile; facing it shows `label`, and stepping
  into it (or E) walks in, fades, and loads map `to` with the hero on `spawn`. A warp with
  `dir` (the exit mat) fires when the hero steps onto it, or pushes `dir` while standing on it.
- **npcs:** block their tile. `model` loads `assets/npc/<model>.glb`; until it exists the hero
  model is used, lerped toward `tint` and scaled by `scale`. `route` (optional) is a loop of
  one-tile steps. Speaking (E or tap while facing them, also across a counter) turns them to
  the hero and runs `dialogue`: the first branch whose `if` holds plays its `lines`, then
  applies its `do` effects. Conditions and effects are the engine's progression language
  (`engine/src/overworld.rs`): `item:x` / `flag:x` (negate with `!`), `give:x` / `flag:x`.
  Talking also sets `flag:talked_<id>`. Progress is saved in localStorage.
- **ground:** the two layers (`base`, and `path` under `p` tiles) and how far the ground
  runs past the map edge.
- **camera:** `fov` (vertical, degrees; `fovPortrait` when the view is taller than wide),
  `pitch` (degrees below horizontal), `distance` (`distancePortrait`), `lookAtOffset`, and
  `clamp` (indoors: keep the view inside the room). The camera looks at hero + lookAtOffset
  from hero + lookAtOffset + distance × (0, sin pitch, cos pitch), never rotating.
- **light:** `sun` colour/intensity and `fromHero` (outdoors the sun sits at hero + fromHero;
  indoors, with `indoor: true`, at the room centre + fromHero, as window light), `hemisphere`
  sky/ground colours, `background` (also the fog colour), optional `fog` range, and indoors
  `lamp` (a point light on every lamp). Intensities are three.js (physically based) units.
