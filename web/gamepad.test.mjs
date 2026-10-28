// node --test web/gamepad.test.mjs — the pure part of web/gamepad.js.
import test from "node:test";
import assert from "node:assert/strict";
import { mapButtons, translateKey, checkBind, label, KEY_DEFAULTS, PAD_DEFAULTS } from "./gamepad.js";

const pad = (down = [], axes = [0, 0]) => ({ buttons: Array.from({ length: 17 }, (_, i) => ({ pressed: down.includes(i), value: down.includes(i) ? 1 : 0 })), axes });
const sorted = (a) => [...a].sort();

test("face buttons and Start map to their actions", () => {
  assert.deepEqual(sorted(mapButtons(pad([0, 1, 9]))), ["act", "cancel", "menu"]);
  assert.deepEqual(mapButtons(pad([])), []);
});

test("R3 and C are the camera angle by default, remappable but never to a reserved key (#148)", () => {
  assert.deepEqual(mapButtons(pad([11])), ["camera"]);
  assert.equal(label("pad", "camera", false), "R3");
  assert.equal(label("keys", "camera", false), "C");
  assert.equal(translateKey("v", { ...KEY_DEFAULTS, camera: "v" }), "c");
  assert.equal(checkBind({ keys: KEY_DEFAULTS, pad: PAD_DEFAULTS }, "keys", "camera", "Tab"), "reserved");
  assert.equal(checkBind({ keys: KEY_DEFAULTS, pad: PAD_DEFAULTS }, "keys", "act", "c"), "taken:camera");
});

test("shoulders switch tabs, the D-pad gives directions", () => {
  assert.deepEqual(sorted(mapButtons(pad([4, 13]))), ["down", "tabPrev"]);
  assert.deepEqual(mapButtons(pad([5])), ["tabNext"]);
});

test("the stick walks past the dead zone and runs when pushed all the way", () => {
  assert.deepEqual(mapButtons(pad([], [0.3, 0])), []);
  assert.deepEqual(mapButtons(pad([], [-0.7, 0.2])), ["left"]);
  assert.deepEqual(sorted(mapButtons(pad([], [0.1, 1]))), ["down", "run"]);
});

test("remapped pad buttons follow the bindings", () => {
  const b = { ...PAD_DEFAULTS, act: 2, run: 0 };
  assert.deepEqual(sorted(mapButtons(pad([2]), b)), ["act"]);
  assert.deepEqual(sorted(mapButtons(pad([0]), b)), ["run"]);
});

test("plain numbers count as button values too", () => {
  assert.deepEqual(mapButtons({ buttons: [1, 0], axes: [] }), ["act"]);
});

test("a remapped key plays as its default; a freed default key is dropped", () => {
  const keys = { ...KEY_DEFAULTS, act: "f" };
  assert.equal(translateKey("f", keys), "e");
  assert.equal(translateKey("F", keys), "e");
  assert.equal(translateKey("e", keys), null);
  assert.equal(translateKey("x", keys), undefined);
  assert.equal(translateKey("e"), undefined);
});

test("two actions trading keys each play as their own default", () => {
  const keys = { ...KEY_DEFAULTS, act: "f", journal: "e" };
  assert.equal(translateKey("e", keys), "j");
  assert.equal(translateKey("j", keys), null);
});

test("Escape always stays itself", () => {
  assert.equal(translateKey("Escape", { ...KEY_DEFAULTS, cancel: "x", menu: "x" }), undefined);
  assert.equal(translateKey("x", { ...KEY_DEFAULTS, cancel: "x" }), "Escape");
});

test("conflicts and reserved controls are refused", () => {
  const binds = { keys: { ...KEY_DEFAULTS }, pad: { ...PAD_DEFAULTS } };
  assert.equal(checkBind(binds, "keys", "act", "m"), "taken:map");
  assert.equal(checkBind(binds, "keys", "act", "M"), "taken:map");
  assert.equal(checkBind(binds, "keys", "act", "ArrowUp"), "reserved");
  assert.equal(checkBind(binds, "keys", "act", "w"), "reserved");
  assert.equal(checkBind(binds, "keys", "act", "Escape"), "reserved");
  assert.equal(checkBind(binds, "keys", "act", "f"), "");
  assert.equal(checkBind(binds, "keys", "menu", "Escape"), ""); // shared with cancel on a keyboard
  assert.equal(checkBind(binds, "pad", "act", 1), "taken:cancel");
  assert.equal(checkBind(binds, "pad", "act", 12), "reserved");
  assert.equal(checkBind(binds, "pad", "act", 4), "reserved");
  assert.equal(checkBind(binds, "pad", "act", 5), "reserved");
  assert.equal(checkBind(binds, "pad", "act", 7), "");
});

test("prompts name the control for the device, in both languages", () => {
  assert.equal(label("keys", "act", false), "E");
  assert.equal(label("keys", "run", true), "Maj");
  assert.equal(label("keys", "menu", false), "Esc");
  assert.equal(label("keys", "walk", true), "Flèches ou ZQSD");
  assert.equal(label("pad", "act", false), "A");
  assert.equal(label("pad", "menu", false), "Start");
  assert.equal(label("pad", "walk", true), "Stick ou croix");
  assert.equal(label("touch", "act", false), "");
  assert.equal(label("keys", "confirm", true), "Entrée");
  assert.equal(label("pad", "confirm", false), "A");
  const binds = { keys: { ...KEY_DEFAULTS, act: "f" }, pad: { ...PAD_DEFAULTS, act: 3 } };
  assert.equal(label("keys", "act", false, binds), "F");
  assert.equal(label("pad", "act", false, binds), "Y");
});
