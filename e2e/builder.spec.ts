import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Locator, type Page } from "@playwright/test";
import { menuItem, readSave, title, moveTo } from "./helpers";

/** From the title to the deck editor of the first deck (the starter). */
async function editStarter(page: Page) {
  await menuItem(page, "My decks").click();
  await page.getByRole("button", { name: "Edit", exact: true }).first().click();
  await expect(page.getByRole("heading", { level: 1, name: "Deck editor" })).toBeAttached();
}

/** axe-core: no serious or critical WCAG 2.2 A/AA violation on what is shown. */
async function scan(page: Page, what: string) {
  const { violations } = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"])
    .exclude("canvas")
    .analyze();
  const bad = violations
    .filter((v) => v.impact === "serious" || v.impact === "critical")
    .map((v) => `${what}: ${v.id} (${v.impact}) ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`);
  expect(bad, bad.join("\n")).toEqual([]);
}

/** The number of copies the deck pane lists for a card ("3×"), 0 when absent. */
async function copies(page: Page, name: string): Promise<number> {
  const line = page.getByRole("tabpanel", { name: /^Deck/ }).locator(".deck-line", { hasText: name });
  if ((await line.count()) === 0) return 0;
  return Number((await line.locator(".line-count").innerText()).replace("×", ""));
}

test("build for me makes a legal deck from your cards and says why", async ({ page }) => {
  await title(page);
  await menuItem(page, "My decks").click();
  await page.getByRole("button", { name: "New deck" }).click();
  await expect(page.getByRole("heading", { level: 1, name: "Deck editor" })).toBeAttached();

  // An empty deck opens on a start panel: build it, or pick Basics.
  await expect(page.getByRole("heading", { name: "Start this deck" })).toBeVisible();
  await page.getByRole("toolbar", { name: "Deck tools" }).getByRole("button", { name: "Build for me" }).click();
  const sheet = page.getByRole("dialog", { name: "Build for me" });
  await expect(sheet).toBeVisible();
  await sheet.getByRole("button", { name: "Ember" }).click();
  await sheet.getByRole("button", { name: "Tide" }).click();
  await expect(sheet.getByRole("button", { name: "Ember" })).toHaveAttribute("aria-pressed", "true");
  await sheet.getByRole("radio", { name: "Aggro" }).check();
  await expect(sheet.getByText("Cheap attacks and plenty of Basics")).toBeVisible();
  await sheet.getByRole("radio", { name: "Balanced" }).check();
  // Every part of the deck comes with its reason.
  await expect(sheet.locator(".reasons li").first()).toContainText("complete Awaken line");
  expect(await sheet.locator(".reasons li").count()).toBeGreaterThan(5);
  await expect(sheet.getByText("A new deck from your cards.")).toBeVisible();
  await scan(page, "build for me");
  // A colour you own no Basic of can't be picked, and says why.
  await sheet.getByRole("button", { name: /^Grove/ }).click({ force: true });
  await expect(sheet.getByText("You own no Basic Grove Hexon yet")).toBeVisible();
  await expect(sheet.getByRole("button", { name: /^Grove/ })).toHaveAttribute("aria-pressed", "false");
  await sheet.getByRole("button", { name: "Use this deck" }).click();
  await expect(sheet).toBeHidden();
  await expect(page.getByRole("toolbar", { name: "Deck tools" }).getByRole("button", { name: "Build for me" })).toBeFocused();

  await expect(page.locator(".deck-total b")).toHaveText("60");
  await expect(page.getByText("Ready to play")).toBeVisible();
  await expect(page.getByLabel("Deck name")).toHaveValue(/Balanced$/);

  // The test hand: seven cards and the odds.
  await page.getByRole("button", { name: "Test a hand" }).click();
  const test = page.getByRole("dialog", { name: "Test a hand" });
  await expect(test.getByRole("list", { name: "Your opening hand" }).getByRole("listitem")).toHaveCount(7);
  await expect(test.getByText("Hands with no Basic")).toBeVisible();
  await scan(page, "test a hand");
  await test.getByRole("button", { name: "Draw again" }).click();
  await test.getByRole("button", { name: "Close" }).click();

  await page.getByRole("button", { name: "Save deck" }).click();
  const decks = (await readSave(page)).decks;
  expect(decks).toHaveLength(2);
  expect(decks[1]).toMatch(/Balanced/);
});

