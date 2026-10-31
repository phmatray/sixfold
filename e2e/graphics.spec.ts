import { expect, test, menuItem, title, type Page } from "./helpers";

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

/** The opening scene skipped, so the world runs and the Menu answers (a scene ignores a skip in its first 400 ms, #155). */
async function skipScene(page: Page) {
  const skip = page.getByRole("button", { name: /^Skip/ });
  await expect(skip).toBeVisible();
  await page.waitForTimeout(450);
  await page.keyboard.press("Escape");
  await expect(skip).toBeHidden();
}

/** Lets `n` animation frames pass. */
const frames = (page: Page, n: number) =>
  page.evaluate((n) => new Promise<void>((done) => { const tick = () => (--n > 0 ? requestAnimationFrame(tick) : done()); requestAnimationFrame(tick); }), n);

/** Menu → Options → Graphics → `name`, then closed. */
async function pickGraphics(page: Page, name: string) {
  await page.getByRole("button", { name: "Menu", exact: true }).click();
  await page.getByRole("tab", { name: "Options" }).click();
  await page.getByRole("group", { name: "Graphics" }).getByRole("radio", { name, exact: true }).check();
  await page.getByRole("button", { name: "Close", exact: true }).click();
}

const choose = (page: Page, tier: string) => page.addInitScript((t) => localStorage.setItem("sixfold.graphics", t), tier);

test.beforeEach(({ page }) => void page.on("dialog", (d) => d.accept()));

test("with nothing saved, a software renderer draws the world on Low", async ({ page }) => {
  await title(page);
  await enterWorld(page);
  await expect.poll(() => graphics(page), { timeout: 30_000 }).toBe("low");
});

test("Auto on Low stays Low while the world runs: the probe never steps Low to Off", async ({ page }) => {
  await title(page);
  await enterWorld(page);
  await skipScene(page);
  await expect.poll(() => graphics(page), { timeout: 30_000 }).toBe("low");
  await frames(page, 150);
  expect(await graphics(page)).toBe("low");
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

test("a saved High is in force on a weak device, while the opening scene holds the world, in one WebGL context", async ({ page }) => {
  // The opening scene pauses the world, and paused frames aren't measured: the probe doesn't run here.
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

test("Options picks the graphics at once, the browser keeps it, and the title's Settings show it", async ({ page }) => {
  await title(page);
  await enterWorld(page);
  // The opening scene: skipped, so the Menu answers (a scene ignores a skip in its first 400 ms, #155).
  const skip = page.getByRole("button", { name: /^Skip/ });
  await expect(skip).toBeVisible();
  await page.waitForTimeout(450);
  await page.keyboard.press("Escape");
  await expect(skip).toBeHidden();
  await expect.poll(() => graphics(page), { timeout: 30_000 }).toBe("low");

  await page.getByRole("button", { name: "Menu", exact: true }).click();
  await page.getByRole("tab", { name: "Options" }).click();
  const group = page.getByRole("group", { name: "Graphics" });
  await expect(group).toBeVisible();
  await expect(group.getByRole("radio", { name: "Auto", exact: true })).toBeChecked();
  await group.getByRole("radio", { name: "Off", exact: true }).check();
  await expect.poll(() => graphics(page)).toBe("off");
  expect(await page.evaluate(() => localStorage.getItem("sixfold.graphics"))).toBe("off");
  await page.getByRole("button", { name: "Close", exact: true }).click();

  // A reload keeps it, in the title's Settings and in the world.
  await page.reload();
  await page.locator("summary", { hasText: "Settings" }).click();
  const settings = page.getByRole("dialog", { name: "Settings" });
  await expect(settings.getByRole("group", { name: "Graphics" })).toBeVisible();
  await expect(settings.getByRole("radio", { name: "Off", exact: true })).toBeChecked();
  await settings.getByRole("button", { name: "Close", exact: true }).click();
  await menuItem(page, "Continue").click();
  await expect(page.getByRole("status").filter({ hasText: "Loading the world" })).toBeHidden({ timeout: 60_000 });
  await expect.poll(() => graphics(page), { timeout: 30_000 }).toBe("off");

  // Auto again: the device decides, Low on SwiftShader.
  await page.getByRole("button", { name: "Menu", exact: true }).click();
  await page.getByRole("tab", { name: "Options" }).click();
  await page.getByRole("group", { name: "Graphics" }).getByRole("radio", { name: "Auto", exact: true }).check();
  await expect.poll(() => graphics(page), { timeout: 30_000 }).toBe("low");
  expect(await page.evaluate(() => localStorage.getItem("sixfold.graphics"))).toBe("auto");
});

test("High saved, slow frames step the world down to Low for the visit, and the saved choice stays High", async ({ page }) => {
  // The world's frame-time probe (#225): frames 30 to 120 at High, under 45 fps → Low, never saved.
  // SwiftShader on High is far past that already (~380 ms a frame at 1280 × 720); the CPU slowed 20× keeps it
  // so on a faster runner, and a small window keeps the 120 frames short.
  test.setTimeout(360_000); // ~1.5 min locally: room for a slower CI runner
  await page.setViewportSize({ width: 640, height: 360 });
  await choose(page, "high");
  await title(page);
  await enterWorld(page);
  await expect.poll(() => graphics(page), { timeout: 30_000 }).toBe("high");
  // The opening scene holds the world (paused frames aren't measured): skipped.
  const skip = page.getByRole("button", { name: /^Skip/ });
  await expect(skip).toBeVisible();
  await page.waitForTimeout(450);
  await page.keyboard.press("Escape");
  await expect(skip).toBeHidden();
  const cdp = await page.context().newCDPSession(page);
  await cdp.send("Emulation.setCPUThrottlingRate", { rate: 20 });
  await expect.poll(() => graphics(page), { timeout: 150_000 }).toBe("low");
  expect(await page.evaluate(() => localStorage.getItem("sixfold.graphics"))).toBe("high");

  // A choice made in Options ends the probe: High picked again (Low first, High is still the checked one)
  // stays High, slow frames or not.
  await pickGraphics(page, "Low");
  await pickGraphics(page, "High");
  await expect.poll(() => graphics(page), { timeout: 30_000 }).toBe("high");
  await frames(page, 150);
  expect(await graphics(page)).toBe("high");
  await cdp.send("Emulation.setCPUThrottlingRate", { rate: 1 });
});

test("High picked in Options before the probe has decided stays High, slow frames or not", async ({ page }) => {
  // Low saved: the probe never measures Low. High picked during the visit is the player's choice, not a tier
  // to probe: without that, its first 120 slow frames would step it back to Low.
  test.setTimeout(360_000); // ~1.1 min locally: room for a slower CI runner
  await page.setViewportSize({ width: 640, height: 360 });
  await choose(page, "low");
  await title(page);
  await enterWorld(page);
  await skipScene(page);
  await expect.poll(() => graphics(page), { timeout: 30_000 }).toBe("low");
  const cdp = await page.context().newCDPSession(page);
  await cdp.send("Emulation.setCPUThrottlingRate", { rate: 20 });
  await pickGraphics(page, "High");
  await expect.poll(() => graphics(page), { timeout: 30_000 }).toBe("high");
  await frames(page, 150);
  expect(await graphics(page)).toBe("high");
  await cdp.send("Emulation.setCPUThrottlingRate", { rate: 1 });
});
