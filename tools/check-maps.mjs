// Run: node tools/check-maps.mjs web/world
// Check every map: at most 40 x 32 tiles; each edge exit has a matching exit back, the
// edge tiles of both spans are walkable, every warp and exit leads to a map that exists,
// and no NPC stands on a solid tile or a blocking prop; NPC ids are unique on a map, and
// no two spots share a tile. Every player-facing string (lines, text, label, locked, title,
// a duelist's challenge / win / lose / after, and an interaction's speaker name) has a
// French `x_fr` twin of the same shape.
// Route duelists (`trainer`, WORLD_DESIGN §4.1): sight 0..5 (0: talk to duel, never watches), `look` directions valid, the
// first tile of each sight line walkable, and no sight line covering an exit's edge tile,
// the map's spawn or a tile a warp from elsewhere lands on (nobody is ambushed as a map
// fades in). Finds (`kind: "find"`): an id, a `give` and a `text`, on a solid tile
// (something to look in, never a featureless one).
// 1.3 (WORLD_DESIGN §7, §13.3): outdoor maps have `title_fr`, a `region` and a `music` track; `fence` and
// `cliff` name known sets. Gates: tiles in bounds; with every gate shut the spawn still reaches an exit or
// a warp (no soft-lock), and with every gate open every person, spot, warp and waymark can be reached
// (floor heights, the 0.6 m step and the ladder rule as build.js has them; a door warp to the same map, a
// lift, carries the fill to where it lands). When the hour turns and the `time:` gates of that phase shut,
// every tile the hero can stand on still has a way out (an exit, a door, a spot that warps). The atlas (atlas.json) lists
// every outdoor map under its own title, its roads join listed maps, and each waymark stands on a
// reachable tile facing its post (a `kind: "waymark"` spot of the same id); posts sit at least 1.7 grid
// squares apart on the chart, so their 44 px targets never overlap on a phone.
import { readFileSync, readdirSync, existsSync } from "node:fs";
const dir = process.argv[2] ?? "web/world";
const src = readFileSync(`${dir}/props.js`, "utf8");
const FOOT = eval(`(${src.match(/export const FOOT = (\{[\s\S]*?\});/)[1]})`);
const DECK = eval(`(${src.match(/export const DECK = (\{[\s\S]*?\});/)[1]})`);
const WALKABLE = new Set([...eval(`(${src.match(/export const WALKABLE = new Set\((\[[\s\S]*?\])\);/)[1]})`), ...Object.keys(DECK)]);
const ACROSS = eval(`(${src.match(/export const ACROSS = new Set\((\[[\s\S]*?\])\);/)[1]})`);
const LADDER = new Set(eval(`(${src.match(/export const LADDER = new Set\((\[[\s\S]*?\])\);/)[1]})`));
const TRACKS = new Set(["overworld", "ember", "storm", "tide", "dream", "grove", "stone", "cave"]);
const FENCES = new Set(["fence", "wall_dry", "hedge"]);
const atlas = existsSync(`${dir}/atlas.json`) ? JSON.parse(readFileSync(`${dir}/atlas.json`, "utf8")) : null;
const BACK = { n: "s", s: "n", e: "w", w: "e" };
const maps = Object.fromEntries(readdirSync(`${dir}/maps`).map((f) => [f.replace(".json", ""), JSON.parse(readFileSync(`${dir}/maps/${f}`, "utf8"))]));
const fail = [];
const SAID = ["lines", "text", "label", "locked", "title", "challenge", "win", "lose", "after"];
const DIR = { n: [0, -1], s: [0, 1], e: [1, 0], w: [-1, 0] };
// Tiles a warp from any map lands on, per map.
const landing = {};
for (const m of Object.values(maps)) for (const w of m.warps ?? []) (landing[w.to] ??= []).push(`${w.spawn.x},${w.spawn.z}`);
const said = (v) => typeof v === "string" || (Array.isArray(v) && v.every((s) => typeof s === "string"));
const french = (o, at, also = []) => {
  if (Array.isArray(o)) return o.forEach((v, i) => french(v, `${at}[${i}]`, also));
  if (!o || typeof o !== "object") return;
  for (const [k, v] of Object.entries(o)) {
    if (k.endsWith("_fr")) {
      const en = o[k.slice(0, -3)], ok = (s) => typeof s === "string" && s.trim() !== "";
      if (en === undefined) fail.push(`${at}.${k} has no English ${k.slice(0, -3)}`);
      else if (Array.isArray(en) ? !Array.isArray(v) || v.length !== en.length || !v.every(ok) : !ok(v)) fail.push(`${at}.${k} does not match the shape of ${k.slice(0, -3)}`);
    } else if ([...SAID, ...also].includes(k) && said(v)) {
      if (!(`${k}_fr` in o)) fail.push(`${at}.${k} has no ${k}_fr`);
    } else french(v, `${at}.${k}`, k === "interactions" ? ["name"] : []);
  }
};
for (const [name, m] of Object.entries(maps)) {
  french(m, name);
  if (m.width > 40 || m.height > 32) fail.push(`${name}: ${m.width} x ${m.height} is over 40 x 32`);
  if (!m.light.indoor) {
    if (!m.title || !m.title_fr) fail.push(`${name}: an outdoor map needs a title and title_fr`);
    if (!atlas?.regions.some((r) => r.id === m.region)) fail.push(`${name}: region ${m.region} is not in atlas.json`);
    if (!TRACKS.has(m.music)) fail.push(`${name}: music ${m.music} is not a track (${[...TRACKS].join(", ")})`);
  }
  if (m.fence !== undefined && !FENCES.has(m.fence)) fail.push(`${name}: fence ${m.fence} is not ${[...FENCES].join(" / ")}`);
  if (m.cliff !== undefined && !(Array.isArray(m.cliff) && m.cliff.length === 2 && m.cliff.every((c) => typeof c === "string"))) fail.push(`${name}: cliff is [block, top variant]`);
  const solid = new Set(), height = {};
  m.tiles.forEach((row, z) => [...row].forEach((c, x) => c >= "1" && c <= "9" && (height[`${x},${z}`] = +c)));
  m.tiles.forEach((row, z) => [...row].forEach((c, x) => "#fWV~".includes(c) && solid.add(`${x},${z}`)));
  for (const p of m.props) {
    let [w, d] = FOOT[p.asset] ?? [1, 1];
    if (p.rot % 180 === 90) [w, d] = [d, w];
    for (let x = p.x; x < p.x + w; x++)
      for (let z = p.z; z < p.z + d; z++) {
        const k = `${x},${z}`;
        if (WALKABLE.has(p.asset)) solid.delete(k);
        else solid.add(k);
        if (DECK[p.asset]) height[k] = (p.y ?? 0) + DECK[p.asset][0];
      }
  }
  for (const n of m.npcs ?? []) if (solid.has(`${n.x},${n.z}`)) fail.push(`${name}: ${n.id} stands on a solid tile`);
  reach(name, m, solid, height);
  const ids = (m.npcs ?? []).map((n) => n.id);
  ids.filter((id, i) => ids.indexOf(id) !== i).forEach((id) => fail.push(`${name}: two NPCs are called ${id}`));
  const spots = (m.interactions ?? []).map((i) => `${i.x},${i.z}`);
  spots.filter((k, i) => spots.indexOf(k) !== i).forEach((k) => fail.push(`${name}: two spots on tile ${k}`));

  // Duelists' sight lines, cast the way trainer.js casts them (people block; routes are not followed).
  const inMap = (x, z) => x >= 0 && z >= 0 && x < m.width && z < m.height;
  const people = new Set((m.npcs ?? []).map((n) => `${n.x},${n.z}`));
  const ambush = new Set([`${m.spawn.x},${m.spawn.z}`, ...(landing[name] ?? [])]);
  for (const e of m.exits ?? [])
    for (let c = e.span[0]; c <= e.span[1]; c++) ambush.add({ n: `${c},0`, s: `${c},${m.height - 1}`, w: `0,${c}`, e: `${m.width - 1},${c}` }[e.side]);
  for (const n of m.npcs ?? []) {
    const t = n.trainer;
    if (!t) continue;
    if (t.sight !== undefined && !(Number.isInteger(t.sight) && t.sight >= 0 && t.sight <= 5)) fail.push(`${name}: ${n.id} sight ${t.sight} is not 0..5`);
    const dirs = t.look ?? [n.facing ?? "s"];
    if (!dirs.every((d) => DIR[d])) fail.push(`${name}: ${n.id} looks a way that is not n/e/s/w`);
    for (const d of dirs.filter((d) => DIR[d])) {
      let [x, z] = [n.x, n.z];
      for (let i = 0; i < (t.sight ?? 3); i++) {
        const nx = x + DIR[d][0], nz = z + DIR[d][1], k = `${nx},${nz}`;
        const stop = !inMap(nx, nz) || solid.has(k) || people.has(k) || Math.abs((height[k] ?? 0) - (height[`${x},${z}`] ?? 0)) > 0.6;
        if (stop) {
          if (i === 0 && t.sight !== 0) fail.push(`${name}: ${n.id} looks ${d} straight into something`);
          break;
        }
        if (ambush.has(k)) fail.push(`${name}: ${n.id} watching ${d} covers ${k}, an exit, spawn or landing tile`);
        [x, z] = [nx, nz];
      }
    }
  }
  for (const i of m.interactions ?? []) {
    if (i.kind !== "find") continue;
    if (!i.id || !i.give || !i.text) fail.push(`${name}: a find at ${i.x},${i.z} needs an id, a give and a text`);
    if (!solid.has(`${i.x},${i.z}`)) fail.push(`${name}: find ${i.id} sits on an open tile; put it on something`);
  }
  for (const t of [...(m.warps ?? []), ...(m.exits ?? [])]) if (!maps[t.to]) fail.push(`${name}: leads to missing map ${t.to}`);
  for (const e of m.exits ?? []) {
    const back = maps[e.to]?.exits?.find((b) => b.side === BACK[e.side] && b.to === name);
    if (!back) fail.push(`${name}: exit ${e.side} to ${e.to} has no exit back`);
    for (let c = e.span[0]; c <= e.span[1]; c++) {
      const [x, z] = { n: [c, 0], s: [c, m.height - 1], w: [0, c], e: [m.width - 1, c] }[e.side];
      if (solid.has(`${x},${z}`)) fail.push(`${name}: exit ${e.side} tile ${x},${z} is blocked`);
    }
  }
}

