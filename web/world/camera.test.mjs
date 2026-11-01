// node --test web/world/camera.test.mjs — the camera presets, the room clamp and the lean-in.
import test from "node:test";
import assert from "node:assert/strict";
import { ANGLES, PRESETS, framing, nextAngle, lowestPitch, clampToRoom, leanOf } from "./camera.js";

const out = { fov: 40, fovPortrait: 50, pitch: 50, distance: 12, distancePortrait: 15, lookAtOffset: [0, 0.75, 0], clamp: false };

test("the default angle, an unknown one and rooms keep the map's own camera", () => {
  assert.equal(framing(out, "default", false), out);
  assert.equal(framing(out, "nope", false), out);
  assert.equal(framing(out, "higher", true), out);
});

test("the lower angle drops the pitch and pulls the camera in", () => {
  const c = framing(out, "lower", false);
  assert.equal(c.pitch, 40);
  assert.ok(Math.abs(c.distance - 10.56) < 1e-9);
  assert.ok(Math.abs(c.distancePortrait - 13.2) < 1e-9);
  assert.equal(c.fov, 40);
  assert.equal(c.fovPortrait, 50);
  assert.equal(c.lookAtOffset, out.lookAtOffset);
});

test("the angles cycle default, higher, lower", () => {
  assert.deepEqual(ANGLES, ["default", "higher", "lower"]);
  assert.equal(nextAngle("default"), "higher");
  assert.equal(nextAngle("higher"), "lower");
  assert.equal(nextAngle("lower"), "default");
  assert.equal(nextAngle("nope"), "default");
});

test("the lowest pitch is 40 and no preset shows the sky in portrait", () => {
  assert.equal(lowestPitch(), 40);
  for (const p of Object.values(PRESETS)) assert.ok(p.pitch - out.fovPortrait / 2 >= 10, `pitch ${p.pitch}`);
});

// A wide room seen from the default angle: the view is about 15.5 m across (halfW ≈ 7.8).
const room = { dist: 12, pitch: (50 * Math.PI) / 180, fov: 40, aspect: 16 / 9, cover: 0, offY: 0.75, width: 30, height: 30, heroZ: 15, wall: 0, hold: false };
const halfW = 12 * Math.tan((20 * Math.PI) / 180) * (16 / 9);

test("a room wider than the view keeps the view inside its side walls", () => {
  assert.ok(Math.abs(clampToRoom(-5, 15, room)[0] - halfW) < 1e-9);
  assert.ok(Math.abs(clampToRoom(99, 15, room)[0] - (30 - halfW)) < 1e-9);
  assert.equal(clampToRoom(15, 15, room)[0], 15);
});

test("a room narrower than the view stays whole on screen while the camera follows", () => {
  const small = { ...room, width: 6 };
  assert.equal(clampToRoom(3, 15, small)[0], 3);
  for (const x of [-20, 20]) {
    const [cx] = clampToRoom(x, 15, small);
    assert.ok(cx - halfW <= 1e-9 && cx + halfW >= 6 - 1e-9, `x ${x} → ${cx}`);
  }
});

test("a scene's shot may show past the walls by `wall`", () => {
  assert.ok(Math.abs(clampToRoom(-5, 15, { ...room, wall: 0.6 })[0] - (halfW - 0.6)) < 1e-9);
});

test("following the hero, the view leans no deeper than keeps the hero above the bottom edge; a shot holds its depth", () => {
  const deep = { ...room, height: 60, heroZ: 4 };
  const [, follow] = clampToRoom(15, 99, deep), [, hold] = clampToRoom(15, 99, { ...deep, hold: true });
  assert.ok(follow < hold, `${follow} < ${hold}`);
  assert.ok(clampToRoom(15, 99, { ...deep, heroZ: 30 })[1] > follow, "a hero further back lets the view go deeper");
});

test("the camera leans in for a scene, closer on an indoor place, less on a tall phone, a little when resting", () => {
  const lean = (o) => leanOf({ scene: false, place: false, tall: false, resting: false, reduced: false, ...o });
  assert.equal(lean({}), 1);
  assert.equal(lean({ resting: true }), 0.88);
  assert.equal(lean({ resting: true, reduced: true }), 1);
  assert.equal(lean({ scene: true }), 0.85);
  assert.equal(lean({ scene: true, tall: true }), 0.95);
  assert.equal(lean({ scene: true, place: true }), 0.55);
  assert.equal(lean({ scene: true, place: true, tall: true }), 0.8);
});
