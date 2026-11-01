// The dressing props (#233): each exported model holds to the asset contract (web/world/ASSETS.md) and carries
// the parts the game hooks onto (the windmill's turning sails, the waterfall's falling water, the lit lanterns).
// Rebuild them with tools/blender/build_dressing.py.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, existsSync, statSync } from "node:fs";

const ASSETS = new URL("../web/world/assets/", import.meta.url);
const PROPS = ["cliff_falls", "lantern_bunting", "washing_line", "water_lilies", "leaf_litter", "windmill", "vegetable_patch",
  "flower_planter", "blossom_tree", "well"];

// The glTF JSON chunk of a .glb, and its triangle count.
function gltf(name) {
  const b = readFileSync(new URL(`${name}.glb`, ASSETS));
  const j = JSON.parse(b.subarray(20, 20 + b.readUInt32LE(12)).toString("utf8"));
  let tris = 0;
  for (const m of j.meshes ?? [])
    for (const p of m.primitives) tris += j.accessors[p.indices ?? p.attributes.POSITION].count / 3;
  return { j, tris, materials: (j.materials ?? []).map((m) => m.name) };
}

for (const name of PROPS)
  test(`${name}.glb exists, within budget`, () => {
    const f = new URL(`${name}.glb`, ASSETS);
    assert.ok(existsSync(f), `${name}.glb is missing: run tools/blender/build_dressing.py`);
    assert.ok(statSync(f).size <= 2 * 1024 * 1024, `${name}.glb is over 2 MB`);
    const { j, tris } = gltf(name);
    assert.equal(j.scenes.length, 1);
    assert.ok(tris <= 3000, `${name}: ${tris} triangles (> 3k)`);
  });

test("the windmill's sails turn on a Loop clip", () => {
  const { j } = gltf("windmill");
  const loop = (j.animations ?? []).find((a) => a.name === "Loop");
  assert.ok(loop, "no Loop clip");
  const targets = new Set(loop.channels.map((c) => j.nodes[c.target.node].name));
  assert.deepEqual([...targets], ["sails"]);
});

test("the waterfall falls, its pool ripples", () => {
  const { materials } = gltf("cliff_falls");
  assert.ok(materials.some((m) => /fall_water/.test(m)), `materials: ${materials}`);
  assert.ok(materials.some((m) => /water/.test(m) && !/fall/.test(m)), `materials: ${materials}`);
});

test("the bunting's lanterns light at night", () => {
  assert.ok(gltf("lantern_bunting").materials.some((m) => /glow/.test(m)));
});

test("the well stays instanced: no animated water in it", () => {
  assert.ok(!gltf("well").materials.some((m) => /water/.test(m)));
});

// The game's footprints (props.js FOOT, read the way tools/check-maps.mjs reads it): a missing entry makes a
// prop block one tile and stand off-centre.
test("props.js knows every footprint larger than a tile", () => {
  const src = readFileSync(process.env.PROPS_JS ?? new URL("../web/world/props.js", import.meta.url), "utf8");
  const FOOT = eval(`(${src.match(/export const FOOT = (\{[\s\S]*?\});/)[1]})`);
  const want = { cliff_falls: [2, 1], lantern_bunting: [3, 1], washing_line: [3, 1], windmill: [3, 3], vegetable_patch: [3, 2], blossom_tree: [2, 2], well: [2, 2] };
  for (const [name, wd] of Object.entries(want)) assert.deepEqual(FOOT[name], wd, name);
});

test("the hero walks under the bunting and over the litter", () => {
  const src = readFileSync(new URL("../web/world/props.js", import.meta.url), "utf8");
  const WALKABLE = eval(src.match(/export const WALKABLE = (new Set\(\[[\s\S]*?\]\));/)[1]);
  for (const name of ["lantern_bunting", "leaf_litter"]) assert.ok(WALKABLE.has(name), name);
});

test("only the windmill carries a clip", () => {
  for (const name of PROPS.filter((n) => n !== "windmill")) assert.equal((gltf(name).j.animations ?? []).length, 0, name);
});
