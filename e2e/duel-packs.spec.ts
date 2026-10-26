import { expect, test } from "@playwright/test";
import { menuItem, owned, readSave, title } from "./helpers";

test("a duel against the AI is played to the end", async ({ page }) => {
  await title(page);
  await menuItem(page, "Play a duel").click();
  // One prize and a fixed seed: the same short duel every run (an engine or AI
  // change that asks the player something mid-duel would need a step here).
  await page.getByText("Seed and prizes").click();
  await page.getByLabel("Seed").fill("59");
  await page.getByLabel("Prizes").selectOption("1");
  await page.getByLabel(/^Opponent/).selectOption("hard");
  await page.getByRole("button", { name: "Start the duel" }).click();
  await expect(page.getByRole("heading", { level: 1, name: /^Duel against/ })).toBeAttached();

  // A Basic to the Active spot, nothing on the Bench, then only ever end the turn:
  // the AI knocks the lone Active out and wins (a release build has no autoplay).
  const hand = page.getByRole("list", { name: /^Your hand/ });
  await hand.getByRole("button", { name: /, playable$/ }).first().click();
  await page.getByRole("button", { name: /^Empty Active spot/ }).click();
  await page.getByRole("button", { name: /^Done/ }).click();

  const result = page.getByRole("dialog", { name: /^You (win|lose)$/ });
  const end = page.getByRole("button", { name: /^(End turn|End anyway)/ });
  for (let i = 0; i < 60 && !(await result.isVisible()); i++) {
    await expect(result.or(end.and(page.locator("[aria-disabled=false]"))).first()).toBeVisible({ timeout: 60_000 });
    if (await result.isVisible()) break;
    await end.click();
  }
  await expect(result).toHaveAccessibleName("You lose");
  await expect(result.getByRole("heading", { name: "Why you lost" })).toBeVisible();
});

test("opening packs adds cards to the collection @phone", async ({ page }) => {
  await title(page);
  await menuItem(page, "Open packs").click();
  await expect(page.getByRole("heading", { level: 1, name: "Open packs" })).toBeVisible();

  // A new player holds three packs, one of each of three kinds.
  const open = async () => {
    await page.getByRole("button", { name: /^Open one .*, 1 left$/ }).first().click();
    await page.getByRole("button", { name: "Tear it open" }).click();
    // With reduced motion the cards turn over at once.
    await expect(page.getByRole("button", { name: "See the collection" })).toBeVisible();
  };
  await open();
  const first = await readSave(page);
  // "Choose another" goes back to the shelf.
  await page.getByRole("button", { name: "Choose another (2 left)" }).click();
  await open();
  const second = await readSave(page);
  expect(owned(second)).toBeGreaterThan(owned(first));
  expect(second.wallet).not.toEqual(first.wallet);
});
