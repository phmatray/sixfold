// Run: node tools/check-maps.mjs web/world
// Check every map: at most 80 x 72 tiles (80 x 64 in all); each edge exit has a matching exit back, the
// edge tiles of both spans are walkable, every warp and exit leads to a map that exists,
// and no NPC stands on a solid tile or a blocking prop; a door on a prop has its STOOP (props.js); NPC ids are unique on a map, and
// no two spots share a tile. Every player-facing string (lines, text, label, locked, title,
// a duelist's challenge / win / lose / after, and an interaction's speaker name) has a
// French `x_fr` twin of the same shape.
// Route duelists (`trainer`, WORLD_DESIGN §4.1): sight 0..5 (0: talk to duel, never watches), `look` directions valid, the
// first tile of each sight line walkable, and no sight line covering an exit's edge tile,
// the map's spawn or a tile a warp from elsewhere lands on (nobody is ambushed as a map
// fades in). Finds (`kind: "find"`): an id, a `give` and a `text`, on a solid tile
// (something to look in, never a featureless one).
// 1.3 (WORLD_DESIGN §7, §13.3): outdoor maps have `title_fr`, a `region` and a `music` track (an optional `ambience` names a bed); `fence` and
// `cliff` name known sets; optional `paint` rows (a ground layer on any tile, raised ones too) match `tiles`. Gates: tiles in bounds; with every gate shut the spawn still reaches an exit or
// a warp (no soft-lock), and with every gate open every person, spot, warp and waymark can be reached
// (floor heights, the 0.6 m step and the ladder rule as build.js has them; a door warp to the same map
// carries the fill to where it lands, and a lift (`lifts`, lift.js) joins its shaft and the landings at its
// two heights). A lift: an id, its shaft tile in bounds, a `top` above its bottom, a landing at each height. When the hour turns and the `time:` gates of that phase shut,
// every tile the hero can stand on still has a way out (an exit, a door, a spot that warps). The atlas (atlas.json) lists
// every outdoor map under its own title, its roads join listed maps, and each waymark stands on a
// reachable tile facing its post (a `kind: "waymark"` spot of the same id); posts sit at least 1.7 grid
// squares apart on the chart, so their 44 px targets never overlap on a phone.
// World rules (docs/WORLD_RULES.md, errors with --strict): clearance (no shared footprints, a free tile in front of every
// post, nothing tall just in front of what you use, a tile between posts and other things you use, waymarks clear all
// round), lamps by paths and with a `why` out of town, routes on open tiles, doors out landing in front of the door back
// in, size per `kind` in screens, exits 3 wide with a path to the edge, an open throat and a signpost naming where they
// go, and a closed border everywhere else. No street lamp or house lamp in a cave (an error even without --strict).
import { readFileSync, readdirSync, existsSync } from "node:fs";
const dir = process.argv.slice(2).find((a) => !a.startsWith("--")) ?? "web/world";
const src = readFileSync(`${dir}/props.js`, "utf8");
const FOOT = eval(`(${src.match(/export const FOOT = (\{[\s\S]*?\});/)[1]})`);
const DECK = eval(`(${src.match(/export const DECK = (\{[\s\S]*?\});/)[1]})`);
const WALKABLE = new Set([...eval(`(${src.match(/export const WALKABLE = new Set\((\[[\s\S]*?\])\);/)[1]})`), ...Object.keys(DECK)]);
const ACROSS = new Set(eval(`(${src.match(/export const ACROSS = new Set\((\[[\s\S]*?\])\);/)[1]})`));
const TALL = eval(`(${src.match(/export const TALL = (\{[\s\S]*?\});/)[1]})`);
const SEAT = eval(`(${src.match(/export const SEAT = (\{[\s\S]*?\});/)[1]})`);
const STOOP = eval(`(${src.match(/export const STOOP = (\{[\s\S]*?\} \});/)[1]})`);
// Heights (m) for depth stacking, from ASSETS.md / ASSETS_V13.md; TALL (props.js) fills the rest.
const HEIGHT = { tree: 3.5, pine: 3.8, birch: 3.6, willow: 3.8, house: 5, sign: 1.3, signpost: 1.6, lantern: 2.2, bush: 0.7, flowers: 0.3,
  rock: 0.6, crates: 0.9, glow_mushroom: 0.35, glow_mushroom_tall: 0.9, glow_shelf: 0.62, bench: 0.9, stall: 2.3, stall_b: 2.3, fountain: 1.6, stump: 0.55, log: 0.55, hanging_sign: 2.5, boat: 0.4, reeds: 0.9,
  shelf: 2, bookcase: 2.3, oven: 2.45, printing_press: 1.8, workbench: 1.8, net_locker: 1.95, plant: 1, chair: 0.9, display_case: 1.1, lamp: 1.6,
  bed: 1.1, light_box: 1.25, type_case: 1.35, curio_cabinet: 2.3, seal_stone: 1.9, cave_mouth: 2.2, crab_pots: 0.45, net_rack: 1.45, bollard: 0.6, rock_sea: 0.55 };
