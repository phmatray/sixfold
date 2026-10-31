// node --test tools/check-maps.test.mjs — check-maps' level-design rules (#110), through the real CLI on small
// fixture worlds: a v2 village (vale) with a cottage, its room (vale_in) and the moor north of it (fell).
import test from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, mkdirSync, copyFileSync, writeFileSync, readFileSync, readdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const CLI = new URL("./check-maps.mjs", import.meta.url).pathname;
const WEB = new URL("../web/world/", import.meta.url).pathname;

// A 26 x 18 tile grid closed by a wall, `open` tiles cut into it ([x, z, char]).
const grid = (cut) => {
  const rows = Array.from({ length: 18 }, (_, z) => [...(z === 0 || z === 17 ? "#".repeat(26) : `#${".".repeat(24)}#`)]);
  for (const [x, z, c] of cut) rows[z][x] = c;
  return rows.map((r) => r.join(""));
};
const outdoor = (o) => ({ region: "dale", kind: "village", music: "overworld", light: { indoor: false }, interactions: [], npcs: [], warps: [], ...o });

function world(change = () => {}) {
  const vale = outdoor({
    title: "Vale", title_fr: "Val", level_design: 2, width: 26, height: 18, spawn: { x: 12, z: 5, facing: "s" },
    tiles: grid([
      ...[11, 12, 13].map((x) => [x, 0, x < 13 ? "p" : "."]),
      ...Array.from({ length: 15 }, (_, z) => [[11, z + 1, "p"], [12, z + 1, "p"]]).flat(),
      ...Array.from({ length: 6 }, (_, i) => [[11 + i, 14, "p"], [11 + i, 15, "p"]]).flat(),
    ]),
    props: [{ asset: "cottage", x: 14, z: 10, rot: 0 }, { asset: "tree", x: 4, z: 4, rot: 0, landmark: true }],
    interactions: [{ kind: "sign", x: 14, z: 3, text: "North: Fell Moor", text_fr: "Nord : Lande du Fell" }],
    warps: [{ x: 16, z: 13, to: "vale_in", label: "Go in", label_fr: "Entrer", spawn: { x: 5, z: 7, facing: "n" } }],
    exits: [{ side: "n", span: [11, 13], to: "fell" }],
  });
  const fell = outdoor({
    title: "Fell Moor", title_fr: "Lande du Fell", width: 26, height: 18, spawn: { x: 12, z: 13, facing: "n" },
    tiles: grid([...[11, 12, 13].map((x) => [x, 17, x < 13 ? "p" : "."]), ...Array.from({ length: 5 }, (_, z) => [[11, z + 12, "p"], [12, z + 12, "p"]]).flat()]),
    props: [],
    interactions: [{ kind: "sign", x: 14, z: 14, text: "South: Vale", text_fr: "Sud : Val" }],
    exits: [{ side: "s", span: [11, 13], to: "vale" }],
  });
  const room = {
    width: 11, height: 9, light: { indoor: true }, props: [], spawn: { x: 5, z: 7, facing: "n" },
    tiles: ["#".repeat(11), ...Array.from({ length: 7 }, () => `#${".".repeat(9)}#`), "#".repeat(11)],
    warps: [{ x: 5, z: 7, dir: "s", to: "vale", label: "Go out", label_fr: "Sortir", spawn: { x: 16, z: 14, facing: "s" } }],
  };
  const maps = { vale, fell, vale_in: room };
  const atlas = {
    regions: [{ id: "dale", name: "Dale", name_fr: "Vallon", color: "#557744" }, { id: "moor", name: "Moor", name_fr: "Lande", color: "#776655" }],
    maps: [{ id: "vale", name: "Vale", name_fr: "Val", region: "dale", x: 1, y: 2 }, { id: "fell", name: "Fell Moor", name_fr: "Lande du Fell", region: "dale", x: 1, y: 1 }],
    links: [{ a: "vale", b: "fell" }], waymarks: [],
  };
  change(maps, atlas);
  for (const e of atlas.maps) if (maps[e.id]) e.region = maps[e.id].region;
  const dir = mkdtempSync(join(tmpdir(), "ld-"));
  mkdirSync(join(dir, "maps"));
  copyFileSync(join(WEB, "props.js"), join(dir, "props.js"));
  writeFileSync(join(dir, "atlas.json"), JSON.stringify(atlas));
  for (const [id, m] of Object.entries(maps)) writeFileSync(join(dir, "maps", `${id}.json`), JSON.stringify(m));
  return dir;
}
const run = (dir, ...args) => {
  const r = spawnSync(process.execPath, [CLI, dir, ...args], { encoding: "utf8" });
  return { code: r.status, out: r.stdout + r.stderr, stdout: r.stdout };
};
const fails = (dir, name) => {
  const r = run(dir, "--strict");
  assert.notEqual(r.code, 0, r.out);
  assert.match(r.out, new RegExp(`vale: \\[${name}\\]`), r.out);
};
const passes = (dir) => {
  const r = run(dir, "--strict");
  assert.equal(r.code, 0, r.out);
};

