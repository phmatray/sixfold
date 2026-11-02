// node arrange.mjs <plan.json>
// Rearranges props by hand, several per map at once: plan = { "<map>": [[asset, x, z, toX, toZ, rot?, carrySpots?], …] }.
// Every prop of a map is found first, then all are moved, so props can trade places (a shelf slides east where a
// plant stood, the plant goes the other side of the window). carrySpots (default true) moves the interactions on
// the prop's old footprint with it; pass false for a bed whose spot should stay reachable where it is.
// rot: omit or null to keep it. It never deletes rot (the game would not draw the prop).
// This writes without judging: run check-maps and placement.mjs right after, and look at the shots.
import { readFileSync, writeFileSync } from "node:fs";
import { REPO, WORLD, mapFile, mapText } from "./lib.mjs";

const { tables } = await import(`${REPO}/tools/placement.mjs`);
const T = tables(WORLD), plan = JSON.parse(readFileSync(process.argv[2], "utf8"));
for (const [map, moves] of Object.entries(plan)) {
  const m = JSON.parse(readFileSync(mapFile(map), "utf8"));
  const found = moves.map(([asset, x, z]) => {
    const p = m.props.find((q) => q.asset === asset && q.x === x && q.z === z);
    if (!p) throw new Error(`${map}: no ${asset} at ${x},${z}`);
    let [w, d] = T.FOOT[asset] ?? [1, 1];
    if ((p.rot ?? 0) % 180 === 90) [w, d] = [d, w];
    return { p, spots: (m.interactions ?? []).filter((s) => s.x >= x && s.x < x + w && s.z >= z && s.z < z + d) };
  });
  moves.forEach(([, x, z, tx, tz, rot, carry = true], i) => {
    const { p, spots } = found[i];
    p.x = tx, p.z = tz;
    if (rot !== undefined && rot !== null) p.rot = rot;
    p.rot ??= 0;
    if (carry) for (const s of spots) (s.x += tx - x), (s.z += tz - z);
  });
  writeFileSync(mapFile(map), mapText(m));
  console.log(`${map}: ${moves.length} move(s)`);
}
