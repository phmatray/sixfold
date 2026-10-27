import { expect, test, type Page } from "@playwright/test";
import { editSave, loaded, menuItem, title } from "./helpers";

/** Talk to Marlo from the tile south of him, through his lines, to his challenge sheet. */
async function talkToMarlo(page: Page) {
  await expect(page.locator("[data-hero]")).toHaveAttribute("data-hero", /^22,16/, { timeout: 15_000 });
  await expect(page.getByRole("button", { name: /^Talk to Marlo/ })).toBeVisible();
  await page.keyboard.press("e");
  // The map's first talk draws the portrait in a new WebGL context: slow on CI's software GL.
  const talk = page.locator(".talk-box");
  await expect(talk).toBeVisible({ timeout: 60_000 });
  while (await talk.isVisible()) {
    await page.keyboard.press("Enter");
    await page.waitForTimeout(400);
  }
  const offer = page.getByRole("dialog", { name: "Challenge Marlo?" });
  await expect(offer).toBeVisible();
  return offer;
}

/** Back into the world beside Marlo at the Bellflats, with these flags added to the save. */
async function atMarlo(page: Page, flags: string[]) {
  await editSave(
    page,
    `secs.progress = [(secs.progress?.[0] ?? "") + ",${flags.join(",")}"]; secs.world_at = ["bellflats,22,16,n"];`,
  );
  await page.reload();
  await menuItem(page, "Continue").click();
  await loaded(page);
}

// #150: a seal keeper accepts only once enough of their region's duelists are bested.
test("Marlo refuses until three of the Saltreach's duelists are bested, and the Journal counts them", async ({ page }) => {
  page.on("dialog", (d) => d.accept());
  await title(page);
  await menuItem(page, "Learn to play").click();
  await page.getByRole("button", { name: "Skip the lesson" }).click();
  await expect(menuItem(page, "Continue")).toBeVisible();

  // Two of the Saltreach's duelists: Marlo refuses, in his own words, and counts them.
  // `f:keepers`: a save of this build, so not vouched for as an old one (which would earn every keeper).
  await atMarlo(page, ["f:keepers", "f:intro", "f:beat_t_tobin", "f:beat_bramble", "f:beat_t_pell", "f:beat_t_nell"]);
  const offer = await talkToMarlo(page);
  await expect(offer).toContainText("The tide doesn't turn for strangers.");
  await expect(offer).toContainText("Marlo wants 3 of the Saltreach's duelists bested first. You've bested 2.");
  await expect(offer.getByRole("img", { name: "2 of 3" })).toBeVisible();
  await expect(offer.getByRole("button", { name: /^Challenge/ })).toHaveCount(0);
  await expect(offer.getByRole("button", { name: /^Not now/ })).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(offer).toBeHidden();

  // The Journal's next step names the duelists still needed.
  await page.keyboard.press("j");
  const journal = page.getByRole("dialog", { name: "Journal" });
  await expect(journal).toContainText("Beat 1 more duelist of the Saltreach to earn Marlo's challenge (2 of 3).");
  await page.keyboard.press("Escape");
  await expect(journal).toBeHidden();
  // The menu's Campaign roster counts them too.
  await page.getByRole("button", { name: "Menu", exact: true }).click();
  await page.getByRole("tab", { name: "Campaign" }).click();
  await expect(page.locator("li.rival").filter({ hasText: "Marlo" })).toContainText("Duelists bested: 2 of 3");
  await page.keyboard.press("Escape");
  await expect(page.locator("li.rival")).toHaveCount(0);

  // A third: the duel is on offer.
  await atMarlo(page, ["f:beat_t_skye"]);
  const again = await talkToMarlo(page);
  await expect(again.getByRole("button", { name: /^Challenge/ })).toBeVisible();
  await expect(again).not.toContainText("bested first");
});
