import { expect, test, frames, menuItem, readSave, title, warpTo, type Page } from "./helpers";

/** Where the save says the hero stands: `map,x,z,facing`. */
const at = async (page: Page) => (await readSave(page)).world_at?.[0] ?? "";
const sitting = (page: Page) => page.evaluate(() => (document.querySelector(".world-host") as any)?.sixfoldStats?.().sitting);

/** Past the lesson and the opening scene, into the world. */
async function intoWorld(page: Page) {
  page.on("dialog", (d) => d.accept());
  await title(page);
  await menuItem(page, "Learn to play").click();
  await page.getByRole("button", { name: "Skip the lesson" }).click();
  await menuItem(page, "Continue").click();
  await expect(page.getByRole("status").filter({ hasText: "Loading the world" })).toBeHidden({ timeout: 60_000 });
  const skip = page.getByRole("button", { name: /^Skip/ });
  await expect(skip).toBeVisible();
  // A scene ignores a skip in its first 400 ms (#155).
  await page.waitForTimeout(450);
  await page.keyboard.press("Escape");
  await expect(skip).toBeHidden();
}

test("sit on a bench: Sit, stand up with Escape or a move, and a reload stands in front of it", async ({ page }) => {
  await intoWorld(page);

  // In front of the meadow's first bench (22..23, 16), facing it.
  await page.evaluate(() => (document.querySelector(".world-host") as any).sixfoldWarp("overworld", 22, 17, "n"));
  await expect.poll(() => at(page)).toBe("overworld,22,17,n");
  const sit = page.getByRole("button", { name: /^Sit/ });
  const up = page.getByRole("button", { name: /^Stand up/ });

  // E sits; Escape stands up (and leaves the menu shut).
  await expect(sit).toBeVisible();
  await page.keyboard.press("e");
  await expect.poll(() => sitting(page)).toBe(true);
  await expect(up).toBeVisible();
  await expect.poll(() => at(page)).toBe("overworld,22,17,s"); // the tile in front stays the hero's
  await page.keyboard.press("Escape");
  await expect.poll(() => sitting(page)).toBe(false);
  await expect(page.getByRole("dialog", { name: "Menu" })).toBeHidden();

  // Facing away now: holding ArrowUp turns back and walks into the bench, which sits the hero, no E.
  // Escape with that key still held stands up for good: the bench doesn't take the hero straight back.
  await page.keyboard.down("ArrowUp");
  await expect.poll(() => sitting(page)).toBe(true);
  await page.keyboard.press("Escape");
  await expect.poll(() => sitting(page)).toBe(false);
  await page.waitForTimeout(500); // well past the hold that walks
  expect(await sitting(page)).toBe(false);
  await page.keyboard.up("ArrowUp");
  // Again (facing away once more): a key pointing into the seat, held or pressed again, leaves it sitting.
  await page.keyboard.down("ArrowUp");
  await expect.poll(() => sitting(page)).toBe(true);
  await frames(page);
  expect(await sitting(page)).toBe(true);
  await page.keyboard.up("ArrowUp");
  await page.keyboard.press("ArrowUp");
  await frames(page);
  expect(await sitting(page)).toBe(true);
  // Any other move stands up and walks on, off the tile in front of the bench.
  await page.keyboard.down("ArrowLeft");
  await expect.poll(() => sitting(page)).toBe(false);
  await expect.poll(async () => Number((await at(page)).match(/^overworld,(\d+),17,w$/)?.[1] ?? 99)).toBeLessThan(22);
  await page.keyboard.up("ArrowLeft");

  // Seated, a reload stands the hero on the tile in front of the bench.
  await page.evaluate(() => (document.querySelector(".world-host") as any).sixfoldWarp("overworld", 22, 17, "n"));
  await expect.poll(() => at(page)).toBe("overworld,22,17,n");
  await expect(sit).toBeVisible(); // the warp has faded in: the prompt acts again
  await frames(page);
  await sit.click();
  await expect.poll(() => sitting(page)).toBe(true);
  // Sitting turns the hero at once, but the turn is saved on the next frame: wait for the save, then reload.
  await expect.poll(() => at(page)).toBe("overworld,22,17,s");
  await page.reload();
  await menuItem(page, "Continue").click();
  await expect(page.getByRole("status").filter({ hasText: "Loading the world" })).toBeHidden({ timeout: 60_000 });
  await expect(page.locator("[data-hero]")).toHaveAttribute("data-hero", "22,17,s");
  expect(await sitting(page)).toBe(false);
});

test("a find taken from under a bench gives the place back to sitting", async ({ page }) => {
  await intoWorld(page);

  // Brightwell Square: Facets stuck under the bench by the fountain (21, 20), faced from the north.
  await page.evaluate(() => (document.querySelector(".world-host") as any).sixfoldWarp("square", 21, 19, "s"));
  await expect.poll(() => at(page)).toBe("square,21,19,s");
  const look = page.getByRole("button", { name: /^Take a look/ });
  await expect(look).toBeVisible();
  // Walking into the bench offers the find, as the prompt does: it doesn't sit on it.
  await page.keyboard.down("ArrowDown");
  await page.waitForTimeout(500); // well past the hold that walks
  await page.keyboard.up("ArrowDown");
  expect(await sitting(page)).toBe(false);
  await expect(look).toBeVisible();
  await page.keyboard.press("e");
  await expect(page.locator("#talk-live")).toHaveText(/stuck to the slats/);
  const note = page.getByRole("button", { name: /^A note/ });
  await note.click(); // one line: it closes
  await expect(note).toBeHidden();
  // Taken: the place is a seat again.
  await expect(page.getByRole("button", { name: /^Sit/ })).toBeVisible();
  await page.keyboard.press("e");
  await expect.poll(() => sitting(page)).toBe(true);
});

