import { expect, test, type Locator, type Page } from "@playwright/test";
import { menuItem } from "./helpers";

/** Press `key` (Tab by default) until `target` has focus. */
async function moveTo(page: Page, target: Locator, key = "Tab") {
  for (let i = 0; i < 40; i++) {
    if (await target.evaluate((el) => el === document.activeElement).catch(() => false)) return;
    await page.keyboard.press(key);
  }
  await expect(target).toBeFocused();
}

test("keyboard only: title, duel setup, a duel started, then left", async ({ page }) => {
  await page.goto("/");
  const press = page.getByRole("button", { name: "Press to start" });
  await expect(press).toBeFocused();
  await page.keyboard.press("Enter");

  // The title lands focus on its first menu entry.
  await expect(menuItem(page, "Learn to play")).toBeFocused();
  await moveTo(page, menuItem(page, "Play a duel"));
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { level: 1, name: "Play a duel" })).toBeVisible();
  await moveTo(page, page.getByRole("button", { name: "Start the duel" }));
  await page.keyboard.press("Enter");

  // H focuses the hand; arrows move along it to a Basic that can be played;
  // Enter picks it and moves focus to where it can go; Enter puts it there.
  await expect(page.getByRole("heading", { level: 1, name: /^Duel against/ })).toBeAttached();
  const hand = page.getByRole("list", { name: /^Your hand/ });
  await expect(hand.getByRole("button", { name: /, playable$/ }).first()).toBeVisible();
  await page.keyboard.press("h");
  await moveTo(page, hand.getByRole("button", { name: /, playable$/ }).first(), "ArrowRight");
  await page.keyboard.press("Enter");
  await expect(page.getByRole("button", { name: /^Empty Active spot/ })).toBeFocused();
  await page.keyboard.press("Enter");
  // E: no Bench this time.
  const done = page.getByRole("button", { name: /^Done/ });
  await expect(done).toHaveAttribute("aria-disabled", "false");
  await page.keyboard.press("e");
  await expect(done).toBeHidden();

  // M opens the duel menu; leaving asks first.
  await page.keyboard.press("m");
  const menu = page.getByRole("dialog", { name: /menu/i });
  await expect(menu).toBeVisible();
  await moveTo(page, menu.getByRole("button", { name: "Leave the duel…" }));
  await page.keyboard.press("Enter");
  const ask = page.getByRole("alertdialog", { name: /^Leave this duel\?/ });
  await expect(ask.getByRole("button", { name: "Keep playing" })).toBeFocused();
  await moveTo(page, ask.getByRole("button", { name: "Leave", exact: true }));
  await page.keyboard.press("Enter");
  await expect(page.getByRole("navigation", { name: "Main menu" })).toBeVisible();
});

test.describe("with motion", () => {
  test.use({ reducedMotion: "no-preference" });

  test("a screen change plays its transition and still lands focus on the new screen", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByRole("button", { name: "Press to start" })).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(menuItem(page, "Learn to play")).toBeFocused();
    await moveTo(page, menuItem(page, "Play a duel"));
    const swapped = page.evaluate(() => new Promise((r) => {
      new MutationObserver(() => document.documentElement.dataset.swap && r(document.documentElement.dataset.swap)).observe(document.documentElement, { attributes: true });
    }));
    await page.keyboard.press("Enter");
    expect(await swapped).toBe("screen");
    await expect(page.getByRole("heading", { level: 1, name: "Play a duel" })).toBeFocused();
  });
});
