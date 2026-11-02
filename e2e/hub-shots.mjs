// Screenshots of the Hub maps (overworld, square) by day and by night, on a desktop and a phone (#257).
//
//   cd e2e && npm ci && node serve.mjs 4410 &   node hub-shots.mjs http://127.0.0.1:4410/ [out-dir]
//
// Warps the hero (`el.sixfoldWarp`) and sets the hour (`el.sixfoldClock`), so the shots are repeatable.
// `out-dir` defaults to e2e/shots/ (git-ignored). Like readme-shots.mjs it drives the installed Chrome in a
// visible window: headless draws the world blank.
import { chromium } from "@playwright/test";
import { mkdirSync } from "node:fs";

const URL_ = process.argv[2] ?? "http://127.0.0.1:4410/";
const OUT = process.argv[3] ?? new URL("./shots/", import.meta.url).pathname;
mkdirSync(OUT, { recursive: true });

const DEVICES = {
  desktop: { viewport: { width: 1440, height: 900 } },
  phone: { viewport: { width: 390, height: 844 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true },
};
const SPOTS = [
  { map: "overworld", at: [25, 15], facing: "n" }, // the meadow's heart, the windmill's road
  { map: "overworld", at: [38, 6], facing: "n" }, // under the windmill
  { map: "square", at: [26, 12], facing: "s" }, // the plaza and its fountain
  { map: "square", at: [26, 28], facing: "s" }, // Mill Lane, toward the willows
];
const HOURS = { day: 12 * 60, night: 22 * 60 };
const HIDE = ".world-top,.world-prompt,.world-controls,.world-legend,.world-pad,.world-run,.scene-caption,.coach-hint,.debug-panel{visibility:hidden!important}";

const browser = await chromium.launch({ channel: "chrome", headless: false });
let failed = 0;
for (const [device, opts] of Object.entries(DEVICES)) {
  const ctx = await browser.newContext({ ...opts, locale: "en-US", serviceWorkers: "block" });
  const page = await ctx.newPage();
  await page.addInitScript(() => sessionStorage.setItem("sixfold.splashed", "1"));
  await page.goto(URL_);
  await page.getByRole("navigation", { name: "Main menu" }).getByRole("button", { name: /^Explore the world/ }).click({ timeout: 120_000 });
  await page.getByRole("button", { name: "Menu", exact: true }).waitFor({ timeout: 120_000 });
  const skip = page.getByRole("button", { name: /^Skip/ });
  for (let i = 0; i < 8 && (await skip.isVisible().catch(() => false)); i++) await skip.click().catch(() => {}), await page.waitForTimeout(1500);
  await page.waitForFunction(() => document.querySelector(".world-host")?.sixfoldWarp, null, { timeout: 120_000 });
  await page.addStyleTag({ content: HIDE });
  const host = (f, ...a) => page.evaluate(([f, a]) => document.querySelector(".world-host")[f](...a), [f, a]);
  for (const [i, s] of SPOTS.entries()) {
    for (const [hour, minute] of Object.entries(HOURS)) {
      try {
        // Asked again until it lands: a warp asked during a scene or another warp's fade is ignored.
        for (let t = 0; (await host("sixfoldStats")).area !== s.map; t++) {
          if (t > 40) throw new Error("never reached " + s.map);
          if (await skip.isVisible().catch(() => false)) await skip.click().catch(() => {});
          await host("sixfoldWarp", s.map, s.at[0], s.at[1], s.facing);
          await page.waitForTimeout(1500);
        }
        await host("sixfoldWarp", s.map, s.at[0], s.at[1], s.facing);
        await host("sixfoldClock", minute);
        for (let t = 0; t < 6 && (await skip.isVisible().catch(() => false)); t++) await skip.click().catch(() => {}), await page.waitForTimeout(1200); // the intro scene
        await host("sixfoldClock", minute);
        await page.waitForTimeout(3000); // shaders compiled, the fade gone
        await page.screenshot({ path: `${OUT}${s.map}-${i}-${hour}-${device}.jpg`, type: "jpeg", quality: 80 });
        console.log("ok  ", s.map, i, hour, device);
      } catch (e) {
        failed++;
        console.log("FAIL", s.map, i, hour, device, e.message.split("\n")[0]);
      }
    }
  }
  await ctx.close();
}
await browser.close();
process.exitCode = failed ? 1 : 0;
