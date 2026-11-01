// Run: node tools/placement-review.mjs [web/world] [--port=8095]   then open http://localhost:8095
// The placement review app: every finding of tools/placement.rules, the map round it drawn from above, before and
// after a proposed fix (lift it onto the floor, turn it, move it to the nearest place where its rules hold, take it out).
// Confirm, and the fix is written to the map's JSON and `check-maps --strict` runs on the result (Undo puts the file
// back). Reject, and the finding is answered "this is fine" in tools/placement.waivers.json, which placement.mjs skips.
// Dev tooling, local only: it writes into web/world/maps. Its labels are English, like the editor's.
import { createServer } from "node:http";
import { readFileSync, writeFileSync, readdirSync, existsSync, unlinkSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { parseRules, tables, world, check, proposals, waivers, WAIVERS } from "./placement.mjs";
import * as ld from "../web/world/ld.js";

const argv = process.argv.slice(2);
const dir = argv.find((a) => !a.startsWith("--")) ?? "web/world";
const port = +(argv.find((a) => a.startsWith("--port="))?.slice(7) ?? 8095);
const RULES = fileURLToPath(new URL("./placement.rules", import.meta.url));
const PAGE = fileURLToPath(new URL("./placement-review.html", import.meta.url));
const t = tables(dir);
const file = (name) => `${dir}/maps/${name}.json`;
const names = () => readdirSync(`${dir}/maps`).map((f) => f.replace(".json", "")).sort();
const rules = () => parseRules(readFileSync(RULES, "utf8")).rules; // read fresh: the rules may change while the app runs
const load = (name) => world(name, JSON.parse(readFileSync(file(name), "utf8")), t);
const undo = []; // [{ path, text | null, what }]

function findings() {
  const r = rules(), waived = waivers();
  return names().flatMap((name) => check(load(name), r, waived).map(({ prop, ...f }) => ({ ...f, id: `${f.map}|${f.index}|${f.rule}`, asset: prop.asset, x: prop.x, z: prop.z, rot: prop.rot ?? 0, y: prop.y ?? 0 })));
}

// What the page needs to draw a map: its tiles and paint, its props and what stands on them, and the props.js sizes.
function mapView(name) {
  const m = JSON.parse(readFileSync(file(name), "utf8"));
  return {
    name, title: m.title ?? name, kind: m.kind ?? "interior", region: m.region ?? null, indoor: !!m.light?.indoor,
    width: m.width, height: m.height, tiles: m.tiles, paint: m.paint ?? null,
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

function apply({ map, index, asset, x, z, patch }) {
  const path = file(map), text = readFileSync(path, "utf8"), m = JSON.parse(text), p = propOf(m, { index, asset, x, z });
  if (!p) return { status: 409, body: { error: `${map} changed since: no ${asset} at ${x},${z} (index ${index}). Reload.` } };
  const dx = (patch.x ?? p.x) - p.x, dz = (patch.z ?? p.z) - p.z;
  for (const i of patch.spots ?? []) if (patch.remove) m.interactions[i] = null; else (m.interactions[i].x += dx), (m.interactions[i].z += dz);
  if (patch.spots && patch.remove) m.interactions = m.interactions.filter(Boolean);
  if (patch.remove) m.props.splice(index, 1);
  else for (const k of ["x", "z", "rot", "y"]) if (k in patch) patch[k] === undefined || patch[k] === null ? delete p[k] : (p[k] = patch[k]);
  writeFileSync(path, JSON.stringify(m, null, 2) + "\n");
  undo.push({ path, text, what: `${asset} ${x},${z} on ${map}` });
  return { status: 200, body: { done: true, check: checkMaps() } };
}

function reject({ map, rule, asset, x, z, note }) {
  const text = existsSync(WAIVERS) ? readFileSync(WAIVERS, "utf8") : null, list = text ? JSON.parse(text) : [];
  list.push({ map, rule, asset, x, z, ...(note && { note }) });
  list.sort((a, b) => `${a.map} ${a.rule}`.localeCompare(`${b.map} ${b.rule}`) || a.x - b.x || a.z - b.z);
  writeFileSync(WAIVERS, JSON.stringify(list, null, 1) + "\n");
  undo.push({ path: WAIVERS, text, what: `the answer on ${asset} ${x},${z} (${rule})` });
  return { status: 200, body: { done: true } };
}

// check-maps over the whole world, as CI runs it (~0.3 s).
function checkMaps() {
  const r = spawnSync(process.execPath, [fileURLToPath(new URL("./check-maps.mjs", import.meta.url)), dir, "--strict"], { encoding: "utf8" });
  const lines = `${r.stdout}${r.stderr}`.trim().split("\n").filter(Boolean);
  return { ok: r.status === 0, lines: lines.slice(-12) };
}

const json = (res, status, body) => (res.writeHead(status, { "content-type": "application/json" }), res.end(JSON.stringify(body)));
const read = (req) => new Promise((ok) => { let b = ""; req.on("data", (c) => (b += c)); req.on("end", () => ok(b ? JSON.parse(b) : {})); });

createServer(async (req, res) => {
  const url = new URL(req.url, "http://localhost"), q = (k) => url.searchParams.get(k);
  try {
    if (req.method === "GET" && url.pathname === "/") return res.writeHead(200, { "content-type": "text/html; charset=utf-8" }), res.end(readFileSync(PAGE));
    if (req.method === "GET" && url.pathname === "/api/findings") return json(res, 200, { findings: findings(), undo: undo.at(-1)?.what ?? null });
    if (req.method === "GET" && url.pathname === "/api/map") {
      if (!names().includes(q("name"))) return json(res, 404, { error: "no such map" });
      return json(res, 200, mapView(q("name")));
    }
    if (req.method === "GET" && url.pathname === "/api/proposals") {
      const w = load(q("map")), p = propOf(w.m, { index: +q("index"), asset: q("asset"), x: +q("x"), z: +q("z") });
      return p ? json(res, 200, { proposals: proposals(w, p, rules()) }) : json(res, 409, { error: "the map changed since; reload" });
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
