import { expect, test, type Page } from "@playwright/test";
import { editSave, frames, loaded, menuItem, readSave, title } from "./helpers";

/** Talk to Marlo from the tile south of him, through his lines, to his challenge sheet. */
async function talkToMarlo(page: Page) {
  await expect(page.locator("[data-hero]")).toHaveAttribute("data-hero", /^22,16/, { timeout: 15_000 });
  await expect(page.getByRole("button", { name: /^Talk to Marlo/ })).toBeVisible();
  await page.keyboard.press("e");
  // The map's first talk draws the portrait in a new WebGL context: slow on CI's software GL.
  const talk = page.locator(".talk-box");
  await expect(talk).toBeVisible({ timeout: 60_000 });
  while (await talk.isVisible()) {
    await page.keyboard.press("Enter");
    await page.waitForTimeout(400);
  }
  const offer = page.getByRole("dialog", { name: "Challenge Marlo?" });
  await expect(offer).toBeVisible();
  return offer;
}

/** Back into the world at the Bellflats (beside Marlo unless `at` says), with these flags added to the save. */
async function atMarlo(page: Page, flags: string[], at = "22,16,n") {
  await editSave(
    page,
    `secs.progress = [(secs.progress?.[0] ?? "") + ",${flags.join(",")}"]; secs.world_at = ["bellflats,${at}"];`,
  );
  await page.reload();
  await menuItem(page, "Continue").click();
  await loaded(page);
}

// #150: a seal keeper accepts only once enough of their region's duelists are bested.
test("Marlo refuses until three of the Saltreach's duelists are bested, and the Journal counts them", async ({ page }) => {
  page.on("dialog", (d) => d.accept());
  await title(page);
  await menuItem(page, "Learn to play").click();
  await page.getByRole("button", { name: "Skip the lesson" }).click();
  await expect(menuItem(page, "Continue")).toBeVisible();

  // Two of the Saltreach's duelists: Marlo refuses, in his own words, and counts them.
  // `f:keepers`: a save of this build, so not vouched for as an old one (which would earn every keeper).
  // `f:reveal_bellflats`: the site has shown itself already (#152), so no scene covers the talk.
  await atMarlo(page, ["f:keepers", "f:intro", "f:reveal_bellflats", "f:beat_t_tobin", "f:beat_bramble", "f:beat_t_pell", "f:beat_t_nell"]);
  const offer = await talkToMarlo(page);
  await expect(offer).toContainText("The tide doesn't turn for strangers.");
  await expect(offer).toContainText("Marlo wants 3 of the Saltreach's duelists bested first. You've bested 2.");
  await expect(offer.getByRole("img", { name: "2 of 3" })).toBeVisible();
  await expect(offer.getByRole("button", { name: /^Challenge/ })).toHaveCount(0);
  await expect(offer.getByRole("button", { name: /^Not now/ })).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(offer).toBeHidden();

  // The Journal's next step names the duelists still needed.
  await page.keyboard.press("j");
  const journal = page.getByRole("dialog", { name: "Journal" });
  await expect(journal).toContainText("Beat 1 more duelist of the Saltreach to earn Marlo's challenge (2 of 3).");
  await page.keyboard.press("Escape");
  await expect(journal).toBeHidden();
  // The menu's Campaign roster counts them too.
  await page.getByRole("button", { name: "Menu", exact: true }).click();
  await page.getByRole("tab", { name: "Campaign" }).click();
  await expect(page.locator("li.rival").filter({ hasText: "Marlo" })).toContainText("Saltreach duelists bested: 2 of 3");
  await page.keyboard.press("Escape");
  await expect(page.locator("li.rival")).toHaveCount(0);

  // A third: the duel is on offer.
  await atMarlo(page, ["f:beat_t_skye"]);
  const again = await talkToMarlo(page);
  await expect(again.getByRole("button", { name: /^Challenge/ })).toBeVisible();
  await expect(again).not.toContainText("bested first");
});

/** A new game, its lesson skipped: a save to edit. */
async function newSave(page: Page) {
  page.on("dialog", (d) => d.accept());
  await title(page);
  await menuItem(page, "Learn to play").click();
  await page.getByRole("button", { name: "Skip the lesson" }).click();
  await expect(menuItem(page, "Continue")).toBeVisible();
}

const walk = (page: Page, x: number, z: number) =>
  page.evaluate(([x, z]) => (document.querySelector(".world-host") as any).sixfoldWalk(x, z).length, [x, z]);

