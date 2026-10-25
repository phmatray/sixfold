# Blender tools for the overworld

Original, procedural models for `web/world/` (contract: `web/world/ASSETS.md`), built from
bmesh primitives. Requires Blender 5.2 (tested with 5.2.2 at `/opt/homebrew/bin/blender`). Run
everything from the repo root.

## One working file, several scenes

`art/Sixfold.blend` is git-ignored (`*.blend`, `*.blend1`). Each script rebuilds only its own
scene inside it and saves the file:

| Scene     | Built by                 | Contents |
|-----------|--------------------------|----------|
| `Assets`  | `build_assets.py`        | every model side by side on a 1 m grid, one `sx_<name>` collection each; the .glb exports come from here |
| `Preview` | `build_assets.py`        | contact-sheet layout (instances, grass, grid, camera) |
| `World`   | `build_world_scene.py`   | `web/world/maps/overworld.json` assembled from the exported .glb files: fences auto-tiled, sway, smoke, hero Idle loop, game camera |
| `NPCs`    | `build_npcs.py`          | NPC variants on the hero's rig (shopkeeper, gardener, wanderer, kid, rival) and the campaign rivals (grass, fire, water, lightning, fighting, psychic, champion) -> `assets/npc/npc_*.glb`, `assets/npc/rival_*.glb` + `npc_palettes.json` |
| `Interior`| `build_interior_scene.py`| `web/world/maps/shop.json`: walls auto-tiled with the fence rules (cutaway `wall_low*` on the camera-side south row, `wall_window` on chosen straight tiles), wood floor, furniture, lamp light, hero |
| `Areas`   | `build_area_props.py`    | the 0.7 outdoor-area props (woodland, seaside, quarry, square) side by side; imports `build_assets.py` for its helpers and exports only its own files |
| `PropsV2` | `build_props_v2.py`      | 1.2 house furniture, signs, the curio cabinet and curios, the 1.3 shared kit (waymark, seal stone, cave mouth, climb wall, inn sign), the find hosts (stump, log) and four more NPCs; imports `build_npcs.py` and `build_area_props.py` for their helpers and exports only its own files |
| `V13_<region>` | `build_regions_v13.py` | the 1.3 region props, one module per region (`v13_ember.py` … `v13_stone.py`) on the shared `v13_kit.py`; headless only (see `web/world/ASSETS_V13.md`) |

