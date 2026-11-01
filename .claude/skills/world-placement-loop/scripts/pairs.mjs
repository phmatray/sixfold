// node pairs.mjs <spots.json> <out dir>
// For every spot shot before and after: a strip "BEFORE | AFTER | CHANGED (red)" in <out>/pairs/<map>-<block>.jpg and
// the share of pixels that changed (the HUD left out), printed and saved to <out>/summary.json, least changed first.
// A strip with almost nothing red where you expected a move means the fix didn't land (or the prop is hidden).
// Grass blades flicker between shots: scattered red specks across the ground are noise, a solid red shape is a prop.
import { readFileSync, writeFileSync, mkdirSync, existsSync } from "node:fs";
import { playwright, readJson } from "./lib.mjs";

const [spotsPath, out] = process.argv.slice(2), spots = readJson(spotsPath);
mkdirSync(`${out}/pairs`, { recursive: true });
const { chromium } = await playwright();
const browser = await chromium.launch(), page = await browser.newPage();
await page.setContent("<canvas></canvas>");
const res = [];
for (const s of spots) {
  const name = `${s.map}-${s.block}`, a = `${out}/before/${name}.png`, b = `${out}/after/${name}.png`;
  if (!existsSync(a) || !existsSync(b)) { res.push({ name, rule: s.rule, missing: true }); continue; }
  const r = await page.evaluate(async ([a, b, label]) => {
    const load = (src) => new Promise((ok) => { const i = new Image(); i.onload = () => ok(i); i.src = src; });
    const [A, B] = await Promise.all([load(a), load(b)]), W = A.width, H = A.height;
    const px = (img) => { const c = new OffscreenCanvas(W, H), g = c.getContext("2d"); g.drawImage(img, 0, 0); return g.getImageData(0, 0, W, H).data; };
    const da = px(A), db = px(B), mask = new ImageData(W, H);
    let changed = 0, counted = 0;
    for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
      const i = (y * W + x) * 4, hud = y < 64 || (y > H - 60 && x < 160);
      const on = !hud && Math.abs(da[i] - db[i]) + Math.abs(da[i + 1] - db[i + 1]) + Math.abs(da[i + 2] - db[i + 2]) > 60;
      if (!hud) counted++;
      if (on) changed++;
      mask.data[i] = on ? 255 : da[i] * 0.35; mask.data[i + 1] = on ? 40 : da[i + 1] * 0.35; mask.data[i + 2] = on ? 40 : da[i + 2] * 0.35; mask.data[i + 3] = 255;
    }
    const w = W / 2, h = H / 2, strip = document.querySelector("canvas");
    strip.width = w * 3; strip.height = h + 28;
    const g = strip.getContext("2d");
    g.fillStyle = "#111"; g.fillRect(0, 0, strip.width, strip.height);
    g.drawImage(A, 0, 28, w, h); g.drawImage(B, w, 28, w, h);
    const mc = new OffscreenCanvas(W, H); mc.getContext("2d").putImageData(mask, 0, 0); g.drawImage(mc, w * 2, 28, w, h);
    g.fillStyle = "#fff"; g.font = "15px sans-serif";
    g.fillText("BEFORE", 8, 19); g.fillText("AFTER", w + 8, 19); g.fillText(`CHANGED (red): ${(changed / counted * 100).toFixed(2)} %   ${label}`, w * 2 + 8, 19);
    return { ratio: changed / counted, strip: strip.toDataURL("image/jpeg", 0.8) };
  }, [`data:image/png;base64,${readFileSync(a).toString("base64")}`, `data:image/png;base64,${readFileSync(b).toString("base64")}`, `${name} ${s.rule}`]);
  writeFileSync(`${out}/pairs/${name}.jpg`, Buffer.from(r.strip.split(",")[1], "base64"));
  res.push({ name, rule: s.rule, changed: +(r.ratio * 100).toFixed(2) });
}
res.sort((x, y) => (x.changed ?? -1) - (y.changed ?? -1));
writeFileSync(`${out}/summary.json`, JSON.stringify(res, null, 1));
for (const o of res) console.log(`${String(o.changed ?? "—").padStart(6)} %  ${o.name}  ${o.rule}${o.missing ? "  (missing shot)" : ""}`);
await browser.close();
