import { expect, test } from "@playwright/test";
import { menuItem, title } from "./helpers";

test("a turning page covers the page under it: no HP badge shows through", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "no-preference" }); // the leaf only turns with motion on
  await page.clock.install();
  await title(page);
  await menuItem(page, "Collection").click();
  await expect(page.locator(".bind-card .card").first()).toBeVisible();
  // Hold the timer that lifts the leaf, then stop the turn half-way.
  await page.clock.pauseAt((await page.evaluate(() => Date.now())) + 1000);
  await page.getByRole("button", { name: "Next page" }).click();
  await expect(page.locator(".leaf")).toBeAttached();
  const through = await page.evaluate(() => {
    for (const a of document.getAnimations())
      if ((a as CSSAnimation).animationName?.startsWith("leaf")) { a.pause(); a.currentTime = 250; }
    const leaf = document.querySelector<HTMLElement>(".leaf")!;
    // Hit-test as if the leaf took the pointer: what is on top at each HP under it.
    leaf.removeAttribute("inert");
    leaf.style.pointerEvents = "auto";
    const box = leaf.getBoundingClientRect();
    return [...document.querySelectorAll(".sheets > .sheet .card-hp")].flatMap((hp) => {
      const r = hp.getBoundingClientRect();
      const x = r.left + r.width / 2, y = r.top + r.height / 2;
      if (!r.width || x < box.left || x > box.right || y < box.top || y > box.bottom) return [];
      const top = document.elementFromPoint(x, y);
      return top && !leaf.contains(top) ? [hp.textContent] : [];
    });
  });
  expect(through).toEqual([]);
});
