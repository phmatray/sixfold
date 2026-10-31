// Run: node tools/check-maps.mjs web/world
// Check every map: at most 80 x 72 tiles (80 x 64 in all); each edge exit has a matching exit back, the
// edge tiles of both spans are walkable, every warp and exit leads to a map that exists,
// and no NPC stands on a solid tile or a blocking prop; a door on a prop has its STOOP (props.js); NPC ids are unique on a map, and
// no two spots share a tile. Every player-facing string (lines, text, label, locked, title,
// a duelist's challenge / win / lose / after, and an interaction's speaker name) has a
// French `x_fr` twin of the same shape.
// Route duelists (`duelist`, WORLD_DESIGN §4.1): sight 0..5 (0: talk to duel, never watches), `look` directions valid, the
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
// Level design v2 (#110, WORLD_RULES §11): the six blocking rules (landmark, hidden-mandatory, dead-end,
// exit-both-ways, unreachable, connection) for maps with `"level_design": 2`, errors with --strict; on other maps
// they are only listed in the report. `--report` (or `--report=<file.md>`) prints an advisory section per outdoor
// map against the budgets in tools/level-design.json (`--budgets=<file>` for another), and never fails.
import { readFileSync, readdirSync, existsSync, writeFileSync } from "node:fs";
import * as ld from "../web/world/ld.js";
import { CRITTERS, boatLane, openTile } from "../web/world/life_schedule.js";
import { minuteOf, plan, DOES } from "../web/world/routine.js";
import { ANGLES, framing, lowestPitch } from "../web/world/camera.js";
// The player picks the outdoor camera angle (#148): occlusion grows as the pitch drops, so the depth rule runs at
// the lowest one, and the camera-ray test at every one.
const rad = (deg) => (deg * Math.PI) / 180;
const LOW = lowestPitch(), DEPTH_SCALE = Math.tan(rad(LOW)) / Math.tan(rad(50));
const visibleAll = (m, boxes, from, target) =>
  (m.light?.indoor ? ["default"] : ANGLES).every((a) => ld.visible({ ...m, camera: framing({ pitch: 50, distance: 12, ...m.camera }, a, false) }, boxes, from, target));
