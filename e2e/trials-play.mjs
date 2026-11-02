// Plays the three trials (web/world/maps/trials.json) end to end in Chrome and takes screenshots along the way:
// each room's solution is found by search with the game's own rules (puzzle.js), then played with the pad
// (one step a click), Reach with E. A debug build must be serving: `cd web && trunk serve`.
//
//   cd e2e && node trials-play.mjs http://localhost:8080/ [out-dir]   (out-dir: the system temp folder's sixfold-trials/)
// With VIDEO=1 it also records the run (a .webm in out-dir).
import { chromium } from "@playwright/test";
import { mkdirSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { trials, blockAt, shove, reachTarget, slide, DIRS } from "../web/world/puzzle.js";

const URL_ = process.argv[2] ?? "http://localhost:8080/";
const OUT = process.argv[3] ?? join(tmpdir(), "sixfold-trials/");
mkdirSync(OUT, { recursive: true });

const map = JSON.parse(readFileSync(new URL("../web/world/maps/trials.json", import.meta.url), "utf8"));
const def = (id) => map.trials.find((t) => t.id === id);
const tile = (x, z) => map.tiles[z]?.[x];
const height = (x, z) => (tile(x, z) >= "1" && tile(x, z) <= "9" ? +tile(x, z) : 0);
const taken = new Set([...map.props.map((p) => `${p.x},${p.z}`), ...map.trials.filter((t) => t.reset).map((t) => String(t.reset)),
  ...map.gates.flatMap((g) => g.tiles.map(String))]);
const open = (ax, az, bx, bz) => tile(bx, bz) !== undefined && !"#fWV~".includes(tile(bx, bz)) && !taken.has(`${bx},${bz}`) && Math.abs(height(bx, bz) - height(ax, az)) <= 0.6;
const see = (hx, hz) => (x, z) => tile(x, z) !== undefined && height(x, z) - height(hx, hz) <= 1.5;
const F = Object.keys(DIRS);

// Breadth-first, keeping how each state was reached: the moves of a shortest solution.
function solve(start, moves, key, goal) {
  const prev = new Map([[key(start), null]]);
  let q = [start];
  while (q.length) {
    const next = [];
    for (const s of q) {
      if (goal(s)) {
        const out = [];
        for (let k = key(s); prev.get(k); k = prev.get(k).from) out.unshift(prev.get(k).move);
        return out;
      }
      for (const [move, n] of moves(s)) if (!prev.has(key(n))) prev.set(key(n), { from: key(s), move: { ...move, h: n.h ?? n } }), next.push(n);
    }
    q = next;
  }
  throw new Error("no solution");
}
const restore = (t, b) => t.blocks.forEach((q, i) => ([q.x, q.z] = b[i]));
const sk = ({ h, b }) => `${h}|${b.map(String).sort().join(";")}`;
const onPlates = (d) => ({ b }) => d.plates.every(([x, z]) => b.some((p) => p[0] === x && p[1] === z));
const step = (t) => ({ h, b }) => {
  restore(t, b);
  return F.flatMap((f) => {
    const [dx, dz] = DIRS[f], x = h[0] + dx, z = h[1] + dz, r = blockAt(t, x, z);
    if (!r) return open(h[0], h[1], x, z) ? [[{ f }, { h: [x, z], b }]] : [];
    const to = r.kind === "boulder" && shove(t, r, f, open);
    return to ? [[{ f }, { h: [x, z], b: b.map((p) => (p[0] === x && p[1] === z ? to : p)) }]] : [];
  });
};

const push = (() => {
  const d = def("push"), t = trials([d]);
  return solve({ h: [map.spawn.x, map.spawn.z], b: t.blocks.map((q) => [q.x, q.z]) }, step(t), sk, onPlates(d));
})();
const ice = (() => {
  const d = def("ice"), t = trials([d]);
  return solve(d.start, ([x, z]) => F.map((f) => [{ f }, slide(t, x, z, f, open)]).filter(([, p]) => p), String, ([x, z]) => x === d.goal[0] && z === d.goal[1]);
})();
const reach = (() => {
  const d = def("reach"), t = trials([d]);
  return solve({ h: d.start, b: t.blocks.map((q) => [q.x, q.z]) }, (s) => {
    const out = step(t)(s);
    restore(t, s.b);
    for (const f of F) {
      const p = reachTarget(t, s.h[0], s.h[1], f, see(...s.h)), to = p && shove(t, p, f, open);
      if (to) out.push([{ f, reach: true }, { h: s.h, b: s.b.map((q) => (q[0] === p.x && q[1] === p.z ? to : q)) }]);
    }
    return out;
  }, sk, onPlates(d));
})();
console.log(`solutions: push ${push.length} moves, ice ${ice.length} slides, reach ${reach.length} moves (${reach.filter((m) => m.reach).length} shoves)`);

const browser = await chromium.launch({ channel: "chrome", headless: false });
const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 }, locale: "en-US", serviceWorkers: "block", ...(process.env.VIDEO && { recordVideo: { dir: OUT, size: { width: 1280, height: 800 } } }) });
await ctx.routeWebSocket(/trunk\/ws/, () => {}); // trunk serve's live reload: never mid-run (an edit would restart the page)
const page = await ctx.newPage();
const errors = [];
page.on("pageerror", (e) => errors.push(String(e)));
page.on("console", (m) => m.type() === "error" && errors.push(m.text()));
await page.addInitScript(() => sessionStorage.setItem("sixfold.splashed", "1"));
await page.goto(`${URL_}?trials`);
await page.getByRole("navigation", { name: "Main menu" }).getByRole("button", { name: /^Explore the world/ }).click({ timeout: 180_000 });
await page.getByRole("button", { name: "Menu", exact: true }).waitFor({ timeout: 120_000 });
await page.waitForFunction(() => document.querySelector(".world-host")?.sixfoldStats?.().area === "trials", null, { timeout: 120_000 });
await page.waitForTimeout(3000);
const stats = () => page.evaluate(() => document.querySelector(".world-host").sixfoldStats());
const shot = async (name) => (await page.screenshot({ path: `${OUT}${name}.jpg`, type: "jpeg", quality: 80 }), console.log("shot", name));
// The hero on (x, z), its step over (the pad sends one step per click).
const at = async (x, z) => {
  for (let i = 0; i < 100; i++) {
    const s = await stats();
    if (s.at[0] === x && s.at[1] === z) return void (await page.waitForTimeout(120));
    await page.waitForTimeout(50);
  }
  throw new Error(`never reached ${x},${z} (at ${(await stats()).at})`);
};
const pad = (f) => page.evaluate((f) => document.querySelector(`.world-pad [data-dir="${f}"]`).click(), f);
// A dialogue open (a trial solved): read it, then close it line by line.
const talk = async (name) => {
  await page.waitForTimeout(1200);
  const box = page.locator(".talk-box");
  if (!(await box.isVisible().catch(() => false))) return console.log("  (no dialogue)");
  console.log("  says:", (await box.innerText()).replace(/\s+/g, " ").slice(0, 160));
  await shot(name);
  for (let i = 0; i < 8 && (await box.isVisible().catch(() => false)); i++) await page.keyboard.press("Enter"), await page.waitForTimeout(i ? 700 : 0);
  await page.waitForTimeout(450);
  await shot(`${name}-reveal`); // the seal sinking, its column of light
  await page.waitForTimeout(2800); // the camera watches the gate open, then comes back
  console.log("  focus after:", await page.evaluate(() => { const a = document.activeElement; return `${a?.tagName}.${a?.className} in dialog: ${!!a?.closest("dialog")}`; }));
};
const prompt = () => page.locator(".world-prompt").innerText().catch(() => "");

