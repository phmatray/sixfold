// node --test tools/check-tokens.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const check = (css) => {
  const f = join(mkdtempSync(join(tmpdir(), "tokens-")), "style.css");
  writeFileSync(f, css);
  try {
    execFileSync("node", ["tools/check-tokens.mjs", f], { stdio: "pipe" });
    return "";
  } catch (e) {
    return String(e.stderr);
  }
};
const root = ":root {\n  --a: #123456;\n}\n";

test("tokens, definitions and fluid sizes pass", () => {
  assert.equal(check(`${root}.x { color: var(--a); --b: #123456; font-size: var(--fs-md); }\n.y { font-size: clamp(12px, 2vw, 16px); font: 700 1em serif; }\n`), "");
});

test("a raw colour a token covers, at any alpha, and bare px sizes fail", () => {
  const out = check(`${root}.x { color: #123456; background: rgba(18, 52, 86, 0.5); font-size: 14px; }\n.y { font: 700 13px/1 serif; }\n`);
  assert.match(out, /#123456 is --a/);
  assert.match(out, /rgba\(18, 52, 86…\) is --a/);
  assert.match(out, /font-size 14px/);
  assert.match(out, /font 13px/);
});
