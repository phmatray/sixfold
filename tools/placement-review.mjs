// Run: node tools/placement-review.mjs [web/world] [--port=8095]   then open http://localhost:8095
// The placement review app: every finding of tools/placement.rules, the map round it drawn from above (and in 3D, with
// the game's own models), before and after a proposed fix: lift it onto the floor, turn it, or move it to the nearest
// place where its rules hold. Nothing is ever taken out, and only fixes that leave `check-maps --strict` passing are
// offered. Findings of one rule on props next to one another (2 tiles or closer, chained) come as one block, with one
// plan: each prop's best fix, each built on the ones before it. Every finding has a short id to quote (#k3f9a).
// Confirm, and the fixes are written to the map's JSON and `check-maps --strict` runs on the result (Undo puts the
// file back). Reject, and the findings are answered "this is fine" in tools/placement.waivers.json, which placement.mjs
// skips. `desk()` is all of that without the web page; tools/placement-mcp.mjs serves it to an agent over MCP.
// Dev tooling, local only: it writes into web/world/maps. Its labels are English, like the editor's.
import { createServer } from "node:http";
import { readFileSync, writeFileSync, readdirSync, existsSync, unlinkSync, mkdtempSync, mkdirSync, symlinkSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve, normalize, extname } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { parseRules, tables, world, check, proposals, place, ridersOf, waivers, WAIVERS } from "./placement.mjs";
import * as ld from "../web/world/ld.js";

const RULES = fileURLToPath(new URL("./placement.rules", import.meta.url));
const PAGE = fileURLToPath(new URL("./placement-review.html", import.meta.url));
const VENDOR = fileURLToPath(new URL("../web/vendor/three/", import.meta.url)); // three.js and its loaders, for the 3D view