```bash
# First time only: create the file
blender -b --factory-startup --python-expr "import bpy; s=bpy.context.scene; s.name='World'; \
  [bpy.data.objects.remove(o) for o in list(s.objects)]; bpy.ops.wm.save_as_mainfile(filepath='art/Sixfold.blend')"

# Models -> web/world/assets/*.glb (+ optional contact sheet). Fails loudly on leaky exports.
blender -b art/Sixfold.blend -P tools/blender/build_assets.py -- --out web/world/assets --preview /tmp/assets.png

# World scene (--check lists z-fighting faces, --render writes a game-camera still)
blender -b art/Sixfold.blend -P tools/blender/build_world_scene.py -- --check --render /tmp/world.png

# Area props -> web/world/assets/{pine,birch,reeds,boardwalk,pier,bridge,lighthouse,boat,lantern,stall,stall_b,
#   cottage,workshop,fountain,cliff,cliff_b,stairs,crates,signpost,bench,lens,letter}.glb
# (--only a,b rebuilds some; --preview renders a contact sheet with a cliff terrace and a deck run; --cam x,y,dist moves it)
blender -b --factory-startup -P tools/blender/build_area_props.py -- --out web/world/assets --preview /tmp/areas.png

# Shop interior scene
blender -b art/Sixfold.blend -P tools/blender/build_interior_scene.py -- --render /tmp/interior.png

# 1.3 region props -> web/world/assets/*.glb, per region (ember, storm, tide, dream, grove, stone or all).
# --sheets DIR writes <region>_tiles.png (every prop, front/back 3/4, labelled) and <region>_day.png / _night.png: a small
# map built from the exported files at the game camera, toon-shaded like the game (gradient ramp, hemisphere, fog) in the
# region's light. --check lists coplanar faces; --textures points at baked ground PNGs; --assets-extra borrows .glb folders.
blender -b --factory-startup -P tools/blender/build_regions_v13.py -- --region ember,storm --out web/world/assets \
    --sheets /tmp/v13 --textures /tmp/tex --check

# Seamless painterly ground textures -> {grass,dirt,wood_floor,sand,stone}.png and the 1.3 set
# {ash,heath,wet_sand,dusk_grass,moss,slate,cave}.png (1024 px; --only sand,stone for some).
# Bake into a scratch folder, then `cwebp -q 82 x.png -o web/world/textures/x.webp`: only the .webp files are committed.
blender -b --factory-startup -P tools/blender/bake_ground_textures.py -- --out /tmp/tex --only sand,stone

# NPC variants (+ lineup sheet of hero and NPCs, front and game camera)
blender -b art/Sixfold.blend -P tools/blender/build_npcs.py -- --out web/world/assets
blender -b --factory-startup -P tools/blender/npc_lineup.py -- --assets web/world/assets --out /tmp/lineup \
    [--names hero,rival_grass,rival_fire,rival_water,rival_lightning,rival_fighting,rival_psychic,rival_champion]

# 1.2 / 1.3 props and NPCs -> assets/{interior/,npc/,}*.glb, merges their tags into npc_palettes.json
# (--only a,b; --check lists z-fighting faces on a y = 0 ground; --sheet front/back 3/4 tiles per model;
#  --preview game camera beside the house, cottage, stall and NPCs, cliffs behind the cave mouth and climb wall)
blender -b --factory-startup -P tools/blender/build_props_v2.py -- --out web/world/assets --check \
    --sheet /tmp/pv2_sheet.png --preview /tmp/pv2.png

# Rival bust portraits -> assets/portraits/<rival id>.webp (512 px, needs cwebp; --only bramble,warden)
blender -b --factory-startup -P tools/blender/rival_portraits.py -- --assets web/world/assets --out web/world/assets/portraits

# Hero turnaround sheet (6 views + a strip per clip), always headless
blender -b --factory-startup -P tools/blender/hero_turnaround.py -- --glb web/world/assets/hero.glb --out /tmp/hero.png
```

### In the live Blender

Open `art/Sixfold.blend` yourself, then run a script with
`runpy.run_path("<abs path>", run_name="__main__")`. That rebuilds the script's scene in place.
`build_world_scene.py` also switches the viewport to the game camera and starts playback.

Rules for live sessions:
- Never call `read_homefile`, `open_mainfile` or `revert_mainfile` from a script.
- Do all rendering, image saving, baking and heavy imports or exports headless (`blender -b`).
- Two live crashes so far: a 2M-face export run under a scene override, and an image I/O thread.

The MCP add-on stores its server and generator toggles per scene. If you switch to a scene
created by a script, its panel may show the server as off even though it is still running.

## What is in the files

- **Axes:** models are built Z-up with the front facing −Y. The glTF exporter (+Y up) turns that into Y-up with the front facing +Z.
- **Grid:** origins sit at the footprint centre. Every base sinks 2 cm into the ground, and nothing is coplanar with y = 0.
- **Exports:**
  - Each export is a single scene restricted to that asset (`use_active_scene`).
  - Clips are taken from NLA tracks.
  - `validate_glb()` rejects extra scenes, hero data inside props, and duplicate or suffixed clip names.
- **Hero:** `hero.glb` is about 1.47 m tall and 0.64 m wide. It is one skinned mesh with rigid weights, including the `Eye.L` / `Eye.R` blink bones. Clips:
  - `Idle`: 4 s, with breathing, a weight shift and one blink.
  - `Fidget`: 2.5 s look-around.
  - `Walk`: 0.5 s, two steps of 0.25 s, one tile each.
  - `Run`: 0.25 s, two steps of 0.125 s.
- **Hero frozen:** the hero is frozen. `build_hero()` and `build_hero_mesh()` must not change its output; the export was checked against the frozen copy (identical glTF JSON).
- **NPCs:**
  - They share `rig_character()`: the same 10 bones and the Idle/Walk/Fidget clips.
  - Each one has its own parameters: scale, `head_scale`, `leg` length, static `pose` offsets (the wanderer's hands in pockets, the rival's confident stance) and arm swing.
  - Hats, tools and binders are weighted to the head or hand bones.