test("a suggestion applies in one tap, and Undo takes it back", async ({ page }) => {
  await title(page);
  await editStarter(page);
  const deck = page.getByRole("tabpanel", { name: /^Deck/ });
  const cindrel = await copies(page, "Cindrel");
  for (let i = 0; i < cindrel; i++) await deck.getByRole("button", { name: "Remove one Cindrel" }).click();
  expect(await copies(page, "Cindrel")).toBe(0);

  // Flarrow has nothing to awaken from: the advice says so and adds Cindrel.
  const tip = page.locator(".advice-item.tip", { hasText: "Add Cindrel" });
  await expect(tip).toContainText("Flarrow");
  await scan(page, "the editor with advice");
  await page.getByRole("button", { name: "Apply: Add Cindrel" }).click();
  await expect.poll(() => copies(page, "Cindrel")).toBe(1);
  await expect(page.getByRole("status").filter({ hasText: "Undo takes it back" })).toBeVisible();

  await page.getByRole("button", { name: "Undo" }).click();
  await expect.poll(() => copies(page, "Cindrel")).toBe(0);
  // Undo again walks back through the removals.
  await page.getByRole("button", { name: "Undo" }).click();
  await expect.poll(() => copies(page, "Cindrel")).toBe(1);
});

test("the deck shelf shows locked decks and how to unlock them", async ({ page }) => {
  await title(page);
  await menuItem(page, "My decks").click();
  const shelf = page.getByRole("region", { name: "Deck shelf" });
  await expect(shelf.getByText("0 of 11 unlocked")).toBeVisible();
  const ember = shelf.getByRole("listitem").filter({ hasText: "Ember Rush" });
  await expect(ember).toContainText("Beat Cinza");
  await expect(ember.getByRole("button")).toHaveCount(0);
  await scan(page, "the deck shelf");
});

test("keyboard only: build a deck for me, undo it, walk the cards, leave", async ({ page }) => {
  await title(page);
  await menuItem(page, "My decks").focus();
  await page.keyboard.press("Enter");
  const edit = page.getByRole("button", { name: "Edit", exact: true }).first();
  await moveTo(page, edit);
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { level: 1, name: "Deck editor" })).toBeAttached();

  // Tab to "Build for me"; the sheet opens on its title.
  await moveTo(page, page.getByRole("button", { name: "Build for me" }));
  await page.keyboard.press("Enter");
  const sheet = page.getByRole("dialog", { name: "Build for me" });
  await expect(sheet.getByRole("heading", { name: "Build for me" })).toBeFocused();
  // Escape closes the sheet only, focus back on its button.
  await page.keyboard.press("Escape");
  await expect(sheet).toBeHidden();
  await expect(page.getByRole("button", { name: "Build for me" })).toBeFocused();
  await page.keyboard.press("Enter");
  // The style is a radio group: Tab reaches the chosen one, the arrows change it.
  await moveTo(page, sheet.getByRole("radio", { name: "Balanced" }));
  await page.keyboard.press("ArrowRight");
  await expect(sheet.getByRole("radio", { name: "Control" })).toBeChecked();
  await moveTo(page, sheet.getByRole("button", { name: "Use this deck" }));
  await page.keyboard.press("Enter");
  await expect(sheet).toBeHidden();
  // Focus goes back to what opened the sheet (on the next frame): wait for it, or it lands mid-walk.
  await expect(page.locator(".bench-actions .build")).toBeFocused();
  await expect(page.getByRole("button", { name: "Save deck" })).toBeEnabled();

  // Ctrl+Z: the deck as it was, nothing left to save.
  await page.keyboard.press("Control+z");
  await expect(page.getByRole("button", { name: "Saved" })).toBeVisible();

  // The arrows walk the card grid.
  const first = page.locator(".pick-grid .bind-card").first();
  await first.focus();
  await page.keyboard.press("ArrowRight");
  await expect(first).not.toBeFocused();
  // Still in the grid (else: what took focus).
  expect(await page.evaluate(() => (document.activeElement?.closest(".pick-grid") ? "grid" : document.activeElement?.outerHTML.slice(0, 120)))).toBe("grid");

  // Escape leaves (nothing unsaved).
  await page.keyboard.press("Escape");
  await expect(page.getByRole("heading", { level: 1, name: "My decks" })).toBeVisible();
});

