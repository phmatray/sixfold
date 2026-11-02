// Phone night frame time on the Tide and Storm maps (#268, AC4 of #265): 390 x 844, the CPU slowed 4x, 22:00,
// the hero on each map's spawn. Modeled on perf-walkers.mjs.
//
//   cd web && trunk build --release
//   cd ../e2e && npm ci && node perf-night.mjs [url] [--runs 3] [--throttle 4] [--maps seaside,downs] [--budget 16.7]
//
// Without `url` it serves ../web/dist itself (on PW_PORT, default 4403). It drives the installed Google Chrome in a
// visible window (headless Chrome has no GPU here and draws the world blank). Per map it prints the rAF median and
// p95, the share of frames over 20 ms and the main thread's busy time per frame (CDP TaskDuration, the room a frame
// had left). Exits 1 when a map's median busy time is over the budget; the rAF delta alone only says whether a
// frame fit the vsync. Frame times are only worth comparing between runs on the same machine at the same load.
import { chromium } from "@playwright/test";
import { spawn } from "node:child_process";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const args = process.argv.slice(2);
const opt = (name, def) => (args.includes(name) ? args[args.indexOf(name) + 1] : def);
const RUNS = +opt("--runs", 3), THROTTLE = +opt("--throttle", 4), BUDGET = +opt("--budget", 16.7), PORT = +(process.env.PW_PORT ?? 4403);
const MAPS = opt("--maps", "seaside,gullhaven,bellflats,downs,stormwatch,rodspire").split(",");
if (!(RUNS >= 1 && THROTTLE >= 1)) throw new Error("--runs and --throttle take a number, at least 1");
const given = args.find((a) => /^https?:/.test(a));
const URL_ = given ?? `http://127.0.0.1:${PORT}/`;
const spawnOf = (m) => JSON.parse(readFileSync(new URL(`../web/world/maps/${m}.json`, import.meta.url))).spawn;
const q = (xs, p) => [...xs].sort((a, b) => a - b)[Math.min(xs.length - 1, Math.floor(p * xs.length))];
const f1 = (x) => x.toFixed(1);

let server = null;
if (!given) {
  server = spawn(process.execPath, [fileURLToPath(new URL("serve.mjs", import.meta.url)), String(PORT)], { stdio: "ignore" });
  for (let i = 0; ; i++) {
    if (await fetch(URL_ + "index.html").then((r) => r.ok, () => false)) break;
    if (i > 50) throw new Error(`serve.mjs did not start on ${PORT} (built web/dist?)`);
    await new Promise((r) => setTimeout(r, 200));
  }
}
process.on("exit", () => server?.kill()); // a throw mid-run must not leave serve.mjs holding PW_PORT
const browser = await chromium.launch(process.env.HEADLESS ? { headless: true, args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"] } : { channel: "chrome", headless: false });
const ctx = await browser.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 3, isMobile: true, hasTouch: true, locale: "en-US", serviceWorkers: "block" });
const page = await ctx.newPage();
await page.addInitScript(() => sessionStorage.setItem("sixfold.splashed", "1"));
await page.goto(URL_);
await page.getByRole("navigation", { name: "Main menu" }).getByRole("button", { name: /^Explore the world/ }).click({ timeout: 120_000 });
await page.getByRole("button", { name: "Menu", exact: true }).waitFor({ timeout: 120_000 });
const skip = page.getByRole("button", { name: /^Skip/ });
for (let i = 0; i < 8 && (await skip.isVisible().catch(() => false)); i++) await skip.click().catch(() => {}), await page.waitForTimeout(1500);
await page.waitForFunction(() => document.querySelector(".world-host")?.sixfoldWarp, null, { timeout: 120_000 });
const host = (f, ...a) => page.evaluate(([f, a]) => document.querySelector(".world-host")[f](...a), [f, a]);
const cdp = await ctx.newCDPSession(page);
await cdp.send("Performance.enable");
const busy = async () => (await cdp.send("Performance.getMetrics")).metrics.find((m) => m.name === "TaskDuration").value * 1000;
const frames = (ms) => page.evaluate((ms) => new Promise((done) => {
  const out = [], end = performance.now() + ms;
  let last = performance.now();
  const frame = (now) => { out.push(now - last); last = now; now > end ? done(out) : requestAnimationFrame(frame); };
  requestAnimationFrame(frame);
}), ms);

let over = 0;
for (const map of MAPS) {
  const s = spawnOf(map), deltas = [], busies = [];
  await cdp.send("Emulation.setCPUThrottlingRate", { rate: 1 }); // the warp's load and shader compile are not the sample
  for (let t = 0; (await host("sixfoldStats")).area !== map; t++) {
    if (t > 40) throw new Error("never reached " + map);
    if (await skip.isVisible().catch(() => false)) await skip.click().catch(() => {});
    await host("sixfoldWarp", map, s.x, s.z, s.facing ?? "s");
    await page.waitForTimeout(1500);
  }
  for (let t = 0; t < 6 && (await skip.isVisible().catch(() => false)); t++) await skip.click().catch(() => {}), await page.waitForTimeout(1200);
  await host("sixfoldClock", +(process.env.HOUR_MIN ?? 22 * 60));
  await page.waitForTimeout(3000);
  await cdp.send("Emulation.setCPUThrottlingRate", { rate: THROTTLE });
  await page.waitForTimeout(2000); // the first throttled frames (JIT, shader recompile) are not the sample
  for (let r = 0; r < RUNS; r++) {
    const b0 = await busy(), d = await frames(4000), b1 = await busy();
    deltas.push(...d);
    busies.push((b1 - b0) / d.length);
  }
  const med = q(busies, 0.5), slow = deltas.filter((d) => d > 20).length / deltas.length;
  if (med > BUDGET) over++;
  const st = await host("sixfoldStats");
  console.log(`${map.padEnd(10)} ${deltas.length} frames  median ${f1(q(deltas, 0.5))} ms  p95 ${f1(q(deltas, 0.95))} ms  >20 ms ${(100 * slow).toFixed(1)}%  busy ${f1(med)} ms/frame (runs ${busies.map(f1).join(", ")})  graphics ${st.graphics}  ${st.calls} calls, ${st.triangles} tris  ${med > BUDGET ? "OVER" : "ok"}`);
}
await browser.close();
server?.kill();
process.exit(over ? 1 : 0);
