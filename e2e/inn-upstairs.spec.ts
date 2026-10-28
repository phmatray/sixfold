import { expect, test, type Page } from "@playwright/test";
import { loaded, menuItem, readSave, title } from "./helpers";

/** Where the save says the hero stands: `map,x,z,facing`. */
const at = async (page: Page) => (await readSave(page)).world_at?.[0] ?? "";
// Warped, not walked: CI draws about a frame a second.
const warp = (page: Page, map: string, x: number, z: number, f: string) =>
  page.evaluate(([map, x, z, f]) => (document.querySelector(".world-host") as any).sixfoldWarp(map, x, z, f), [map, x, z, f] as const);

test("an inn's upstairs (#193): up the stairs, rest in a bed, a reload stays up, and back down", async ({ page }) => {
  page.on("dialog", (d) => d.accept());
  await title(page);
  await menuItem(page, "Learn to play").click();
  await page.getByRole("button", { name: "Skip the lesson" }).click();
  await menuItem(page, "Continue").click();
  await loaded(page);
  const skip = page.getByRole("button", { name: /^Skip/ });
  await expect(skip).toBeVisible();
  await page.waitForTimeout(450); // a scene ignores a skip in its first 400 ms (#155)
  await page.keyboard.press("Escape");
  await expect(skip).toBeHidden();

  // In Rootholm's inn, in front of the stairs: E climbs them.
  await warp(page, "inn_grove", 8, 1, "n");
  await expect.poll(() => at(page)).toBe("inn_grove,8,1,n");
  await expect(page.getByRole("button", { name: /^Go upstairs/ })).toBeVisible();
  await page.keyboard.press("e");
  await expect.poll(() => at(page), { timeout: 60_000 }).toBe("inn_grove_up,8,2,s");

  // A bed upstairs: its line, then the inn's rest panel, without the counter (that stays downstairs).
  await warp(page, "inn_grove_up", 2, 2, "n");
  await expect.poll(() => at(page)).toBe("inn_grove_up,2,2,n");
  await expect(page.getByRole("button", { name: /^Rest/ })).toBeVisible();
  await page.keyboard.press("e");
  const talk = page.locator(".talk-box");
  await expect(talk).toBeVisible({ timeout: 60_000 });
  while (await talk.isVisible()) {
    await page.keyboard.press("Enter");
    await page.waitForTimeout(400);
  }
  const rest = page.getByRole("dialog", { name: "Rest a while" });
  await expect(rest).toBeVisible();
  await expect(rest.locator(".inn-choice")).toHaveCount(4);
  await expect(rest.locator(".inn-counter")).toHaveCount(0);
  await rest.getByRole("button", { name: "Close" }).click();
  await expect(rest).toBeHidden();

  // A save upstairs loads upstairs.
  const saved = await at(page);
  await page.reload();
  await menuItem(page, "Continue").click();
  await loaded(page);
  await expect.poll(() => at(page)).toBe(saved);

  // Back down the stairwell: onto the tile in front of the stairs.
  await warp(page, "inn_grove_up", 8, 2, "n");
  await expect.poll(() => at(page)).toBe("inn_grove_up,8,2,n");
  await expect(page.getByRole("button", { name: /^Go downstairs/ })).toBeVisible();
  await page.keyboard.press("e");
  await expect.poll(() => at(page), { timeout: 60_000 }).toBe("inn_grove,8,1,s");
});
