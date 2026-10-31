// node --test tools/transition.test.mjs — the biome-transition tool (#118): its output is the same on every run,
// keeps off paths and the exit's throat, and passes check-maps --strict. Runs on a copy of the real world.
import test from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { cpSync, mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { bandOf, grids, bfs, edgeTiles } from "../web/world/ld.js";

const TOOL = new URL("./transition.mjs", import.meta.url).pathname;
const CHECK = new URL("./check-maps.mjs", import.meta.url).pathname;
const WEB = new URL("../web/world/", import.meta.url).pathname;
const node = (...args) => spawnSync(process.execPath, args, { encoding: "utf8" });

test("AC2: cinderwick east writes the same props twice, off paths and the throat, cuts nothing off, and passes --strict", () => {
  const dir = mkdtempSync(join(tmpdir(), "band-"));
  for (const f of ["maps", "props.js", "atlas.json"]) cpSync(join(WEB, f), join(dir, f), { recursive: true });
  const cinder = join(dir, "maps", "cinderwick.json");
  const tool = (file, side) => {
    const r = node(TOOL, file, side);
    assert.equal(r.status, 0, r.stdout + r.stderr);
  };
  tool(cinder, "e");
  const once = readFileSync(cinder, "utf8");
  tool(cinder, "e");
  assert.equal(readFileSync(cinder, "utf8"), once, "a second run changed the map");

  const m = JSON.parse(once), b = bandOf(m, m.transitions.find((t) => t.to === "downs"));
  const placed = m.props.filter((p) => p.band === "downs");
  assert.ok(placed.length >= 3, `only ${placed.length} props placed`);
  for (const p of placed) {
    assert.ok(b.scatter.includes(p.asset), `${p.asset} is not in the band's scatter`);
    assert.ok(!"pr=H".includes(m.tiles[p.z][p.x]) && !"pr".includes(m.paint?.[p.z]?.[p.x] ?? "-"), `${p.asset} ${p.x},${p.z} is on a path`);
    assert.ok(p.z < b.span[0] - 2 || p.z > b.span[1] + 2, `${p.asset} ${p.x},${p.z} is in the throat`);
  }

  // Every tile the hero reached before the band still reaches (bar the ones the band now stands on).
  const src = readFileSync(join(dir, "props.js"), "utf8"), table = (re) => eval(`(${src.match(re)[1]})`);
  const T = { FOOT: table(/export const FOOT = (\{[\s\S]*?\});/), DECK: table(/export const DECK = (\{[\s\S]*?\});/), WALKABLE: new Set(), tall: () => 1 };
  const reached = (map) => {
    const g = grids(map, T), open = new Set();
    for (let z = 0; z < map.height; z++) for (let x = 0; x < map.width; x++) if (!g.solid.has(`${x},${z}`)) open.add(`${x},${z}`);
    const step = (ax, az, bx, bz) => Math.abs((g.floor[`${ax},${az}`] ?? 0) - (g.floor[`${bx},${bz}`] ?? 0)) <= 0.6;
    return new Set(bfs(open, [[map.spawn.x, map.spawn.z], ...map.exits.flatMap((e) => edgeTiles(map, e))], step).dist.keys());
  };
  const after = reached(m), on = new Set(placed.map((p) => `${p.x},${p.z}`));
  for (const k of reached({ ...m, props: m.props.filter((p) => p.band !== "downs") })) assert.ok(after.has(k) || on.has(k), `the band cut ${k} off`);

  tool(join(dir, "maps", "downs.json"), "w"); // the band back, or check-maps fails the pair
  const r = node(CHECK, dir, "--strict");
  assert.equal(r.status, 0, r.stdout + r.stderr);
});
