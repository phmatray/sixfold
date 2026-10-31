// node --test web/world/tier.test.mjs — the graphics tier (#225).
import test from "node:test";
import assert from "node:assert/strict";
import { TIERS, KEY, pickTier, downgrade, readSaved } from "./tier.js";

test("Auto, nothing saved or an unknown choice: High, or Low on a weak device", () => {
  for (const saved of ["auto", null, "", "ultra"]) {
    assert.equal(pickTier(saved, false), "high");
    assert.equal(pickTier(saved, true), "low");
  }
});

test("a chosen tier is kept, whatever the device", () => {
  for (const t of TIERS) {
    assert.equal(pickTier(t, false), t);
    assert.equal(pickTier(t, true), t);
  }
});

test("a tier steps down one at a time and stops at Off", () => {
  assert.equal(downgrade("high"), "low");
  assert.equal(downgrade("low"), "off");
  assert.equal(downgrade("off"), "off");
});

test("the saved choice is read from sixfold.graphics, and storage that throws reads as nothing", () => {
  assert.equal(KEY, "sixfold.graphics");
  globalThis.localStorage = { getItem: (k) => (k === KEY ? "off" : null) };
  assert.equal(readSaved(), "off");
  globalThis.localStorage = { getItem: () => { throw new Error("blocked"); } };
  assert.equal(readSaved(), null);
  delete globalThis.localStorage;
  assert.equal(readSaved(), null);
});
