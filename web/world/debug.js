// World debug overlays (#111): debug builds only (`trunk serve`), loaded by `?debug` in the URL, the way
// `?edit` loads editor.js (release builds never import this file). Eight layers drawn in the 3D scene, so they
// read at the real camera and show in screenshots: keys 1-8 (or the panel) toggle them, 0 hides the HUD and
// the hero for a clean capture. From the console or a test: `await el.sixfoldDebug(["visibility", "pois"])`
// turns exactly those layers on ("clean" in the list too), and returns what each layer drew.
// The geometry is web/world/ld.js, the analysis tools/check-maps.mjs judges maps with, run on the map's file
// as written (not the localized copy the game plays), so the overlay and CI agree.
// Dev tooling: its labels are English only, like the editor's.
import * as THREE from "three";
import * as ld from "./ld.js";
import { FOOT, DECK, WALKABLE, TALL, LADDER } from "./props.js";
import { DIR, key } from "./actors.js";
import { sightLine } from "./trainer.js";

export const LAYERS = ["collisions", "walkable", "circulation", "visibility", "pois", "npcs", "lights", "links"];
const TITLES = ["Collisions", "Walkable", "Circulation", "Visibility", "POIs and secrets", "NPC zones", "Lights", "Map links"];
const STORE = "sixfold.debug";
// What a clean capture hides besides the hero (and this panel).
const HUD = ".world-top, .world-prompt, .world-controls, .world-legend, .debug-panel";

const node = (tag, props = {}, ...kids) => {
  const n = Object.assign(document.createElement(tag), props);
  n.append(...kids);
  return n;
};
const read = () => {
  try {
    return JSON.parse(sessionStorage.getItem(STORE) ?? "{}");
  } catch {
    return {};
  }
};

export function attach({ world, scene, camera, el }) {
  const root = new THREE.Group();
  root.name = "debug";
  scene.add(root);
  const saved = read();
  let on = new Set((saved.on ?? []).filter((l) => LAYERS.includes(l))), clean = !!saved.clean;
  let shown = null, sig = "", ctx = null, ready = null, alive = true, raf = 0;
  const groups = {};

  // ----- The panel -----
  const boxes = LAYERS.map((l, i) => {
    const b = node("input", { type: "checkbox", checked: on.has(l), onchange: () => toggle(l, b.checked) });
    return node("label", { style: "display:flex;gap:6px;align-items:center" }, b, `${i + 1} ${TITLES[i]}`);
  });
  const count = node("div", { style: "opacity:.75" });
  const panel = node("div", { className: "debug-panel", style: "position:absolute;right:12px;top:76px;z-index:5;display:grid;gap:4px;padding:10px;border-radius:12px;background:rgba(13,26,44,.86);color:#f6ecd2;font:13px system-ui;max-width:220px" },
    node("b", { textContent: "Debug overlays" }), ...boxes, node("span", { style: "opacity:.75", textContent: "0 clean capture" }), count);
  el.append(panel);
  const style = node("style", { textContent: `.debug-clean :is(${HUD}) { visibility: hidden !important; }` });
  document.head.append(style);

  const save = () => {
    try {
      sessionStorage.setItem(STORE, JSON.stringify({ on: [...on], clean }));
    } catch {}
  };
  function toggle(l, v = !on.has(l)) {
    v ? on.add(l) : on.delete(l);
    boxes[LAYERS.indexOf(l)].firstChild.checked = v;
    save();
    draw();
  }
  const onKey = (e) => {
    if (e.target.closest?.("input, select, textarea, dialog") || e.ctrlKey || e.metaKey || e.altKey || e.repeat) return;
    const i = "12345678".indexOf(e.key);
    if (i >= 0) toggle(LAYERS[i]);
    else if (e.key === "0") (clean = !clean), save();
  };
  addEventListener("keydown", onKey);

  // ----- Layers -----
  const free = (o) =>
    o.traverse((c) => {
      c.geometry?.dispose();
      c.material?.map?.dispose();
      c.material?.dispose();
    });
  // (Re)draw the layers that are on and not drawn yet; drop the ones turned off.
  function draw(force = []) {
    if (!ctx) return;
    for (const l of LAYERS) {
      if (groups[l] && (!on.has(l) || force.includes(l))) root.remove(groups[l]), free(groups[l]), delete groups[l];
      if (on.has(l) && !groups[l]) root.add((groups[l] = BUILD[l](ctx)));
    }
    count.textContent = LAYERS.filter((l) => groups[l]).map((l) => `${l} ${groups[l].userData.count}`).join(" · ");
  }

  // Each frame: a new map rebuilds everything, a gate that changed the static layers, a person or a live light
  // that moved its own layer. Never more per frame than comparing a few strings.
  function frame() {
    raf = requestAnimationFrame(frame);
    const w = world();
    if (!w?.name) return;
    document.body.classList.toggle("debug-clean", clean);
    if (w.hero) (w.hero.obj.visible = !clean), (w.hero.blob.visible = !clean);
    if (w !== shown) {
      shown = w;
      ctx = null;
      for (const l of Object.keys(groups)) root.remove(groups[l]), free(groups[l]), delete groups[l];
      ready = analyse(w).then((c) => {
        if (!alive || shown !== w) return;
        ctx = c;
        sig = gates(w);
        draw();
      });
      return;
    }
    if (!ctx) return;
    const g = gates(w);
    if (g !== sig) (sig = g), draw(LAYERS.filter((l) => !DYNAMIC[l]));
    for (const [l, f] of Object.entries(DYNAMIC)) {
      if (!groups[l]) continue;
      const s = f(ctx);
      if (s !== groups[l].userData.sig) draw([l]), (groups[l].userData.sig = s);
    }
  }
  raf = requestAnimationFrame(frame);

  el.sixfoldDebug = async (list) => {
    if (list) {
      on = new Set(list.filter((l) => LAYERS.includes(l)));
      clean = list.includes("clean");
      boxes.forEach((b, i) => (b.firstChild.checked = on.has(LAYERS[i])));
      save();
    }
    while (!ctx && alive) await Promise.all([ready, new Promise((r) => requestAnimationFrame(r))]);
    draw();
    return { on: [...on], clean, counts: Object.fromEntries([...on].map((l) => [l, groups[l]?.userData.count ?? 0])) };
  };

  return {
    refresh: () => draw(LAYERS),
    stop() {
      alive = false;
      cancelAnimationFrame(raf);
      removeEventListener("keydown", onKey);
      document.body.classList.remove("debug-clean");
      const h = world()?.hero;
      if (h) h.obj.visible = h.blob.visible = true;
      root.removeFromParent();
      free(root);
      panel.remove();
      style.remove();
      delete el.sixfoldDebug;
    },
  };
}

