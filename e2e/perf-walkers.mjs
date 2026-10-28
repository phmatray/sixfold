// Frame time with 6 people walking their routines (#187, AC6 of #113): Brightwell Square at night on the
// phone viewport, the CPU slowed 4×, against the same scene with nobody walking.
//
//   cd web && trunk build --release
//   cd ../e2e && npm ci && node perf-walkers.mjs [url] [--runs 5] [--throttle 4] [--webkit]
//
// Without `url` it serves ../web/dist itself (on PW_PORT, default 4402). It drives the installed Google Chrome
// in a visible window (headless Chrome has no GPU here and draws the world blank). Six extra people with
// placeholder ids are added to the square's map as it loads (nothing shipped changes); each walks across the
// square at 21:00, so with Tansy and Odile eight want to walk and the per-map cap (`walkers_max`, 6) is full.
// A run: the clock set to 20:59.5 (a jump, so everyone is placed at their evening spot), then every frame's
// rAF delta while six walk. The baseline: 21:30, the same people, nobody walking. It prints and never asserts:
// frame times are only worth comparing between runs on the same machine at the same load.
import { chromium, webkit } from "@playwright/test";
import { spawn } from "node:child_process";
import { loadavg } from "node:os";

const args = process.argv.slice(2);
const opt = (name, def) => (args.includes(name) ? args[args.indexOf(name) + 1] : def);
const RUNS = +opt("--runs", 5), THROTTLE = +opt("--throttle", 4), PORT = +(process.env.PW_PORT ?? 4402);
const given = args.find((a) => /^https?:/.test(a));
const URL_ = given ?? `http://127.0.0.1:${PORT}/`;

// Six walkers on the square's lanes: [evening spot, night spot], each walk long enough (20-34 tiles) to
// outlast the others setting off.
const LANES = [[[25, 1], [25, 34]], [[27, 34], [27, 1]], [[6, 10], [45, 10]], [[17, 13], [36, 21]], [[36, 13], [17, 21]], [[4, 30], [26, 30]]];
const MODELS = ["npc_kid", "npc_wanderer", "npc_gardener", "npc_elder"];
const extras = LANES.map(([a, b], i) => ({
  id: `perf_${i + 1}`, name: `Walker ${i + 1}`, model: MODELS[i % MODELS.length], x: a[0], z: a[1], facing: "s",
  dialogue: ["…"], glance: false,
  routine: [{ from: "06:00", at: a, face: "s", do: "idle" }, { from: "21:00", at: b, face: "s", do: "idle" }],
}));

const q = (xs, p) => xs.length ? [...xs].sort((a, b) => a - b)[Math.min(xs.length - 1, Math.floor(p * xs.length))] : NaN;
const f1 = (x) => x.toFixed(1);

