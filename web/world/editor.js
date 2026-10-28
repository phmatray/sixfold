// The map editor (WORLD_DESIGN §13.3): debug builds only (`trunk serve`), loaded
// by `?edit` in the URL. It edits the current map as written in its file (not the
// localized, expanded copy the game plays), rebuilds the scene after every change
// and downloads the JSON on Ctrl+S (keys in their file order, new entries in a
// fixed one). Dialogue and other text are still written by hand in the JSON.
//
// Tools: Walk (play as usual), Paint (a tile character), Prop (place; R turns the
// next one, or the last one placed), Scatter (two clicks: corners of a rectangle
// filled with the prop at the density), Person (an NPC; an id starting `t_` is a
// route duelist, drawn with its sight lines), Find, Erase (whatever is on the tile).
import * as THREE from "three";
import { setMap } from "./map.js";
import { FOOT } from "./props.js";
import { sightLine } from "./duelist.js";

const TILES = [".", "p", "s", "r", "w", "=", "H", "~", "#", "f", "W", "V", "1", "2", "3"];
const PROPS = ["tree", "pine", "birch", "bush", "flowers", "rock", "reeds", "stump", "log", "crates", "bench", "lantern", "sign", "signpost",
  "stall", "stall_b", "curio_cabinet", "hanging_sign", "fountain", "cottage", "workshop", "house", "lighthouse", "boat", "bridge", "stairs",
  "cliff", "waymark", "seal_stone", "cave_mouth", "climb_wall", "inn_sign",
  "counter", "shelf", "bookcase", "duel_table", "chair", "rug", "plant", "display_case", "lamp", "exit_mat",
  "oven", "printing_press", "type_case", "bed", "workbench", "light_box", "net_locker"];
const TOOLS = ["walk", "paint", "prop", "scatter", "person", "find", "erase"];

const el = (tag, props = {}, ...kids) => {
  const n = Object.assign(document.createElement(tag), props);
  n.append(...kids);
  return n;
};
const select = (list, value) => {
  const s = el("select", {}, ...list.map((v) => el("option", { value: v, textContent: v })));
  s.value = value;
  return s;
};

