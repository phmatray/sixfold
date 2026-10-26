import { expect, test, type Page } from "@playwright/test";
import { menuItem, readSave, title } from "./helpers";

/** Where the save says the hero stands: `map,x,z,facing`. */
const at = async (page: Page) => (await readSave(page)).world_at?.[0] ?? "";

/** Hold an arrow key until the saved position matches `want`. */
async function walk(page: Page, key: string, want: RegExp) {
  await page.keyboard.down(key);
  try {
    // Polled often: the hero walks 4 tiles a second.
    await expect.poll(() => at(page), { timeout: 30_000, intervals: [100] }).toMatch(want);
  } finally {
    await page.keyboard.up(key);
  }
}

/** The world screen is up and its scene has loaded (the loading line is gone). */
async function loaded(page: Page) {
  await expect(page.getByRole("button", { name: "Menu", exact: true })).toBeVisible();
  await expect(page.getByRole("status").filter({ hasText: "Loading the world" })).toBeHidden({ timeout: 60_000 });
}

test("the world: walk, take a door and back, open the menu, and Continue resumes there", async ({ page }) => {
  page.on("dialog", (d) => d.accept());
  await title(page);
  // Past the lesson, the title offers Continue.
  await menuItem(page, "Learn to play").click();
  await page.getByRole("button", { name: "Skip the lesson" }).click();
  await menuItem(page, "Continue").click();

  // A new game opens on the meadow's intro scene: Escape skips it.
  await loaded(page);
  const skip = page.getByRole("button", { name: /^Skip/ });
  await expect(skip).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(skip).toBeHidden();

  // Up the path and through the shop's door, then back out.
  await expect.poll(() => at(page)).toMatch(/^overworld,/);
  const start = await at(page);
  await walk(page, "ArrowUp", /^shop,/);
  await walk(page, "ArrowDown", /^overworld,/);
  await walk(page, "ArrowDown", /^overworld,25,1[4-9],/);
  expect(await at(page)).not.toEqual(start);
  // The controls hint has done its job once the hero has walked: it fades for good.
  await expect(page.locator(".world-hint")).toHaveClass(/gone/);
  expect(await page.evaluate(() => localStorage.getItem("sixfold.hint-seen"))).toBe("1");

  // Escape opens the in-game menu, and closes it.
  await page.keyboard.press("Escape");
  const menu = page.getByRole("dialog", { name: "Menu" });
  await expect(menu).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(menu).toBeHidden();

  // Reload: Continue puts the hero back on the saved tile (the scene would
  // save its spawn tile on its first frame otherwise). The scene marks the
  // hero's tile on its host only outside a cutscene, so this also says the
  // intro didn't play again.
  const saved = await at(page);
  await page.reload();
  await menuItem(page, "Continue").click();
  await loaded(page);
  await expect(page.locator("[data-hero]")).toHaveAttribute("data-hero", saved.split(",").slice(1).join(","));
  expect(await at(page)).toEqual(saved);
});

test("a duelist's challenge stays in the world: their words, the terms, arrows and Escape", async ({ page }) => {
  await title(page);
  await menuItem(page, "Explore the world").click();
  await loaded(page);
  // The intro scene: wait for it to be playing before skipping it. Escape pressed before it starts
  // skips nothing, and the scene then plays over the warp below (moving the hero, holding input).
  const skip = page.getByRole("button", { name: /^Skip/ });
  await expect(skip).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(skip).toBeHidden();
  // Face Tobin from the tile south of him (out of his line of sight, so he
  // doesn't come over on his own) and talk to him: he challenges.
  await page.locator(".world-host").evaluate((el) => (el as unknown as { sixfoldWarp: (...a: unknown[]) => void }).sixfoldWarp("overworld", 23, 28, "n"));
  await expect(page.locator("[data-hero]")).toHaveAttribute("data-hero", /^23,28/, { timeout: 15_000 });
  // The act prompt shows only when a press would act (world.js): then E talks. (A click on it
  // could land on the scene behind while the prompt still settled after the warp, and a tap
  // there walks the hero instead: the CI retries.)
  const prompt = page.getByRole("button", { name: /^Talk to Tobin/ });
  await expect(prompt).toBeVisible();
  // ponytail: a timeline of the talk opening, reported if it doesn't (a CI-only race under study).
  await page.evaluate(() => {
    const w = window as unknown as { tl: string[] };
    w.tl = [];
    const t0 = performance.now();
    const at = (s: string) => w.tl.push(`${Math.round(performance.now() - t0)} ${s}`);
    addEventListener("keydown", (e) => at(`keydown ${e.key}`), true);
    let had = false;
    new MutationObserver(() => {
      const has = !!document.querySelector(".talk-box");
      if (has !== had) at(has ? "talk-box +" : "talk-box -"), (had = has);
    }).observe(document.body, { subtree: true, childList: true });
    for (const d of document.querySelectorAll("dialog")) for (const ev of ["close", "cancel"]) d.addEventListener(ev, () => at(`${ev} ${d.className}`));
    let frames = 0;
    const tick = () => (frames++, requestAnimationFrame(tick));
    tick();
    setInterval(() => at(`frames ${frames} hero ${(document.querySelector(".world-host") as HTMLElement)?.dataset.hero} prompt ${document.querySelector(".world-prompt")?.textContent ?? "-"}`), 1000);
  });
  await page.keyboard.press("e");
  const talk = page.locator(".talk-box");
  await expect(talk).toBeVisible().catch(async (e) => {
    throw new Error(`${e.message}\ntimeline: ${(await page.evaluate(() => (window as unknown as { tl: string[] }).tl)).join(" | ")}`);
  });
  while (await talk.isVisible()) {
    await page.keyboard.press("Enter");
    await page.waitForTimeout(400);
  }

  const offer = page.getByRole("dialog", { name: "Challenge Tobin?" });
  await expect(offer).toBeVisible();
  await expect(offer).toContainText("Duel me!"); // the line he just said
  await expect(offer).toContainText("Laid-back");
  await expect(offer).not.toContainText(/\bAI\b/);
  const go = offer.getByRole("button", { name: /^Challenge/ });
  await expect(go).toBeFocused();
  // Arrows move between the answers; Escape is Not now.
  await page.keyboard.press("ArrowLeft");
  await expect(offer.getByRole("button", { name: /^Not now/ })).toBeFocused();
  await page.keyboard.press("ArrowRight");
  await expect(go).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(offer).toBeHidden();
});
