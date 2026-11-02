// node --test web/world/puzzle.test.mjs — the trials' rules, and every prototype level held to its numbers
// (docs/PUZZLES.md): solvable, long enough, and as unforgiving (or as safe) as it was designed to be.
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { trials, blockAt, gapAt, shove, move, slide } from "./puzzle.js";
import { measure, traps, walkable } from "../../tools/puzzles.mjs";

const map = (name) => JSON.parse(readFileSync(new URL(`./maps/${name}.json`, import.meta.url), "utf8"));
const free = () => true;

test("a crate moves one tile, never onto another or into a wall; on ice it slides until stopped", () => {
  const t = trials([{ id: "a", crates: [[1, 1], [2, 1]], plates: [[1, 2]] }]);
  assert.equal(shove(t, t.blocks[0], "e", free), null, "another crate is there");
  assert.deepEqual(shove(t, t.blocks[0], "s", free), { x: 1, z: 2, sunk: false, n: 1 });
  assert.equal(shove(t, t.blocks[0], "s", () => false), null, "a wall");
  assert.equal(move(t, t.blocks[1], { x: 2, z: 2 }), null, "not on the mark");
  assert.equal(move(t, t.blocks[0], { x: 1, z: 2 })?.id, "a", "every mark held: solved");
  const i = trials([{ id: "i", crates: [[0, 0]], ice: [[1, 0, 5, 0]] }]);
  assert.deepEqual(shove(i, i.blocks[0], "e", (ax, az, bx) => bx <= 4), { x: 4, z: 0, sunk: false, n: 4 }, "over the ice to the wall");
});

test("a solved trial starts solved, its crates on its marks", () => {
  const d = { id: "a", flag: "f", crates: [[1, 1], [5, 5]], plates: [[3, 3], [4, 4]] };
  const t = trials([d], () => true);
  assert.deepEqual(t.blocks.map((b) => [b.x, b.z]), [[3, 3], [4, 4]]);
  assert.ok(t.solved.has(d));
});

test("a log rolls across its length until it stops, never along it", () => {
  const t = trials([{ id: "l", logs: [[2, 2, "x"]] }]);
  const log = t.blocks[0], room = (ax, az, bx, bz) => bz >= 0 && bz <= 6;
  assert.equal(shove(t, log, "e", room), null, "along its length");
  assert.deepEqual(shove(t, log, "s", room), { x: 2, z: 6, sunk: false, n: 4 }, "to the wall");
  assert.ok(blockAt(t, 3, 2) === log, "it covers two tiles");
});

test("a log drops into a channel exactly under it and makes a bridge; it won't roll half over one", () => {
  const t = trials([{ id: "l", logs: [[2, 0, "x"], [6, 0, "x"]], channel: [[2, 3, 3, 3], [7, 3, 8, 3]] }]);
  const [a, b] = t.blocks;
  const to = shove(t, a, "s", free);
  assert.deepEqual(to, { x: 2, z: 3, sunk: true, n: 3 });
  assert.ok(gapAt(t, 2, 3), "open before it lands");
  move(t, a, to);
  assert.ok(!gapAt(t, 2, 3) && !gapAt(t, 3, 3) && !blockAt(t, 2, 3), "a bridge: floor now");
  assert.deepEqual(shove(t, b, "s", free), { x: 6, z: 2, sunk: false, n: 2 }, "half over the gap: it stops at its edge");
  assert.deepEqual(slide(t, 2, 2, "s", free), [2, 3], "the hero walks onto the bridge");
});

test("the hero slides over ice until a wall, a block, an open channel or dry ground", () => {
  const t = trials([{ id: "i", ice: [[1, 0, 5, 0]], crates: [[4, 0]] }]);
  const open = (ax, az, bx) => bx <= 6;
  assert.deepEqual(slide(t, 0, 0, "e", open), [3, 0], "stops before the crate");
  t.blocks.length = 0;
  assert.deepEqual(slide(t, 0, 0, "e", open), [6, 0], "carries on to dry ground");
});

// ---------- The levels ----------

// Solved, and the hero can still walk to every tile in `ways` (the way in, the next door): no soft-lock once done.
const solvedAndFree = (m, id, ways) => (s, h) => s.plates.every((p) => blockAt(s, p.x, p.z)?.kind === "crate") && ways.every((w) => walkable(m, id, s, h, w));

test("the depot's front room: a short lesson in getting stuck, and the way on stays open", () => {
  const depot = map("trial_depot"), m = measure(depot, "depot1"), tr = traps(depot, "depot1");
  assert.ok(m.solvable);
  assert.ok(m.pushes >= 12, `${m.pushes} pushes`);
  assert.ok(tr.traps >= 0.5, `only ${Math.round(tr.traps * 100)}% of states are dead ends`);
  assert.ok(measure(depot, "depot1", { goal: solvedAndFree(depot, "depot1", [[4, 11], [4, 7]]) }).solvable, "solved, the hero is shut in");
});

test("the depot's back store: a real Sokoban (30 pushes), and the way out stays open", () => {
  const depot = map("trial_depot"), m = measure(depot, "depot2");
  assert.ok(m.solvable);
  assert.ok(m.pushes >= 30, `${m.pushes} pushes`);
  assert.ok(m.moves >= 100, `${m.moves} moves`);
  assert.ok(measure(depot, "depot2", { goal: solvedAndFree(depot, "depot2", [[4, 6]]) }).solvable, "solved, the hero is shut in");
});

test("the rime cave's first cavern: a dozen slides, and the ice never strands the hero", () => {
  const m = measure(map("trial_ice"), "ice1"), tr = traps(map("trial_ice"), "ice1");
  assert.ok(m.solvable);
  assert.ok(m.moves >= 12, `${m.moves} slides`);
  assert.equal(tr.dead, 0, "a place the ice carries you to with no way on");
});

test("the rime cave's second cavern: the card is out of reach without sliding the crates, and you can always get back", () => {
  const ice = map("trial_ice"), m = measure(ice, "ice2");
  assert.ok(m.solvable);
  assert.ok(m.pushes >= 3 && m.moves >= 35, `${m.pushes} pushes, ${m.moves} moves`);
  const bare = { ...ice, trials: ice.trials.map((t) => (t.id === "ice2" ? { ...t, crates: [] } : t)) };
  assert.ok(!measure(bare, "ice2").solvable, "solvable without the crates");
  // From anywhere, crates wherever, the passage (and the rune stone beside it) is still in reach.
  const home = traps(ice, "ice2", { goal: (s, h) => h[0] === 5 && h[1] === 8 });
  assert.equal(home.dead, 0, "the hero can be stranded");
});

test("the log run: a log rolled the wrong way is lost (until the rune stone), in both rooms", () => {
  const logs = map("trial_logs");
  for (const id of ["logs1", "logs2"]) {
    const m = measure(logs, id), tr = traps(logs, id);
    assert.ok(m.solvable, `${id} unsolvable`);
    assert.ok(m.pushes >= 3, `${id}: ${m.pushes} rolls`);
    assert.ok(tr.traps >= 0.4, `${id}: only ${Math.round(tr.traps * 100)}% of states are dead ends`);
  }
});
