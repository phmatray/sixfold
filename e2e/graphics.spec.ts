import { expect, test, type Page } from "@playwright/test";
import { menuItem, title } from "./helpers";

// The graphics tier (#225): the world draws through a post-processing composer picked from the
// device (Low on CI's SwiftShader) or the player's choice (`sixfold.graphics`), in the one context it had.
const graphics = (page: Page): Promise<string | undefined> =>
  page.evaluate(() => (document.querySelector(".world-host") as any)?.sixfoldStats?.().graphics);
const glMade = (page: Page): Promise<number> => page.evaluate(() => (window as any).glMade);

/** From the title, a new player's way in: the lesson skipped, the world loaded. */
async function enterWorld(page: Page) {
  await menuItem(page, "Learn to play").click();
  await page.getByRole("button", { name: "Skip the lesson" }).click();
  await menuItem(page, "Continue").click();
  await expect(page.getByRole("status").filter({ hasText: "Loading the world" })).toBeHidden({ timeout: 60_000 });
}

const choose = (page: Page, tier: string) => page.addInitScript((t) => localStorage.setItem("sixfold.graphics", t), tier);

test.beforeEach(({ page }) => void page.on("dialog", (d) => d.accept()));

test("with nothing saved, a software renderer draws the world on Low", async ({ page }) => {
  await title(page);
  await enterWorld(page);
  await expect.poll(() => graphics(page), { timeout: 30_000 }).toBe("low");
});

test("Off saved draws the plain frame", async ({ page }) => {
  await choose(page, "off");
  await title(page);
  await enterWorld(page);
  await expect.poll(() => graphics(page), { timeout: 30_000 }).toBe("off");
});

/** Counts the WebGL contexts the page makes, in `window.glMade`. */
const countContexts = (page: Page) =>
  page.addInitScript(() => {
    const get = HTMLCanvasElement.prototype.getContext;
    (window as any).glMade = 0;
    HTMLCanvasElement.prototype.getContext = function (this: HTMLCanvasElement, kind: string, ...rest: any[]) {
      if (kind.startsWith("webgl")) (window as any).glMade++;
      return (get as any).call(this, kind, ...rest);
    } as any;
  });

test("High saved is kept, even on a weak device, and the world makes one WebGL context only", async ({ page }) => {
  await choose(page, "high");
  await countContexts(page);
  await title(page);
  // SwiftShader: the title tries a context once, finds the device weak and goes flat.
  await expect(page.locator(".title-backdrop.flat")).toHaveCount(1);
  const before = await glMade(page);
  await enterWorld(page);
  await expect.poll(() => graphics(page), { timeout: 30_000 }).toBe("high");
  expect(await glMade(page)).toBe(before + 1);
});

test("High saved, the title's backdrop still goes flat on a weak device and opens no new context on a return", async ({ page }) => {
  // The title draws through the same look, but only on a device that passed `weak` (#214):
  // a saved High never brings a context back on SwiftShader.
  await choose(page, "high");
  await countContexts(page);
  await title(page);
  await expect(page.locator(".title-backdrop.flat")).toHaveCount(1);
  const made = await glMade(page);
  await menuItem(page, "Learn to play").click();
  await page.getByRole("button", { name: "Skip the lesson" }).click();
  await expect(page.locator(".title-backdrop.flat")).toHaveCount(1);
  expect(await glMade(page)).toBe(made);
});
