# 1.3 region assets ("Six regions")

The props, cliff sets, auto-tile sets and ground textures the six 1.3 regions need (`docs/WORLD_DESIGN.md` §10, §14),
built ahead of the code. Every model follows the contract in `ASSETS.md`: glTF binary, Y-up, 1 unit = 1 m, origin at the
footprint centre on the ground, front +Z, ≤ 3k triangles, < 2 MB, flat palette colours merged by `build.js`. Materials whose
name contains `glow` are the night-emissive parts (the props-v2 rule: keep them out of the merged vertex-colour mesh so
the name survives). Wind props carry COLOR_0 (red = sway weight).

**Everything below the asset tables is a suggestion.** This branch adds models and textures only; `props.js`, `build.js`,
`ground.js` and the maps are left to the 1.3 code PRs.

Counts: Ember 12, Storm 12, Tide 11, Dream 9, Grove 14, Stone 13 — **71 models**, plus 7 ground textures.
Rebuilds are geometry-deterministic (seeded); the glTF exporter may order n-gon triangles differently between runs, so a
rebuild can change index bytes without changing a single vertex.

Regenerate (headless only):

```bash
blender -b --factory-startup -P tools/blender/build_regions_v13.py -- --region all --out web/world/assets \
    --sheets /tmp/v13 --textures /tmp/tex --check
blender -b --factory-startup -P tools/blender/bake_ground_textures.py -- --out /tmp/tex \
    --only ash,heath,wet_sand,dusk_grass,moss,slate,cave       # then cwebp -q 82 into web/world/textures/
```

`tools/blender/v13_<region>.py` holds each region's props; `v13_kit.py` the shared helpers, export checks and the review
renders. The `--sheets` renders imitate the game: toon gradient (90/160/215/255 on N·L), shadowed sun, hemisphere light and
linear fog from the region's `light` block, plus the 1.3 night preset with `glow` parts lit.

## What the code needs to use these (all small)

1. **Per-map cliff set.** `build.js` stacks `cliff` / `cliff_b` for digit tiles. Add a map key, for example
   `"cliff": ["cliff_basalt", "cliff_basalt_b"]` (the `_b` keeps the existing 35 % top-layer rule). Sets:
   `cliff_basalt` (Ember, the Rodspire causeway), `cliff_heath` (Storm), `cliff_sea` (Tide), `cliff_slate` (Stone);
   Dream and Grove keep `cliff` / `cliff_b`.
2. **Per-map fence set** (`docs/WORLD_DESIGN.md` §14.1): `"fence": "wall_dry" | "hedge"`. Both sets use the fence arm
   contract and the same `AUTOTILE` table. `hedge` is used by the Moonwell (the Tide and Dream PR) as well as the Elder Ring.
3. **Glow parts kept separate** when a model is flattened (props-v2 rule), so `daylight.js` can find `*glow*`.
4. **`Open` clips on props other than the hall door**: `pylon` (the lever, on facing it) and `moonflower` (the petals, on
   the dusk phase change; backwards at dawn). Animated props must not be `BATCHED`.
5. **Bobbing**: add `ferry`, `buoy`, `floating_lantern` and `lantern_raft` to the loop that bobs `boat`.
6. **`light_origin` / `smoke_origin`** empties on the new props are named as today (lamp glow sprite, chimney smoke,
   steam from the vents).
7. **Ladder**: the 3 m `ladder` joins floor 0 in front to 3.1 behind, more than the 0.6 m step rule allows. Suggest
   `DECK.ladder = [1.5]` plus a `LADDER` set whose tiles skip the step check toward their front and back neighbours.
8. **Ground textures**: `ash`, `heath`, `wet_sand`, `dusk_grass`, `moss`, `slate`, `cave` are in `textures/` (1024²
   webp, seamless). Suggested `REPEAT` (metres per tile): `ash: 5, heath: 6, wet_sand: 5, dusk_grass: 6, moss: 5,
   slate: 3, cave: 4`.

## Build order (follows the PR plan and §16's cut order)