async function measure(browserType, launch) {
  const browser = await browserType.launch(launch);
  const ctx = await browser.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 3, isMobile: browserType !== webkit || undefined, hasTouch: true, locale: "en-US", serviceWorkers: "block" });
  const page = await ctx.newPage();
  await page.route("**/maps/square.json", async (route) => {
    const map = await (await route.fetch()).json();
    map.npcs.push(...extras);
    await route.fulfill({ json: map });
  });
  await page.addInitScript(() => sessionStorage.setItem("sixfold.splashed", "1"));
  await page.goto(URL_);
  await page.getByRole("navigation", { name: "Main menu" }).getByRole("button", { name: /^Explore the world/ }).click({ timeout: 120_000 });
  await page.getByRole("button", { name: "Menu", exact: true }).waitFor({ timeout: 120_000 });
  const skip = page.getByRole("button", { name: /^Skip/ });
  for (let i = 0; i < 8 && (await skip.isVisible().catch(() => false)); i++) await skip.click().catch(() => {}), await page.waitForTimeout(1500);

  await page.waitForFunction(() => document.querySelector(".world-host")?.sixfoldWarp, null, { timeout: 120_000 });
  const host = (f, ...a) => page.evaluate(([f, a]) => document.querySelector(".world-host")[f](...a), [f, a]);
  const people = () => page.evaluate(() => document.querySelector(".world-host").sixfoldStats().people);
  await host("sixfoldClock", 21 * 60 + 30);
  // Asked again until it lands: a warp asked during a scene or another warp's fade is ignored.
  for (let t = 0; (await page.evaluate(() => document.querySelector(".world-host").sixfoldStats().area)) !== "square"; t++) {
    if (t > 60) await page.screenshot({ path: "perf-walkers-stuck.png" }), console.error((await page.locator("body").innerText()).slice(0, 500));
    if (t > 60) throw new Error("never reached the square (perf-walkers-stuck.png)");
    if (await skip.isVisible().catch(() => false)) await skip.click().catch(() => {});
    await host("sixfoldWarp", "square", 26, 11, "s");
    await page.waitForTimeout(2000);
  }
  await page.waitForTimeout(3000); // shaders compiled, the warp's fade gone
  // The world must be running (no scene, talk or sheet holding it): its clock moves.
  const minute = () => page.evaluate(() => document.querySelector(".world-host").sixfoldStats().minute);
  for (let t = 0, m = await minute(); ; t++) {
    if (await skip.isVisible().catch(() => false)) await skip.click().catch(() => {});
    await page.waitForTimeout(2500);
    if (m !== (m = await minute())) break;
    if (t > 10) await page.screenshot({ path: "perf-walkers-stuck.png" });
    if (t > 10) throw new Error(`the world's clock stands still (perf-walkers-stuck.png): ${(await page.locator("body").innerText()).slice(0, 300)}`);
  }

  // Chrome: the CPU slowed, and the main thread's busy time (CDP TaskDuration) read round each sample, since a
  // rAF delta only says whether a frame fit the 16.7 ms vsync, not how much room it had left.
  const cdp = browserType === chromium ? await ctx.newCDPSession(page) : null;
  if (cdp) await cdp.send("Performance.enable"), await cdp.send("Emulation.setCPUThrottlingRate", { rate: THROTTLE });
  const busy = async () => (await cdp?.send("Performance.getMetrics"))?.metrics.find((m) => m.name === "TaskDuration").value * 1000;

  // Every frame's rAF delta, tagged with how many people walked when it was drawn (asked 5× a second, so the
  // asking costs nothing measurable). Collected while at least `minWalking` walk, or for `ms` (wall time).
  const frames = (minWalking, ms) => page.evaluate(([minWalking, ms]) => new Promise((done) => {
    const el = document.querySelector(".world-host"), out = [];
    let walking = 0, seen = false, last = performance.now();
    const poll = setInterval(() => (walking = el.sixfoldStats().people.filter((p) => p.walking).length), 200);
    const end = performance.now() + ms;
    const frame = (now) => {
      const ok = walking >= minWalking;
      if (ok) (seen = true), out.push(now - last);
      last = now;
      if (now > end || (seen && !ok && minWalking > 0)) return clearInterval(poll), done(out);
      requestAnimationFrame(frame);
    };
    requestAnimationFrame(frame);
  }), [minWalking, ms]);
  const sample = async (minWalking, ms) => {
    const b0 = await busy(), deltas = await frames(minWalking, ms), b1 = await busy();
    return { deltas, busy: cdp ? (b1 - b0) / (deltas.length || 1) : NaN };
  };

  const runs = { walking: [], still: [] };
  for (let r = 0; r < RUNS; r++) {
    await host("sixfoldClock", 21 * 60 + 30); // placed at the night spots, nobody walking
    await page.waitForTimeout(1500);
    const still = await sample(0, 4000);
    await host("sixfoldClock", 20 * 60 + 59.5); // a jump: placed at the evening spots; at 21:00 they set off
    const walking = await sample(6, 90_000);
    runs.still.push(still);
    runs.walking.push(walking);
    const most = (await people()).filter((p) => p.id.startsWith("perf_")).map((p) => `${p.id}@${p.x},${p.z}`).join(" ");
    const line = (s) => `${s.deltas.length} frames, median ${f1(q(s.deltas, 0.5))} ms` + (cdp ? `, busy ${f1(s.busy)} ms/frame` : "");
    console.log(`run ${r + 1}: 6 walking ${line(walking)} · nobody walking ${line(still)} · ${most}`);
  }
  await browser.close();
  return runs;
}

function report(label, runs) {
  console.log(`\n${label}`);
  const span = (xs) => `${f1(Math.min(...xs))}–${f1(Math.max(...xs))}`;
  for (const [name, list] of [["6 walking", runs.walking], ["nobody walking", runs.still]]) {
    const all = list.flatMap((r) => r.deltas), med = list.map((r) => q(r.deltas, 0.5)), p95 = list.map((r) => q(r.deltas, 0.95)), busy = list.map((r) => r.busy);
    const slow = all.filter((d) => d > 20).length / (all.length || 1);
    console.log(`  ${name.padEnd(15)} ${String(all.length).padStart(5)} frames  median ${f1(q(all, 0.5))} ms  p95 ${f1(q(all, 0.95))} ms  >20 ms ${(100 * slow).toFixed(1)}%` +
      (isNaN(busy[0]) ? "" : `  main thread busy ${f1(q(busy, 0.5))} ms/frame`) +
      `  (per run: median ${span(med)}, p95 ${span(p95)}${isNaN(busy[0]) ? "" : `, busy ${span(busy)}`})`);
  }
  return runs.walking.every((r) => r.deltas.length > 0);
}

let server = null;
if (!given) {
  server = spawn(process.execPath, [new URL("serve.mjs", import.meta.url).pathname, String(PORT)], { stdio: "ignore" });
  for (let i = 0; ; i++) {
    if (await fetch(URL_ + "index.html").then((r) => r.ok, () => false)) break;
    if (i > 50) throw new Error(`serve.mjs did not start on ${PORT} (built web/dist?)`);
    await new Promise((r) => setTimeout(r, 200));
  }
}
let ok = true;
try {
  console.log(`load average ${loadavg().map((l) => l.toFixed(0)).join(" ")} · ${RUNS} runs · ${URL_}`);
  ok = report(`Chrome, 390 × 844, CPU ${THROTTLE}× slower`, await measure(chromium, { channel: "chrome", headless: false }));
  if (args.includes("--webkit")) ok = report("WebKit, 390 × 844, no CPU throttling (WebKit has none)", await measure(webkit, { headless: false })) && ok;
} finally {
  server?.kill();
}
if (!ok) {
  console.error("\nsix people were never seen walking at once: the scene did not set up (check the lanes against the map)");
  process.exit(1);
}
