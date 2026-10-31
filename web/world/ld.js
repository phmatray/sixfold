// Level-design analysis (#110): the geometry tools/check-maps.mjs judges a map with, and the debug overlays
// (#111) draw. Plain ES with no imports, so Node and the browser share one truth. The props.js tables (FOOT, DECK,
// WALKABLE, TALL) come in as arguments; nothing here reads props.js.
// Tile (x, z) covers [x, x + 1] x [z, z + 1]; north is -z, the camera sits south (+z) looking north and down.

// Prop heights (m), from ASSETS.md / ASSETS_V13.md, for the camera boxes and depth stacking; props.js TALL fills
// the rest (the `tall` table check-maps and the overlays pass to grids()).
export const HEIGHT = { tree: 3.5, pine: 3.8, birch: 3.6, willow: 3.8, house: 5, sign: 1.3, signpost: 1.6, lantern: 2.2, bush: 0.7, flowers: 0.3,
  rock: 0.6, crates: 0.9, glow_mushroom: 0.35, glow_mushroom_tall: 0.9, glow_shelf: 0.62, bench: 0.9, stall: 2.3, stall_b: 2.3, fountain: 1.6, stump: 0.55, log: 0.55, hanging_sign: 2.5, boat: 0.4, reeds: 0.9,
  shelf: 2, bookcase: 2.3, oven: 2.45, printing_press: 1.8, workbench: 1.8, net_locker: 1.95, plant: 1, chair: 0.9, display_case: 1.1, lamp: 1.6,
  bed: 1.1, light_box: 1.25, type_case: 1.35, curio_cabinet: 2.3, seal_stone: 1.9, cave_mouth: 2.2, crab_pots: 0.45, net_rack: 1.45, bollard: 0.6, rock_sea: 0.55 };

const DIRS = [[0, -1], [0, 1], [1, 0], [-1, 0]];
const key = (x, z) => `${x},${z}`;

// A prop's footprint in tiles, its rotation applied.
export function foot(p, FOOT) {
  let [w, d] = FOOT[p.asset] ?? [1, 1];
  if (p.rot % 180 === 90) [w, d] = [d, w];
  return [w, d];
}

// The solid, floor-height and path grids of a map, and a box (FOOT x height) per standing prop.
// t: { FOOT, DECK, WALKABLE: Set, tall: (asset) => metres }.
export function grids(map, t) {
  const solid = new Set(), height = {}, path = new Set(), boxes = [];
  map.tiles.forEach((row, z) => [...row].forEach((c, x) => {
    if (c >= "1" && c <= "9") height[key(x, z)] = +c;
    if ("#fWV~".includes(c)) solid.add(key(x, z));
    if ("pr=H".includes(c) || "pr".includes(map.paint?.[z]?.[x] ?? "-")) path.add(key(x, z));
  }));
  for (const p of map.props ?? []) {
    const [w, d] = foot(p, t.FOOT), deck = t.DECK[p.asset];
    for (let x = p.x; x < p.x + w; x++)
      for (let z = p.z; z < p.z + d; z++) {
        if (t.WALKABLE.has(p.asset) || deck) solid.delete(key(x, z));
        else solid.add(key(x, z));
        if (deck) height[key(x, z)] = (p.y ?? 0) + deck[0], path.add(key(x, z));
      }
    // Walked-on props (decks, rugs) hide nothing; everything else is a box from its base to its top.
    if (!t.WALKABLE.has(p.asset) && !deck) boxes.push({ x0: p.x, z0: p.z, x1: p.x + w, z1: p.z + d, y0: p.y ?? 0, y1: (p.y ?? 0) + t.tall(p.asset), prop: p });
  }
  return { width: map.width, height: map.height, solid, floor: height, path, boxes };
}

// Is the top of `target` seen from the map's camera while the hero stands on `from`? The camera follows the
// hero's tile at the map's pitch and distance (world.js); the ray runs from it to the target's top centre and is
// blocked by any box hit before it gets there, except the target's own (`target.self`: a box or a list).
// from: { x, z, y? (floor) }; target: { x, z, w = 1, d = 1, y (top, m), self? }.
// ponytail: props are their bounding boxes and the terrain (cliffs, raised tiles) hides nothing; a real mesh
// test belongs in the browser overlay if the boxes prove too strict.
export const visible = (map, boxes, from, target) => !blocker(map, boxes, from, target);