| PR | Regions | Needs from this branch |
|---|---|---|
| 1.3 PR 1 | systems | `waymark`, `seal_stone`, `cave_mouth`, `climb_wall`, `inn_sign` (props-v2), the textures |
| 1.3 PR 2 | Ember + Storm | the Ember and Storm sets below |
| 1.3 PR 3 | Tide + Dream | the Tide and Dream sets **and the Grove `hedge` set** (Moonwell) |
| 1.3 PR 4 | Grove + Stone | the Grove and Stone sets |

Every region got its full list; nothing was cut. If art has to be trimmed later, the late regions' extras go first
(`fallen_trunk_end`, `canopy_deck`, `menhir`, `cave_mouth_slate`, `bollard`, `rock_sea`), as the cut order suggests.

---

## Ember: the Emberlands (1.3 PR 2)

Columnar basalt and ash, orange-lit brick bottle shapes, glass glinting everywhere. Silhouettes: hexagonal basalt columns
(cliff tops, stairs, rocks, the climb wall), the bottle kiln, scorched leaning pines, steam plumes. Ground `ash`, path
`stone`. Glow: `kiln_glow` (kiln mouths, furnace glory holes), `ember_glow` (vents, cracks, the pool).

| Asset | Foot | Collision | Height | Glow / empties / clip / wind | Tris | KB | Use |
|---|---|---|---|---|---|---|---|
| `kiln` | 4×4 | solid | 4.5 m | `kiln_glow`; `smoke_origin` | 2962 | 138 | the Great Kiln on Cinderwick's plaza: brick bottle on a plinth, big glowing arched mouth (+Z), four small firemouths, log stack, blowpipe rack, water butt in the corners |
| `glass_furnace` | 2×2 | solid | 2.8 m | `kiln_glow`; `smoke_origin` | 1416 | 72 | glassblowers' beehive furnace, three glory holes, copper hood and flue, a hot gather on its yoke |
| `glass_rack` | 2×1 | solid | 1.8 m | — | 2372 | 84 | cooling rack of coloured glassware (Wendell's "singing glass", the `e_rack` find) |
| `ember_vent` | 1×1 | solid | 1.7 m | `ember_glow`; `smoke_origin` | 784 | 41 | the Kilnmouth vent gates' `prop_closed`: cracked basalt mound, glowing throat, a leaning steam plume that clearly blocks |
| `lava_glass` | 2×2 | solid | 0.3 m | `ember_glow` | 708 | 49 | the Kilnmouth floor's lava-glass pool (draw it as a prop over `.` tiles instead of `~` water) |
| `ash_pine` | 1×1 | solid | 3.1 m | wind | 442 | 28 | the region's ring tree and scatter: charred leaning trunk, snapped top, lopsided surviving needles |
| `rock_basalt` | 1×1 | solid | 0.6 m | — | 376 | 28 | scatter and ring filler: toppled columns, a boulder, glass glints |
| `cliff_basalt`, `cliff_basalt_b` | 1×1 | cliff block, top 1.0 | 1 m | `_b`: `ember_glow` crack | 728 / 656 | 41 / 38 | the cliff set; `_b` is the top-layer variant (a cooled vent glowing among the columns) |
| `stairs_basalt` | 1×1 | deck 0.5 | 1 m | — | 756 | 44 | as `stairs` (treads 0.125…0.875, climbs toward −Z, stacks with `y`) |
| `climb_wall_basalt` | 1×1 | deck 0.5, gloves gate | 1 m | — | 904 | 49 | as `climb_wall` (tops 0.5 / 1.0) with copper staples: kiln catwalk, Kilnmouth NE ledge, the Rodspire wall |
| `cullet` | — | item | — | — | 120 | 12 | quest item popup (three shards) |

Suggested `props.js`:

```js
FOOT:    kiln: [4, 4], glass_furnace: [2, 2], glass_rack: [2, 1], lava_glass: [2, 2],
DECK:    stairs_basalt: [0.5], climb_wall_basalt: [0.5],
WIND_H:  ash_pine: 3.1,
BATCHED: ash_pine, rock_basalt, cliff_basalt, cliff_basalt_b, stairs_basalt, climb_wall_basalt, glass_rack  // ash_pine also in TREES
TALL:    kiln: 4.5, glass_furnace: 2.8, glass_rack: 1.8, ember_vent: 1.7, lava_glass: 0.3, ash_pine: 3.1, cliff_basalt: 1, cliff_basalt_b: 1, stairs_basalt: 0.5, climb_wall_basalt: 1,
```

Maps: `"cliff": ["cliff_basalt", "cliff_basalt_b"]`, `ground: { base: "ash", path: "stone", tufts: 0 }`,
`ring: ["ash_pine", "ash_pine", "rock_basalt"]`. The vents' steam comes from their `smoke_origin` (same puffs as the
cottage chimney); the kiln's too.

