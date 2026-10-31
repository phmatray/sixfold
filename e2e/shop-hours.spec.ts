import { expect, test, menuItem, readSave, title, type Page } from "./helpers";

/** Shop hours (#114): a door's `open` keeps it shut out of hours, with its closed text and no keeper inside. */
const host = (page: Page, f: string, ...args: unknown[]) => page.evaluate(([f, args]) => (document.querySelector(".world-host") as any)[f as string](...(args as unknown[])), [f, args] as const);
const at = async (page: Page) => (await readSave(page)).world_at?.[0] ?? "";
const people = (page: Page) => page.evaluate(() => ((document.querySelector(".world-host") as any).sixfoldStats().people as { id: string }[]).map((p) => p.id));
// A warp asked while a fade is still up is dropped: asked until the save says so.
const warp = (page: Page, map: string, x: number, z: number, f: string) =>
  expect.poll(async () => (await host(page, "sixfoldWarp", map, x, z, f), at(page)), { timeout: 30_000, intervals: [1500] }).toBe(`${map},${x},${z},${f}`);

test("AC5: at 21:00 the bakery door reads its closed text and Hob has gone home; at 09:00 it opens", async ({ page }) => {
  test.slow(); // warps and fades at ~1 frame a second on CI's software GL
  page.on("dialog", (d) => d.accept());
  await title(page);
  await menuItem(page, "Learn to play").click();
  await page.getByRole("button", { name: "Skip the lesson" }).click();
  await menuItem(page, "Continue").click();
  await expect(page.getByRole("status").filter({ hasText: "Loading the world" })).toBeHidden({ timeout: 60_000 });
  const skip = page.getByRole("button", { name: /^Skip/ });
  await expect(skip).toBeVisible();
  await page.waitForTimeout(450); // a scene ignores a skip in its first 400 ms (#155)
  await page.keyboard.press("Escape");
  await expect(skip).toBeHidden();

  // Nobody behind the counter at 21:00.
  await host(page, "sixfoldClock", 21 * 60);
  await warp(page, "bakery", 5, 7, "n");
  await expect.poll(() => people(page)).not.toContain("hob");

  // In front of the bakery door (Brightwell Square), facing it: closed, and pressing E doesn't go in.
  await warp(page, "square", 19, 9, "n");
  await expect(page.getByRole("button", { name: "Try the door" })).toBeVisible();
  await page.keyboard.press("e");
  await expect(page.getByText("Closed. Back at eight.")).toBeVisible();
  expect(await at(page)).toBe("square,19,9,n");
  await page.keyboard.press("Escape");
  await expect(page.getByText("Closed. Back at eight.")).toBeHidden();

  // At 09:00 it opens, and Hob is in.
  await host(page, "sixfoldClock", 9 * 60);
  await expect(page.getByRole("button", { name: "Go into Hob's Bakery" })).toBeVisible();
  await page.keyboard.press("e");
  await expect.poll(() => at(page), { timeout: 60_000 }).toMatch(/^bakery,/);
  await expect.poll(() => people(page)).toContain("hob");
});
