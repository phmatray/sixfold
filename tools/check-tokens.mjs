// The visual grammar's guard (DESIGN.md): outside a custom property definition,
// web/style.css may not use a raw colour that a :root colour token covers (at
// any alpha: use color-mix with the token), nor a font size in bare pixels
// (use the type ramp, --fs-*, or a fluid clamp/em/cqw size).
//   node tools/check-tokens.mjs [web/style.css]
import { readFileSync } from "node:fs";

const file = process.argv[2] ?? "web/style.css";
// Comments out, line numbers kept.
const css = readFileSync(file, "utf8").replace(/\/\*[\s\S]*?\*\//g, (c) => c.replace(/[^\n]/g, " "));
const lines = css.split("\n");

const start = lines.findIndex((l) => /^:root\s*{/.test(l));
const end = lines.findIndex((l, i) => i > start && /^}/.test(l));
if (start < 0 || end < 0) throw new Error(`${file}: no :root block`);

const hex = (h) => {
  let s = h.slice(1).toLowerCase();
  if (s.length <= 4) s = [...s].map((c) => c + c).join("");
  return [0, 2, 4].map((i) => parseInt(s.slice(i, i + 2), 16)).join(",");
};
// Opaque colour tokens, by their r,g,b.
const tokens = new Map();
for (const l of lines.slice(start + 1, end)) {
  const m = /^\s*(--[\w-]+):\s*(#[0-9a-f]{6})\s*;/i.exec(l);
  if (m && !tokens.has(hex(m[2]))) tokens.set(hex(m[2]), m[1]);
}

const bad = [];
lines.forEach((line, i) => {
  if (i >= start && i <= end) return;
  // Token definitions (anywhere) and data URIs may hold raw values.
  const l = line.replace(/--[\w-]+\s*:[^;]*;?/g, "").replace(/url\([^)]*\)/g, "");
  const at = `${file}:${i + 1}`;
  for (const m of l.matchAll(/#(?:[0-9a-f]{8}|[0-9a-f]{6}|[0-9a-f]{3,4})\b/gi)) {
    // #rgba and #rrggbbaa: the colour without its alpha.
    const k = hex(m[0].length === 5 || m[0].length === 9 ? m[0].slice(0, m[0].length === 5 ? 4 : 7) : m[0]);
    if (tokens.has(k)) bad.push(`${at}: ${m[0]} is ${tokens.get(k)}`);
  }
  for (const m of l.matchAll(/rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)/g)) {
    const k = `${m[1]},${m[2]},${m[3]}`;
    if (tokens.has(k)) bad.push(`${at}: ${m[0]}…) is ${tokens.get(k)} (color-mix it)`);
  }
  for (const m of l.matchAll(/font-size:\s*([\d.]+px)\s*(?:!important)?\s*[;}]/g)) bad.push(`${at}: font-size ${m[1]} (use --fs-*)`);
  for (const m of l.matchAll(/\bfont:\s*([^;}]*)/g)) {
    const px = /(?:^|\s)([\d.]+)px(?=[\s/])/.exec(m[1]);
    // Under 6 px it is an SVG's user units (the atlas compass), not a reading size.
    if (px && Number(px[1]) >= 6) bad.push(`${at}: font ${px[1]}px (use --fs-*)`);
  }
});

if (bad.length) {
  console.error(bad.join("\n"));
  console.error(`\n${bad.length} raw value(s) a token covers (DESIGN.md).`);
  process.exit(1);
}
console.log(`check-tokens: ${file} uses its tokens (${tokens.size} colours, type ramp).`);
