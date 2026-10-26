import { expect, test, type Page } from "@playwright/test";
import { frames, menuItem, readSave, title } from "./helpers";

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
  const logs: string[] = [];
  page.on("console", (m) => m.type() !== "debug" && logs.push(`${m.type()}: ${m.text()}`.slice(0, 200)));
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
  // The act prompt (a key tap can fall between two frames of a slow headless scene), once the
  // scene has drawn the warp (its first frames compile shaders: slow on CI's software GL).
  const prompt = page.getByRole("button", { name: /^Talk to Tobin/ });
  await expect(prompt).toBeVisible();
  await frames(page);
  await prompt.click();
  const talk = page.locator(".talk-box");
  await expect(talk).toBeVisible({ timeout: 15_000 }).catch(async (e) => {
    // What the world was doing (reported in the failure, for a CI-only race).
    const state = await page.evaluate(() => {
      const host = document.querySelector(".world-host") as unknown as { sixfoldStats?: () => unknown; dataset: DOMStringMap };
      return {
        stats: host?.sixfoldStats?.(),
        hero: host?.dataset.hero,
        prompt: [...document.querySelectorAll(".world-prompt")].map((p) => `${p.textContent} visible=${(p as HTMLElement).checkVisibility()}`),
        dialogs: [...document.querySelectorAll("dialog[open], [role=dialog]")].map((d) => `${d.className}: ${(d.textContent ?? "").slice(0, 120)}`),
        statuses: [...document.querySelectorAll("[role=status], [aria-live]")].map((d) => (d.textContent ?? "").slice(0, 80)).filter(Boolean),
      };
    });
    throw new Error(`${e.message}\nworld: ${JSON.stringify(state)}\nconsole: ${logs.join(" | ")}`);
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
