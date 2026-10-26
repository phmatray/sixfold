# Sixfold — World Rules

*Règles du monde : où l'on pose les choses, et pourquoi.*

These are the placement rules every map follows. A map author applies them while laying a map, and
`tools/check-maps.mjs` enforces the ones a machine can judge. **Check** says how each rule is held:
**error** fails the build with `--strict` (`node tools/check-maps.mjs web/world --strict`; CI runs it so since every map was re-laid to these rules; without the flag the errors are listed as warnings), **warn** is listed
for a person to judge, and **eye** is checked by walking the map (overhead view, then ground level, by day
and by night, on a desktop and a phone).

They come from the owner's level-design checklist (landmarks, guiding without arrows, readable
collisions, rhythm, density, scale) and from what went wrong in 1.3: props stacked one in front of
another, lamps scattered with no reason, maps too small to feel like places, and exits you had to hunt for.

The camera looks north and down at about 50°. A prop therefore hides what stands behind it, up to about
0.85 × its height. "In front of" means south of, toward the camera. A prop's face is its +Z side
(`rot` 0 faces the camera).

---

## 1. Clearance — *dégagement*

| # | Rule | Check |
|---|---|---|
| 1.1 | No prop stands in another's footprint. Props on decks and rugs are fine, and so are two `if` props that take turns. | error |
| 1.2 | A post (sign, signpost, waymark, hanging sign, inn sign) has a free, walkable tile in front of its face. | error |
| 1.3 | Nothing taller than 0.95 m stands on the tile in front of something you use: a spot, a find, a person, a door. Counters and stalls are the exception, because you speak across them. | error |
| 1.4 | Nothing over 2 m stands two tiles in front of something you use, and nothing over 3 m (a tree, a house) three tiles in front. If the thing in front is itself a post or a person, it's an error at any of those distances. | warn / error |
| 1.5 | A post keeps one clear tile between itself and any other interactable. Things you use are never side by side, and never stacked in depth. A shop's hanging sign or inn sign may stand beside its own door. | error |
| 1.6 | People walk their `route` on open tiles only. | error |
| 1.7 | Coming out of a house puts you on the tile in front of the door that leads back in. | error |
| 1.8 | Going in, the hero climbs a house's steps to its door and never wades through them: every prop a door warp sits on has a `STOOP` entry in `props.js` (how far in to walk, and the tread heights read off the model). | error |

## 2. Lamps follow use — *l'éclairage suit l'usage*

A lamp marks where people go after dark. It never decorates empty ground.

| # | Rule | Check |
|---|---|---|
| 2.1 | A street lantern stands within 2 tiles of a path or plaza tile (`p`, `r`, `=`, `H` or a deck). | error |
| 2.2 | In town, lanterns line the main street at a steady rhythm of one every 5–7 tiles, alternating sides. They also stand at each junction, flank the doors of public houses (inn, shop, hall), ring the square, and light bridges, piers and stairs. Two lanterns closer than 4 tiles make a clump. | warn |
| 2.3 | Out of town (a `route`, `site` or `cave` map, or a `wild` rect on any map), there are **no street lamps** unless a lamp gives a reason: `"why": "the boathouse door"`. Good reasons are a door, a bridge, a pier, a camp, a shrine, or a town gate at a map edge. | error |
| 2.4 | The wilderness glows its own way: a campfire, a lantern hung by a door or a boat, glowing mushrooms (`mushroom_ring`), crystals (`crystal_cluster`), moonflowers, floating lanterns. | eye |
| 2.5 | Budget: only the hero's lantern and the 3 nearest lamps are real lights (`daylight.js` `LIVE`), and every other lamp only glows. Any number of lamps is cheap, but each one still needs a reason. | — |
| 2.6 | **Light by setting.** Nothing underground could power a street lamp, so a cave (`dark: true`, kind `cave`, or a room with `"cave": true`) has **no `lantern` and no `lamp`**. Natural caves and wild places glow with mushrooms (`glow_mushroom`, `glow_mushroom_tall`, `glow_shelf`), crystals (`crystal_cluster`) and glow plants. A worked mine has miners' oil lanterns hung on its timbers, plus mushrooms. Towns and paths have street lamps. A few mushrooms by the path and at junctions show the way, and a tall cluster or a crystal marks a landmark. | error |

## 3. Signs — *panneaux*

