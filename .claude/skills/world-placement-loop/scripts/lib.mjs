// Shared by the loop's scripts: where the repo is, where Playwright is, and the game's URL.
import { existsSync, readFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

// .claude/skills/world-placement-loop/scripts → the repo (or worktree) root.
export const REPO = resolve(dirname(fileURLToPath(import.meta.url)), "../../../..");
export const WORLD = `${REPO}/web/world`;
export const GAME = process.env.GAME ?? "http://localhost:8080";

// e2e's Playwright: in this checkout, or in the main checkout when this is a worktree without node_modules.
export async function playwright() {
  const tried = [process.env.PLAYWRIGHT, `${REPO}/e2e/node_modules/playwright/index.mjs`];
  try {
    const common = execFileSync("git", ["-C", REPO, "rev-parse", "--path-format=absolute", "--git-common-dir"], { encoding: "utf8" }).trim();
    tried.push(`${dirname(common)}/e2e/node_modules/playwright/index.mjs`);
  } catch {}
  const at = tried.find((p) => p && existsSync(p));
  if (!at) throw new Error(`no Playwright: run \`npm ci\` in e2e/ or set PLAYWRIGHT=<path to playwright/index.mjs> (tried ${tried.filter(Boolean).join(", ")})`);
  return import(at);
}

export const readJson = (path) => JSON.parse(readFileSync(path, "utf8"));
export const mapFile = (name) => `${WORLD}/maps/${name}.json`;
// The game turns every prop by its rot (build.js, degToRad): a prop without one is never drawn. Write maps like this.
export const mapText = (m) => JSON.stringify(m, null, 2) + "\n";
