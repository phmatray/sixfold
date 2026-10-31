import AxeBuilder from "@axe-core/playwright";
import { expect, test, editSave, loaded, menuItem, readSave, title, type Page } from "./helpers";

/** Talk to Wynne across her counter, through her lines. */
async function talkToWynne(page: Page) {
  await expect(page.getByRole("button", { name: /^Talk to Wynne/ })).toBeVisible({ timeout: 30_000 });
  await page.keyboard.press("e");
  const talk = page.locator(".talk-box");
  await expect(talk).toBeVisible({ timeout: 60_000 });
  while (await talk.isVisible()) {
    await page.keyboard.press("Enter");
    await page.waitForTimeout(400);
  }
}

// #194: a pack gained in the world opens from the menu's Packs tab, over the paused world, and
// closing the menu leaves the hero on the tile they stood on (no trip to the title, no map reload).
test("packs open from the world: Wynne's gift and her counter offer Open it now, and the hero stays put", async ({ page }) => {
  test.slow(); // a warp and a map load at ~1 frame a second on CI's software GL
  page.on("dialog", (d) => d.accept());
  await title(page);
  await menuItem(page, "Learn to play").click();
  await page.getByRole("button", { name: "Skip the lesson" }).click();
  await expect(menuItem(page, "Continue")).toBeVisible();
  // A first seal (Wynne gives a Wild Pack for it), the shoes she gives first, and Facets for her counter.
  await editSave(
    page,
    `secs.progress = [(secs.progress?.[0] ?? "").replace(/c:\\d+/, "c:500") + ",f:intro,i:running_shoes,f:beat_bramble"]; secs.world_at = ["shop,7,3,n"];`,
  );
  await page.reload();
  await menuItem(page, "Continue").click();
  await loaded(page);
  const hero = page.locator("[data-hero]");
  await expect(hero).toHaveAttribute("data-hero", /^7,3,/, { timeout: 15_000 });
  const tile = await hero.getAttribute("data-hero");

  // The gift: its line no longer sends the player to the title screen.
  await talkToWynne(page);
  const gift = page.getByRole("dialog", { name: "A gift" });
  await expect(gift).toContainText("You got 1 Wild Pack.");
  await expect(gift).not.toContainText("title screen");
  await gift.getByRole("button", { name: "Open it now" }).click();

  // The menu, on its Packs tab: the count in its name (three starting packs and the gift).
  const menu = page.getByRole("dialog", { name: "Menu" });
  await expect(menu.getByRole("tab", { name: "Packs (4 to open)" })).toHaveAttribute("aria-selected", "true");
  await expect(menu.getByRole("tab", { name: "Packs (4 to open)" })).toBeFocused();
  const { violations } = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"])
    .exclude("canvas")
    .analyze();
  const bad = violations.filter((v) => v.impact === "serious" || v.impact === "critical").map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`);
  expect(bad, bad.join("\n")).toEqual([]);

  const before = (await readSave(page)).wallet;
  await menu.getByRole("button", { name: /^Open one Wild Pack, 2 left$/ }).click();
  await menu.getByRole("button", { name: "Tear it open" }).click();
  // With reduced motion the cards turn over at once; the pack is spent.
  const collection = menu.getByRole("button", { name: "See the collection" });
  await expect(collection).toBeVisible();
  await expect(menu.getByRole("tab", { name: "Packs (3 to open)" })).toBeVisible();
  expect((await readSave(page)).wallet).not.toEqual(before);
  // The reveal's way out is the menu's own Collection tab, not the title's screen.
  await collection.click();
  await expect(menu.getByRole("tab", { name: "Collection" })).toHaveAttribute("aria-selected", "true");
  await menu.getByRole("button", { name: "Close", exact: true }).click();
  await expect(menu).toBeHidden();
  await expect(hero).toHaveAttribute("data-hero", tile!);

  // Her counter: a pack bought there opens from there.
  await talkToWynne(page);
  const counter = page.getByRole("dialog", { name: "Wynne's pack counter" });
  await expect(counter).toBeVisible();
  await expect(counter).not.toContainText("title screen");
  await counter.getByRole("button", { name: "Buy one Wild Pack for 200 Facets" }).click();
  await expect(counter.getByRole("status")).toContainText("You bought 1 Wild Pack.");
  await counter.getByRole("button", { name: "Open it now" }).click();
  await expect(menu.getByRole("tab", { name: "Packs (4 to open)" })).toHaveAttribute("aria-selected", "true");
  await page.keyboard.press("Escape");
  await expect(menu).toBeHidden();
  await expect(hero).toHaveAttribute("data-hero", tile!);
});
