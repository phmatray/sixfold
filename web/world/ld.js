// Level-design analysis (#110): the geometry tools/check-maps.mjs judges a map with, and the debug overlays
// (#111) draw. Plain ES with no imports, so Node and the browser share one truth. Tables (FOOT, DECK, WALKABLE,
// heights) come in as arguments; nothing here reads props.js.
// Tile (x, z) covers [x, x + 1] x [z, z + 1]; north is -z, the camera sits south (+z) looking north and down.

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
export function visible(map, boxes, from, target) {
  const cam = map.camera ?? {}, pitch = ((cam.pitch ?? 50) * Math.PI) / 180, dist = cam.distance ?? 12, off = cam.lookAtOffset ?? [0, 0.75, 0];
  const look = [from.x + 0.5 + off[0], (from.y ?? 0) + off[1], from.z + 0.5 + off[2]];
  const o = [look[0], look[1] + Math.sin(pitch) * dist, look[2] + Math.cos(pitch) * dist];
  const e = [target.x + (target.w ?? 1) / 2, target.y, target.z + (target.d ?? 1) / 2];
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
    if (t0 < t1) return false;
  }
  return true;
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
