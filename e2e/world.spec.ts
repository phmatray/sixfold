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
  // The act prompt shows only when a press would act (world.js): then E talks. (A key press,
  // not a click: a click's own 15 s wait would run into the hold below.)
  const prompt = page.getByRole("button", { name: /^Talk to Tobin/ });
  await expect(prompt).toBeVisible();
  await page.keyboard.press("e");
  // The map's first talk draws the speaker's portrait in a new offscreen WebGL context, at once:
  // on CI's software GL that holds the page for ~25 s (measured), as loading the world does.
  const talk = page.locator(".talk-box");
  await expect(talk).toBeVisible({ timeout: 60_000 });
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

test("on a phone, a tap on the ground walks only once Options allow it @phone", async ({ page }, info) => {
  test.skip(info.project.name !== "phone", "taps are a touch screen's");
  await title(page);
  await menuItem(page, "Explore the world").click();
  await loaded(page);
  const skip = page.getByRole("button", { name: /^Skip/ });
  await expect(skip).toBeVisible();
  await skip.click();
  await expect(skip).toBeHidden();
  const host = page.locator(".world-host");
  await expect(host).toHaveAttribute("data-hero", /\d/);
  const canvas = host.locator("canvas");
  const box = (await canvas.boundingBox())!;
  // The path a few tiles below the hero (the camera keeps the hero near the middle).
  const ground = { x: box.width / 2, y: box.height * 0.72 };

  // By default the on-screen arrows walk; a tap on the scene does nothing.
  const start = await host.getAttribute("data-hero");
  await canvas.tap({ position: ground });
  await page.waitForTimeout(1500);
  expect(await host.getAttribute("data-hero")).toEqual(start);

  // Options: tap to walk, kept in this browser.
  await page.getByRole("button", { name: "Menu", exact: true }).click();
  await page.getByRole("tab", { name: "Options" }).click();
  const option = page.getByRole("checkbox", { name: /Tap the ground to walk/ });
  await expect(option).not.toBeChecked();
  await option.check();
  expect(await page.evaluate(() => localStorage.getItem("sixfold.tap-walk"))).toBe("1");
  await page.getByRole("button", { name: "Close", exact: true }).click();

  await canvas.tap({ position: ground });
  await expect.poll(() => host.getAttribute("data-hero"), { timeout: 10_000 }).not.toEqual(start);
});
