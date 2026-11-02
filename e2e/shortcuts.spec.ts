import { expect, test, loaded, menuItem, readSave, title, warpTo, type Page } from "./helpers";

// #117 on Scree Pass: the barred stair (gate scree_bar at 10,25, lifted from 10,24 above) and the drop at the broken
// fence (18,18 down to 18,19).

/** Where the save says the hero stands: `map,x,z,facing`. */
const at = async (page: Page) => (await readSave(page)).world_at?.[0] ?? "";

/** Hold an arrow key until the saved position matches `want` (600 frames at most, as world.spec.ts walks). */
async function walk(page: Page, key: string, want: RegExp) {
  await page.keyboard.down(key);
  try {
    await page.evaluate(() => ((window as unknown as { walked: number }).walked = 0));
    await page.waitForFunction(
      ([src, max]) => {
        const at = /\[world_at\]\n(.*)/.exec(localStorage.getItem("sixfold.save.1") ?? "")?.[1] ?? "";
        if (new RegExp(src).test(at)) return true;
        const w = window as unknown as { walked: number };
        if (++w.walked > max) throw new Error(`still at "${at}" after ${max} frames`);
        return false;
      },
      [want.source, 600] as const,
      { polling: "raf", timeout: 0 },
    );
  } finally {
    await page.keyboard.up(key);
  }
}

/** A new game in the world, its intro scene skipped. */
async function explore(page: Page) {
  await title(page);
  await menuItem(page, "Explore the world").click();
  await loaded(page);
  const skip = page.getByRole("button", { name: /^Skip/ });
  await expect(skip).toBeVisible();
  await page.waitForTimeout(450); // a scene ignores a skip in its first 400 ms (#155)
  await page.keyboard.press("Escape");
  await expect(skip).toBeHidden();
}

test("AC4 (#117): a shortcut is barred from the near side, lifted from the far side, and stays open after a reload", async ({ page }) => {
  await explore(page);
  // From below, on the stair: the gate only reads its text.
  await warpTo(page, "screepass", 10, 26, "n");
  await expect(page.getByRole("button", { name: /^Look at the barred stair/ })).toBeVisible();
  await page.keyboard.press("e");
  const note = page.getByRole("dialog").filter({ hasText: "No way through" });
  await expect(note).toContainText("Barred from the other side.");
  await page.keyboard.press("Escape");
  await expect(note).toBeHidden();

  // Not lifted yet: the world map marks the pass's drop (it is seen), but no shortcut.
  await page.keyboard.press("m");
  const chart = page.getByRole("dialog").filter({ hasText: "World map" });
  await expect(chart.locator('.atlas-mark[data-mark="drop"]')).toBeVisible();
  await expect(chart.locator('.atlas-mark[data-mark="shortcut"]')).toHaveCount(0);
  // #240: what a mark means is written in the page, and no mark covers a name, a crystal or the pin.
  await expect(chart.locator(".atlas-legend")).toContainText("a one-way drop");
  for (const vp of [{ width: 1440, height: 900 }, { width: 1280, height: 800 }, { width: 390, height: 844 }]) {
    await page.setViewportSize(vp);
    const hits = await chart.evaluate((root) => {
      const box = (e: Element) => e.getBoundingClientRect();
      const hit = (a: DOMRect, b: DOMRect) => a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom;
      const others = [...root.querySelectorAll(".atlas-label, .atlas-posts > *, .atlas-you")];
      return [...root.querySelectorAll(".atlas-mark")].filter((m) => {
        const own = m.closest(".atlas-label");
        return others.some((o) => o !== own && hit(box(m), box(o)));
      }).length;
    });
    expect(hits, `${vp.width}x${vp.height}`).toBe(0);
  }
  await page.keyboard.press("Escape");
  await expect(chart).toBeHidden();

  // From the terrace above: lift the bar.
  await warpTo(page, "screepass", 10, 24, "s");
  await expect(page.getByRole("button", { name: /^Lift the bar/ })).toBeVisible();
  await page.keyboard.press("e");
  const talk = page.locator(".talk-box");
  await expect(talk).toBeVisible();
  while (await talk.isVisible()) {
    await page.keyboard.press("Enter");
    await page.waitForTimeout(400);
  }
  await expect.poll(async () => (await readSave(page)).progress?.join("\n") ?? "").toContain("sc_scree_bar");

  // Saved: after a reload the way down the stair is open.
  await page.reload();
  await menuItem(page, "Continue").click();
  await loaded(page);
  expect(await at(page)).toBe("screepass,10,24,s");
  await walk(page, "ArrowDown", /^screepass,10,2[56],/);

  // And the world map marks it, with the pass's drop.
  await page.keyboard.press("m");
  await expect(chart.locator('.atlas-mark[data-mark="shortcut"]')).toBeVisible();
  await expect(chart.locator('.atlas-mark[data-mark="drop"]')).toBeVisible();
});

test("AC5 (#117): the hero steps down a drop and can't step back up", async ({ page }) => {
  await explore(page);
  await warpTo(page, "screepass", 18, 18, "s");
  await walk(page, "ArrowDown", /^screepass,18,19,/);
  // Back up: the hero turns, but stays below.
  await page.keyboard.down("ArrowUp");
  await expect.poll(() => at(page)).toBe("screepass,18,19,n");
  await page.waitForTimeout(1500);
  await page.keyboard.up("ArrowUp");
  expect(await at(page)).toBe("screepass,18,19,n");
});
