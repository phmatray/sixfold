import { expect, test, menuItem, readSave, title, type Page } from "./helpers";

/** From the title: edit the first deck, take one card out, fill back with energy, save. */
async function editFirstDeck(page: Page) {
  await menuItem(page, "My decks").click();
  await page.getByRole("button", { name: "Edit", exact: true }).first().click();
  await expect(page.getByRole("heading", { level: 1, name: "Deck editor" })).toBeAttached();
  const deck = page.getByRole("tabpanel", { name: /^Deck/ });
  await deck.getByRole("button", { name: /^Remove one / }).first().click();
  await deck.getByRole("button", { name: "Fill with energy" }).click();
  await page.getByRole("button", { name: "Save deck" }).click();
  await expect(page.getByRole("button", { name: "Saved" })).toBeVisible();
  await page.getByRole("button", { name: "Decks", exact: true }).click();
  await page.getByRole("button", { name: "Back" }).click();
  await expect(menuItem(page, "My decks")).toBeVisible();
}

test("an edited deck is saved and survives a reload", async ({ page }) => {
  await title(page);
  await editFirstDeck(page);
  const once = (await readSave(page)).decks;
  expect(once?.length).toBeGreaterThan(0);

  // A second edit changes the saved list again.
  await editFirstDeck(page);
  const twice = (await readSave(page)).decks;
  expect(twice).not.toEqual(once);

  await page.reload();
  await expect(menuItem(page, "My decks")).toBeVisible();
  expect((await readSave(page)).decks).toEqual(twice);
});
