// The trials solved by search (docs/PUZZLES.md): read a map's collision as build.js does, play a move with the
// game's own rules (web/world/puzzle.js), and find shortest solutions. puzzle.test.mjs holds every level to its
// numbers with this; e2e/trials-play.mjs plays what it finds in the browser.
//
//   node tools/puzzles.mjs web/world/maps/trial_depot.json     prints each trial's numbers
import { readFileSync } from "node:fs";
import { trials, cells, blockAt, iceAt, shove, move, slide, DIRS } from "../web/world/puzzle.js";

export const F = Object.keys(DIRS);
const WALKABLE = new Set(["rug", "exit_mat", "leaf_litter"]); // props.js WALKABLE (the ones the trials use)

// A map's collision for trial `id`: solid tiles, digit heights (a step climbs 0.6 at most), props and rune stones on
// their own tiles, people on theirs. A gate is shut, unless another trial's flag opens it (the depot's back door,
// open once its front room is done): then the hero walks through it, but a block never goes onto it, nor onto a door
// (world.js `roomFor`).
export function grid(map, id) {
  const tile = (x, z) => map.tiles[z]?.[x];
  const height = (x, z) => (tile(x, z) >= "1" && tile(x, z) <= "9" ? +tile(x, z) : 0);
  const earlier = new Set((map.trials ?? []).filter((t) => t.id !== id && t.flag).map((t) => `flag:${t.flag}`));
  const gates = (map.gates ?? []).flatMap((g) => g.tiles.map((t) => [String(t), earlier.has(g.if)]));
  const taken = new Set([
    ...map.props.filter((p) => !WALKABLE.has(p.asset)).map((p) => `${p.x},${p.z}`),
    ...(map.trials ?? []).filter((t) => t.reset).map((t) => String(t.reset)),
    ...gates.filter(([, open]) => !open).map(([k]) => k),
    ...(map.npcs ?? []).map((n) => `${n.x},${n.z}`),
  ]);
  const open = (ax, az, bx, bz) => tile(bx, bz) !== undefined && !"#fWV~".includes(tile(bx, bz)) && !taken.has(`${bx},${bz}`) && Math.abs(height(bx, bz) - height(ax, az)) <= 0.6;
  const noBlock = new Set([...(map.warps ?? []).map((w) => `${w.x},${w.z}`), ...gates.map(([k]) => k)]);
  const blockOpen = (ax, az, bx, bz) => open(ax, az, bx, bz) && !noBlock.has(`${bx},${bz}`);
  return { tile, height, open, blockOpen };
}

// One move toward `f` from hero tile `h`, as world.js plays it: into a block, the block is pushed (the hero follows
// one tile, then slides on if that tile is ice); else a step, sliding on over ice. Mutates `s`; returns the hero's
// new tile, or null when nothing moves.
export function play(s, h, f, open, blockOpen = open) {
  const [dx, dz] = DIRS[f], tx = h[0] + dx, tz = h[1] + dz, b = blockAt(s, tx, tz);
  if (!b) return slide(s, h[0], h[1], f, open);
  const to = shove(s, b, f, blockOpen);
  if (!to) return null;
  move(s, b, to);
  return iceAt(s, tx, tz) ? slide(s, tx, tz, f, open) ?? [tx, tz] : [tx, tz];
}

// A trial as a state machine: its blocks as [x, z, sunk] and the hero's tile.
const snap = (s) => s.blocks.map((b) => [b.x, b.z, b.sunk ? 1 : 0]);
function restore(s, blocks) {
  s.filled.clear();
  s.blocks.forEach((b, i) => {
    [b.x, b.z] = blocks[i];
    b.sunk = !!blocks[i][2];
    if (b.sunk) cells(b).forEach(([x, z]) => s.filled.add(`${x},${z}`));
  });
}
const key = (h, blocks) => `${h}|${blocks.map(String).join(";")}`;