test("AC1: the real world passes --strict unchanged", () => passes(WEB));

test("the fixture world passes --strict", () => passes(world()));

test("AC2: a v2 map with no landmark fails; a visible one passes", () => {
  fails(world((m) => delete m.vale.props[1].landmark), "landmark");
  passes(world());
});

test("AC3: a pine two tiles south of a door hides it; north of the door it doesn't", () => {
  fails(world((m) => m.vale.props.push({ asset: "pine", x: 16, z: 15, rot: 0 })), "hidden-mandatory");
  passes(world((m) => m.vale.props.push({ asset: "pine", x: 16, z: 8, rot: 0 })));
});

test("#148: outdoors, depth stacking is checked at the lowest camera angle", () => {
  // A bush (0.7 m) a tile in front of a sign passed at 50° and hides it at 40°; two tiles in front, it doesn't.
  const r = run(world((m) => m.vale.props.push({ asset: "bush", x: 14, z: 4, rot: 0 })), "--strict");
  assert.notEqual(r.code, 0, r.out);
  assert.match(r.out, /vale: bush 14,4 stands 1 tile in front of sign at 14,3 and hides it from the camera at 40°/);
  passes(world((m) => m.vale.props.push({ asset: "bush", x: 14, z: 5, rot: 0 })));
});

test("AC4: a pointless dead end fails; a view at its end passes", () => {
  const branch = (m) => {
    for (let x = 13; x <= 19; x++) m.vale.tiles[7] = m.vale.tiles[7].slice(0, x) + "p" + m.vale.tiles[7].slice(x + 1);
  };
  fails(world(branch), "dead-end");
  passes(world((m) => (branch(m), (m.vale.view = [19, 7]))));
});

test("AC4: an exit hidden from the inside fails", () => {
  fails(world((m) => [11, 12, 13].forEach((x) => m.vale.props.push({ asset: "pine", x, z: 3, rot: 0 }))), "exit-both-ways");
});

test("AC4: an unreachable 4-tile pocket fails; a vista over it passes", () => {
  const wall = (m) => {
    const rows = m.vale.tiles.map((r) => [...r]);
    for (let x = 19; x <= 22; x++) for (let z = 2; z <= 5; z++) if (x === 19 || x === 22 || z === 2 || z === 5) rows[z][x] = "#";
    m.vale.tiles = rows.map((r) => r.join(""));
  };
  fails(world(wall), "unreachable");
  passes(world((m) => (wall(m), (m.vale.vista = [[20, 3, 21, 4]]))));
});

test("AC4: a warp landing on its own way back fails; a moved landing tile passes", () => {
  // On the room's mat facing out: one push and the hero is back outside.
  fails(world((m) => (m.vale.warps[0].spawn = { x: 5, z: 7, facing: "s" })), "connection");
  passes(world((m) => (m.vale.warps[0].spawn = { x: 5, z: 6, facing: "s" })));
  // Back outside on the door itself.
  fails(world((m) => (m.vale_in.warps[0].spawn = { x: 16, z: 13, facing: "s" })), "connection");
});

// Ambient life (#115): a map's `life` and `calm`.
const lively = (life, calm) => (m) => Object.assign(m.vale, { life, ...(calm ? { calm } : {}) });
const LIFE = { critters: [{ kind: "rabbit", zone: [2, 2, 8, 8], n: 2 }], perch: [{ at: [[15, 11, 2.5]] }], leaves: { under: ["tree"], rate: 0.3 } };
const lifeFails = (dir, what) => {
  const r = run(dir, "--strict");
  assert.notEqual(r.code, 0, r.out);
  assert.match(r.out, new RegExp(`vale: \\[(life|calm)\\] ${what}`), r.out);
};

