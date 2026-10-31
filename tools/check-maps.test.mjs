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
    regions: [{ id: "dale", name: "Dale", name_fr: "Vallon", color: "#557744" }],
    maps: [{ id: "vale", name: "Vale", name_fr: "Val", region: "dale", x: 1, y: 2 }, { id: "fell", name: "Fell Moor", name_fr: "Lande du Fell", region: "dale", x: 1, y: 1 }],
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