| # | Rule | Check |
|---|---|---|
| 3.1 | Every edge exit has a signpost within 6 tiles that names the destination by its title, in both languages: "North: Cinderwick" / "Au nord : Mèche-Cendre". | error (EN), warn (FR) |
| 3.2 | Each junction of main paths has one signpost naming where each branch goes. | eye |
| 3.3 | A sign stands beside the path, at its edge, facing the path or the camera. It never stands on the path and never in front of another prop (1.2–1.5). | error / eye |
| 3.4 | One sign says one thing. A house sign stands by its door, and a story sign stands by what it describes. | eye |

## 4. Waymarks — *bornes de voyage*

| # | Rule | Check |
|---|---|---|
| 4.1 | A waymark stands in a small clearing or plaza with a clear tile all round it (8 neighbours open). | error |
| 4.2 | You see it on the way in: it stands on or beside the main path into the town or site, not behind a building. | eye |
| 4.3 | The atlas landing tile is the tile in front of it, facing it (already checked). | error |

## 5. Exits and entrances — *entrées et sorties*

An exit should be obvious from a screen away.

| # | Rule | Check |
|---|---|---|
| 5.1 | An exit is at least **3 tiles wide**. The way back on the next map is as wide. | error / warn |
| 5.2 | A path (`p`, `r`, `=`, a deck) runs right out to the edge: at least 2 of the exit's edge tiles are path. | error |
| 5.3 | The throat is open: the two rows behind every exit tile are walkable and free. | error |
| 5.4 | The border elsewhere is **closed**: every other walkable edge tile is a tree, fence, cliff, rock, wall or water. There are no invisible walls. | error |
| 5.5 | The opening reads as a gap: gateposts, a fence ending, a hedge arch, or a trail between two trees, with a signpost (3.1) and if possible a landmark that hints at the next region (a kiln chimney, masts, a standing stone). | eye |
| 5.6 | You arrive facing into the map, on a walkable tile. `world.js` already turns you the way you walked, and check-maps checks the edge tiles. | error |
| 5.7 | Biome transition: the 4–6 tiles by an exit borrow the next region's ground and scatter, such as ash and an ash pine toward the Emberlands, sand and reeds toward the coast, or heather toward the Downs. A region never changes in one tile. | eye |

## 6. Districts and logic — *quartiers et logique*

| # | Rule | Check |
|---|---|---|
| 6.1 | A house faces a path: its door has path within 3 tiles in front. It has a front garden, a yard or a doorstep, never a bare wall on the street. | warn / eye |
| 6.2 | Fences enclose something: a field, a garden, a pen, a drop. A fence never stands alone in the open. | eye |
| 6.3 | Props cluster by function. A market has its stalls, crates and benches together. A harbour has its piers, nets, pots and boats. A farm has its fields, fences, hay and a barn. A quarry has its stairs, crates and carts. Nothing is sprinkled evenly over the map. | eye |
| 6.4 | Density varies on purpose. Dense groves sit beside open meadows, and busy squares beside quiet lanes. An empty stretch is a view or a breather, never missing content. | eye |
| 6.5 | Every map has a landmark visible from far off, and at least one wordless story (a cold campfire, a fallen bridge, tracks). Each nook off the main path hides something: a find, a shortcut or a detail. | eye |
| 6.6 | No pocket of open ground the hero can't reach. A save that lands there falls back to the spawn, but it still looks like a place you should be able to go. | warn |

## 7. Scale — *échelle*

| # | Rule | Check |
|---|---|---|
| 7.1 | Doors fit the hero (about 1.45 m), and buildings use the asset contract's sizes (ASSETS.md). We don't shrink a house to fit a map. | eye |
| 7.2 | Houses on one street stand 2–3 tiles apart, with their gardens between. A town keeps its spacing. | eye |
| 7.3 | Paths are 2 tiles wide for a main road, 3 for a high street or a road into a plaza, and 1 for a garden or goat track. | eye |

## 8. Map size — *taille des cartes*

### 8.1 Reference: the classic handheld RPGs

The classic handheld RPGs measure their maps in walking steps, and one screen is 10 × 9 steps. These sizes
come from the community disassembly:

| Kind | Steps | Screens |
|---|---|---|
| small village | 20 × 18 | 2 × 2 |
| town | 40 × 36 | 4 × 4 |
| big city | 50 × 36 | 5 × 4 |
| short route | 20 × 36 | 2 × 4 |
| long route | 20 × 72 or 90 × 18 | 2 × 8 or 9 × 2 |