await shot("0-start");
// 1. Push.
for (const [i, m] of push.entries()) {
  await pad(m.f);
  await at(...m.h);
  if (i === Math.floor(push.length / 2)) await shot("1-push-midway");
}
await page.waitForTimeout(1200); // the last boulder settles; the camera glides to the gate
await talk("1-push-solved");
await shot("1-push-open");
// Into the ice room: through the gate to its first tile.
await page.evaluate(() => document.querySelector(".world-host").sixfoldWalk(11, 4));
await at(11, 4);
await shot("2-ice-start");
// 2. Ice.
for (const [i, m] of ice.entries()) {
  await pad(m.f);
  await at(...m.h);
  await page.waitForTimeout(150);
  if (i === 2) await shot("2-ice-slide");
}
await shot("2-ice-across");
// 3. Reach.
await page.evaluate(() => document.querySelector(".world-host").sixfoldWalk(21, 8));
await at(21, 8);
await shot("3-reach-start");
let shoves = 0;
for (const m of reach) {
  if (!m.reach) {
    await pad(m.f);
    await at(...m.h);
    continue;
  }
  // Turn to it (a short press only turns), then E.
  const s = await stats();
  // Held for a few frames (an instant press is gone before the world reads it), under the 0.12 s that makes a step.
  const arrow = { n: "ArrowUp", s: "ArrowDown", e: "ArrowRight", w: "ArrowLeft" }[m.f];
  if (s.at[2] !== m.f) await page.keyboard.down(arrow), await page.waitForTimeout(60), await page.keyboard.up(arrow), await page.waitForTimeout(250);
  if ((await stats()).at[2] !== m.f) throw new Error(`did not turn ${m.f} at ${s.at}`);
  const p = await prompt();
  if (!shoves) console.log("  prompt before Reach:", p.replace(/\s+/g, " "));
  await page.keyboard.press("e");
  shoves++;
  if (shoves === 1) await page.waitForTimeout(260), await shot("3-reach-hand"); // the hand about to strike
  await page.waitForTimeout(shoves === 1 ? 850 : 1100); // the hand flies, strikes and fades
}
await talk("3-reach-solved");
await shot("3-reach-open");
await page.evaluate(() => document.querySelector(".world-host").sixfoldWalk(29, 1));
// Stepping into the niche is the way out (its warp): the hero leaves the cave.
await page.waitForFunction(() => document.querySelector(".world-host").sixfoldStats().area !== "trials", null, { timeout: 20_000 })
  .then(async () => console.log("  out to:", (await stats()).area), (e) => console.log("  exit FAILED:", e.message.split("\n")[0]));
await page.waitForTimeout(2000);
await shot("4-exit");
console.log("errors:", errors.length ? errors : "none");
await ctx.close(); // the video (VIDEO=1) is written now
if (process.env.VIDEO) console.log("video:", await page.video().path());
await browser.close();