const TALL_OF = (a) => TALL[a] ?? 1;
const LADDER = new Set(eval(`(${src.match(/export const LADDER = new Set\((\[[\s\S]*?\])\);/)[1]})`));
const TRACKS = new Set(["overworld", "ember", "storm", "tide", "dream", "grove", "stone", "cave"]);
const BEDS = new Set([...TRACKS, "interior"]); // audio.js BEDS
const THEMES = new Set([...BEDS, "rook", "finale"]); // a scene's `music:` may also be a theme of its own (engine overworld::TRACKS)
const FENCES = new Set(["fence", "wall_dry", "hedge"]);
const atlas = existsSync(`${dir}/atlas.json`) ? JSON.parse(readFileSync(`${dir}/atlas.json`, "utf8")) : null;
const BACK = { n: "s", s: "n", e: "w", w: "e" };
const maps = Object.fromEntries(readdirSync(`${dir}/maps`).map((f) => [f.replace(".json", ""), JSON.parse(readFileSync(`${dir}/maps/${f}`, "utf8"))]));
const fail = [], warn = [];
const open = {}; // per map: is a tile open ground (for warp: cues from other maps)
// The placement rules (docs/WORLD_RULES.md) fail the build with --strict (as CI runs it); without it they are listed
// as warnings, for a map in progress.
const STRICT = process.argv.includes("--strict");
const rule = [];
// One screen at the default camera, measured: about 13 tiles across and 9 deep on a 1440 x 900 desktop (a phone
// in portrait sees about 7 x 13). Map size limit: 80 x 72, at most 80 x 64 tiles in all (built in ~0.6 s with the CPU
// slowed 4x; the scenery is culled in 16-tile chunks, so what's drawn doesn't grow with the map).
const SCREEN = [13, 9], MAX = [80, 72, 80 * 64];
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
  if (m.width > MAX[0] || m.height > MAX[1] || m.width * m.height > MAX[2]) fail.push(`${name}: ${m.width} x ${m.height} is over ${MAX[0]} x ${MAX[1]} or ${MAX[2]} tiles`);
  if (!m.light.indoor) {
    if (!m.title || !m.title_fr) fail.push(`${name}: an outdoor map needs a title and title_fr`);
    if (!atlas?.regions.some((r) => r.id === m.region)) fail.push(`${name}: region ${m.region} is not in atlas.json`);
    if (!TRACKS.has(m.music)) fail.push(`${name}: music ${m.music} is not a track (${[...TRACKS].join(", ")})`);
  }
  if (m.ambience !== undefined && !BEDS.has(m.ambience)) fail.push(`${name}: ambience ${m.ambience} is not a bed (${[...BEDS].join(", ")})`);
  if (m.fence !== undefined && !FENCES.has(m.fence)) fail.push(`${name}: fence ${m.fence} is not ${[...FENCES].join(" / ")}`);
  if (m.cliff !== undefined && !(Array.isArray(m.cliff) && m.cliff.length === 2 && m.cliff.every((c) => typeof c === "string"))) fail.push(`${name}: cliff is [block, top variant]`);
  if (m.paint !== undefined && !(Array.isArray(m.paint) && m.paint.length === m.tiles.length && m.paint.every((r, z) => r.length === m.tiles[z].length && /^[.psr]*$/.test(r))))
    fail.push(`${name}: paint is one row per tile row, the same length, of . p s r`);
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
  // A lift's shaft stays walkable under its tower's footprint (lift.js); it needs a landing at both ends.
  for (const l of m.lifts ?? []) {
    const k = `${l.x},${l.z}`;
    solid.delete(k);
    if (!l.id || (m.lifts.filter((o) => o.id === l.id).length > 1)) fail.push(`${name}: lift ids must be set and unique (${l.id})`);
    if (!(l.x >= 0 && l.z >= 0 && l.x < m.width && l.z < m.height)) fail.push(`${name}: lift ${l.id} is off the map`);
    if (!(l.top > (l.bottom ?? height[k] ?? 0))) fail.push(`${name}: lift ${l.id} needs a top above its bottom`);
    for (const y of [l.bottom ?? height[k] ?? 0, l.top])
      if (!liftLandings(l, y, solid, height).length) fail.push(`${name}: lift ${l.id} has no landing at ${y} m`);
  }
  // Seats (seat.js): every footprint tile of a bench or chair is a place, sat on from in front or beside (not from
  // behind). A person with `sit: true` sits on one; a seat nobody can walk up to must say `seat: false` (scenery).
  const FACE = { 0: "s", 90: "e", 180: "n", 270: "w" };
  const seatAt = new Map();
  for (const p of m.props) {
    if (!SEAT[p.asset] || p.seat === false || p.if) continue;
    let [w, d] = FOOT[p.asset] ?? [1, 1];
    if (p.rot % 180 === 90) [w, d] = [d, w];
    for (let x = p.x; x < p.x + w; x++) for (let z = p.z; z < p.z + d; z++) seatAt.set(`${x},${z}`, { p, face: FACE[(((p.rot ?? 0) % 360) + 360) % 360] ?? "s" });
  }
  for (const n of m.npcs ?? []) {
    if (n.sit) {
      if (!seatAt.has(`${n.x},${n.z}`)) fail.push(`${name}: ${n.id} sits where there is no seat`);
      if (n.route) fail.push(`${name}: ${n.id} sits, so it can't walk a route`);
    } else if (solid.has(`${n.x},${n.z}`)) fail.push(`${name}: ${n.id} stands on a solid tile`);
  }
  const all = reach(name, m, solid, height);
  const sittable = new Set();
  for (const [k, { p, face }] of seatAt) {
    const [x, z] = k.split(",").map(Number);
    for (const [f, [dx, dz]] of Object.entries(DIR)) {
      const a = `${x - dx},${z - dz}`; // the hero there faces f
      if (f !== face && all.has(a) && Math.abs((height[a] ?? 0) - (height[k] ?? 0)) <= 0.6) sittable.add(p);
    }
  }
  for (const p of new Set([...seatAt.values()].map((s) => s.p)))
    if (!sittable.has(p)) rule.push(`${name}: the ${p.asset} at ${p.x},${p.z} has no free tile in front or beside it to sit from; free one, or mark it "seat": false`);
  // Underground nothing could power a street lamp: caves (`dark`, kind `cave`, a `cave` room) light with mushrooms,
  // crystals and glow plants, a worked mine with oil lanterns on its timbers (WORLD_RULES §2.6). Always an error.
  if (m.dark || m.kind === "cave" || m.cave)
    for (const p of m.props) if (["lantern", "lamp"].includes(p.asset)) fail.push(`${name}: a ${p.asset} at ${p.x},${p.z} in a cave; light it with glow_mushroom, glow_shelf or crystal_cluster`);
  open[name] = (x, z) => x >= 0 && z >= 0 && x < m.width && z < m.height && !solid.has(`${x},${z}`);
  rules(name, m, solid, all);
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
  scripts(name, m, solid, inMap);
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

