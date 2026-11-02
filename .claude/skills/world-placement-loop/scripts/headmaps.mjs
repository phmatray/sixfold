// node headmaps.mjs <out dir> <map>…
// The maps as committed (git HEAD), for shots.mjs --from: "before" photos of maps you have already edited.
import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { REPO } from "./lib.mjs";

const [out, ...maps] = process.argv.slice(2);
mkdirSync(out, { recursive: true });
for (const n of maps) writeFileSync(`${out}/${n}.json`, execFileSync("git", ["-C", REPO, "show", `HEAD:web/world/maps/${n}.json`]));
console.log(`${maps.length} map(s) as committed in ${out}`);
