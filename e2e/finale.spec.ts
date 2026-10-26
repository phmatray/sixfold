import { expect, test, type Page } from "@playwright/test";
import { menuItem, readSave, title } from "./helpers";

/**
 * Rewrite slot 1 the way the game stores it: `sixfold-save;1;<fnv1a>` over the
 * section lines (engine `save.rs`). `edit` changes the sections in place.
 */
async function editSave(page: Page, edit: string) {
  await page.evaluate((edit) => {
    const fnv = (s: string) => {
      let h = 0x811c9dc5;
      for (const b of new TextEncoder().encode(s)) h = Math.imul(h ^ b, 0x01000193) >>> 0;
      return h >>> 0;
    };
    const secs: Record<string, string[]> = {};
    let cur: string[] = [];
    for (const l of (localStorage.getItem("sixfold.save.1") ?? "").split("\n").slice(1)) {
      const m = /^\[([a-z0-9_]+)\]$/.exec(l);
      if (m) secs[m[1]] = cur = [];
      else cur.push(l);
    }
    new Function("secs", edit)(secs);
    const body = Object.entries(secs).flatMap(([k, v]) => [`[${k}]`, ...v]).join("\n");
    localStorage.setItem("sixfold.save.1", `sixfold-save;1;${fnv(body).toString(16).padStart(8, "0")}\n${body}`);
    localStorage.removeItem("sixfold.save.1.prev");
  }, edit);
}

async function loaded(page: Page) {
  await expect(page.getByRole("button", { name: "Menu", exact: true })).toBeVisible();
  await expect(page.getByRole("status").filter({ hasText: "Loading the world" })).toBeHidden({ timeout: 60_000 });
}

test("the finale: back from the first Warden win, the scene, the credits, and the champion in the hall", async ({ page }) => {
  page.on("dialog", (d) => d.accept());
  await title(page);
  await menuItem(page, "Learn to play").click();
  await page.getByRole("button", { name: "Skip the lesson" }).click();
  await expect(menuItem(page, "Continue")).toBeVisible();

  // Six seals and a Warden duel just won (what `campaign::record` leaves), in the Warden's room.
  const seals = ["bramble", "marlo", "cinza", "quill", "orla", "lumen", "warden"].map((r) => `f:beat_${r}`);
  await editSave(
    page,
    `secs.progress = [(secs.progress?.[0] ?? "") + ",${seals.join(",")},f:intro,won:warden"]; secs.world_at = ["warden,5,4,n"];`,
  );
  await page.reload();
  await menuItem(page, "Continue").click();
  await loaded(page);

  // The scene plays instead of Sable's post-duel line; Escape skips it.
  await expect(page.getByRole("dialog", { name: "Sable" })).toContainText("Every seal stone");
  await page.keyboard.press("Escape");

  // The credits open on the seal stone, then the title; Next moves on.
  const credits = page.getByRole("main", { name: "Credits" });
  await expect(credits.getByText("Six faces, all lit.", { exact: true })).toBeVisible();
  await credits.getByRole("button", { name: "Next" }).click();
  await expect(credits.getByRole("heading", { name: "Champion", exact: true })).toBeVisible();

  // Skip: the champion stands in the duel hall by the sealed door, saved, the finale done.
  await page.keyboard.press("Escape");
  await loaded(page);
  await expect.poll(async () => (await readSave(page)).world_at?.[0]).toBe("shop,1,6,e");
  const progress = (await readSave(page)).progress?.[0] ?? "";
  expect(progress).toContain("f:finale");
  expect(progress, "the scene settles the duel it spoke for").not.toContain("won:warden");
  await expect(page.locator("[data-hero]")).toHaveAttribute("data-hero", "1,6,e");

  // Once only: a reload comes back to the hall with no scene.
  await page.reload();
  await menuItem(page, "Continue").click();
  await loaded(page);
  await expect(page.locator("[data-hero]")).toHaveAttribute("data-hero", "1,6,e");
  await expect(page.getByRole("button", { name: /^Skip/ })).toBeHidden();
});

test("the credits from the campaign: no finale, every plate by hand, and back to the campaign", async ({ page }) => {
  page.on("dialog", (d) => d.accept());
  await title(page);
  await menuItem(page, "Learn to play").click();
  await page.getByRole("button", { name: "Skip the lesson" }).click();
  await expect(menuItem(page, "Continue")).toBeVisible();
  // A champion (the button is theirs) who has seen the finale, out on the meadow.
  const beaten = ["bramble", "marlo", "cinza", "quill", "orla", "lumen", "warden"].map((r) => `f:beat_${r}`);
  await editSave(page, `secs.progress = [(secs.progress?.[0] ?? "") + ",${beaten.join(",")},f:intro,f:finale"]; secs.world_at = ["overworld,25,13,s"];`);
  await page.reload();
  const before = (await readSave(page)).world_at?.[0];
  await menuItem(page, "Campaign").click();
  await page.getByRole("button", { name: "Watch the credits" }).click();

  // No seal stone or title: they open on the wordmark. With reduced motion they wait for Next.
  const credits = page.getByRole("main", { name: "Credits" });
  await expect(credits.getByRole("heading", { name: "Sixfold", level: 1 })).toBeVisible();
  await expect(credits.getByText("Six faces, all lit.", { exact: true })).toBeHidden();
  const next = credits.getByRole("button", { name: "Next" });
  while (await next.isVisible()) await next.click();

  // The last plate: Continue has the focus (Next is gone), and leads back to the campaign.
  await expect(credits.getByText("Thank you for playing.", { exact: true })).toBeVisible();
  await expect(credits.getByRole("button", { name: "Continue" })).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { name: "Campaign", level: 1 })).toBeVisible();
  expect((await readSave(page)).world_at?.[0]).toEqual(before);
});