// That ray: the camera's eye `o` and the target's top centre `e`, both [x, y, z].
export function ray(map, from, target) {
  const cam = map.camera ?? {}, pitch = ((cam.pitch ?? 50) * Math.PI) / 180, dist = cam.distance ?? 12, off = cam.lookAtOffset ?? [0, 0.75, 0];
  const look = [from.x + 0.5 + off[0], (from.y ?? 0) + off[1], from.z + 0.5 + off[2]];
  return { o: [look[0], look[1] + Math.sin(pitch) * dist, look[2] + Math.cos(pitch) * dist], e: [target.x + (target.w ?? 1) / 2, target.y, target.z + (target.d ?? 1) / 2] };
}

// The first box in `boxes` that blocks that ray (null: the target is seen); the debug overlay highlights it.
export function blocker(map, boxes, from, target) {
  const { o, e } = ray(map, from, target);
  const dir = [e[0] - o[0], e[1] - o[1], e[2] - o[2]];
  const self = new Set([target.self ?? []].flat());
  for (const b of boxes) {
    if (self.has(b) || self.has(b.prop)) continue;
    const lo = [b.x0, b.y0, b.z0], hi = [b.x1, b.y1, b.z1];
    let t0 = 0, t1 = 1 - 1e-6;
    for (let i = 0; i < 3 && t0 <= t1; i++) {
      if (Math.abs(dir[i]) < 1e-12) {
        if (o[i] < lo[i] || o[i] > hi[i]) t0 = Infinity;
        continue;
      }
      let a = (lo[i] - o[i]) / dir[i], c = (hi[i] - o[i]) / dir[i];
      if (a > c) [a, c] = [c, a];
      t0 = Math.max(t0, a), t1 = Math.min(t1, c);
    }
    if (t0 < t1) return b;
  }
  return null;
}

// Points of interest by tier (WORLD_RULES §9): a `poi` tag, or major by default for warps, waymarks, exit
// signposts (`signs`, a Set of interactions) and people who start a story duel (a rival, a seal keeper: a `duel:`
// cue in what they say). Each: { x, z, tier, what, prop? (a tagged prop), o? (the spot, person or warp), npc? }.
export function pois(m, signs = new Set()) {
  const duelist = (n) => n.model === "npc_rival" || /"duel:/.test(JSON.stringify(n.dialogue ?? []));
  return [
    ...m.props.filter((p) => p.poi).map((p) => ({ x: p.x, z: p.z, tier: p.poi, what: `${p.asset} ${p.x},${p.z}`, prop: p })),
    ...(m.interactions ?? []).map((i) => ({ x: i.x, z: i.z, tier: i.poi ?? (i.kind === "waymark" || signs.has(i) ? "major" : null), what: `${i.kind} ${i.id ?? ""} ${i.x},${i.z}`.replace("  ", " "), o: i })),
    ...(m.npcs ?? []).map((n) => ({ x: n.x, z: n.z, tier: n.poi ?? (duelist(n) ? "major" : null), what: `${n.id} ${n.x},${n.z}`, npc: true, o: n })),
    ...(m.warps ?? []).map((w) => ({ x: w.x, z: w.z, tier: w.poi ?? "major", what: `warp to ${w.to} ${w.x},${w.z}`, o: w })),
  ].filter((p) => p.tier);
}