- **House:** `house.glb` is a duel hall with a hipped roof, a hex turret, an awning and a hanging hex sign. It contains:
  - a `door` node hinged on −X, with the `Open` clip (0.42 s);
  - an empty `smoke_origin` on top of the chimney;
  - a two-step stoop on the door tile (column 3).
- **Wind:** tree, bush and flowers carry `COLOR_0`. Red is the sway weight (0 at the base, 1 at the tips). G, B and A are 1.
- **Fences:** `fence`, `fence_corner`, `fence_end` and `fence_t` are picked per `f` tile by `build_assets.fence_pick()`, using the same table as ASSETS.md.
- **Paths:** `path`, `path_b` and `path_c` are 1 m dirt tiles. They are being replaced by the splat ground.
- **Ground:** `grass_edge` is a 1 m strip of wind-weighted blades for grass/path borders.
- **Variants:** `tree_b` (tall, tiered), `tree_c` (short, wide, with fruit) and `bush_dark` (dense borders).
- **Contact shadows:** flowers, sign, rock and the bushes carry translucent dark discs, 5–8 mm above the ground.
- **House roof:** a painted scallop-tile base-colour texture (generated with numpy, 512 px, exported as JPEG) with planar UVs per slope.
- **Interior (`assets/interior/`):**
  - walls: `wall`, `wall_corner`, `wall_end`, `wall_t` (2.6 m, 0.25 m thick, arms as for fences), `wall_window`, and the `wall_low*` set at 0.6 m;
  - furniture: `counter`, `shelf`, `duel_table`, `chair`, `rug`, `plant`, `display_case`, `exit_mat`;
  - `lamp`, which has a `light_origin` empty.
- **Item:** `shoes.glb` is the running-shoes pickup, centred on the origin.
- **Area props (`build_area_props.py`):**
  - decks: `boardwalk` (top at 0.10 m), `pier` (top at 0.30 m, posts into the water), `bridge` (2 × 4, deck 0.12 / 0.24 / 0.24 / 0.12 m at the row centres, rails at x = ±0.92); planks reach the tile edges so runs chain;
  - quarry: `cliff` / `cliff_b` (1 × 1 × 1 sandstone block, flat top at exactly 1 m, stackable, good under random 90° turns; `_b` has low moss), `stairs` (treads at 0.125 / 0.375 / 0.625 / 0.875 m, rising toward −Z, a last 12.5 cm riser onto a cliff top);
  - woodland and shore: `pine`, `birch`, `reeds` (wind `COLOR_0`);
  - seaside: `lighthouse` (2 × 2, `light_origin` in the lantern room), `boat` (2 × 1, hull below the water line), `crates`;
  - square: `fountain` (3 × 3 hex basin; its water uses the `sx_water` material for the game to animate), `stall` / `stall_b` (2 × 1), `lantern` (`light_origin`), `bench` (2 × 1), `signpost`;
  - buildings: `cottage` (5 × 4, door on column 2 of the front row, `smoke_origin`) and `workshop` (3 × 3, door on column 1 of the front row), decorative;
  - items: `lens` and `letter`, centred on the origin for the popup.

## Generated models (text-to-3D trials, dropped for now)

The generated house was dropped: it broke the grid and door rules, and the procedural house won.

- `postprocess_generated.py` turns a raw download into a game-ready prop:
  - decimate to the triangle budget;
  - fit to the grid footprint;
  - base colour only, at most 1024 px;
  - optional saturation/value grade;
  - wind weights.
- Raw downloads larger than 5 MB stay outside the repo. The first house trial (Meshy, model "latest", 2.1M faces, 3 × 2k textures, 70 MB) is in the session scratchpad as `generated/house_raw.glb`.
- Prompt used for that trial: "cosy cartoon card-shop cottage, single storey, hipped deep-teal tile roof, hexagonal corner turret with pointed teal spire on the front right, red brick lower walls and cream plaster upper walls with dark timber bands, navy blue front door with a round window, red and cream striped awning over the door, hanging hexagon sign on a bracket, small brick chimney, low-poly stylized, chunky soft bevels, hand-painted texture"