/** A fake standard gamepad the page polls; `press` holds a button for a few frames. */
async function fakePad(page: Page) {
  await page.addInitScript(() => {
    const buttons = Array.from({ length: 17 }, () => ({ pressed: false, touched: false, value: 0 }));
    const pad = { id: "Fake pad", index: 0, connected: true, mapping: "standard", axes: [0, 0, 0, 0], buttons, timestamp: 0 };
    Object.defineProperty(navigator, "getGamepads", { value: () => [pad] });
    (window as any).__pad = (i: number, on: boolean) => {
      buttons[i] = { pressed: on, touched: on, value: on ? 1 : 0 };
      if (on) dispatchEvent(new Event("gamepadconnected"));
    };
  });
}

const A = 0, B = 1, RIGHT = 15;

/** Two frames: the input layer polls the pad once a frame, so it has seen the change. */
const frames = (page: Page) => page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));

async function press(page: Page, button: number) {
  await page.evaluate((b) => (window as any).__pad(b, true), button);
  await frames(page);
  await page.evaluate((b) => (window as any).__pad(b, false), button);
  await frames(page);
}

test("gamepad: build a deck for me, walk the cards, save and leave", async ({ page }) => {
  await fakePad(page);
  await title(page);
  await menuItem(page, "My decks").focus();
  await press(page, A);
  const edit = page.getByRole("button", { name: "Edit", exact: true }).first();
  await edit.focus();
  await press(page, A);
  await expect(page.getByRole("heading", { level: 1, name: "Deck editor" })).toBeAttached();

  await page.getByRole("button", { name: "Build for me" }).focus();
  await press(page, A);
  const sheet = page.getByRole("dialog", { name: "Build for me" });
  await expect(sheet.getByRole("heading", { name: "Build for me" })).toBeFocused();
  // The D-pad reaches the sheet's buttons; A uses the deck.
  await sheet.getByRole("button", { name: "Use this deck" }).focus();
  await press(page, A);
  await expect(sheet).toBeHidden();

  // The D-pad walks the card grid.
  const first = page.locator(".pick-grid .bind-card").first();
  await first.focus();
  await press(page, RIGHT);
  await expect(first).not.toBeFocused();

  // B steps back: unsaved changes ask first; A on "Save and leave".
  await press(page, B);
  await expect(page.getByRole("button", { name: "Save and leave" })).toBeFocused();
  await press(page, A);
  await expect(page.getByRole("heading", { level: 1, name: "My decks" })).toBeVisible();
  expect((await readSave(page)).decks?.[0]).toMatch(/Balanced|Trailhead/);
});

test("on a phone: the tools stay at hand, the advice and the deck have their tabs @phone", async ({ page }, info) => {
  test.skip(info.project.name !== "phone", "the tabs are the phone layout");
  await title(page);
  await editStarter(page);
  await page.getByRole("tab", { name: "Advice" }).click();
  await expect(page.getByRole("tabpanel", { name: "Advice" }).getByText("Ready to play")).toBeVisible();
  // A tab list: the arrows choose and focus the next tab, Home the first.
  await page.getByRole("tab", { name: "Advice" }).press("ArrowRight");
  await expect(page.getByRole("tab", { name: "Your cards" })).toBeFocused();
  await expect(page.getByRole("tab", { name: "Your cards" })).toHaveAttribute("aria-selected", "true");
  await page.keyboard.press("Home");
  await expect(page.getByRole("tab", { name: "Advice" })).toHaveAttribute("aria-selected", "true");
  await page.getByRole("button", { name: "Build for me" }).click();
  const sheet = page.getByRole("dialog", { name: "Build for me" });
  await sheet.getByRole("radio", { name: "Aggro" }).check();
  await sheet.getByRole("button", { name: "Use this deck" }).click();
  await expect(page.getByRole("tab", { name: "Deck (60)" })).toBeVisible();
  await page.getByRole("button", { name: "Undo" }).click();
  await expect(page.getByRole("button", { name: "Saved" })).toBeVisible();
});