// #152: once earned, stepping toward Marlo stages his entrance; a skip still lands on the challenge.
test("Marlo's entrance plays as you step toward him, and Escape lands on his challenge", async ({ page }) => {
  await newSave(page);
  const tide = ["f:beat_t_tobin", "f:beat_t_pell", "f:beat_t_nell", "f:beat_t_skye"];
  await atMarlo(page, ["f:keepers", "f:intro", "f:reveal_bellflats", "f:beat_bramble", ...tide], "22,18,n");
  await expect(page.locator("[data-hero]")).toHaveAttribute("data-hero", /^22,18/, { timeout: 15_000 });
  expect(await walk(page, 22, 17)).toBeGreaterThan(0);

  // Two tiles from him: the scene, his first line in his own voice.
  const talk = page.getByRole("dialog", { name: "Marlo" });
  await expect(talk).toContainText("There you are.", { timeout: 60_000 });
  await page.keyboard.press("Escape");

  // Skipped: the challenge is still on offer, and the meeting is remembered.
  const offer = page.getByRole("dialog", { name: "Challenge Marlo?" });
  await expect(offer).toBeVisible();
  await expect(offer.getByRole("button", { name: /^Challenge/ })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(offer).toBeHidden();
  expect((await readSave(page)).progress?.[0]).toContain("f:met_marlo");
  // Input is back: a walk is taken (none is while a scene plays). Not waited out: on CI's software GL a
  // step takes many seconds. `!flag:met_marlo` keeps the ring quiet from now on (engine/tests/world_data.rs).
  await expect(page.getByRole("button", { name: /^Skip/ })).toBeHidden();
  expect(await walk(page, 22, 19)).toBeGreaterThan(0);
});

// #152: below the threshold, the ring stays quiet.
test("Marlo's entrance waits until he is earned", async ({ page }) => {
  await newSave(page);
  await atMarlo(page, ["f:keepers", "f:intro", "f:reveal_bellflats", "f:beat_bramble", "f:beat_t_tobin"], "22,18,n");
  await expect(page.locator("[data-hero]")).toHaveAttribute("data-hero", /^22,18/, { timeout: 15_000 });
  expect(await walk(page, 22, 17)).toBeGreaterThan(0);
  await expect(page.locator("[data-hero]")).toHaveAttribute("data-hero", /^22,17/);
  await expect(page.getByRole("button", { name: /^Skip/ })).toBeHidden();
});

// #152: the first visit shows the site; Skip (a tap on a phone) ends it and gives the hero back.
test.describe(() => {
  test.use({ reducedMotion: "no-preference" }); // the camera eases: the reveal holds long enough to skip
  test("the Bellflats reveal themselves on the first visit, and Skip gives the hero back @phone", async ({ page }) => {
    await newSave(page);
    await editSave(page, `secs.progress = [(secs.progress?.[0] ?? "") + ",f:keepers,f:intro"]; secs.world_at = ["bellflats,9,2,s"];`);
    await page.reload();
    await menuItem(page, "Continue").click();
    await loaded(page);

    const skip = page.getByRole("button", { name: /^Skip/ });
    await expect(skip).toBeVisible();
    expect(await walk(page, 9, 4), "no walking while the site shows itself").toBe(0);
    await skip.click();
    await expect(skip).toBeHidden();
    expect((await readSave(page)).progress?.[0]).toContain("f:reveal_bellflats");
    expect(await walk(page, 9, 4), "the hero is free again").toBeGreaterThan(0);

    // Once only: back again, the map up and drawn (an `enter` scene would have started), no scene.
    await page.reload();
    await menuItem(page, "Continue").click();
    await loaded(page);
    await frames(page);
    await frames(page);
    await expect(skip).toBeHidden();
    expect(await walk(page, 9, 5), "no scene holds the hero").toBeGreaterThan(0);
  });
});

// #151
/** Stand on `map,x,z,facing`, talk to whoever is ahead, and accept their challenge. */
async function challenge(page: Page, name: string, map: string, x: number, z: number) {
  await page.locator(".world-host").evaluate((el, a) => (el as unknown as { sixfoldWarp: (...a: unknown[]) => void }).sixfoldWarp(...a), [map, x, z, "n"]);
  await expect(page.locator("[data-hero]")).toHaveAttribute("data-hero", new RegExp(`^${x},${z}`), { timeout: 15_000 });
  await expect(page.getByRole("button", { name: new RegExp(`^Talk to ${name}`) })).toBeVisible();
  await page.keyboard.press("e");
  // The map's first talk draws the speaker's portrait in a new WebGL context: slow on CI's software GL.
  const talk = page.locator(".talk-box");
  await expect(talk).toBeVisible({ timeout: 60_000 });
  while (await talk.isVisible()) {
    await page.keyboard.press("Enter");
    await page.waitForTimeout(400);
  }
  const offer = page.getByRole("dialog", { name: `Challenge ${name}?` });
  await expect(offer.getByRole("button", { name: /^Challenge/ })).toBeFocused();
  // The panel ignores presses in its first half second (those that ended the lines).
  await page.waitForTimeout(600);
  await page.keyboard.press("Enter");
  // (Not its heading: a seal duel's title card is modal, and hides the table from the accessibility tree.)
  await expect(page.locator(".duel")).toBeVisible();
}

async function leaveDuel(page: Page) {
  await page.getByRole("button", { name: "Duel menu" }).click();
  await page.getByRole("dialog", { name: /menu/i }).getByRole("button", { name: "Leave the duel…" }).click();
  await page.getByRole("alertdialog", { name: /^Leave this duel\?/ }).getByRole("button", { name: "Leave", exact: true }).click();
  await loaded(page);
}

test("a seal duel opens on its title card and wears its region's colour; a route duel does neither", async ({ page }) => {
  page.on("dialog", (d) => d.accept());
  await title(page);
  await menuItem(page, "Learn to play").click();
  await page.getByRole("button", { name: "Skip the lesson" }).click();
  await expect(menuItem(page, "Continue")).toBeVisible();
  // Past the intro and vouched for (the keepers' threshold, #150), the site already shown (#152): on the Bellflats, south of Marlo.
  await editSave(
    page,
    `secs.progress = [(secs.progress?.[0] ?? "") + ",f:intro,f:vouched,f:reveal_bellflats"]; secs.world_at = ["bellflats,22,16,n"];`,
  );
  await page.reload();
  await menuItem(page, "Continue").click();
  await loaded(page);

  await challenge(page, "Marlo", "bellflats", 22, 16);
  const card = page.getByRole("dialog", { name: "Seal of the Tide" });
  await expect(card).toBeVisible();
  await expect(card).toContainText("Marlo");
  await expect(page.locator(".duel")).toHaveClass(/\bseal-duel\b.*\bt-water\b|\bt-water\b.*\bseal-duel\b/);
  // Enter lets the duel begin: the setup's cards come up playable.
  await page.keyboard.press("Enter");
  await expect(card).toBeHidden();
  await expect(page.getByRole("list", { name: /^Your hand/ }).getByRole("button", { name: /, playable$/ }).first()).toBeVisible();
  await leaveDuel(page);

  // Tobin, a route duelist: no card, a plain table.
  await challenge(page, "Tobin", "overworld", 23, 28);
  await expect(page.getByRole("list", { name: /^Your hand/ }).getByRole("button", { name: /, playable$/ }).first()).toBeVisible();
  await expect(page.getByRole("dialog", { name: /^Seal of/ })).toHaveCount(0);
  await expect(page.locator(".duel")).not.toHaveClass(/seal-duel/);
});

// #153: back from the first win, the seal is won on stage; Escape keeps the seal, and it plays once.
test("Marlo's seal is won on stage at the stone, once, and a skip keeps it", async ({ page }) => {
  await newSave(page);
  // What `campaign::record` leaves after the first win (`won:marlo` still to be spoken for).
  const tide = ["f:beat_t_pell", "f:beat_t_nell", "f:beat_t_skye"];
  await atMarlo(page, ["f:keepers", "f:intro", "f:reveal_bellflats", "f:met_marlo", ...tide, "f:beat_marlo", "won:marlo"]);

  // The stone wakes, the Tide face lights (said aloud), and Marlo's farewell, in his own voice.
  const talk = page.getByRole("dialog", { name: "Marlo" });
  await expect(talk).toContainText("The Tide's face has it now", { timeout: 60_000 });
  await expect(page.locator(".world-stage [aria-live]")).toHaveText("Marlo's seal!");
  await page.keyboard.press("Escape");
  await expect(talk).toBeHidden();

  // Skipped: the seal is kept, the duel settled; input is back.
  const progress = (await readSave(page)).progress?.[0] ?? "";
  expect(progress).toContain("f:sealed_marlo");
  expect(progress, "the scene settles the duel it spoke for").not.toContain("won:marlo");
  await expect(page.getByRole("button", { name: /^Skip/ })).toBeHidden();

  // Once only: a reload comes back with no scene.
  await page.reload();
  await menuItem(page, "Continue").click();
  await loaded(page);
  await expect(page.locator("[data-hero]")).toHaveAttribute("data-hero", /^22,16/);
  await expect(page.getByRole("button", { name: /^Skip/ })).toBeHidden();
  await expect(talk).toBeHidden();
});

// #153: a lost seal duel is met kindly, and the challenge stands (no duelist to beat again).
test("after losing to Marlo, he encourages, and the challenge is still open", async ({ page }) => {
  await newSave(page);
  const tide = ["f:beat_t_pell", "f:beat_t_nell", "f:beat_t_skye"];
  await atMarlo(page, ["f:keepers", "f:intro", "f:reveal_bellflats", "f:met_marlo", ...tide, "lost:marlo"]);

  await expect(page.locator("[data-hero]")).toHaveAttribute("data-hero", /^22,16/, { timeout: 15_000 });
  await page.keyboard.press("e");
  // His lines, read to the end: a kind word, and the seal and the duelists bested still waiting.
  const talk = page.getByRole("dialog", { name: "Marlo" });
  await expect(talk).toBeVisible({ timeout: 60_000 });
  const said: string[] = [];
  while (await talk.isVisible()) {
    said.push((await talk.textContent()) ?? "");
    await page.keyboard.press("Enter");
    await page.waitForTimeout(400);
  }
  expect(said.join(" ")).toContain("The seal will wait on the flats, and the Saltreach's duelists you've bested still count.");

  const offer = await talkToMarlo(page);
  await expect(offer.getByRole("button", { name: /^Challenge/ })).toBeEnabled();
  await expect(offer).not.toContainText("bested first");
});

// #154: the Warden's duel is the grandest. With six seals the sealed door rings and Sable speaks through it,
// her room stages her entrance, a skip of either still lands on her challenge, and the duel opens on the champion's card.
test("the sealed door rings for six seals, and Sable's entrance lands on her challenge", async ({ page }) => {
  await newSave(page);
  const seals = ["bramble", "marlo", "cinza", "quill", "orla", "lumen"].map((r) => `f:beat_${r}`);
  await editSave(page, `secs.progress = [(secs.progress?.[0] ?? "") + ",f:intro,f:vouched,${seals.join(",")}"]; secs.world_at = ["shop,2,6,w"];`);
  await page.reload();
  await menuItem(page, "Continue").click();
  await loaded(page);
  await expect(page.locator("[data-hero]")).toHaveAttribute("data-hero", /^2,6/, { timeout: 15_000 });

  // The tile before the door: the bells, then Sable's voice through it. Escape skips.
  expect(await walk(page, 1, 6)).toBeGreaterThan(0);
  const sable = page.getByRole("dialog", { name: "Sable" });
  await expect(sable).toContainText("Come through", { timeout: 60_000 });
  await page.keyboard.press("Escape");
  await expect(sable).toBeHidden();
  expect((await readSave(page)).progress?.[0]).toContain("f:door_warden");

  // Through the door: her entrance plays on arrival; skipped, it still ends on her challenge.
  await expect(page.getByRole("button", { name: /^Skip/ })).toBeHidden();
  expect(await walk(page, 0, 6)).toBeGreaterThan(0);
  await expect(sable).toContainText("Six seals.", { timeout: 60_000 });
  await page.keyboard.press("Escape");
  const offer = page.getByRole("dialog", { name: "Challenge Sable?" });
  const accept = offer.getByRole("button", { name: /^Challenge/ });
  await expect(accept).toBeFocused();
  expect((await readSave(page)).progress?.[0]).toContain("f:met_warden");

  // The champion's card: its own heading, the six seals that opened the door, brass.
  await page.waitForTimeout(600); // the panel ignores presses in its first half second
  await accept.click();
  const card = page.getByRole("dialog", { name: "The Champion's Duel" });
  await expect(card).toBeVisible();
  await expect(card).toContainText("Sable");
  await expect(card.getByRole("img", { name: "Your six seals" }).locator(".seal.earned")).toHaveCount(6);
  await expect(page.locator(".duel")).toHaveClass(/\bgrand-duel\b/);
  await page.keyboard.press("Enter");
  await expect(card).toBeHidden();
});
