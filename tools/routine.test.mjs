// node --test tools/routine.test.mjs — the NPC routine scheduler (web/world/routine.js, #113).
import test from "node:test";
import assert from "node:assert/strict";
import { activityAt, plan, seedOf, minuteOf } from "../web/world/routine.js";

const DIR = { n: [0, -1], s: [0, 1], e: [1, 0], w: [-1, 0] };
// A 10 x 10 grid from rows of "." (open) and "#" (solid).
const gridOf = (rows, door) => ({
  width: rows[0].length, height: rows.length,
  open: (x, z) => rows[z]?.[x] === ".",
  door: (name) => door?.[name] ?? null,
});
const walk = (g, [x, z], steps) => {
  for (const f of steps) {
    [x, z] = [x + DIR[f][0], z + DIR[f][1]];
    assert.ok(g.open(x, z), `the path walks onto ${x},${z}, which is solid`);
  }
  return [x, z];
};

const phases = [{ from: "dawn", do: "work" }, { from: "day", do: "sit" }, { from: "dusk", do: "talk" }, { from: "night", do: "idle" }];
const clock = [{ from: "06:00", do: "work" }, { from: "12:00", do: "sit" }, { from: "21:00", home: "bakery" }];

test("times read as HH:MM or a phase name; anything else is not a time", () => {
  assert.equal(minuteOf("06:30"), 390);
  assert.equal(minuteOf("dusk"), 1080);
  for (const bad of ["24:00", "6:00", "noon", undefined]) assert.ok(Number.isNaN(minuteOf(bad)), String(bad));
});

test("activityAt changes exactly at the phase edges", () => {
  assert.equal(activityAt(phases, 479).do, "work");
  assert.equal(activityAt(phases, 480).do, "sit");
  assert.equal(activityAt(phases, 1079).do, "sit");
  assert.equal(activityAt(phases, 1080).do, "talk");
  assert.equal(activityAt(phases, 1260).do, "idle");
});

test("activityAt carries the evening's activity across midnight", () => {
  assert.ok(activityAt(clock, 23 * 60).home);
  assert.ok(activityAt(clock, 2 * 60).home);
  assert.equal(activityAt(clock, 6 * 60).do, "work");
  assert.equal(activityAt(clock, 1440 + 13 * 60).do, "sit"); // a minute past the day wraps
  assert.ok(activityAt(phases, 200).do === "idle"); // before dawn: still night
});

test("plan walks round a wall to the activity's tile", () => {
  const g = gridOf([
    "..........",
    "..........",
    "..........",
    "#######...",
    "..........",
    "..........",
    "..........",
    "..........",
    "..........",
    "..........",
  ]);
  const npc = { x: 1, z: 1, def: { routine: [{ from: "06:00", at: [1, 8], face: "n", do: "work" }] } };
  const p = plan(npc, 600, g);
  assert.equal(p.act.do, "work");
  assert.deepEqual(p.to, [1, 8]);
  assert.deepEqual(walk(g, [1, 1], p.steps), [1, 8]);
  assert.equal(p.steps.length, 6 + 7 + 6, "the shortest way round, not through");
});

test("plan sends a person home to the tile beside its door, and nowhere on arrival", () => {
  const rows = Array.from({ length: 10 }, (_, z) => (z === 5 ? "####......" : ".........."));
  const g = gridOf(rows, { bakery: [2, 5] });
  const npc = { x: 8, z: 0, def: { routine: clock } };
  const p = plan(npc, 22 * 60, g);
  assert.ok(p.act.home);
  const end = walk(g, [8, 0], p.steps);
  assert.equal(Math.abs(end[0] - 2) + Math.abs(end[1] - 5), 1, `ends beside the door, at ${end}`);
  const now = plan(npc, 22 * 60, g, true);
  assert.equal(now.steps, null, "on entering the map nobody is seen walking");
});

test("plan places the person when there's no way there, and on entering the map", () => {
  const g = gridOf(Array.from({ length: 10 }, (_, z) => (z === 5 ? "##########" : "..........")));
  const npc = { x: 1, z: 1, def: { routine: [{ from: "day", at: [1, 8], do: "idle" }] } };
  assert.deepEqual(plan(npc, 600, g), { act: npc.def.routine[0], to: [1, 8], steps: null });
  assert.deepEqual(plan({ ...npc, x: 1, z: 7 }, 600, g, true).steps, null);
  assert.deepEqual(plan({ ...npc, x: 1, z: 7 }, 600, g).steps, ["s"]);
});

test("each person's seed sets its own idle phase, look interval and pause", () => {
  const a = seedOf("tansy"), b = seedOf("posy");
  assert.notDeepEqual(a, b);
  assert.deepEqual(seedOf("tansy"), a, "the same id gives the same seed");
  for (const s of [a, b]) {
    assert.ok(s.phase >= 0 && s.phase < 1);
    assert.ok(s.look >= 2 && s.look <= 4);
    assert.ok(s.pause >= 0.6 && s.pause <= 1.4);
  }
});
