import { expect, test } from "@playwright/test";
import { menuItem, readSave, title } from "./helpers";

// A book from the duel hall's shelf, read with the keyboard only (#104): the prompt names it,
// E opens it, the arrows turn its pages (the page's text is in the DOM for screen readers),
// Escape puts it back, and the Journal's Library keeps it to read again.
test("read a book from the duel hall shelf by keyboard", async ({ page }) => {
  page.on("dialog", (d) => d.accept());
  await title(page);
  await menuItem(page, "Learn to play").click();
  await page.getByRole("button", { name: "Skip the lesson" }).click();
  await menuItem(page, "Continue").click();
  await expect(page.getByRole("status").filter({ hasText: "Loading the world" })).toBeHidden({ timeout: 60_000 });
  const skip = page.getByRole("button", { name: /^Skip/ });
  await expect(skip).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(skip).toBeHidden();

  // In front of the first shelf, facing it.
  await page.locator(".world-host").evaluate((el) => (el as unknown as { sixfoldWarp: Function }).sixfoldWarp("shop", 1, 2, "n"));
  await expect(page.locator("[data-hero]")).toHaveAttribute("data-hero", "1,2,n");
  await expect(page.getByRole("button", { name: /^Read “The Duel, Turn by Turn”/ })).toBeVisible();

  await page.keyboard.press("e");
  const book = page.getByRole("dialog", { name: "The Duel, Turn by Turn" });
  await expect(book).toBeVisible();
  await expect(book.getByText("Page 1 of 5")).toBeVisible();
  const text = book.locator(".reading-text");
  await expect(text.getByRole("heading", { name: "A deck of sixty" })).toBeAttached();
  await expect(book.getByRole("button", { name: /Next/ })).toBeFocused();

  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("ArrowRight");
  await expect(book.getByText("Page 3 of 5")).toBeVisible();
  await expect(text.getByRole("heading", { name: "Setting the table" })).toBeAttached();
  await page.keyboard.press("ArrowLeft");
  await expect(book.getByText("Page 2 of 5")).toBeVisible();
  await page.keyboard.press("End");
  await expect(book.getByText("Page 5 of 5")).toBeVisible();

  await page.keyboard.press("Escape");
  await expect(book).toBeHidden();
  // Read, in the save.
  await expect.poll(async () => (await readSave(page)).progress?.join(",") ?? "").toContain("f:read_duel");

  // The Journal (J) lists it in its Library, and opens it again.
  await expect(page.getByRole("button", { name: /^Read “The Duel/ })).toBeVisible();
  await page.keyboard.press("j");
  const journal = page.getByRole("dialog", { name: "Journal" });
  await expect(journal.getByText("1 of 11 found")).toBeVisible();
  await journal.getByRole("button", { name: /The Duel, Turn by Turn/ }).click();
  await expect(book).toBeVisible();
  await expect(book.getByText("Page 1 of 5")).toBeVisible();
});