test("ambient life: a sound life block and calm rect pass", () => passes(world(lively(LIFE, [[2, 2, 5, 5]]))));

test("AC6: ambient life fails a zone off the map, a perch on a missing prop and an unknown critter kind", () => {
  lifeFails(world(lively({ critters: [{ kind: "rabbit", zone: [20, 2, 30, 8], n: 2 }] })), "the rabbit zone \\[20,2,30,8\\] is not");
  lifeFails(world(lively({ perch: [{ at: [[6, 6, 1]] }] })), "perch 6,6 is on no prop");
  lifeFails(world(lively({ critters: [{ kind: "dragon", zone: [2, 2, 8, 8] }] })), "critter kind dragon");
  lifeFails(world(lively({ leaves: { under: ["birch"], rate: 0.3 } })), "leaves");
  lifeFails(world(lively({ boats: [{ pier: [6, 6] }] })), "boat pier 6,6 is not on a deck");
  lifeFails(world(lively({}, [[0, 0, 40, 4]])), "\\[0,0,40,4\\] is not");
});

test("a map without level_design only lists the rules (no error)", () => passes(world((m) => (delete m.vale.level_design, delete m.vale.props[1].landmark))));

const outdoorMaps = readdirSync(join(WEB, "maps")).filter((f) => !JSON.parse(readFileSync(join(WEB, "maps", f), "utf8")).light.indoor);