test("a tap toward a seat only turns, and walking into one from behind doesn't sit", async ({ page }) => {
  await intoWorld(page);
  // In front of the meadow's first bench: sit (E) and stand up (Escape), so the hero faces away from it with no
  // warp fading. A tap: down, then up in the very next frame, once the game's own frame has seen it (well under
  // the hold that walks), whatever the frame rate.
  await warpTo(page, "overworld", 22, 17, "n");
  await expect(page.getByRole("button", { name: /^Sit/ })).toBeVisible();
  await page.keyboard.press("e");
  await expect.poll(() => sitting(page)).toBe(true);
  await page.keyboard.press("Escape");
  await expect.poll(() => sitting(page)).toBe(false);
  await expect.poll(() => at(page)).toBe("overworld,22,17,s");
  await page.evaluate(() => new Promise<void>((done) => {
    const key = (type: string) => document.body.dispatchEvent(new KeyboardEvent(type, { key: "ArrowUp", bubbles: true }));
    key("keydown");
    requestAnimationFrame(() => (key("keyup"), done()));
  }));
  await expect.poll(() => at(page)).toBe("overworld,22,17,n");
  await frames(page);
  expect(await sitting(page)).toBe(false);

  // Behind the bench (its back to the north), facing away: holding ArrowDown turns the hero (so the world took
  // the key), then pushes into the backrest, which is just a wall.
  await warpTo(page, "overworld", 22, 15, "n");
  await page.keyboard.down("ArrowDown");
  await expect.poll(() => at(page)).toBe("overworld,22,15,s");
  await page.waitForTimeout(500); // well past the hold that walks
  await page.keyboard.up("ArrowDown");
  expect(await sitting(page)).toBe(false);
  expect(await at(page)).toBe("overworld,22,15,s");
});

test("on a phone, holding the pad into a seat sits, and another way stands up @phone", async ({ page }) => {
  await intoWorld(page);
  const pad = page.getByRole("group", { name: "Walk" });
  const hold = (name: string, type: string) => pad.getByRole("button", { name }).dispatchEvent(type, { pointerId: 1, isPrimary: true });
  // In front of the meadow's first bench, facing away: the pad's Up, held, turns back and sits.
  await warpTo(page, "overworld", 22, 17, "s");
  await hold("Up", "pointerdown");
  await expect.poll(() => sitting(page)).toBe(true);
  await frames(page);
  expect(await sitting(page)).toBe(true);
  await hold("Up", "pointerup");
  await hold("Left", "pointerdown");
  await expect.poll(() => sitting(page)).toBe(false);
  await hold("Left", "pointerup");
});

test.describe("with motion", () => {
  test.use({ reducedMotion: "no-preference" }); // the turn and the SitDown clip play out

  test("walking into a seat sits, whatever way it faces @webkit", async ({ page }) => {
    const seat = (page: Page) => page.evaluate(() => (document.querySelector(".world-host") as any)?.sixfoldStats?.().seat);
    // The way a seat faces, as a yaw: atan2 of its direction (+z is south, toward the camera).
    const DIR: Record<string, [number, number]> = { n: [0, -1], s: [0, 1], e: [1, 0], w: [-1, 0] };
    await intoWorld(page);
    // One seat per rotation, each walked into: [map, x, z, facing, key, the way the seat faces]. The two
    // chairs at a duel table (rot 180, 270) are walked into from beside: the table fills their front.
    const seats: [string, number, number, string, string, string][] = [
      ["overworld", 22, 17, "n", "ArrowUp", "s"], // bench, rot 0
      ["bakery", 2, 4, "w", "ArrowLeft", "e"], // chair, rot 90
      ["inn_dream", 9, 6, "w", "ArrowLeft", "n"], // chair, rot 180
      ["lodge", 3, 6, "n", "ArrowUp", "w"], // chair, rot 270
    ];
    for (const [map, x, z, facing, key, face] of seats) {
      await warpTo(page, map, x, z, facing);
      await page.keyboard.down(key); // kept through the warp's fade
      const want = Math.atan2(DIR[face][0], DIR[face][1]);
      await expect
        .poll(
          async () => {
            const s = await seat(page);
            const off = s && Math.abs(Math.atan2(Math.sin(s.yaw - want), Math.cos(s.yaw - want)));
            return s ? `${s.face} settled:${s.k === 1} turned:${off < 0.05}` : "standing";
          },
          { message: `${map} (${x},${z})` },
        )
        .toBe(`${face} settled:true turned:true`);
      await page.keyboard.up(key);
      await page.keyboard.press("Escape");
      await expect.poll(() => seat(page)).toBeNull();
    }
  });
});