const dir = process.argv.slice(2).find((a) => !a.startsWith("--")) ?? "web/world";
const src = readFileSync(`${dir}/props.js`, "utf8");
const FOOT = eval(`(${src.match(/export const FOOT = (\{[\s\S]*?\});/)[1]})`);
const DECK = eval(`(${src.match(/export const DECK = (\{[\s\S]*?\});/)[1]})`);
const WALKABLE = new Set([...eval(`(${src.match(/export const WALKABLE = new Set\((\[[\s\S]*?\])\);/)[1]})`), ...Object.keys(DECK)]);
const ACROSS = new Set(eval(`(${src.match(/export const ACROSS = new Set\((\[[\s\S]*?\])\);/)[1]})`));
const TALL = eval(`(${src.match(/export const TALL = (\{[\s\S]*?\});/)[1]})`);
const SEAT = eval(`(${src.match(/export const SEAT = (\{[\s\S]*?\});/)[1]})`);
const STOOP = eval(`(${src.match(/export const STOOP = (\{[\s\S]*?\} \});/)[1]})`);
const HEIGHT = ld.HEIGHT; // heights (m) for depth stacking (web/world/ld.js)
const TALL_OF = (a) => TALL[a] ?? 1;
const LD = { FOOT, DECK, WALKABLE, tall: (a) => HEIGHT[a] ?? TALL_OF(a) }; // the tables web/world/ld.js reads
const TIERS = ["major", "secondary", "micro", "secret"]; // points of interest (`poi`), WORLD_RULES §9
const LADDER = new Set(eval(`(${src.match(/export const LADDER = new Set\((\[[\s\S]*?\])\);/)[1]})`));
const TRACKS = new Set(["overworld", "ember", "storm", "tide", "dream", "grove", "stone", "cave"]);
const BEDS = new Set([...TRACKS, "interior"]); // audio.js BEDS
// A scene's `music:` may also be a theme of its own (engine overworld::TRACKS): Rook's, the finale's, a keeper's or the Warden's duel theme.
const THEMES = new Set([...BEDS, "rook", "finale", "duel_boss", ...["ember", "storm", "tide", "dream", "grove", "stone"].map((r) => `duel_rival_${r}`)]);
const KEEPERS = ["bramble", "marlo", "cinza", "quill", "orla", "lumen"]; // the seal keepers (engine campaign::RIVALS with a seal)
const FENCES = new Set(["fence", "wall_dry", "hedge"]);
const atlas = existsSync(`${dir}/atlas.json`) ? JSON.parse(readFileSync(`${dir}/atlas.json`, "utf8")) : null;
const BACK = { n: "s", s: "n", e: "w", w: "e" };
// Does a model have window panes (build.js WINDOW: `glass`, `window_glow`) for a `lit` (#114)? Read from its glTF's materials.
const WINDOW = /window_glow|(^|_)glass$/i, ASSETS = new URL("../web/world/assets/", import.meta.url), panes = {};
const windowed = (asset) => (panes[asset] ??= (() => {
  const f = new URL(`${asset}.glb`, ASSETS);
  if (!existsSync(f)) return false;
  const b = readFileSync(f);
  return (JSON.parse(b.subarray(20, 20 + b.readUInt32LE(12))).materials ?? []).some((m) => WINDOW.test(m.name ?? ""));
})());
const maps = Object.fromEntries(readdirSync(`${dir}/maps`).map((f) => [f.replace(".json", ""), JSON.parse(readFileSync(`${dir}/maps/${f}`, "utf8"))]));
const fail = [], warn = [];
// Level-design budgets, in one place (tools/level-design.json; --budgets=<file> reads another).
const BUDGET = JSON.parse(readFileSync(process.argv.find((a) => a.startsWith("--budgets="))?.slice(10) || new URL("./level-design.json", import.meta.url), "utf8"));
const ctx = {}, designed = {}; // per map: what the level-design pass needs (solid, reachable, floor heights), and its findings
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
    // Barks (#114): a line said in passing, short, in both languages; `if` is checked by the engine (world_data.rs).
    if (n.glance !== undefined && typeof n.glance !== "boolean") fail.push(`${name}: ${n.id}'s glance is true or false`);
    (n.barks ?? []).forEach((b, i) => {
      const at = `${name}: ${n.id}'s bark ${i}`;
      if (typeof b.say !== "string" || !b.say.trim()) fail.push(`${at} needs a say`);
      if (!("say_fr" in b)) fail.push(`${at} has no say_fr`);
      if (b.if !== undefined && typeof b.if !== "string") fail.push(`${at}: its if is a condition string`);
      if (String(b.say).length > 60 || String(b.say_fr ?? "").length > 60) fail.push(`${at} is over 60 characters`);
    });
  }
  routines(name, m, solid, seatAt, height);
  const all = reach(name, m, solid, height);
  ctx[name] = { m, solid, all, height };
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
  lifeRules(name, m, solid);
  const ids = (m.npcs ?? []).map((n) => n.id);
  ids.filter((id, i) => ids.indexOf(id) !== i).forEach((id) => fail.push(`${name}: two NPCs are called ${id}`));
  const spots = (m.interactions ?? []).map((i) => `${i.x},${i.z}`);
  spots.filter((k, i) => spots.indexOf(k) !== i).forEach((k) => fail.push(`${name}: two spots on tile ${k}`));

  // Duelists' sight lines, cast the way duelist.js casts them (people block; routes are not followed).
  const inMap = (x, z) => x >= 0 && z >= 0 && x < m.width && z < m.height;
  const people = new Set((m.npcs ?? []).map((n) => `${n.x},${n.z}`));
  const ambush = new Set([`${m.spawn.x},${m.spawn.z}`, ...(landing[name] ?? [])]);
  for (const e of m.exits ?? [])
    for (let c = e.span[0]; c <= e.span[1]; c++) ambush.add({ n: `${c},0`, s: `${c},${m.height - 1}`, w: `0,${c}`, e: `${m.width - 1},${c}` }[e.side]);
  for (const n of m.npcs ?? []) {
    const t = n.duelist;
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
  scripts(name, m, solid, inMap, all);
  for (const t of [...(m.warps ?? []), ...(m.exits ?? [])]) if (!maps[t.to]) fail.push(`${name}: leads to missing map ${t.to}`);
  // Hours (#114): `open` keeps a door's hours (an edge or a spot has none); `lit` lights a model's own windows.
  for (const t of m.warps ?? []) if (t.open !== undefined && t.dir) fail.push(`${name}: the warp to ${t.to} at ${t.x},${t.z} has an open, but only a door keeps hours`);
  for (const p of m.props ?? []) if (p.lit !== undefined && !windowed(p.asset)) fail.push(`${name}: ${p.asset} at ${p.x},${p.z} is lit, but has no windows`);
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
// tracks, `stone:wake` only on a map with a seal stone, `stone:seal:<keeper>` (#153) only at that keeper's own seal
// stone; a `duel:` only in the last step (it ends the scene).
// A seal site's staging (#152; engine/tests/world_data.rs holds the rest of its shape): `reveal_<map>` keeps to
// 3 `cam` and 2 000 ms of `wait`; `entrance_<keeper>` starts on every reachable open tile two from the keeper (but
// one where someone stands for good: no route or routine), so nobody walks up to them without it; one that plays on arriving
// anywhere (the Warden's, #154) needs no ring.
function scripts(name, m, solid, inMap, reached) {
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
    const fx = steps.flatMap((st) => (Array.isArray(st.do) ? st.do : []));
    if (s.id?.startsWith("reveal_")) {
      const cams = fx.filter((e) => e.startsWith("cam:")).length, waits = fx.reduce((t, e) => t + (e.startsWith("wait:") ? +e.slice(5) || 0 : 0), 0);
      if (cams > 3 || waits > 2000) fail.push(`${at}: a reveal keeps to 3 cams and 2000 ms of waits (has ${cams} and ${waits} ms)`);
    }
    const keeper = s.id?.startsWith("entrance_") && !(s.on === "enter" && !s.at) && npcs[s.id.slice(9)];
    if (keeper) {
      const others = new Set((m.npcs ?? []).filter((n) => n !== keeper && !n.route && !n.routine).map((n) => `${n.x},${n.z}`)), tiles = new Set((s.at ?? []).map((t) => `${t[0]},${t[1]}`));
      for (let dx = -2; dx <= 2; dx++)
        for (let dz = -2; dz <= 2; dz++) {
          const k = `${keeper.x + dx},${keeper.z + dz}`;
          if (Math.max(Math.abs(dx), Math.abs(dz)) === 2 && reached.has(k) && !solid.has(k) && !others.has(k) && !tiles.has(k)) fail.push(`${at}: ${k} is two from ${keeper.id} and reachable, so it starts the entrance too`);
        }
    }
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
        if (kind === "face" && !DIR[a[1]] && !actor(a[1])) bad("faces n, e, s, w, the hero or an actor on this map");
        if (kind === "wait" && !(/^\d+$/.test(a[0]) && +a[0] <= 5000)) bad("waits 0 to 5000 ms");
        if (kind === "music" && !THEMES.has(a[0])) bad(`not a track (${[...THEMES].join(", ")})`);
        const seal = a[0] === "seal" && a.length === 2;
        if (kind === "stone" && !((a.length === 1 && a[0] === "wake") || seal)) bad("is stone:wake or stone:seal:<keeper>");
        else if (kind === "stone" && seal && !(KEEPERS.includes(a[1]) && npcs[a[1]])) bad(`lights the seal of a keeper who stands here (${KEEPERS.join(", ")})`);
        else if (kind === "stone" && !(m.props ?? []).some((p) => p.asset === "seal_stone")) bad("this map has no seal_stone to wake");
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
// People's routines (web/world/routine.js, WORLD_RULES §9): valid times and activities, every place open and
// walkable from where the person starts (a `sit` may name a seat: it walks up to it), every `home` a door on this
// map. A duelist or a rival keeps still: its sight line and its story need it where the map puts it.
function routines(name, m, solid, seatAt, height) {
  if (m.walkers_max !== undefined && !(Number.isInteger(m.walkers_max) && m.walkers_max > 0)) fail.push(`${name}: walkers_max is a whole number above 0`);
  const doors = (m.warps ?? []).filter((w) => !w.dir);
  const grid = {
    width: m.width, height: m.height,
    climb: (ax, az, bx, bz) => Math.abs((height[`${bx},${bz}`] ?? 0) - (height[`${ax},${az}`] ?? 0)) <= 0.6, // as build.js (ladders aside)
    open: (x, z) => x >= 0 && z >= 0 && x < m.width && z < m.height && !solid.has(`${x},${z}`),
    door: (to) => ((w) => w && [w.x, w.z])(doors.find((w) => w.to === to)),
  };
  for (const n of m.npcs ?? []) {
    if (n.routine === undefined) continue;
    const who = `${name}: ${n.id}'s routine`;
    if (n.duelist || JSON.stringify(n.dialogue ?? "").includes("duel:")) fail.push(`${name}: ${n.id} duels, so it keeps no routine (it stands where the map puts it)`);
    if (n.route) fail.push(`${name}: ${n.id} has a route and a routine; give it one or the other`);
    if (!Array.isArray(n.routine) || !n.routine.length) {
      fail.push(`${who} is a list of activities`);
      continue;
    }
    for (const a of n.routine) {
      const at = `${who} at ${a.from}`;
      if (Number.isNaN(minuteOf(a.from))) fail.push(`${who}: "${a.from}" is not a time (HH:MM, or dawn, day, dusk, night)`);
      if (a.do !== undefined && !DOES.includes(a.do)) fail.push(`${at}: "${a.do}" is not an activity (${DOES.join(", ")})`);
      if (a.face !== undefined && !DIR[a.face]) fail.push(`${at} faces a way that is not n/e/s/w`);
      if (a.with !== undefined && !(m.npcs ?? []).some((o) => o.id === a.with && o !== n)) fail.push(`${at} is with ${a.with}, who is not on this map`);
      const [x, z] = a.at ?? [];
      if (a.home !== undefined) {
        if (!grid.door(a.home)) fail.push(`${at} goes home to ${a.home}, but no door here leads there`);
      } else if (!(Number.isInteger(x) && Number.isInteger(z))) fail.push(`${at} needs \`at: [x, z]\` or \`home\``);
      else if (!grid.open(x, z) && !(a.do === "sit" && seatAt.has(`${x},${z}`))) fail.push(`${at} stands on ${x},${z}, which is blocked or off the map`);
      if (!(a.home !== undefined ? grid.door(a.home) : grid.open(x, z) || seatAt.has(`${x},${z}`))) continue;
      const p = plan({ x: n.x, z: n.z, def: { routine: [a] } }, 0, grid);
      if (!p.to || !p.steps) fail.push(`${at} can't be walked to from ${n.x},${n.z}`);
    }
  }
}

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
levelDesign();
transitions();
if (STRICT) fail.push(...rule);
else warn.unshift(...rule);
if (warn.length) console.warn(`${warn.length} placement warnings (docs/WORLD_RULES.md; judge each one):\n  ${warn.join("\n  ")}`);
if (fail.length) throw new Error(fail.join("\n"));
console.log(`maps ok (${Object.keys(maps).join(", ")})`);

// The placement rules (docs/WORLD_RULES.md). `all`: the tiles the spawn reaches with every gate open.
// Heights (m) of what a prop shows above the ground, for depth stacking: the camera looks north and
// down at 40–60° outdoors (50° in rooms), so a prop hides what stands up to ~1.2 x its height behind it at 40°
// (~0.85 x at 50°).
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
  // People who stand still: where the map puts them, or (with a routine) at each of its places.
  const npcs = (m.npcs ?? []).flatMap((n) => (n.route ? [] : n.routine ? n.routine.filter((a) => a.at).map((a) => ({ ...n, x: a.at[0], z: a.at[1] })) : [n]));
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
        ...npcs.filter((n) => n.x === t.x && n.z === t.z + d && `npc ${n.id}` !== t.what).map((n) => ({ h: 1.45, what: `npc ${n.id}`, use: true }))]; // one person at two hours never hides itself
      for (const f of front) {
        const limit = [0, 0.95, 2.0, 3.0][d] * (out ? DEPTH_SCALE : 1);
        if (f.h <= limit) continue;
        const msg = `${name}: ${f.what} stands ${d} tile${d > 1 ? "s" : ""} in front of ${t.what} at ${t.x},${t.z} and hides it from the camera${out ? ` at ${LOW}°` : ""}`;
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
    const dest = maps[e.to], sign = exitSign(m, e);
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

// Biome transitions (#118, WORLD_RULES 5.7): every edge exit into another region has a `transitions` band
// (ld.bandOf), 4-6 tiles deep, holding at least 3 props of its `scatter` set, and the exit back has its own.
// tools/transition.mjs writes them. A border with no band is an error with --strict.
function transitions() {
  for (const [name, m] of Object.entries(maps)) {
    if (m.light.indoor) continue;
    for (const e of m.exits ?? []) {
      const o = maps[e.to];
      if (o && o.region !== m.region && !(m.transitions ?? []).some((t) => t.to === e.to))
        rule.push(`${name}: [transition] the ${e.side} exit to ${e.to} crosses into ${o.region} with no transition band (node tools/transition.mjs)`);
    }
    for (const t of m.transitions ?? []) {
      const b = ld.bandOf(m, t), at = `${name}: [transition] the band to ${t.to}`;
      if (!b) {
        fail.push(`${at} matches no edge exit`);
        continue;
      }
      if (!(Number.isInteger(t.depth) && t.depth >= 4 && t.depth <= 6)) fail.push(`${at} is ${t.depth} tiles deep; make it 4-6`);
      else {
        const n = m.props.filter((p) => t.scatter?.includes(p.asset) && ld.bandWeight(m, b, p.x + 0.5, p.z + 0.5) > 0).length;
        if (n < 3) fail.push(`${at} holds ${n} props of its scatter (${t.scatter}); it needs at least 3`);
      }
      if (!(maps[t.to]?.transitions ?? []).some((u) => u.to === name)) fail.push(`${at} has no band back on ${t.to}`);
    }
  }
}

// Level design v2 (#110, docs/WORLD_RULES.md §11, docs/LEVEL_DESIGN.md). The geometry is web/world/ld.js, shared
// with the browser overlays. The tags are checked on every map; the six blocking rules are errors (with --strict)
// on maps with `"level_design": 2` and report lines on the rest.
function levelDesign() {
  const tile = (t) => Array.isArray(t) && t.length === 2 && t.every(Number.isInteger);
  for (const [name, m] of Object.entries(maps)) {
    const inMap = ([x, z]) => x >= 0 && z >= 0 && x < m.width && z < m.height;
    if (m.level_design !== undefined && m.level_design !== 2) fail.push(`${name}: level_design is 2 (or left out)`);
    if (m.level_design === 2 && m.light.indoor) fail.push(`${name}: level_design 2 is for outdoor maps`);
    for (const v of views(m)) if (!tile(v) || !inMap(v)) fail.push(`${name}: view ${JSON.stringify(v)} is not a tile on the map`);
    if (m.vista !== undefined && !(Array.isArray(m.vista) && m.vista.every((r) => Array.isArray(r) && r.length === 4 && r.every(Number.isInteger))))
      fail.push(`${name}: vista is [[x0, z0, x1, z1], …]`);
    const tagged = [...m.props.map((p) => ["prop", p]), ...(m.interactions ?? []).map((i) => ["spot", i]), ...(m.npcs ?? []).map((n) => ["npc", n]), ...(m.warps ?? []).map((w) => ["warp", w])];
    for (const [what, o] of tagged) {
      const at = `${name}: the ${what} at ${o.x},${o.z}`;
      if (o.poi !== undefined && !TIERS.includes(o.poi)) fail.push(`${at}: poi is ${TIERS.join(" / ")}`);
      if (o.landmark !== undefined && (what !== "prop" || o.landmark !== true)) fail.push(`${at}: landmark is true, on a prop`);
      if (o.optional !== undefined && (what === "prop" || o.optional !== true)) fail.push(`${at}: optional is true, on a spot, a person or a warp`);
      if (o.clue !== undefined && (o.poi !== "secret" || !Array.isArray(o.clue) || !o.clue.every((c) => tile(c) && inMap(c)))) fail.push(`${at}: clue is [[x, z], …] on a secret`);
    }
  }
  for (const [name, m] of Object.entries(maps)) {
    if (m.light.indoor) continue;
    const found = design(name).blocking;
    if (m.level_design === 2) rule.push(...found.map((f) => `${name}: ${f}`));
  }
  const flag = process.argv.find((a) => a === "--report" || a.startsWith("--report="));
  if (!flag) return;
  const text = ["# Level design report", "", ...Object.keys(maps).filter((n) => !maps[n].light.indoor).flatMap(report)].join("\n");
  if (flag.includes("=")) writeFileSync(flag.slice(9), text + "\n");
  else console.log(text);
}
// One outdoor map's section of the advisory report (never a failure): what design() found, against the budgets.
function report(name) {
  const { m, all, height } = ctx[name], d = design(name), k = (x, z) => `${x},${z}`, advice = [];
  // Walks follow the 0.6 m step and ladders, as reach() does. ponytail: lifts and the per-row deck heights are left
  // out, so a walk through a lift reads as unreachable (skipped), never as shorter.
  const ladders = new Set(m.props.filter((p) => LADDER.has(p.asset)).map((p) => k(p.x, p.z)));
  const step = (ax, az, bx, bz) => Math.abs((height[k(bx, bz)] ?? 0) - (height[k(ax, az)] ?? 0)) <= 0.6 || ladders.has(k(ax, az)) || ladders.has(k(bx, bz));
  const walk = (starts) => ld.bfs(all, starts, step);
  const xz = (t) => t.split(",").map(Number);
  const range = ([lo, hi]) => (lo === hi ? `${lo}` : `${lo}–${hi}`);
  // POIs per tier, against the budget for the map's kind.
  const count = Object.fromEntries(TIERS.map((t) => [t, d.pois.filter((p) => p.tier === t).length]));
  const budget = BUDGET[m.kind] ?? {};
  for (const [t, [lo, hi]] of Object.entries(budget)) if (count[t] < lo || count[t] > hi) advice.push(`[poi-budget] ${count[t]} ${t} POIs (${range([lo, hi])} for a ${m.kind})`);
  // Discovery spacing: from each entrance and each discovery (a POI or a find), the walk to the nearest other one.
  const finds2 = ld.discoveries(m, d.pois, all, FOOT);
  const edgeMid = (e) => ld.edgeTile(m, e.side, Math.round((e.span[0] + e.span[1]) / 2));
  const entrances = [{ what: "the spawn", tiles: [k(m.spawn.x, m.spawn.z)] }, ...(landing[name] ?? []).map((t) => ({ what: `the landing ${t}`, tiles: [t] })),
    ...(m.exits ?? []).map((e) => ({ what: `the exit ${e.side} to ${e.to}`, tiles: [k(...edgeMid(e))] }))].filter((e) => e.tiles.every((t) => all.has(t)));
  const gap = ld.gap([...entrances, ...finds2], finds2, walk);
  const gapLine = gap ? `${ld.walkTime(gap.tiles).toFixed(1)} s (${gap.tiles} tiles), from ${gap.from.what}` : "no discoveries to measure";
  if (gap && ld.walkTime(gap.tiles) > BUDGET.discovery_gap_s[1]) advice.push(`[discovery-gap] ${ld.walkTime(gap.tiles).toFixed(1)} s without a discovery from ${gap.from.what} (${range(BUDGET.discovery_gap_s)} s)`);
  // Crossroads: one line per cluster of junction tiles, at its widest.
  const wide = d.graph.junctions.filter((j) => j.branches > BUDGET.crossroads_max), left = new Set(wide.map((j) => k(j.x, j.z))), crossroads = [];
  for (const j of wide) {
    if (!left.has(k(j.x, j.z))) continue;
    const cluster = [...ld.bfs(left, [[j.x, j.z]]).dist.keys()];
    cluster.forEach((t) => left.delete(t));
    const top = wide.filter((o) => cluster.includes(k(o.x, o.z))).sort((a, b) => b.branches - a.branches)[0];
    crossroads.push(`${top.x},${top.z} (${top.branches} ways)`);
    advice.push(`[crossroads] ${top.branches} ways out at ${top.x},${top.z} (${BUDGET.crossroads_max} at most)`);
  }
  // Scatter and variety, then density along the main path (between the two entrances farthest apart).
  const props = d.g.boxes.map((b) => ({ x: b.x0, z: b.z0, asset: b.prop.asset, rot: b.prop.rot }));
  const sc = ld.scatter(props), dom = sc.chunks.filter((c) => c.n >= 8 && c.share > BUDGET.dominant_share_max);
  if (sc.ratio > BUDGET.scatter_ratio_max) advice.push(`[scatter] props spread evenly (ratio ${sc.ratio.toFixed(2)}, ${BUDGET.scatter_ratio_max} at most): cluster them by function`);
  for (const c of dom) advice.push(`[variety] chunk ${c.chunk} is ${Math.round(c.share * 100)}% ${c.asset} (${c.n} props; ${Math.round(BUDGET.dominant_share_max * 100)}% at most)`);
  const ends = entrances.map((e) => e.tiles[0]);
  const far = (from) => {
    const r = walk([xz(from)]), pool = ends.length > 1 ? ends : [...r.dist.keys()];
    return { r, to: pool.reduce((a, b) => ((r.dist.get(b) ?? -1) > (r.dist.get(a) ?? -1) ? b : a)) };
  };
  const a = far(k(m.spawn.x, m.spawn.z)).to, { r, to: b } = far(a), route = [];
  for (let t = b; t; t = r.parent.get(t)) route.unshift(xz(t));
  const dn = ld.density(props, route, SCREEN);
  if (dn.curve.length > 1 && dn.cv < BUDGET.density_cv_min) advice.push(`[density] flat along the main path ${a} → ${b} (cv ${dn.cv.toFixed(2)}, ${BUDGET.density_cv_min} at least): add a breather or a dense patch`);
  for (const e of m.exits ?? []) if (e.span[1] - e.span[0] + 1 < BUDGET.exit_min) advice.push(`[exit] exit ${e.side} to ${e.to} is ${e.span[1] - e.span[0] + 1} wide (${BUDGET.exit_min} at least)`);
  const v2 = m.level_design === 2;
  return [
    `## ${name} — ${m.title} (${m.kind})`, "",
    `- level design: ${v2 ? "2 (the §11 rules are errors)" : "not opted in (the §11 rules are listed below, not enforced)"}`,
    `- landmark: ${d.marks.map((p) => `${p.asset} ${p.x},${p.z}`).join(", ") || "none"}`,
    `- POIs: ${Object.entries(count).map(([t, n]) => `${n} ${t}`).join(", ")}${Object.keys(budget).length ? ` (budget: ${Object.entries(budget).map(([t, b]) => `${t} ${range(b)}`).join(", ")})` : ""}`,
    `- longest walk without a discovery: ${gapLine}`,
    `- crossroads over ${BUDGET.crossroads_max} ways: ${crossroads.join(", ") || "none"}`,
    `- scatter: ratio ${sc.ratio?.toFixed(2) ?? "–"} (${BUDGET.scatter_ratio_max} at most); dominant: ${sc.chunks.filter((c) => c.n >= 8).sort((a, b) => b.share - a.share).slice(0, 3).map((c) => `${c.asset} ${Math.round(c.share * 100)}% of chunk ${c.chunk}`).join(", ") || "–"}`,
    `- density along the main path, a screen at a time: ${dn.curve.join(" · ") || "–"} (cv ${dn.cv.toFixed(2)}, ${BUDGET.density_cv_min} at least)${dn.breathers.length ? `; breathers at screens ${dn.breathers.map((i) => i + 1).join(", ")}` : ""}`,
    `- advisories:${advice.length ? "" : " none"}`, ...advice.map((s) => `  - ${s}`),
    ...(v2 ? [] : [`- §11 findings:${d.blocking.length ? "" : " none"}`, ...d.blocking.map((s) => `  - ${s}`)]),
    "",
  ];
}
// An exit's signpost (web/world/ld.js, with edgeTile and edgeTiles).
function exitSign(m, e) {
  return ld.exitSign(m, e, maps[e.to]);
}
// `view` (a map's viewpoints): one [x, z] or a list of them.
function views(m) {
  return m.view === undefined ? [] : Array.isArray(m.view?.[0]) ? m.view : [m.view];
}
// One outdoor map's level-design analysis: the blocking findings, and what the report shows.
function design(name) {
  if (designed[name]) return designed[name];
  const { m, all, height } = ctx[name], W = m.width, H = m.height;
  const g = ld.grids(m, LD), boxes = g.boxes, blocking = [];
  const k = (x, z) => `${x},${z}`, floor = (x, z) => height[k(x, z)] ?? 0;
  const seen = (from, target) => visibleAll(m, boxes, { x: from[0], z: from[1], y: floor(...from) }, target);
  const signs = new Set((m.exits ?? []).map((e) => exitSign(m, e)).filter(Boolean));
  const pois = ld.pois(m, signs); // points of interest by tier (web/world/ld.js)

  // Landmark: one or more props tagged, each seen from a main-path tile within a screen of it.
  const marks = m.props.filter((p) => p.landmark);
  if (marks.length < BUDGET.landmarks_min) blocking.push(`[landmark] ${marks.length ? `only ${marks.length} props carry` : "no prop carries"} "landmark": true (${BUDGET.landmarks_min} wanted)`);
  const walkway = [...g.path].filter((t) => all.has(t)).map((t) => t.split(",").map(Number));
  const lanes = walkway.length ? walkway : [...all].map((t) => t.split(",").map(Number));
  for (const p of marks) {
    const box = boxes.find((b) => b.prop === p), [w, d] = ld.foot(p, FOOT), tops = [];
    for (let x = p.x; x < p.x + w; x++) for (let z = p.z; z < p.z + d; z++) tops.push({ x, z, y: box?.y1 ?? 1, self: p });
    const ok = tops.some((t) => lanes.some(([x, z]) => Math.abs(x - t.x) <= SCREEN[0] >> 1 && Math.abs(z - t.z) <= SCREEN[1] >> 1 && seen([x, z], t)));
    if (!ok) blocking.push(`[landmark] the ${p.asset} at ${p.x},${p.z} can't be seen from any path tile within a screen of it`);
  }

  // Hidden mandatory: every warp, waymark, exit signpost and major POI is seen from the tile you use it from.
  for (const p of pois) {
    if (p.tier !== "major" || p.o?.optional) continue;
    const a = ld.approach(p, g, all, floor, FOOT);
    if (!a) continue; // unreachable: the reach rules already say so
    if (!visibleAll(m, boxes, a.from, a.target)) blocking.push(`[hidden-mandatory] the ${p.what} is hidden from the camera when you stand at ${a.from.x},${a.from.z}`);
  }

  // Dead ends: a path branch over 3 tiles long ends within 2 tiles of something (or a `view`), or of a stair or
  // ladder that takes it on at another height.
  const tilesOf = (p) => {
    const [w, d] = ld.foot(p, FOOT), t = [];
    for (let x = p.x; x < p.x + w; x++) for (let z = p.z; z < p.z + d; z++) t.push({ x, z });
    return t;
  };
  const ends = [
    ...(m.interactions ?? []), ...(m.npcs ?? []), ...(m.warps ?? []),
    ...m.props.filter((p) => p.poi || p.landmark || Math.max(...(DECK[p.asset] ?? [0])) >= 0.5).flatMap(tilesOf),
    ...views(m).map(([x, z]) => ({ x, z })),
    ...(m.exits ?? []).flatMap((e) => ld.edgeTiles(m, e).map(([x, z]) => ({ x, z }))),
  ];
  const graph = ld.pathGraph(g);
  for (const e of graph.deadEnds) {
    if (e.length <= 3 || ends.some((o) => Math.max(Math.abs(o.x - e.x), Math.abs(o.z - e.z)) <= 2)) continue;
    blocking.push(`[dead-end] the path ending at ${e.x},${e.z} runs ${Number.isFinite(e.length) ? `${e.length} tiles from a junction` : "with no junction"} to nothing; end it on something to find, a person or a "view"`);
  }

  // Exits both ways: from 4 tiles inside, 2+ of the exit's edge tiles are on screen and seen (here and on the map
  // it leads to).
  for (const e of m.exits ?? []) {
    for (const [on, ex] of [[name, e], [e.to, maps[e.to]?.exits?.find((b) => b.side === BACK[e.side] && b.to === name)]]) {
      if (!ex || !ctx[on] || (on !== name && maps[on].level_design === 2)) continue; // a v2 neighbour reports its own
      const n = exitSeen(on, ex);
      if (n < 2) blocking.push(`[exit-both-ways] ${on === name ? "" : `on ${on}, `}exit ${ex.side} to ${ex.to}: ${n} of its edge tiles can be seen from 4 tiles inside (2 needed)`);
    }
  }

  // Unreachable: no pocket of 4+ open tiles the hero can't reach with every gate open, unless a `vista` covers it.
  const rock = (x, z) => m.tiles[z][x] >= "1" && m.tiles[z][x] <= "9";
  const lost = new Set();
  for (let x = 0; x < W; x++) for (let z = 0; z < H; z++) if (!ctx[name].solid.has(k(x, z)) && !all.has(k(x, z)) && m.tiles[z][x] !== "~" && !rock(x, z)) lost.add(k(x, z));
  const vista = (x, z) => (m.vista ?? []).some(([x0, z0, x1, z1]) => x >= x0 && x <= x1 && z >= z0 && z <= z1);
  for (const t of lost) {
    const pocket = [...ld.bfs(lost, [t.split(",").map(Number)]).dist.keys()];
    pocket.forEach((p) => lost.delete(p));
    if (pocket.length >= 4 && !pocket.every((p) => vista(...p.split(",").map(Number))))
      blocking.push(`[unreachable] ${pocket.length} open tiles round ${t} can't be reached with every gate open; fill them, or list them in "vista"`);
  }

  // Connections: every warp from or to this map lands beside its way back and never one step from taking it again:
  // not on a door back or facing one (a door is entered by stepping into it), not on a mat (a `dir` warp) facing
  // its way out, not facing off the map (world.js tryStep).
  for (const [from, f] of Object.entries(maps))
    for (const w of f.warps ?? []) {
      if (from !== name && (w.to !== name || f.level_design === 2)) continue; // a v2 map reports its own warps
      const to = maps[w.to], s = w.spawn ?? to?.spawn;
      if (!to || !s || w.to === from) continue;
      const backs = (to.warps ?? []).filter((b) => b.to === from), on = backs.find((b) => b.x === s.x && b.z === s.z);
      const ahead = DIR[s.facing] ? [s.x + DIR[s.facing][0], s.z + DIR[s.facing][1]] : null;
      const at = `[connection] the warp at ${from} ${w.x},${w.z} lands on ${w.to} ${s.x},${s.z}`;
      if (on && !on.dir) blocking.push(`${at}, on the door back`);
      else if (on && on.dir === s.facing) blocking.push(`${at} facing ${s.facing}, the way back (one push and it loops)`);
      else if (ahead && backs.some((b) => !b.dir && b.x === ahead[0] && b.z === ahead[1])) blocking.push(`${at} facing the door back (one step and it loops)`);
      else if (ahead && !(ahead[0] >= 0 && ahead[1] >= 0 && ahead[0] < to.width && ahead[1] < to.height)) blocking.push(`${at} facing out of the map`);
      if (backs.length && !backs.some((b) => Math.abs(b.x - s.x) + Math.abs(b.z - s.z) <= 1)) blocking.push(`${at}, away from the way back (land on or beside it)`);
    }

  return (designed[name] = { blocking, g, graph, pois, marks, lanes });
}
// How many of an exit's edge tiles are on screen and seen (a person's height above them, 1.45 m) from the tile 4
// inside its middle.
function exitSeen(name, e) {
  const { m, height, all } = ctx[name];
  const boxes = (designed[name]?.g ?? ld.grids(m, LD)).boxes;
  const inward = { n: [0, 1], s: [0, -1], w: [1, 0], e: [-1, 0] }[e.side];
  // The tile 4 in from the exit's middle, or the nearest reachable one on the way out to it.
  const mid = ld.edgeTile(m, e.side, Math.round((e.span[0] + e.span[1]) / 2));
  let hero = null;
  for (let d = 4; d >= 0 && !hero; d--) if (all.has(`${mid[0] + inward[0] * d},${mid[1] + inward[1] * d}`)) hero = [mid[0] + inward[0] * d, mid[1] + inward[1] * d];
  if (!hero) return 0;
  let n = 0;
  for (const [x, z] of ld.edgeTiles(m, e)) {
    if (Math.abs(x - hero[0]) > SCREEN[0] >> 1 || Math.abs(z - hero[1]) > SCREEN[1] >> 1) continue;
    if (visibleAll(m, boxes, { x: hero[0], z: hero[1], y: height[`${hero[0]},${hero[1]}`] ?? 0 }, { x, z, y: (height[`${x},${z}`] ?? 0) + 1.45 })) n++;
  }
  return n;
}

// Ambient life (#115, WORLD_RULES §9): a map's `life` and `calm`. Always errors.
function lifeRules(name, m, solid) {
  const inside = (r) => Array.isArray(r) && r.length === 4 && r.every(Number.isInteger) && r[0] >= 0 && r[1] >= 0 && r[2] < m.width && r[3] < m.height && r[0] <= r[2] && r[1] <= r[3];
  const rect = (r) => JSON.stringify(r);
  for (const r of m.calm ?? []) if (!inside(r)) fail.push(`${name}: [calm] ${rect(r)} is not [x0, z0, x1, z1] inside the map`);
  if (m.kind === "site")
    for (const p of m.props.filter((p) => p.asset === "seal_stone"))
      if (!(m.calm ?? []).some((r) => inside(r) && p.x >= r[0] && p.x <= r[2] && p.z >= r[1] && p.z <= r[3])) fail.push(`${name}: [calm] the seal stone at ${p.x},${p.z} needs a calm rect round it`);
  const L = m.life;
  if (!L) return;
  const bad = (what) => fail.push(`${name}: [life] ${what}`);
  if (m.light.indoor) bad("is for outdoor maps");
  const open = (x, z) => !solid.has(`${x},${z}`) && openTile(m, x, z); // as build.js places them
  for (const c of L.critters ?? []) {
    const n = c.n ?? 1;
    if (!CRITTERS[c.kind]) bad(`critter kind ${c.kind} is not one of ${Object.keys(CRITTERS).join(", ")}`);
    if (!(Number.isInteger(n) && n >= 1 && n <= 6)) bad(`${c.kind}: n is 1 to 6`);
    if (!inside(c.zone)) {
      bad(`the ${c.kind} zone ${rect(c.zone)} is not [x0, z0, x1, z1] inside the map`);
      continue;
    }
    let free = 0;
    for (let x = c.zone[0]; x <= c.zone[2]; x++) for (let z = c.zone[1]; z <= c.zone[3]; z++) free += open(x, z) ? 1 : 0;
    if (free < n) bad(`the ${c.kind} zone ${rect(c.zone)} has ${free} open tiles off the paths, fewer than its ${n} critters`);
  }
  const models = [...(L.critters ?? []).filter((c) => CRITTERS[c.kind]).map((c) => `critter_${c.kind}`), ...((L.perch ?? []).length ? ["critter_bird"] : []), ...((L.boats ?? []).length ? ["fishing_boat"] : [])];
  for (const f of new Set(models)) if (!existsSync(new URL(`${f}.glb`, ASSETS))) bad(`${f}.glb is missing (tools/blender/build_critters.py builds it)`);
  const on = (x, z, ok) =>
    m.props.some((p) => {
      let [w, d] = FOOT[p.asset] ?? [1, 1];
      if ((p.rot ?? 0) % 180 === 90) [w, d] = [d, w];
      return ok(p) && x >= p.x && x < p.x + w && z >= p.z && z < p.z + d;
    });
  for (const g of L.perch ?? [])
    for (const [x, z, y] of g.at ?? []) if (typeof y !== "number" || !on(x, z, (p) => !WALKABLE.has(p.asset) && !p.if)) bad(`perch ${x},${z} is on no prop (or has no height)`);
  const l = L.leaves;
  if (l && !(Array.isArray(l.under) && l.under.length && l.under.every((a) => m.props.some((p) => p.asset === a)) && l.rate > 0 && l.rate <= 1))
    bad("leaves: under names trees on this map, and rate is above 0 and at most 1");
  if (L.embers && L.embers.from !== "smoke") bad('embers: from is "smoke"');
  for (const b of L.boats ?? []) {
    const [x, z] = b.pier ?? [];
    if (!("H=".includes(m.tiles[z]?.[x] ?? "-") || on(x, z, (p) => DECK[p.asset]))) bad(`boat pier ${x},${z} is not on a deck or a pier`);
    else if (!boatLane(m, b.pier, (x, z) => on(x, z, (p) => !p.if))) bad(`boat pier ${x},${z} has no 4 tiles of open water beside it`);
    if (b.every !== undefined && !(b.every >= 30)) bad("boats: every is 30 s at least");
  }
}
