import { expect, test, type Page } from "@playwright/test";
import { menuItem, readSave, title } from "./helpers";

/** Where the save says the hero stands: `map,x,z,facing`. */
const at = async (page: Page) => (await readSave(page)).world_at?.[0] ?? "";

/** Hold an arrow key until the saved position matches `want`. */
async function walk(page: Page, key: string, want: RegExp) {
  await page.keyboard.down(key);
  await expect.poll(() => at(page), { timeout: 30_000 }).toMatch(want);
  await page.keyboard.up(key);
}

/** The scene is up: its loading line is gone. */
async function loaded(page: Page) {
  await expect(page.getByRole("status").filter({ hasText: "Loading the world" })).toBeVisible();
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

  // Escape opens the in-game menu, and closes it.
  await page.keyboard.press("Escape");
  const menu = page.getByRole("dialog", { name: "Menu" });
  await expect(menu).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(menu).toBeHidden();

  // Reload: Continue puts the hero back on the saved tile (the scene would
  // save its spawn tile on its first frame otherwise), with no intro again.
  const saved = await at(page);
  await page.reload();
  await menuItem(page, "Continue").click();
  await loaded(page);
  // The scene marks the hero's tile on its host once a frame has drawn it.
  await expect(page.locator("[data-hero]")).toHaveAttribute("data-hero", saved.split(",").slice(1).join(","));
  expect(await at(page)).toEqual(saved);
  await expect(skip).toBeHidden();
});
