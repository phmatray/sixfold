import { editSave, expect, loaded, menuItem, readSave, test, title, warpTo as warp, type Page } from "./helpers";

/** Secrets (#116): people who come out only once someone has told of them, and the hum by a hidden find. */
const host = (page: Page, f: string, ...args: unknown[]) => page.evaluate(([f, args]) => (document.querySelector(".world-host") as any)[f as string](...(args as unknown[])), [f, args] as const);
const people = (page: Page) => page.evaluate(() => ((document.querySelector(".world-host") as any).sixfoldStats().people as { id: string }[]).map((p) => p.id));
const hum = (page: Page) => page.evaluate(() => (globalThis as any).sixfoldAudio?.debug().hum as number);
const flags = async (page: Page) => (await readSave(page)).progress?.[0] ?? "";

/** A new game, past the lesson, back in the world at `at` (`map,x,z,f`) with these flags added. */
async function into(page: Page, at: string, add: string[] = []) {
  if (!(await page.evaluate(() => localStorage.getItem("sixfold.save.1")))) {
    page.on("dialog", (d) => d.accept());
    await title(page);
    await menuItem(page, "Learn to play").click();
    await page.getByRole("button", { name: "Skip the lesson" }).click();
    await expect(menuItem(page, "Continue")).toBeVisible();
  }
  const f = ["intro", ...add].map((x) => `f:${x}`).join(",");
  await editSave(page, `secs.progress = [(secs.progress?.[0] ?? "") + ",${f}"]; secs.world_at = ["${at}"];`);
  await page.reload();
  await menuItem(page, "Continue").click();
  await loaded(page);
}

/** Talk to whoever the hero faces, through every line. */
async function talk(page: Page, who: string) {
  await expect(page.getByRole("button", { name: new RegExp(`^Talk to ${who}`) })).toBeVisible();
  await page.keyboard.press("e");
  const box = page.locator(".talk-box");
  await expect(box).toBeVisible({ timeout: 60_000 });
  while (await box.isVisible()) {
    await page.keyboard.press("Enter");
    await page.waitForTimeout(400);
  }
}

test("AC5: Faye waits in Lanternmere's willows only at dusk, and only once Lisette has told of her", async ({ page }) => {
  test.slow(); // warps and fades at ~1 frame a second on CI's software GL
  await page.goto("/");
  await into(page, "lanternmere,3,26,n");
  // Dusk, untold: Lisette has gone down to the jetty (the hour holds), and nobody is in the willows.
  await host(page, "sixfoldClock", 19 * 60);
  await expect.poll(() => people(page)).toContain("lisette_dusk");
  expect(await people(page)).not.toContain("t_faye");

  // By day, Lisette tells of the dancer in the willows.
  await host(page, "sixfoldClock", 12 * 60);
  await warp(page, "lanternmere", 23, 9, "n");
  await talk(page, "Lisette");
  await expect.poll(() => flags(page)).toContain("f:heard_willow_dusk");
  // Told, but it is still day: not yet.
  await warp(page, "lanternmere", 3, 26, "n");
  await page.waitForTimeout(1_000);
  expect(await people(page)).not.toContain("t_faye");
  // At dusk she is there.
  await host(page, "sixfoldClock", 19 * 60);
  await expect.poll(() => people(page)).toContain("t_faye");
  // And gone at night.
  await host(page, "sixfoldClock", 22 * 60);
  await expect.poll(() => people(page)).not.toContain("t_faye");
});

test("AC5: Barnaby deals cards in the meadow's orchard only at dusk, once told", async ({ page }) => {
  test.slow();
  await page.goto("/");
  await into(page, "overworld,9,7,n");
  await host(page, "sixfoldClock", 19 * 60);
  await page.waitForTimeout(1_000);
  expect(await people(page)).not.toContain("t_barnaby");
  await into(page, "overworld,9,7,n", ["heard_orchard_dusk"]);
  await host(page, "sixfoldClock", 12 * 60);
  await page.waitForTimeout(1_000);
  expect(await people(page)).not.toContain("t_barnaby");
  await host(page, "sixfoldClock", 19 * 60);
  await expect.poll(() => people(page)).toContain("t_barnaby");
});

