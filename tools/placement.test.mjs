// The placement DSL (tools/placement.mjs): it parses, its predicates read a map the way the rules mean, and the
// fixes it proposes hold, and the shipped rules (tools/placement.rules) name only assets that exist.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { parseRules, world, check, evaluate, tables, proposals } from "./placement.mjs";

const T = { FOOT: { house: [2, 2] }, DECK: { boardwalk: [0.1], pier: [0.3], dock: [0.3] }, WALKABLE: new Set(["rug"]), tall: (a) => ({ house: 4, crates: 0.9 })[a] ?? 1 };
// 6 x 5, a path down column 1, water in the east column, a 1 m ledge on the bottom row.
const MAP = {
  width: 6, height: 5, light: {}, kind: "town",
  tiles: [".p...~", ".p...~", ".p...~", ".p...~", "111111"],
  props: [
    { asset: "lantern", x: 2, z: 1, rot: 0 },
    { asset: "lantern", x: 3, z: 2, rot: 0 },
    { asset: "house", x: 3, z: 0, rot: 0 },
    { asset: "crates", x: 0, z: 4, rot: 0 },
    { asset: "crates", x: 2, z: 4, rot: 0, y: 1 },
  ],
  warps: [{ x: 4, z: 1, to: "inside" }],
};
const w = world("test", MAP, T);
const ask = (src, p) => evaluate(w, p, parseRules(`rule r warn "r"\n for *\n need ${src}`).rules[0].need[0].e);
const [lampA, lampB, house, sunk, ledge] = MAP.props;

test("tags expand and rules read their clauses", () => {
  const { tags, rules } = parseRules(`
    tag path = tile:p paint:p   # a comment
    tag lit  = #path | lantern
    rule lamp.path error "a lantern lights a path"
      for lantern
      when outdoor
      unless has(why)
      need near(2, #lit)
      need count(3, lantern) == 0`);
  assert.deepEqual(tags.lit, [{ tile: "p" }, { paint: "p" }, { asset: "lantern" }]);
  const r = rules[0];
  assert.equal(r.severity, "error");
  assert.deepEqual(r.for, [{ asset: "lantern" }]);
  assert.equal(r.when.length, 2);
  assert.deepEqual(r.need.map((n) => n.src), ["near(2, #lit)", "count(3, lantern) == 0"]);
});

test("a bad rule fails with its line", () => {
  assert.throws(() => parseRules(`rule a warn "x"\n for lantern\n need nearby(2, tile:p)`), /line 3: unknown predicate nearby/);
  assert.throws(() => parseRules(`rule a warn "x"\n for #nope\n need true`), /line 2: unknown tag #nope/);
  assert.throws(() => parseRules(`rule a fatal "x"`), /severity/);
  assert.throws(() => parseRules(`rule a warn "x"\n for lantern`), /needs a "for"/);
});

test("on, near, front and beside read tiles, props and builtins", () => {
  assert.equal(ask("near(1, tile:p)", lampA), true);
  assert.equal(ask("near(1, tile:p)", lampB), false);
  assert.equal(ask("near(2, tile:p)", lampB), true);
  assert.equal(ask("near(1, lantern)", lampA), true, "the other lantern is diagonal to it");
  assert.equal(ask("on(tile:p)", lampA), false);
  assert.equal(ask("front(house)", lampA), false);
  assert.equal(ask("beside(house)", lampA), true, "the house's west column is beside it");
  assert.equal(ask("front(door)", house), false);
  assert.equal(ask("near(0, door)", house), true, "the door is on its footprint");
  assert.equal(ask("touches(tile:~) or near(1, edge)", house), true);
  assert.equal(ask("front(open)", lampB), true);
  assert.equal(ask("is(lantern | crates)", house), false);
});

test("count counts a prop once, dist measures to the nearest", () => {
  assert.equal(ask("count(1, house) == 1", lampA), true);
  assert.equal(ask("count(2, lantern) == 1", lampA), true, "itself excluded");
  assert.equal(ask("dist(tile:~) == 2", lampB), true);
  assert.equal(ask("dist(tile:~) == 0", house), false);
});

test("grounded: a prop rests on the floor under it", () => {
  assert.equal(ask("grounded", lampA), true);
  assert.equal(ask("grounded", sunk), false, "y 0 on a 1 m ledge is buried");
  assert.equal(ask("grounded", ledge), true);
  assert.equal(ask("flat", house), true);
});

test("check names the first need that fails", () => {
  const { rules } = parseRules(`rule lamp.water warn "by the water"\n for lantern\n need near(1, tile:p)\n need near(1, tile:~)`);
  const found = check(w, rules);
  assert.deepEqual(found.map((f) => [f.prop.x, f.prop.z, f.need]), [[2, 1, "near(1, tile:~)"], [3, 2, "near(1, tile:p)"]]);
});

test("fixes: a sunk prop is lifted, a stray one moved to the nearest place its rules hold", () => {
  const { rules } = parseRules(`
    rule ground warn "rests"
      for *
      need grounded
    rule lamp warn "by a path"
      for lantern
      need near(1, tile:p)`);
  const m = structuredClone(MAP), v = world("test", m, T);
  const [lift] = proposals(v, m.props[3], rules);
  assert.deepEqual([lift.kind, lift.patch.y], ["lift", 1]);
  const lamp = proposals(v, m.props[1], rules);
  assert.equal(lamp[0].kind, "move");
  assert.deepEqual([lamp[0].patch.x, lamp[0].patch.z], [2, 2], "one tile west, beside the path");
  assert.equal(lamp.at(-1).kind, "remove");
  assert.deepEqual(m, MAP, "trying fixes leaves the map as it was");
  assert.deepEqual(proposals(v, m.props[2], rules).map((p) => p.kind), [], "the house breaks nothing");
});

test("the shipped rules parse and name only assets that exist", () => {
  const { tags, rules } = parseRules(readFileSync(new URL("./placement.rules", import.meta.url), "utf8"));
  const names = new Set(Object.keys(tables().FOOT));
  for (const f of readdirSync(new URL("../web/world/maps/", import.meta.url)))
    for (const p of JSON.parse(readFileSync(new URL(`../web/world/maps/${f}`, import.meta.url), "utf8")).props) names.add(p.asset);
  const named = new Set();
  const walk = (e) => { if (Array.isArray(e)) e.forEach(walk); else if (e && typeof e === "object") { if (typeof e.asset === "string") named.add(e.asset); Object.values(e).forEach(walk); } };
  walk([Object.values(tags), rules.map((r) => [r.for, r.when, r.need])]);
  const unknown = [...named].filter((a) => !names.has(a));
  assert.deepEqual(unknown, [], `placement.rules names assets no map uses: ${unknown.join(", ")}`);
});
