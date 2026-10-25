// Run: node tools/check-maps.mjs web/world
// Check every map: each edge exit has a matching exit back, the edge tiles of both
// spans are walkable, every warp and exit leads to a map that exists, and no NPC
// stands on a solid tile or a blocking prop. Every player-facing string (lines, text, label,
// locked, title, and an interaction's speaker name) has a French `x_fr` twin of the same shape.
import { readFileSync, readdirSync } from "node:fs";
const dir = process.argv[2] ?? "web/world";
const src = readFileSync(`${dir}/props.js`, "utf8");
const FOOT = eval(`(${src.match(/export const FOOT = (\{[\s\S]*?\});/)[1]})`);
const DECK = eval(`(${src.match(/export const DECK = (\{[\s\S]*?\});/)[1]})`);
const WALKABLE = new Set(["rug", "exit_mat", ...Object.keys(DECK)]);
const BACK = { n: "s", s: "n", e: "w", w: "e" };
const maps = Object.fromEntries(readdirSync(`${dir}/maps`).map((f) => [f.replace(".json", ""), JSON.parse(readFileSync(`${dir}/maps/${f}`, "utf8"))]));
const fail = [];
const SAID = ["lines", "text", "label", "locked", "title"];
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
  const solid = new Set();
  m.tiles.forEach((row, z) => [...row].forEach((c, x) => "#fWV~".includes(c) && solid.add(`${x},${z}`)));
  for (const p of m.props) {
    let [w, d] = FOOT[p.asset] ?? [1, 1];
    if (p.rot % 180 === 90) [w, d] = [d, w];
    for (let x = p.x; x < p.x + w; x++)
      for (let z = p.z; z < p.z + d; z++) {
        const k = `${x},${z}`;
        if (WALKABLE.has(p.asset)) solid.delete(k);
        else solid.add(k);
      }
  }
  for (const n of m.npcs ?? []) if (solid.has(`${n.x},${n.z}`)) fail.push(`${name}: ${n.id} stands on a solid tile`);
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
