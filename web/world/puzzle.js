// The trials' rules (a prototype of field puzzles): boulders pushed by hand, pillars shoved from afar with
// Reach, plates they settle on, and ice that carries the hero on until something stops it. Pure (no three.js),
// so node tests it (puzzle.test.mjs); trials.js draws it and world.js moves the hero through it.
//
// A map's `trials`: [{ "id", "flag", "boulders": [[x, z]], "pillars": [[x, z]], "plates": [[x, z]], "reset": [x, z],
//   "ice": [[x0, z0, x1, z1]], "dry": [[x, z]], "dais": [[x0, z0, x1, z1]], "solved": { "lines", "do" } }]
//   (`dais`: flagstones drawn on raised ground, trials.js only; `start`, `goal`: where the tests
//   begin and end a search). `reset`: a rune stone that puts the trial's blocks back. A trial with plates is solved when every plate
// holds a block; its `solved` dialogue then runs (its `do` sets `flag`), and a solved trial is laid out solved.
export const REACH = 4; // how many tiles Reach carries
export const DIRS = { n: [0, -1], s: [0, 1], e: [1, 0], w: [-1, 0] };
const k = (x, z) => `${x},${z}`;

// `done(def)`: the trial was solved before (its flag holds): its blocks start on its plates.
export function trials(defs = [], done = () => false) {
  const blocks = [], plates = [], ice = new Set();
  for (const d of defs) {
    const solved = !!d.plates?.length && done(d);
    let n = 0; // the trial's blocks so far: solved, the nth sits on the nth plate
    const add = (kind) => (p) => {
      const [x, z] = (solved && d.plates[n]) || p;
      blocks.push({ kind, x, z, trial: d }), n++;
    };
    (d.boulders ?? []).forEach(add("boulder"));
    (d.pillars ?? []).forEach(add("pillar"));
    for (const [x, z] of d.plates ?? []) plates.push({ x, z, trial: d });
    for (const [x0, z0, x1, z1] of d.ice ?? []) for (let z = z0; z <= z1; z++) for (let x = x0; x <= x1; x++) ice.add(k(x, z));
    for (const [x, z] of d.dry ?? []) ice.delete(k(x, z)); // dry ground in the ice: a place to stop
  }
  return { blocks, plates, ice, solved: new Set(defs.filter((d) => d.plates?.length && done(d))) };
}

export const blockAt = (s, x, z) => s.blocks.find((b) => b.x === x && b.z === z) ?? null;
export const iceAt = (s, x, z) => s.ice.has(k(x, z));

// Where block `b` goes when shoved toward `f`, or null. `open(ax, az, bx, bz)`: the world lets a step go there
// (no wall, person, door or cliff). A block never lands on another.
export function shove(s, b, f, open) {
  const [dx, dz] = DIRS[f], x = b.x + dx, z = b.z + dz;
  return !blockAt(s, x, z) && open(b.x, b.z, x, z) ? [x, z] : null;
}

// Move it; returns its trial if that move solved it (every plate of the trial holds a block), else null.
export function move(s, b, x, z) {
  (b.x = x), (b.z = z);
  const t = b.trial;
  if (s.solved.has(t) || !t.plates?.length || !t.plates.every(([px, pz]) => blockAt(s, px, pz))) return null;
  s.solved.add(t);
  return t;
}

// The pillar Reach would shove from (hx, hz) facing `f`, or null: the first block in line within REACH, if it
// is a pillar. `see(x, z)`: nothing in the way there (a pit, water or a ledge between is fine; a wall is not).
export function reachTarget(s, hx, hz, f, see) {
  const [dx, dz] = DIRS[f];
  for (let i = 1; i <= REACH; i++) {
    const x = hx + dx * i, z = hz + dz * i, b = blockAt(s, x, z);
    if (b) return b.kind === "pillar" ? b : null;
    if (!see(x, z)) return null;
  }
  return null;
}

// Where the hero comes to rest after a step from (x, z) toward `f`: one step, then on while it stands on ice and
// the next tile is open. (world.js plays it a tile at a time; tests and tools read it whole.) null: no step.
export function slide(s, x, z, f, open) {
  const [dx, dz] = DIRS[f];
  if (blockAt(s, x + dx, z + dz) || !open(x, z, x + dx, z + dz)) return null;
  (x += dx), (z += dz);
  while (iceAt(s, x, z) && !blockAt(s, x + dx, z + dz) && open(x, z, x + dx, z + dz)) (x += dx), (z += dz);
  return [x, z];
}
