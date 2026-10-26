import { expect, type Page } from "@playwright/test";

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
  await expect(page.getByRole("navigation", { name: "Main menu" })).toBeVisible();
}

/** A title menu entry, by its label (each entry also carries a subtitle). */
export function menuItem(page: Page, label: string) {
  return page.getByRole("navigation", { name: "Main menu" }).getByRole("button", { name: new RegExp(`^${label}`) });
}
