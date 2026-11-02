// node --test web/world/puzzle.test.mjs — the trials' rules, and the three trials of maps/trials.json solved by search.
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { trials, blockAt, shove, move, reachTarget, slide, DIRS } from "./puzzle.js";

const map = JSON.parse(readFileSync(new URL("./maps/trials.json", import.meta.url), "utf8"));
const def = (id) => map.trials.find((t) => t.id === id);
// The map's own collision (build.js): solid tiles, digit heights, a step climbs 0.6 at most. Gates stay shut.
const tile = (x, z) => map.tiles[z]?.[x];
const height = (x, z) => (tile(x, z) >= "1" && tile(x, z) <= "9" ? +tile(x, z) : 0);
// Props and the rune stones stand on their tiles; a shut gate is a wall.
const taken = new Set([...map.props.map((p) => `${p.x},${p.z}`), ...map.trials.filter((t) => t.reset).map((t) => String(t.reset)),
  ...map.gates.flatMap((g) => g.tiles.map(String))]);
const open = (ax, az, bx, bz) => tile(bx, bz) !== undefined && !"#fWV~".includes(tile(bx, bz)) && !taken.has(`${bx},${bz}`) && Math.abs(height(bx, bz) - height(ax, az)) <= 0.6;
const see = (hx, hz) => (x, z) => tile(x, z) !== undefined && height(x, z) - height(hx, hz) <= 1.5; // world.js `see`
const F = Object.keys(DIRS);

// Breadth-first over whole states: `moves(state)` lists the next ones, `key` names a state. Returns the fewest
// moves to a goal and the states seen.
function search(start, moves, key, goal) {
  const seen = new Map([[key(start), start]]);
  let frontier = [start], depth = 0, best = null;
  while (frontier.length && best === null) {
    if (frontier.some(goal)) best = depth;
    const next = [];
    for (const s of frontier) for (const n of moves(s)) if (!seen.has(key(n))) seen.set(key(n), n), next.push(n);
    (frontier = next), depth++;
  }
  return { best, seen };
}
// A state for one trial: the hero's tile and its blocks' tiles.
const at = (s) => s.blocks.map((b) => [b.x, b.z]);
const restore = (t, hero, blocks) => (t.blocks.forEach((b, i) => ([b.x, b.z] = blocks[i])), hero);
const stateKey = ({ h, b }) => `${h}|${b.map(String).sort().join(";")}`;
// Moves by hand: walk, or push a boulder ahead.
const handMoves = (t) => ({ h, b }) => {
  restore(t, h, b);
  return F.flatMap((f) => {
    const [dx, dz] = DIRS[f], x = h[0] + dx, z = h[1] + dz, rock = blockAt(t, x, z);
    if (!rock) return open(h[0], h[1], x, z) ? [{ h: [x, z], b }] : [];
    const to = rock.kind === "boulder" && shove(t, rock, f, open);
    return to ? [{ h: [x, z], b: b.map((p) => (p[0] === x && p[1] === z ? to : p)) }] : [];
  });
};
// Moves with Reach too: from where the hero stands, shove the pillar it faces.
const reachMoves = (t) => (s) => {
  const out = handMoves(t)(s);
  restore(t, s.h, s.b);
  for (const f of F) {
    const p = reachTarget(t, s.h[0], s.h[1], f, see(...s.h));
    const to = p && shove(t, p, f, open);
    if (to) out.push({ h: s.h, b: s.b.map((q) => (q[0] === p.x && q[1] === p.z ? to : q)) });
  }
  return out;
};
const onPlates = (d) => ({ b }) => d.plates.every(([x, z]) => b.some((p) => p[0] === x && p[1] === z));

test("a boulder is pushed one tile, never onto another or into a wall", () => {
  const t = trials([{ id: "a", boulders: [[1, 1], [2, 1]], plates: [[1, 2]] }]);
  const free = () => true;
  assert.equal(shove(t, t.blocks[0], "e", free), null, "another boulder is there");
  assert.deepEqual(shove(t, t.blocks[0], "s", free), [1, 2]);
  assert.equal(shove(t, t.blocks[0], "s", () => false), null, "a wall");
  assert.equal(move(t, t.blocks[1], 2, 2), null, "not on the plate");
  assert.equal(move(t, t.blocks[0], 1, 2)?.id, "a", "every plate held: solved");
  assert.equal(move(t, t.blocks[0], 1, 2), null, "solved once");
});

test("a solved trial starts solved, its blocks on its plates", () => {
  const d = { id: "a", flag: "f", boulders: [[1, 1]], pillars: [[5, 5]], plates: [[3, 3], [4, 4]] };
  const t = trials([d], () => true);
  assert.deepEqual(at(t), [[3, 3], [4, 4]]);
  assert.ok(t.solved.has(d));
});

test("Reach takes the first pillar in line within four tiles, through a ledge, not through a wall or a boulder", () => {
  const t = trials([{ id: "a", pillars: [[4, 0]], boulders: [[0, 2]] }]);
  const clear = () => true;
  assert.equal(reachTarget(t, 0, 0, "e", clear)?.kind, "pillar");
  assert.equal(reachTarget(t, -1, 0, "e", clear), null, "five tiles: too far");
  assert.equal(reachTarget(t, 0, 0, "e", (x) => x !== 2), null, "a wall between");
  assert.equal(reachTarget(t, 0, 0, "s", clear), null, "a boulder is not Reach's");
});

test("the hero slides over ice until a wall, a block or dry ground", () => {
  const t = trials([{ id: "i", ice: [[1, 0, 5, 0]], boulders: [[4, 0]] }]);
  const free = (ax, az, bx) => bx <= 6;
  assert.deepEqual(slide(t, 0, 0, "e", free), [3, 0], "stops before the boulder");
  t.blocks.length = 0;
  assert.deepEqual(slide(t, 0, 0, "e", free), [6, 0], "carries on to dry ground");
  assert.deepEqual(slide(t, 0, 0, "e", (ax, az, bx) => bx <= 4), [4, 0], "stops at a wall");
});

test("the boulder trial is solvable and takes some thought", () => {
  const d = def("push"), t = trials([d]);
  const start = { h: map.spawn ? [map.spawn.x, map.spawn.z] : null, b: at(t) };
  const { best } = search(start, handMoves(t), stateKey, onPlates(d));
  assert.ok(best !== null, "no solution");
  assert.ok(best >= 20, `solved in ${best} moves: too easy`);
});

test("the ice trial reaches the far side in a few slides, and never strands the hero", () => {
  const d = def("ice"), t = trials([d]);
  const [sx, sz] = d.start, [gx, gz] = d.goal;
  const moves = ([x, z]) => F.map((f) => slide(t, x, z, f, open)).filter(Boolean);
  const { best, seen } = search([sx, sz], moves, String, ([x, z]) => x === gx && z === gz);
  assert.ok(best !== null, "no way across");
  assert.ok(best >= 5, `across in ${best} slides: too easy`);
  // From anywhere the ice can carry the hero, both the far side and the way back are still in reach.
  for (const p of seen.values()) {
    for (const [tx, tz] of [d.goal, d.start]) assert.ok(search(p, moves, String, ([x, z]) => x === tx && z === tz).best !== null, `stranded at ${p}`);
  }
});

test("the Reach trial is solvable with Reach and takes some thought", () => {
  const d = def("reach"), t = trials([d]);
  const { best } = search({ h: d.start, b: at(t) }, reachMoves(t), stateKey, onPlates(d));
  assert.ok(best !== null, "no solution");
  assert.ok(best >= 12, `solved in ${best} moves: too easy`);
});
