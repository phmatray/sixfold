import { expect, test, type Page } from "@playwright/test";
import { editSave, menuItem, title } from "./helpers";

async function loaded(page: Page) {
  await expect(page.getByRole("button", { name: "Menu", exact: true })).toBeVisible();
  await expect(page.getByRole("status").filter({ hasText: "Loading the world" })).toBeHidden({ timeout: 60_000 });
}

/** Stand on `map,x,z,facing`, talk to whoever is ahead, and accept their challenge. */
async function challenge(page: Page, name: string, map: string, x: number, z: number) {
  await page.locator(".world-host").evaluate((el, a) => (el as unknown as { sixfoldWarp: (...a: unknown[]) => void }).sixfoldWarp(...a), [map, x, z, "n"]);
  await expect(page.locator("[data-hero]")).toHaveAttribute("data-hero", new RegExp(`^${x},${z}`), { timeout: 15_000 });
  await expect(page.getByRole("button", { name: new RegExp(`^Talk to ${name}`) })).toBeVisible();
  await page.keyboard.press("e");
  // The map's first talk draws the speaker's portrait in a new WebGL context: slow on CI's software GL.
  const talk = page.locator(".talk-box");
  await expect(talk).toBeVisible({ timeout: 60_000 });
  while (await talk.isVisible()) {
    await page.keyboard.press("Enter");
    await page.waitForTimeout(400);
  }
  const offer = page.getByRole("dialog", { name: `Challenge ${name}?` });
  await expect(offer.getByRole("button", { name: /^Challenge/ })).toBeFocused();
  // The panel ignores presses in its first half second (those that ended the lines).
  await page.waitForTimeout(600);
  await page.keyboard.press("Enter");
  // (Not its heading: a seal duel's title card is modal, and hides the table from the accessibility tree.)
  await expect(page.locator(".duel")).toBeVisible();
}

async function leaveDuel(page: Page) {
  await page.getByRole("button", { name: "Duel menu" }).click();
  await page.getByRole("dialog", { name: /menu/i }).getByRole("button", { name: "Leave the duel…" }).click();
  await page.getByRole("alertdialog", { name: /^Leave this duel\?/ }).getByRole("button", { name: "Leave", exact: true }).click();
  await loaded(page);
}

test("a seal duel opens on its title card and wears its region's colour; a route duel does neither", async ({ page }) => {
  page.on("dialog", (d) => d.accept());
  await title(page);
  await menuItem(page, "Learn to play").click();
  await page.getByRole("button", { name: "Skip the lesson" }).click();
  await expect(menuItem(page, "Continue")).toBeVisible();
  // Past the intro and vouched for (the keepers' threshold, #150): on the Bellflats, south of Marlo.
  await editSave(
    page,
    `secs.progress = [(secs.progress?.[0] ?? "") + ",f:intro,f:vouched"]; secs.world_at = ["bellflats,22,16,n"];`,
  );
  await page.reload();
  await menuItem(page, "Continue").click();
  await loaded(page);

  await challenge(page, "Marlo", "bellflats", 22, 16);
  const card = page.getByRole("dialog", { name: "Seal of the Tide" });
  await expect(card).toBeVisible();
  await expect(card).toContainText("Marlo");
  await expect(page.locator(".duel")).toHaveClass(/\bseal-duel\b.*\bt-water\b|\bt-water\b.*\bseal-duel\b/);
  // Enter lets the duel begin: the setup's cards come up playable.
  await page.keyboard.press("Enter");
  await expect(card).toBeHidden();
  await expect(page.getByRole("list", { name: /^Your hand/ }).getByRole("button", { name: /, playable$/ }).first()).toBeVisible();
  await leaveDuel(page);

  // Tobin, a route duelist: no card, a plain table.
  await challenge(page, "Tobin", "overworld", 23, 28);
  await expect(page.getByRole("list", { name: /^Your hand/ }).getByRole("button", { name: /, playable$/ }).first()).toBeVisible();
  await expect(page.getByRole("dialog", { name: /^Seal of/ })).toHaveCount(0);
  await expect(page.locator(".duel")).not.toHaveClass(/seal-duel/);
});
