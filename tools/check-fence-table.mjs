// Run: node tools/check-fence-table.mjs web/world/props.js
// Check AUTOTILE in props.js: each piece's rot-0 arms, rotated like three.js rotation.y, equal the mask.
import { readFileSync } from "node:fs";
const src = readFileSync(process.argv[2], "utf8");
const FENCE = eval(src.match(/const AUTOTILE = (\[[\s\S]*?\]);/)[1]);
const DIR = { n: [0, -1], e: [1, 0], s: [0, 1], w: [-1, 0] }, sides = ["n", "e", "s", "w"];
const ARMS = { "": ["w", "e"], _corner: ["w", "s"], _end: ["w"], _t: ["w", "e", "s"] };
for (let mask = 1; mask < 15; mask++) {
  const [piece, rot] = FENCE[mask], t = (rot * Math.PI) / 180;
  let m = 0;
  for (const a of ARMS[piece]) {
    const [x, z] = DIR[a], rx = Math.round(x * Math.cos(t) + z * Math.sin(t)), rz = Math.round(-x * Math.sin(t) + z * Math.cos(t));
    m |= 1 << sides.findIndex((f) => DIR[f][0] === rx && DIR[f][1] === rz);
  }
  if (m !== mask) throw new Error(`mask ${mask}: ${piece}@${rot} gives ${m}`);
}
console.log("fence table ok");