// Cutscenes (WORLD_DESIGN §9.1): a unique id, an `if`, `on: "enter"` and/or in-bounds `at` tiles, and steps that are each
// a `say` (an actor on the map or the hero, with lines; lines_fr is checked with the other text) or a `do` list. The
// cues' arguments as the engine reads them (engine/tests/world_data.rs parses every effect): walk paths of n/e/s/w
// that keep an NPC on open tiles, face directions, cam on an actor or an in-bounds tile, waits of 0..5000 ms, music
// tracks; a `duel:` only in the last step (it ends the scene).
function scripts(name, m, solid, inMap) {
  const list = m.scripts ?? [];
  if (!Array.isArray(list)) return void fail.push(`${name}: scripts is a list`);
  const npcs = Object.fromEntries((m.npcs ?? []).map((n) => [n.id, n]));
  const actor = (id) => id === "hero" || id in npcs;
  const ids = list.map((s) => s.id);
  for (const s of list) {
    const at = `${name}: script ${s.id}`;
    if (!/^[a-z0-9_]+$/.test(s.id ?? "") || ids.filter((i) => i === s.id).length > 1) fail.push(`${at}: needs a unique id of a-z, 0-9 and _`);
    if (typeof s.if !== "string") fail.push(`${at}: needs an if (or it plays every time)`);
    if (!(s.on === "enter" || (s.on === undefined && s.at !== undefined))) fail.push(`${at}: starts on: "enter" (optionally only arriving on at tiles) or at: [[x, z], ...]`);
    if (s.at !== undefined && !(Array.isArray(s.at) && s.at.length && s.at.every((t) => Array.isArray(t) && inMap(t[0], t[1]) && !solid.has(`${t[0]},${t[1]}`))))
      fail.push(`${at}: at tiles must be open tiles on the map`);
    const steps = Array.isArray(s.steps) ? s.steps : [];
    if (!steps.length) fail.push(`${at}: has no steps`);
    const pos = {}; // where each NPC stands as the scene goes on
    steps.forEach((st, i) => {
      const where = `${at}.steps[${i}]`;
      if ((st.say !== undefined) === (st.do !== undefined)) return void fail.push(`${where}: is either a say or a do`);
      if (st.say !== undefined) {
        if (!actor(st.say)) fail.push(`${where}: no actor ${st.say} on this map`);
        if (!Array.isArray(st.lines) || !st.lines.length) fail.push(`${where}: a say needs lines`);
        return;
      }
      if (!Array.isArray(st.do) || !st.do.every((e) => typeof e === "string")) return void fail.push(`${where}: do is a list of effects`);
      for (const e of st.do) {
        const [kind, ...a] = e.split(":");
        const bad = (why) => fail.push(`${where}: ${e}: ${why}`);
        if (kind === "duel" && i !== steps.length - 1) bad("a duel ends the scene, so it goes in the last step");
        if (["walk", "face", "emote"].includes(kind) && !actor(a[0])) bad(`no actor ${a[0]} on this map`);
        if (kind === "face" && !DIR[a[1]]) bad("faces n, e, s or w");
        if (kind === "wait" && !(/^\d+$/.test(a[0]) && +a[0] <= 5000)) bad("waits 0 to 5000 ms");
        if (kind === "music" && !THEMES.has(a[0])) bad(`not a track (${[...THEMES].join(", ")})`);
        if (kind === "cam" && !(a.length === 1 ? actor(a[0]) : a.length === 2 && inMap(+a[0], +a[1]))) bad("looks at hero, an actor on this map or a tile on it");
        if (kind === "walk") {
          if (!/^[nesw]+$/.test(a[1] ?? "")) bad("walks a path of n, e, s and w");
          else if (a[0] !== "hero" && npcs[a[0]]) {
            let [x, z] = pos[a[0]] ?? [npcs[a[0]].x, npcs[a[0]].z];
            for (const f of a[1]) {
              [x, z] = [x + DIR[f][0], z + DIR[f][1]];
              if (!inMap(x, z) || solid.has(`${x},${z}`)) {
                bad(`walks ${a[0]} into ${x},${z}, which is not open`);
                break;
              }
            }
            pos[a[0]] = [x, z];
          }
        }
      }
    });
  }
}

