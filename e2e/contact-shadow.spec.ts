import { expect, test, frames, loaded, menuItem, title, warpTo, type Page } from "./helpers";

// #340: the hero casts no sun shadow (world.js keeps it out of the shadow pass), so the soft contact shadow under its
// feet (actors.js) is its only one. It was the scenery's, 0.8 of a tile, its dark middle hidden under the body: the
// hero looked shadowless and its cream soles floated on the ground. This reads the drawn frame: the same patch of
// floor just south of the feet (toward the camera, past the toes) is darker with the hero standing on the tile than
// with the hero two tiles away.
const host = (page: Page, call: string, ...a: unknown[]) => page.evaluate(([c, a]) => (document.querySelector(".world-host") as any)[c as string](...(a as unknown[])), [call, a] as const);

/** The mean luminance (0-255) of a 9 × 9 px patch of the page's screenshot, centred on (x, y) in CSS px. */
async function patch(page: Page, [x, y]: number[]) {
  await frames(page);
  const png = (await page.screenshot({ scale: "css" })).toString("base64");
  return page.evaluate(
    async ([png, x, y]) => {
      const img = new Image();
      img.src = `data:image/png;base64,${png}`;
      await img.decode();
      const c = document.createElement("canvas");
      c.width = img.width;
      c.height = img.height;
      const g = c.getContext("2d")!;
      g.drawImage(img, 0, 0);
      const d = g.getImageData(Math.round(x as number) - 4, Math.round(y as number) - 4, 9, 9).data;
      let sum = 0;
      for (let i = 0; i < d.length; i += 4) sum += 0.2126 * d[i] + 0.7152 * d[i + 1] + 0.0722 * d[i + 2];
      return sum / (d.length / 4);
    },
    [png, x, y] as const,
  );
}

test("@webkit @phone the hero stands on a dark contact shadow, not on bare ground (#340)", async ({ page }) => {
  await title(page);
  await menuItem(page, "Explore the world").click();
  await loaded(page);
  const skip = page.getByRole("button", { name: /^Skip/ });
  await expect(skip).toBeVisible();
  await page.waitForTimeout(450); // a scene ignores a skip in its first 400 ms (#155)
  await page.keyboard.press("Escape");
  await expect(skip).toBeHidden();
  // Nothing drawn over the floor where it is read.
  await page.addStyleTag({ content: ".world-top,.world-prompt,.world-controls,.world-legend,.world-pad,.world-run,.scene-caption,.coach-hint{visibility:hidden!important}" });

  // The meadow's plaza at noon, the hero facing the camera. The patch: 0.45 of a tile south of the feet.
  await warpTo(page, "overworld", 25, 15, "s");
  await host(page, "sixfoldClock", 720);
  await page.waitForTimeout(1500); // the warp's fade gone
  const shaded = await patch(page, (await host(page, "sixfoldFeet", 0, 0.45)) as number[]);

  // The same patch of floor with the hero two tiles north.
  await warpTo(page, "overworld", 25, 13, "s");
  await host(page, "sixfoldClock", 720);
  await page.waitForTimeout(1500);
  const bare = await patch(page, (await host(page, "sixfoldFeet", 0, 2.45)) as number[]);

  expect(shaded, `shaded ${shaded.toFixed(1)} vs bare ${bare.toFixed(1)}`).toBeLessThan(bare * 0.85);
});
