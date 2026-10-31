import { test as base, expect, type Locator, type Page } from "@playwright/test";

export { expect };

/**
 * A frame that waits longer than this (`PW_STALL_MS`) fails the test. Off by default: on CI's
 * software GL a passing journey can wait 13 s for its shaders. Set it on a machine with a GPU.
 */
const STALL_MS = Number(process.env.PW_STALL_MS) || Infinity;

/**
 * Known errors that don't fail a journey: each goes in the report as a "known error" instead.
 * - WebKit reports a fetch cut short by a reload as a page error, even one the game catches.
 * - WebKit sometimes fails to read a GLB's embedded texture from its blob: URL (about one run in
 *   three of a world journey): a WebKit bug to chase, kept in sight here rather than failing CI.
 */
const KNOWN = [/due to access control checks\.$/, /^THREE\.GLTFLoader: Couldn't load texture blob:/];

/**
 * Every journey also fails when the page throws or logs an error (a Rust panic lands there too),
 * and notes its longest frame in the report.
 */
export const test = base.extend<{ guard: void }>({
  guard: [
    async ({ page }, use, info) => {
      const errors: string[] = [];
      let worst = 0;
      const report = (text: string, detail = "") => {
        if (KNOWN.some((k) => k.test(text))) info.annotations.push({ type: "known error", description: text });
        else errors.push(text + detail);
      };
      page.on("pageerror", (e) => report(e.message, `\n${e.stack}`));
      page.on("console", (m) => {
        if (m.type() === "error") report(m.text());
      });
      await page.exposeFunction("__stall", (ms: number) => {
        worst = Math.max(worst, ms);
        if (ms > STALL_MS) errors.push(`no frame for ${Math.round(ms)} ms`);
      });
      await page.addInitScript(() => {
        let last = 0;
        const tick = (t: number) => {
          if (last && t - last > 250) (window as unknown as { __stall(ms: number): void }).__stall(t - last);
          last = t;
          requestAnimationFrame(tick);
        };
        requestAnimationFrame(tick);
      });
      await use();
      info.annotations.push({ type: "longest frame", description: `${Math.round(worst)} ms` });
      expect(errors, "the page logged no error and never froze").toEqual([]);
    },
    { auto: true },
  ],
});

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

/**
 * Warp the hero (the world's `sixfoldWarp`) and wait until the save says it stands there. Asked again only while
 * the world is not yet on that tile: a warp asked during another one is ignored, but one asked after it landed and
 * before the save caught up (a frame or more, seconds on CI's software GL) loads the map a second time, and what
 * the first arrival did (a bark said once a visit) is gone.
 */
export async function warpTo(page: Page, map: string, x: number, z: number, f: string) {
  await expect
    .poll(
      async () => {
        const there = await page.evaluate(([map, x, z]) => {
          const s = (document.querySelector(".world-host") as any).sixfoldStats();
          return s.area === map && s.at?.[0] === x && s.at?.[1] === z;
        }, [map, x, z] as const);
        if (!there) await page.evaluate(([map, x, z, f]) => (document.querySelector(".world-host") as any).sixfoldWarp(map, x, z, f), [map, x, z, f] as const);
        return (await readSave(page)).world_at?.[0] ?? "";
      },
      { timeout: 60_000 },
    )
    .toBe(`${map},${x},${z},${f}`);
}
