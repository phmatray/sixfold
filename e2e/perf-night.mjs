// Phone night frame time on the Hub and Grove maps (#259, AC4 of the Hub/Grove job): 390 × 844, the CPU slowed 4×,
// at 22:00, on every map of the two regions, from the spot a player stands longest in.
//
//   cd web && trunk build --release
//   cd ../e2e && npm ci && node perf-night.mjs [url] [--runs 3] [--throttle 4] [--ms 6000] [--graphics off|low|high]
//
// Without `url` it serves ../web/dist itself (on PW_PORT, default 4403). Like perf-walkers.mjs it drives the
// installed Google Chrome in a visible window (headless has no GPU here and draws the world blank), and reads the
// main thread's busy time (CDP TaskDuration) round each sample as well as the rAF deltas. It prints a table and
// exits 1 when a map's median frame is over 16.7 ms (the budget) or its main thread is busy more than 16.7 ms a frame.
import { chromium } from "@playwright/test";
import { spawn } from "node:child_process";
import { loadavg } from "node:os";
import { fileURLToPath } from "node:url";

const args = process.argv.slice(2);
const opt = (name, def) => (args.includes(name) ? args[args.indexOf(name) + 1] : def);
const RUNS = +opt("--runs", 3), THROTTLE = +opt("--throttle", 4), MS = +opt("--ms", 6000), PORT = +(process.env.PW_PORT ?? 4403);
if (!(RUNS >= 1 && THROTTLE >= 1 && MS >= 1000)) throw new Error("--runs, --throttle and --ms take a number (at least 1, 1, 1000)");
const GRAPHICS = opt("--graphics", null);
if (GRAPHICS !== null && !["off", "low", "high"].includes(GRAPHICS)) throw new Error("--graphics takes off, low or high");
const given = args.find((a) => /^https?:/.test(a));
const URL_ = given ?? `http://127.0.0.1:${PORT}/`;
const BUDGET = 1000 / 60;
const NIGHT = 22 * 60;

// Where the hero stands: the middle of each map's busiest stretch (its landmark or its square), facing the way in.
const SPOTS = [
  ["overworld", 25, 20, "n"], // the meadow's crossroads
  ["square", 26, 11, "s"], // the fountain
  ["rootholm", 28, 16, "n"], // the Moot, under the standing tree
  ["elder_ring", 22, 24, "n"], // the ring road's crossroads
  ["woodland", 57, 12, "w"], // the reference map (Fernhollow Trail), measured under the same load for comparison
];

const q = (xs, p) => xs.length ? [...xs].sort((a, b) => a - b)[Math.min(xs.length - 1, Math.floor(p * xs.length))] : NaN;
const f1 = (x) => x.toFixed(1);

async function measure() {
  const browser = await chromium.launch({ channel: "chrome", headless: false });
  const ctx = await browser.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 3, isMobile: true, hasTouch: true, locale: "en-US", serviceWorkers: "block" });
  const page = await ctx.newPage();
  await page.addInitScript((g) => {
    sessionStorage.setItem("sixfold.splashed", "1");
    if (g) localStorage.setItem("sixfold.graphics", g);
  }, GRAPHICS);
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
    const frame = (now) => {
      out.push(now - last);
      last = now;
      if (now > end) return done(out);
      requestAnimationFrame(frame);
    };
    requestAnimationFrame(frame);
  }), ms);

  const rows = [];
  for (const [map, x, z, face] of SPOTS) {
    // Asked again until it lands: a warp asked during a scene or another warp's fade is ignored.
    for (let t = 0; (await host("sixfoldStats")).area !== map; t++) {
      if (t > 60) throw new Error(`never reached ${map}`);
      if (await skip.isVisible().catch(() => false)) await skip.click().catch(() => {});
      await host("sixfoldWarp", map, x, z, face);
      await page.waitForTimeout(2000);
    }
    await host("sixfoldClock", NIGHT);
    await page.waitForTimeout(3000); // shaders compiled, the warp's fade gone
    await cdp.send("Emulation.setCPUThrottlingRate", { rate: THROTTLE });
    const all = [], busies = [];
    for (let r = 0; r < RUNS; r++) {
      const b0 = await busy(), deltas = await frames(MS), b1 = await busy();
      all.push(...deltas);
      busies.push((b1 - b0) / (deltas.length || 1));
    }
    await cdp.send("Emulation.setCPUThrottlingRate", { rate: 1 });
    const tier = (await host("sixfoldStats")).graphics;
    rows.push({ map, tier, n: all.length, median: q(all, 0.5), p95: q(all, 0.95), slow: all.filter((d) => d > 20).length / (all.length || 1), busy: q(busies, 0.5) });
  }
  await browser.close();
  return rows;
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
let over = false;
try {
  console.log(`load average ${loadavg().map((l) => l.toFixed(0)).join(" ")} · ${RUNS} × ${MS} ms per map · ${URL_}`);
  console.log(`Chrome, 390 × 844, 22:00, CPU ${THROTTLE}× slower, budget ${f1(BUDGET)} ms\n`);
  for (const r of await measure()) {
    const bad = r.map !== "woodland" && (r.median > BUDGET + 0.5 || r.busy > BUDGET);
    over ||= bad;
    console.log(`${r.map.padEnd(11)} graphics ${r.tier.padEnd(5)} ${String(r.n).padStart(5)} frames  median ${f1(r.median)} ms  p95 ${f1(r.p95)} ms  >20 ms ${(100 * r.slow).toFixed(1)}%  main thread busy ${f1(r.busy)} ms/frame  ${bad ? "OVER" : "ok"}`);
  }
} finally {
  server?.kill();
}
process.exitCode = over ? 1 : 0;
