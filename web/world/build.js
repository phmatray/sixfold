// Building a map's scene (`build`): lights, collision and floor heights, ground,
// props (static ones instanced, flat-coloured parts merged), raised ground, fences
// and walls, grass, the tree ring, lamps, glints, gates, `Open` clips and NPCs. Also
// the props' own animation (`animateProps`), `refresh` (re-check what conditions
// show, hide, open or shut) and `dispose`. The light itself, by time of day, is daylight.js.
import * as THREE from "three";
import { mergeGeometries } from "three/addons/utils/BufferGeometryUtils.js";
import { windify, waterify, ambience } from "./life.js";
import { groundMesh } from "./ground.js";
import { rng, mesh, group, toon, PLACEHOLDER, FOOT, WALKABLE, DECK, LADDER, BOB, WIND_H, AUTOTILE, SUFFIX_ARMS, armsPiece, prep, placeholderBox } from "./props.js";
import { key, glowTex, blobGeo, blobMat } from "./actors.js";
import { spawnNpcs } from "./npc.js";
import { OPTIONAL, RING, cliffSet } from "./map.js";

// Instanced: static, never animated, never read like a sign (one draw per mesh of the model, not per prop).
const BATCHED = new Set(["tree", "bush", "flowers", "rock", "pine", "birch", "reeds", "boardwalk", "pier", "bridge", "stairs",
  "lantern", "stall", "stall_b", "crates", "bench",
  "ash_pine", "rock_basalt", "cliff_basalt", "cliff_basalt_b", "stairs_basalt", "climb_wall_basalt", "glass_rack", // Ember
  "heather", "cliff_heath", "cliff_heath_b", "stairs_heath", "wall_dry", "wall_dry_corner", "wall_dry_end", "wall_dry_t", "chime_post", // Storm (not pylon: animated)
  "crab_pots", "rock_sea", "bollard", "tidepool", "net_rack", "cliff_sea", "cliff_sea_b", // Tide (not ferry or buoy: they bob)
  "willow", "wishing_tree", "ford_stones", "wisteria_arch", "hedge", "hedge_corner", "hedge_end", "hedge_t"]); // Dream (not moonflower: animated)
const TREES = new Set(["tree", "pine", "birch", "ash_pine", "willow"]); // varied in size, turn and tint; half a tile out on the map edge
const SMOKY = { kiln: "soot", glass_furnace: "soot", ember_vent: "steam" }; // props whose smoke_origin smokes
const BLOOMS = new Set(["moonflower"]); // with an `open` condition: instanced closed and open, cross-faded (build, refresh)
const SMOKE = 8;
const CLIMB = 0.6; // the most the floor may rise or drop in one step (a cliff edge is more)
const SIDES = [[0, -1], [1, 0], [0, 1], [-1, 0]]; // N E S W, the mask bit order

// Re-check what a condition shows or hides (`holds(cond)`), after a map is built
// and whenever the game state may have changed (a dialogue closed, the time of day
// turned): people, spots and props with an `if` come and go, the glint over each
// find, the waymarks' crystals, props' `open` clips, and gates. `gate(cond, wasOpen,
// underHero)` is the engine's rule (a gate never shuts under the hero); the first
// refresh after a build counts a hero spawning on a gate as having come through it.
// `animate`: a gate that changes now plays its opening (not on a fresh build).
export function refresh(w, holds, gate, animate = false) {
  for (const n of w.npcs) n.hidden = !holds(n.def.if);
  for (const i of w.map.interactions) {
    if (!i.if) continue;
    const k = key(i.x, i.z);
    if (holds(i.if)) w.talk.set(k, i);
    else if (w.talk.get(k) === i) w.talk.delete(k);
  }
  w.glints.forEach((g) => (g.sprite.visible = holds(g.it.if) && holds(g.cond)));
  for (const c of w.shown) c.obj.visible = holds(c.p.if);
  for (const o of w.openers) if (o.cond) o.want = holds(o.cond) ? 1 : 0;
  for (const p of w.pulses) p.on = holds(p.cond);
  for (const m of w.waymarks) {
    const lit = holds(`flag:way_${m.it.id}`);
    if (m.lit === false && lit && animate) m.flash = 1; // just touched: a burst of light
    m.lit = lit;
  }
  const on = new Set([key(w.hero?.x ?? w.spawnAt.x, w.hero?.z ?? w.spawnAt.z)]);
  if (w.hero?.step) on.add(key(w.hero.step.x, w.hero.step.z));
  for (const g of w.gates) {
    const under = g.tiles.some((t) => on.has(key(t.x, t.z)));
    const open = gate(g.def.if ?? "", g.open ?? under, under);
    if (open === g.open) continue;
    const was = g.open;
    g.open = open;
    const play = animate && was !== null;
    if (play && !g.sea) (g.anim = 0), w.life.puffs?.(g.tiles), (w.gateSound = open);
    else g.anim = null;
    showGate(g, reduced(w) ? null : g.anim);
    if (g.sea) {
      // The tide turns over a few seconds (at once on a fresh build or with reduced motion).
      g.tideWant = open ? 0 : 1;
      if (!play || reduced(w)) g.sea((g.tide = g.tideWant));
    }
  }
  for (const g of w.glimmers ?? []) g.on = g.gate ? !!g.gate.open : holds(g.cond);
  for (const b of w.blooms ?? []) {
    const open = holds(b.cond);
    if (open === b.open) continue;
    b.want = open ? 1 : 0;
    if (b.open === null || !animate || reduced(w)) b.k = b.want;
    b.open = open;
    showBloom(b);
  }
  collide(w);
  w.lightDirty = true;
}
const reduced = (w) => w.reduced;

// Collision over the static map: shut gates block their tiles (open ones take
// their own floor), and a prop with an `if` blocks its footprint while it is there.
function collide(w) {
  const { solid, floor, base } = w.grid0;
  for (const i of w.dynamic) (solid[i] = base.solid[i]), (floor[i] = base.floor[i]);
  for (const g of w.gates) for (const t of g.tiles) (solid[t.i] = g.open ? 0 : 1), g.open && (floor[t.i] = t.open);
  for (const c of w.shown) if (c.obj.visible) for (const t of c.tiles) (solid[t.i] = t.deck === null ? 1 : 0), t.deck !== null && (floor[t.i] = t.deck);
}

