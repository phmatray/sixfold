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
| `NPCs`    | `build_npcs.py`          | five NPC variants on the hero's rig (shopkeeper, gardener, wanderer, kid, rival) -> `assets/npc/npc_*.glb` + `npc_palettes.json` |
| `Interior`| `build_interior_scene.py`| `web/world/maps/shop.json`: walls auto-tiled with the fence rules (cutaway `wall_low*` on the camera-side south row, `wall_window` on chosen straight tiles), wood floor, furniture, lamp light, hero |

```bash
# First time only: create the file
blender -b --factory-startup --python-expr "import bpy; s=bpy.context.scene; s.name='World'; \
  [bpy.data.objects.remove(o) for o in list(s.objects)]; bpy.ops.wm.save_as_mainfile(filepath='art/Sixfold.blend')"

# Models -> web/world/assets/*.glb (+ optional contact sheet). Fails loudly on leaky exports.
blender -b art/Sixfold.blend -P tools/blender/build_assets.py -- --out web/world/assets --preview /tmp/assets.png

# World scene (--check lists z-fighting faces, --render writes a game-camera still)
blender -b art/Sixfold.blend -P tools/blender/build_world_scene.py -- --check --render /tmp/world.png

# Shop interior scene
blender -b art/Sixfold.blend -P tools/blender/build_interior_scene.py -- --render /tmp/interior.png

# Seamless painterly ground textures -> web/world/textures/{grass,dirt,wood_floor}.png (1024 px)
blender -b --factory-startup -P tools/blender/bake_ground_textures.py -- --out web/world/textures

# NPC variants (+ lineup sheet of hero and NPCs, front and game camera)
blender -b art/Sixfold.blend -P tools/blender/build_npcs.py -- --out web/world/assets
blender -b --factory-startup -P tools/blender/npc_lineup.py -- --assets web/world/assets --out /tmp/lineup

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
