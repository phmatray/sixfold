// node shots.mjs <spots.json> <out dir> <before|after> [--from <dir of map JSONs>] [--part i/n]
// Photographs every spot in the running game (GAME, default http://localhost:8080; `cd web && trunk serve`): the
// hero warped just south of the spot, the camera pulled back to frame it, at noon, motion reduced. Shots land in
// <out>/<phase>/<map>-<block>.png; a rerun skips the ones already there.
// --from hands the game those maps first (map.js setMap): "before" shots of maps you already edited — make the
// folder with headmaps.mjs. --part i/n takes every n-th spot from the i-th, to run two browsers side by side.
import { mkdirSync, existsSync } from "node:fs";
import { REPO, WORLD, GAME, playwright, readJson, mapFile } from "./lib.mjs";

const args = process.argv.slice(2), [spotsPath, out, phase] = args;
if (!["before", "after"].includes(phase)) throw new Error("usage: shots.mjs <spots.json> <out dir> <before|after> [--from dir] [--part i/n]");
const opt = (k) => { const i = args.indexOf(`--${k}`); return i < 0 ? null : args[i + 1]; };
const FROM = opt("from"), [part, parts] = (opt("part") ?? "0/1").split("/").map(Number);
const spots = readJson(spotsPath);
mkdirSync(`${out}/${phase}`, { recursive: true });
const { tables } = await import(`${REPO}/tools/placement.mjs`);
const T = tables(WORLD), maps = {};
const mapOf = (n) => (maps[n] ??= readJson(FROM && existsSync(`${FROM}/${n}.json`) ? `${FROM}/${n}.json` : mapFile(n)));

// Where the hero stands: the nearest open tile 2 rows south of the spot's centre (the camera looks north).
function stand(spot) {
  const m = mapOf(spot.map), xs = spot.members.map((p) => p.x), zs = spot.members.map((p) => p.z);
  const cx = Math.round((Math.min(...xs) + Math.max(...xs)) / 2), cz = Math.max(...zs) + 2, taken = new Set();
  for (const p of m.props) {
    if (T.WALKABLE.has(p.asset) || T.DECK[p.asset]) continue;
    let [w, d] = T.FOOT[p.asset] ?? [1, 1];
    if (p.rot % 180 === 90) [w, d] = [d, w];
    for (let x = p.x; x < p.x + w; x++) for (let z = p.z; z < p.z + d; z++) taken.add(`${x},${z}`);
  }
  const open = (x, z) => m.tiles[z]?.[x] !== undefined && !"#fWV~".includes(m.tiles[z][x]) && !taken.has(`${x},${z}`);
  for (let r = 0; r < 12; r++) for (let dz = -r; dz <= r; dz++) for (let dx = -r; dx <= r; dx++) if (Math.max(Math.abs(dx), Math.abs(dz)) === r && open(cx + dx, cz + dz)) return [cx + dx, cz + dz];
  return [cx, cz];
}

const { chromium } = await playwright();
const browser = await chromium.launch({ args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--ignore-gpu-blocklist"] });
const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 }, serviceWorkers: "block", locale: "en-US", reducedMotion: "reduce" });
await ctx.addInitScript(() => sessionStorage.setItem("sixfold.splashed", "1"));
const page = await ctx.newPage();
await page.goto(GAME, { timeout: 60_000 }).catch((e) => { throw new Error(`the game isn't running at ${GAME} (cd web && trunk serve, or set GAME=<url>): ${e.message.split("\n")[0]}`); });
await page.getByRole("navigation", { name: "Main menu" }).getByRole("button", { name: /^Explore the world/ }).click({ timeout: 240_000 });
await page.getByRole("button", { name: "Menu", exact: true }).waitFor({ timeout: 120_000 });
await page.waitForFunction(() => document.querySelector(".world-host")?.sixfoldWarp, null, { timeout: 120_000 });
const host = (f, ...a) => page.evaluate(([f, a]) => document.querySelector(".world-host")[f](...a), [f, a]);
const skip = page.getByRole("button", { name: /^Skip/ });
const skipScenes = async () => { for (let t = 0; t < 10 && (await skip.isVisible().catch(() => false)); t++) await skip.click().catch(() => {}), await page.waitForTimeout(700); };
await skipScenes(); // the new game's opening scene

let n = 0;
for (const spot of spots.filter((_, i) => i % parts === part)) {
  const file = `${out}/${phase}/${spot.map}-${spot.block}.png`;
  if (existsSync(file)) continue;
  const [x, z] = stand(spot), xs = spot.members.map((p) => p.x), zs = spot.members.map((p) => p.z);
  const extent = Math.max(Math.max(...xs) - Math.min(...xs), Math.max(...zs) - Math.min(...zs));
  const cam = { distance: Math.min(30, Math.max(13, extent * 1.3)), pitch: 50 };
  if (FROM && existsSync(`${FROM}/${spot.map}.json`)) await page.evaluate(async ([n, m]) => (await import("/world/map.js")).setMap(n, m), [spot.map, readJson(`${FROM}/${spot.map}.json`)]);
  // Warp and wait for the map and the hero (the game puts the hero on the nearest free tile, so within 2 tiles is
  // there); a warp asked during a fade is ignored, so try again. A site's arrival scene takes the camera: skip it.
  let ok = false;
  for (let tries = 0; tries < 4 && !ok; tries++) {
    await host("sixfoldWarp", spot.map, x, z, "n");
    for (let t = 0; t < 12 && !ok; t++) {
      await page.waitForTimeout(500);
      const st = await page.evaluate(() => document.querySelector(".world-host").sixfoldStats());
      ok = st.area === spot.map && Math.abs(st.at?.[0] - x) <= 2 && Math.abs(st.at?.[1] - z) <= 2;
    }
    await page.waitForTimeout(800);
    if (await skip.isVisible().catch(() => false)) await skipScenes(), (ok = false);
  }
  if (!ok) console.log(`\n${spot.map} ${spot.block}: never got there`);
  await host("sixfoldClock", 12 * 60);
  await host("sixfoldCam", cam);
  await page.waitForTimeout(1800);
  for (let again = 0; again < 3 && (await skip.isVisible().catch(() => false)); again++) { // a scene that started late
    await skipScenes();
    await host("sixfoldWarp", spot.map, x, z, "n");
    await page.waitForTimeout(3000);
    await host("sixfoldCam", cam);
    await page.waitForTimeout(1500);
  }
  try { await page.screenshot({ path: file, timeout: 120_000 }); n++; }
  catch (e) { console.log(`\n${spot.map} ${spot.block}: no shot (${e.message.split("\n")[0]})`); }
  process.stdout.write(".");
}
console.log(`\n${n} shot(s) in ${out}/${phase}`);
await browser.close();
