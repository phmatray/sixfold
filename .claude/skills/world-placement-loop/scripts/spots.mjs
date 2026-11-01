// node spots.mjs <out.json> [--severity warn|error|info|all] [--map a,b] [--rule r1,r2] [--ids id1,id2]
//   [--at map:x,z[:x,z…] …]
// The places to photograph, from the open findings (placement.mjs --json): findings of one map and rule within 3
// tiles of each other make one spot. --at adds a spot by hand (a room, the place a prop is moving to), named after
// its first tile. Each spot: { map, block, rule, members: [{ id, x, z }] }.
import { writeFileSync, mkdirSync } from "node:fs";
import { dirname } from "node:path";
import { execFileSync } from "node:child_process";
import { REPO, WORLD } from "./lib.mjs";

const args = process.argv.slice(2), out = args.shift();
const opt = (k) => { const i = args.indexOf(`--${k}`); return i < 0 ? null : args[i + 1]; };
const list = (k) => opt(k)?.split(",") ?? null;
const severity = opt("severity") ?? "warn", maps = list("map"), rules = list("rule"), ids = list("ids");
const findings = JSON.parse(execFileSync(process.execPath, [`${REPO}/tools/placement.mjs`, WORLD, "--json"], { encoding: "utf8", maxBuffer: 1 << 26 }));
const keep = findings.filter((f) => (severity === "all" || f.severity === severity) && (!maps || maps.includes(f.map)) && (!rules || rules.includes(f.rule)) && (!ids || ids.includes(f.id)));
const spots = [];
for (const f of keep) {
  const s = spots.find((s) => s.map === f.map && s.rule === f.rule && s.members.some((m) => Math.max(Math.abs(m.x - f.x), Math.abs(m.z - f.z)) <= 3));
  if (s) s.members.push({ id: f.id, x: f.x, z: f.z });
  else spots.push({ map: f.map, block: f.id, rule: f.rule, members: [{ id: f.id, x: f.x, z: f.z }] });
}
args.forEach((a, i) => {
  if (a !== "--at") return;
  const [map, ...tiles] = args[i + 1].split(":"), members = tiles.map((t) => { const [x, z] = t.split(",").map(Number); return { x, z }; });
  spots.push({ map, block: `at${members[0].x}-${members[0].z}`, rule: "by hand", members });
});
mkdirSync(dirname(out), { recursive: true });
writeFileSync(out, JSON.stringify(spots, null, 1));
for (const s of spots) console.log(`${s.block}  ${s.map}  ${s.rule}  ${s.members.map((m) => `${m.x},${m.z}`).join(" ")}`);
console.log(`${spots.length} spot(s) in ${out}`);
