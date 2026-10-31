// Run: node tools/transition.mjs web/world/maps/<map>.json <side> [<to>]
// Biome transitions (#118, WORLD_RULES 5.7): writes the band of the map's edge exit on <side> (name the map it
// leads to when that side has two exits). It adds the map's `transitions` entry if it has none ({ to, depth 5,
// scatter: the next region's plants and rocks }; edit depth and scatter by hand, then re-run), and scatters
// props of that set over the band, thinning from the edge (ld.bandWeight). The props carry `"band": "<to>"`, so a
// re-run replaces them; a fixed seed per map and side makes it write the same thing every time. It keeps off
// paths and the tile round them, the exit's throat, 2 tiles round anything you use, and any tile whose loss
// would cut the hero off from another (a tile already out of reach is fair game); reeds, sea rocks and floating
// lanterns may stand in the water. Where the next region's usual plants can't fit (a border of trees, a causeway),
// name what can in `scatter`: props of that set already in the band count too. The ground, light and sound of the band need nothing more: the game reads
// them off the map next door. Then run check-maps (--strict).
import { readFileSync, writeFileSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import * as ld from "../web/world/ld.js";

// The plants and rocks a band borrows from each region, low ones first (they read best by a path).
const SCATTER = { hub: ["bush", "flowers", "rock"], ember: ["rock_basalt", "rock", "ash_pine"], storm: ["heather", "rock"],
  tide: ["reeds", "rock_sea"], dream: ["moonflower", "reeds", "bush"], grove: ["bush_dark", "stump", "flowers"], stone: ["rock", "heather"] };
// What may stand in the water ("~") too, and what only floats there.
const WET = new Set(["reeds", "rock_sea", "floating_lantern"]), AFLOAT = new Set(["floating_lantern"]);
const POST = new Set(["sign", "signpost", "waymark", "hanging_sign", "inn_sign"]);
const CLIMB = 0.6; // build.js: the highest step the hero takes

const [file, side, to] = process.argv.slice(2);
if (!file || !"nsew".includes(side ?? "-")) throw new Error("usage: node tools/transition.mjs <map.json> <n|s|e|w> [<to>]");
const dir = dirname(file), name = basename(file, ".json");
const read = (f) => JSON.parse(readFileSync(f, "utf8"));
const m = read(file);
const exits = (m.exits ?? []).filter((e) => e.side === side && (!to || e.to === to));
if (exits.length !== 1) throw new Error(`${name}: ${exits.length} exits on side ${side}${to ? ` to ${to}` : ""}; name the map it leads to`);
const exit = exits[0], next = read(join(dir, `${exit.to}.json`));
if (m.exits.filter((e) => e.to === exit.to).length > 1) throw new Error(`${name}: two exits lead to ${exit.to}; a band is named by the map it leads to`);

// The props.js tables (props.js imports three, so read them the way check-maps does).
const src = readFileSync(join(dir, "..", "props.js"), "utf8");
const table = (re) => eval(`(${src.match(re)[1]})`);
const FOOT = table(/export const FOOT = (\{[\s\S]*?\});/), DECK = table(/export const DECK = (\{[\s\S]*?\});/);
const WALKABLE = new Set(table(/export const WALKABLE = new Set\((\[[\s\S]*?\])\);/));
const LADDER = new Set(table(/export const LADDER = new Set\((\[[\s\S]*?\])\);/));

m.transitions ??= [];
let t = m.transitions.find((t) => t.to === exit.to);
if (!t) m.transitions.push((t = { to: exit.to, depth: 5, scatter: SCATTER[next.region] ?? ["rock"] }));
// The band's props go back where they were in the list, so a map with two bands re-runs byte for byte.
const slot = m.props.slice(0, Math.max(0, m.props.findIndex((p) => p.band === exit.to))).filter((p) => p.band !== exit.to).length;
const had = m.props.some((p) => p.band === exit.to);
m.props = m.props.filter((p) => p.band !== exit.to);
const fresh = [];
const b = ld.bandOf(m, t);

const key = (x, z) => `${x},${z}`;
const tilesOf = (p) => {
  const [w, d] = ld.foot(p, FOOT), t = [];
  for (let dx = 0; dx < w; dx++) for (let dz = 0; dz < d; dz++) t.push([p.x + dx, p.z + dz]);
  return t;
};
const g = ld.grids(m, { FOOT, DECK, WALKABLE, tall: () => 1 });
const open = new Set();
for (let z = 0; z < m.height; z++) for (let x = 0; x < m.width; x++) if (!g.solid.has(key(x, z))) open.add(key(x, z));
// A ladder joins any heights (leniently, either way: a tile it might reach counts as reached, so it stays free).
const ladder = new Set(m.props.filter((p) => LADDER.has(p.asset)).flatMap(tilesOf).map(([x, z]) => key(x, z)));
const step = (ax, az, bx, bz) => ladder.has(key(ax, az)) || ladder.has(key(bx, bz)) || Math.abs((g.floor[key(ax, az)] ?? 0) - (g.floor[key(bx, bz)] ?? 0)) <= CLIMB;
const starts = [[m.spawn.x, m.spawn.z], ...(m.exits ?? []).flatMap((e) => ld.edgeTiles(m, e))];
const reach = () => ld.bfs(open, starts, step).dist.size;

// Anything you use: spots, people, doors and posts keep 2 tiles clear.
const used = [...(m.interactions ?? []), ...(m.npcs ?? []), ...(m.warps ?? []), ...m.props.filter((p) => POST.has(p.asset))];
const near = (x, z, list, r) => list.some((p) => Math.abs(p.x - x) <= r && Math.abs(p.z - z) <= r);
const pathNear = (x, z) => [-1, 0, 1].some((dx) => [-1, 0, 1].some((dz) => g.path.has(key(x + dx, z + dz))));
const along = (x, z) => (side === "n" || side === "s" ? x : z);
const taken = new Set(m.props.flatMap(tilesOf).map(([x, z]) => key(x, z))); // rugs, tidepools and arches too
const fits = (x, z, asset) => {
  const c = m.tiles[z]?.[x], wet = c === "~" && WET.has(asset), dry = c === "." && !AFLOAT.has(asset) && open.has(key(x, z));
  return (wet || dry) && !taken.has(key(x, z)) && !pathNear(x, z) && (along(x, z) < b.span[0] - 2 || along(x, z) > b.span[1] + 2) && !near(x, z, used, 2);
};

// mulberry32, seeded by the map and the exit.
let seed = [...`${name}>${exit.to}`].reduce((h, c) => (Math.imul(h, 31) + c.charCodeAt(0)) | 0, 7);
const rand = () => {
  seed = (seed + 0x6d2b79f5) | 0;
  let r = Math.imul(seed ^ (seed >>> 15), 1 | seed);
  r = (r + Math.imul(r ^ (r >>> 7), 61 | r)) ^ r;
  return ((r ^ (r >>> 14)) >>> 0) / 4294967296;
};

let reached = reach(), placed = 0;
for (let z = 0; z < m.height; z++)
  for (let x = 0; x < m.width; x++) {
    const w = ld.bandWeight(m, b, x + 0.5, z + 0.5);
    if (w <= 0) continue;
    const roll = rand(), asset = t.scatter[Math.floor(rand() * t.scatter.length)], rot = [0, 90, 180, 270][Math.floor(rand() * 4)];
    if (roll >= 0.55 * w) continue;
    const p = { asset, x, z, rot, band: exit.to }, tiles = tilesOf(p);
    if (!tiles.every(([a, c]) => fits(a, c, asset))) continue;
    for (const [a, c] of tiles) open.delete(key(a, c));
    const now = reach();
    if (now !== reached - tiles.length && now !== reached) {
      for (const [a, c] of tiles) open.add(key(a, c)); // it would cut a tile off: leave it open
      continue;
    }
    reached = now;
    for (const [a, c] of tiles) taken.add(key(a, c));
    fresh.push(p);
    placed++;
  }
m.props.splice(had ? slot : m.props.length, 0, ...fresh);
writeFileSync(file, JSON.stringify(m, null, 2) + "\n");
console.log(`${name}: ${placed} props in the ${t.depth}-tile band to ${exit.to} (${t.scatter.join(", ")})`);