// The hidden-mandatory check (§11) on one POI of `pois`: the tile it is used from (its front row first, then
// south, east, west, north of each tile: the camera looks north) and the target the camera must see from there
// (its top). null when no reachable tile is beside it. g: grids(); open: a Set of reachable tiles; floor(x, z).
export function approach(p, g, open, floor, FOOT) {
  const near4 = [[0, 1], [1, 0], [-1, 0], [0, -1]];
  const box = p.prop ? g.boxes.find((b) => b.prop === p.prop) : g.boxes.find((b) => p.x >= b.x0 && p.x < b.x1 && p.z >= b.z0 && p.z < b.z1);
  const [w, d] = p.prop ? foot(p.prop, FOOT) : [1, 1];
  let from = null;
  for (let z = p.z + d - 1; z >= p.z && !from; z--)
    for (let x = p.x; x < p.x + w && !from; x++) from = near4.map(([dx, dz]) => [x + dx, z + dz]).find(([a, b]) => open.has(key(a, b))) ?? null;
  if (!from) return null;
  const y = p.prop ? box?.y1 ?? 1 : p.npc ? 1.45 : box ? Math.min(box.y1, 2) : 1;
  return { from: { x: from[0], z: from[1], y: floor(...from) }, target: { x: p.x, z: p.z, w, d, y, self: box } };
}

// The tile on a map's edge at position c along an exit's side, and all of an exit's edge tiles.
export const edgeTile = (m, side, c) => ({ n: [c, 0], s: [c, m.height - 1], w: [0, c], e: [m.width - 1, c] })[side];
export const edgeTiles = (m, e) => Array.from({ length: e.span[1] - e.span[0] + 1 }, (_, c) => edgeTile(m, e.side, e.span[0] + c));
// A biome transition (WORLD_RULES 5.7, #118): a map's `transitions` entry `{ to, depth, scatter }` shades the
// ground, plants, light and ambience toward the map its edge exit to `to` leads to. Its band hugs that exit:
// weight 1 at the edge down to 0 `depth` tiles in; along the edge it covers the exit's span and `depth` tiles
// either side, then fades out over another `depth`. bandOf: the entry with its exit's side and span (or nothing).
export function bandOf(m, t) {
  const e = (m.exits ?? []).find((e) => e.to === t.to);
  return e && { ...t, side: e.side, span: e.span };
}
// The band's weight at a point (x, z) in tiles (a tile's centre is x + 0.5, z + 0.5), 0..1.
export function bandWeight(m, b, x, z) {
  const d = { n: z, s: m.height - z, w: x, e: m.width - x }[b.side], a = b.side === "n" || b.side === "s" ? x : z;
  const off = Math.max(0, b.span[0] - b.depth - a, a - (b.span[1] + 1 + b.depth));
  return Math.min(1, Math.max(0, 1 - d / b.depth)) * Math.max(0, 1 - off / b.depth);
}
// An exit's signpost: a sign within 6 tiles of the exit's edge naming `dest`, the map it leads to (WORLD_RULES 5).
export function exitSign(m, e, dest) {
  return dest?.title && (m.interactions ?? []).find((i) => i.kind === "sign" && String(i.text).includes(dest.title) && edgeTiles(m, e).some(([x, z]) => Math.abs(i.x - x) + Math.abs(i.z - z) <= 6));
}

// The discoveries of a map, for its spacing: each POI (the reachable tiles round its footprint) and each find
// that isn't one (its tile and the four round it). [{ what, tiles: ["x,z"] }], those with a reachable tile.
export function discoveries(m, pois, open, FOOT) {
  const finds = (m.interactions ?? []).filter((i) => i.kind === "find" && !pois.some((p) => p.o === i)).map((i) => ({ x: i.x, z: i.z, what: `find ${i.id} ${i.x},${i.z}` }));
  const near = (x, z) => [[0, 0], [0, 1], [1, 0], [-1, 0], [0, -1]].map(([dx, dz]) => key(x + dx, z + dz)).filter((t) => open.has(t));
  return [...pois.map((p) => {
    const [w, d] = p.prop ? foot(p.prop, FOOT) : [1, 1], tiles = [];
    for (let x = p.x - 1; x <= p.x + w; x++) for (let z = p.z - 1; z <= p.z + d; z++) if (open.has(key(x, z))) tiles.push(key(x, z));
    return { what: p.what, tiles };
  }), ...finds.map((f) => ({ what: f.what, tiles: near(f.x, f.z) }))].filter((f) => f.tiles.length);
}

