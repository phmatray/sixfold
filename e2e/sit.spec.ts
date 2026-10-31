import { expect, test, frames, menuItem, readSave, title, type Page } from "./helpers";

/** Where the save says the hero stands: `map,x,z,facing`. */
const at = async (page: Page) => (await readSave(page)).world_at?.[0] ?? "";
const sitting = (page: Page) => page.evaluate(() => (document.querySelector(".world-host") as any)?.sixfoldStats?.().sitting);

test("sit on a bench: Sit, stand up with Escape or a move, and a reload stands in front of it", async ({ page }) => {
  page.on("dialog", (d) => d.accept());
  await title(page);
  await menuItem(page, "Learn to play").click();
  await page.getByRole("button", { name: "Skip the lesson" }).click();
  await menuItem(page, "Continue").click();
  await expect(page.getByRole("status").filter({ hasText: "Loading the world" })).toBeHidden({ timeout: 60_000 });
  const skip = page.getByRole("button", { name: /^Skip/ });
  await expect(skip).toBeVisible();
  // A scene ignores a skip in its first 400 ms (#155).
  await page.waitForTimeout(450);
  await page.keyboard.press("Escape");
  await expect(skip).toBeHidden();

  // In front of the meadow's first bench (22..23, 16), facing it.
  await page.evaluate(() => (document.querySelector(".world-host") as any).sixfoldWarp("overworld", 22, 17, "n"));
  await expect.poll(() => at(page)).toBe("overworld,22,17,n");
  const sit = page.getByRole("button", { name: /^Sit/ });
  const up = page.getByRole("button", { name: /^Stand up/ });

  // E sits; Escape stands up (and leaves the menu shut).
  await expect(sit).toBeVisible();
  await page.keyboard.press("e");
  await expect.poll(() => sitting(page)).toBe(true);
  await expect(up).toBeVisible();
  await expect.poll(() => at(page)).toBe("overworld,22,17,s"); // the tile in front stays the hero's
  await page.keyboard.press("Escape");
  await expect.poll(() => sitting(page)).toBe(false);
  await expect(page.getByRole("dialog", { name: "Menu" })).toBeHidden();

  // Facing away now: turn back (held until a frame sees it; the bench stops the step), sit with the prompt,
  // and a move key stands up and walks on.
  await page.keyboard.down("ArrowUp");
  await expect.poll(() => at(page)).toBe("overworld,22,17,n");
  await page.keyboard.up("ArrowUp");
  await expect(sit).toBeVisible();
  await frames(page);
  await sit.click();
  await expect.poll(() => sitting(page)).toBe(true);
  await page.keyboard.down("ArrowLeft");
  await expect.poll(() => sitting(page)).toBe(false);
  await page.keyboard.up("ArrowLeft");
  await expect.poll(() => at(page)).toMatch(/^overworld,\d+,17,w$/);

  // Seated, a reload stands the hero on the tile in front of the bench.
  await page.evaluate(() => (document.querySelector(".world-host") as any).sixfoldWarp("overworld", 22, 17, "n"));
  await expect.poll(() => at(page)).toBe("overworld,22,17,n");
  await expect(sit).toBeVisible(); // the warp has faded in: E acts again
  await page.keyboard.press("e");
  await expect.poll(() => sitting(page)).toBe(true);
  // Sitting turns the hero at once, but the turn is saved on the next frame: wait for the save, then reload.
  await expect.poll(() => at(page)).toBe("overworld,22,17,s");
  await page.reload();
  await menuItem(page, "Continue").click();
  await expect(page.getByRole("status").filter({ hasText: "Loading the world" })).toBeHidden({ timeout: 60_000 });
  await expect(page.locator("[data-hero]")).toHaveAttribute("data-hero", "22,17,s");
  expect(await sitting(page)).toBe(false);
});

test("a find taken from under a bench gives the place back to sitting", async ({ page }) => {
  page.on("dialog", (d) => d.accept());
  await title(page);
  await menuItem(page, "Learn to play").click();
  await page.getByRole("button", { name: "Skip the lesson" }).click();
  await menuItem(page, "Continue").click();
  await expect(page.getByRole("status").filter({ hasText: "Loading the world" })).toBeHidden({ timeout: 60_000 });
  const skip = page.getByRole("button", { name: /^Skip/ });
  await expect(skip).toBeVisible();
  // A scene ignores a skip in its first 400 ms (#155).
  await page.waitForTimeout(450);
  await page.keyboard.press("Escape");
  await expect(skip).toBeHidden();

  // Brightwell Square: Facets stuck under the bench by the fountain (21, 20), faced from the north.
  await page.evaluate(() => (document.querySelector(".world-host") as any).sixfoldWarp("square", 21, 19, "s"));
  await expect.poll(() => at(page)).toBe("square,21,19,s");
  const look = page.getByRole("button", { name: /^Take a look/ });
  await expect(look).toBeVisible();
  await page.keyboard.press("e");
  await expect(page.locator("#talk-live")).toHaveText(/stuck to the slats/);
  const note = page.getByRole("button", { name: /^A note/ });
  await note.click(); // one line: it closes
  await expect(note).toBeHidden();
  // Taken: the place is a seat again.
  await expect(page.getByRole("button", { name: /^Sit/ })).toBeVisible();
  await page.keyboard.press("e");
  await expect.poll(() => sitting(page)).toBe(true);
});
