import { expect, test, type Locator, type Page } from "@playwright/test";
import { menuItem, title } from "./helpers";

// #100: every menu is driven by the keys and by a pad. The painted hand points at
// the focused control; the arrows (a pad's D-pad) move it; Enter (A) confirms;
// Escape (B) folds a panel or goes back.

const hand = (page: Page) => page.locator(".focus-hand.on");

/** Press `key` until `target` has focus. */
async function moveTo(page: Page, target: Locator, key: string) {
  for (let i = 0; i < 20; i++) {
    if (await target.evaluate((el) => el === document.activeElement).catch(() => false)) return;
    await page.keyboard.press(key);
  }
  await expect(target).toBeFocused();
}

test("keyboard: arrows move the hand through the title and two menu screens, Escape goes back", async ({ page }) => {
  await title(page);
  await page.keyboard.press("ArrowDown"); // nothing focused yet: the first control
  await moveTo(page, menuItem(page, "Play a duel"), "ArrowDown");
  await expect(hand(page)).toBeVisible();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { level: 1, name: "Play a duel" })).toBeVisible();
  // The arrows reach the deck choices (the choice follows), and lead on out of them.
  const decks = page.getByRole("group", { name: "Your deck" }).getByRole("radio");
  await moveTo(page, decks.first(), "ArrowDown");
  await expect(hand(page)).toBeVisible();
  await page.keyboard.press("ArrowDown");
  await expect(decks.nth(1)).toBeFocused();
  await expect(decks.nth(1)).toBeChecked();
  await page.keyboard.press("ArrowRight");
  await expect(page.getByRole("group", { name: "The AI plays" }).getByRole("radio").first()).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("navigation", { name: "Main menu" })).toBeVisible();

  await moveTo(page, menuItem(page, "Records"), "ArrowDown");
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { level: 1, name: "Records" })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("navigation", { name: "Main menu" })).toBeVisible();

  // Settings opens as a sheet that keeps focus; Escape folds it, back on its button.
  const settings = page.locator("summary", { hasText: "Settings" });
  await moveTo(page, settings, "Tab");
  await page.keyboard.press("Enter");
  const sheet = page.getByRole("dialog", { name: "Settings" });
  await expect(sheet).toBeVisible();
  await expect.poll(() => sheet.evaluate((d) => d.contains(document.activeElement))).toBe(true);
  await page.keyboard.press("Tab");
  await expect.poll(() => sheet.evaluate((d) => d.contains(document.activeElement))).toBe(true);
  await page.keyboard.press("Escape");
  await expect(sheet).toBeHidden();
  await expect(settings).toBeFocused();
});

// A standard-mapping pad, held in the page: `window.pad(i)` taps button i, held
// until the game's poll has read it pressed, then released until read again
// (a headless page may run few frames a second).
const FAKE_PAD = () => {
  const buttons = Array.from({ length: 17 }, () => ({ pressed: false, touched: false, value: 0 }));
  const pad = { id: "test pad", index: 0, connected: true, mapping: "standard", axes: [0, 0, 0, 0], buttons, timestamp: 0 };
  let reads = 0;
  Object.defineProperty(navigator, "getGamepads", { value: () => (reads++, [pad]) });
  const read = async () => {
    const at = reads;
    while (reads < at + 1) await new Promise((r) => setTimeout(r, 20));
  };
  (window as unknown as { pad: (i: number) => Promise<void> }).pad = async (i: number) => {
    dispatchEvent(new Event("gamepadconnected"));
    buttons[i] = { pressed: true, touched: true, value: 1 };
    await read();
    buttons[i] = { pressed: false, touched: false, value: 0 };
    await read();
  };
};
const DOWN = 13, A = 0, B = 1, R = 5;

test("gamepad: the D-pad moves the hand, A opens, R switches tabs, B goes back", async ({ page }) => {
  await page.addInitScript(FAKE_PAD);
  await title(page);
  const tap = (i: number) => page.evaluate((b) => (window as unknown as { pad: (i: number) => Promise<void> }).pad(b), i);
  const target = menuItem(page, "Records");
  for (let i = 0; i < 12 && !(await target.evaluate((el) => el === document.activeElement)); i++) await tap(DOWN);
  await expect(target).toBeFocused();
  await expect(page.locator("html")).toHaveAttribute("data-input", "pad");
  await expect(hand(page)).toBeVisible();
  await tap(A);
  await expect(page.getByRole("heading", { level: 1, name: "Records" })).toBeVisible();
  await tap(R);
  await expect(page.getByRole("tab", { name: "Stats" })).toHaveAttribute("aria-selected", "true");
  await tap(B);
  await expect(page.getByRole("navigation", { name: "Main menu" })).toBeVisible();

  const decks = menuItem(page, "My decks");
  for (let i = 0; i < 12 && !(await decks.evaluate((el) => el === document.activeElement)); i++) await tap(DOWN);
  await tap(A);
  await expect(page.getByRole("heading", { level: 1, name: "My decks" })).toBeVisible();
  await tap(B);
  await expect(page.getByRole("navigation", { name: "Main menu" })).toBeVisible();
});