Later generations are similar: villages about 20 × 20 and routes 40–60 long.

### 8.2 Our screen, measured

Our camera frames differently, so we use **screens** as the unit and convert them with our own framing,
measured with the debug grid (`G`) at the default camera:

- **desktop** (1440 × 900): about **13 tiles across** at the hero's row and **9 deep**;
- **phone** (390 × 844, portrait): about **7 tiles across** and **13 deep**.

One screen = **13 × 9 tiles** (the desktop). The hero walks 4 tiles a second (runs about 6), so crossing a
screen takes about 3 s, close to the classic games.

### 8.3 Targets and minimums

| Kind (`"kind"`) | Target | In tiles | Minimum (error) |
|---|---|---|---|
| `village` | 2 × 2 to 3 × 3 screens | 26 × 18 to 39 × 27 | 2 × 2 (26 × 18) |
| `town`, regional hub | about 4 × 4 | 52 × 36 | 3.7 × 3.5 (49 × 32) |
| `hub` (the meadow) | town-sized, 4 × 4 | 52 × 36 | as a town |
| `route` | 2 × 4 to 2 × 6, or long and narrow | 26 × 36–54 north–south, 52–78 × 18 east–west | 2 across × 4 along |
| `site` (seal site) | 3 × 3 to 4 × 4 | 39 × 27 to 52 × 36 | 3 × 3 (39 × 27) |
| `cave` | 1 × 3 or longer | 13 × 27, or 39 × 9 | 1 across × 3 along |
| interiors | one room, on the kit | 11 × 9 (13 × 11) | — |

Every outdoor map has a `kind`.

### 8.4 The limit, measured

The limit is now **80 × 72**, with at most **80 × 64 tiles in all** (it was 40 × 32). Only the current map is
built. Instanced scenery is split into 16 × 16-tile chunks, so whatever is off screen and outside the sun's
shadow box is culled, and the triangles drawn don't grow with the map. These figures were measured on the
phone viewport (390 × 844) at night with the CPU slowed 4×:

| Map | Draw calls | Triangles drawn | Build | Frame |
|---|---|---|---|---|
| meadow 28 × 24, before chunks | 207 | 333k | — | 16.7 ms |
| meadow 28 × 24, with chunks | 206 | 251k | 0.7 s | 16.7 ms |
| test 64 × 48 forest, before chunks | 71 | 1,033k | — | 16.7 ms |
| test 64 × 48 forest, with chunks | 194 | 505k | 0.6 s | 16.7 ms (p95 17.5) |
| test 80 × 64 forest, with chunks | 195 | 505k | 0.6 s | 16.7 ms (p95 17.3) |

Grass tufts scale with the map's area, so a big map keeps the same density.

### 8.5 Bigger is not emptier

A bigger map needs more places, not more space. For every extra screen, add a district, a landmark or a
point of interest: a pond, an orchard, a ruin, a lookout, a camp, a side garden with a find. The density and
reward rules (6.3–6.5) apply to every screen. A screen with nothing to stop for is a screen too many.

---

## 9. Map data for the rules

- `kind` (outdoor maps): `village`, `town`, `hub`, `route`, `site` or `cave`.
- `wild` (optional): `[[x0, z0, x1, z1], …]`, rects of wilderness on a town or hub map, where lamps need a `why`.
- `why` (lantern props): the reason a lamp stands where no street does.
- `cave` (an interior): a cave room; no lamps (2.6).
- `glow` (a glow prop): its hue, such as teal `#5fe0c8` on the coast, violet in the Hushlands or amber in the Stonebacks. `light: true` (indoors) gives the prop the room's point light, in that hue.

## 10. Method — *méthode*

1. **Paper map first.** Mark the links, the landmark, the districts and the gates (WORLD_DESIGN §2.1, §10).
2. **Blockout.** Lay out the paths, plazas and the closed border, then walk it in the overhead view
   (`el.sixfoldCam({ distance: 40, pitch: 62 })`).
3. **Dress by function**, district by district (6.3), then check against this list.
4. Run `node tools/check-maps.mjs web/world --strict` and fix every error. Read every warning.
5. **Walk every exit both ways**, by day and by night, on a desktop and a phone. Make sure it's obvious
   where to go.
