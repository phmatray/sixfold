import { expect, type Locator, type Page } from "@playwright/test";

/**
 * The save in slot `n` as the game stored it (`sixfold.save.<n>` in localStorage):
 * its sections by name, each a list of lines (unescaped). `{}` while the slot is empty.
 */
export async function readSave(page: Page, n = 1): Promise<Record<string, string[]>> {
  const text = await page.evaluate((k) => localStorage.getItem(k), `sixfold.save.${n}`);
  const out: Record<string, string[]> = {};
  let cur: string[] | undefined;
  for (const line of (text ?? "").split("\n").slice(1)) {
    const m = /^\[([a-z0-9_]+)\]$/.exec(line);
    if (m) out[m[1]] = cur = [];
    else cur?.push(line.startsWith("\\") ? line.slice(1) : line);
  }
  return out;
}

/** Copies owned, from a save's `[collection]` (`v1;<id>:<count>,...;<checksum>`). */
export function owned(save: Record<string, string[]>): number {
  const body = save.collection?.[0]?.split(";")[1] ?? "";
  return body ? body.split(",").reduce((n, e) => n + Number(e.split(":")[1]), 0) : 0;
}

/** The opening (studio, logo, "Press to start") already seen this launch: straight to the title. */
export async function skipSplash(page: Page) {
  await page.addInitScript(() => sessionStorage.setItem("sixfold.splashed", "1"));
}

/** Open the game on its title screen (a new player unless the context already has a save). */
export async function title(page: Page) {
  await skipSplash(page);
  await page.goto("/");
  await expect(page.getByRole("navigation", { name: "Main menu" })).toBeVisible({ timeout: 30_000 }); // the first load compiles the wasm: slower on a busy machine
}

/** A title menu entry, by its label (each entry also carries a subtitle). */
export function menuItem(page: Page, label: string) {
  return page.getByRole("navigation", { name: "Main menu" }).getByRole("button", { name: new RegExp(`^${label}`) });
}

/**
 * Press `key` until `target` has focus. One key, one move: after each press it waits (frame by frame,
 * at most a second) for focus to leave where it was, so a slow frame never has two keys land on the
 * same control, nor a key land before the app moved focus itself.
 */
export async function moveTo(page: Page, target: Locator, key = "Tab", tries = 60) {
  const here = () => target.evaluate((el) => el === document.activeElement, undefined, { timeout: 2_000 }).catch(() => false);
  for (let i = 0; i < tries; i++) {
    if (await here()) return;
    await page.evaluate(() => ((window as unknown as { was: Element | null }).was = document.activeElement));
    await page.keyboard.press(key);
    await page
      .waitForFunction(() => document.activeElement !== (window as unknown as { was: Element | null }).was, null, { polling: "raf", timeout: 1_000 })
      .catch(() => {});
  }
  await expect(target).toBeFocused();
}

/**
 * Two frames drawn: the page's main thread is free again. On CI's software GL the first frames
 * after a warp or a map build compile shaders and can hold the thread for many seconds, and a
 * click sent then times out waiting for the page; wait for the frames (on the test's clock) first.
 */
export async function frames(page: Page) {
  await page.evaluate(() => new Promise<void>((r) => requestAnimationFrame(() => requestAnimationFrame(() => r()))));
}

/**
 * Every bust on this map drawn (world.js bakes them ahead of a talk, one at a time): on CI's software GL each
 * holds the page for seconds, and a click sent then times out. None is drawn while a menu is open.
 */
export async function bustsDrawn(page: Page) {
  await page.waitForFunction(() => (document.querySelector(".world-host") as unknown as { sixfoldStats: () => { busts: number } }).sixfoldStats().busts === 0, null, { timeout: 0 });
}

/**
 * Rewrite slot 1 the way the game stores it: `sixfold-save;1;<fnv1a>` over the
 * section lines (engine `save.rs`). `edit` changes the sections in place.
 */
export async function editSave(page: Page, edit: string) {
  await page.evaluate((edit) => {
    const fnv = (s: string) => {
      let h = 0x811c9dc5;
      for (const b of new TextEncoder().encode(s)) h = Math.imul(h ^ b, 0x01000193) >>> 0;
      return h >>> 0;
    };
    const secs: Record<string, string[]> = {};
    let cur: string[] = [];
    for (const l of (localStorage.getItem("sixfold.save.1") ?? "").split("\n").slice(1)) {
      const m = /^\[([a-z0-9_]+)\]$/.exec(l);
      if (m) secs[m[1]] = cur = [];
      else cur.push(l);
    }
    new Function("secs", edit)(secs);
    const body = Object.entries(secs).flatMap(([k, v]) => [`[${k}]`, ...v]).join("\n");
    localStorage.setItem("sixfold.save.1", `sixfold-save;1;${fnv(body).toString(16).padStart(8, "0")}\n${body}`);
    localStorage.removeItem("sixfold.save.1.prev");
  }, edit);
}

/** The world is up: its menu button shown, the loading status gone. */
export async function loaded(page: Page) {
  await expect(page.getByRole("button", { name: "Menu", exact: true })).toBeVisible();
  await expect(page.getByRole("status").filter({ hasText: "Loading the world" })).toBeHidden({ timeout: 60_000 });
}
