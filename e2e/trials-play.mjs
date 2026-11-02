// Plays the field-puzzle prototype (docs/PUZZLES.md) end to end in Chrome, with screenshots: from the hall, each
// trial's two stages, every stage's shortest solution found by tools/puzzles.mjs on the game's own rules and played
// with the pad (one move a click). A debug build must be serving: `cd web && trunk serve`.
//
//   cd e2e && node trials-play.mjs http://localhost:8080/ [out-dir]   (out-dir: the system temp folder's sixfold-trials/)
// With VIDEO=1 it also records the run (a .webm in out-dir). ONLY=depot|ice|logs plays one trial.
import { chromium } from "@playwright/test";
import { mkdirSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { measure } from "../tools/puzzles.mjs";

const URL_ = process.argv[2] ?? "http://localhost:8080/";
const OUT = process.argv[3] ?? join(tmpdir(), "sixfold-trials/");
mkdirSync(OUT, { recursive: true });
const map = (name) => JSON.parse(readFileSync(new URL(`../web/world/maps/${name}.json`, import.meta.url), "utf8"));
const plan = (name, id, start) => {
  const m = measure(map(name), id, start && { start });
  if (!m.solvable) throw new Error(`${name} ${id}: no solution`);
  console.log(`${name} ${id}: ${m.moves} moves, ${m.pushes} pushes`);
  return m.moveList;
};

const browser = await chromium.launch({ channel: "chrome", headless: false });
const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 }, locale: "en-US", serviceWorkers: "block", ...(process.env.VIDEO && { recordVideo: { dir: OUT, size: { width: 1280, height: 800 } } }) });
await ctx.routeWebSocket(/trunk\/ws/, () => {}); // trunk serve's live reload: never mid-run (an edit would restart the page)
const page = await ctx.newPage();
const errors = [];
page.on("pageerror", (e) => errors.push(String(e)));
page.on("console", (m) => m.type() === "error" && errors.push(m.text()));
await page.addInitScript(() => sessionStorage.setItem("sixfold.splashed", "1"));
await page.goto(`${URL_}?trials`);
await page.getByRole("navigation", { name: "Main menu" }).getByRole("button", { name: /^(Explore the world|Continue)/ }).click({ timeout: 180_000 });
await page.getByRole("button", { name: "Menu", exact: true }).waitFor({ timeout: 120_000 });
await page.waitForFunction(() => document.querySelector(".world-host")?.sixfoldStats?.().area === "trials", null, { timeout: 120_000 });
await page.waitForTimeout(2500);
const stats = () => page.evaluate(() => document.querySelector(".world-host").sixfoldStats());
const shot = async (name) => (await page.screenshot({ path: `${OUT}${name}.jpg`, type: "jpeg", quality: 80 }), console.log("shot", name));
// The hero on (x, z) in `area`, its step over.
const at = async (x, z, area) => {
  for (let i = 0; i < 200; i++) {
    const s = await stats();
    if (s.at[0] === x && s.at[1] === z && (!area || s.area === area)) return void (await page.waitForTimeout(120));
    await page.waitForTimeout(50);
  }
  throw new Error(`never reached ${x},${z} (at ${(await stats()).at} in ${(await stats()).area})`);
};
const pad = (f) => page.evaluate((f) => document.querySelector(`.world-pad [data-dir="${f}"]`).click(), f);
// A trial solved: the camera glides to what it opened, its lines show; read them, close them, watch the gate open.
const talk = async (name) => {
  await page.waitForTimeout(1300);
  const box = page.locator(".talk-box");
  if (!(await box.isVisible().catch(() => false))) return console.log("  (no lines)");
  console.log("  says:", (await box.innerText()).replace(/\s+/g, " ").slice(0, 140));
  await shot(name);
  for (let i = 0; i < 8 && (await box.isVisible().catch(() => false)); i++) await page.keyboard.press("Enter"), await page.waitForTimeout(i ? 700 : 0);
  await page.waitForTimeout(450);
  await shot(`${name}-reveal`);
  await page.waitForTimeout(2800); // the camera watches the gate open, then comes back
};
// Play a stage's moves; a screenshot about halfway.
async function play(moves, name, area) {
  for (const [i, m] of moves.entries()) {
    await pad(m.f);
    await at(...m.h, area);
    if (m.push) await page.waitForTimeout(200); // let a crate settle, a log finish rolling
    if (i === Math.floor(moves.length / 2)) await shot(`${name}-midway`);
  }
  await page.waitForTimeout(700);
}
// Through a door of the hall (its warp tile, then a push against the wall), into `area`.
async function enter(area, x, z, f) {
  await page.evaluate(([x, z]) => document.querySelector(".world-host").sixfoldWalk(x, z), [x, z]);
  await at(x, z, "trials");
  await pad(f);
  await page.waitForFunction((a) => document.querySelector(".world-host").sixfoldStats().area === a, area, { timeout: 20_000 });
  await page.waitForTimeout(2200);
}
async function leave(x, z) {
  await page.evaluate(([x, z]) => document.querySelector(".world-host").sixfoldWalk(x, z), [x, z]);
  await at(x, z);
  await pad("s");
  await page.waitForFunction(() => document.querySelector(".world-host").sixfoldStats().area === "trials", null, { timeout: 20_000 });
  await page.waitForTimeout(1500);
}
const only = process.env.ONLY;

await shot("0-hall");
if (!only || only === "depot") {
  await enter("trial_depot", 1, 4, "w");
  await shot("1-depot");
  await play(plan("trial_depot", "depot1"), "1-depot-front", "trial_depot");
  await talk("1-depot-front-solved");
  await page.evaluate(() => document.querySelector(".world-host").sixfoldWalk(4, 6));
  await at(4, 6, "trial_depot");
  await play(plan("trial_depot", "depot2", [4, 6]), "1-depot-back", "trial_depot");
  await talk("1-depot-back-solved");
  await leave(4, 11);
}
if (!only || only === "ice") {
  await enter("trial_ice", 5, 1, "n");
  await shot("2-ice");
  await play(plan("trial_ice", "ice1"), "2-ice-first", "trial_ice");
  await pad("n");
  await at(5, 8, "trial_ice");
  await play(plan("trial_ice", "ice2", [5, 8]), "2-ice-second", "trial_ice");
  await talk("2-ice-card");
}
if (!only || only === "logs") {
  if (!only) await page.evaluate(() => document.querySelector(".world-host").sixfoldWarp("trials", 8, 4, "e")), await page.waitForTimeout(2500);
  await enter("trial_logs", 9, 4, "e");
  await shot("3-logs");
  await play(plan("trial_logs", "logs1"), "3-logs-first", "trial_logs");
  await pad("n"); // up into the passage, where the second room starts
  await at(...map("trial_logs").trials[1].start, "trial_logs");
  await play(plan("trial_logs", "logs2", map("trial_logs").trials[1].start), "3-logs-second", "trial_logs");
  await talk("3-logs-done");
}
console.log("errors:", errors.length ? errors : "none");
await ctx.close(); // the video (VIDEO=1) is written now
if (process.env.VIDEO) console.log("video:", await page.video().path());
await browser.close();
