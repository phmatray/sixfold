// Short clips of each region's ambient life (#190: critters, perching birds, leaves, embers, Gullhaven's
// docking boat, the seal site's calm), by day, UI hidden, for AC7 of #115/#182 and as trailer footage.
//
//   cd e2e && npm ci && node life-clips.mjs [url] [clip-name ...]
//
// `url` defaults to the live site; pass http://127.0.0.1:4409/ after `node serve.mjs 4409` for a local release
// build. Writes e2e/clips/<name>.mp4 (git-ignored): 1920x1080, H.264, 60 fps, ~8 s, needs ffmpeg on PATH.
// Like readme-shots.mjs it drives the installed Google Chrome in a visible window (headless draws the world
// blank), with motion on. It records a CDP screencast, not Playwright's recordVideo (fixed 25 fps, VP8): the
// frames carry timestamps, so ffmpeg rebuilds a 60 fps clip from whatever the machine really drew. On a
// loaded machine that is fewer real frames; the script prints the achieved fps per clip.
import { chromium } from "@playwright/test";
import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const args = process.argv.slice(2);
const URL_ = args.find((a) => a.startsWith("http")) ?? "https://phmatray.github.io/sixfold/";
const only = args.filter((a) => !a.startsWith("http"));
const OUT = new URL("./clips/", import.meta.url).pathname;
const SECONDS = 8;
mkdirSync(OUT, { recursive: true });

// name, map, hero tile, what must be on screen before the clip starts (read from el.sixfoldLife()).
const CLIPS = [
  { name: "fernhollow-rabbits-leaves", map: "woodland", at: [48, 13], need: (l) => l.critters.rabbit > 0 && l.leaves > 0 },
  { name: "thrumming-downs-birds", map: "downs", at: [7, 14], need: (l) => l.critters.bird > 0 },
  { name: "gullhaven-boat-docking", map: "gullhaven", at: [19, 16], boat: true, need: (l) => l.boats.length === 1 },
  { name: "gullhaven-crabs", map: "gullhaven", at: [5, 24], need: (l) => l.critters.crab > 0 },
  { name: "cinderwick-lizards-embers", map: "cinderwick", at: [39, 31], need: (l) => l.critters.lizard > 0 },
  { name: "kilnmouth-lizards-embers", map: "kilnmouth", at: [26, 18], need: (l) => l.critters.lizard > 0 },
  { name: "saltwind-crabs-ducks", map: "seaside", at: [28, 17], need: (l) => l.critters.crab > 0 || l.critters.duck > 0 },
  { name: "scree-pass-goats", map: "screepass", at: [17, 17], need: (l) => l.critters.goat > 0 },
  { name: "rootholm-squirrels-leaves", map: "rootholm", at: [18, 6], need: (l) => l.critters.squirrel > 0 },
  { name: "elder-ring-calm", map: "elder_ring", at: [20, 17], need: (l) => l.calm === true },
].filter((c) => !only.length || only.includes(c.name));

// A release build has no ?debug clean-capture mode (debug.js), so the HUD is hidden by CSS.
const HIDE = ".world-top,.world-prompt,.world-controls,.world-legend,.world-pad,.world-run,.scene-caption,.coach-hint,.debug-panel{visibility:hidden!important}*{cursor:none!important}";

const browser = await chromium.launch({ channel: "chrome", headless: false, args: ["--hide-scrollbars"] });
const ctx = await browser.newContext({ viewport: { width: 1920, height: 1080 }, deviceScaleFactor: 1, reducedMotion: "no-preference", locale: "en-US", serviceWorkers: "block" });
const page = await ctx.newPage();
page.on("dialog", (d) => d.accept());
await page.addInitScript(() => sessionStorage.setItem("sixfold.splashed", "1"));
await page.goto(URL_);
await page.getByRole("navigation", { name: "Main menu" }).waitFor({ timeout: 60_000 });
await page.getByRole("navigation", { name: "Main menu" }).getByRole("button", { name: /^Explore the world/ }).click();
await page.getByRole("button", { name: "Menu", exact: true }).waitFor({ timeout: 60_000 });
await page.waitForTimeout(1000);
for (let i = 0; i < 5; i++) {
  const skip = page.getByRole("button", { name: /^Skip/ });
  if (!(await skip.isVisible().catch(() => false))) break;
  await page.waitForTimeout(450); // a scene ignores a skip in its first 400 ms
  await page.keyboard.press("Escape");
  await page.waitForTimeout(1500);
}
await page.addStyleTag({ content: HIDE });

const host = (call, ...a) => page.evaluate(([c, a]) => document.querySelector(".world-host")[c](...a), [call, a]);
const life = () => host("sixfoldLife");
const until = async (f, ms, what) => {
  const end = Date.now() + ms;
  while (Date.now() < end) {
    const v = await f();
    if (v) return v;
    await page.waitForTimeout(250);
  }
  throw new Error("timed out: " + what);
};

async function record(c) {
  const cdp = await ctx.newCDPSession(page);
  const frames = [];
  cdp.on("Page.screencastFrame", (f) => {
    frames.push({ t: f.metadata.timestamp, buf: Buffer.from(f.data, "base64") });
    cdp.send("Page.screencastFrameAck", { sessionId: f.sessionId }).catch(() => {});
  });
  await cdp.send("Page.startScreencast", { format: "jpeg", quality: 92, maxWidth: 1920, maxHeight: 1080, everyNthFrame: 1 });
  await page.waitForTimeout(SECONDS * 1000);
  await cdp.send("Page.stopScreencast");
  await cdp.detach();
  if (frames.length < 2) throw new Error("the screencast drew no frames");
  const dir = mkdtempSync(join(tmpdir(), "life-clip-"));
  const list = frames.map((f, i) => {
    writeFileSync(join(dir, i + ".jpg"), f.buf);
    return `file '${i}.jpg'\nduration ${((frames[i + 1]?.t ?? f.t + 1 / 60) - f.t).toFixed(5)}`;
  });
  list.push(`file '${frames.length - 1}.jpg'`); // concat ignores the last duration without a repeat
  writeFileSync(join(dir, "list.txt"), list.join("\n"));
  const out = OUT + c.name + ".mp4";
  execFileSync("ffmpeg", ["-y", "-loglevel", "error", "-f", "concat", "-safe", "0", "-i", join(dir, "list.txt"), "-vf", "fps=60,format=yuv420p", "-c:v", "libx264", "-crf", "17", "-preset", "medium", "-r", "60", "-movflags", "+faststart", out]);
  rmSync(dir, { recursive: true, force: true });
  const span = frames.at(-1).t - frames[0].t;
  return `${frames.length} real frames over ${span.toFixed(1)} s = ${(frames.length / span).toFixed(0)} fps drawn`;
}

for (const c of CLIPS) {
  try {
    await host("sixfoldWarp", c.map, c.at[0], c.at[1], "s");
    await until(async () => (await host("sixfoldStats")).area === c.map, 60_000, "warp to " + c.map);
    await host("sixfoldClock", 720); // midday
    await until(async () => { const l = await life(); return l && c.need(l); }, 60_000, "its life");
    if (c.boat) await until(async () => { const b = (await life()).boats[0]; return b.on && b.k > 0 && b.k < 0.3; }, 150_000, "the boat sailing in"); // ~8 s later it ties up
    else await page.waitForTimeout(2500); // shaders compiled, the first frames settled
    console.log("ok  ", c.name, await record(c));
  } catch (e) {
    console.log("FAIL", c.name, e.message.split("\n")[0]);
    process.exitCode = 1;
  }
}
await browser.close();
