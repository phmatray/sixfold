import { expect, test, editSave, loaded, menuItem, title, warpTo as warp, type Page } from "./helpers";

/** Reactions (#114): people glance at the hero, and say a line in passing. Read through `sixfoldStats().people`. */
type Person = { id: string; x: number; z: number; face: string };
const who = (page: Page, id: string) =>
  page.evaluate((id) => ((document.querySelector(".world-host") as any).sixfoldStats().people as Person[]).find((p) => p.id === id), id);
const host = (page: Page, f: string, ...args: unknown[]) => page.evaluate(([f, args]) => (document.querySelector(".world-host") as any)[f as string](...(args as unknown[])), [f, args] as const);

/** In the world with three seals (and their scenes already played), at noon in Brightwell Square. */
async function threeSeals(page: Page) {
  page.on("dialog", (d) => d.accept());
  await title(page);
  await menuItem(page, "Learn to play").click();
  await page.getByRole("button", { name: "Skip the lesson" }).click();
  await expect(menuItem(page, "Continue")).toBeVisible();
  const flags = ["intro", "rook_1", ...["bramble", "marlo", "cinza"].flatMap((r) => [`beat_${r}`, `sealed_${r}`])].map((f) => `f:${f}`);
  await editSave(page, `secs.progress = [(secs.progress?.[0] ?? "") + ",${flags.join(",")}"]; secs.world_at = ["square,25,10,s"];`);
  await page.reload();
  await menuItem(page, "Continue").click();
  await loaded(page);
  await host(page, "sixfoldClock", 12 * 60);
}

test("AC2: an idle townsperson turns to the hero and back once the hero has gone; a duelist doesn't turn", async ({ page }) => {
  test.slow(); // warps and fades at ~1 frame a second on CI's software GL
  await threeSeals(page);
  // Posy minds her stall facing south; the hero arrives beside her (west).
  expect(await who(page, "posy")).toMatchObject({ x: 18, z: 13, face: "s" });
  await warp(page, "square", 17, 13, "s");
  await expect.poll(async () => (await who(page, "posy"))?.face).toBe("w");
  // Three tiles off (a warp, not a walk: CI draws ~1 frame a second), she is back at her stall's way.
  await warp(page, "square", 17, 16, "s");
  await expect.poll(async () => (await who(page, "posy"))?.face).toBe("s");
  // Lark, a duelist, watches east along his line; the hero a tile south of him (beside) doesn't turn him.
  expect(await who(page, "t_lark")).toMatchObject({ x: 18, z: 17, face: "e" });
  await warp(page, "square", 18, 18, "s");
  await page.waitForTimeout(3_000);
  expect((await who(page, "t_lark"))?.face).toBe("e");
});

test("AC3: with three seals, Moss says her line about them once as the hero passes, and the live region says it", async ({ page }) => {
  test.slow();
  await threeSeals(page);
  const live = page.locator("p.sr-only[aria-live=polite]:not(#talk-live)");
  // Every bark said, counted as the live region changes (a later announcement may replace the text).
  await live.evaluate((el) => {
    let was = false;
    (window as any).barks = 0;
    new MutationObserver(() => {
      const now = !!el.textContent?.includes("My beans are jealous");
      if (now && !was) (window as any).barks++;
      was = now;
    }).observe(el, { childList: true, characterData: true, subtree: true });
  });
  // At 14:00 Moss stands idle at 33,27 facing west: the hero comes in at the garden gate, three tiles off, facing no one.
  await host(page, "sixfoldClock", 14 * 60);
  await warp(page, "overworld", 30, 27, "n");
  await expect(live).toContainText("Moss: Three seals already? My beans are jealous."); // after the area's name, if just said
  // Away and back within the visit: not again.
  await warp(page, "square", 25, 10, "s");
  await warp(page, "overworld", 30, 27, "n");
  await page.waitForTimeout(3_000);
  expect(await page.evaluate(() => (window as any).barks)).toBe(1);
});