// The gates' state: the static layers redraw when it changes.
const gates = (w) => w.gates.map((g) => (g.open ? 1 : 0)).join("");

// ----- The analysis (web/world/ld.js, as tools/check-maps.mjs runs it) -----

const LD = { FOOT, DECK, WALKABLE: new Set([...WALKABLE, ...Object.keys(DECK)]), tall: (a) => ld.HEIGHT[a] ?? TALL[a] ?? 1 };
const files = new Map();
// A map's file as written (the game plays a localized, expanded copy; check-maps reads this one).
const file = (name) => {
  if (!files.has(name)) files.set(name, fetch(new URL(`maps/${name}.json`, import.meta.url), { cache: "no-store" }).then((r) => (r.ok ? r.json() : null)).catch(() => null));
  return files.get(name);
};
const atlas = () => file("../atlas");

async function analyse(w) {
  const m = await file(w.name);
  if (!m) throw new Error(`debug: no maps/${w.name}.json`);
  const dests = Object.fromEntries(await Promise.all((m.exits ?? []).map(async (e) => [e.to, await file(e.to)])));
  const g = ld.grids(m, LD), floor = (x, z) => g.floor[key(x, z)] ?? 0;
  // Reachable with every gate open, from the spawn: the live collision, gate tiles open at their open floor.
  // ponytail: lifts and same-map warps don't carry the fill on (check-maps' reach() does), so the far side of
  // a lift reads as unreachable here; join them if a map with lifts gets reviewed.
  const gated = new Map(w.gates.flatMap((gt) => gt.tiles.map((t) => [key(t.x, t.z), t])));
  const F = (x, z) => gated.get(key(x, z))?.open ?? w.floorAt(x, z);
  const ladders = new Map(m.props.filter((p) => LADDER.has(p.asset)).map((p) => [key(p.x, p.z), p.rot % 180 === 90 ? "x" : "z"]));
  const step = (ax, az, bx, bz) => Math.abs(F(bx, bz) - F(ax, az)) <= 0.6 || [key(ax, az), key(bx, bz)].some((k) => ladders.get(k) === (ax !== bx ? "x" : "z"));
  const open = new Set();
  for (let z = 0; z < m.height; z++) for (let x = 0; x < m.width; x++) if (!w.blocked(x, z) || gated.has(key(x, z))) open.add(key(x, z));
  const walk = (starts) => ld.bfs(open, starts, step);
  const all = walk([[m.spawn.x, m.spawn.z]]).dist;
  const signs = new Set((m.exits ?? []).map((e) => ld.exitSign(m, e, dests[e.to])).filter(Boolean));
  const pois = ld.pois(m, signs);
  return { w, m, g, floor, all, walk, dests, signs, pois, atlas: await atlas() };
}