// Moonflowers opening or closing: the buds sink as the blooms rise, over a few seconds.
function showBloom(b) {
  const e = b.k * b.k * (3 - 2 * b.k);
  b.opened.visible = b.k > 0.001;
  b.closed.visible = b.k < 0.999;
  b.opened.scale.y = Math.max(0.001, e);
  b.closed.scale.y = Math.max(0.001, 1 - e);
}

// A gate's props: `prop` always, `prop_closed` / `prop_open` by state. `t` (0..1)
// plays the change: the going group sinks, the coming one rises.
function showGate(g, t) {
  const k = t === null ? 1 : t;
  const ease = k * k * (3 - 2 * k);
  if (g.closed) (g.closed.visible = !g.open || k < 1), (g.closed.scale.y = g.open ? Math.max(0.001, 1 - ease) : t === null ? 1 : ease);
  if (g.opened) (g.opened.visible = g.open || k < 1), (g.opened.scale.y = g.open ? Math.max(0.001, ease) : t === null ? 1 : 1 - ease);
  if (t === null) {
    if (g.closed) g.closed.scale.y = 1;
    if (g.opened) g.opened.scale.y = 1;
  }
}

// Free GPU memory under `root`; cached model geometry (instanced) is kept.
export function dispose(root) {
  root.parent?.remove(root);
  root.traverse((o) => {
    o.userData.dispose?.();
    if (o.userData.shared) return;
    o.geometry?.dispose();
    [o.material].flat().forEach((m) => m?.dispose());
  });
}

// The house door easing open or shut, a sign wobbling after it was read, boats
// bobbing, the lighthouse beacon and the glint of something to find.
export function animateProps(w, dt, reduced) {
  for (const b of w.blooms ?? []) if (b.k !== b.want) (b.k += Math.sign(b.want - b.k) * Math.min(Math.abs(b.want - b.k), dt / 3)), showBloom(b);
  for (const g of w.gates) {
    if (g.sea && g.tide !== g.tideWant) g.sea((g.tide += Math.sign(g.tideWant - g.tide) * Math.min(Math.abs(g.tideWant - g.tide), dt / 5)));
    if (g.anim === null) continue;
    g.anim = Math.min(1, g.anim + dt / 0.7);
    showGate(g, g.anim);
    if (g.anim >= 1) (g.anim = null), showGate(g, null);
  }
  for (const o of w.openers) {
    if (o.want === undefined) continue;
    o.open = reduced ? o.want : o.open + Math.sign(o.want - o.open) * Math.min(Math.abs(o.want - o.open), dt / o.length);
    o.action.time = o.open * o.length;
    o.mixer.update(0);
  }
  for (const m of w.waymarks) if (m.flash) m.flash = Math.max(0, m.flash - dt * 0.8);
  for (const p of w.pulses) {
    // A crackle every ~3.5 s (steady with reduced motion), brighter in a lightning flash.
    const k = reduced ? 0.6 : Math.max(0, Math.sin(w.now * 1.8)) ** 12 + Math.max(0, Math.sin(w.now * 23)) * 0.15;
    p.sprite.material.opacity = p.on ? Math.min(1, k + (w.flash ?? 0)) : 0;
    p.sprite.scale.setScalar(2.4 + k * 2);
  }
  if (!reduced) {
    for (const b of w.boats) {
      // Paper lanterns ride on lily pads: a gentle bob that never dips them under the water (maps float them at y 0.06).
      b.obj.position.y = (b.y ?? 0) + Math.sin(w.now * 1.4 + b.cx) * (b.asset === "floating_lantern" ? 0.012 : 0.035);
      b.obj.rotation.z = Math.sin(w.now * 1.1 + b.cz) * 0.03;
    }
    for (const g of w.glows) g.material.opacity = 0.55 + 0.3 * Math.sin(w.now * 2.2);
    for (const g of w.glints) g.sprite.material.opacity = 0.5 + 0.5 * Math.sin(w.now * 4 + g.sprite.position.x);
  }
  const d = w.door;
  if (d) {
    d.open = reduced ? d.want : d.open + (d.want - d.open) * Math.min(1, dt * 7);
    if (d.action) {
      d.action.time = d.open * d.action.getClip().duration;
      d.mixer.update(0);
    } else if (d.node) d.node.rotation.y = d.rest + d.open * 1.5;
  }
  for (const it of w.talk.values()) {
    const o = it.obj;
    if (!o?.userData.wobble) continue;
    o.userData.wobble = Math.max(0, o.userData.wobble - dt * 1.4);
    o.rotation.z = Math.sin(w.now * 22) * 0.07 * o.userData.wobble;
  }
}

// One mesh for all the flat-coloured opaque parts of a model: each part's colour
// moves into a vertex colour, so the model costs one draw (and one shadow draw)
// instead of one per material. Textured and see-through parts stay apart. The
// wind weight (COLOR_0) moves to `windw`. Cached on the model for the session.
const SHADOWLESS = new Set(["grass_edge", "boardwalk", "flowers", "reeds"]); // low enough not to need a shadow pass
function flatten(template) {
  if (template.userData.flat) return template.userData.flat;
  template.updateMatrixWorld(true);
  const inv = new THREE.Matrix4().copy(template.matrixWorld).invert(), keep = [], parts = [];
  template.traverse((m) => {
    if (!m.isMesh || !m.visible) return; // a part `recolor` hid
    const rel = new THREE.Matrix4().multiplyMatrices(inv, m.matrixWorld);
    // Textured, see-through, `glow` and `glass` parts (lit at night, by name) stay apart.
    if (m.material.map || m.material.transparent || m.isSkinnedMesh || /glow|glass/i.test(m.material.name)) return keep.push([m, rel]);
    const src = m.geometry, n = src.attributes.position.count, c = m.material.color;
    const g = new THREE.BufferGeometry();
    g.setAttribute("position", src.attributes.position.clone());
    g.setAttribute("normal", src.attributes.normal.clone());
    g.setIndex(src.index ? src.index.clone() : [...Array(n).keys()]);
    g.setAttribute("color", new THREE.Float32BufferAttribute(Array.from({ length: n }, () => [c.r, c.g, c.b]).flat(), 3));
    const w = src.attributes.windw ?? src.attributes.color;
    g.setAttribute("windw", new THREE.Float32BufferAttribute(Array.from({ length: n }, (_, i) => (w ? [w.getX(i), 1, 1, 1] : [-1, 1, 1, 1])).flat(), 4));
    parts.push(g.applyMatrix4(rel));
  });
  const out = new THREE.Group();
  if (parts.length) {
    const merged = new THREE.Mesh(mergeGeometries(parts), toon("#ffffff", { vertexColors: true }));
    merged.userData.flat = true;
    out.add(merged);
  }
  for (const [m, rel] of keep) {
    const k = new THREE.Mesh(m.geometry, m.material);
    rel.decompose(k.position, k.quaternion, k.scale);
    out.add(k);
  }
  return (template.userData.flat = out);
}