// Gates and reachability: flood fill from the spawn over walkable tiles, a step at most 0.6 m up or
// down (a ladder joins its front and back whatever the height), first with every gate shut, then open.
function reach(name, m, solid0, height0) {
  const W = m.width, H = m.height, inMap = (x, z) => x >= 0 && z >= 0 && x < W && z < H;
  const ladders = new Map();
  for (const p of m.props) if (LADDER.has(p.asset)) ladders.set(`${p.x},${p.z}`, p.rot % 180 === 90 ? "x" : "z");
  // Deck rows, front first, as build.js lays them (check above only reads the first row).
  const height = { ...height0 };
  for (const p of m.props) {
    const deck = DECK[p.asset];
    if (!deck) continue;
    let [w, d] = FOOT[p.asset] ?? [1, 1];
    if (p.rot % 180 === 90) [w, d] = [d, w];
    for (let x = p.x; x < p.x + w; x++)
      for (let z = p.z; z < p.z + d; z++) {
        const row = { 0: p.z + d - 1 - z, 90: p.x + w - 1 - x, 180: z - p.z, 270: x - p.x }[p.rot % 360] ?? 0;
        height[`${x},${z}`] = (p.y ?? 0) + deck[Math.min(row, deck.length - 1)];
      }
  }
  const gates = m.gates ?? [];
  const ids = gates.map((g) => g.id);
  ids.filter((id, i) => !id || ids.indexOf(id) !== i).forEach((id) => fail.push(`${name}: gate ids must be set and unique (${id})`));
  for (const g of gates) {
    if (!Array.isArray(g.tiles) || !g.tiles.length) fail.push(`${name}: gate ${g.id} has no tiles`);
    for (const t of g.tiles ?? []) if (!inMap(t[0], t[1])) fail.push(`${name}: gate ${g.id} tile ${t} is off the map`);
    if (typeof g.if !== "string") fail.push(`${name}: gate ${g.id} has no if`);
    if (!g.text) fail.push(`${name}: gate ${g.id} needs a text for when it's shut`);
    if (g.floor !== undefined && !(g.floor >= 0)) fail.push(`${name}: gate ${g.id} floor ${g.floor} is below the ground (it would hide the hero's feet)`);
  }
  const flood = (open, starts = [[m.spawn.x, m.spawn.z]]) => {
    const solid = new Set(solid0), h = { ...height };
    for (const g of gates)
      for (const [x, z, y] of g.tiles ?? []) {
        const k = `${x},${z}`, c = m.tiles[z]?.[x];
        if (!(typeof open === "function" ? open(g) : open)) {
          solid.add(k);
          continue;
        }
        solid.delete(k);
        const deck = DECK[g.prop_open] ?? DECK[g.prop];
        const base = y ?? (c >= "1" && c <= "9" ? +c : 0);
        h[k] = g.floor ?? (deck ? base + deck[0] : h[k] ?? 0);
      }
    const step = (ax, az, bx, bz) => {
      if (Math.abs((h[`${bx},${bz}`] ?? 0) - (h[`${ax},${az}`] ?? 0)) <= 0.6) return true;
      const axis = ax !== bx ? "x" : "z";
      return ladders.get(`${ax},${az}`) === axis || ladders.get(`${bx},${bz}`) === axis;
    };
    const seen = new Set(starts.map(([x, z]) => `${x},${z}`)), queue = starts.map((t) => [...t]);
    // A door warp to this same map (a lift) carries the fill on to its landing.
    const lifts = (m.warps ?? []).filter((w) => w.to === name && !w.dir);
    for (;;) {
      while (queue.length) {
        const [x, z] = queue.shift();
        for (const [dx, dz] of Object.values(DIR)) {
          const nx = x + dx, nz = z + dz, k = `${nx},${nz}`;
          if (!inMap(nx, nz) || seen.has(k) || solid.has(k) || !step(x, z, nx, nz)) continue;
          seen.add(k);
          queue.push([nx, nz]);
        }
      }
      const next = lifts.find((w) => !seen.has(`${w.spawn.x},${w.spawn.z}`) && Object.values(DIR).some(([dx, dz]) => seen.has(`${w.x + dx},${w.z + dz}`)));
      if (!next) return seen;
      seen.add(`${next.spawn.x},${next.spawn.z}`);
      queue.push([next.spawn.x, next.spawn.z]);
    }
  };
  const near = (seen, x, z, across = false) =>
    seen.has(`${x},${z}`) || Object.values(DIR).some(([dx, dz]) => seen.has(`${x + dx},${z + dz}`) || (across && seen.has(`${x + 2 * dx},${z + 2 * dz}`)));
  const exitTiles = (m.exits ?? []).flatMap((e) => Array.from({ length: e.span[1] - e.span[0] + 1 }, (_, i) => {
    const c = e.span[0] + i;
    return { n: [c, 0], s: [c, H - 1], w: [0, c], e: [W - 1, c] }[e.side];
  }));
  const shut = flood(false);
  const out = exitTiles.some(([x, z]) => shut.has(`${x},${z}`)) || (m.warps ?? []).some((w) => near(shut, w.x, w.z));
  if (!out) fail.push(`${name}: with every gate shut, the spawn reaches no exit or warp (a soft-lock)`);
  const all = flood(true);
  for (const n of m.npcs ?? []) if (!near(all, n.x, n.z, true)) fail.push(`${name}: ${n.id} can't be reached, even with every gate open`);
  for (const i of m.interactions ?? []) if (!near(all, i.x, i.z, true)) fail.push(`${name}: the spot at ${i.x},${i.z} can't be reached, even with every gate open`);
  for (const w of m.warps ?? []) if (!near(all, w.x, w.z)) fail.push(`${name}: the warp at ${w.x},${w.z} can't be reached, even with every gate open`);
  // The hour turning: wherever the hero can stand (every gate open), once the gates on `time:` that are shut in
  // a phase have shut (items and flags never take a gate back), the way out must still be there: an edge exit, a
  // door, or a spot whose dialogue warps (a rope down). A gate never shuts under the hero, so its own tiles are
  // skipped; the hero steps off them onto one of the tiles tested here.
  const timed = gates.filter((g) => /time:/.test(g.if ?? ""));
  if (timed.length) {
    const outs = [...exitTiles];
    for (const w of m.warps ?? []) if (w.to !== name || w.dir) Object.values(DIR).forEach(([dx, dz]) => outs.push([w.x + dx, w.z + dz]));
    for (const i of m.interactions ?? []) if (JSON.stringify(i.dialogue ?? []).includes('"warp:')) Object.values(DIR).forEach(([dx, dz]) => outs.push([i.x + dx, i.z + dz]));
    const onGate = new Set(gates.flatMap((g) => (g.tiles ?? []).map(([x, z]) => `${x},${z}`)));
    // A gate's condition at phase `ph`, its items and flags taken as held.
    const at = (c, ph) => c.split("&").every((t) => {
      const neg = t.trim().startsWith("!"), term = t.trim().replace(/^!/, "");
      return !term.startsWith("time:") || (term.slice(5) === ph) !== neg;
    });
    for (const ph of ["dawn", "day", "dusk", "night"]) {
      const back = flood((g) => !/time:/.test(g.if) || at(g.if, ph), outs.filter(([x, z]) => inMap(x, z)));
      const stuck = [...all].find((t) => !onGate.has(t) && !back.has(t));
      if (stuck) fail.push(`${name}: at ${ph}, once the timed gates shut, ${stuck} has no way out (give it one: a rope with a warp:, a door)`);
    }
  }
  for (const wm of atlas?.waymarks ?? [])
    if (wm.map === name && !all.has(`${wm.x},${wm.z}`)) fail.push(`${name}: waymark ${wm.id}'s tile ${wm.x},${wm.z} can't be reached`);
  return all;
}

