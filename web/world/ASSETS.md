# World assets and map (contract for the Blender side)

`world.js` builds the scene from `map.json` and loads each model below from
`web/world/assets/`. A missing model is fine: the scene draws a placeholder of the
same footprint, so models can land one at a time. Reload the page to see a new file.

## The grid

- 1 tile = 1 m = 1 world unit. Tile (x, z) covers [x, x+1] × [z, z+1]; its centre
  is (x + 0.5, z + 0.5). The map spans [0, width] × [0, height].
- −Z is "north", away from the camera (the top of the screen); +Z faces the camera.
- The hero stands on tile centres and moves one tile per step (0.25 s per tile).
- Collision is per tile: a tile is blocked if a prop's footprint covers it, or its
  tile character is `#` or `f` (fence). Outside the map is blocked.
- Press G in the game to show the grid (red = blocked, gold = interaction tile).

## Models

| File          | What                                        | Footprint (X × Z tiles)   | Height guide |
|---------------|---------------------------------------------|---------------------------|--------------|
| `house.glb`   | Cottage, door on the front (+Z) edge         | 7 × 5                     | ~5 m         |
| `tree.glb`    | Round-canopy tree                            | 2 × 2                     | ~3.5 m       |
| `bush.glb`    | Leafy bush                                   | 1 × 1                     | ~0.7 m       |
| `flowers.glb` | Flower clump                                 | 1 × 1                     | ~0.3 m       |
| `rock.glb`    | Boulder                                      | 1 × 1                     | ~0.6 m       |
| `sign.glb`    | Signboard on one post, face toward +Z        | 1 × 1                     | ~1.3 m       |
| `fence.glb`, `fence_corner.glb`, `fence_end.glb`, `fence_t.glb` | Fence pieces, see Fences below | 1 × 1 | ~0.9 m |
| `path.glb`    | One dirt tile, raised ~2 cm, edges that join neighbours (optional variants `path_b.glb`, `path_c.glb`) | 1 × 1, walkable | — |
| `hero.glb`    | Chibi player character                       | —                         | ~1.45 m      |

The house is 7 wide so the door sits on the centre column: door tile = column 3
(0-based) of the front row, i.e. footprint origin + (3, 4). The door's model
position is x = 0 (the footprint centre), on the +Z wall at z ≈ +2.5.

Rules for every model:

- **Format:** glTF binary (`.glb`), one root scene per file.
- **Axes and units:** glTF Y-up, 1 unit = 1 m (Blender's glTF exporter "+Y Up" does this).
- **Origin:** at the **centre of the footprint**, on the ground (y = 0). A 2 × 2 tree's
  origin is at the shared corner of its four tiles; a 1 × 1 prop's is at the tile centre.
- **Facing:** the front faces **+Z** (door, sign face, hero's eyes).
- **Fit:** stay inside the footprint (a little canopy or roof overhang is fine).
- **Materials:** flat colours or a baked painterly base-colour texture (≤ 1024 px). No
  PBR maps; the game re-shades with a toon material, keeping base colour, its texture and alpha.
- **Budget:** ≤ 3k triangles per prop, ≤ 6k for the hero.
- **Hero animations:** optional, named `Idle` and `Walk` (any case). Walk is an in-place
  loop; one step (one leg forward) per 0.25 s tile reads best. Models load at 1:1, never rescaled.
- **Legal:** original designs only. No logos, characters, buildings or text from any
  existing game. The only emblem is the Sixfold hexagon.

## Fences

Fences are tiles (`f` in `map.json`), not props. Each fence tile picks a piece and a
rotation from which of its 4 neighbours are also fence. Every piece is 1 × 1, origin
at the tile centre, arms reaching the tile edges. At rot 0:

- `fence.glb`: straight, arms to −X and +X
- `fence_corner.glb`: arms to −X and +Z
- `fence_end.glb`: one arm to −X (a post at the tile centre)
- `fence_t.glb`: arms to −X, +X and +Z

Mask bits: N (−Z) = 1, E (+X) = 2, S (+Z) = 4, W (−X) = 8. `rot` as for props below.

| Mask | Neighbours | Piece    | rot |   | Mask | Neighbours | Piece    | rot |
|------|------------|----------|-----|---|------|------------|----------|-----|
| 0    | none       | fence    | 0   |   | 8    | W          | end      | 0   |
| 1    | N          | end      | 270 |   | 9    | N W        | corner   | 270 |
| 2    | E          | end      | 180 |   | 10   | E W        | fence    | 0   |
| 3    | N E        | corner   | 180 |   | 11   | N E W      | t        | 180 |
| 4    | S          | end      | 90  |   | 12   | S W        | corner   | 0   |
| 5    | N S        | fence    | 90  |   | 13   | N S W      | t        | 270 |
| 6    | E S        | corner   | 90  |   | 14   | E S W      | t        | 0   |
| 7    | N E S      | t        | 90  |   | 15   | all        | t 0 + end 270 | — |

## `map.json`

```jsonc
{
  "width": 28, "height": 24,            // in tiles
  "tiles": ["....p....", ...],          // `height` rows of `width` chars; row 0 is z = 0 (far side)
                                        // "." grass, "p" path (walkable, draws path.glb),
                                        // "f" fence (blocked, auto-tiled), "#" blocked
  "props": [{ "asset": "house", "x": 10, "z": 6, "rot": 0 }, ...],
  "spawn": { "x": 13, "z": 14, "facing": "n" },
  "interactions": [{ "x": 13, "z": 10, "kind": "duel", "text": "..." }, ...],
  "camera": { ... },
  "light": { ... }
}
```

- **props:** `x`, `z` = the footprint's min-corner tile. `rot` ∈ 0, 90, 180, 270 degrees
  about +Y (counter-clockwise seen from above: rot 90 turns the +Z front toward +X).
  90 and 270 swap the footprint's X and Z size. The model is placed at the footprint's
  centre: (x + w/2, 0, z + d/2) with the rotated w × d.
- **spawn.facing / directions:** `n` = −Z, `s` = +Z, `e` = +X, `w` = −X.
- **interactions:** the tile the hero must **face** (standing next to it) to act with E / tap.
  `kind` `duel` offers a match; anything else shows `text` as a sign.
- **camera:** `fov` (vertical, degrees; `fovPortrait` when the view is taller than wide),
  `pitch` (degrees below horizontal), `distance` (`distancePortrait`), `lookAtOffset`.
  The camera looks at hero + lookAtOffset from
  hero + lookAtOffset + distance × (0, sin pitch, cos pitch), i.e. from +Z, never rotating.
- **light:** sun colour/intensity and `fromHero` (the sun sits at hero + fromHero and points
  at the hero), hemisphere sky/ground colours, background (also the fog colour), fog range.
  Intensities are three.js (physically based) units.
