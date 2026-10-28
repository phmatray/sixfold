import { expect, test, type Page } from "@playwright/test";
import { frames, menuItem, readSave, title } from "./helpers";

// The outdoor camera angle (#148): C cycles it, Options picks it, the save keeps it per slot; rooms keep 50°.
type Cam = { angle: string; pitch: number };
const cam = (page: Page): Promise<Cam> => page.evaluate(() => (document.querySelector(".world-host") as any)?.sixfoldStats?.().camera);
const settled = (page: Page) => cam(page).then((c) => c && `${c.angle} ${Math.round(c.pitch)}`);
const warp = (page: Page, map: string, x: number, z: number, f: string) =>
  page.evaluate(([m, x, z, f]) => (document.querySelector(".world-host") as any).sixfoldWarp(m, x, z, f), [map, x, z, f] as const);
/** Two drawn frames, then the camera. */
const twoFrames = (page: Page): Promise<Cam> =>
  page.evaluate(
    () =>
      new Promise((done) =>
        requestAnimationFrame(() => requestAnimationFrame(() => done((document.querySelector(".world-host") as any).sixfoldStats().camera))),
      ),
  );

async function enterWorld(page: Page) {
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
  // In front of the meadow's first bench: its prompt shows once the warp has faded in.
  await warp(page, "overworld", 22, 17, "n");
  await expect(page.getByRole("button", { name: /^Sit/ })).toBeVisible();
  await expect.poll(() => settled(page)).toBe("default 50");
}

test("C cycles the outdoor camera angle, the save keeps it, and rooms keep their own", async ({ page }) => {
  await enterWorld(page);
  await page.keyboard.press("c");
  await expect.poll(() => settled(page)).toBe("higher 60");
  await expect(page.locator("p.sr-only[aria-live=polite]").filter({ hasText: "Camera: higher" })).toHaveCount(1);
  await page.keyboard.press("c");
  await expect.poll(() => settled(page)).toBe("lower 40");
  await expect.poll(async () => (await readSave(page)).camera).toEqual(["lower"]);

  // A reload starts at the saved angle.
  await page.reload();
  await menuItem(page, "Continue").click();
  await expect(page.getByRole("status").filter({ hasText: "Loading the world" })).toBeHidden({ timeout: 60_000 });
  await expect.poll(() => settled(page)).toBe("lower 40");

  // Indoors the room keeps 50°; back outdoors, the angle is in force again.
  await warp(page, "shop", 5, 7, "n");
  await expect.poll(() => settled(page)).toBe("lower 50");
  await warp(page, "overworld", 22, 17, "n");
  await expect.poll(() => settled(page)).toBe("lower 40");
  await page.keyboard.press("c");
  await expect.poll(() => settled(page)).toBe("default 50");
});

test("a new angle eases in, and cuts under reduced motion", async ({ page }) => {
  await enterWorld(page);
  // Reduced motion (the suite's default): the pitch is there at once.
  await page.keyboard.press("c");
  expect(Math.round((await twoFrames(page)).pitch)).toBe(60);
  // Without it, two frames in the pitch is on its way.
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.keyboard.press("c");
  const mid = (await twoFrames(page)).pitch;
  expect(mid).toBeLessThan(60);
  expect(mid).toBeGreaterThan(40);
  await expect.poll(() => settled(page)).toBe("lower 40");
});

test("Options picks the camera angle, saved, and the title's Settings show it", async ({ page }) => {
  await enterWorld(page);
  await frames(page);
  await page.getByRole("button", { name: "Menu", exact: true }).click();
  await page.getByRole("tab", { name: "Options" }).click();
  await expect(page.getByRole("radio", { name: "Default" })).toBeChecked();
  await page.getByRole("radio", { name: "Higher" }).check();
  await page.getByRole("button", { name: "Close", exact: true }).click();
  await expect.poll(() => settled(page)).toBe("higher 60");
  await expect.poll(async () => (await readSave(page)).camera).toEqual(["higher"]);

  await page.reload();
  await page.locator("summary", { hasText: "Settings" }).click();
  const settings = page.getByRole("dialog", { name: "Settings" });
  await expect(settings.getByRole("group", { name: "Camera angle" })).toBeVisible();
  await expect(settings.getByRole("radio", { name: "Higher" })).toBeChecked();
  // The Controls list it too, bound to C (and R3 on a pad), and it can be rebound.
  await expect(settings.getByRole("button", { name: "Camera angle: C" })).toBeVisible();
});

test("indoors C changes nothing, and says nothing", async ({ page }) => {
  await enterWorld(page);
  await warp(page, "shop", 5, 7, "n");
  await expect.poll(() => settled(page)).toBe("default 50");
  await page.keyboard.press("c");
  await frames(page);
  expect(await settled(page)).toBe("default 50");
  await expect(page.locator("p.sr-only[aria-live=polite]").filter({ hasText: "Camera:" })).toHaveCount(0);
  expect((await readSave(page)).camera ?? []).not.toEqual(["higher"]);
});

test("a reset leaves the Options radio and the next C on the loaded save's angle", async ({ page }) => {
  await enterWorld(page);
  await frames(page);
  await page.getByRole("button", { name: "Menu", exact: true }).click();
  await page.getByRole("tab", { name: "Options" }).click();
  await page.getByRole("radio", { name: "Lower" }).check();
  await expect.poll(() => settled(page)).toBe("lower 40");
  // Same panel, still open: the reset loads a new player, whose angle is the default.
  await page.getByRole("button", { name: "Reset progress…" }).click();
  await page.getByRole("button", { name: "Reset progress", exact: true }).click();
  await expect(page.getByRole("radio", { name: "Default" })).toBeChecked();
  await expect.poll(() => settled(page)).toBe("default 50");
  await page.getByRole("button", { name: "Close", exact: true }).click();
  await page.keyboard.press("c");
  await expect.poll(() => settled(page)).toBe("higher 60");
  await expect.poll(async () => (await readSave(page)).camera).toEqual(["higher"]);
});

test("the title's Settings keep the angle in memory and create no save", async ({ page }) => {
  await title(page);
  await page.locator("summary", { hasText: "Settings" }).click();
  const settings = page.getByRole("dialog", { name: "Settings" });
  await settings.getByRole("radio", { name: "Higher" }).check();
  await expect(settings.getByRole("radio", { name: "Higher" })).toBeChecked();
  expect(await page.evaluate(() => localStorage.getItem("sixfold.save.1"))).toBeNull();
});
