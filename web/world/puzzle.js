// The trials' rules (a prototype of field puzzles, docs/PUZZLES.md): crates pushed by hand onto marks, ice that
// carries the hero (and a crate) on until something stops it, and logs that roll across their length until they
// stop, dropping into a channel as long as they are to make a bridge. Pure (no three.js), so node tests it
// (puzzle.test.mjs, tools/puzzles.mjs); trials.js draws it and world.js moves the hero through it.
//
// A map's `trials`: [{ "id", "flag", "crates": [[x, z]], "look": "crate" | "stone", "plates": [[x, z]],
//   "logs": [[x, z, "x" | "z"]], "channel": [[x0, z0, x1, z1]], "ice": [[x0, z0, x1, z1]], "dry": [[x, z]],
//   "reset": [x, z], "solved": { "lines", "do" }, "start": [x, z], "goal": [x, z] }]
// A log lies on (x, z) and the next tile along its axis. `reset`: a rune stone that puts the trial's blocks back.
// A trial with plates is solved when every plate holds a crate; its `solved` lines then run (their `do` sets
// `flag`), and a solved trial is laid out solved. `start` and `goal`: where tools/puzzles.mjs begins and ends.
export const DIRS = { n: [0, -1], s: [0, 1], e: [1, 0], w: [-1, 0] };
export const LOG = 2; // tiles a log covers
const k = (x, z) => `${x},${z}`;
const rect = (out, [x0, z0, x1, z1]) => {
  for (let z = z0; z <= z1; z++) for (let x = x0; x <= x1; x++) out.add(k(x, z));
};

// `done(def)`: the trial was solved before (its flag holds): its crates start on its plates.
export function trials(defs = [], done = () => false) {
  const blocks = [], plates = [], ice = new Set(), channel = new Set();
  for (const d of defs) {
    const solved = !!d.plates?.length && done(d);
    (d.crates ?? []).forEach(([x, z], i) => blocks.push({ kind: "crate", ...(solved && d.plates[i] ? { x: d.plates[i][0], z: d.plates[i][1] } : { x, z }), trial: d }));
    for (const [x, z, axis] of d.logs ?? []) blocks.push({ kind: "log", x, z, axis, trial: d, sunk: false });
    for (const [x, z] of d.plates ?? []) plates.push({ x, z, trial: d });
    (d.ice ?? []).forEach((r) => rect(ice, r));
    for (const [x, z] of d.dry ?? []) ice.delete(k(x, z)); // dry ground in the ice: a place to stop
    (d.channel ?? []).forEach((r) => rect(channel, r));
  }
  return { blocks, plates, ice, channel, filled: new Set(), solved: new Set(defs.filter((d) => d.plates?.length && done(d))) };
}

// The tiles a block covers.
export const cells = (b) => (b.kind === "log" ? Array.from({ length: LOG }, (_, i) => (b.axis === "x" ? [b.x + i, b.z] : [b.x, b.z + i])) : [[b.x, b.z]]);
// The standing block on (x, z) (a sunk log is floor now), or null.
export const blockAt = (s, x, z, except = null) => s.blocks.find((b) => b !== except && !b.sunk && cells(b).some(([cx, cz]) => cx === x && cz === z)) ?? null;
export const iceAt = (s, x, z) => s.ice.has(k(x, z));
// A channel tile no log fills yet: nothing walks or rolls onto it (a log as long as the gap drops into it).
export const gapAt = (s, x, z) => s.channel.has(k(x, z)) && !s.filled.has(k(x, z));

// Where block `b` ends up when pushed toward `f`, or null when it can't move: { x, z, sunk, n } (n: tiles moved).
// `open(ax, az, bx, bz)`: the world lets a step go there (no wall, person, door or cliff).
// A crate moves one tile, and on over ice until something stops it. A log rolls only across its length, tile after
// tile until it can't; it never rolls half over a channel, and one that rolls wholly over channel tiles drops in.
export function shove(s, b, f, open) {
  const [dx, dz] = DIRS[f];
  if (b.kind === "log" && (b.axis === "x") === (dx !== 0)) return null; // along its length: it won't roll
  const floor = (ax, az, bx, bz) => !blockAt(s, bx, bz, b) && !gapAt(s, bx, bz) && open(ax, az, bx, bz);
  let x = b.x, z = b.z, n = 0;
  const at = () => cells({ ...b, x, z });
  while (true) {
    const next = at().map(([cx, cz]) => [cx, cz, cx + dx, cz + dz]);
    const gaps = next.filter(([, , nx, nz]) => gapAt(s, nx, nz)).length;
    if (b.kind === "log" && gaps === next.length && next.every(([, , nx, nz]) => !blockAt(s, nx, nz, b))) return { x: x + dx, z: z + dz, sunk: true, n: n + 1 };
    if (gaps || !next.every((c) => floor(...c))) break;
    (x += dx), (z += dz), n++;
    if (b.kind === "crate" && !iceAt(s, x, z)) break;
  }
  return n ? { x, z, sunk: false, n } : null;
}

// Put a block on (x, z), sunk or standing: a sunk log fills the channel under it, one lifted out frees it again.
// The one place that moves a block, so the game (`move`, a rune stone's reset) and the solver can't drift apart.
export function place(s, b, x, z, sunk = false) {
  if (b.sunk) cells(b).forEach(([cx, cz]) => s.filled.delete(k(cx, cz)));
  (b.x = x), (b.z = z), (b.sunk = sunk);
  if (sunk) cells(b).forEach(([cx, cz]) => s.filled.add(k(cx, cz)));
}

// Move it there (`to` from shove); returns its trial if that solved it (every plate holds a crate), else null.
export function move(s, b, to) {
  place(s, b, to.x, to.z, to.sunk);
  const t = b.trial;
  if (s.solved.has(t) || !t.plates?.length || !t.plates.every(([px, pz]) => blockAt(s, px, pz)?.kind === "crate")) return null;
  s.solved.add(t);
  return t;
}

// Where the hero comes to rest after a step from (x, z) toward `f`: one step, then on while it stands on ice and
// the next tile is open. (world.js plays it a tile at a time; tools read it whole.) null: no step.
export function slide(s, x, z, f, open) {
  const [dx, dz] = DIRS[f];
  const free = (ax, az) => !blockAt(s, ax + dx, az + dz) && !gapAt(s, ax + dx, az + dz) && open(ax, az, ax + dx, az + dz);
  if (!free(x, z)) return null;
  (x += dx), (z += dz);
  while (iceAt(s, x, z) && free(x, z)) (x += dx), (z += dz);
  return [x, z];
}