// The longest walk without a discovery: from each of `sources` ({ what, tiles }) the walk to the nearest of `finds`
// (not itself), the longest of those. { tiles, from (the source), to (the tile reached), parent (to trace the walk
// back) }, or null. walk(starts) is a bfs() over the map's open tiles.
export function gap(sources, finds, walk) {
  let best = null;
  for (const src of sources) {
    const { dist, parent } = walk(src.tiles.map((t) => t.split(",").map(Number)));
    let near = null;
    for (const f of finds) if (f !== src) for (const t of f.tiles) if (dist.has(t) && (!near || dist.get(t) < near.tiles)) near = { tiles: dist.get(t), to: t };
    if (near && (!best || near.tiles > best.tiles)) best = { ...near, from: src, parent };
  }
  return best;
}

// The ring of tiles 2 away round (x, z), in order round the loop.
const RING = [[-2, -2], [-1, -2], [0, -2], [1, -2], [2, -2], [2, -1], [2, 0], [2, 1], [2, 2], [1, 2], [0, 2], [-1, 2], [-2, 2], [-2, 1], [-2, 0], [-2, -1]];
// The ways out of a path tile: the runs of path on that ring, with their lengths. An all-path ring is a plaza (none).
export function branches(path, x, z) {
  const on = RING.map(([dx, dz]) => path.has(key(x + dx, z + dz)));
  if (on.every(Boolean)) return [];
  const start = on.indexOf(false), runs = [];
  for (let i = 1, n = 0; i <= RING.length; i++) {
    if (on[(start + i) % RING.length]) n++;
    else if (n) runs.push(n), (n = 0);
  }
  return runs;
}

// The path graph: junctions (3+ ways out) and dead ends (one narrow way out, 1-2 tiles wide), each dead end with
// its length back to the nearest junction along the path; a piece of path with no junction measures its own length.
export function pathGraph(g) {
  const { path } = g, junctions = [], endTiles = new Set(), runs = {};
  for (const k of path) {
    const [x, z] = k.split(",").map(Number), r = (runs[k] = branches(path, x, z));
    if (r.length >= 3) junctions.push({ x, z, branches: r.length });
    if (r.length === 1 && r[0] <= 3) endTiles.add(k);
  }
  const dist = bfs(path, junctions.map((j) => [j.x, j.z])).dist;
  const deadEnds = [], seen = new Set();
  for (const k of endTiles) {
    if (seen.has(k)) continue;
    // One dead end per cluster of end tiles: its tile farthest from a junction.
    let best = null;
    for (const c of bfs(endTiles, [k.split(",").map(Number)]).dist.keys()) {
      seen.add(c);
      const d = dist.get(c) ?? Infinity;
      if (!best || d > best.length) best = { x: +c.split(",")[0], z: +c.split(",")[1], length: d };
    }
    if (!Number.isFinite(best.length)) best.length = Math.max(...bfs(path, [[best.x, best.z]]).dist.values());
    deadEnds.push(best);
  }
  return { junctions, deadEnds };
}

// The loops of the path (#117): each hole the walkable path rings round, a patch of other ground (8-connected, off
// the map's edge) with at least one tile the path layout leaves out (a prop standing on a plaza is no loop), and its
// ring, the walkable path tiles round it, whose count is the loop's length in tiles. open: the walkable path tiles.
// ponytail: the ring ignores floor heights (a path over a cliff edge would read as joined); paths change height by
// stairs, so it holds for now.
export function loops(g, open) {
  const holes = new Set(), out = [];
  for (let x = 0; x < g.width; x++) for (let z = 0; z < g.height; z++) if (!open.has(key(x, z))) holes.add(key(x, z));
  const ring8 = [...DIRS, [1, 1], [1, -1], [-1, 1], [-1, -1]];
  for (const k0 of holes) {
    const patch = [k0], ring = new Set();
    holes.delete(k0);
    let edge = false;
    for (let i = 0; i < patch.length; i++) {
      const [x, z] = patch[i].split(",").map(Number);
      if (x === 0 || z === 0 || x === g.width - 1 || z === g.height - 1) edge = true;
      for (const [dx, dz] of ring8) {
        const k = key(x + dx, z + dz);
        if (open.has(k)) ring.add(k);
        else if (holes.delete(k)) patch.push(k);
      }
    }
    if (!edge && patch.some((k) => !g.path.has(k))) out.push({ hole: patch, ring: [...ring] });
  }
  return out;
}