// A copy of a model keeping only its upward faces (a cliff block's top, for tiles
// inside a plateau whose sides nobody sees).
function topOnly(template) {
  const out = template.clone(true), a = new THREE.Vector3(), b = new THREE.Vector3(), c = new THREE.Vector3();
  delete out.userData.flat;
  out.traverse((m) => {
    if (!m.isMesh) return;
    const g = m.geometry.clone(), pos = g.attributes.position, idx = g.index ? g.index.array : [...Array(pos.count).keys()], keep = [];
    for (let i = 0; i < idx.length; i += 3) {
      a.fromBufferAttribute(pos, idx[i]), b.fromBufferAttribute(pos, idx[i + 1]), c.fromBufferAttribute(pos, idx[i + 2]);
      const ny = c.clone().sub(b).cross(a.clone().sub(b)).normalize().y; // winding is CCW: (b-a)x(c-a) == (c-b)x(a-b)
      if (Math.abs(ny) > 0.3 && Math.max(a.y, b.y, c.y) > 0.75) keep.push(idx[i], idx[i + 1], idx[i + 2]);
    }
    g.setIndex(keep);
    m.geometry = g;
  });
  return out;
}

// A copy of a model posed at the end of `clip` (the moonflower open), for instancing.
function posed(scene, clip) {
  const flat = scene.userData.flat; // a session cache (flatten): not to be copied into the clone
  delete scene.userData.flat;
  const out = scene.clone(true), mixer = new THREE.AnimationMixer(out), action = mixer.clipAction(clip);
  if (flat) scene.userData.flat = flat;
  action.play();
  action.time = clip.duration;
  mixer.update(0);
  out.updateMatrixWorld(true);
  return out;
}

// One placed model, merged (`flatten`), keeping its named empties (light_origin,
// smoke_origin) for the lamps and the smoke.
function merged(obj) {
  obj.updateMatrixWorld(true);
  const out = new THREE.Group();
  delete obj.userData.flat; // a clone carries a (JSON-copied) cache from its template
  out.add(flatten(obj));
  out.traverse((o) => o.isMesh && ((o.castShadow = !o.material.transparent), (o.receiveShadow = true)));
  const inv = new THREE.Matrix4().copy(obj.matrixWorld).invert();
  obj.traverse((o) => {
    if (o.isMesh || o === obj || !o.name) return;
    const e = new THREE.Object3D();
    e.name = o.name;
    e.applyMatrix4(new THREE.Matrix4().multiplyMatrices(inv, o.matrixWorld)); // returns nothing: add `e` itself
    out.add(e);
  });
  out.position.copy(obj.position);
  out.rotation.copy(obj.rotation);
  return out;
}

// Static props of one kind become InstancedMeshes: one draw per mesh of the
// (flattened) model, not one per prop. `add` records a placement; `build` makes the meshes.
function batcher() {
  const kinds = new Map();
  return {
    add(name, template, matrix, color) {
      if (!kinds.has(name)) kinds.set(name, { template, list: [] });
      kinds.get(name).list.push({ matrix, color });
    },
    build(parent, windH) {
      const white = new THREE.Color("#ffffff"), tmp = new THREE.Matrix4();
      for (const [name, { template, list }] of kinds) {
        prep(template, true);
        const flat = flatten(template);
        flat.updateMatrixWorld(true);
        const tinted = list.some((p) => p.color);
        flat.traverse((m) => {
          if (!m.isMesh) return;
          const inst = new THREE.InstancedMesh(m.geometry, m.material, list.length);
          list.forEach(({ matrix, color }, i) => {
            inst.setMatrixAt(i, tmp.multiplyMatrices(matrix, m.matrixWorld));
            if (tinted) inst.setColorAt(i, color ?? white);
          });
          inst.castShadow = !m.material.transparent && !SHADOWLESS.has(name);
          inst.receiveShadow = true;
          inst.userData.shared = true; // geometry and material belong to the cached model
          if (windH(name)) windify(inst, windH(name));
          parent.add(inst);
        });
      }
    },
  };
}

