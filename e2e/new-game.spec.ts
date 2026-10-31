import { expect, test, menuItem, readSave } from "./helpers";

test("a new player opens the game, plays the guided duel and comes back to Continue @webkit", async ({ page }) => {
  // A fresh browser: the studio opening first.
  await page.goto("/");
  // The prompt takes focus once the studio and the logo have played (a press before that only skips ahead).
  const press = page.getByRole("button", { name: "Press to start" });
  await expect(press).toBeFocused();
  await press.click();
  await menuItem(page, "Learn to play").click();

  // The guided duel: the coach rings one control per step; "Show me" focuses it, Enter plays it.
  const won = page.getByRole("dialog", { name: "You win" });
  const showMe = page.getByRole("button", { name: "Show me" });
  const lesson = page.getByRole("region", { name: "Lesson" }).getByRole("status");
  const held = page.getByRole("list", { name: /^Your hand/ }).getByRole("button", { pressed: true });
  // Where the lesson is: its text, and whether a card is held (picking one doesn't change the text).
  const state = async () => `${await lesson.textContent()}|${await held.count()}`;
  await expect(lesson).toContainText("Step 1 of 9");
  for (let i = 0; i < 40; i++) {
    await expect(showMe.or(won).first()).toBeVisible({ timeout: 30_000 });
    if (await won.isVisible()) break;
    const before = await state();
    await showMe.click();
    await page.keyboard.press("Enter");
    // Wait for the step to move on (or the duel to end) before looking again.
    await expect(async () => {
      expect((await won.isVisible()) || (await state()) !== before).toBe(true);
    }).toPass({ timeout: 30_000 });
  }
  await expect(won).toBeVisible();
  await expect.poll(async () => (await readSave(page)).tutorial).toEqual(["done"]);

  // The lesson's gift: packs to open; from there, back to the title.
  await won.getByRole("button", { name: "Open my packs" }).click();
  await expect(page.getByRole("heading", { level: 1, name: "Open packs" })).toBeVisible();
  // Back to the title, which builds the world behind it (slow on CI's software GL): the click
  // doesn't wait on that, and the menu gets as long as the title's first load.
  await page.getByRole("button", { name: "Back" }).click({ noWaitAfter: true });
  await expect(menuItem(page, "Continue")).toBeVisible({ timeout: 30_000 });
  await expect(menuItem(page, "Learn to play")).toHaveCount(0);
});
