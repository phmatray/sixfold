// Frame time on the busiest map of the Ember and Stone regions at night (AC4 of #264): the phone viewport
// (390 x 844), the CPU slowed 4x, the hero standing in the middle of the map at 22:00. Modeled on perf-walkers.mjs.
//
//   cd web && trunk build --release
//   cd ../e2e && npm ci && node perf-night.mjs [url] [--maps kilnmouth,anvilstead] [--runs 3] [--throttle 4] [--webkit]
//
// Without `url` it serves ../web/dist itself (on PW_PORT, default 4403). It drives the installed Google Chrome in a
// visible window (headless Chrome draws the world blank). Per map: warp (`el.sixfoldWarp`), hour 22:00
// (`el.sixfoldClock`), wait for the shaders, then every frame's rAF delta and the main thread's busy time (CDP
// TaskDuration) over 8 s, per run. It prints; it exits 1 only when a map's median busy time is over 16.7 ms.
import { chromium, webkit } from "@playwright/test";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

const args = process.argv.slice(2);
const opt = (name, def) => (args.includes(name) ? args[args.indexOf(name) + 1] : def);
const RUNS = +opt("--runs", 3), THROTTLE = +opt("--throttle", 4), PORT = +(process.env.PW_PORT ?? 4403);
const MAPS = opt("--maps", "kilnmouth,anvilstead").split(",");
if (!(RUNS >= 1 && THROTTLE >= 1)) throw new Error("--runs and --throttle take a number, at least 1");
const given = args.find((a) => /^https?:/.test(a));
const URL_ = given ?? `http://127.0.0.1:${PORT}/`;
const BUDGET = 1000 / 60;
const q = (xs, p) => xs.length ? [...xs].sort((a, b) => a - b)[Math.min(xs.length - 1, Math.floor(p * xs.length))] : NaN;
const f1 = (x) => x.toFixed(1);

async function measure(browserType, launch) {
  const browser = await browserType.launch(launch);
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
  const cdp = browserType === chromium ? await ctx.newCDPSession(page) : null;
  const busy = async () => (await cdp?.send("Performance.getMetrics"))?.metrics.find((m) => m.name === "TaskDuration").value * 1000;
  const frames = (ms) => page.evaluate((ms) => new Promise((done) => {
    const out = [], end = performance.now() + ms;
    let last = performance.now();
    const frame = (now) => { out.push(now - last); last = now; now > end ? done(out) : requestAnimationFrame(frame); };
    requestAnimationFrame(frame);
  }), ms);
  const res = {};
  for (const map of MAPS) {
    const spawn_ = await page.evaluate(async (m) => (await (await fetch(`world/maps/${m}.json`)).json()), map);
    const at = [Math.floor(spawn_.width / 2), Math.floor(spawn_.height / 2)];
    await host("sixfoldClock", 22 * 60);
    for (let t = 0; (await page.evaluate(() => document.querySelector(".world-host").sixfoldStats().area)) !== map; t++) {
      if (t > 40) throw new Error(`never reached ${map}`);
      if (await skip.isVisible().catch(() => false)) await skip.click().catch(() => {});
      await host("sixfoldWarp", map, spawn_.spawn?.x ?? at[0], spawn_.spawn?.z ?? at[1], "n");
      await page.waitForTimeout(2000);
    }
    await page.waitForTimeout(4000);
    if (cdp) await cdp.send("Performance.enable"), await cdp.send("Emulation.setCPUThrottlingRate", { rate: THROTTLE });
    res[map] = [];
    for (let r = 0; r < RUNS; r++) {
      const b0 = await busy(), d = await frames(8000), b1 = await busy();
      res[map].push({ deltas: d, busy: cdp ? (b1 - b0) / (d.length || 1) : NaN });
    }
    if (cdp) await cdp.send("Emulation.setCPUThrottlingRate", { rate: 1 });
  }
  await browser.close();
  return res;
}

function report(label, res) {
  console.log(`\n${label}`);
  let ok = true;
  for (const [map, runs] of Object.entries(res)) {
    const all = runs.flatMap((r) => r.deltas), busy = runs.map((r) => r.busy);
    const mb = q(busy, 0.5);
    if (!(isNaN(mb) || mb <= BUDGET)) ok = false;
    console.log(`  ${map.padEnd(11)} ${String(all.length).padStart(5)} frames  median ${f1(q(all, 0.5))} ms  p95 ${f1(q(all, 0.95))} ms  >20 ms ${(100 * all.filter((d) => d > 20).length / (all.length || 1)).toFixed(1)}%` +
      (isNaN(mb) ? "" : `  main thread busy ${f1(mb)} ms/frame (budget ${f1(BUDGET)})`));
  }
  return ok;
}

let server = null;
if (!given) {
  server = spawn(process.execPath, [fileURLToPath(new URL("serve.mjs", import.meta.url)), String(PORT)], { stdio: "ignore" });
  for (let i = 0; ; i++) {
    if (await fetch(URL_ + "index.html").then((r) => r.ok, () => false)) break;
    if (i > 50) throw new Error(`serve.mjs did not start on ${PORT} (built web/dist?)`);
    await new Promise((r) => setTimeout(r, 200));
  }
}
let ok = true;
try {
  ok = report(`Chrome, 390 x 844, night 22:00, CPU ${THROTTLE}x slower`, await measure(chromium, { channel: "chrome", headless: false }));
  if (args.includes("--webkit")) ok = report("WebKit, 390 x 844, night, no CPU throttling", await measure(webkit, { headless: false })) && ok;
} finally {
  server?.kill();
}
process.exit(ok ? 0 : 1);