export async function build(map, got, reduced) {
  const { width: W, height: H } = map;
  const L = map.light;
  const root = new THREE.Group();
  const r = rng(W * 131 + H);

  // Lights. Every light also lights layer 1 (the hero's own passes).
  const hemi = new THREE.HemisphereLight(L.hemisphere.sky, L.hemisphere.ground, L.hemisphere.intensity);
  const sun = new THREE.DirectionalLight(L.sun.color, L.sun.intensity);
  sun.castShadow = true;
  sun.shadow.mapSize.set(2048, 2048);
  const ext = L.indoor ? 9 : 16;
  Object.assign(sun.shadow.camera, { left: -ext, right: ext, top: ext, bottom: -ext, near: 1, far: 60 });
  sun.shadow.camera.layers.enableAll();
  sun.shadow.bias = -0.0005;
  sun.shadow.normalBias = 0.03;
  if (L.indoor) {
    // Window light from a fixed side of the room.
    sun.position.set(W / 2 + L.sun.fromHero[0], L.sun.fromHero[1], H / 2 + L.sun.fromHero[2]);
    sun.target.position.set(W / 2, 0, H / 2);
  }
  root.add(hemi, sun, sun.target);

  // Collision: "#" rock, "f" fence, "W" wall, "V" window and "~" water tiles, plus
  // prop footprints (decks clear them). The floor height: a digit is a raised cliff
  // top that many metres up, "=" a boardwalk, "H" a pier; decks and stairs set
  // their own. A step may climb or drop at most CLIMB.
  const solid = new Uint8Array(W * H);
  const floor = new Float32Array(W * H);
  const tile = (x, z) => map.tiles[z]?.[x];
  const FLOOR = { "=": DECK.boardwalk[0], H: DECK.pier[0] };
  map.tiles.forEach((row, z) =>
    [...row].forEach((c, x) => {
      solid[z * W + x] = "#fWV~".includes(c) ? 1 : 0;
      floor[z * W + x] = c >= "1" && c <= "9" ? +c : FLOOR[c] ?? 0;
    }),
  );
  const inMap = (x, z) => x >= 0 && z >= 0 && x < W && z < H;
  const blocked = (x, z) => !inMap(x, z) || solid[z * W + x] === 1;
  const floorAt = (x, z) => (inMap(x, z) ? floor[z * W + x] : 0);
  // A ladder joins its front and back neighbours at any height (along its own axis).
  const ladders = new Map();
  const climb = (ax, az, bx, bz) => {
    if (Math.abs(floorAt(bx, bz) - floorAt(ax, az)) <= CLIMB) return true;
    const axis = ax !== bx ? "x" : "z";
    return ladders.get(key(ax, az)) === axis || ladders.get(key(bx, bz)) === axis;
  };

  const ground = await groundMesh(map, tile);
  root.add(ground);

  const batch = batcher();
  const blobs = [];
  const BLOB = { tree: 2.2, bush: 1.1, flowers: 0.95, rock: 1.0, sign: 0.6, pine: 1.4, birch: 1.4 };
  const at = (x, z, rot, s = 1, y = 0) =>
    new THREE.Matrix4().compose(new THREE.Vector3(x, y, z), new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(0, 1, 0), rot), new THREE.Vector3(s, s, s));
  const templates = new Map();
  const fallback = (n) => (PLACEHOLDER[n] ?? (() => placeholderBox(n)))();
  const tpl = (n, make) => {
    if (!templates.has(n)) templates.set(n, got[n] ? got[n].scene : make ? make() : fallback(n));
    return templates.get(n);
  };
  const variants = (n) => [n, `${n}_b`, `${n}_c`].filter((v) => v === n || got[v]);
  const tint = (base, spread) => new THREE.Color(base).offsetHSL((r() - 0.5) * spread, 0, (r() - 0.5) * spread);

  // Props sit at their footprint's centre; rot 90/270 swaps the footprint. A prop
  // with an `if` (WORLD_DESIGN §9.1) is only there, and only blocks, while it holds.
  const blooms = new Map();
  const props = [];
  const propAt = new Map();
  const shown = [];
  for (const p of map.props) {
    let [fw, fd] = FOOT[p.asset] ?? [1, 1];
    if (p.rot % 180 === 90) [fw, fd] = [fd, fw];
    const cx = p.x + fw / 2, cz = p.z + fd / 2, y = p.y ?? 0; // `y` stacks a prop (a step on a cliff)
    let obj = null;
    const clip = BLOOMS.has(p.asset) && p.open && !p.if && got[p.asset]?.animations.find((c) => c.name.toLowerCase() === "open");
    if (clip) {
      // A bed of moonflowers costs two draws, not two per flower: closed and open instances, swapped by `open`.
      if (!blooms.has(p.open)) blooms.set(p.open, { cond: p.open, closed: batcher(), opened: batcher() });
      const set = blooms.get(p.open), m = at(cx, cz, THREE.MathUtils.degToRad(p.rot), 1, y);
      set.closed.add(p.asset, tpl(p.asset), m);
      set.opened.add(`${p.asset}#open`, tpl(`${p.asset}#open`, () => posed(got[p.asset].scene, clip)), m);
    } else if (BATCHED.has(p.asset) && !p.if && !p.recolor) {
      // Trees vary in model, size, tint and turn; the ones on the map edge sit half a tile out.
      const isTree = TREES.has(p.asset);
      const kinds = variants(p.asset);
      const name = p.asset === "tree" ? kinds[Math.floor(r() * kinds.length)] : p.asset;
      const ox = isTree ? (p.x === 0 ? -0.5 : p.x + fw === W ? 0.5 : 0) : 0;
      const oz = isTree ? (p.z === 0 ? -0.5 : p.z + fd === H ? 0.5 : 0) : 0;
      const s = isTree ? 0.88 + r() * 0.3 : 1;
      const rot = isTree ? r() * Math.PI * 2 : THREE.MathUtils.degToRad(p.rot);
      batch.add(name, tpl(name), at(cx + ox, cz + oz, rot, s, y), isTree ? tint("#ffffff", 0.06) : null);
      if (!got[name] && BLOB[p.asset]) blobs.push([cx + ox, cz + oz, BLOB[p.asset] * s]); // models bake their own shadow disc
    } else {
      obj = got[p.asset] ? got[p.asset].scene.clone(true) : fallback(p.asset);
      obj.position.set(cx, y, cz);
      obj.rotation.y = THREE.MathUtils.degToRad(p.rot);
      prep(obj, true);
      obj.traverse((o) => o.isMesh && /water/i.test(o.material.name) && waterify(o.material)); // the fountain's pool
      // A variant of a shared model: `recolor` maps a material name to a colour,
      // or to null to hide that part (the clinic is the bakery's cottage, re-dressed).
      if (p.recolor) obj.traverse((o) => {
        if (!o.isMesh || !(o.material.name in p.recolor)) return;
        const c = p.recolor[o.material.name];
        if (c === null) o.visible = false;
        else (o.material = o.material.clone()), o.material.color.set(c);
      });
      if (p.glow) obj.traverse((o) => o.isMesh && /glow/i.test(o.material.name) && ((o.material = o.material.clone()), o.material.color.set(p.glow), o.material.emissive.set(p.glow).multiplyScalar(0.55))); // a waymark's region colour
      // A static model (no clip, not afloat) is merged like the instanced ones: one
      // draw for its flat-coloured parts instead of one per material.
      if (got[p.asset] && !got[p.asset].animations.length && !BOB.has(p.asset)) obj = merged(obj);
      root.add(obj);
      if (BLOB[p.asset] && !got[p.asset]) blobs.push([cx, cz, BLOB[p.asset]]);
    }
    const rec = { ...p, obj, cx, cz, y };
    props.push(rec);
    for (const i of map.interactions) if (i.x >= p.x && i.x < p.x + fw && i.z >= p.z && i.z < p.z + fd) i.obj = obj;
    const deck = DECK[p.asset];
    const cond = p.if ? { p, obj, tiles: [] } : null;
    if (cond) shown.push(cond);
    for (let x = p.x; x < p.x + fw; x++)
      for (let z = p.z; z < p.z + fd; z++) {
        if (!cond) propAt.set(key(x, z), rec);
        if (LADDER.has(p.asset)) ladders.set(key(x, z), p.rot % 180 === 90 ? "x" : "z");
        // The deck's rows run from its front (+Z at rot 0) to its back.
        const row = { 0: p.z + fd - 1 - z, 90: p.x + fw - 1 - x, 180: z - p.z, 270: x - p.x }[p.rot % 360] ?? 0;
        const top = deck ? y + deck[Math.min(row, deck.length - 1)] : null;
        if (cond) {
          if (deck || !WALKABLE.has(p.asset)) cond.tiles.push({ i: z * W + x, deck: top });
        } else if (deck) {
          solid[z * W + x] = 0;
          floor[z * W + x] = top;
        } else if (!WALKABLE.has(p.asset)) solid[z * W + x] = 1;
      }
  }
  const bloomSets = [...blooms.values()].map((b) => {
    const closed = new THREE.Group(), opened = new THREE.Group();
    b.closed.build(closed, () => 0);
    b.opened.build(opened, () => 0);
    root.add(closed, opened);
    return { cond: b.cond, closed, opened, open: null, k: 0, want: 0 };
  });
  // Props with an `Open` clip (a pylon's lever, moonflowers at dusk) play it while
  // their `open` condition holds; the house door opens as the hero walks up (below).
  const openers = [];
  for (const p of props) {
    const clip = p.obj && p.asset !== "house" && got[p.asset]?.animations.find((a) => a.name.toLowerCase() === "open");
    if (!clip) continue;
    const mixer = new THREE.AnimationMixer(p.obj), action = mixer.clipAction(clip);
    action.play();
    action.paused = true;
    openers.push({ mixer, action, length: clip.duration || 1, open: 0, want: p.open ? 0 : undefined, cond: p.open ?? null });
  }

  // Raised ground ("1".."9": stacked cliff blocks, a mossy top now and then; the map's
  // own `cliff` set when it has one) and deck tiles.
  const cliffs = cliffSet(map);
  const rise = (x, z) => (inMap(x, z) && tile(x, z) >= "1" && tile(x, z) <= "9" ? +tile(x, z) : 0); // off the map counts as open ground
  for (let z = 0; z < H; z++)
    for (let x = 0; x < W; x++) {
      const c = tile(x, z), turn = () => (Math.floor(r() * 4) * Math.PI) / 2;
      if (c >= "1" && c <= "9")
        for (let k = 0; k < +c; k++) {
          // A block whose four neighbours all stand higher is hidden: only the top
          // layer of a plateau and the faces of its edges are drawn (a 4 m crater
          // rim would otherwise be four blocks per tile, most of them never seen).
          if (k < +c - 1 && SIDES.every(([dx, dz]) => rise(x + dx, z + dz) > k)) continue;
          const n = k === +c - 1 && got[cliffs[1]] && r() < 0.35 ? cliffs[1] : cliffs[0];
          // Inside a plateau only the top shows: the block without its sides.
          const inner = k === +c - 1 && got[n] && SIDES.every(([dx, dz]) => rise(x + dx, z + dz) >= +c);
          batch.add(inner ? `${n}#top` : n, inner ? tpl(`${n}#top`, () => topOnly(tpl(n))) : tpl(n), at(x + 0.5, z + 0.5, turn(), 1, k));
        }
      else if (c === "=" || c === "H") batch.add(c === "=" ? "boardwalk" : "pier", tpl(c === "=" ? "boardwalk" : "pier"), at(x + 0.5, z + 0.5, 0));
    }

  // Fences ("f": the map's `fence` set, "fence", "wall_dry" or "hedge") and walls ("W",
  // or "V" for a wall with a window) auto-tile from their neighbours; walls on the
  // south (camera-facing) row use the low cutaway pieces.
  const joins = (c, x, z) => (c === "f" ? tile(x, z) === "f" : "WV".includes(tile(x, z) ?? "-"));
  const fence = map.fence ?? "fence";
  const fenceTint = map.fence_tint ? new THREE.Color(map.fence_tint) : null; // a map's own shade of its fences (the Moonwell's dusky hedges)
  for (let z = 0; z < H; z++)
    for (let x = 0; x < W; x++) {
      const c = tile(x, z);
      if (!"fWV".includes(c ?? "-")) continue;
      const kind = c === "f" ? fence : z === H - 1 ? "wall_low" : "wall";
      const mask = SIDES.reduce((m, [dx, dz], i) => m | (joins(c, x + dx, z + dz) ? 1 << i : 0), 0);
      for (const [sfx, deg] of [AUTOTILE[mask], ...(mask === 15 ? [["_end", 270]] : [])]) {
        const n = c === "V" && sfx === "" ? "wall_window" : kind + sfx; // a window only fits a straight run
        const stand = n === "wall_window" ? n : c === "f" ? "fence" : kind; // placeholder pieces
        batch.add(n, tpl(n, () => armsPiece(stand, SUFFIX_ARMS[sfx])), at(x + 0.5, z + 0.5, THREE.MathUtils.degToRad(deg)), c === "f" ? fenceTint : null);
      }
    }

  // Gates (WORLD_DESIGN §7.1): tiles walkable only while a condition holds, whatever
  // they are (water, a cliff, a ledge). Each tile is [x, z] or [x, z, y]; `y` stacks
  // its props (a climb wall on a 1 m ledge: by default the tile's own height). Open,
  // a tile's floor is the gate's `floor`, else its deck prop's top, else the ground's.
  // Props: `prop` always, `prop_closed` / `prop_open` by state (afloat ones bob).
  const boats = [];
  const gates = (map.gates ?? []).map((def) => {
    const deck = DECK[def.prop_open] ?? DECK[def.prop];
    const tiles = def.tiles.map(([x, z, y]) => {
      const c = tile(x, z);
      y ??= c >= "1" && c <= "9" ? +c : 0;
      return { x, z, y, i: z * W + x, open: def.floor ?? (deck ? y + deck[0] : floor[z * W + x]) };
    });
    const rot = THREE.MathUtils.degToRad(def.rot ?? 0);
    const props = (name) => {
      if (!name) return null;
      const g = new THREE.Group(), gb = batcher();
      for (const t of tiles) {
        if (name === "lantern_raft") gb.add(name, tpl(name), at(t.x + 0.5, t.z + 0.5, rot + (r() - 0.5) * 0.5, 0.9, t.y)); // separate rafts, not a plank bridge; instanced
        else if (BOB.has(name)) {
          const obj = got[name] ? got[name].scene.clone(true) : fallback(name);
          obj.position.set(t.x + 0.5, t.y, t.z + 0.5);
          obj.rotation.y = rot;
          prep(obj, true);
          g.add(obj);
          boats.push({ obj, cx: t.x + 0.5, cz: t.z + 0.5, y: t.y });
        } else gb.add(name, tpl(name), at(t.x + 0.5, t.z + 0.5, rot, 1, t.y));
      }
      gb.build(g, (n) => WIND_H[n] ?? 0);
      root.add(g);
      return g;
    };
    props(def.prop);
    // A tide gate (`tide: true`): the sea covers its tiles, and its `flood` tiles, while it is shut.
    const flood = def.tide ? [...tiles.map((t) => t.i), ...(def.flood ?? []).map(([x, z]) => z * W + x)] : null;
    const sea = flood && ((level) => ground.userData.water(flood, level));
    return { def, tiles, open: null, anim: null, closed: props(def.prop_closed), opened: props(def.prop_open), sea, tide: 0, tideWant: 0 };
  });
  const gateAt = new Map(gates.flatMap((g) => g.tiles.map((t) => [key(t.x, t.z), g])));

  // Open grass: plain base tiles, not under a prop, a deck or a cliff.
  const wet = (x, z) => SIDES.some(([dx, dz]) => "~H".includes(tile(x + dx, z + dz) ?? "-"));
  const open = (x, z) => !blocked(x, z) && tile(x, z) === "." && floor[z * W + x] === 0 && !wet(x, z);
  const exits = map.exits ?? [];
  const tufts = map.ground.tufts ?? 1800; // 0 on sand
  if (!L.indoor) {
    if (tufts) scatter(root, W, H, open, tufts);
    // Grass tufts along the path edges.
    for (let z = 0; z < H; z++)
      for (let x = 0; x < W; x++) {
        if (!tufts || !open(x, z)) continue;
        for (const [dx, dz] of SIDES) {
          if (!"pr".includes(tile(x + dx, z + dz) ?? "-")) continue;
          for (let k = 0; k < 2; k++) {
            const along = (r() - 0.5) * 0.8;
            const ex = x + 0.5 + dx * 0.4 + (dz ? along : 0), ez = z + 0.5 + dz * 0.4 + (dx ? along : 0);
            batch.add("grass_edge", tpl("grass_edge", PLACEHOLDER.grass_edge), at(ex, ez, r() * Math.PI * 2, 0.8 + r() * 0.4), tint("#ffffff", 0.08));
          }
        }
      }
    // A denser, darker band of trees beyond the map edge hides the empty plain,
    // on the sides the map asks for (not over the sea), with a gap at each exit.
    const outer = (map.ring ?? RING).filter((n) => got[n] || !OPTIONAL.test(n));
    const sides = map.ringSides ?? "nesw";
    const gap = (side, c) => exits.some((e) => e.side === side && c > e.span[0] - 2.2 && c < e.span[1] + 3.2);
    const ring = (side, x, z) => {
      if (!sides.includes(side) || gap(side, "ns".includes(side) ? x : z)) return;
      const n = outer[Math.floor(r() * outer.length)];
      batch.add(n, tpl(n), at(x, z, r() * Math.PI * 2, 1.05 + r() * 0.45), tint("#b9c9a8", 0.05));
    };
    for (let x = -5; x <= W + 5; x += 1.6 + r() * 0.6) ring("n", x, -1.6 - r() * 2.2), ring("s", x, H + 1.4 + r() * 2.2);
    for (let z = -2; z <= H + 2; z += 1.6 + r() * 0.6) ring("w", -1.6 - r() * 2.2, z), ring("e", W + 1.6 + r() * 2.2, z);
  }
  batch.build(root, (n) => (n === "grass_edge" ? 0.35 : WIND_H[n.replace(OPTIONAL, "")] ?? 0));

  // Contact shadows under the scenery.
  if (blobs.length) {
    const inst = new THREE.InstancedMesh(blobGeo, blobMat(), blobs.length);
    blobs.forEach(([x, z, s], i) => inst.setMatrixAt(i, at(x, z, 0, s).setPosition(x, 0.015, z)));
    inst.userData.shared = true; // blobGeo is module-wide
    root.add(inst);
  }

  // Indoors: a warm light, and a glow, on every lamp.
  if (L.indoor && L.lamp) {
    for (const p of props.filter((q) => q.asset === "lamp")) {
      const light = new THREE.PointLight(L.lamp.color, L.lamp.intensity, L.lamp.distance, 1.6);
      p.obj.updateMatrixWorld(true);
      const origin = p.obj.getObjectByName("light_origin");
      if (origin) origin.getWorldPosition(light.position);
      else light.position.set(p.cx, 1.55, p.cz);
      const glow = new THREE.Sprite(new THREE.SpriteMaterial({ map: glowTex, color: L.lamp.color, transparent: true, opacity: 0.7, blending: THREE.AdditiveBlending, depthWrite: false }));
      glow.position.copy(light.position);
      glow.scale.setScalar(1.3);
      root.add(light, glow);
    }
  }
  root.traverse((o) => o.isLight && o.layers.enableAll());

  // The static collision, kept so gates and conditional props can be laid over it.
  const dynamic = [...new Set([...gates.flatMap((g) => g.tiles.map((t) => t.i)), ...shown.flatMap((c) => c.tiles.map((t) => t.i))])];
  const grid0 = { solid, floor, base: { solid: solid.slice(), floor: floor.slice() } };

  // Where light comes from at night (daylight.js): every `light_origin` in the scene,
  // the instanced lanterns' too. Low ones may get a real light; high ones only glow.
  const lamps = [];
  const origin = new THREE.Vector3();
  for (const p of props) {
    if (p.asset === "lighthouse") continue; // its beacon has its own glow
    if (p.obj) {
      p.obj.updateMatrixWorld(true);
      p.obj.traverse((o) => o.name === "light_origin" && lamps.push(o.getWorldPosition(new THREE.Vector3())));
    } else if (got[p.asset]) {
      const t = got[p.asset].scene;
      t.updateMatrixWorld(true);
      const o = t.getObjectByName("light_origin");
      if (o) lamps.push(o.getWorldPosition(origin).clone().add(new THREE.Vector3(p.cx, p.y, p.cz)));
    }
  }
  // Glimmers (daylight.js): small glows that are not lamps, a halo and a soft pool of light under each,
  // at a model's `glow` parts: lanterns afloat, the lantern rafts while their gate is open, moonflowers
  // while open. `on` is kept by refresh(); moths visit them like lamps.
  const glowAt = new Map();
  const glowSpot = (name) => {
    if (!glowAt.has(name)) {
      const box = new THREE.Box3(), at = [];
      got[name]?.scene.updateMatrixWorld(true);
      got[name]?.scene.traverse((o) => o.isMesh && /glow/i.test(o.material.name) && at.push(box.setFromObject(o).getCenter(new THREE.Vector3())));
      glowAt.set(name, at.length ? at.reduce((a, b) => a.add(b), new THREE.Vector3()).divideScalar(at.length) : null);
    }
    return glowAt.get(name);
  };
  const GLIMMER = { floating_lantern: ["#ffc98a", 1.3, 2.6], lantern_raft: ["#ffbcc0", 0.8, 1.1], moonflower: ["#cfdcff", 0.8, 2.2] };
  const glimmers = [];
  const spin = (v, deg) => v.clone().applyAxisAngle(new THREE.Vector3(0, 1, 0), THREE.MathUtils.degToRad(deg));
  for (const p of props) {
    const look = GLIMMER[p.asset], o = look && glowSpot(p.asset);
    if (o) glimmers.push({ at: spin(o, p.rot).add(new THREE.Vector3(p.cx, p.y, p.cz)), color: look[0], size: look[1], pool: look[2], cond: p.open ?? p.if ?? null, on: true });
  }
  for (const g of gates) {
    const look = GLIMMER[g.def.prop_open], o = look && glowSpot(g.def.prop_open);
    if (o) for (const t of g.tiles) glimmers.push({ at: o.clone().add(new THREE.Vector3(t.x + 0.5, t.y, t.z + 0.5)), color: look[0], size: look[1], pool: look[2], gate: g, on: false });
  }
  // Waymarks: the crystal is dim until touched (`flag:way_<id>`), then lit.
  const waymarks = map.interactions.filter((i) => i.kind === "waymark" && i.obj).map((it) => {
    const mats = [];
    it.obj.traverse((o) => o.isMesh && /glow/i.test(o.material.name) && mats.push(o.material));
    return { it, mats, lit: null, flash: 0 };
  });

  const talk = new Map(map.interactions.map((i) => [key(i.x, i.z), i]));
  const warps = new Map((map.warps ?? []).map((wp) => [key(wp.x, wp.z), wp]));
  // Edge exits: walking off side `side` at a tile within `span` leads to map `to`.
  const exitAt = (x, z) => {
    if (inMap(x, z)) return null;
    const side = z < 0 ? "n" : z >= H ? "s" : x < 0 ? "w" : "e", c = "ns".includes(side) ? x : z;
    return exits.find((e) => e.side === side && c >= e.span[0] && c <= e.span[1]) ?? null;
  };
  // Floor levels, highest first (taps on raised ground), and a glint over each find (`glint`: shown while it holds).
  const levels = [...new Set([...floor].map(Math.round))].sort((a, b) => b - a);
  const glints = map.interactions.filter((i) => i.glint).map((i) => {
    const sprite = new THREE.Sprite(new THREE.SpriteMaterial({ map: glowTex, color: "#fff6c8", transparent: true, blending: THREE.AdditiveBlending, depthWrite: false }));
    sprite.position.set(i.x + 0.5, floorAt(i.x, i.z) + 0.8, i.z + 0.5);
    sprite.scale.setScalar(0.55);
    root.add(sprite);
    return { sprite, cond: i.glint, it: i };
  });
  // Prop `pulse` (a condition): an arc of light at its `light_origin` every few seconds
  // while it holds (the Rodspire's tip once its pylons are on).
  const pulses = props.filter((p) => p.pulse && p.obj).map((p) => {
    const s = new THREE.Sprite(new THREE.SpriteMaterial({ map: glowTex, color: "#9feaff", transparent: true, opacity: 0, blending: THREE.AdditiveBlending, depthWrite: false, fog: false }));
    p.obj.updateMatrixWorld(true);
    const o = p.obj.getObjectByName("light_origin");
    if (o) o.getWorldPosition(s.position);
    else s.position.set(p.cx, p.y + 3, p.cz);
    s.scale.setScalar(3.2);
    root.add(s);
    return { sprite: s, cond: p.pulse, on: false };
  });
  // Boats, buoys and lanterns afloat bob; a lighthouse beacon glows at its `light_origin`.
  boats.push(...props.filter((p) => BOB.has(p.asset) && p.obj));
  root.updateMatrixWorld(true);
  const glows = props
    .filter((p) => p.asset === "lighthouse" && p.obj)
    .map((p) => {
      const g = new THREE.Sprite(new THREE.SpriteMaterial({ map: glowTex, color: "#ffe2a0", transparent: true, blending: THREE.AdditiveBlending, depthWrite: false, fog: false }));
      const o = p.obj.getObjectByName("light_origin");
      if (o) o.getWorldPosition(g.position);
      else g.position.set(p.cx, 6.2, p.cz);
      g.scale.setScalar(2.4);
      root.add(g);
      return g;
    });

  const npcs = spawnNpcs(map, got, floorAt, root);
  const npcAt = (x, z, except) => npcs.find((n) => n !== except && !n.hidden && ((n.x === x && n.z === z) || (n.step && n.step.x === x && n.step.z === z))) ?? null;

  const grid = debugGrid(W, H, blocked, talk, warps, npcs);
  grid.visible = false;
  root.add(grid);

  // The house door: its "Open" clip when the model has one, else a node named "door"
  // swung on its origin (the hinge). `open` eases toward `want`, 0 shut to 1 open.
  const house = props.find((p) => p.asset === "house")?.obj;
  let door = null;
  if (house) {
    const clip = got.house?.animations.find((a) => a.name.toLowerCase() === "open");
    door = { open: 0, want: 0, node: house.getObjectByName("door") ?? null, mixer: null, action: null };
    if (clip) {
      door.mixer = new THREE.AnimationMixer(house);
      door.action = door.mixer.clipAction(clip);
      door.action.play();
      door.action.paused = true;
    }
    if (door.node) door.rest = door.node.rotation.y;
  }

  root.updateMatrixWorld(true);
  // Smoke: the first chimney; every kiln and furnace (soot); vents (steam), a gate's
  // only while it is shut. At most SMOKE sources, so the sprites stay cheap.
  const smoke = [];
  const chimney = house ?? props.find((p) => p.asset === "cottage")?.obj;
  const top = chimney?.getObjectByName("smoke_origin");
  if (top) smoke.push({ at: top.getWorldPosition(new THREE.Vector3()), kind: "chimney" });
  const originOf = (name) => {
    const o = got[name]?.scene;
    if (!o) return null;
    o.updateMatrixWorld(true);
    return o.getObjectByName("smoke_origin")?.getWorldPosition(new THREE.Vector3()) ?? null;
  };
  for (const p of props) {
    if (!SMOKY[p.asset]) continue;
    const o = originOf(p.asset);
    if (o) smoke.push({ at: o.applyAxisAngle(new THREE.Vector3(0, 1, 0), THREE.MathUtils.degToRad(p.rot)).add(new THREE.Vector3(p.cx, p.y, p.cz)), kind: SMOKY[p.asset] });
  }
  for (const g of gates) {
    const o = SMOKY[g.def.prop_closed] && originOf(g.def.prop_closed);
    if (o) for (const t of g.tiles) smoke.push({ at: o.clone().add(new THREE.Vector3(t.x + 0.5, t.y, t.z + 0.5)), kind: SMOKY[g.def.prop_closed], on: () => !g.open });
  }
  smoke.length = Math.min(smoke.length, SMOKE);
  const flowers = props.filter((p) => p.asset === "flowers").filter((_, i) => i % 3 === 0).map((p) => ({ x: p.cx, z: p.cz }));
  const life = ambience(root, { flowers, smoke, lamps: [...lamps, ...glimmers.map((g) => g.at)], gulls: map.gulls ?? 0 }, reduced);
  return { map, root, blocked, floorAt, climb, levels, exitAt, npcAt, propAt, talk, warps, npcs, grid, sun, hemi, door, life, boats, glows, glints,
    gates, gateAt, shown, openers, waymarks, lamps, glimmers, blooms: bloomSets, pulses, dynamic, grid0, reduced, spawnAt: map.spawn, path: [], now: 0, nearKey: null };
}

