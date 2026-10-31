// node --test tools/ld.test.mjs — the level-design analysis (web/world/ld.js).
import test from "node:test";
import assert from "node:assert/strict";
import { grids, visible, blocker, approach, pathGraph, walkTime, scatter, density, bandOf, bandWeight } from "../web/world/ld.js";

const T = { FOOT: { cottage: [5, 4] }, DECK: {}, WALKABLE: new Set(), tall: (a) => ({ pine: 3.8, bush: 0.7, cottage: 4.5 })[a] ?? 1 };
const blank = (w, h, rows = {}) => Array.from({ length: h }, (_, z) => rows[z] ?? ".".repeat(w));
const map = (props, tiles = blank(20, 20)) => ({ width: tiles[0].length, height: tiles.length, tiles, props, camera: { pitch: 50, distance: 12 } });

test("a pine two tiles south of a door hides it at the real camera pitch; a bush doesn't", () => {
  const house = { asset: "cottage", x: 8, z: 4, rot: 0 }; // door on its front row, z = 7
  const door = { x: 10, z: 7, y: 2, self: house };
  const from = { x: 10, z: 8 };
  for (const [asset, seen] of [["pine", false], ["bush", true]]) {
    const g = grids(map([house, { asset, x: 10, z: 9, rot: 0 }]), T);
    assert.equal(visible(map([]), g.boxes, from, door), seen, asset);
    // The overlay highlights the box that blocks the ray: the pine's.
    assert.equal(blocker(map([]), g.boxes, from, door)?.prop.asset, seen ? undefined : "pine", asset);
  }
  // North of the door (behind the house) the pine hides nothing.
  const g = grids(map([house, { asset: "pine", x: 10, z: 2, rot: 0 }]), T);
  assert.equal(visible(map([]), g.boxes, from, door), true);
});

test("a warp is checked from the open tile south of it, at a door's height", () => {
  const g = grids(map([]), T), open = new Set(["10,8", "11,7"]);
  const a = approach({ x: 10, z: 7, tier: "major", o: {} }, g, open, () => 0, T.FOOT);
  assert.deepEqual([a.from.x, a.from.z, a.target.y], [10, 8, 1]);
  assert.equal(approach({ x: 3, z: 3, tier: "major", o: {} }, g, open, () => 0, T.FOOT), null);
});

test("a 1-wide dead-end branch is found, with its length back to the junction", () => {
  const rows = { 10: "p".repeat(20) };
  for (let z = 4; z < 10; z++) rows[z] = ".".repeat(12) + "p" + ".".repeat(7);
  const { deadEnds, junctions } = pathGraph(grids(map([], blank(20, 20, rows)), T));
  assert.ok(junctions.some((j) => j.x === 12 && j.z === 10));
  const branch = deadEnds.find((d) => d.x === 12);
  assert.deepEqual([branch.x, branch.z], [12, 4]);
  assert.ok(branch.length > 3, `length ${branch.length}`);
  // A loose piece of path, with no junction, measures its own length: 4 tiles is short enough, 8 isn't.
  const piece = (n) => pathGraph(grids(map([], blank(20, 20, { 16: "..." + "p".repeat(n) + ".".repeat(17 - n) })), T)).deadEnds;
  assert.ok(piece(4).length && piece(4).every((d) => d.length <= 3), JSON.stringify(piece(4)));
  assert.ok(piece(8).some((d) => d.length === 7), JSON.stringify(piece(8)));
});

test("walking 40 tiles takes 10 s", () => assert.equal(walkTime(40), 10));

test("an evenly spaced grid scores a scatter ratio over 1.1, a clump under 1", () => {
  const grid = [];
  for (let x = 0; x < 5; x++) for (let z = 0; z < 5; z++) grid.push({ x: x * 3, z: z * 3, asset: "bush" });
  assert.ok(scatter(grid).ratio > 1.1, `grid ${scatter(grid).ratio}`);
  const clump = [];
  for (const [cx, cz] of [[0, 0], [18, 0], [0, 18], [18, 18]]) for (const [dx, dz] of [[0, 0], [1, 0], [0, 1], [1, 1]]) clump.push({ x: cx + dx, z: cz + dz, asset: "bush" });
  assert.ok(scatter(clump).ratio < 1, `clump ${scatter(clump).ratio}`);
  assert.equal(scatter(clump).chunks[0].share, 1);
});

test("density counts props a screen at a time along the route", () => {
  const route = Array.from({ length: 39 }, (_, x) => [x, 5]);
  const d = density([{ x: 2, z: 5 }, { x: 3, z: 4 }, { x: 27, z: 6 }], route);
  assert.deepEqual(d.curve, [2, 0, 1]);
  assert.deepEqual(d.breathers, [1]);
  assert.ok(d.cv > 0.25);
});

test("#118: a transition band is 1 at the edge, 0 at its depth, and fades out past the exit's span plus its depth", () => {
  const m = { width: 40, height: 30, exits: [{ side: "n", span: [18, 20], to: "a" }, { side: "s", span: [18, 20], to: "b" }, { side: "e", span: [10, 12], to: "c" }, { side: "w", span: [10, 12], to: "d" }] };
  const w = (to, x, z) => bandWeight(m, bandOf(m, { to, depth: 5 }), x, z);
  assert.equal(bandOf(m, { to: "nowhere", depth: 5 }), undefined);
  // Into the map from each edge, on the exit's line: 1, 0.5, then nothing from `depth` on.
  for (const [to, at] of [["a", (d) => [19, d]], ["b", (d) => [19, 30 - d]], ["c", (d) => [40 - d, 11]], ["d", (d) => [d, 11]]]) {
    assert.equal(w(to, ...at(0)), 1, to);
    assert.equal(w(to, ...at(2.5)), 0.5, to);
    assert.equal(w(to, ...at(5)), 0, to);
    assert.equal(w(to, ...at(7)), 0, to);
  }
  // Along the north edge: full over 13..26 (span 18..20 plus 5 either side), half 2.5 tiles past it, gone 5 past.
  assert.equal(w("a", 13, 0), 1);
  assert.equal(w("a", 26, 0), 1);
  assert.equal(w("a", 10.5, 0), 0.5);
  assert.equal(w("a", 28.5, 0), 0.5);
  assert.equal(w("a", 8, 0), 0);
  assert.equal(w("a", 31, 0), 0);
});