// The atlas.
if (!atlas) fail.push("atlas.json is missing");
else {
  const listed = new Map(atlas.maps.map((n) => [n.id, n]));
  for (const [name, m] of Object.entries(maps)) {
    if (m.light.indoor) continue;
    const n = listed.get(name);
    if (!n) fail.push(`atlas: outdoor map ${name} is not on the chart`);
    else if (n.name !== m.title || n.name_fr !== m.title_fr) fail.push(`atlas: ${name} is "${n.name}" / "${n.name_fr}" on the chart but titled "${m.title}" / "${m.title_fr}"`);
    else if (n.region !== m.region) fail.push(`atlas: ${name} is in ${n.region} on the chart but ${m.region} on the map`);
  }
  for (const n of atlas.maps) {
    if (!maps[n.id]) fail.push(`atlas: ${n.id} is not a map`);
    if (!(n.x >= 0 && n.x <= 12 && n.y >= 0 && n.y <= 9)) fail.push(`atlas: ${n.id} is off the 12 x 9 chart`);
  }
  for (const r of atlas.regions) if (!r.id || !r.name || !r.name_fr || !/^#[0-9a-f]{6}$/i.test(r.color)) fail.push(`atlas: region ${r.id} needs an id, name, name_fr and a #colour`);
  for (const l of atlas.links) {
    if (!listed.has(l.a) || !listed.has(l.b)) fail.push(`atlas: road ${l.a} - ${l.b} joins a map not on the chart`);
    if (l.if && !["lantern", "ferry", "boots", "gloves"].includes(l.icon)) fail.push(`atlas: gated road ${l.a} - ${l.b} needs an icon`);
  }
  const ids = atlas.waymarks.map((w) => w.id);
  ids.filter((id, i) => ids.indexOf(id) !== i).forEach((id) => fail.push(`atlas: two waymarks are called ${id}`));
  for (const w of atlas.waymarks) {
    const m = maps[w.map];
    if (!m || !listed.has(w.map)) {
      fail.push(`atlas: waymark ${w.id} is on ${w.map}, not a charted map`);
      continue;
    }
    if (!w.name || !w.name_fr || !DIR[w.facing]) fail.push(`atlas: waymark ${w.id} needs a name, name_fr and a facing`);
    const [px, pz] = [w.x + (DIR[w.facing]?.[0] ?? 0), w.z + (DIR[w.facing]?.[1] ?? 0)];
    if (!(m.interactions ?? []).some((i) => i.kind === "waymark" && i.id === w.id && i.x === px && i.z === pz)) fail.push(`atlas: waymark ${w.id} faces ${px},${pz} on ${w.map}, where there is no waymark spot of that id`);
  }
  for (const [name, m] of Object.entries(maps))
    for (const i of m.interactions ?? []) if (i.kind === "waymark" && !ids.includes(i.id)) fail.push(`${name}: waymark ${i.id} is not in atlas.json`);
  const at = (w) => listed.get(w.map) ?? { x: 0, y: 0 };
  for (let i = 0; i < atlas.waymarks.length; i++)
    for (let j = i + 1; j < atlas.waymarks.length; j++) {
      const a = at(atlas.waymarks[i]), b = at(atlas.waymarks[j]);
      if (Math.hypot(a.x - b.x, a.y - b.y) < 1.7) fail.push(`atlas: waymarks ${atlas.waymarks[i].id} and ${atlas.waymarks[j].id} are under 1.7 squares apart on the chart`);
    }
}
if (fail.length) throw new Error(fail.join("\n"));
console.log(`maps ok (${Object.keys(maps).join(", ")})`);
