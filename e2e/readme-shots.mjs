// The README's screenshots (docs/screenshots/*.jpg), taken from a running build.
//
//   cd e2e && npm ci && node readme-shots.mjs [url]
//
// `url` defaults to the live site; pass http://127.0.0.1:4173/ after `node serve.mjs 4173` to shoot a
// local release build. It drives the installed Google Chrome in a visible window: headless Chrome has
// no GPU here and draws the three.js world blank. Each scene starts from a new player (fresh context).
import { chromium } from "@playwright/test";
import { mkdirSync } from "node:fs";

const URL_ = process.argv[2] ?? "https://phmatray.github.io/sixfold/";
const OUT = new URL("../docs/screenshots/", import.meta.url).pathname;
mkdirSync(OUT, { recursive: true });

const browser = await chromium.launch({ channel: "chrome", headless: false });
const DESKTOP = { viewport: { width: 1440, height: 900 } };
const PHONE = { viewport: { width: 390, height: 844 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true };

async function open(device) {
  const ctx = await browser.newContext({ ...device, locale: "en-US", serviceWorkers: "block" });
  const page = await ctx.newPage();
  await page.addInitScript(() => sessionStorage.setItem("sixfold.splashed", "1"));
  await page.goto(URL_);
  await page.getByRole("navigation", { name: "Main menu" }).waitFor({ timeout: 60_000 });
  await page.waitForTimeout(1500);
  return page;
}
const menu = (page, label) =>
  page.getByRole("navigation", { name: "Main menu" }).getByRole("button", { name: new RegExp(`^${label}`) });
const shot = (page, name) => page.screenshot({ path: OUT + name + ".jpg", type: "jpeg", quality: 82 });

// A duel past setup: one Basic Active, one on the Bench, an energy attached on the first turn.
async function duel(page, wait) {
  await menu(page, "Play a duel").click();
  await page.getByRole("button", { name: "Start the duel" }).click();
  await page.waitForTimeout(2500);
  const hand = page.getByRole("list", { name: /^Your hand/ });
  const basics = hand.getByRole("button", { name: /Basic, playable$/ });
  await basics.first().click();
  await page.getByRole("button", { name: /^Empty Active spot/ }).click();
  await page.getByRole("button", { name: /^Done/ }).click();
  await page.waitForTimeout(wait); // the AI's first turn
  const energy = hand.getByRole("button", { name: /Basic energy, playable$/ });
  if (await energy.count()) {
    await energy.first().click();
    await page.getByRole("button", { name: /Your Active/ }).first().click().catch(() => {});
  }
  await page.keyboard.press("Escape");
  await page.mouse.move(1000, 400);
  await page.waitForTimeout(1200);
}

const scenes = {
  async title() {
    const page = await open(DESKTOP);
    await page.waitForTimeout(7000); // the world behind the menu
    await shot(page, "title");
  },
  async world() {
    const page = await open(DESKTOP);
    await menu(page, "Explore the world").click();
    await page.getByRole("button", { name: "Menu", exact: true }).waitFor({ timeout: 60_000 });
    await page.waitForTimeout(3000);
    const skip = page.getByRole("button", { name: /^Skip scene/ });
    for (let i = 0; i < 5 && (await skip.isVisible().catch(() => false)); i++) {
      await skip.click();
      await page.waitForTimeout(1500);
    }
    await page.waitForTimeout(3000);
    await shot(page, "world");
  },
  async duel() {
    const page = await open(DESKTOP);
    await duel(page, 12_000);
    await shot(page, "duel");
  },
  async packs() {
    const page = await open(DESKTOP);
    await menu(page, "Open packs").click();
    await page.getByRole("button", { name: /^Open one .*, 1 left$/ }).first().click();
    await page.getByRole("button", { name: "Tear it open" }).click();
    await page.getByRole("button", { name: "See the collection" }).waitFor({ timeout: 30_000 });
    await page.waitForTimeout(6000); // every card turned
    await shot(page, "packs");
  },
  async collection() {
    const page = await open(DESKTOP);
    await menu(page, "Collection").click();
    await page.waitForTimeout(2500);
    await shot(page, "collection");
  },
  async decks() {
    const page = await open(DESKTOP);
    await menu(page, "My decks").click();
    await page.waitForTimeout(2000);
    await shot(page, "decks");
  },
  async phone() {
    const page = await open(PHONE);
    await duel(page, 25_000);
    await shot(page, "phone");
  },
};

for (const [name, run] of Object.entries(scenes)) {
  try {
    await run();
    console.log("ok  ", name);
  } catch (e) {
    console.log("FAIL", name, e.message.split("\n")[0]);
    process.exitCode = 1;
  }
}
await browser.close();
