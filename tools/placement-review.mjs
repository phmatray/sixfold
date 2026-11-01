// Run: node tools/placement-review.mjs [web/world] [--port=8095]   then open http://localhost:8095
// The placement review app: every finding of tools/placement.rules, the map round it drawn from above (and in a
// simple 3D built from the same data), before and after a proposed fix: lift it onto the floor, turn it, or move it to
// the nearest place where its rules hold. Nothing is ever taken out, and only fixes that leave `check-maps --strict`
// passing are offered. Findings of one rule on props next to one another (2 tiles or closer, chained) come as one
// block, with one plan: each prop's best fix, each built on the ones before it.
// Confirm, and the fixes are written to the map's JSON and `check-maps --strict` runs on the result (Undo puts the
// file back). Reject, and the findings are answered "this is fine" in tools/placement.waivers.json, which placement.mjs
// skips. Dev tooling, local only: it writes into web/world/maps. Its labels are English, like the editor's.
import { createServer } from "node:http";
import { readFileSync, writeFileSync, readdirSync, existsSync, unlinkSync, mkdtempSync, mkdirSync, symlinkSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve, normalize, extname } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { parseRules, tables, world, check, proposals, place, waivers, WAIVERS } from "./placement.mjs";
import * as ld from "../web/world/ld.js";

const argv = process.argv.slice(2);
const dir = argv.find((a) => !a.startsWith("--")) ?? "web/world";
const port = +(argv.find((a) => a.startsWith("--port="))?.slice(7) ?? 8095);
const RULES = fileURLToPath(new URL("./placement.rules", import.meta.url));
const PAGE = fileURLToPath(new URL("./placement-review.html", import.meta.url));
const VENDOR = fileURLToPath(new URL("../web/vendor/three/build/", import.meta.url)); // three.js, for the 3D view
const t = tables(dir);
const file = (name) => `${dir}/maps/${name}.json`;
const names = () => readdirSync(`${dir}/maps`).map((f) => f.replace(".json", "")).sort();
const rules = () => parseRules(readFileSync(RULES, "utf8")).rules; // read fresh: the rules may change while the app runs
const load = (name) => world(name, JSON.parse(readFileSync(file(name), "utf8")), t);
const undo = []; // [{ path, text | null, what }]

// The findings, in blocks: one rule's findings on one map whose props stand 2 tiles or closer, chained.
function blocks() {
  const r = rules(), waived = waivers(), out = [];
  for (const name of names()) {
    const w = load(name), found = check(w, r, waived);
    const box = (p) => { const [fw, fd] = ld.foot(p, t.FOOT); return [p.x, p.z, p.x + fw - 1, p.z + fd - 1]; };
    const near = (a, b) => { const [ax0, az0, ax1, az1] = box(a.prop), [bx0, bz0, bx1, bz1] = box(b.prop); return Math.max(bx0 - ax1, ax0 - bx1, bz0 - az1, az0 - bz1, 0) <= 2; };
    const left = new Set(found);
    for (const f of found) {
      if (!left.has(f)) continue;
      const group = [f];
      left.delete(f);
      for (let i = 0; i < group.length; i++) for (const g of [...left]) if (g.rule === group[i].rule && near(group[i], g)) group.push(g), left.delete(g);
      group.sort((a, b) => a.prop.z - b.prop.z || a.prop.x - b.prop.x);
      const members = group.map(({ prop, ...g }) => ({ id: g.id, index: g.index, need: g.need, asset: prop.asset, x: prop.x, z: prop.z, rot: prop.rot ?? 0, y: prop.y ?? 0 }));
      out.push({ id: members[0].id, map: name, rule: f.rule, severity: f.severity, why: f.why, need: f.need, members });
    }
  }
  return out;
}

// What the page needs to draw a map: its tiles and paint, its props and what stands on them, and the props.js sizes.
function mapView(name) {
  const m = JSON.parse(readFileSync(file(name), "utf8"));
  return {
    name, title: m.title ?? name, kind: m.kind ?? "interior", region: m.region ?? null, indoor: !!m.light?.indoor,
    width: m.width, height: m.height, tiles: m.tiles, paint: m.paint ?? null, camera: m.camera ?? null,
    props: m.props.map((p) => ({ asset: p.asset, x: p.x, z: p.z, rot: p.rot ?? 0, y: p.y ?? 0, w: ld.foot(p, t.FOOT)[0], d: ld.foot(p, t.FOOT)[1], ground: t.WALKABLE.has(p.asset) || !!t.DECK[p.asset], tall: t.tall(p.asset) })),
    spots: (m.interactions ?? []).map((i) => ({ x: i.x, z: i.z, kind: i.kind })),
    doors: (m.warps ?? []).map((w) => ({ x: w.x, z: w.z, to: w.to })),
    npcs: (m.npcs ?? []).map((n) => ({ x: n.x, z: n.z, id: n.id })),
  };
}

