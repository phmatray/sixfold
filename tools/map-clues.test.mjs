// Pins the secret clues of #228 against the real map JSON.
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const map = (n) => JSON.parse(readFileSync(new URL(`../web/world/maps/${n}.json`, import.meta.url), "utf8"));
const strings = (o) => (typeof o === "string" ? [o] : o && typeof o === "object" ? Object.values(o).flatMap(strings) : []);

test("Edda stands on the crater floor within 3 tiles of the arrival path", () => {
  const k = map("kilnmouth");
  const edda = k.npcs.find((n) => n.id === "edda");
  assert.ok(edda.z >= 25);
  const near = [];
  for (let z = 26; z <= 29; z++) [...k.tiles[z]].forEach((c, x) => c === "p" && near.push([x, z]));
  assert.ok(near.some(([x, z]) => Math.max(Math.abs(x - edda.x), Math.abs(z - edda.z)) <= 3));
  assert.equal(k.tiles[edda.z][edda.x], ".");
  assert.ok(!k.props.some((p) => p.x === edda.x && p.z === edda.z));
});

test("Edda does not speak from above the crater floor", () => {
  const edda = map("kilnmouth").npcs.find((n) => n.id === "edda");
  assert.doesNotMatch(strings(edda.dialogue).join("\n"), /down on the crater floor|en bas,/);
});

test("the crossroads signpost names the Swap Shop and its orchard", () => {
  const s = map("overworld").interactions.find((i) => i.kind === "sign" && i.x === 28 && i.z === 18);
  assert.match(s.text, /Swap Shop/);
  assert.match(s.text, /orchard/);
  assert.match(s.text_fr, /Bourse aux cartes/);
  assert.match(s.text_fr, /verger/);
});

test("Hazel promises no hum", () => {
  const hazel = map("woodland").npcs.find((n) => JSON.stringify(n.duelist ?? "").includes("Hivevolt"));
  assert.ok(hazel);
  assert.doesNotMatch(strings(hazel).join("\n"), /\bhum\b|bourdonn/i);
});