// Breadth-first over every (hero, blocks) state from `start`: the fewest moves to a state `goal(s, h)` holds in,
// the moves themselves, and how many states were seen. `limit`: give up past this many states.
export function solve(map, id, { start, goal, limit = 3e6 } = {}) {
  const def = map.trials.find((t) => t.id === id), s = trials([def]), { open, blockOpen } = grid(map, id);
  start ??= def.start ?? [map.spawn.x, map.spawn.z];
  goal ??= def.plates?.length ? (s) => s.plates.every((p) => blockAt(s, p.x, p.z)?.kind === "crate") : (s, h) => h[0] === def.goal[0] && h[1] === def.goal[1];
  const b0 = snap(s), prev = new Map([[key(start, b0), null]]);
  let q = [[start, b0]];
  while (q.length) {
    const next = [];
    for (const [h, b] of q) {
      restore(s, b);
      if (goal(s, h)) {
        const moves = [];
        for (let kk = key(h, b); prev.get(kk); kk = prev.get(kk).from) moves.unshift(prev.get(kk).m);
        return { moves, seen: prev.size };
      }
      for (const f of F) {
        restore(s, b);
        const pushes = !!blockAt(s, h[0] + DIRS[f][0], h[1] + DIRS[f][1]);
        const nh = play(s, h, f, open, blockOpen);
        if (!nh) continue;
        const nb = snap(s), nk = key(nh, nb);
        if (prev.has(nk)) continue;
        prev.set(nk, { from: key(h, b), m: { f, h: nh, push: pushes } });
        next.push([nh, nb]);
      }
      if (prev.size > limit) return { moves: null, seen: prev.size, gaveUp: true };
    }
    q = next;
  }
  return { moves: null, seen: prev.size };
}

// Every state the player can reach from the start, and which of them are dead ends: states from which no goal can
// be reached any more (a crate in a corner, a log rolled to the wall, the hero carried off by the ice). `traps` is
// their share of all reachable states: the Lens of Challenge's "how easily can I ruin this?".
export function traps(map, id, { start, goal, limit = 3e5 } = {}) {
  const def = map.trials.find((t) => t.id === id), s = trials([def]), { open, blockOpen } = grid(map, id);
  start ??= def.start ?? [map.spawn.x, map.spawn.z];
  goal ??= def.plates?.length ? (s) => s.plates.every((p) => blockAt(s, p.x, p.z)?.kind === "crate") : (s, h) => h[0] === def.goal[0] && h[1] === def.goal[1];
  const b0 = snap(s), k0 = key(start, b0), into = new Map([[k0, []]]), goals = [];
  let q = [[start, b0, k0]];
  while (q.length && into.size < limit) {
    const next = [];
    for (const [h, b, kk] of q) {
      restore(s, b);
      if (goal(s, h)) { goals.push(kk); continue; } // a goal state: play stops there
      for (const f of F) {
        restore(s, b);
        const nh = play(s, h, f, open, blockOpen);
        if (!nh) continue;
        const nb = snap(s), nk = key(nh, nb);
        if (!into.has(nk)) into.set(nk, []), next.push([nh, nb, nk]);
        into.get(nk).push(kk);
      }
    }
    q = next;
  }
  const alive = new Set(goals), back = [...goals];
  while (back.length) for (const p of into.get(back.pop()) ?? []) if (!alive.has(p)) alive.add(p), back.push(p);
  const dead = into.size - alive.size;
  return { states: into.size, dead, traps: dead / into.size, capped: into.size >= limit };
}

// Whether the hero can walk from `h` to `to` as things stand (no pushing, no sliding needed): a solved trial must leave
// its way out open, or the player is shut in once it's done.
export function walkable(map, id, s, h, to) {
  const { open } = grid(map, id), seen = new Set([String(h)]), q = [h];
  while (q.length) {
    const [x, z] = q.pop();
    if (x === to[0] && z === to[1]) return true;
    for (const [dx, dz] of Object.values(DIRS)) {
      const nx = x + dx, nz = z + dz;
      if (seen.has(`${nx},${nz}`) || blockAt(s, nx, nz) || !open(x, z, nx, nz)) continue;
      seen.add(`${nx},${nz}`), q.push([nx, nz]);
    }
  }
  return false;
}

// The numbers a level is judged by: moves and pushes on the shortest solution (shortest in moves), and how many
// states the search went through (a rough measure of how much there is to get lost in).
export function measure(map, id, opts) {
  const r = solve(map, id, opts);
  return { solvable: !!r.moves, moves: r.moves?.length ?? null, pushes: r.moves?.filter((m) => m.push).length ?? null, states: r.seen, moveList: r.moves };
}

if (import.meta.url === `file://${process.argv[1]}`) {
  for (const path of process.argv.slice(2)) {
    const map = JSON.parse(readFileSync(path, "utf8"));
    for (const t of map.trials ?? []) {
      const { moveList, ...m } = measure(map, t.id), tr = traps(map, t.id);
      console.log(path.split("/").pop(), t.id, JSON.stringify({ ...m, traps: +tr.traps.toFixed(2), dead: tr.dead }));
    }
  }
}
