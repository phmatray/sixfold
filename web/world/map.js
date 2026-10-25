// Loading, cached for the session: maps (./maps/<name>.json, `x_fr` swapped in
// for `x` on a French page), the models a map uses, and the idle prefetch of the
// maps next door. Map blocks that expand into plainer data before a build (a
// `trainer` into dialogue branches, finds, waymarks) belong in `loadMap`, after `localize`.
import { GLTFLoader } from "three/addons/loaders/GLTFLoader.js";
import { groundLayers, layer } from "./ground.js";
import { SUFFIX_ARMS } from "./props.js";

const BASE = new URL("./", import.meta.url);
const INTERIOR = new Set(["counter", "shelf", "duel_table", "chair", "rug", "plant", "display_case", "lamp", "exit_mat"]);

const loader = new GLTFLoader();
const assets = new Map();
export const PERSON = /^(npc|rival)_/; // models in assets/npc/
const folder = (n) => (INTERIOR.has(n) || n.startsWith("wall") ? "interior/" : PERSON.test(n) ? "npc/" : "");
export const loadAsset = (n) => {
  if (!assets.has(n)) assets.set(n, loader.loadAsync(new URL(`assets/${folder(n)}${n}.glb`, BASE).href).catch(() => null));
  return assets.get(n);
};
// French when the page is (the app sets <html lang>): a map's `x_fr` replaces its `x`.
const FR = document.documentElement.lang === "fr";
export const tr = (en, fr) => (FR ? fr : en);
const localize = (o) => {
  if (Array.isArray(o)) o.forEach(localize);
  else if (o && typeof o === "object")
    for (const k of Object.keys(o)) {
      if (FR && k.endsWith("_fr")) o[k.slice(0, -3)] = o[k];
      else localize(o[k]);
    }
  return o;
};
const maps = new Map();
export const loadMap = (name) => {
  if (!maps.has(name))
    maps.set(name, fetch(new URL(`maps/${name}.json`, BASE)).then((r) => (r.ok ? r.json().then(localize) : Promise.reject(new Error(`map ${name}: HTTP ${r.status}`)))));
  return maps.get(name);
};
export const OPTIONAL = /_b$|_c$|_dark$/; // variants: used when present, never missed
export const RING = ["tree", "tree_b", "tree_c", "bush_dark"]; // the default band of trees outside an outdoor map

// Every model a map can use, so they load in parallel before it is built.
export function assetNames(map) {
  const names = new Set(["hero", ...map.props.map((p) => p.asset), ...(map.npcs ?? []).map((n) => n.model).filter(Boolean)]);
  const tiles = map.tiles.join("");
  const pieces = (kind) => Object.keys(SUFFIX_ARMS).forEach((s) => names.add(kind + s));
  if (tiles.includes("f")) pieces("fence");
  if (/[WV]/.test(tiles)) pieces("wall"), pieces("wall_low"), names.add("wall_window");
  if (/[1-9]/.test(tiles)) names.add("cliff"), names.add("cliff_b");
  if (tiles.includes("=")) names.add("boardwalk");
  if (tiles.includes("H")) names.add("pier");
  if (!map.light.indoor) [...(map.ring ?? RING), "grass_edge"].forEach((n) => names.add(n));
  if (names.has("tree")) names.add("tree_b"), names.add("tree_c");
  return [...names];
}

// Fetch and parse the maps, models and ground layers next door, one at a time while
// idle (`alive()`: false once the world is unmounted).
export function preload(map, alive) {
  const idle = (f) => (globalThis.requestIdleCallback ? requestIdleCallback(f, { timeout: 2000 }) : setTimeout(f, 300));
  const queue = [];
  const pump = () => idle(() => alive() && queue.length && loadAsset(queue.shift()).finally(pump));
  const next = new Set([...(map.exits ?? []), ...(map.warps ?? [])].map((e) => e.to));
  idle(() =>
    next.forEach((n) =>
      loadMap(n)
        .then((m) => {
          groundLayers(m).forEach(layer);
          const was = queue.length;
          queue.push(...assetNames(m).filter((a) => !assets.has(a)));
          if (!was) pump();
        })
        .catch(() => {}),
    ),
  );
}
