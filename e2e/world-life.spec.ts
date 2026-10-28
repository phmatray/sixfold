import { expect, test, type Page } from "@playwright/test";
import { loaded, menuItem, title } from "./helpers";

// A map's ambient life (#115), read through `el.sixfoldLife()`: what life.js drew last frame.
type Life = { critters: Record<string, number>; leaves: number; embers: number; boats: { k: number; on: boolean; x: number; z: number }[]; event: string | null; calm: boolean } | null;
const life = (page: Page) => page.evaluate(() => (document.querySelector(".world-host") as any).sixfoldLife() as Life);
const host = (page: Page, call: string, ...a: unknown[]) => page.evaluate(([c, a]) => (document.querySelector(".world-host") as any)[c as string](...(a as unknown[])), [call, a] as const);

async function enter(page: Page) {
  page.on("dialog", (d) => d.accept());
  await title(page);
  await menuItem(page, "Explore the world").click();
  await loaded(page);
  const skip = page.getByRole("button", { name: /^Skip/ });
  await expect(skip).toBeVisible();
  await page.waitForTimeout(450); // a scene ignores a skip in its first 400 ms (#155)
  await page.keyboard.press("Escape");
  await expect(skip).toBeHidden();
}
const at = async (page: Page, map: string, x: number, z: number) => {
  await host(page, "sixfoldWarp", map, x, z, "s");
  await expect.poll(async () => (await host(page, "sixfoldStats")).area).toBe(map);
  await host(page, "sixfoldClock", 720); // midday
};

test.describe("with motion", () => {
  test.use({ reducedMotion: "no-preference" });

  test("Fernhollow's rabbits and leaves; Gullhaven's boat docks, and waits while a menu is open", async ({ page }) => {
    await enter(page);
    await at(page, "woodland", 45, 13);
    await expect.poll(async () => (await life(page))?.critters.rabbit ?? 0).toBeGreaterThan(0);
    await expect.poll(async () => (await life(page))?.leaves ?? 0).toBeGreaterThan(0);

    await at(page, "gullhaven", 19, 16);
    await expect.poll(async () => (await life(page))?.boats.length).toBe(1);
    await page.getByRole("button", { name: "Menu", exact: true }).click();
    await expect(page.getByRole("dialog", { name: "Menu" })).toBeVisible();
    await page.waitForTimeout(500); // the frame that opened it is the last one drawn
    const held = (await life(page))!.boats[0];
    await page.waitForTimeout(1500);
    expect((await life(page))!.boats[0]).toEqual(held);
  });

  test("a seal site's calm: nothing spawns round the seal stone", async ({ page }) => {
    await enter(page);
    await at(page, "elder_ring", 20, 17);
    await expect.poll(async () => (await life(page))?.calm).toBe(true); // a frame drawn there
    for (let i = 0; i < 10; i++) {
      const l = (await life(page))!;
      expect(l.calm).toBe(true);
      expect(Object.values(l.critters).reduce((a, b) => a + b, 0)).toBe(0);
      expect(l.event).toBeNull();
      await page.waitForTimeout(500);
    }
  });
});

test("with reduced motion, none of the new life is drawn", async ({ page }) => {
  await enter(page);
  await at(page, "woodland", 45, 13);
  await page.waitForTimeout(1000);
  expect(await life(page)).toBeNull();
});