// Breadth-first search over a set of open tiles from one or more starts: tile -> steps, and each tile's parent.
// `step(ax, az, bx, bz)`, when given, says whether the hero can step between two neighbours (a ledge can't be).
export function bfs(open, starts, step = () => true) {
  const dist = new Map(), parent = new Map(), queue = [];
  for (const [x, z] of starts) if (open.has(key(x, z)) && !dist.has(key(x, z))) dist.set(key(x, z), 0), queue.push([x, z]);
  for (let i = 0; i < queue.length; i++) {
    const [x, z] = queue[i], d = dist.get(key(x, z));
    for (const [dx, dz] of DIRS) {
      const k = key(x + dx, z + dz);
      if (!open.has(k) || dist.has(k) || !step(x, z, x + dx, z + dz)) continue;
      dist.set(k, d + 1), parent.set(k, key(x, z)), queue.push([x + dx, z + dz]);
    }
  }
  return { dist, parent };
}

// Seconds to walk a number of tiles (4 tiles a second, docs/WORLD_RULES.md §8.2).
export const walkTime = (tiles) => tiles / 4;

// How props are spread: the Clark-Evans nearest-neighbour ratio over their bounding area (1 random, under 1
// clumped, over 1 evenly spaced), and per 16 x 16 chunk the share of its dominant asset and rotation.
// points: [{ x, z, asset?, rot? }].
export function scatter(points) {
  const chunks = new Map();
  for (const p of points) {
    const k = key(Math.floor(p.x / 16), Math.floor(p.z / 16));
    (chunks.get(k) ?? chunks.set(k, []).get(k)).push(p);
  }
  const top = (list, f) => {
    const n = new Map();
    for (const p of list) n.set(f(p), (n.get(f(p)) ?? 0) + 1);
    const [v, c] = [...n].sort((a, b) => b[1] - a[1])[0];
    return [v, c / list.length];
  };
  const perChunk = [...chunks].map(([k, list]) => {
    const [asset, share] = top(list, (p) => p.asset), [rot, rotShare] = top(list, (p) => ((p.rot ?? 0) % 360 + 360) % 360);
    return { chunk: k, n: list.length, asset, share, rot, rotShare };
  });
  if (points.length < 2) return { ratio: null, chunks: perChunk };
  const xs = points.map((p) => p.x), zs = points.map((p) => p.z);
  const area = (Math.max(...xs) - Math.min(...xs) + 1) * (Math.max(...zs) - Math.min(...zs) + 1);
  // ponytail: O(n²) nearest neighbours; a few hundred props per map is nothing, a grid hash if maps grow.
  let sum = 0;
  for (const p of points) {
    let best = Infinity;
    for (const q of points) if (q !== p) best = Math.min(best, Math.hypot(p.x - q.x, p.z - q.z));
    sum += best;
  }
  return { ratio: sum / points.length / (0.5 / Math.sqrt(points.length / area)), chunks: perChunk };
}

// Props along the main path: the count in a 13 x 9 window (one screen) every 13 tiles of `route` (a list of
// [x, z]), the coefficient of variation of those counts, and the breathers (windows under half the mean).
export function density(points, route, screen = [13, 9]) {
  const curve = [];
  for (let i = 0; i < route.length; i += screen[0]) {
    const [cx, cz] = route[i];
    curve.push(points.filter((p) => Math.abs(p.x - cx) <= screen[0] >> 1 && Math.abs(p.z - cz) <= screen[1] >> 1).length);
  }
  const mean = curve.reduce((a, b) => a + b, 0) / (curve.length || 1);
  const cv = mean ? Math.sqrt(curve.reduce((a, b) => a + (b - mean) ** 2, 0) / curve.length) / mean : 0;
  return { curve, cv, breathers: curve.flatMap((n, i) => (n < mean / 2 ? [i] : [])) };
}
