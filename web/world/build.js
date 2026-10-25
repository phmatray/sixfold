// Building a map's scene (`build`): lights, collision and floor heights, ground,
// props (static ones instanced, flat-coloured parts merged), raised ground, fences
// and walls, grass, the tree ring, lamps, glints, the house door and NPCs. Also
// the props' own animation (`animateProps`), `refresh` (re-check what conditions
// show or hide) and `dispose`.
import * as THREE from "three";
import { mergeGeometries } from "three/addons/utils/BufferGeometryUtils.js";
import { windify, waterify, ambience } from "./life.js";
import { groundMesh } from "./ground.js";
import { rng, mesh, group, toon, PLACEHOLDER, FOOT, WALKABLE, DECK, WIND_H, AUTOTILE, SUFFIX_ARMS, armsPiece, prep, placeholderBox } from "./props.js";
import { key, glowTex, blobGeo, blobMat } from "./actors.js";
import { spawnNpcs } from "./npc.js";
import { OPTIONAL, RING } from "./map.js";

// Instanced: static, never animated, never read like a sign (one draw per mesh of the model, not per prop).
const BATCHED = new Set(["tree", "bush", "flowers", "rock", "pine", "birch", "reeds", "boardwalk", "pier", "bridge", "stairs",
  "lantern", "stall", "stall_b", "crates", "bench"]);
const TREES = new Set(["tree", "pine", "birch"]); // varied in size, turn and tint; half a tile out on the map edge
const CLIMB = 0.6; // the most the floor may rise or drop in one step (a cliff edge is more)
const SIDES = [[0, -1], [1, 0], [0, 1], [-1, 0]]; // N E S W, the mask bit order

// Re-check what a condition shows or hides (`holds(cond)`), after a map is built
// and whenever the game state may have changed (a dialogue closed). Today: the
// glint over each find.
export function refresh(w, holds) {
  w.glints.forEach((g) => (g.sprite.visible = holds(g.cond)));
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
  if (!reduced) {
    for (const b of w.boats) {
      b.obj.position.y = Math.sin(w.now * 1.4 + b.cx) * 0.035;
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
    if (!m.isMesh) return;
    const rel = new THREE.Matrix4().multiplyMatrices(inv, m.matrixWorld);
    if (m.material.map || m.material.transparent || m.isSkinnedMesh) return keep.push([m, rel]);
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
  const climb = (ax, az, bx, bz) => Math.abs(floorAt(bx, bz) - floorAt(ax, az)) <= CLIMB;

  root.add(await groundMesh(map, tile));

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

  // Props sit at their footprint's centre; rot 90/270 swaps the footprint.
  const props = [];
  const propAt = new Map();
  for (const p of map.props) {
    let [fw, fd] = FOOT[p.asset] ?? [1, 1];
    if (p.rot % 180 === 90) [fw, fd] = [fd, fw];
    const cx = p.x + fw / 2, cz = p.z + fd / 2, y = p.y ?? 0; // `y` stacks a prop (a step on a cliff)
    let obj = null;
    if (BATCHED.has(p.asset)) {
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
      root.add(obj);
      if (BLOB[p.asset] && !got[p.asset]) blobs.push([cx, cz, BLOB[p.asset]]);
    }
    const rec = { ...p, obj, cx, cz };
    props.push(rec);
    for (const i of map.interactions) if (i.x >= p.x && i.x < p.x + fw && i.z >= p.z && i.z < p.z + fd) i.obj = obj;
    const deck = DECK[p.asset];
    for (let x = p.x; x < p.x + fw; x++)
      for (let z = p.z; z < p.z + fd; z++) {
        propAt.set(key(x, z), rec);
        if (deck) {
          // The deck's rows run from its front (+Z at rot 0) to its back.
          const row = { 0: p.z + fd - 1 - z, 90: p.x + fw - 1 - x, 180: z - p.z, 270: x - p.x }[p.rot % 360] ?? 0;
          solid[z * W + x] = 0;
          floor[z * W + x] = y + deck[Math.min(row, deck.length - 1)];
        } else if (!WALKABLE.has(p.asset)) solid[z * W + x] = 1;
      }
  }

  // Raised ground ("1".."9": stacked cliff blocks, a mossy top now and then) and deck tiles.
  for (let z = 0; z < H; z++)
    for (let x = 0; x < W; x++) {
      const c = tile(x, z), turn = () => (Math.floor(r() * 4) * Math.PI) / 2;
      if (c >= "1" && c <= "9")
        for (let k = 0; k < +c; k++) {
          const n = k === +c - 1 && got.cliff_b && r() < 0.35 ? "cliff_b" : "cliff";
          batch.add(n, tpl(n), at(x + 0.5, z + 0.5, turn(), 1, k));
        }
      else if (c === "=" || c === "H") batch.add(c === "=" ? "boardwalk" : "pier", tpl(c === "=" ? "boardwalk" : "pier"), at(x + 0.5, z + 0.5, 0));
    }

  // Fences ("f") and walls ("W", or "V" for a wall with a window) auto-tile from
  // their neighbours; walls on the south (camera-facing) row use the low cutaway pieces.
  const joins = (c, x, z) => (c === "f" ? tile(x, z) === "f" : "WV".includes(tile(x, z) ?? "-"));
  for (let z = 0; z < H; z++)
    for (let x = 0; x < W; x++) {
      const c = tile(x, z);
      if (!"fWV".includes(c ?? "-")) continue;
      const kind = c === "f" ? "fence" : z === H - 1 ? "wall_low" : "wall";
      const mask = SIDES.reduce((m, [dx, dz], i) => m | (joins(c, x + dx, z + dz) ? 1 << i : 0), 0);
      for (const [sfx, deg] of [AUTOTILE[mask], ...(mask === 15 ? [["_end", 270]] : [])]) {
        const n = c === "V" && sfx === "" ? "wall_window" : kind + sfx; // a window only fits a straight run
        batch.add(n, tpl(n, () => armsPiece(n === "wall_window" ? n : kind, SUFFIX_ARMS[sfx])), at(x + 0.5, z + 0.5, THREE.MathUtils.degToRad(deg)));
      }
    }

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
    return { sprite, cond: i.glint };
  });
  // Boats bob; a lighthouse beacon glows at its `light_origin`.
  const boats = props.filter((p) => p.asset === "boat" && p.obj);
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
  const npcAt = (x, z, except) => npcs.find((n) => n !== except && ((n.x === x && n.z === z) || (n.step && n.step.x === x && n.step.z === z))) ?? null;

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
  const chimney = house ?? props.find((p) => p.asset === "cottage")?.obj;
  const smoke = chimney?.getObjectByName("smoke_origin")?.getWorldPosition(new THREE.Vector3()) ?? null;
  const flowers = props.filter((p) => p.asset === "flowers").filter((_, i) => i % 3 === 0).map((p) => ({ x: p.cx, z: p.cz }));
  const life = ambience(root, { flowers, smoke }, reduced);
  return { map, root, blocked, floorAt, climb, levels, exitAt, npcAt, propAt, talk, warps, npcs, grid, sun, door, life, boats, glows, glints, path: [], now: 0, nearKey: null };
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