// G toggles this: tile lines; red = blocked, gold = look/use, blue = door, green = person.
function debugGrid(W, H, blocked, talk, warps, npcs) {
  const pts = [];
  for (let x = 0; x <= W; x++) pts.push(x, 0.04, 0, x, 0.04, H);
  for (let z = 0; z <= H; z++) pts.push(0, 0.04, z, W, 0.04, z);
  const geo = new THREE.BufferGeometry();
  geo.setAttribute("position", new THREE.Float32BufferAttribute(pts, 3));
  const g = group(new THREE.LineSegments(geo, new THREE.LineBasicMaterial({ color: "#ffffff", transparent: true, opacity: 0.7 })));
  const quad = new THREE.PlaneGeometry(0.86, 0.86).rotateX(-Math.PI / 2);
  const flat = { transparent: true, depthWrite: false, polygonOffset: true, polygonOffsetFactor: -4, polygonOffsetUnits: -4 };
  const mat = (color, opacity) => new THREE.MeshBasicMaterial({ color, opacity, ...flat });
  const red = mat("#ff2d55", 0.6), gold = mat("#ffd60a", 0.75), blue = mat("#3fa9ff", 0.75), green = mat("#34e07a", 0.75);
  for (let x = 0; x < W; x++)
    for (let z = 0; z < H; z++) {
      const k = key(x, z);
      const m = warps.has(k) ? blue : talk.has(k) ? gold : npcs.some((n) => n.def.x === x && n.def.z === z) ? green : blocked(x, z) ? red : null;
      if (m) g.add(mesh(quad, m, x + 0.5, 0.05, z + 0.5));
    }
  return g;
}