test("AC5: Quenby sits in the Undermine's crystal grotto only once Pike has told of him", async ({ page }) => {
  test.slow();
  await page.goto("/");
  await into(page, "undermine,42,8,e");
  expect(await people(page)).toContain("pike");
  expect(await people(page)).not.toContain("quenby");
  await talk(page, "Pike");
  await expect.poll(() => flags(page)).toContain("f:heard_grotto_collector");
  await expect.poll(() => people(page)).toContain("quenby");
  // Every one of his branches sets the secret, the first hello too (map.js).
  await warp(page, "undermine", 31, 10, "n");
  await talk(page, "Quenby");
  await expect.poll(() => flags(page)).toContain("f:secret_grotto_collector");
});

test("the strongbox hums near its wall until it is found", async ({ page }) => {
  test.slow();
  await page.goto("/");
  await into(page, "undermine,2,7,e");
  await page.waitForTimeout(1_000);
  expect(await hum(page)).toBe(0);
  await warp(page, "undermine", 10, 6, "n");
  await expect.poll(() => hum(page)).toBeGreaterThan(0.2);
  // Found (and so a secret): silent, even beside it.
  await into(page, "undermine,10,6,n", ["found_sn_strongbox", "secret_strongbox_hum"]);
  await page.waitForTimeout(1_000);
  expect(await hum(page)).toBe(0);
});

test("the hum stops when the world unmounts", async ({ page }) => {
  test.slow();
  await page.goto("/");
  await into(page, "undermine,2,7,e");
  await warp(page, "undermine", 10, 6, "n");
  await expect.poll(() => hum(page)).toBeGreaterThan(0.2);
  // Out through the in-game menu to the title.
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "Quit to title" }).click();
  await expect(page.getByRole("navigation", { name: "Main menu" })).toBeVisible();
  await expect.poll(() => hum(page)).toBe(0);
  // Back in, by the strongbox again; then another map replaces this one.
  await menuItem(page, "Continue").click();
  await loaded(page);
  await warp(page, "undermine", 10, 6, "n");
  await expect.poll(() => hum(page)).toBeGreaterThan(0.2);
  await warp(page, "square", 25, 10, "s");
  await expect.poll(() => hum(page)).toBe(0);
});

test("AC4: a secret found stays found: Teo's chest is opened after a reload, and Records counts it", async ({ page }) => {
  test.slow();
  await page.goto("/");
  // Nettie has told of her brother's sea chest, on top of the net locker the hero faces.
  await into(page, "boathouse,2,2,w", ["heard_loft_chest"]);
  const look = page.getByRole("button", { name: /^Take a look/ });
  const box = page.locator(".talk-box");
  await expect(look).toBeVisible();
  await page.keyboard.press("e");
  await expect(box).toContainText("Teo's sea chest", { timeout: 60_000 });
  while (await box.isVisible()) {
    await page.keyboard.press("Enter");
    await page.waitForTimeout(400);
  }
  await expect.poll(() => flags(page)).toContain("f:found_i_loft_chest");
  expect(await flags(page)).toContain("f:secret_loft_chest");

  // After a reload Records counts it, and the chest stays opened: looking again finds nothing.
  await page.reload();
  await menuItem(page, "Records").click();
  await page.getByRole("tab", { name: "Stats" }).click();
  // The total grows as regions add secrets (meta/src/stats.rs SECRETS, unit-tested there); this checks the found count.
  await expect(page.locator(".stat").filter({ hasText: "Secrets" })).toContainText(/\b1 \/ \d+\b/);
  await page.getByRole("button", { name: "Back", exact: true }).click();
  await menuItem(page, "Continue").click();
  await loaded(page);
  await expect(look).toBeVisible();
  await page.keyboard.press("e");
  await expect(box).toContainText("Nothing else here.", { timeout: 60_000 });
});

test("a find looted before secrets earns its secret on a second look", async ({ page }) => {
  test.slow();
  await page.goto("/");
  // Teo's chest taken by a save from before #116: found, but never counted a secret.
  await into(page, "boathouse,2,2,w", ["heard_loft_chest", "found_i_loft_chest"]);
  expect(await flags(page)).not.toContain("f:secret_loft_chest");
  const box = page.locator(".talk-box");
  await expect(page.getByRole("button", { name: /^Take a look/ })).toBeVisible();
  await page.keyboard.press("e");
  await expect(box).toContainText("Nothing else here.", { timeout: 60_000 });
  while (await box.isVisible()) {
    await page.keyboard.press("Enter");
    await page.waitForTimeout(400);
  }
  await expect.poll(() => flags(page)).toContain("f:secret_loft_chest");
});