// The prop at index, as the page saw it (asset and tile), or null when the file moved on since.
function propOf(m, { index, asset, x, z }) {
  const p = m.props[index];
  return p && p.asset === asset && p.x === x && p.z === z ? p : null;
}

// A fix on a map's JSON: the prop's new place, turn and height, the spots it carries moved with it, and its twins (a
// find's crates and opened crates, each with an `if`, on one tile) moved alongside.
function patchMap(m, p, patch) {
  const dx = (patch.x ?? p.x) - p.x, dz = (patch.z ?? p.z) - p.z;
  const twins = p.if ? m.props.filter((q) => q !== p && q.if && q.x === p.x && q.z === p.z) : [];
  for (const i of patch.spots ?? []) (m.interactions[i].x += dx), (m.interactions[i].z += dz);
  for (const q of [p, ...twins]) for (const k of ["x", "z", "rot", "y"]) if (k in patch) patch[k] === undefined || patch[k] === null ? delete q[k] : (q[k] = patch[k]);
}

// Fixes for several props of one map at once: items [{ index, asset, x, z, patch }], all written in one go.
function apply({ map, items }) {
  const path = file(map), text = readFileSync(path, "utf8"), m = JSON.parse(text);
  const props = items.map((i) => propOf(m, i));
  const stale = items.find((_, k) => !props[k]);
  if (stale) return { status: 409, body: { error: `${map} changed since: no ${stale.asset} at ${stale.x},${stale.z}. Reload.` } };
  items.forEach((i, k) => patchMap(m, props[k], i.patch));
  writeFileSync(path, JSON.stringify(m, null, 2) + "\n");
  undo.push({ path, text, what: items.length > 1 ? `${items.length} fixes on ${map}` : `${items[0].asset} ${items[0].x},${items[0].z} on ${map}` });
  return { status: 200, body: { done: true, check: checkMaps() } };
}

function reject({ map, rule, items, note }) {
  const text = existsSync(WAIVERS) ? readFileSync(WAIVERS, "utf8") : null, list = text ? JSON.parse(text) : [];
  for (const { asset, x, z } of items) list.push({ map, rule, asset, x, z, ...(note && { note }) });
  list.sort((a, b) => `${a.map} ${a.rule}`.localeCompare(`${b.map} ${b.rule}`) || a.x - b.x || a.z - b.z);
  writeFileSync(WAIVERS, JSON.stringify(list, null, 1) + "\n");
  undo.push({ path: WAIVERS, text, what: `the answer on ${items.length > 1 ? `${items.length} ${items[0].asset}` : `${items[0].asset} ${items[0].x},${items[0].z}`} (${rule})` });
  return { status: 200, body: { done: true } };
}

// check-maps over the whole world, as CI runs it (~0.3 s). Its findings are the lines naming a map (or the atlas).
function checkMaps(at = dir) {
  const r = spawnSync(process.execPath, [fileURLToPath(new URL("./check-maps.mjs", import.meta.url)), at, "--strict"], { encoding: "utf8" });
  const lines = `${r.stdout}${r.stderr}`.trim().split("\n").filter(Boolean);
  const said = lines.filter((l) => /^(Error: )?\s*[a-z_]+: /.test(l) && !l.startsWith("maps ok")).map((l) => l.replace(/^Error: /, "").trim());
  return { ok: r.status === 0, lines: lines.slice(-12), first: said[0] ?? lines.find((l) => l.startsWith("Error"))?.slice(7) ?? null };
}

// A world of links to the real one, where one map at a time is swapped for a fixed copy to see whether check-maps
// still passes, before a fix is ever offered.
const shadow = mkdtempSync(join(tmpdir(), "placement-review-"));
mkdirSync(`${shadow}/maps`);
for (const f of ["props.js", "atlas.json", "quests.json"]) if (existsSync(`${dir}/${f}`)) symlinkSync(resolve(`${dir}/${f}`), `${shadow}/${f}`);
for (const name of names()) symlinkSync(resolve(file(name)), `${shadow}/maps/${name}.json`);
process.on("exit", () => rmSync(shadow, { recursive: true, force: true }));
process.on("SIGINT", () => process.exit(0));
function holds(map, fixes) { // fixes: [{ index, patch }]
  const link = `${shadow}/maps/${map}.json`, m = JSON.parse(readFileSync(file(map), "utf8"));
  for (const f of fixes) patchMap(m, m.props[f.index], f.patch);
  unlinkSync(link);
  writeFileSync(link, JSON.stringify(m));
  try { return checkMaps(shadow); } finally { unlinkSync(link); symlinkSync(resolve(file(map)), link); }
}

