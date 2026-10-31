// Run: node tools/transition.mjs web/world/maps/<map>.json <side> [<to>]
// Biome transitions (#118, WORLD_RULES 5.7): writes the band of the map's edge exit on <side> (name the map it
// leads to when that side has two exits). It adds the map's `transitions` entry if it has none ({ to, depth 5,
// scatter: the next region's plants and rocks }; edit depth and scatter by hand, then re-run), and scatters
// props of that set over the band, thinning from the edge (ld.bandWeight). The props carry `"band": "<to>"`, so a
// re-run replaces them; a fixed seed per map and side makes it write the same thing every time. It keeps off
// paths and the tile round them, the exit's throat, 3 tiles round anything you use, and any tile whose loss
// would cut the hero off from another. The ground, light and sound of the band need nothing more: the game reads
// them off the map next door. Then run check-maps (--strict).
import { readFileSync, writeFileSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import * as ld from "../web/world/ld.js";

// The plants and rocks a band borrows from each region, low ones first (they read best by a path).
const SCATTER = { hub: ["bush", "flowers", "rock"], ember: ["rock_basalt", "rock", "ash_pine"], storm: ["heather", "rock"],
  tide: ["reeds", "rock_sea"], dream: ["moonflower", "reeds", "bush"], grove: ["bush_dark", "stump", "flowers"], stone: ["rock", "heather"] };
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

// The props.js tables (props.js imports three, so read them the way check-maps does).
const src = readFileSync(join(dir, "..", "props.js"), "utf8");
const table = (re) => eval(`(${src.match(re)[1]})`);
const FOOT = table(/export const FOOT = (\{[\s\S]*?\});/), DECK = table(/export const DECK = (\{[\s\S]*?\});/);
const WALKABLE = new Set(table(/export const WALKABLE = new Set\((\[[\s\S]*?\])\);/));

m.transitions ??= [];
let t = m.transitions.find((t) => t.to === exit.to);
if (!t) m.transitions.push((t = { to: exit.to, depth: 5, scatter: SCATTER[next.region] ?? ["rock"] }));
m.props = m.props.filter((p) => p.band !== exit.to);
const b = ld.bandOf(m, t);

const key = (x, z) => `${x},${z}`;
const g = ld.grids(m, { FOOT, DECK, WALKABLE, tall: () => 1 });
const open = new Set();
for (let z = 0; z < m.height; z++) for (let x = 0; x < m.width; x++) if (!g.solid.has(key(x, z))) open.add(key(x, z));
const step = (ax, az, bx, bz) => Math.abs((g.floor[key(ax, az)] ?? 0) - (g.floor[key(bx, bz)] ?? 0)) <= CLIMB;
const starts = [[m.spawn.x, m.spawn.z], ...(m.exits ?? []).flatMap((e) => ld.edgeTiles(m, e))];
const reach = () => ld.bfs(open, starts, step).dist.size;

// Anything you use: spots, people, doors and posts keep 3 tiles clear.
const used = [...(m.interactions ?? []), ...(m.npcs ?? []), ...(m.warps ?? []), ...m.props.filter((p) => POST.has(p.asset))];
const near = (x, z, list, r) => list.some((p) => Math.abs(p.x - x) <= r && Math.abs(p.z - z) <= r);
const pathNear = (x, z) => [-1, 0, 1].some((dx) => [-1, 0, 1].some((dz) => g.path.has(key(x + dx, z + dz))));
const along = (x, z) => (side === "n" || side === "s" ? x : z);
const fits = (x, z) =>
  x >= 0 && z >= 0 && x < m.width && z < m.height && m.tiles[z][x] === "." && open.has(key(x, z)) && !pathNear(x, z) &&
  (along(x, z) < b.span[0] - 2 || along(x, z) > b.span[1] + 2) && !near(x, z, used, 3);

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
    const p = { asset, x, z, rot, band: exit.to }, [fw, fd] = ld.foot(p, FOOT), tiles = [];
    for (let dx = 0; dx < fw; dx++) for (let dz = 0; dz < fd; dz++) tiles.push([x + dx, z + dz]);
    if (!tiles.every(([a, c]) => fits(a, c))) continue;
    for (const [a, c] of tiles) open.delete(key(a, c));
    const now = reach();
    if (now !== reached - tiles.length) {
      for (const [a, c] of tiles) open.add(key(a, c)); // it would cut a tile off: leave it open
      continue;
    }
    reached = now;
    m.props.push(p);
    placed++;
  }
writeFileSync(file, JSON.stringify(m, null, 2) + "\n");
console.log(`${name}: ${placed} props in the ${t.depth}-tile band to ${exit.to} (${t.scatter.join(", ")})`);
