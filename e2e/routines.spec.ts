import { type Page } from "@playwright/test";
import { expect, test, menuItem, readSave, title, warpTo as warp } from "./helpers";

/** People's routines (#113), through `sixfoldStats().people`: where each person is and what it does. */
type Person = { id: string; x: number; z: number; do: string | null; away: boolean; walking: boolean; sitting: boolean };
const people = (page: Page) => page.evaluate(() => (document.querySelector(".world-host") as any).sixfoldStats().people as Person[]);
const who = async (page: Page, id: string) => (await people(page)).find((p) => p.id === id);
const host = (page: Page, f: string, ...args: unknown[]) => page.evaluate(([f, args]) => (document.querySelector(".world-host") as any)[f as string](...(args as unknown[])), [f, args] as const);
const at = async (page: Page) => (await readSave(page)).world_at?.[0] ?? "";

test("routines: on arrival everyone is already at the hour's place; when the hour turns they walk there", async ({ page }) => {
  test.slow(); // walks timed in game seconds, at ~1 frame a second on CI (below)
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

  // AC3: entering Brightwell Square at 22:00, the night is already in place: Wendel has gone in, nobody walks.
  await host(page, "sixfoldClock", 22 * 60);
  await warp(page, "square", 25, 10, "s");
  await expect.poll(async () => (await who(page, "wendel"))?.away).toBe(true);
  const night = await people(page);
  expect(night.filter((p) => p.walking).map((p) => p.id)).toEqual([]);
  expect(night.find((p) => p.id === "tansy")).toMatchObject({ x: 33, z: 13, do: "sweep" });
  // Someone gone in is not there: the tile by the inn door he went in from is free to stand on. Stepped onto from
  // the tile west of it (the way in from the square): game time is wall time only at full frame rate — a frame
  // counts at most 50 ms (world.js), and software GL on CI draws about one a second, so every game second is ~20
  // real ones and a walk across the square alone would eat most of the test's budget.
  const gone = night.find((p) => p.id === "wendel")!;
  await warp(page, "square", gone.x - 1, gone.z, "e");
  // (sixfoldWalk answers [] while the warp's fade is still up: asked until it walks.)
  await expect.poll(async () => ((await host(page, "sixfoldWalk", gone.x, gone.z)) as string[]).length).toBeGreaterThan(0);
  await expect.poll(() => at(page), { timeout: 60_000 }).toMatch(new RegExp(`^square,${gone.x},${gone.z},`));

  // By day Wendel sits on his bench.
  await host(page, "sixfoldClock", 12 * 60);
  await expect.poll(async () => await who(page, "wendel")).toMatchObject({ x: 18, z: 27, do: "sit", sitting: true, away: false });

  // AC2: set to 17:59, the stalls are still open; at 18:00 Tansy and Odile walk to the fountain to talk,
  // and Wendel leaves his bench for the inn. Setting off (up to 3.5 game s) and Tansy's ten tiles (4.5 game s) are
  // ~160 frames: minutes on CI, hence the slow test.
  await host(page, "sixfoldClock", 17 * 60 + 59.9); // a few frames before 18:00 (frames are slow on CI)
  await expect.poll(async () => (await people(page)).filter((p) => p.walking).length, { timeout: 120_000 }).toBeGreaterThanOrEqual(2);
  await expect.poll(async () => [(await who(page, "tansy"))?.x, (await who(page, "tansy"))?.z], { timeout: 240_000 }).toEqual([28, 18]);
  await expect.poll(async () => [(await who(page, "odile"))?.x, (await who(page, "odile"))?.z], { timeout: 120_000 }).toEqual([29, 18]);
  expect(await who(page, "tansy")).toMatchObject({ do: "talk", walking: false });
  const wendel = await who(page, "wendel");
  expect(wendel?.walking || wendel?.away, "Wendel has left his bench for the inn").toBe(true);

  // AC7: a map not converted keeps its loops and its two-copies schedule: in the Downs by day the route walkers
  // walk and the night copies are out of sight.
  await host(page, "sixfoldClock", 12 * 60);
  await warp(page, "downs", 2, 12, "e");
  const start = await people(page);
  expect(start.map((p) => p.id)).not.toContain("biscuit_night");
  await expect.poll(async () => (await who(page, "biscuit"))?.x !== 16 || (await who(page, "biscuit"))?.z !== 18, { timeout: 60_000 }).toBe(true);
});