Sheets: `ember_tiles.png`, `ember_day.png`, `ember_night.png`.

## Storm: the Galewold (1.3 PR 2)

High heath on sea cliffs, dry-stone lanes, chime posts, copper and slate. Silhouettes: low walls in long lines, thin
vertical masts (chime posts, pylons, the lift, the Rodspire), the white observatory dome. Ground `heath`, path `dirt`.
Glow: `coil_glow` (pylon coils, the lift's line lamp, the spire tip), `lamp_glow` (observatory slit and windows).

| Asset | Foot | Collision | Height | Glow / empties / clip / wind | Tris | KB | Use |
|---|---|---|---|---|---|---|---|
| `wall_dry`, `_corner`, `_end`, `_t` | 1×1 | solid, auto-tiled | 0.95 m | — | 342 / 342 / 212 / 400 | 15–24 | the Downs' `f` tiles (`"fence": "wall_dry"`): granite courses with a cock-and-hen coping |
| `heather` | 1×1 | solid | 0.5 m | wind | 442 | 18 | ring and scatter: olive cushions crowned with lilac bloom |
| `chime_post` | 1×1 | solid | 2.6 m | wind | 734 | 56 | the chime ring on the Downs (the `st_chimes` night find) |
| `pylon` | 1×1 | solid | 3.2 m | `coil_glow`; `Open` (node `pylon_lever`, knife switch off → on) | 1880 | 96 | the three relay pylons; play `Open` and light the coil when `flag:pylon_n` is set |
| `observatory` | 5×5 | solid; door tile origin + (2, 4) | 5.7 m | `lamp_glow` | 2886 | 148 | Stormwatch's observatory (warp to `observatory`); telescope through the slit, gallery ring at ~2.6 m |
| `lift` | 2×2 | deck 0.1 | 5.4 m | `coil_glow` | 1584 | 90 | the Rodspire lift cage (its tiles hold the door warp `if` pylons 1–3) |
| `rodspire` | 3×3 | solid | 12.1 m | `coil_glow`; `light_origin` at the tip | 2750 | 141 | the copper spire on the sea stack (arcs at night from `light_origin`) |
| `cliff_heath`, `cliff_heath_b` | 1×1 | cliff block, top 1.0 | 1 m (+ tufts) | — | 520 / 536 | 32 | the cliff set: grey weathered granite under a heath top; `_b` adds heather and a boulder |

Suggested `props.js`:

```js
FOOT:    observatory: [5, 5], lift: [2, 2], rodspire: [3, 3],
DECK:    lift: [0.1],
WIND_H:  heather: 0.55, chime_post: 2.3,
BATCHED: heather, cliff_heath, cliff_heath_b, wall_dry, wall_dry_corner, wall_dry_end, wall_dry_t, chime_post  // not pylon (animated)
TALL:    observatory: 5.7, lift: 5.4, rodspire: 12.1, pylon: 3.2, chime_post: 2.6, heather: 0.5, wall_dry: 0.95, cliff_heath: 1, cliff_heath_b: 1,
```

Maps: `downs` and `stormwatch` use `"cliff": ["cliff_heath", "cliff_heath_b"]` and `"fence": "wall_dry"`; `rodspire`
uses `"cliff": ["cliff_basalt", "cliff_basalt_b"]`. `ground: { base: "heath", path: "dirt" }`,
`ring: ["heather", "rock", "bush_dark"]`, no ring on the sea sides. The Stormwatch bluff stairs can use `stairs_basalt`.

Sheets: `storm_tiles.png`, `storm_day.png`, `storm_night.png`.

## Tide: the Saltreach (1.3 PR 3)

Harbour life on stilts, painted boats, nets, buoys, then the wide flats. Silhouettes: steep-roofed painted houses on
posts, the leaning half-drowned bell tower, bobbing red buoys, sea-worn buff cliffs with a dark weedy foot. Ground
`sand`, path `wood_floor`, `wet_sand` on the flats. Glow: `td_window_glow`, `lamp_glow`, `td_buoy_glow`.
`td_pool_water` is named for the game's rippling-water shader on purpose.

| Asset | Foot | Collision | Height | Glow / empties / clip / wind | Tris | KB | Use |
|---|---|---|---|---|---|---|---|
| `harbour_house` | 5×4 | solid; door on column 2 of the front row (warp tile origin + (2, 3)), steps down from a 0.68 m porch | 4.3 m | `td_window_glow`, `lamp_glow`; `smoke_origin` | 2930 | 146 | Gullhaven houses, the inn and the Harbour Office (tint per house if wanted) |
| `ferry` | 4×2 | solid (deck top 0.44 for scripts); bow +X | 2.1 m | `td_window_glow`, `lamp_glow`; `light_origin` (masthead) | 1738 | 99 | Bo and Lou's ferry at the Gullhaven pier, the Stormwatch dock and the Lanternmere jetty; bobs |
| `bell_tower` | 3×3 | solid, leans ~7° | 6.8 m | `lamp_glow`; `light_origin` | 2388 | 126 | the Bellflats islet's drowned bell tower (`t_bell` by day, `t_lamp` at night) |
| `crab_pots` | 1×1 | solid | 0.45 m | — | 688 | 39 | quay and dock dressing (`st_dock` find) |
| `buoy` | 1×1 | solid (sits in water) | 1.4 m | `td_buoy_glow`; `light_origin` | 358 | 17 | channel markers; bobs |
| `net_rack` | 2×1 | solid | 1.45 m | — | 1440 | 63 | drying nets on the quay and boardwalk (Finch's post) |
| `cliff_sea`, `cliff_sea_b` | 1×1 | cliff block, top 1.0 | 1 m (+ tufts) | — | 592 / 664 | 43 / 49 | the cliff set: buff wave-worn strata, barnacles and weed at the foot, marram and sea pinks on top |
| `tidepool` | 1×1 | walkable flat (< 8 cm) | 0.07 m | `td_pool_water` | 768 | 50 | the Bellflats tide pools (finds with `day + boots`) |
| `rock_sea` | 1×1 | solid | 0.55 m | — | 452 | 25 | barnacled boulders on the flats and in the shallows; ring filler |
| `bollard` | 1×1 | solid | 0.6 m | — | 386 | 20 | pier and quay edges |

Suggested `props.js`:

```js
FOOT:     harbour_house: [5, 4], ferry: [4, 2], bell_tower: [3, 3], net_rack: [2, 1],
WALKABLE: tidepool,
BATCHED:  crab_pots, rock_sea, bollard, tidepool, net_rack, cliff_sea, cliff_sea_b  // not ferry or buoy (they bob)
TALL:     harbour_house: 4.3, ferry: 2.1, bell_tower: 6.8, buoy: 1.4, net_rack: 1.45, cliff_sea: 1, cliff_sea_b: 1, tidepool: 0.07,
```

Maps: `"cliff": ["cliff_sea", "cliff_sea_b"]` (the Bellflats' west wall), `ground: { base: "sand", path: "wood_floor",
tufts: 0 }` with the flats gate's tiles on `wet_sand`, `ring: ["reeds", "rock_sea", "rock"]` on the land sides. A tall
stack shows one wet band per block (it reads as strata). The fish market reuses `stall` / `stall_b`.

Sheets: `tide_tiles.png`, `tide_day.png`, `tide_night.png`.

## Dream: the Hushlands (1.3 PR 3)

Willow country in a soft twilight tint; a lake of floating paper lanterns; busier at night than by day. Silhouettes:
weeping willow curtains, the pink wishing tree, round lilac-roofed houses on piles, lantern rafts in a line across the
water, the moonwell's crescent arch. Ground `dusk_grass`, path `stone`. Glow: `dr_paper_glow`, `dr_rose_glow`,
`dr_window_glow`, `dr_moon_glow` (moonflower petals), `dr_moonpool_glow` (the well's pool).

| Asset | Foot | Collision | Height | Glow / empties / clip / wind | Tris | KB | Use |
|---|---|---|---|---|---|---|---|
| `willow` | 2×2 | solid | 3.8 m | wind (the curtain hem most) | 2228 | 133 | the region's ring and lane tree |
| `wishing_tree` | 2×2 | solid | 3.8 m | wind | 1578 | 99 | Hushwillow Lane's wishing tree: blossom, ribbons and tags, a coin bowl (`d_tree` find) |
| `floating_lantern` | 1×1 | solid (on water) | 0.6 m | `dr_paper_glow` | 344 | 18 | lanterns on Lanternmere; bobs |
| `lantern_raft` | 1×1 | deck 0.1 | 0.75 m (pole) | `dr_rose_glow` | 504 | 29 | the lantern path gate's `prop_open` (`floor: 0.1`); bobs |
| `ford_stones` | 1×1 | over water, low | tops ≤ 0.1 m | — | 412 | 26 | the Hushwillow ford gate's `prop` (always shown): stepping stones, foam, reeds |
| `moonflower` | 1×1 | solid | 0.5 m | `dr_moon_glow`; `Open` (buds `moonflower_bud0..6` shrink, blooms `moonflower_bloom0..6` grow and twist open) | 1574 | 60 | the Moonwell's beds and the Fernhollow glade: play `Open` at dusk, backwards at dawn (`d_bed`, `g_glade`) |
| `wisteria_arch` | 2×1 | walk-under (posts at x ±0.9) | 2.7 m | wind | 1920 | 96 | arches over lanes and the Moonwell's paths |
| `moonwell` | 3×3 | solid | 3.7 m | `dr_moonpool_glow`, `dr_paper_glow`; `light_origin` | 2516 | 138 | the seal-site well: stone ring open at the front, steps down to a glowing pool, crescent arch and lantern. The 2×2 `seal_stone` stands beside it (it can't fit inside) |
| `lake_house` | 4×4 | solid; door tile origin + (2, 3) | 4.6 m | `dr_window_glow`, `dr_paper_glow`, `dr_rose_glow`; `smoke_origin` | 2912 | 145 | Lanternmere's round houses on piles, the tailor's lake house |

Suggested `props.js`:

```js
FOOT:     willow: [2, 2], wishing_tree: [2, 2], moonwell: [3, 3], lake_house: [4, 4], wisteria_arch: [2, 1],
DECK:     lantern_raft: [0.1],
WALKABLE: wisteria_arch,            // or block just its post tiles with a small exception, as §10.4 suggests
WIND_H:   willow: 3.8, wishing_tree: 3.8, wisteria_arch: 2.3,
BATCHED:  willow, wishing_tree, ford_stones, wisteria_arch   // not moonflower (animated), not the bobbing lanterns
TALL:     willow: 3.8, wishing_tree: 3.8, moonwell: 3.7, lake_house: 4.6, wisteria_arch: 2.7, moonflower: 0.5, floating_lantern: 0.6, lantern_raft: 0.1, ford_stones: 0.1,
```

Maps: `ground: { base: "dusk_grass", path: "stone" }`, `ring: ["willow", "willow", "bush_dark"]`; `moonwell` uses
`"fence": "hedge"` (Grove set). The night sheet uses the doc's 30 %-brighter `light_night`.

Sheets: `dream_tiles.png`, `dream_day.png`, `dream_night.png`, and `dream_moonflower_open.png` (the bloom at the end of
the clip; the sheets show the rest pose).

## Grove: the Deepgrove (1.3 PR 4; the hedge set ships with PR 3)

Old forest grown huge: a fallen elder whose roots hold a village, walkways at 3 m, moss, a hedge maze, bees.
Silhouettes: the colossal mossy log, root arches, houses on stilts with ladders, plank decks and rope bridges in the air,
the seven ancient trees with flat stepped canopies, clipped hedges. Ground `moss`, path `dirt`. Glow: `gv_window_glow`,
`lamp_glow` (treehouse, root-arch lantern), `gv_shroom_glow` (three caps in the mushroom ring).

| Asset | Foot | Collision | Height | Glow / empties / clip / wind | Tris | KB | Use |
|---|---|---|---|---|---|---|---|
| `hedge`, `_corner`, `_end`, `_t` | 1×1 | solid, auto-tiled | 1.3 m | — | 628 / 604 / 424 / 808 | 18–32 | `"fence": "hedge"` in the Elder Ring maze and the Moonwell's walls |
| `standing_tree` | 3×3 | solid | 5.7 m | wind (canopy plates only) | 2836 | 109 | the Elder Ring's seven: buttressed gnarled trunk, a hollow, a red cord with ribbons, three flat stepped canopy plates with hanging moss |
| `great_root` | 4×2 | solid | 3.0 m | `lamp_glow`; `light_origin` | 1412 | 62 | root arches round Rootholm (houses hide under them) |
| `fallen_trunk` | 4×3 | solid | 2.9 m (+ ferns) | — | 896 | 41 | the fallen elder across Rootholm's north: tile segments along X; ends are open, so run them off-map or close with the end piece |
| `fallen_trunk_end` | 4×3 | solid | 4.1 m | — | 1764 | 74 | the root plate (on +X; rot 180 puts it on the west) |
| `treehouse` | 4×4 | solid; door at 3 m, door tile origin + (2, 3) | 7.6 m | `gv_window_glow`, `lamp_glow`; `light_origin`, `smoke_origin` | 2320 | 110 | Rootholm's treehouse (Jory); put a `ladder` at origin + (2, 4), rot 0 |
| `ladder` | 1×1 | deck (needs the ladder rule, top 3.1) | 3.4 m | — | 808 | 40 | up to the canopy walkway and the treehouse porch |
| `canopy_deck` | 1×1 | deck, used at `y` 3, top +0.1 | post to −3 m | — | 556 | 31 | the 3 m walkway (planks along X, like `boardwalk`), a post down to the ground |
| `rope_bridge` | 1×1 | deck, used at `y` 3, top +0.1 | 1 m rails | — | 612 | 29 | spans between walkway decks (walked along Z; rot 90 for east–west) |
| `beehive` | 1×1 | solid | 0.9 m | — | 682 | 43 | Agnes's skeps (the `g_hive` find) |
| `mushroom_ring` | 1×1 | solid | 0.45 m | `gv_shroom_glow` | 596 | 31 | decor in the glades and the maze |

Suggested `props.js`:

```js
FOOT:    standing_tree: [3, 3], great_root: [4, 2], treehouse: [4, 4], fallen_trunk: [4, 3], fallen_trunk_end: [4, 3],
DECK:    canopy_deck: [0.1], rope_bridge: [0.1], ladder: [1.5],   // ladder: plus the step exception (see above)
WIND_H:  standing_tree: 5.5,
BATCHED: hedge, hedge_corner, hedge_end, hedge_t, canopy_deck, rope_bridge, ladder, beehive, fallen_trunk, standing_tree
TALL:    standing_tree: 6, great_root: 3, treehouse: 7.6, fallen_trunk: 2.9, fallen_trunk_end: 4.1, ladder: 3.4, hedge: 1.3,
```

Maps: `elder_ring` (and `moonwell`) use `"fence": "hedge"`; `ground: { base: "moss", path: "dirt" }`;
`ring: ["pine", "birch", "standing_tree", "bush_dark"]` (the big tree alone makes the ring heavy). The hedge pieces are
richer than the fence (424–808 triangles); if a long maze costs too much on phones, dropping the face clumps saves ~30 %.

Sheets: `grove_tiles.png`, `grove_day.png`, `grove_night.png`.

## Stone: the Stonebacks (1.3 PR 4)

Slate mountains and mining terraces, rails and carts, a headframe, cairns; cool blue-greys with rust-orange accents, and
the dark Undermine under it all. Silhouettes: tilted plated slate terraces, the rust-red headframe with its wheels, carts
on rails, leaning menhirs, flagged cairns, timber mine sets and pale crystals in the dark. Ground `slate`, path `stone`
(the Undermine: `cave`). Glow: `sn_lamp_glow`, `sn_window_glow`, `sn_gem_glow` (crystals and ore glints).

| Asset | Foot | Collision | Height | Glow / empties / clip / wind | Tris | KB | Use |
|---|---|---|---|---|---|---|---|
| `cliff_slate`, `cliff_slate_b` | 1×1 | cliff block, top 1.0 | 1 m | — | 534 / 606 | 33 / 38 | the cliff set: thin tilted beds, rust drips, a plated top that hides the grid; `_b` adds lichen and a grass tuft |
| `stairs_slate` | 1×1 | deck 0.5 | 1 m | — | 652 | 37 | as `stairs` (Scree Pass switchbacks, Anvilstead and Bastion terraces) |
| `headframe` | 3×3 | solid | 6.1 m | `sn_lamp_glow`; `light_origin` | 2630 | 149 | the winding tower over Anvilstead's old shaft (the `sn_headframe` gloves find) |
| `terrace_house` | 4×3 | solid; back flush with the north edge, door on column 1 of the front row (warp tile origin + (1, 2)) | 5.0 m | `sn_window_glow`, `sn_lamp_glow`; `smoke_origin` | 2718 | 152 | Anvilstead's houses, the Tally Room inn, the climbing school, backed into a cliff on the row behind |
| `minecart` | 2×1 | solid | 1.1 m | `sn_gem_glow` | 1170 | 71 | ore carts on the rails (along X; wheels at the 0.6 m gauge) |
| `rail` | 1×1 | walkable flat | 0.07 m | — | 160 | 12 | track along X (rot 90 for Z); rails reach the tile edges |
| `rubble` | 1×1 | solid | 1.4 m | — | 820 | 52 | the Undermine's `rubble_1..3` gates' `prop_closed` |
| `cairn` | 1×1 | solid | 1.45 m | wind (pennant) | 484 | 41 | the six switchback cairns on Scree Pass (`flag:cairn_n`) |
| `support_beam` | 1×1 | walk-under frame (posts at the X edges) or solid | 2.1 m | — | 448 | 28 | mine timber sets in the Undermine |
| `crystal_cluster` | 1×1 | solid | 1.0 m | `sn_gem_glow` | 396 | 25 | the Undermine's crystals (`sn_crystal`); the cave's only cool light. Not named `crystal_glow`, so the waymark tint doesn't recolour it |
| `menhir` | 1×1 | solid | 2.7 m | — | 656 | 42 | the Bastion's stone ring (`sn_menhir`) |
| `cave_mouth_slate` | 2×1 | solid; the door warp is its tile | 2.2 m | `sn_lamp_glow` | 1090 | 66 | as props-v2 `cave_mouth` (back flush north, against a ≥ 2 m cliff) in slate and rust, rails running in: the Scree Pass side of the Undermine |

Suggested `props.js`:

```js
FOOT:     headframe: [3, 3], terrace_house: [4, 3], minecart: [2, 1], cave_mouth_slate: [2, 1],
DECK:     stairs_slate: [0.5],
WALKABLE: rail, support_beam,       // support_beam only if it should be a frame you walk under
WIND_H:   cairn: 1.2,
BATCHED:  cliff_slate, cliff_slate_b, stairs_slate, rail, cairn, menhir, rubble, support_beam
TALL:     cliff_slate: 1, cliff_slate_b: 1, stairs_slate: 0.5, headframe: 6.1, terrace_house: 5, minecart: 1.1, menhir: 2.7, support_beam: 2.1, rubble: 1.4, cave_mouth_slate: 2.2, rail: 0.07,
```

Maps: `"cliff": ["cliff_slate", "cliff_slate_b"]`, `ground: { base: "slate", path: "stone", tufts: 0 }`,
`ring: ["pine", "rock", "bush_dark"]`; the Undermine `ground: { base: "cave" }` with rails, beams, crystals and carts.

Sheets: `stone_tiles.png`, `stone_day.png`, `stone_night.png`.

---

## Not built (left for later)

- **Sheep** for Thrumming Downs (Ansel's flock): an NPC needs its own quadruped rig with `Idle`/`Walk`/`Fidget`.
- **Interior furniture** for the new 1.3 interiors (seven inns, glassworks, observatory, harbour office, tailor, cobbler,
  climbing school): they reuse the 1.2 interior kit; bespoke pieces (a glory-hole bench, a telescope) are a later pass.
- **Climb-wall variants** for slate and the Grove trunk: the shared sandstone `climb_wall` and `climb_wall_basalt` cover
  the gloves walls for now (the Rootholm trunk and the headframe would each suit a bespoke one).
- **Stairs for Storm**: `stairs_basalt` stands in on the Stormwatch bluff.