export function attach({ el: host, canvas, world, tileAt, rebuild }) {
  let raw = null, name = "", tool = "walk", rot = 0, corner = null, last = null, busy = false;

  // ----- The panel -----
  const tools = select(TOOLS, tool), tiles = select(TILES, "p"), props = select(PROPS, "tree");
  const y = el("input", { type: "number", value: 0, step: 0.5, style: "width:4em" });
  const density = el("input", { type: "range", min: 0.05, max: 1, step: 0.05, value: 0.35 });
  const status = el("div", { style: "opacity:.8" });
  const panel = el("div", { style: "position:absolute;left:12px;top:76px;z-index:5;display:grid;gap:6px;padding:10px;border-radius:12px;background:rgba(13,26,44,.86);color:#f6ecd2;font:13px system-ui;max-width:220px" },
    el("b", { textContent: "Map editor" }),
    el("label", {}, "Tool ", tools), el("label", {}, "Tile ", tiles), el("label", {}, "Prop ", props), el("label", {}, "y ", y), el("label", {}, "Density ", density),
    el("button", { type: "button", textContent: "Save JSON (Ctrl+S)", onclick: () => save() }),
    status);
  host.append(panel);
  tools.onchange = () => ((tool = tools.value), (corner = null), say());
  const say = (msg = "") => (status.textContent = `${name} · ${tool} · rot ${rot}${msg ? ` · ${msg}` : ""}`);

  // The map file, fetched fresh (the game's copy is localized and expanded).
  async function load() {
    name = world().name;
    raw = await fetch(new URL(`maps/${name}.json`, import.meta.url), { cache: "no-store" }).then((r) => r.json());
    say();
    overlay();
  }

  async function apply(msg) {
    if (busy) return;
    busy = true;
    setMap(name, raw);
    await rebuild();
    busy = false;
    overlay();
    say(msg);
  }

  // Sight lines of route duelists, drawn on the floor (every `look` direction).
  function overlay() {
    const w = world();
    if (!w) return;
    const g = new THREE.Group(), quad = new THREE.PlaneGeometry(0.8, 0.8).rotateX(-Math.PI / 2);
    const mat = new THREE.MeshBasicMaterial({ color: "#ff5a3c", transparent: true, opacity: 0.45, depthWrite: false });
    for (const n of w.npcs.filter((m) => m.def.duelist))
      for (const f of n.def.duelist.look ?? [n.face])
        for (const [x, z] of sightLine(w, n, f)) {
          const m = new THREE.Mesh(quad, mat);
          m.position.set(x + 0.5, w.floorAt(x, z) + 0.06, z + 0.5);
          g.add(m);
        }
    g.name = "editor-sight";
    w.root.add(g);
  }

  const free = (x, z) => !world().blocked(x, z) && !world().npcAt(x, z);
  const prop = (asset, x, z) => ({ asset, x, z, rot, ...(+y.value ? { y: +y.value } : {}) });
  const at = (list, x, z) => (list ?? []).findIndex((o) => o.x === x && o.z === z);
  const under = (x, z) =>
    raw.props.findIndex((p) => {
      let [fw, fd] = FOOT[p.asset] ?? [1, 1];
      if (p.rot % 180 === 90) [fw, fd] = [fd, fw];
      return x >= p.x && x < p.x + fw && z >= p.z && z < p.z + fd;
    });

  async function click(x, z) {
    if (x < 0 || z < 0 || x >= raw.width || z >= raw.height) return;
    if (tool === "paint") {
      raw.tiles[z] = raw.tiles[z].slice(0, x) + tiles.value + raw.tiles[z].slice(x + 1);
      return apply(`painted ${x},${z}`);
    }
    if (tool === "prop") {
      raw.props.push((last = prop(props.value, x, z)));
      return apply(`${props.value} at ${x},${z}`);
    }
    if (tool === "scatter") {
      if (!corner) return (corner = [x, z]), say("now the other corner");
      const [x0, x1] = [Math.min(x, corner[0]), Math.max(x, corner[0])], [z0, z1] = [Math.min(z, corner[1]), Math.max(z, corner[1])];
      corner = null;
      let n = 0;
      for (let i = x0; i <= x1; i++) for (let k = z0; k <= z1; k++) if (Math.random() < +density.value && free(i, k) && under(i, k) < 0) raw.props.push({ asset: props.value, x: i, z: k, rot: 0 }), n++;
      return apply(`${n} × ${props.value}`);
    }
    if (tool === "person") {
      const id = prompt("NPC id (t_<name> for a route duelist)");
      if (!id) return;
      const nm = id.replace(/^t_/, "");
      const npc = { id, name: nm[0].toUpperCase() + nm.slice(1), model: "npc_wanderer", x, z, facing: "s", idle: "Idle" };
      if (id.startsWith("t_")) npc.duelist = { sight: 3, challenge: ["…"], challenge_fr: ["…"], win: ["…"], win_fr: ["…"], lose: ["…"], lose_fr: ["…"], after: ["…"], after_fr: ["…"] };
      else npc.dialogue = [{ lines: ["…"], lines_fr: ["…"] }];
      (raw.npcs ??= []).push(npc);
      return apply(`${id} at ${x},${z}`);
    }
    if (tool === "find") {
      const id = prompt("Find id (e.g. m_stump)"), give = id && prompt("Gives (coins:40, card:<slug>, give:<item>, pack:<booster>)", "coins:40");
      if (!give) return;
      raw.interactions.push({ kind: "find", id, x, z, give, hidden: false, text: "…", text_fr: "…" });
      return apply(`find ${id}`);
    }
    if (tool === "erase") {
      for (const list of [raw.npcs, raw.interactions, raw.warps]) {
        const i = at(list, x, z);
        if (i >= 0) return list.splice(i, 1), apply(`erased at ${x},${z}`);
      }
      const i = under(x, z);
      if (i >= 0) return raw.props.splice(i, 1), apply(`erased ${x},${z}`);
    }
  }

  // Taps edit instead of walking (except with the Walk tool).
  const onDown = (e) => {
    if (tool === "walk" || !raw) return;
    e.stopImmediatePropagation();
    const t = tileAt(e.clientX, e.clientY);
    if (t) click(...t);
  };
  const onKey = (e) => {
    if (e.target.closest?.("input, select")) return;
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") return e.preventDefault(), save();
    if (e.key.toLowerCase() === "r" && tool !== "walk") {
      rot = (rot + 90) % 360;
      if (last) (last.rot = rot), apply("turned");
      else say();
    }
  };
  function save() {
    const blob = new Blob([JSON.stringify(raw, null, 2) + "\n"], { type: "application/json" });
    const a = el("a", { href: URL.createObjectURL(blob), download: `${name}.json` });
    a.click();
    URL.revokeObjectURL(a.href);
    say("saved");
  }
  canvas.addEventListener("pointerdown", onDown, true);
  addEventListener("keydown", onKey);
  // A warp or an edge exit took the hero to another map: edit that one.
  const watch = setInterval(() => world() && world().name !== name && !busy && ((last = null), load()), 500);
  load();
  return {
    editing: () => tool !== "walk",
    stop() {
      clearInterval(watch);
      canvas.removeEventListener("pointerdown", onDown, true);
      removeEventListener("keydown", onKey);
      panel.remove();
    },
  };
}
