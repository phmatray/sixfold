import { expect, test } from "@playwright/test";

/**
 * The shell around the game: the typefaces ship with it (web/fonts), and the
 * service worker runs in a browser but never in the desktop app (desktop/,
 * where Tauri sets window.__TAURI_INTERNALS__ before the page's scripts).
 */

test("both typefaces load from the game's own files", async ({ page }) => {
  const outside: string[] = [];
  page.on("request", (r) => {
    if (!r.url().startsWith("http://127.0.0.1")) outside.push(r.url());
  });
  await page.goto("/");
  const loaded = await page.evaluate(async () => [
    (await document.fonts.load('400 16px "Atkinson Hyperlegible"', "Œuvre déjà, łódź")).length,
    (await document.fonts.load('700 16px "Atkinson Hyperlegible"', "Œuvre déjà, łódź")).length,
    (await document.fonts.load('800 16px "Bricolage Grotesque"', "Œuvre déjà, łódź")).length,
  ]);
  expect(loaded).toEqual([2, 2, 2]); // latin and latin-ext, each
  expect(outside).toEqual([]);
});

test.describe("the service worker", () => {
  test.use({ serviceWorkers: "allow" });

  test("registers in a browser", async ({ page }) => {
    await page.goto("/");
    const active = await page.evaluate(() => navigator.serviceWorker.ready.then((r) => !!r.active));
    expect(active).toBe(true);
  });

  test("stays off in the desktop app", async ({ page }) => {
    await page.addInitScript(() => {
      (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
    });
    await page.goto("/");
    await page.waitForLoadState("load");
    // The page registers on load; give it the same chance it would have in a browser.
    await page.waitForTimeout(2000);
    const regs = await page.evaluate(async () => (await navigator.serviceWorker.getRegistrations()).length);
    expect(regs).toBe(0);
  });
});
