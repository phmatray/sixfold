import { expect, test, frames, loaded, menuItem, readSave, title, type Page } from "./helpers";

// Authored camera shots (#253) are played by @theatre/core, vendored as one ESM file behind the importmap.
test("@theatre/core imports from the game's own vendored file", async ({ page }) => {
  await page.goto("/");
  const api = await page.evaluate(async () => {
    const core: any = await import("@theatre/core" as string);
    const project = core.getProject("sixfold-import-check");
    return { getProject: typeof core.getProject, name: project.address.projectId };
  });
  expect(api).toEqual({ getProject: "function", name: "sixfold-import-check" });
});

// Cinderwick's reveal (#262): the first authored shot, played on the first entry.
type Cam = { angle: string; pitch: number; shot: boolean };
const stats = (page: Page) => page.evaluate(() => (document.querySelector(".world-host") as any).sixfoldStats());
const warpTo = (page: Page, map: string, x: number, z: number) =>
  page.evaluate(([m, x, z]) => (document.querySelector(".world-host") as any).sixfoldWarp(m, x, z, "n"), [map, x, z] as const);
const intoCinderwick = (page: Page) => warpTo(page, "cinderwick", 25, 30);
const revealed = async (page: Page) => (await readSave(page)).progress?.[0]?.split(",").includes("f:reveal_cinderwick");

/** A new player in the world, then every drawn frame's camera is recorded into `window.__cam`. */
async function intoWorld(page: Page) {
  page.on("dialog", (d) => d.accept());
  await title(page);
  await menuItem(page, "Learn to play").click();
  await page.getByRole("button", { name: "Skip the lesson" }).click();
  await menuItem(page, "Continue").click();
  await loaded(page);
  const skip = page.getByRole("button", { name: /^Skip/ });
  await expect(skip).toBeVisible();
  await page.waitForTimeout(450); // a scene ignores a skip in its first 400 ms
  await page.keyboard.press("Escape");
  await expect(skip).toBeHidden();
  await page.evaluate(() => {
    const log: Cam[] = ((window as any).__cam = []);
    const tick = () => {
      log.push((document.querySelector(".world-host") as any).sixfoldStats().camera);
      requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  });
}
const log = (page: Page): Promise<Cam[]> => page.evaluate(() => (window as any).__cam);
/** Motion on (the suite's default is reduced); the world hears the change on a frame. */
async function withMotion(page: Page) {
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await frames(page);
}

test("the first entry to Cinderwick plays the reveal shot, hands back with no jump, and a second plays nothing", async ({ page }) => {
  await intoWorld(page);
  await withMotion(page);
  await intoCinderwick(page);
  await expect.poll(() => revealed(page), { timeout: 90_000 }).toBe(true);
  await expect.poll(async () => (await stats(page)).camera.shot).toBe(false);
  await expect.poll(async () => Math.round((await stats(page)).camera.pitch)).toBe(50);
  const cams = await log(page);
  // The pitch moves through the shot: how many distinct values are drawn depends on the runner's frame rate (a few
  // frames a second on software GL), so this asks for a handful, not one per frame.
  expect(new Set(cams.filter((c) => c.shot).map((c) => c.pitch.toFixed(2))).size).toBeGreaterThanOrEqual(4);
  expect((await stats(page)).camera.angle).toBe("default");
  // No jump at the hand-back: the first frame after the shot is within a degree of the last frame of it.
  const last = cams.map((c) => c.shot).lastIndexOf(true);
  expect(Math.abs(cams[last + 1].pitch - cams[last].pitch)).toBeLessThan(1);
  // A second entry plays nothing.
  await warpTo(page, "overworld", 22, 17);
  await expect.poll(async () => (await stats(page)).area).toBe("overworld");
  await page.evaluate(() => ((window as any).__cam.length = 0));
  await intoCinderwick(page);
  await expect.poll(async () => (await stats(page)).area).toBe("cinderwick");
  await page.waitForTimeout(1500);
  expect((await log(page)).some((c) => c.shot)).toBe(false);
});

test("Escape during the shot ends the scene with the camera back on the hero", async ({ page }) => {
  await intoWorld(page);
  await withMotion(page);
  await intoCinderwick(page);
  await expect.poll(async () => (await stats(page)).camera.shot, { timeout: 60_000 }).toBe(true);
  await page.waitForTimeout(450);
  await page.keyboard.press("Escape");
  await expect.poll(() => revealed(page), { timeout: 30_000 }).toBe(true);
  await expect.poll(async () => (await stats(page)).camera.shot).toBe(false);
  await expect.poll(async () => Math.round((await stats(page)).camera.pitch)).toBe(50);
});

test("under reduced motion only the final framing is shown, then the scene goes on", async ({ page }) => {
  await intoWorld(page);
  await intoCinderwick(page);
  await expect.poll(() => revealed(page), { timeout: 90_000 }).toBe(true);
  const pitches = new Set((await log(page)).filter((c) => c.shot).map((c) => c.pitch.toFixed(1)));
  expect(pitches.size).toBeLessThanOrEqual(1);
});

test("a shot file that fails to load logs one warning and the scene goes on", async ({ page }) => {
  const warnings: string[] = [];
  page.on("console", (m) => m.type() === "warning" && /no shot/.test(m.text()) && warnings.push(m.text()));
  await page.route("**/world/shots/*.json", (r) => r.fulfill({ body: "not a shot" }));
  await intoWorld(page);
  await intoCinderwick(page);
  await expect.poll(() => revealed(page), { timeout: 90_000 }).toBe(true);
  expect(warnings).toHaveLength(1);
  const before = (await stats(page)).at;
  await page.keyboard.down("ArrowRight");
  await expect.poll(async () => (await stats(page)).at[0], { timeout: 15_000 }).not.toBe(before[0]);
  await page.keyboard.up("ArrowRight");
});