test("AC5: --report prints one section per outdoor map, with its measures, and exits 0 with advisories", () => {
  const r = run(WEB, "--report");
  assert.equal(r.code, 0, r.out);
  const sections = r.stdout.split("\n## ").slice(1);
  assert.equal(sections.length, outdoorMaps.length);
  for (const s of sections)
    for (const line of ["- landmark:", "- POIs:", "- longest walk without a discovery:", "- scatter: ratio", "- density along the main path"]) assert.ok(s.includes(line), `${s.split("\n")[0]} lacks ${line}`);
  assert.match(r.stdout, /## woodland .*\n[\s\S]*?- POIs: \d+ major, \d+ secondary, \d+ micro, \d+ secret/);
  assert.match(r.stdout, /\[poi-budget\]/);
});

test("--report=<file> writes the report there", () => {
  const file = join(mkdtempSync(join(tmpdir(), "ld-")), "report.md");
  assert.equal(run(world(), `--report=${file}`).code, 0);
  assert.match(readFileSync(file, "utf8"), /^# Level design report\n[\s\S]*\n## vale — Vale \(village\)\n/);
});

test("AC6: a changed budget changes the report, with no code change", () => {
  const woodland = (out) => out.split("\n## ").find((s) => s.startsWith("woodland"));
  const before = woodland(run(WEB, "--report").stdout);
  // Fernhollow Trail is the reference map (#112): it meets the route budget, so a stricter one flags it.
  assert.doesNotMatch(before, /\[poi-budget\] \d+ secondary POIs/);
  const budgets = JSON.parse(readFileSync(new URL("./level-design.json", import.meta.url), "utf8"));
  budgets.route.secondary = [9, 9];
  const file = join(mkdtempSync(join(tmpdir(), "ld-")), "level-design.json");
  writeFileSync(file, JSON.stringify(budgets));
  const after = woodland(run(WEB, "--report", `--budgets=${file}`).stdout);
  assert.match(after, /\[poi-budget\] \d+ secondary POIs \(9 for a route\)/);
  assert.match(after, /budget: secondary 9/);
});

test("AC5 (#113): a routine on a solid tile or on a duelist fails; a good one passes", () => {
  const ann = (routine, more = {}) => (m) => m.vale.npcs.push({ id: "ann", name: "Ann", x: 12, z: 8, facing: "s", routine, ...more });
  const good = [{ from: "06:00", at: [12, 12], face: "s", do: "work" }, { from: "night", home: "vale_in" }];
  passes(world(ann(good)));
  const bad = (change, msg) => {
    const r = run(world(change), "--strict");
    assert.notEqual(r.code, 0, r.out);
    assert.match(r.out, msg, r.out);
  };
  bad(ann([{ from: "06:00", at: [14, 10], do: "work" }]), /vale: ann's routine at 06:00 stands on 14,10, which is blocked/);
  bad(ann(good, { duelist: { sight: 2 } }), /vale: ann duels, so it keeps no routine/);
  bad(ann([{ from: "25:00", at: [12, 12] }]), /"25:00" is not a time/);
  bad(ann([{ from: "dusk", home: "nowhere" }]), /goes home to nowhere, but no door here leads there/);
});

test("#114: a bark needs its say_fr, a condition string and 60 characters at most; a good one passes", () => {
  const bark = (b) => (m) => m.vale.npcs.push({ id: "ann", name: "Ann", x: 12, z: 8, facing: "s", barks: [b] });
  passes(world(bark({ if: "seals:3", say: "Three seals already!", say_fr: "Déjà trois sceaux !" })));
  const bad = (change, msg) => {
    const r = run(world(change), "--strict");
    assert.notEqual(r.code, 0, r.out);
    assert.match(r.out, msg, r.out);
  };
  bad(bark({ if: "seals:3", say: "Three seals already!" }), /vale: ann's bark 0 has no say_fr/);
  bad(bark({ if: 3, say: "Hi.", say_fr: "Salut." }), /vale: ann's bark 0: its if is a condition string/);
  bad(bark({ say: "x".repeat(61), say_fr: "Salut." }), /vale: ann's bark 0 is over 60 characters/);
});

test("AC7 (#114): an open on a warp that is no door, and a lit on a model without windows, fail; a shop's hours and a lit cottage pass", () => {
  passes(world((m) => ((m.vale.warps[0].open = "hour:8-19"), (m.vale.props[0].lit = "hour:17-23"))));
  const bad = (change, msg) => {
    const r = run(world(change), "--strict");
    assert.notEqual(r.code, 0, r.out);
    assert.match(r.out, msg, r.out);
  };
  bad((m) => (m.vale_in.warps[0].open = "hour:8-19"), /vale_in: the warp to vale at 5,7 has an open, but only a door keeps hours/);
  bad((m) => (m.vale.props[1].lit = "hour:17-23"), /vale: tree at 4,4 is lit, but has no windows/);
});

// #117: the world map is checked against the real maps, and roads and rivers line up across an edge.
const put = (row, x, s) => row.slice(0, x) + s + row.slice(x + s.length);
// A third outdoor map, Tarn, behind the cottage's back door: a road vale - tarn through a room.
const tarn = (cond) => (m, a) => {
  const gate = cond ? { if: cond, locked: "Shut.", locked_fr: "Fermé." } : {};
  m.tarn = outdoor({ title: "Tarn", title_fr: "Lac", width: 26, height: 18, spawn: { x: 12, z: 9, facing: "s" }, tiles: grid([]), props: [],
    warps: [{ x: 12, z: 8, to: "vale_in", label: "Go in", label_fr: "Entrer", spawn: { x: 5, z: 2, facing: "s" }, ...gate }] });
  m.vale_in.warps.push({ x: 5, z: 1, dir: "n", to: "tarn", label: "Go out", label_fr: "Sortir", spawn: { x: 12, z: 9, facing: "s" }, ...gate });
  a.maps.push({ id: "tarn", name: "Tarn", name_fr: "Lac", region: "dale", x: 3, y: 1 });
};
const atlasSays = (dir) => run(dir).out.split("\n").filter((l) => l.includes("[atlas-"));

test("AC1 (#117): an atlas without a real road names exactly that road; one with a road that isn't there fails", () => {
  passes(world());
  const missing = atlasSays(world(tarn()));
  assert.equal(missing.length, 1, missing.join("\n"));
  assert.match(missing[0], /atlas: \[atlas-links\] the road tarn - vale is in the maps but not in atlas.json/);
  assert.deepEqual(atlasSays(world((m, a) => (tarn()(m, a), a.links.push({ a: "vale", b: "tarn" })))), []);
  const extra = atlasSays(world((m, a) => (tarn()(m, a), a.links.push({ a: "vale", b: "tarn" }, { a: "fell", b: "tarn" }))));
  assert.equal(extra.length, 1, extra.join("\n"));
  assert.match(extra[0], /atlas: \[atlas-links\] the road fell - tarn is in atlas.json, but no exit, door or ferry joins them/);
  assert.match(atlasSays(world((m, a) => a.links.push({ a: "fell", b: "vale" }))).join("\n"), /atlas: \[atlas-links\] the road fell - vale is listed twice/);
});

test("#117: a gated road's if and icon match the gate that holds it", () => {
  const road = (l) => (m, a) => (tarn("item:wading_boots")(m, a), a.links.push({ a: "vale", b: "tarn", ...l }));
  assert.deepEqual(atlasSays(world(road({ if: "item:wading_boots", icon: "boots" }))), []);
  assert.match(atlasSays(world(road({})))[0], /\[atlas-gate\] the road vale - tarn has no if, but "item:wading_boots" holds it/);
  assert.match(atlasSays(world(road({ if: "item:wading_boots", icon: "gloves" })))[0], /\[atlas-gate\] the road vale - tarn shows gloves, but its item wading_boots is drawn boots/);
  assert.match(atlasSays(world((m, a) => Object.assign(a.links[0], { if: "item:ferry_pass", icon: "ferry" })))[0], /\[atlas-gate\] the road vale - fell has if "item:ferry_pass", but nothing holds it/);
  assert.match(atlasSays(world((m, a) => (a.links[0].icon = "boots")))[0], /\[atlas-gate\] the road vale - fell shows boots, but no item holds it/);
  // A second, lantern-locked door into the cottage: the free one still makes the road through it free.
  const lantern = { x: 17, z: 13, to: "vale_in", label: "Go in", label_fr: "Entrer", spawn: { x: 5, z: 7, facing: "n" }, if: "item:kiln_lantern", locked: "Dark.", locked_fr: "Noir." };
  assert.deepEqual(atlasSays(world((m, a) => (tarn()(m, a), m.vale.warps.unshift(lantern), a.links.push({ a: "vale", b: "tarn" })))), []);
});

test("AC2 (#117): a road that leaves an edge 3 tiles off from where it arrives fails; 1 tile off passes", () => {
  const shift = (at) => (m) => {
    m.vale.exits[0].span = m.fell.exits[0].span = [11, 16];
    m.vale.tiles[0] = put(m.vale.tiles[0], 13, "....");
    m.fell.tiles[17] = put(put(m.fell.tiles[17], 11, "......"), at, "pp");
  };
  fails(world(shift(14)), "road-offset");
  passes(world(shift(12)));
});

test("AC2 (#117): a river that stops at an edge fails; one that runs on, or ends with a water_ends note, passes", () => {
  const river = (m) => [0, 1, 2, 3].forEach((z) => (m.vale.tiles[z] = put(m.vale.tiles[z], 16, "~")));
  fails(world(river), "water-edge");
  passes(world((m) => (river(m), (m.vale.exits[0].water_ends = "a fall"))));
  passes(world((m) => (river(m), [16, 17].forEach((z) => (m.fell.tiles[z] = put(m.fell.tiles[z], 16, "~"))))));
});

// #117: drops you can only go down, and shortcuts you open from the far side.
const drop = (more = {}, cue = true) => (m, a) => {
  m.vale.drops = [{ from: [8, 8], to: [8, 9], why: "a broken fence over the scree", ...more }];
  if (cue) m.vale.props.push({ asset: "rock", x: 9, z: 8, rot: 0 });
  (a.marks ??= []).push({ kind: "drop", map: "vale" });
};
const says = (dir, msg) => {
  const r = run(dir, "--strict");
  assert.notEqual(r.code, 0, r.out);
  assert.match(r.out, msg, r.out);
};

test("AC3 (#117): a good drop passes; one with no why, or no cue, fails", () => {
  passes(world(drop()));
  says(world(drop({ why: undefined })), /vale: \[drop\] the drop 8,8 > 8,9 needs a why/);
  says(world(drop({}, false)), /vale: \[drop\] the drop 8,8 > 8,9 has no cue \(a prop or paint\) within 1 tile/);
  // Path paint beside it is the path, no cue; a slide mark (sand paint) is.
  const paint = (c) => (m, a) => (drop({}, false)(m, a), (m.vale.paint = m.vale.tiles.map((r, z) => (z === 8 ? put(".".repeat(r.length), 7, c) : ".".repeat(r.length)))));
  says(world(paint("p")), /vale: \[drop\] the drop 8,8 > 8,9 has no cue/);
  assert.doesNotMatch(run(world(paint("s")), "--strict").out, /has no cue/);
  says(world((m, a) => (drop()(m, a), (m.vale_in.drops = [{ from: [3, 3], to: [3, 4], why: "a step" }]))), /vale_in: \[drop\] the drop 3,3 > 3,4 is indoors/);
  says(world(drop({ to: [8, 10] })), /vale: \[drop\] the drop 8,8 > 8,10 must join two open tiles side by side/);
});

test("AC3 (#117): a drop that is the only way out of an area fails", () => {
  // A walled pit below the drop: once down, nothing leads out.
  const pit = (m, a) => {
    drop()(m, a);
    for (let z = 9; z <= 12; z++) for (let x = 6; x <= 10; x++) if (!(x === 8 && z === 9) && (x === 6 || x === 10 || z === 9 || z === 12)) m.vale.tiles[z] = put(m.vale.tiles[z], x, "#");
  };
  says(world(pit), /vale: \[drop\] below the drop 8,8 > 8,9, nothing reaches an exit, a warp or a waymark without it \(a soft-lock\)/);
  // The pit's way out is a shortcut in its south wall: lifted from inside, it frees the pit; from outside (the far side,
  // never reached from below), it doesn't.
  const hatch = (from) => (m, a) => {
    pit(m, a);
    m.vale.tiles[12] = put(m.vale.tiles[12], 8, ".");
    m.vale.gates = [{ id: "hatch", tiles: [[8, 12]], if: "flag:sc_vale_hatch", name: "the hatch", name_fr: "la trappe", text: "Barred.", text_fr: "Barré.",
      shortcut: { id: "vale_hatch", from, label: "Lift the bar", label_fr: "Lever la barre" } }];
    a.marks.push({ kind: "shortcut", map: "vale", id: "vale_hatch" });
  };
  assert.doesNotMatch(run(world(hatch([8, 11])), "--strict").out, /below the drop/);
  says(world(hatch([8, 13])), /vale: \[drop\] below the drop 8,8 > 8,9, nothing reaches an exit, a warp or a waymark without it \(a soft-lock\)/);
  // A walled pocket above the drop: its only way out is down.
  const ledge = (m, a) => (drop()(m, a), [[7, 8], [8, 7], [9, 8]].forEach(([x, z]) => (m.vale.tiles[z] = put(m.vale.tiles[z], x, "#"))));
  says(world(ledge), /vale: \[drop\] above the drop 8,8 > 8,9, nothing reaches an exit, a warp or a waymark without it \(a soft-lock\)/);
});

test("#117: when the hour shuts a gate, a drop is still a way out, taken downward", () => {
  // The ledge's way in is a gate open by day; at night the hero on it still gets out, down the drop.
  const day = (m, a) => {
    drop()(m, a);
    [[7, 8], [9, 8]].forEach(([x, z]) => (m.vale.tiles[z] = put(m.vale.tiles[z], x, "#")));
    m.vale.gates = [{ id: "stile", tiles: [[8, 7]], if: "time:day", name: "the stile", name_fr: "l'échalier", text: "Shut at night.", text_fr: "Fermé la nuit." }];
  };
  assert.doesNotMatch(run(world(day), "--strict").out, /once the timed gates shut/);
});

test("#117: a shortcut gate is checked: its flag, its far-side tile and its labels", () => {
  const bar = (sc = {}, more = {}) => (m, a) => {
    m.vale.tiles[9] = put(m.vale.tiles[9], 1, "#".repeat(10));
    m.vale.gates = [{ id: "bar", tiles: [[5, 9]], if: "flag:sc_vale_bar", name: "the barred gap", name_fr: "le passage barré",
      text: "Barred from the other side.", text_fr: "Barré de l'autre côté.", ...more,
      shortcut: { id: "vale_bar", from: [5, 10], label: "Lift the bar", label_fr: "Lever la barre", ...sc } }];
    (a.marks ??= []).push({ kind: "shortcut", map: "vale", id: "vale_bar" });
  };
  passes(world(bar()));
  says(world(bar({}, { if: "flag:vale_bar" })), /vale: \[shortcut\] gate bar opens on "flag:vale_bar", but its shortcut sets flag:sc_vale_bar/);
  says(world(bar({ from: [5, 12] })), /vale: \[shortcut\] gate bar: its from 5,12 is not beside the gate/);
  says(world(bar({ label_fr: undefined })), /vale: \[shortcut\] gate bar: its shortcut needs an id, a label and a label_fr/);
  // The wall across the whole width: the far side can't be reached to lift the bar.
  says(world((m, a) => (bar()(m, a), (m.vale.tiles[9] = put(m.vale.tiles[9], 1, "#".repeat(24))))), /vale: \[shortcut\] gate bar: its from 5,10 can't be reached the long way round/);
});

test("#117: the chart's marks match the maps' shortcuts and drops", () => {
  const none = (m, a) => (drop()(m, a), (a.marks = []));
  assert.match(atlasSays(world(none)).join("\n"), /atlas: \[atlas-marks\] vale has a drop, but atlas.json marks none/);
  assert.match(atlasSays(world((m, a) => (drop()(m, a), a.marks.push({ kind: "drop", map: "fell" })))).join("\n"), /atlas: \[atlas-marks\] atlas.json marks a drop on fell, which has none/);
});

test("#117: the report says when a village has no loop through its main path, and stops once a path rings a block", () => {
  const vale = (dir) => run(dir, "--report").stdout.split("\n## ").find((s) => s.startsWith("vale"));
  assert.match(vale(world()), /\[loop\] no loop/);
  // A lane round the grass east of the main street (x 13-16, z 4-9), through the main street.
  const ring = (m) => {
    m.vale.tiles[4] = put(m.vale.tiles[4], 13, "pppp");
    m.vale.tiles[9] = put(m.vale.tiles[9], 13, "pppp");
    for (let z = 5; z <= 8; z++) m.vale.tiles[z] = put(m.vale.tiles[z], 16, "p");
  };
  const s = vale(world(ring));
  assert.doesNotMatch(s, /\[loop\]/);
  assert.match(s, /- loop through the main path: \d+ tiles round 12 tiles of other ground/);
});

// Biome transitions (#118, WORLD_RULES 5.7): Fell Moor in another region, so the vale's north exit crosses a border.
const ROCKS = { vale: [[7, 2], [8, 3], [18, 2]], fell: [[7, 15], [8, 16], [18, 15]] };
const border = (depth = 5, back = true) => (m) => {
  m.fell.region = "moor";
  for (const [id, to] of [["vale", "fell"], ["fell", "vale"]]) {
    if (id === "fell" && !back) continue;
    m[id].transitions = [{ to, depth, scatter: ["rock"] }];
    for (const [x, z] of ROCKS[id]) m[id].props.push({ asset: "rock", x, z, rot: 0 });
  }
};

test("AC1 (#118): a region border with a 5-tile band on both sides passes", () => passes(world(border())));

// Each failure by its own message, so one rule can't pass on another's error.
const bandFails = (change, msg) => {
  const r = run(world(change), "--strict");
  assert.notEqual(r.code, 0, r.out);
  assert.match(r.out, msg, r.out);
};

test("AC1 (#118): a region-border exit with no transition fails", () =>
  bandFails((m) => (m.fell.region = "moor"), /vale: \[transition\] the n exit to fell crosses into moor with no transition band/));

test("AC1 (#118): a band 3 tiles deep fails", () => bandFails(border(3), /vale: \[transition\] the band to fell is 3 tiles deep; make it 4-6/));

test("AC1 (#118): a band whose exit back has none of its own fails", () => bandFails(border(5, false), /vale: \[transition\] the band to fell has no band back on fell/));

test("AC1 (#118): a band with under 3 props of its scatter fails", () =>
  bandFails((m) => (border()(m), m.vale.props.pop()), /vale: \[transition\] the band to fell holds 2 props of its scatter/));

test("AC1 (#118): a band toward a map no edge exit leads to fails", () =>
  bandFails((m) => (border()(m), m.vale.transitions.push({ to: "vale_in", depth: 5, scatter: ["rock"] })), /vale: \[transition\] the band to vale_in matches no edge exit/));

test("#118: a prop left behind by a band the map no longer has fails", () =>
  bandFails((m) => (border()(m), m.vale.props.push({ asset: "rock", x: 20, z: 8, rot: 0, band: "moor" })), /vale: \[transition\] rock 20,8 belongs to a band to moor, which the map no longer has/));

// Secrets (#116, WORLD_RULES §9): a well-formed one in the vale, a box on crates behind a hedge break, with a moss
// trail through the break as its clue, a neighbour who hints at it, and a hum by the box.
const secret = (m) => {
  m.vale.paint = m.vale.tiles.map((r) => ".".repeat(r.length));
  m.vale.paint[10] = m.vale.paint[10].slice(0, 7) + "ss" + m.vale.paint[10].slice(9);
  m.vale.props.push({ asset: "crates", x: 5, z: 10, rot: 0 }, { asset: "hedge_gap", x: 7, z: 10, rot: 90 });
  m.vale.npcs.push({ id: "ann", name: "Ann", x: 8, z: 6, facing: "s", dialogue: [{ lines: ["Old Tam hid a box west of here."], lines_fr: ["Le vieux Tam a caché une caisse à l'ouest."], do: ["flag:heard_old_box"] }] });
  m.vale.interactions.push({
    kind: "find", id: "v_box", x: 5, z: 10, give: "coins:10", hidden: true, text: "A box.", text_fr: "Une caisse.", poi: "secret",
    secret: { id: "old_box", kind: "spatial", clue: [[7, 10], [8, 10]], hint: "ann" }, sound: { at: [5, 10], sfx: "hum", radius: 3 },
  });
};
const box = (m) => m.vale.interactions.find((i) => i.id === "v_box");
const secretFails = (change, msg) => {
  const r = run(world((m) => (secret(m), change(m))), "--strict");
  assert.notEqual(r.code, 0, r.out);
  assert.match(r.out, msg, r.out);
};

test("#116: a well-formed secret (clue, hint, fed hedge gap, hum) passes", () => passes(world(secret)));

test("AC2 (#116): a secret with no clue and no hint fails", () => {
  secretFails((m) => (delete box(m).secret.clue, delete box(m).secret.hint), /vale: \[secret\] v_box's secret old_box has no clue and no hint/);
  secretFails((m) => (box(m).clue = [[7, 10]]), /vale: \[secret\] .*clue now lives in the secret block/);
  secretFails((m) => delete box(m).secret, /vale: \[secret\] v_box is poi: secret but has no secret block/);
});

test("AC2 (#116): a field item behind a secret fails, as does an exit reached only through a secret gate", () => {
  secretFails((m) => (box(m).give = "give:wading_boots"), /vale: \[behind-secret\] v_box's secret old_box gives wading_boots/);
  const gate = { id: "tam_door", name: "Tam's door", name_fr: "La porte de Tam", tiles: [[11, 0], [12, 0], [13, 0]], if: "flag:secret_old_box", text: "Shut.", text_fr: "Fermé." };
  secretFails((m) => (m.vale.gates = [gate]), /vale: \[behind-secret\] the exit n to fell is only reached through a secret/);
});

test("AC2 (#116): a hedge gap with no trail fails", () => {
  secretFails((m) => (m.vale.paint[10] = ".".repeat(26)), /vale: \[hedge-gap\] the hedge_gap at 7,10 has no trail/);
});

test("AC2 (#116): a heard_ flag no NPC sets fails, as does a hint whose NPC never sets it", () => {
  secretFails((m) => (m.vale.npcs[0].if = "flag:heard_nobody"), /vale: \[heard\] .*flag:heard_nobody is read, but no one's line sets it/);
  secretFails((m) => (m.vale.npcs[0].dialogue[0].do = []), /vale: \[secret\] .*hint ann never sets flag:heard_old_box/);
});

test("#116: a sound needs a secret, a tile on the map, sfx hum and a radius of 1..8", () => {
  secretFails((m) => (box(m).sound.radius = 12), /vale: \[sound\] v_box's sound/);
  secretFails((m) => (m.vale.interactions[0].sound = { at: [5, 10], sfx: "hum", radius: 3 }), /vale: \[sound\] .*only on a secret/);
});
