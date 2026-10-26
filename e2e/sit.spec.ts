import { expect, test, type Page } from "@playwright/test";
import { menuItem, readSave, title } from "./helpers";

/** Where the save says the hero stands: `map,x,z,facing`. */
const at = async (page: Page) => (await readSave(page)).world_at?.[0] ?? "";
const sitting = (page: Page) => page.evaluate(() => (document.querySelector(".world-host") as any)?.sixfoldStats?.().sitting);

test("sit on a bench: Sit, stand up with Escape or a move, and a reload stands in front of it", async ({ page }) => {
  page.on("dialog", (d) => d.accept());
  await title(page);
  await menuItem(page, "Learn to play").click();
  await page.getByRole("button", { name: "Skip the lesson" }).click();
  await menuItem(page, "Continue").click();
  await expect(page.getByRole("status").filter({ hasText: "Loading the world" })).toBeHidden({ timeout: 60_000 });
  const skip = page.getByRole("button", { name: /^Skip/ });
  await expect(skip).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(skip).toBeHidden();

  // In front of the meadow's first bench (22..23, 16), facing it.
  await page.evaluate(() => (document.querySelector(".world-host") as any).sixfoldWarp("overworld", 22, 17, "n"));
  await expect.poll(() => at(page)).toBe("overworld,22,17,n");
  const sit = page.getByRole("button", { name: /^Sit/ });
  const up = page.getByRole("button", { name: /^Stand up/ });

  // E sits; Escape stands up (and leaves the menu shut).
  await expect(sit).toBeVisible();
  await page.keyboard.press("e");
  await expect.poll(() => sitting(page)).toBe(true);
  await expect(up).toBeVisible();
  await expect.poll(() => at(page)).toBe("overworld,22,17,s"); // the tile in front stays the hero's
  await page.keyboard.press("Escape");
  await expect.poll(() => sitting(page)).toBe(false);
  await expect(page.getByRole("dialog", { name: "Menu" })).toBeHidden();

  // Facing away now: turn back, sit with the prompt, and a move key stands up (on the same tile).
  await page.keyboard.press("ArrowUp");
  await sit.click();
  await expect.poll(() => sitting(page)).toBe(true);
  await page.keyboard.press("ArrowLeft");
  await expect.poll(() => sitting(page)).toBe(false);
  expect(await at(page)).toBe("overworld,22,17,s");

  // Seated, a reload stands the hero on the tile in front of the bench.
  await page.evaluate(() => (document.querySelector(".world-host") as any).sixfoldWarp("overworld", 22, 17, "n"));
  await expect.poll(() => at(page)).toBe("overworld,22,17,n");
  await page.keyboard.press("e");
  await expect.poll(() => sitting(page)).toBe(true);
  await page.reload();
  await menuItem(page, "Continue").click();
  await expect(page.getByRole("status").filter({ hasText: "Loading the world" })).toBeHidden({ timeout: 60_000 });
  await expect(page.locator("[data-hero]")).toHaveAttribute("data-hero", "22,17,s");
  expect(await sitting(page)).toBe(false);
});
