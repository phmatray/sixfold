import { expect, test, type Locator, type Page } from "@playwright/test";
import { menuItem, title, moveTo } from "./helpers";

// #100: every menu is driven by the keys and by a pad. The painted hand points at
// the focused control; the arrows (a pad's D-pad) move it; Enter (A) confirms;
// Escape (B) folds a panel or goes back.

const hand = (page: Page) => page.locator(".focus-hand.on");

/** The hand sits on one side of `target` and points at it (its glyph turned toward it). */
async function pointsAt(page: Page, target: Locator) {
  await expect(hand(page)).toBeVisible();
  await expect
    .poll(() =>
      target.evaluate((el) => {
        const hd = document.querySelector<HTMLElement>(".focus-hand")!;
        const h = hd.getBoundingClientRect();
        let t = el.getBoundingClientRect();
        const label = (el as HTMLInputElement).labels?.[0] ?? el.closest("label");
        if (t.width < 4 && label) t = label.getBoundingClientRect();
        const side = hd.dataset.side;
        const ok = side === "left" ? h.right <= t.left + 6 : side === "right" ? h.left >= t.right - 6
          : side === "above" ? h.bottom <= t.top + 6 : side === "below" ? h.top >= t.bottom - 6 : false;
        return ok ? side : `wrong: ${side} ${JSON.stringify(h)} ${JSON.stringify(t)}`;
      }),
    )
    .toMatch(/^(left|right|above|below)$/);
}

test("keyboard: arrows move the hand through the title and two menu screens, Escape goes back", async ({ page }) => {
  await title(page);
  await page.keyboard.press("ArrowDown"); // nothing focused yet: the first control
  await moveTo(page, menuItem(page, "Play a duel"), "ArrowDown");
  await pointsAt(page, menuItem(page, "Play a duel"));
  await expect(page.locator(".focus-hand")).toHaveAttribute("data-side", "left");
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { level: 1, name: "Play a duel" })).toBeVisible();
  // The arrows reach the deck choices (the choice follows), and lead on out of them.
  const decks = page.getByRole("group", { name: "Your deck" }).getByRole("radio");
  await moveTo(page, decks.first(), "ArrowDown");
  await pointsAt(page, decks.first());
  // A new player holds one deck (prebuilts unlock later, #102); the opponent's list has them all.
  const theirs = page.getByRole("group", { name: "Your opponent plays" }).getByRole("radio");
  await page.keyboard.press("ArrowRight");
  await expect(theirs.first()).toBeFocused();
  await page.keyboard.press("ArrowDown");
  await expect(theirs.nth(1)).toBeFocused();
  await expect(theirs.nth(1)).toBeChecked();
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
  await pointsAt(page, page.locator(":focus"));
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
  await pointsAt(page, target);
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

test("on a phone, the hand still points at what has focus, from whichever side has room @phone", async ({ page }) => {
  await title(page);
  for (const label of ["Learn to play", "Play a duel", "Campaign", "Collection", "Records"]) {
    await moveTo(page, menuItem(page, label), "Tab");
    await pointsAt(page, menuItem(page, label));
  }
  await moveTo(page, page.locator("summary", { hasText: "Settings" }), "Tab");
  await pointsAt(page, page.locator("summary", { hasText: "Settings" }));
});
