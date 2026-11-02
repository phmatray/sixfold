// Loading, cached for the session: maps (./maps/<name>.json, `x_fr` swapped in
// for `x` on a French page), the models a map uses, and the idle prefetch of the
// maps next door. Map blocks that expand into plainer data before a build (a
// `duelist` into dialogue branches, finds, waymarks) belong in `loadMap`, after `localize`.
import { GLTFLoader } from "three/addons/loaders/GLTFLoader.js";
import { groundLayers, layer } from "./ground.js";
import { SUFFIX_ARMS } from "./props.js";

const BASE = new URL("./", import.meta.url);
const INTERIOR = new Set(["counter", "shelf", "duel_table", "chair", "rug", "plant", "display_case", "lamp", "exit_mat",
  "oven", "printing_press", "type_case", "bookcase", "bed", "workbench", "light_box", "net_locker",
  "chart_table", "dress_form", "cloth_rack", "paper_lanterns", "chowder_stove",
  "cobbler_bench", "boot_rack", "hold_wall", "rope_coil", "tally_board", "soup_cauldron"]);

const loader = new GLTFLoader();
const assets = new Map();
export const PERSON = /^(npc|rival)_/; // models in assets/npc/
const WALL = /^wall(_low)?(|_corner|_end|_t|_window)$/; // the interior wall kit (not the outdoor `wall_dry` set)
const folder = (n) => (INTERIOR.has(n) || WALL.test(n) ? "interior/" : PERSON.test(n) ? "npc/" : "");
const BUILT = new Set(["seal_door", "inn_stairs", "inn_stairwell", "hedge_gap", "crates_open", "stone_odd", "stone_moved"]); // drawn in code only (props.js PLACEHOLDER): nothing to fetch
export const loadAsset = (n) => {
  if (BUILT.has(n)) return Promise.resolve(null);
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
    maps.set(name, fetch(new URL(`maps/${name}.json`, BASE)).then((r) => (r.ok ? r.json().then((m) => expand(localize(m))) : Promise.reject(new Error(`map ${name}: HTTP ${r.status}`)))));
  return maps.get(name);
};

// The editor (editor.js) previews a map it changed: `raw` is the map as written in its file.
export const setMap = (name, raw) => maps.set(name, Promise.resolve(expand(localize(structuredClone(raw)))));

// Shorthand blocks become plain dialogue, so one grammar drives everything
// (WORLD_DESIGN §4.1, §5.1; meta/tests/world_data.rs checks the same expansion).
// A route duelist (`duelist`): won: -> `win`, lost: -> `lose`, then any hand-written
// `dialogue`, beat: -> `after`, else `challenge` and the duel offer.
// A find (`kind: "find"`): `text` and its `give` once (flag found_<id>), a glint
// until then unless `hidden`, and a shrug after.
// A waymark (`kind: "waymark"`, §7.3): the first touch lights it (flag way_<id>),
// and every touch opens the world map to travel.
// A `secret` block (#116) sets flag:secret_<id> on discovery: a find's take (and its shrug after), or
// every branch of any other spot or NPC.
export function expand(map) {
  for (const n of map.npcs ?? []) {
    // A map cached by an older build (the service worker keeps one) still names the block `trainer` (#192).
    const t = (n.duelist ??= n.trainer);
    if (!t) continue;
    const id = n.id, say = (lines, extra) => (lines?.length ? [{ ...extra, lines }] : []);
    n.dialogue = [
      ...say(t.win, { if: `won:${id}` }),
      ...say(t.lose, { if: `lost:${id}` }),
      ...(n.dialogue ?? []),
      ...say(t.after, { if: `beat:${id}` }),
      { lines: t.challenge, do: [`duel:${id}`] },
    ];
  }
  for (const i of map.interactions ?? []) {
    if (i.kind !== "find") continue;
    const flag = `found_${i.id}`;
    if (!i.hidden) i.glint = i.glint ? `${i.glint} & !flag:${flag}` : `!flag:${flag}`;
    i.name ??= "";
    const secret = i.secret ? [`flag:secret_${i.secret.id}`] : [];
    // The shrug sets it too: a save that looted the find before secrets existed earns it by looking again.
    i.dialogue = [{ if: `!flag:${flag}`, lines: [i.text], do: [i.give, `flag:${flag}`, ...secret] }, { lines: [tr("Nothing else here.", "Plus rien ici.")], ...(i.secret && { do: secret }) }];
  }
  for (const i of map.interactions ?? []) {
    if (i.kind !== "waymark") continue;
    const flag = `way_${i.id}`;
    i.name ??= tr("Waymark", "Borne de voyage");
    i.dialogue = [
      {
        if: `!flag:${flag}`,
        lines: [tr("The crystal on the post wakes under your hand and hums, a note that seems to come from far away.",
          "Le cristal de la borne s'éveille sous votre main et se met à bourdonner, une note qui semble venir de très loin."),
        tr("Lit waymarks are linked. From any of them, or from your map whenever you're outdoors, you can travel to another.",
          "Les bornes allumées sont reliées. Depuis l'une d'elles, ou depuis votre carte dès que vous êtes dehors, vous pouvez voyager jusqu'à une autre.")],
        do: [`flag:${flag}`, "show:map"],
      },
      { lines: [tr("The waymark hums. Where to?", "La borne bourdonne. Où aller\u202f?")], do: ["show:map"] },
    ];
  }
  // Any other spot or NPC with a `secret` block: meeting it is the discovery, so every branch sets it
  // (first, so a short `pay:` can't skip it).
  for (const o of [...(map.npcs ?? []), ...(map.interactions ?? [])]) {
    if (!o.secret || o.kind === "find") continue;
    for (const b of o.dialogue ?? []) b.do = [`flag:secret_${o.secret.id}`, ...[b.do ?? []].flat()];
  }
  return map;
}
export const OPTIONAL = /_b$|_c$|_dark$/; // variants: used when present, never missed
export const RING = ["tree", "tree_b", "tree_c", "bush_dark"]; // the default band of trees outside an outdoor map

// Every model a map can use, so they load in parallel before it is built.
export function assetNames(map) {
  const names = new Set(["hero", ...map.props.map((p) => p.asset), ...(map.npcs ?? []).map((n) => n.model).filter(Boolean)]);
  const tiles = map.tiles.join("");
  const pieces = (kind) => Object.keys(SUFFIX_ARMS).forEach((s) => names.add(kind + s));
  if (tiles.includes("f")) pieces(map.fence ?? "fence");
  if (/[WV]/.test(tiles)) pieces("wall"), pieces("wall_low"), names.add("wall_window");
  for (const g of map.gates ?? []) for (const n of [g.prop, g.prop_closed, g.prop_open]) if (n) names.add(n);
  if (tiles.includes("=")) names.add("boardwalk");
  if (tiles.includes("H")) names.add("pier");
  if (!map.light.indoor) (map.ring ?? RING).forEach((n) => names.add(n));
  if (names.has("tree")) names.add("tree_b"), names.add("tree_c");
  for (const l of map.lifts ?? []) names.add(l.cage ?? "lift_cage");
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