// The review desk over one world folder: what the app and the MCP server both call.
export function desk(dir = "web/world") {
  const t = tables(dir);
  const file = (name) => `${dir}/maps/${name}.json`;
  const names = () => readdirSync(`${dir}/maps`).map((f) => f.replace(".json", "")).sort();
  const rules = () => parseRules(readFileSync(RULES, "utf8")).rules; // read fresh: the rules may change while it runs
  const load = (name) => world(name, JSON.parse(readFileSync(file(name), "utf8")), t);
  const undo = []; // [{ path, text | null, what }]
  const known = (name) => { if (!names().includes(name)) throw new Error(`no map ${name}`); };

  // The findings, in blocks: one rule's findings on one map whose props stand 2 tiles or closer, chained.
  function blocks() {
    const r = rules(), waived = waivers(), out = [];
    for (const name of names()) {
      const found = check(load(name), r, waived);
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
  // The block holding a finding's id, and that finding.
  function find(id) {
    id = String(id).replace(/^#/, "").toLowerCase();
    for (const b of blocks()) { const m = b.members.find((x) => x.id === id); if (m) return { block: b, member: m }; }
    return null;
  }

  // What the page needs to draw a map: its tiles and paint, its props and what stands on them, and the props.js sizes.
  function mapView(name) {
    known(name);
    const m = JSON.parse(readFileSync(file(name), "utf8"));
    return {
      name, title: m.title ?? name, kind: m.kind ?? "interior", region: m.region ?? null, indoor: !!m.light?.indoor,
      width: m.width, height: m.height, tiles: m.tiles, paint: m.paint ?? null,
      props: m.props.map((p) => ({ asset: p.asset, x: p.x, z: p.z, rot: p.rot ?? 0, y: p.y ?? 0, recolor: p.recolor ?? null, glow: p.glow ?? null, if: p.if ?? null, w: ld.foot(p, t.FOOT)[0], d: ld.foot(p, t.FOOT)[1], ground: t.WALKABLE.has(p.asset) || !!t.DECK[p.asset], tall: t.tall(p.asset) })),
      spots: (m.interactions ?? []).map((i) => ({ x: i.x, z: i.z, kind: i.kind })),
      doors: (m.warps ?? []).map((w) => ({ x: w.x, z: w.z, to: w.to })),
      npcs: (m.npcs ?? []).map((n) => ({ x: n.x, z: n.z, id: n.id })),
    };
  }

  // The prop at index, as the caller saw it (asset and tile), or null when the file moved on since.
  const propOf = (m, { index, asset, x, z }) => { const p = m.props[index]; return p && p.asset === asset && p.x === x && p.z === z ? p : null; };

  // A fix on a map's JSON: the prop's new model, place, turn and height, the spots it carries moved with it, its twins
  // (a find's crates and opened crates, each with an `if`, on one tile) moved alongside, and the ground re-laid.
  function patchMap(m, p, patch) {
    const dx = (patch.x ?? p.x) - p.x, dz = (patch.z ?? p.z) - p.z;
    const twins = p.if ? m.props.filter((q) => q !== p && q.if && q.x === p.x && q.z === p.z) : [];
    const riders = ridersOf(m.props, p, t), dy = ("y" in patch ? patch.y ?? 0 : p.y ?? 0) - (p.y ?? 0); // crates set on it go with it
    for (const i of patch.spots ?? []) (m.interactions[i].x += dx), (m.interactions[i].z += dz);
    for (const q of [p, ...twins]) for (const k of ["x", "z", "rot", "y"]) if (k in patch) patch[k] === undefined || patch[k] === null ? delete q[k] : (q[k] = patch[k]);
    for (const q of riders) (q.x += dx), (q.z += dz), (q.y = +((q.y ?? 0) + dy).toFixed(2));
    if (patch.asset) p.asset = patch.asset; // its own model only: a twin keeps its own
    for (const [x, z, g] of patch.tiles ?? []) m.tiles[z] = m.tiles[z].slice(0, x) + g + m.tiles[z].slice(x + 1);
  }

  // check-maps over a world folder, as CI runs it (~0.3 s). Its findings are the lines naming a map (or the atlas).
  function checkMaps(at = dir) {
    const r = spawnSync(process.execPath, [fileURLToPath(new URL("./check-maps.mjs", import.meta.url)), at, "--strict"], { encoding: "utf8" });
    const lines = `${r.stdout}${r.stderr}`.trim().split("\n").filter(Boolean);
    const said = lines.filter((l) => /^(Error: )?\s*[a-z_]+: /.test(l) && !l.startsWith("maps ok")).map((l) => l.replace(/^Error: /, "").trim());
    const warnings = lines.filter((l) => /^ {2}[a-z_]+: /.test(l)).map((l) => l.trim()); // "N placement warnings …" then one indented line each
    return { ok: r.status === 0, lines: lines.slice(-12), warnings, first: said[0] ?? lines.find((l) => l.startsWith("Error"))?.slice(7) ?? null };
  }
  // The world's own warnings, so a fix is judged on the ones it adds (a sign moved behind a lamp, a pocket closed off).
  let base = null;
  const baseWarnings = () => {
    const stamp = names().map((n) => statSync(file(n)).mtimeMs).join();
    if (base?.stamp !== stamp) base = { stamp, warnings: new Set(checkMaps().warnings) };
    return base.warnings;
  };

  // A world of links to the real one, where one map at a time is swapped for a fixed copy to see whether check-maps
  // still passes, before a fix is ever offered. Made on first use, gone when the process ends.
  let shadow = null;
  function holds(map, fixes) { // fixes: [{ index, patch }]
    if (!shadow) {
      shadow = mkdtempSync(join(tmpdir(), "placement-review-"));
      mkdirSync(`${shadow}/maps`);
      for (const f of ["props.js", "atlas.json", "quests.json"]) if (existsSync(`${dir}/${f}`)) symlinkSync(resolve(`${dir}/${f}`), `${shadow}/${f}`);
      for (const name of names()) symlinkSync(resolve(file(name)), `${shadow}/maps/${name}.json`);
      const gone = shadow;
      process.on("exit", () => rmSync(gone, { recursive: true, force: true }));
    }
    const known = baseWarnings(), link = `${shadow}/maps/${map}.json`, m = JSON.parse(readFileSync(file(map), "utf8"));
    for (const f of fixes) patchMap(m, m.props[f.index], f.patch);
    unlinkSync(link);
    writeFileSync(link, JSON.stringify(m));
    try {
      const r = checkMaps(shadow), added = r.warnings.filter((w) => !known.has(w));
      // A fix that passes but adds a warning (hides a sign behind a lamp, closes off a pocket) is no fix.
      return added.length ? { ...r, ok: false, first: `it adds a warning: ${added[0]}` } : r;
    } finally { unlinkSync(link); symlinkSync(resolve(file(map)), link); }
  }

  // The fixes worth offering for one prop: those check-maps still passes, at most one lift, swap, ground fill and turn,
  // and two moves.
  function offered(map, { index, asset, x, z }) {
    known(map);
    const w = load(map), p = propOf(w.m, { index, asset, x, z });
    if (!p) throw new Error(`${map} changed since: no ${asset} at ${x},${z}. Reload.`);
    const out = [], room = { lift: 1, swap: 1, fill: 1, turn: 1, move: 2 };
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
    known(map);
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

  // Fixes for several props of one map at once: items [{ index, asset, x, z, patch }], all written in one go.
  function apply({ map, items }) {
    known(map);
    const path = file(map), text = readFileSync(path, "utf8"), m = JSON.parse(text);
    const props = items.map((i) => propOf(m, i));
    const stale = items.find((_, k) => !props[k]);
    if (stale) throw Object.assign(new Error(`${map} changed since: no ${stale.asset} at ${stale.x},${stale.z}. Reload.`), { status: 409 });
    items.forEach((i, k) => patchMap(m, props[k], i.patch));
    writeFileSync(path, JSON.stringify(m, null, 2) + "\n");
    undo.push({ path, text, what: items.length > 1 ? `${items.length} fixes on ${map}` : `${items[0].asset} ${items[0].x},${items[0].z} on ${map}` });
    return { done: true, check: checkMaps() };
  }

  function reject({ map, rule, items, note }) {
    const text = existsSync(WAIVERS) ? readFileSync(WAIVERS, "utf8") : null, list = text ? JSON.parse(text) : [];
    for (const { asset, x, z } of items) list.push({ map, rule, asset, x, z, ...(note && { note }) });
    list.sort((a, b) => `${a.map} ${a.rule}`.localeCompare(`${b.map} ${b.rule}`) || a.x - b.x || a.z - b.z);
    writeFileSync(WAIVERS, JSON.stringify(list, null, 1) + "\n");
    undo.push({ path: WAIVERS, text, what: `the answer on ${items.length > 1 ? `${items.length} ${items[0].asset}` : `${items[0].asset} ${items[0].x},${items[0].z}`} (${rule})` });
    return { done: true };
  }

  function undoLast() {
    const u = undo.pop();
    if (!u) throw new Error("nothing to undo");
    if (u.text === null) unlinkSync(u.path);
    else writeFileSync(u.path, u.text);
    return { undone: u.what, check: u.path === WAIVERS ? null : checkMaps() };
  }

  // A placement a person chose ({ x, z, rot, y }), judged like an offered fix: the spots on the prop go with it, its y
  // follows the floor where it lands (as high above it as before), and the answer names the rules it still breaks
  // there and whether check-maps passes without a new warning. { patch, broken: [rule ids], check: { ok, first } }.
  function tryFix(map, { index, asset, x, z }, chosen) {
    known(map);
    const m = JSON.parse(readFileSync(file(map), "utf8")), p = propOf(m, { index, asset, x, z });
    if (!p) throw Object.assign(new Error(`${map} changed since: no ${asset} at ${x},${z}. Reload.`), { status: 409 });
    const [fw, fd] = ld.foot(p, t.FOOT), on = (px, pz, w, d, sx, sz) => sx >= px && sx < px + w && sz >= pz && sz < pz + d;
    const spots = (m.interactions ?? []).flatMap((s, i) => (on(p.x, p.z, fw, fd, s.x, s.z) ? [i] : []));
    const floor = (tx, tz) => { const c = m.tiles[tz]?.[tx]; return c >= "1" && c <= "9" ? +c : c === "=" ? t.DECK.boardwalk[0] : c === "H" ? t.DECK.pier[0] : 0; };
    const patch = { x: chosen.x, z: chosen.z, rot: chosen.rot ?? p.rot ?? 0, ...(spots.length && { spots }) };
    if (chosen.y !== undefined) patch.y = chosen.y || undefined;
    else { const y = +(floor(chosen.x, chosen.z) + Math.max(0, (p.y ?? 0) - floor(p.x, p.z))).toFixed(2); patch.y = y || undefined; }
    const after = patched(map, [{ index, patch }]), w = world(map, after, t);
    const broken = check(w, rules()).filter((f) => f.index === index).map((f) => f.rule);
    const r = holds(map, [{ index, patch }]);
    return { patch, broken, check: { ok: r.ok, first: r.ok ? null : r.first } };
  }

  // A map's JSON with some fixes applied ([{ index, patch }]), for the game view.
  function patched(map, fixes = []) {
    known(map);
    const m = JSON.parse(readFileSync(file(map), "utf8"));
    for (const f of fixes) patchMap(m, m.props[f.index], f.patch);
    return m;
  }

  // Where the hero stands for a game shot: the nearest tile two rows south of [cx, cz] (the camera looks north) that no
  // standing prop's footprint covers, off walls, fences and water.
  function stand(m, [cx, cz]) {
    const taken = new Set();
    for (const p of m.props) {
      if (t.WALKABLE.has(p.asset) || t.DECK[p.asset]) continue;
      const [w, dd] = ld.foot(p, t.FOOT);
      for (let x = p.x; x < p.x + w; x++) for (let z = p.z; z < p.z + dd; z++) taken.add(`${x},${z}`);
    }
    const open = (x, z) => m.tiles[z]?.[x] !== undefined && !"#fWV~".includes(m.tiles[z][x]) && !taken.has(`${x},${z}`);
    const x0 = Math.round(cx), z0 = Math.round(cz) + 2;
    for (let r = 0; r < 12; r++) for (let dz = -r; dz <= r; dz++) for (let dx = -r; dx <= r; dx++) if (Math.max(Math.abs(dx), Math.abs(dz)) === r && open(x0 + dx, z0 + dz)) return [x0 + dx, z0 + dz];
    return [x0, z0];
  }

  return { dir, names, rules, blocks, find, mapView, offered, plan, apply, reject, patched, stand, tryFix, undo: undoLast, lastUndo: () => undo.at(-1)?.what ?? null, checkMaps: () => checkMaps() };
}

// ----- The game view: the running game, driven headless, photographs a map before and after its fixes -----
// The game (trunk serve) runs at `game`; Playwright (e2e's) drives one hidden page of it, booted into the world once.
// Each shot hands the game the map as given (map.js setMap, as the editor previews a map), warps the hero just south
// of the props, frames them and takes a picture. One shot at a time.
export function camera({ game = "http://localhost:8080", playwright }) {
  let page = null, queue = Promise.resolve();
  const boot = async () => {
    const pw = playwright ?? fileURLToPath(new URL("../e2e/node_modules/playwright/index.mjs", import.meta.url));
    if (!existsSync(pw)) throw Object.assign(new Error(`the game view needs Playwright: run \`npm ci\` in e2e/ (or pass --playwright=<path to playwright/index.mjs>)`), { status: 503 });
    const { chromium } = await import(pw);
    const browser = await chromium.launch({ args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--ignore-gpu-blocklist"] });
    const ctx = await browser.newContext({ viewport: { width: 1200, height: 900 }, serviceWorkers: "block", locale: "en-US", reducedMotion: "reduce" });
    await ctx.addInitScript(() => sessionStorage.setItem("sixfold.splashed", "1"));
    const p = await ctx.newPage();
    process.on("exit", () => browser.close().catch(() => {}));
    await enter(p);
    return p;
  };
  // Into the world from the title (again, when trunk serve reloaded the page after a map changed).
  const enter = async (p) => {
    try { await p.goto(game, { timeout: 60_000, waitUntil: "domcontentloaded" }); } // a debug build loads slowly on a busy machine
    catch { throw Object.assign(new Error(`the game isn't running at ${game}: start it with \`cd web && trunk serve\` (or pass --game=<url>)`), { status: 503 }); }
    await p.getByRole("navigation", { name: "Main menu" }).getByRole("button", { name: /^Explore the world/ }).click({ timeout: 240_000 });
    await p.waitForFunction(() => document.querySelector(".world-host")?.sixfoldWarp, null, { timeout: 120_000 });
  };
  // raw: the map's JSON; [x, z]: where the hero stands; extent: how many tiles the props spread over.
  const shoot = async (name, raw, [x, z], extent) => {
    page ??= boot();
    const p = await page.catch((e) => { page = null; throw e; }), host = (f, ...a) => p.evaluate(([f, a]) => document.querySelector(".world-host")[f](...a), [f, a]);
    if (!(await p.evaluate(() => !!document.querySelector(".world-host")?.sixfoldWarp).catch(() => false))) await enter(p); // trunk serve reloads its pages when a file changes
    await p.evaluate(async ([n, r]) => (await import("/world/map.js")).setMap(n, r), [name, raw]);
    const skip = p.getByRole("button", { name: /^Skip/ });
    // A warp rebuilds the map from the copy just handed over. Wait for the map and the hero (put on the nearest free
    // tile, so within 2 of the one asked); warp again only if the map never came (one asked during a fade is ignored).
    let there = false;
    for (let tries = 0; tries < 3 && !there; tries++) {
      for (let t = 0; t < 10 && (await skip.isVisible().catch(() => false)); t++) await skip.click().catch(() => {}), await p.waitForTimeout(600);
      await host("sixfoldWarp", name, x, z, "n");
      for (let t = 0; t < 12 && !there; t++) {
        await p.waitForTimeout(500);
        const st = await p.evaluate(() => document.querySelector(".world-host").sixfoldStats());
        there = st.area === name && Math.abs(st.at?.[0] - x) <= 2 && Math.abs(st.at?.[1] - z) <= 2;
      }
    }
    if (!there) throw new Error(`the game never reached ${name} ${x},${z}`);
    // A site's arrival scene can start after the fade and take the camera: skip it, warp back, settle again.
    await p.waitForTimeout(800);
    for (let again = 0; again < 3 && (await skip.isVisible().catch(() => false)); again++) {
      for (let t = 0; t < 10 && (await skip.isVisible().catch(() => false)); t++) await skip.click().catch(() => {}), await p.waitForTimeout(700);
      await host("sixfoldWarp", name, x, z, "n");
      await p.waitForTimeout(3000);
    }
    await host("sixfoldClock", 12 * 60);
    await host("sixfoldCam", { distance: Math.min(30, Math.max(12, extent * 1.3)), pitch: 50 });
    await p.waitForTimeout(1500);
    return p.screenshot({ type: "jpeg", quality: 82 });
  };
  // One shot at a time; of those waiting, only the last two (the card on screen, before and after) are kept: a card
  // left behind drops its shots instead of holding up the one now in view.
  const waiting = [];
  const shot = (...a) => new Promise((ok, no) => {
    waiting.push({ a, ok, no });
    while (waiting.length > 2) waiting.shift().no(Object.assign(new Error("superseded by a newer shot"), { status: 409 }));
    queue = queue.then(async () => {
      const job = waiting.shift();
      if (!job) return;
      try { job.ok(await shoot(...job.a)); } catch (e) { job.no(e); }
    });
  });
  return { shot };
}


// ----- The web app -----

// Static files: three.js, and the game's own world folder (its models and the code that loads and dresses them), so
// the 3D view draws the real props.
const TYPES = { ".js": "text/javascript", ".mjs": "text/javascript", ".html": "text/html; charset=utf-8", ".json": "application/json", ".glb": "model/gltf-binary", ".png": "image/png", ".webp": "image/webp", ".jpg": "image/jpeg" };
function serve(res, root, rel) {
  const path = normalize(join(root, decodeURIComponent(rel)));
  if (!path.startsWith(root) || !existsSync(path)) return json(res, 404, { error: "not found" });
  res.writeHead(200, { "content-type": TYPES[extname(path)] ?? "application/octet-stream" });
  res.end(readFileSync(path));
}
const json = (res, status, body) => (res.writeHead(status, { "content-type": "application/json" }), res.end(JSON.stringify(body)));
const read = (req) => new Promise((ok) => { let b = ""; req.on("data", (c) => (b += c)); req.on("end", () => ok(b ? JSON.parse(b) : {})); });

function app(dir, port, cam) {
  const d = desk(dir), WORLD = resolve(dir) + "/";
  createServer(async (req, res) => {
    const url = new URL(req.url, "http://localhost"), q = (k) => url.searchParams.get(k), get = req.method === "GET", post = req.method === "POST";
    try {
      if (get && url.pathname === "/") return res.writeHead(200, { "content-type": "text/html; charset=utf-8" }), res.end(readFileSync(PAGE));
      if (get && url.pathname.startsWith("/vendor/three/")) return serve(res, VENDOR, url.pathname.slice("/vendor/three/".length));
      if (get && url.pathname.startsWith("/world/")) return serve(res, WORLD, url.pathname.slice("/world/".length));
      if (get && url.pathname === "/api/blocks") return json(res, 200, { blocks: d.blocks(), undo: d.lastUndo() });
      if (get && url.pathname === "/api/map") return json(res, 200, d.mapView(q("name")));
      if (get && url.pathname === "/api/proposals") return json(res, 200, d.offered(q("map"), { index: +q("index"), asset: q("asset"), x: +q("x"), z: +q("z") }));
      if (get && url.pathname === "/api/plan") return json(res, 200, d.plan(q("map"), q("indices").split(",").map(Number)));
      if (post && url.pathname === "/api/try") { const b = await read(req); return json(res, 200, d.tryFix(b.map, b.member, b.place)); }
      if (post && url.pathname === "/api/apply") return json(res, 200, d.apply(await read(req)));
      if (post && url.pathname === "/api/reject") {
        // A page from before ⌘R was kept off the R key answered a finding on every reload: it sends no `page`.
        const body = await read(req);
        if (body.page !== 2) return json(res, 409, { error: "this page is out of date: reload it" });
        return json(res, 200, d.reject(body));
      }
      if (post && url.pathname === "/api/undo") return json(res, 200, d.undo());
      if (post && url.pathname === "/api/shot") {
        // { map, fixes: [{ index, patch }] (none: the map as it is), at: [x, z], extent }: a JPEG from the game.
        const { map, fixes, at, extent } = await read(req), raw = d.patched(map, fixes), jpeg = await cam.shot(map, raw, d.stand(raw, at), extent ?? 6);
        return res.writeHead(200, { "content-type": "image/jpeg" }), res.end(jpeg);
      }
      json(res, 404, { error: "not found" });
    } catch (e) {
      json(res, e.status ?? (/no map|nothing to undo|changed since/.test(e.message) ? 400 : 500), { error: e.message });
    }
  }).listen(port, "127.0.0.1", () => console.log(`placement review: http://localhost:${port}`));
  // Playwright's own signal handlers close its browser but leave the process running: exit on them ourselves.
  for (const sig of ["SIGINT", "SIGTERM", "SIGHUP"]) process.on(sig, () => process.exit(0));
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const argv = process.argv.slice(2), opt = (k) => argv.find((a) => a.startsWith(`--${k}=`))?.slice(k.length + 3);
  app(argv.find((a) => !a.startsWith("--")) ?? "web/world", +(opt("port") ?? 8095), camera({ game: opt("game"), playwright: opt("playwright") }));
}
