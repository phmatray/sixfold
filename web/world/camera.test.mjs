// node --test web/world/camera.test.mjs — the camera presets.
import test from "node:test";
import assert from "node:assert/strict";
import { ANGLES, PRESETS, framing, nextAngle, lowestPitch } from "./camera.js";

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