// Grass tufts and tiny flowers, instanced, only on open grass tiles.
function scatter(root, W, H, open, count) {
  const r = rng(99);
  const blade = new THREE.ConeGeometry(0.05, 0.32, 3);
  blade.translate(0, 0.16, 0);
  const tufts = new THREE.InstancedMesh(blade, new THREE.MeshLambertMaterial(), count);
  const bloom = new THREE.InstancedMesh(new THREE.SphereGeometry(0.06, 6, 4), new THREE.MeshLambertMaterial(), Math.ceil(count / 9));
  const m = new THREE.Matrix4(), q = new THREE.Quaternion(), e = new THREE.Euler(), s = new THREE.Vector3(), v = new THREE.Vector3(), col = new THREE.Color();
  const free = [];
  for (let z = 0; z < H; z++) for (let x = 0; x < W; x++) if (open(x, z)) free.push([x, z]);
  if (!free.length) return;
  const spot = () => {
    const [x, z] = free[Math.floor(r() * free.length)];
    return [x + r(), z + r()];
  };
  for (let i = 0; i < tufts.count; ) {
    const [cx, cz] = spot();
    for (let k = 0; k < 8 && i < tufts.count; k++, i++) {
      e.set((r() - 0.5) * 0.6, r() * 6, (r() - 0.5) * 0.6);
      s.setScalar(0.7 + r() * 0.8);
      m.compose(v.set(cx + (r() - 0.5) * 0.35, 0, cz + (r() - 0.5) * 0.35), q.setFromEuler(e), s);
      tufts.setMatrixAt(i, m);
      tufts.setColorAt(i, col.set(r() < 0.5 ? "#3c9a2f" : "#6cc44a"));
    }
  }
  const petals = ["#ffffff", "#ffd2b8", "#fff1a0"];
  for (let j = 0; j < bloom.count; j++) {
    const [x, z] = spot();
    m.compose(v.set(x, 0.07, z), q.identity(), s.set(1, 0.6, 1));
    bloom.setMatrixAt(j, m);
    bloom.setColorAt(j, col.set(petals[Math.floor(r() * 3)]));
  }
  tufts.receiveShadow = bloom.receiveShadow = true;
  windify(tufts, 0.32);
  windify(bloom, 0.12);
  root.add(tufts, bloom);
}
