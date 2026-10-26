import { expect, test, type Locator, type Page } from "@playwright/test";
import { menuItem, title } from "./helpers";

/** Press `key` (Tab by default) until `target` has focus. */
async function moveTo(page: Page, target: Locator, key = "Tab", tries = 40) {
  for (let i = 0; i < tries; i++) {
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

test("keyboard only: packs, collection, decks, campaign and records", async ({ page }) => {
  await title(page);
  const go = async (label: string, heading: string | RegExp) => {
    await moveTo(page, menuItem(page, label));
    await page.keyboard.press("Enter");
    await expect(page.getByRole("heading", { level: 1, name: heading })).toBeAttached();
  };
  const back = async () => {
    // Back sits at the top: walk up to it.
    await moveTo(page, page.getByRole("button", { name: "Back", exact: true }), "Shift+Tab", 400);
    await page.keyboard.press("Enter");
    await expect(page.getByRole("navigation", { name: "Main menu" })).toBeVisible();
  };

  // A pack opened with the keyboard: pick it, tear it, then go on to the collection.
  await go("Open packs", "Open packs");
  await moveTo(page, page.getByRole("button", { name: /^Open one .*, 1 left$/ }).first());
  await page.keyboard.press("Enter");
  await moveTo(page, page.getByRole("button", { name: "Tear it open" }));
  await page.keyboard.press("Enter");
  await moveTo(page, page.getByRole("button", { name: "See the collection" }));
  await page.keyboard.press("Enter");

  // The collection: a type filter, and a card read in the side panel on focus.
  await expect(page.getByRole("heading", { level: 1, name: "Collection" })).toBeAttached();
  const fire = page.getByRole("button", { name: "Ember", exact: true });
  await moveTo(page, fire);
  await page.keyboard.press("Enter");
  await expect(fire).toHaveAttribute("aria-pressed", "true");
  await back();

  // A deck edited and saved.
  await go("My decks", "My decks");
  await moveTo(page, page.getByRole("button", { name: "Edit", exact: true }).first());
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { level: 1, name: "Deck editor" })).toBeAttached();
  // Past the whole card grid: a long way round, but every card is reachable.
  await moveTo(page, page.getByRole("button", { name: /^Remove one / }).first(), "Tab", 400);
  await page.keyboard.press("Enter");
  await moveTo(page, page.getByRole("button", { name: "Fill with energy" }), "Shift+Tab");
  await page.keyboard.press("Enter");
  await moveTo(page, page.getByRole("button", { name: "Save deck" }), "Shift+Tab", 400);
  await page.keyboard.press("Enter");
  await expect(page.getByRole("button", { name: "Saved" })).toBeVisible();
  await moveTo(page, page.getByRole("button", { name: "Decks", exact: true }), "Shift+Tab");
  await page.keyboard.press("Enter");
  await back();

  // The campaign and the records, in and back out.
  await go("Campaign", "Campaign");
  await back();
  await go("Records", "Records");
  await back();
});

test("keyboard only: the world, a cutscene and the in-game menu", async ({ page }) => {
  page.on("dialog", (d) => d.accept());
  await title(page);
  await moveTo(page, menuItem(page, "Learn to play"));
  await page.keyboard.press("Enter");
  await moveTo(page, page.getByRole("button", { name: "Skip the lesson" }));
  await page.keyboard.press("Enter");
  await moveTo(page, menuItem(page, "Continue"));
  await page.keyboard.press("Enter");
  await expect(page.getByRole("status").filter({ hasText: "Loading the world" })).toBeHidden({ timeout: 60_000 });

  // The intro scene: Enter goes on to the next line, which is read out; Escape skips the rest.
  const scene = page.getByRole("dialog");
  await expect(scene.getByRole("button", { name: /Next line|Continue|Close/ })).toBeFocused({ timeout: 30_000 });
  const first = await scene.locator("[aria-live]").textContent();
  expect(first).toBeTruthy();
  await page.keyboard.press("Enter");
  await page.keyboard.press("Enter");
  await expect(scene.locator("[aria-live]")).not.toHaveText(first!);
  await expect(scene.locator("[aria-live]")).not.toBeEmpty();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("button", { name: /^Skip/ })).toBeHidden();

  // Walk with the arrows (the hero leaves the tile it stood on).
  const at = () => page.locator("[data-hero]").getAttribute("data-hero");
  await expect.poll(at).toBeTruthy();
  const start = await at();
  await page.keyboard.down("ArrowDown");
  await expect.poll(at, { timeout: 15_000 }).not.toEqual(start);
  await page.keyboard.up("ArrowDown");

  // J opens the journal and closes it again.
  await page.keyboard.press("j");
  await expect(page.getByRole("dialog", { name: "Journal" })).toBeVisible();
  await page.keyboard.press("j");
  await expect(page.getByRole("dialog", { name: "Journal" })).toBeHidden();

  // Escape opens the in-game menu (focus on its first tab) and closes it.
  // ponytail: headless Chromium sometimes closes the menu just after a skipped
  // cutscene (not seen in desktop Chrome; see PR #94), so a closed menu is opened again.
  const menu = page.getByRole("dialog", { name: "Menu" });
  await expect(async () => {
    if (!(await menu.isVisible())) await page.keyboard.press("Escape");
    await expect(menu.getByRole("tab", { name: "Decks" })).toBeFocused({ timeout: 2_000 });
  }).toPass({ timeout: 20_000 });
  await page.keyboard.press("Escape");
  await expect(menu).toBeHidden();
});

test.describe("with motion", () => {
  test.use({ reducedMotion: "no-preference" });

  test("a screen change plays its transition and still lands focus on the new screen", async ({ page }) => {
    // Note each transition's kind (html[data-swap]) as it starts.
    await page.addInitScript(() => {
      const start = document.startViewTransition?.bind(document);
      if (!start) return;
      (window as unknown as { swaps: string[] }).swaps = [];
      document.startViewTransition = ((cb: () => unknown) => {
        (window as unknown as { swaps: string[] }).swaps.push(document.documentElement.dataset.swap ?? "");
        return start(cb);
      }) as typeof document.startViewTransition;
    });
    const swaps = () => page.evaluate(() => (window as unknown as { swaps?: string[] }).swaps ?? []);
    await page.goto("/");
    await expect(page.getByRole("button", { name: "Press to start" })).toBeFocused();
    await page.keyboard.press("Enter");
    await expect.poll(swaps).toContain("splash");
    await expect(menuItem(page, "Learn to play")).toBeFocused();
    await moveTo(page, menuItem(page, "Play a duel"));
    await page.keyboard.press("Enter");
    await expect.poll(swaps).toContain("screen");
    await expect(page.getByRole("heading", { level: 1, name: "Play a duel" })).toBeFocused();
  });
});