// ----- Drawing: flat tiles, lines, boxes and labels, drawn over the scene (no depth test, no fog) -----

const over = (o) => ({ transparent: true, depthTest: false, depthWrite: false, fog: false, ...o });
const M4 = new THREE.Matrix4();
const COL = { red: "#ff4040", orange: "#ff9a3c", amber: "#ffbf00", green: "#3ddc84", magenta: "#ff3cf0", cyan: "#3cd8ff", blue: "#5a8cff", white: "#ffffff", gold: "#ffd84a", violet: "#b07cff" };
const TIER = { major: COL.red, secondary: COL.amber, micro: COL.cyan, secret: COL.violet };

// Tiles ([[x, z, y]]) as flat squares, one instanced draw.
function tiles(list, color, opacity = 0.35, size = 0.9) {
  const m = new THREE.InstancedMesh(new THREE.PlaneGeometry(size, size).rotateX(-Math.PI / 2), new THREE.MeshBasicMaterial(over({ color, opacity })), Math.max(1, list.length));
  m.count = list.length;
  list.forEach(([x, z, y], i) => m.setMatrixAt(i, M4.makeTranslation(x + 0.5, y + 0.05, z + 0.5)));
  m.frustumCulled = false;
  m.renderOrder = 998;
  return m;
}
// Line segments from pairs of points ([[x, y, z], [x, y, z], …]).
function lines(pts, color, dashed = false) {
  const geo = new THREE.BufferGeometry().setFromPoints(pts.map((p) => new THREE.Vector3(...p)));
  const l = new THREE.LineSegments(geo, dashed ? new THREE.LineDashedMaterial(over({ color, dashSize: 0.3, gapSize: 0.2 })) : new THREE.LineBasicMaterial(over({ color })));
  if (dashed) l.computeLineDistances();
  l.frustumCulled = false;
  l.renderOrder = 999;
  return l;
}
// The 12 edges of a box, as segment pairs.
function edges(x0, y0, z0, x1, y1, z1) {
  const c = [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1], [x0, y1, z0], [x1, y1, z0], [x1, y1, z1], [x0, y1, z1]];
  return [0, 1, 1, 2, 2, 3, 3, 0, 4, 5, 5, 6, 6, 7, 7, 4, 0, 4, 1, 5, 2, 6, 3, 7].map((i) => c[i]);
}
const boxEdges = (b) => edges(b.x0, b.y0, b.z0, b.x1, b.y1, b.z1);
// A flat circle of radius r round (x, y, z), as segments.
function circle(x, y, z, r, n = 32) {
  const out = [];
  for (let i = 0; i < n; i++) for (const j of [i, i + 1]) out.push([x + Math.cos((j / n) * Math.PI * 2) * r, y, z + Math.sin((j / n) * Math.PI * 2) * r]);
  return out;
}
// An arrow on the ground from a to b ([x, y, z]).
function arrow(a, b) {
  const dx = b[0] - a[0], dz = b[2] - a[2], l = Math.hypot(dx, dz) || 1, ux = dx / l, uz = dz / l, h = 0.25;
  return [a, b, b, [b[0] - h * (ux - uz * 0.6), b[1], b[2] - h * (uz + ux * 0.6)], b, [b[0] - h * (ux + uz * 0.6), b[1], b[2] - h * (uz - ux * 0.6)]];
}
// A camera-facing label, its bottom centre at (x, y, z).
function label(text, x, y, z, color = COL.white) {
  const c = document.createElement("canvas"), g = c.getContext("2d"), font = "600 26px system-ui, sans-serif";
  g.font = font;
  c.width = Math.ceil(g.measureText(text).width) + 16;
  c.height = 36;
  g.fillStyle = "rgba(10, 16, 28, 0.8)";
  g.fillRect(0, 0, c.width, c.height);
  g.font = font;
  g.fillStyle = color;
  g.textBaseline = "middle";
  g.fillText(text, 8, 19);
  const t = new THREE.CanvasTexture(c);
  t.colorSpace = THREE.SRGBColorSpace;
  const s = new THREE.Sprite(new THREE.SpriteMaterial(over({ map: t })));
  s.center.set(0.5, 0);
  s.scale.set((c.width / c.height) * 0.36, 0.36, 1);
  s.position.set(x, y, z);
  s.renderOrder = 1000;
  return s;
}
// A solid marker: a small ball (filled) or a ring standing up (hollow).
function ball(x, y, z, color, filled = true, r = 0.2) {
  if (!filled) return lines([...circle(0, 0, 0, r, 16)].map(([a, b, c]) => [x + a, y + c, z]).concat(circle(x, y, z, r, 16)), color);
  const m = new THREE.Mesh(new THREE.SphereGeometry(r, 12, 8), new THREE.MeshBasicMaterial(over({ color })));
  m.position.set(x, y, z);
  m.renderOrder = 999;
  return m;
}
const layer = (count, ...kids) => {
  const g = new THREE.Group();
  g.add(...kids.flat().filter(Boolean));
  g.userData.count = count;
  return g;
};
const at = (c, x, z) => [x, z, c.w.floorAt(x, z)];
const mid = (c, x, z, up = 0) => [x + 0.5, c.w.floorAt(x, z) + up, z + 0.5];