// The fixes worth offering for one prop: those check-maps still passes, at most one lift, one turn and two moves.
function offered(map, index, w, p) {
  const out = [], room = { lift: 1, turn: 1, move: 2 };
  let blocked = null;
  for (const c of proposals(w, p, rules(), { max: 6 })) {
    if (!room[c.kind]) continue;
    const r = holds(map, [{ index, patch: c.patch }]);
    if (r.ok) out.push(c), room[c.kind]--;
    else blocked ??= `${c.label.toLowerCase()}: ${r.first}`;
  }
  return { proposals: out, blocked: out.length ? null : blocked };
}

// One plan for a block: each prop's first fix, found with the fixes before it already in place. Checked as a whole
// with check-maps; if that fails, rebuilt one prop at a time, keeping only the fixes check-maps still passes.
function plan(map, indices) {
  const r = rules();
  const build = (verify) => {
    const w = load(map), done = [], items = [];
    for (const index of indices) {
      const p = w.m.props[index], item = { index, asset: p.asset, x: p.x, z: p.z, fix: null, why: null };
      items.push(item);
      const cands = proposals(w, p, r, { max: verify ? 4 : 1 });
      if (!cands.length) { item.why = "no placement within 10 tiles keeps its rules"; continue; }
      for (const c of cands) {
        const res = verify ? holds(map, [...done, { index, patch: c.patch }]) : { ok: true };
        if (!res.ok) { item.why ??= `${c.label.toLowerCase()}: ${res.first}`; continue; }
        item.fix = c, item.why = null;
        done.push({ index, patch: c.patch });
        place(w, p, c.patch);
        break;
      }
    }
    return { items, done };
  };
  let { items, done } = build(false);
  if (done.length && !holds(map, done).ok) ({ items } = build(true));
  return { items };
}

// Static files: the page, and three.js for its 3D view.
const TYPES = { ".js": "text/javascript", ".mjs": "text/javascript", ".html": "text/html; charset=utf-8" };
function serve(res, root, rel) {
  const path = normalize(join(root, rel));
  if (!path.startsWith(root) || !existsSync(path)) return json(res, 404, { error: "not found" });
  res.writeHead(200, { "content-type": TYPES[extname(path)] ?? "application/octet-stream" });
  res.end(readFileSync(path));
}

const json = (res, status, body) => (res.writeHead(status, { "content-type": "application/json" }), res.end(JSON.stringify(body)));
const read = (req) => new Promise((ok) => { let b = ""; req.on("data", (c) => (b += c)); req.on("end", () => ok(b ? JSON.parse(b) : {})); });

createServer(async (req, res) => {
  const url = new URL(req.url, "http://localhost"), q = (k) => url.searchParams.get(k);
  try {
    if (req.method === "GET" && url.pathname === "/") return res.writeHead(200, { "content-type": "text/html; charset=utf-8" }), res.end(readFileSync(PAGE));
    if (req.method === "GET" && url.pathname.startsWith("/vendor/three/")) return serve(res, VENDOR, url.pathname.slice("/vendor/three/".length));
    if (req.method === "GET" && url.pathname === "/api/blocks") return json(res, 200, { blocks: blocks(), undo: undo.at(-1)?.what ?? null });
    if (req.method === "GET" && url.pathname === "/api/map") {
      if (!names().includes(q("name"))) return json(res, 404, { error: "no such map" });
      return json(res, 200, mapView(q("name")));
    }
    if (req.method === "GET" && url.pathname === "/api/proposals") {
      const w = load(q("map")), p = propOf(w.m, { index: +q("index"), asset: q("asset"), x: +q("x"), z: +q("z") });
      return p ? json(res, 200, offered(q("map"), +q("index"), w, p)) : json(res, 409, { error: "the map changed since; reload" });
    }
    if (req.method === "GET" && url.pathname === "/api/plan") {
      if (!names().includes(q("map"))) return json(res, 404, { error: "no such map" });
      return json(res, 200, plan(q("map"), q("indices").split(",").map(Number)));
    }
    if (req.method === "POST" && url.pathname === "/api/apply") { const r = apply(await read(req)); return json(res, r.status, r.body); }
    if (req.method === "POST" && url.pathname === "/api/reject") { const r = reject(await read(req)); return json(res, r.status, r.body); }
    if (req.method === "POST" && url.pathname === "/api/undo") {
      const u = undo.pop();
      if (!u) return json(res, 400, { error: "nothing to undo" });
      if (u.text === null) unlinkSync(u.path);
      else writeFileSync(u.path, u.text);
      return json(res, 200, { undone: u.what, check: u.path === WAIVERS ? null : checkMaps() });
    }
    json(res, 404, { error: "not found" });
  } catch (e) {
    json(res, 500, { error: String(e.stack ?? e) });
  }
}).listen(port, "127.0.0.1", () => console.log(`placement review: http://localhost:${port}`));
