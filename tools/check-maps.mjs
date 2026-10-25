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
import { readFileSync, readdirSync } from "node:fs";
const dir = process.argv[2] ?? "web/world";
const src = readFileSync(`${dir}/props.js`, "utf8");
const FOOT = eval(`(${src.match(/export const FOOT = (\{[\s\S]*?\});/)[1]})`);
const DECK = eval(`(${src.match(/export const DECK = (\{[\s\S]*?\});/)[1]})`);
const WALKABLE = new Set(["rug", "exit_mat", ...Object.keys(DECK)]);
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
if (fail.length) throw new Error(fail.join("\n"));
console.log(`maps ok (${Object.keys(maps).join(", ")})`);
