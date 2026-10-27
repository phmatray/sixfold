// World debug overlays (#111): debug builds only (`trunk serve`), loaded by `?debug` in the URL, the way
// `?edit` loads editor.js (release builds never import this file). Eight layers drawn in the 3D scene, so they
// read at the real camera and show in screenshots: keys 1-8 (or the panel) toggle them, 0 hides the HUD and
// the hero for a clean capture. From the console or a test: `await el.sixfoldDebug(["visibility", "pois"])`
// turns exactly those layers on ("clean" in the list too), and returns what each layer drew.
// The geometry is web/world/ld.js, the analysis tools/check-maps.mjs judges maps with, run on the map's file
// as written (not the localized copy the game plays), so the overlay and CI agree.
// Dev tooling: its labels are English only, like the editor's.
import * as THREE from "three";

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
  // The map's analysis, once per map: its file as written (as check-maps reads it).
  async function prepare(w) {
    const raw = await fetch(new URL(`maps/${w.name}.json`, import.meta.url), { cache: "no-store" }).then((r) => r.json());
    return { w, raw };
  }
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
      ready = prepare(w).then((c) => {
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

// One builder per layer: ctx -> a Group whose userData.count is how many things it drew.
const layer = (count = 0, ...kids) => {
  const g = new THREE.Group();
  if (kids.length) g.add(...kids);
  g.userData.count = count;
  return g;
};
const BUILD = Object.fromEntries(LAYERS.map((l) => [l, () => layer()]));
// The layers that follow what moves (people, the live lights): a signature per frame, redrawn when it changes.
const DYNAMIC = {};
