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
  change(maps);
  const dir = mkdtempSync(join(tmpdir(), "ld-"));
  mkdirSync(join(dir, "maps"));
  copyFileSync(join(WEB, "props.js"), join(dir, "props.js"));
  writeFileSync(join(dir, "atlas.json"), JSON.stringify({
    regions: [{ id: "dale", name: "Dale", name_fr: "Vallon", color: "#557744" }, { id: "moor", name: "Moor", name_fr: "Lande", color: "#776655" }],
    maps: [{ id: "vale", name: "Vale", name_fr: "Val", region: vale.region, x: 1, y: 2 }, { id: "fell", name: "Fell Moor", name_fr: "Lande du Fell", region: fell.region, x: 1, y: 1 }],
    links: [], waymarks: [],
  }));
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

test("#225: a grade out of range, or not a number, fails even without --strict; a gentle one passes", () => {
  const grade = (k, g) => (m) => (m.vale[k] = { ...m.vale[k], grade: g });
  passes(world(grade("light", { exposure: 1.05, saturation: 0.08, contrast: 0.05 })));
  passes(world(grade("light_night", { exposure: 1.2 })));
  const bad = (change) => {
    const r = run(world(change));
    assert.notEqual(r.code, 0, r.out);
    assert.match(r.out, /vale: .*grade saturation and contrast are -0\.5 to 0\.5, exposure 0\.5 to 2/, r.out);
  };
  bad(grade("light", { saturation: 2 }));
  bad(grade("light_night", { exposure: 3 }));
  bad(grade("light_dusk", { contrast: "0.1" }));
});
