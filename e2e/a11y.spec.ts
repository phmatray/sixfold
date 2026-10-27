import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { menuItem, title } from "./helpers";

/**
 * axe-core on every screen: no serious or critical WCAG 2.2 A/AA violation.
 * The world's canvas is left out (a WebGL scene has no DOM to check).
 */
async function scan(page: Page, screen: string) {
  const { violations } = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"])
    .exclude("canvas")
    .analyze();
  const bad = violations
    .filter((v) => v.impact === "serious" || v.impact === "critical")
    .map((v) => `${screen}: ${v.id} (${v.impact}) ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`);
  expect(bad, bad.join("\n")).toEqual([]);
}

test("title, settings, campaign, collection, records, packs and decks @phone", async ({ page }) => {
  await title(page);
  await scan(page, "title");
  // Settings opens as a sheet over the title (#100): its summary opens and closes it.
  const settings = page.locator("summary", { hasText: "Settings" });
  await settings.click();
  await expect(page.getByRole("dialog", { name: "Settings" })).toBeVisible();
  await scan(page, "settings");
  await settings.click();

  for (const [item, heading] of [
    ["Campaign", "Campaign"],
    ["Collection", "Collection"],
    ["Records", "Records"],
    ["My decks", "My decks"],
  ]) {
    await menuItem(page, item).click();
    await expect(page.getByRole("heading", { level: 1, name: heading })).toBeAttached();
    await scan(page, item);
    await page.getByRole("button", { name: "Back", exact: true }).click();
  }

  await menuItem(page, "My decks").click();
  await page.getByRole("button", { name: "Edit", exact: true }).first().click();
  await expect(page.getByRole("heading", { level: 1, name: "Deck editor" })).toBeAttached();
  await scan(page, "deck editor");
  await page.getByRole("button", { name: "Decks", exact: true }).click();
  await page.getByRole("button", { name: "Back", exact: true }).click();

  await menuItem(page, "Open packs").click();
  await scan(page, "packs");
  await page.getByRole("button", { name: /^Open one .*, 1 left$/ }).first().click();
  await page.getByRole("button", { name: "Tear it open" }).click();
  await expect(page.getByRole("button", { name: "See the collection" })).toBeVisible();
  await scan(page, "pack reveal");
});

test("a duel: setup, the board, the log and the menu @phone", async ({ page }) => {
  await title(page);
  await menuItem(page, "Play a duel").click();
  await scan(page, "duel setup");
  await page.getByRole("button", { name: "Start the duel" }).click();
  await expect(page.getByRole("heading", { level: 1, name: /^Duel against/ })).toBeAttached();
  const hand = page.getByRole("list", { name: /^Your hand/ });
  await hand.getByRole("button", { name: /, playable$/ }).first().click();
  await page.getByRole("button", { name: /^Empty Active spot/ }).click();
  await page.getByRole("button", { name: /^Done/ }).click();
  // Your turn: End turn enabled (it shows, disabled, while the opponent's first turn still plays out).
  await expect(page.getByRole("button", { name: /^End turn/ })).toBeEnabled({ timeout: 60_000 });
  // A screen reader hears the setup and whose turn it is, in one announcement.
  await expect(page.getByRole("status").filter({ hasText: /Turn \d+: Your turn/ })).toBeAttached();
  await scan(page, "duel board");

  await page.keyboard.press("l");
  await expect(page.getByRole("dialog", { name: "Duel log" })).toBeVisible();
  await scan(page, "duel log");
  await page.keyboard.press("Escape");

  await page.keyboard.press("m");
  await expect(page.getByRole("dialog", { name: /menu/i })).toBeVisible();
  await scan(page, "duel menu");
});

test("the world and its in-game menu", async ({ page }) => {
  page.on("dialog", (d) => d.accept());
  await title(page);
  await menuItem(page, "Learn to play").click();
  await page.getByRole("button", { name: "Skip the lesson" }).click();
  await menuItem(page, "Continue").click();
  await expect(page.getByRole("status").filter({ hasText: "Loading the world" })).toBeHidden({ timeout: 60_000 });
  // The intro scene's first line, a dialogue box.
  await expect(page.getByRole("button", { name: /^Skip/ })).toBeVisible();
  await scan(page, "world dialogue");
  await page.keyboard.press("Escape");
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog", { name: "Menu" })).toBeVisible();
  await scan(page, "world menu");
});

test("text size: 150 % is applied at once and kept", async ({ page }) => {
  await title(page);
  await page.getByText("Settings").click();
  await page.getByRole("group", { name: "Text size" }).getByLabel("150%").check();
  const size = () => page.evaluate(() => parseFloat(getComputedStyle(document.body).fontSize));
  await expect.poll(size).toBeCloseTo(22.5);
  await page.reload();
  await expect(page.getByRole("navigation", { name: "Main menu" })).toBeVisible();
  await expect.poll(size).toBeCloseTo(22.5);
});