// ----- The layers: ctx -> a Group whose userData.count is how many things it drew -----

const BUILD = {
  // Solid tiles (red) and the prop boxes the camera rays test against (orange), floor heights on raised ground.
  collisions(c) {
    const { w, m, g } = c, solid = [], raised = new Map();
    for (let z = 0; z < m.height; z++)
      for (let x = 0; x < m.width; x++) {
        if (w.blocked(x, z)) solid.push(at(c, x, z));
        const y = Math.round(w.floorAt(x, z) * 100) / 100;
        if (y > 0) raised.set(key(x, z), y);
      }
    // One height label per patch of tiles at that height.
    const labels = [], left = new Set(raised.keys());
    for (const k of raised.keys()) {
      if (!left.has(k)) continue;
      const y = raised.get(k), patch = [...ld.bfs(new Set([...left].filter((t) => raised.get(t) === y)), [k.split(",").map(Number)]).dist.keys()];
      patch.forEach((t) => left.delete(t));
      const [x, z] = k.split(",").map(Number);
      labels.push(label(`${y} m`, x + 0.5, y + 0.1, z + 0.5, COL.cyan));
    }
    return layer(solid.length + g.boxes.length, tiles(solid, COL.red, 0.3), lines(g.boxes.flatMap(boxEdges), COL.orange), labels);
  },

  // Reachable from the spawn with every gate open (green), gated (amber), open but unreachable (magenta), vistas.
  walkable(c) {
    const { w, m, g, all } = c, lost = [];
    for (let z = 0; z < m.height; z++)
      for (let x = 0; x < m.width; x++) {
        const t = m.tiles[z][x];
        if (!g.solid.has(key(x, z)) && !all.has(key(x, z)) && t !== "~" && !(t >= "1" && t <= "9")) lost.push(at(c, x, z));
      }
    const reach = [...all.keys()].map((k) => at(c, ...k.split(",").map(Number)));
    const gates = [...w.gateAt.keys()].map((k) => at(c, ...k.split(",").map(Number)));
    const vista = (m.vista ?? []).flatMap(([x0, z0, x1, z1]) => edges(x0, 0.1, z0, x1 + 1, 0.1, z1 + 1).slice(0, 8));
    return layer(reach.length, tiles(reach, COL.green, 0.22), tiles(gates, COL.amber, 0.5), tiles(lost, COL.magenta, 0.45), lines(vista, COL.cyan),
      (m.vista ?? []).map(([x0, z0]) => label("vista", x0 + 0.5, 0.2, z0 + 0.5, COL.cyan)));
  },

  // The path graph: path tiles, junctions with their ways out, dead ends, people's routes, the main route.
  circulation(c) {
    const { m, g, walk, all } = c, graph = ld.pathGraph(g);
    const path = [...g.path].map((k) => at(c, ...k.split(",").map(Number)));
    // One label per cluster of junction tiles, at its widest (as check-maps' crossroads).
    const left = new Set(graph.junctions.map((j) => key(j.x, j.z))), junctions = [];
    for (const j of [...graph.junctions].sort((a, b) => b.branches - a.branches)) {
      if (!left.has(key(j.x, j.z))) continue;
      for (const t of ld.bfs(left, [[j.x, j.z]]).dist.keys()) left.delete(t);
      junctions.push(label(`${j.branches} ways`, ...mid(c, j.x, j.z, 0.2), COL.gold));
    }
    const ends = graph.deadEnds.map((e) => label(`dead end ${Number.isFinite(e.length) ? e.length : "–"}`, ...mid(c, e.x, e.z, 0.2), COL.red));
    // Routes: one arrow per step of the loop.
    const routes = (m.npcs ?? []).filter((n) => n.route?.length).flatMap((n) => {
      let [x, z] = [n.x, n.z];
      return n.route.flatMap((f) => {
        const a = mid(c, x, z, 0.15);
        [x, z] = [x + DIR[f][0], z + DIR[f][1]];
        return arrow(a, mid(c, x, z, 0.15));
      });
    });
    // The main route: between the two entrances (the spawn, the exits' middles) farthest apart, as check-maps' report.
    const ends2 = [key(m.spawn.x, m.spawn.z), ...(m.exits ?? []).map((e) => key(...ld.edgeTile(m, e.side, Math.round((e.span[0] + e.span[1]) / 2))))].filter((t) => all.has(t));
    const xz = (t) => t.split(",").map(Number), far = (from) => {
      const r = walk([xz(from)]);
      return { r, to: ends2.reduce((a, b) => ((r.dist.get(b) ?? -1) > (r.dist.get(a) ?? -1) ? b : a)) };
    };
    const main = [];
    if (ends2.length > 1) {
      const a = far(ends2[0]).to, { r, to: b } = far(a);
      for (let t = b, p = r.parent.get(t); p; t = p, p = r.parent.get(p)) main.push(mid(c, ...xz(t), 0.12), mid(c, ...xz(p), 0.12));
    }
    return layer(graph.junctions.length + graph.deadEnds.length, tiles(path, COL.blue, 0.18), tiles(graph.junctions.map((j) => at(c, j.x, j.z)), COL.gold, 0.6),
      tiles(graph.deadEnds.map((e) => at(c, e.x, e.z)), COL.red, 0.6), junctions, ends, lines(routes, COL.cyan), lines(main, COL.white));
  },

  // Each mandatory interaction's camera ray from the tile it is used from: green seen, red hidden (its blocker
  // boxed), as check-maps' hidden-mandatory rule; duelists' sight lines.
  visibility(c) {
    const { w, m, g, floor, all, pois } = c, kids = [];
    let n = 0;
    for (const p of pois) {
      if (p.tier !== "major" || p.o?.optional) continue;
      const a = ld.approach(p, g, all, floor, FOOT);
      if (!a) continue;
      const b = ld.blocker(m, g.boxes, a.from, a.target), { o, e } = ld.ray(m, a.from, a.target), col = b ? COL.red : COL.green;
      n++;
      kids.push(lines([o, e], col), tiles([at(c, a.from.x, a.from.z)], col, 0.55), ball(...e, col, true, 0.09));
      if (b) {
        const box = new THREE.Mesh(new THREE.BoxGeometry(b.x1 - b.x0, b.y1 - b.y0, b.z1 - b.z0), new THREE.MeshBasicMaterial(over({ color: COL.red, opacity: 0.3 })));
        box.position.set((b.x0 + b.x1) / 2, (b.y0 + b.y1) / 2, (b.z0 + b.z1) / 2);
        kids.push(box, lines(boxEdges(b), COL.red), label(`hides ${p.what}`, (b.x0 + b.x1) / 2, b.y1 + 0.1, (b.z0 + b.z1) / 2, COL.red));
      }
    }
    const sight = w.npcs.filter((q) => q.def.trainer).flatMap((q) => (q.def.trainer.look ?? [q.face]).flatMap((f) => sightLine(w, q, f).map(([x, z]) => at(c, x, z))));
    return layer(n, kids, tiles(sight, COL.orange, 0.4));
  },

  pois: () => layer(0),
  npcs: () => layer(0),
  lights: () => layer(0),
  links: () => layer(0),
};

// The layers that follow what moves: a signature each frame, the layer redrawn when it changes.
const DYNAMIC = {
};