// Gates and reachability: flood fill from the spawn over walkable tiles, a step at most 0.6 m up or
// down (a ladder joins its front and back whatever the height), first with every gate shut, then open.
// A lift's landings at height `y`: its open neighbours whose floor is within a step of the cage floor (y + 0.1).
function liftLandings(l, y, solid, height) {
  return Object.values(DIR).map(([dx, dz]) => `${l.x + dx},${l.z + dz}`).filter((k) => !solid.has(k) && Math.abs((height[k] ?? 0) - (y + 0.1)) <= 0.6);
}
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
    if (!g.name) fail.push(`${name}: gate ${g.id} needs a name (announced when it opens)`);
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
    // A lift: its shaft and the landings at both its heights, joined once any of them is reached.
    const cages = (m.lifts ?? []).map((l) => {
      const k = `${l.x},${l.z}`, low = l.bottom ?? h[k] ?? 0;
      return [k, ...liftLandings(l, low, solid, h), ...liftLandings(l, l.top, solid, h)];
    });
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
      const cage = cages.find((c) => !c.done && c.some((k) => seen.has(k)));
      if (cage) {
        cage.done = true;
        for (const k of cage) if (!seen.has(k)) seen.add(k), queue.push(k.split(",").map(Number));
        continue;
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

// Every `warp:<map>:<x>:<z>:<dir>` cue (a ferry, a lift) lands on open ground there.
for (const [name, m] of Object.entries(maps))
  for (const [, to, x, z] of JSON.stringify(m).matchAll(/"warp:([a-z_]+):(\d+):(\d+):[nesw]"/g))
    if (!open[to]?.(+x, +z)) rule.push(`${name}: a warp: cue lands on ${to} ${x},${z}, which is not open ground`);

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
if (STRICT) fail.push(...rule);
else warn.unshift(...rule);
if (warn.length) console.warn(`${warn.length} placement warnings (docs/WORLD_RULES.md; judge each one):\n  ${warn.join("\n  ")}`);
if (fail.length) throw new Error(fail.join("\n"));
console.log(`maps ok (${Object.keys(maps).join(", ")})`);

// The placement rules (docs/WORLD_RULES.md). `all`: the tiles the spawn reaches with every gate open.
// Heights (m) of what a prop shows above the ground, for depth stacking: the camera looks north and
// down at ~50°, so a prop hides what stands up to ~0.85 x its height behind it.
function rules(name, m, solid, all) {
  const W = m.width, H = m.height, out = !m.light.indoor, inMap = (x, z) => x >= 0 && z >= 0 && x < W && z < H;
  const tile = (x, z) => m.tiles[z]?.[x];
  const foot = (p) => {
    let [w, d] = FOOT[p.asset] ?? [1, 1];
    if (p.rot % 180 === 90) [w, d] = [d, w];
    const t = [];
    for (let x = p.x; x < p.x + w; x++) for (let z = p.z; z < p.z + d; z++) t.push([x, z]);
    return t;
  };
  const tall = (a) => HEIGHT[a] ?? TALL_OF(a);
  const ground = (p) => WALKABLE.has(p.asset) || DECK[p.asset]; // walked on: other props may stand on them
  const npcs = (m.npcs ?? []).filter((n) => !n.route);
  const at = new Map(); // tile -> props standing on it (not decks or rugs)
  for (const p of m.props) if (!ground(p)) for (const [x, z] of foot(p)) (at.get(`${x},${z}`) ?? at.set(`${x},${z}`, []).get(`${x},${z}`)).push(p);
  const who = (p) => `${p.asset} ${p.x},${p.z}`;
  const free = (x, z) => inMap(x, z) && !solid.has(`${x},${z}`) && !(m.npcs ?? []).some((n) => n.x === x && n.z === z);
  const pathy = (x, z) => "pr=H".includes(tile(x, z) ?? "-") || "pr".includes(m.paint?.[z]?.[x] ?? "-") || m.props.some((p) => DECK[p.asset] && foot(p).some(([a, b]) => a === x && b === z));

  // 1. Clearance: no prop stands in another's footprint (props on decks and rugs, and `if` props
  // that take turns with another, excepted).
  for (const [k, list] of at) {
    const live = list.filter((p) => !p.if);
    if (live.length > 1) rule.push(`${name}: ${live.map(who).join(" and ")} share tile ${k}`);
  }
  // Interactables: posts read from the front (signs, signposts, waymarks, inn and hanging signs), spots, people, doors.
  const POST = new Set(["sign", "signpost", "waymark", "hanging_sign", "inn_sign"]);
  const spots = (m.interactions ?? []).map((i) => ({ x: i.x, z: i.z, what: i.kind === "find" ? `find ${i.id}` : `${i.kind} ${i.id ?? ""}`.trim(), host: at.get(`${i.x},${i.z}`)?.[0] }));
  const things = [...spots, ...npcs.map((n) => ({ x: n.x, z: n.z, what: `npc ${n.id}`, npc: true })),
    ...(m.warps ?? []).filter((w) => !w.dir).map((w) => ({ x: w.x, z: w.z, what: `door to ${w.to}`, door: true, host: at.get(`${w.x},${w.z}`)?.[0] }))];
  // A door on a prop (a house's front row) needs that prop's STOOP (props.js): how far in the hero walks and the
  // heights of its steps, or the hero wades through them.
  for (const t of things) if (t.door && t.host && !t.host.if && !STOOP[t.host.asset]) fail.push(`${name}: the ${t.what} at ${t.x},${t.z} is on a ${t.host.asset}, which has no STOOP entry in props.js`);
  for (const p of m.props.filter((p) => POST.has(p.asset))) {
    const f = { 0: [0, 1], 90: [1, 0], 180: [0, -1], 270: [-1, 0] }[p.rot % 360] ?? [0, 1];
    if (!free(p.x + f[0], p.z + f[1])) rule.push(`${name}: ${who(p)} has no free tile in front of its face (${p.x + f[0]},${p.z + f[1]})`);
  }
  // Depth stacking: nothing tall stands just in front of (south of, toward the camera) something you use.
  for (const t of things) {
    const mine = new Set(t.host ? foot(t.host).map(String) : []);
    for (let d = 1; d <= 3; d++) {
      const k = `${t.x},${t.z + d}`;
      if (mine.has(`${t.x},${t.z + d}`)) continue;
      const front = [...(at.get(k) ?? []).filter((p) => !p.if && !ACROSS.has(p.asset)).map((p) => ({ h: tall(p.asset), what: who(p), use: POST.has(p.asset) })),
        ...npcs.filter((n) => n.x === t.x && n.z === t.z + d).map((n) => ({ h: 1.45, what: `npc ${n.id}`, use: true }))];
      for (const f of front) {
        const limit = [0, 0.95, 2.0, 3.0][d];
        if (f.h <= limit) continue;
        const msg = `${name}: ${f.what} stands ${d} tile${d > 1 ? "s" : ""} in front of ${t.what} at ${t.x},${t.z} and hides it from the camera`;
        if (d === 1 || f.use) rule.push(msg);
        else warn.push(msg);
      }
    }
  }
  // Breathing room between things you use: posts keep a tile clear from any other interactable.
  const posts = m.props.filter((p) => POST.has(p.asset));
  for (const p of posts)
    for (const t of things) {
      if (t.host === p || foot(p).some(([x, z]) => x === t.x && z === t.z)) continue;
      if (t.door && ["inn_sign", "hanging_sign"].includes(p.asset)) continue; // a shop's sign hangs by its door
      if (Math.max(Math.abs(t.x - p.x), Math.abs(t.z - p.z)) < 2) rule.push(`${name}: ${who(p)} is right next to ${t.what} at ${t.x},${t.z}; keep a tile between them`);
    }
  // Waymarks: a clear tile all round.
  for (const p of m.props.filter((p) => p.asset === "waymark"))
    for (let dx = -1; dx <= 1; dx++)
      for (let dz = -1; dz <= 1; dz++)
        if ((dx || dz) && !free(p.x + dx, p.z + dz)) rule.push(`${name}: waymark ${p.x},${p.z} needs a clear tile all round; ${p.x + dx},${p.z + dz} is taken`);

  // 2. Lamps follow use: a street lantern stands by a path or plaza; out of town (a route or a seal
  // site, or a `wild` rect) it says `why`. Two closer than 4 tiles are a clump (a warning).
  const lamps = m.props.filter((p) => p.asset === "lantern");
  const wild = (x, z) => ["route", "site", "cave"].includes(m.kind) || (m.wild ?? []).some(([x0, z0, x1, z1]) => x >= x0 && x <= x1 && z >= z0 && z <= z1);
  for (const p of lamps) {
    let near = false;
    for (let dx = -2; dx <= 2; dx++) for (let dz = -2; dz <= 2; dz++) near ||= pathy(p.x + dx, p.z + dz);
    if (!near) rule.push(`${name}: lantern ${p.x},${p.z} lights no path or plaza (none within 2 tiles)`);
    if (wild(p.x, p.z) && !p.why) rule.push(`${name}: lantern ${p.x},${p.z} is out of town; give it a "why" (a door, a bridge, a camp) or take it out`);
  }
  for (let i = 0; i < lamps.length; i++)
    for (let j = i + 1; j < lamps.length; j++)
      if (Math.max(Math.abs(lamps[i].x - lamps[j].x), Math.abs(lamps[i].z - lamps[j].z)) < 4 && !lamps[i].why && !lamps[j].why)
        warn.push(`${name}: lanterns ${lamps[i].x},${lamps[i].z} and ${lamps[j].x},${lamps[j].z} are under 4 tiles apart (a rhythm is 5-7)`);

  // People walk their routes on open ground.
  for (const n of m.npcs ?? []) {
    let [x, z] = [n.x, n.z];
    for (const s of n.route ?? []) {
      [x, z] = [x + (DIR[s]?.[0] ?? 0), z + (DIR[s]?.[1] ?? 0)];
      if (!free(x, z) && !(x === n.x && z === n.z)) rule.push(`${name}: ${n.id}'s route walks onto ${x},${z}, which is blocked`);
    }
  }
  // A door you come out of lands in front of the door that leads back in.
  for (const w of m.warps ?? []) {
    const o = maps[w.to];
    if (!o || o.light.indoor === m.light.indoor || !m.light.indoor) continue;
    const s = w.spawn ?? o.spawn;
    if (!(o.warps ?? []).some((b) => b.to === name && Math.abs(b.x - s.x) + Math.abs(b.z - s.z) === 1)) rule.push(`${name}: the way out lands on ${w.to} ${s.x},${s.z}, not in front of the door back in`);
  }
  if (!out) return;

  // 3. Size per kind, in screens (WORLD_RULES §8): a place should take several screens to cross.
  // A route is measured along its length: 2 screens across, 4 along (north-south or east-west).
  const [sx, sz] = [W / SCREEN[0], H / SCREEN[1]];
  const MINS = { village: [2, 2], town: [3.7, 3.5], hub: [3.7, 3.5], site: [3, 3], route: [2, 4], cave: [1, 3] };
  const min = MINS[m.kind];
  if (!min) rule.push(`${name}: an outdoor map needs a kind (${Object.keys(MINS).join(", ")})`);
  else {
    const ok = ["route", "cave"].includes(m.kind) ? (sx >= min[0] && sz >= min[1]) || (sx >= min[1] && sz >= min[0]) : sx >= min[0] && sz >= min[1];
    if (!ok) rule.push(`${name}: a ${m.kind} is at least ${min[0]} x ${min[1]} screens (${Math.ceil(min[0] * SCREEN[0])} x ${Math.ceil(min[1] * SCREEN[1])} tiles${["route", "cave"].includes(m.kind) ? `, or ${Math.ceil(min[1] * SCREEN[0])} x ${Math.ceil(min[0] * SCREEN[1])} east-west` : ""}); this one is ${sx.toFixed(1)} x ${sz.toFixed(1)} (${W} x ${H})`);
  }

  // 4. Exits read as exits: 3+ tiles wide, a path to the edge, the two rows behind it open, a signpost
  // nearby naming where it goes, and the border closed everywhere else.
  const edge = (side, c) => ({ n: [c, 0], s: [c, H - 1], w: [0, c], e: [W - 1, c] })[side];
  const inward = { n: [0, 1], s: [0, -1], w: [1, 0], e: [-1, 0] };
  const onExit = new Set();
  for (const e of m.exits ?? []) {
    const width = e.span[1] - e.span[0] + 1, back = maps[e.to]?.exits?.find((b) => b.side === BACK[e.side] && b.to === name);
    if (width < 3) rule.push(`${name}: exit ${e.side} to ${e.to} is ${width} tile${width > 1 ? "s" : ""} wide; make it 3`);
    if (back && back.span[1] - back.span[0] !== e.span[1] - e.span[0]) warn.push(`${name}: exit ${e.side} to ${e.to} is not as wide as the way back`);
    let path = 0;
    for (let c = e.span[0]; c <= e.span[1]; c++) {
      const [x, z] = edge(e.side, c);
      onExit.add(`${x},${z}`);
      if (pathy(x, z)) path++;
      for (let d = 1; d <= 2; d++) if (!free(x + inward[e.side][0] * d, z + inward[e.side][1] * d)) rule.push(`${name}: exit ${e.side} to ${e.to}: ${x + inward[e.side][0] * d},${z + inward[e.side][1] * d} blocks the way in`);
    }
    if (path < 2) rule.push(`${name}: exit ${e.side} to ${e.to} has no path running out to the edge`);
    const dest = maps[e.to], names = [dest?.title, dest?.title_fr].filter(Boolean);
    const sign = (m.interactions ?? []).find((i) => i.kind === "sign" && Array.from({ length: width }, (_, k) => edge(e.side, e.span[0] + k)).some(([x, z]) => Math.abs(i.x - x) + Math.abs(i.z - z) <= 6)
      && names.length && dest.title && String(i.text).includes(dest.title));
    if (!sign) rule.push(`${name}: exit ${e.side} to ${e.to} has no signpost within 6 tiles naming "${dest?.title}"`);
    else if (dest.title_fr && !String(sign.text_fr).includes(dest.title_fr)) warn.push(`${name}: the signpost at ${sign.x},${sign.z} names ${dest.title} but not "${dest.title_fr}" in French`);
  }
  for (let x = 0; x < W; x++)
    for (let z = 0; z < H; z++)
      if ((x === 0 || z === 0 || x === W - 1 || z === H - 1) && !onExit.has(`${x},${z}`) && all.has(`${x},${z}`))
        rule.push(`${name}: edge tile ${x},${z} is open but no exit; close the border (trees, a fence, rock, water)`);

  // 5. No pocket of open ground the hero can't reach (a save landing there would strand the hero).
  // Raised ground no stair reaches (a cliff face, a rock mass, a backdrop ridge) is scenery, not a pocket.
  const rock = (x, z) => tile(x, z) >= "1" && tile(x, z) <= "9";
  let lost = 0;
  for (let x = 0; x < W; x++) for (let z = 0; z < H; z++) if (!solid.has(`${x},${z}`) && !all.has(`${x},${z}`) && tile(x, z) !== "~" && !rock(x, z)) lost++;
  if (lost) warn.push(`${name}: ${lost} open tile${lost > 1 ? "s" : ""} the spawn can't reach (fill them, or let a save there fall back to the spawn)`);

  // 6. Houses face paths: the tile in front of a door is within 3 tiles of a path.
  for (const w of (m.warps ?? []).filter((w) => !w.dir)) {
    let near = false;
    for (let dx = -3; dx <= 3; dx++) for (let dz = 0; dz <= 4; dz++) near ||= pathy(w.x + dx, w.z + dz);
    if (!near) warn.push(`${name}: the door to ${w.to} at ${w.x},${w.z} has no path in front of it`);
  }
}
