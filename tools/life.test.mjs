// node --test tools/life.test.mjs — the ambient-life schedule (#115): calm places stay calm, micro-events
// keep their distance, and no more than the budget is ever animated in view.
import test from "node:test";
import assert from "node:assert/strict";
import { lifeSchedule, calmAt, boatAt, BUDGET, GAP, CRITTERS } from "../web/world/life_schedule.js";

const CALM = [30, 2, 38, 10];
const map = (life = {}) => ({
  width: 40, height: 30,
  tiles: Array.from({ length: 30 }, () => ".".repeat(40)),
  life: {
    critters: [{ kind: "rabbit", zone: [2, 2, 20, 20], n: 6 }, { kind: "squirrel", zone: [25, 0, 39, 15], n: 6 }, { kind: "hedgehog", zone: [2, 2, 20, 20], n: 3 }],
    perch: [{ at: [[10, 10, 1], [11, 10, 1], [12, 10, 1]] }, { at: [[33, 5, 2], [34, 5, 2]] }],
    leaves: { under: ["birch"], rate: 0.3 },
    ...life,
  },
  calm: [CALM],
});
// The hero walks a slow loop round the map, through the calm rect and past the critters.
const walk = (t) => ({ x: 20 + Math.cos(t / 40) * 15, z: 14 + Math.sin(t / 40) * 11, night: 0 });
const simulate = (m, hero, minutes = 10, dt = 0.25) => {
  const mem = {}, out = [];
  for (let t = 0; t < minutes * 60; t += dt) out.push({ t, h: hero(t), ...lifeSchedule(m, t, hero(t), mem) });
  return out;
};

test("no spawn ever stands inside a calm rect", () => {
  for (const f of simulate(map(), walk)) for (const s of f.spawns) assert.ok(!calmAt(map(), s.x, s.z), `t=${f.t}: ${s.kind} at ${s.x},${s.z}`);
});

test("micro-events are one at a time and at least 20 s apart", () => {
  const starts = [...new Set(simulate(map(), walk).filter((f) => f.event).map((f) => f.event.t0))];
  assert.ok(starts.length >= 5, `only ${starts.length} events in 10 minutes`);
  for (let i = 1; i < starts.length; i++) assert.ok(starts[i] - starts[i - 1] >= GAP, `${starts[i - 1]} then ${starts[i]}`);
});

test("never more than the budget animated in view", () => {
  const busy = map({ critters: [{ kind: "rabbit", zone: [0, 0, 39, 29], n: 80 }] });
  const frames = simulate(busy, () => ({ x: 20, z: 15, night: 0 }), 2);
  assert.ok(frames.some((f) => f.spawns.length === BUDGET), "the budget is reached");
  for (const f of frames) assert.ok(f.spawns.length <= BUDGET, `${f.spawns.length} at t=${f.t}`);
});

test("standing in a calm rect for 3 minutes: no critter, bird or event near", () => {
  for (const f of simulate(map(), () => ({ x: 34, z: 6, night: 0 }), 3)) {
    assert.equal(f.event, null, `t=${f.t}: ${f.event?.kind}`);
    for (const s of f.spawns) assert.ok(!calmAt(map(), s.x, s.z));
  }
});

test("a critter bolts when the hero comes within 2.5 tiles, and comes back 20 to 40 s later", () => {
  const one = map({ critters: [{ kind: "rabbit", zone: [5, 5, 5, 5], n: 1 }], perch: [] });
  const mem = {};
  const far = { x: 10, z: 10, night: 0 }, near = { x: 7, z: 5, night: 0 };
  const rabbits = (t, hero) => lifeSchedule(one, t, hero, mem).spawns.filter((s) => s.id !== "cross"); // not a micro-event's
  assert.equal(rabbits(0, far).length, 1);
  assert.ok(rabbits(1, near)[0].bolt >= 0, "bolting");
  assert.equal(rabbits(3, near).length, 0, "hidden");
  assert.equal(rabbits(19, far).length, 0, "still hidden");
  assert.equal(rabbits(41, far).length, 1, "back");
});

test("day kinds sleep at night, night kinds wake", () => {
  const kinds = (night) => new Set(simulate(map({ perch: [] }), (t) => ({ ...walk(t), night }), 1).flatMap((f) => f.spawns.map((s) => s.kind)));
  assert.ok(kinds(0).has("rabbit") && !kinds(0).has("hedgehog"));
  assert.ok(kinds(1).has("hedgehog") && !kinds(1).has("rabbit"));
  for (const k of Object.values(CRITTERS)) assert.ok(["day", "night"].includes(k.when));
});

test("no life block: nothing", () => {
  const f = lifeSchedule({ width: 10, height: 10, tiles: [] }, 100, { x: 5, z: 5, night: 0 }, {});
  assert.deepEqual(f, { spawns: [], event: null });
});

test("a boat comes in, waits at the pier and leaves, on its cycle", () => {
  assert.equal(boatAt(90, 0).k, 1); // at the far end of its lane
  assert.equal(boatAt(90, 30).k, 0); // tied up
  assert.ok(boatAt(90, 60).k > 0 && boatAt(90, 60).on); // sailing out
  assert.equal(boatAt(90, 85).on, false); // out of sight
  assert.equal(boatAt(90, 120).k, boatAt(90, 30).k); // the cycle repeats
});

test("a micro-event playing when the hero steps into a calm rect ends there", () => {
  const mem = {}, out = { x: 25, z: 20, night: 0 }, calm = { x: 34, z: 6, night: 0 };
  let t = 0;
  while (!lifeSchedule(map(), t, out, mem).event) t += 0.25;
  assert.equal(lifeSchedule(map(), t + 0.25, calm, mem).event, null);
});
