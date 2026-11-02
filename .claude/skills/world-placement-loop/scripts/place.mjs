// node place.mjs [--apply] <map> <asset> <x> <z> <toX> <toZ> [rot] [y]
// A placement you chose, judged exactly as the review app's "Place it yourself" does: the spots (interactions) on
// the prop go with it, its y follows the floor, and the answer lists the rules it still breaks there and whether
// check-maps passes without adding a warning. With --apply, and only if check-maps holds, it is written.
// Use it for what the fix generator can't propose: a move that needs judgment, a turn plus a move, putting a prop
// back. To move several props of one map at once (props trading places), see arrange.mjs.
import { REPO, WORLD } from "./lib.mjs";

const args = process.argv.slice(2), apply = args[0] === "--apply" && args.shift();
const [map, asset, x, z, tx, tz, rot, y] = args;
if (tz === undefined) throw new Error("usage: place.mjs [--apply] <map> <asset> <x> <z> <toX> <toZ> [rot] [y]");
process.chdir(REPO);
const { desk } = await import(`${REPO}/tools/placement-review.mjs`);
const d = desk(WORLD), view = d.mapView(map);
const index = view.props.findIndex((p) => p.asset === asset && p.x === +x && p.z === +z);
if (index < 0) throw new Error(`${map}: no ${asset} at ${x},${z}`);
const member = { index, asset, x: +x, z: +z };
const r = d.tryFix(map, member, { x: +tx, z: +tz, ...(rot !== undefined && { rot: +rot }), ...(y !== undefined && { y: +y }) });
console.log(`${map} ${asset} ${x},${z} → ${tx},${tz}${rot !== undefined ? ` rot ${rot}` : ""}: ${r.broken.length ? `still breaks ${r.broken.join(", ")}` : "keeps every rule"}; check-maps ${r.check.ok ? "holds" : `fails: ${r.check.first}`}`);
if (apply) {
  if (!r.check.ok) { console.log("not applied"); process.exit(1); }
  const done = d.apply({ map, items: [{ ...member, patch: r.patch }] });
  console.log(done.check.ok ? "applied" : "applied, but check-maps now FAILS: put it back");
}
process.exit(0);
