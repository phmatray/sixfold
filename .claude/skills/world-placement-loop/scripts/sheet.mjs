// node sheet.mjs <spots.json> <out dir> [before|after]
// Contact sheets of the shots, four to a page (2 × 2, each pane captioned with its spot), in <out>/sheets/: a quick
// look at many spots in one image before deciding which deserve a fix, a rule change, or nothing.
import { readFileSync, writeFileSync, mkdirSync, existsSync } from "node:fs";
import { playwright, readJson } from "./lib.mjs";

const [spotsPath, out, phase = "before"] = process.argv.slice(2), spots = readJson(spotsPath);
mkdirSync(`${out}/sheets`, { recursive: true });
const { chromium } = await playwright();
const browser = await chromium.launch(), page = await browser.newPage();
await page.setContent("<canvas></canvas>");
const have = spots.filter((s) => existsSync(`${out}/${phase}/${s.map}-${s.block}.png`));
for (let i = 0; i < have.length; i += 4) {
  const group = have.slice(i, i + 4), file = `${out}/sheets/${phase}-${i / 4}.jpg`;
  const panes = group.map((s) => [`data:image/png;base64,${readFileSync(`${out}/${phase}/${s.map}-${s.block}.png`).toString("base64")}`, `#${s.block} ${s.map} · ${s.rule} · ${s.members.map((m) => `${m.x},${m.z}`).join(" ")}`]);
  const url = await page.evaluate(async (panes) => {
    const load = (src) => new Promise((ok) => { const i = new Image(); i.onload = () => ok(i); i.src = src; });
    const w = 800, h = 500, c = document.querySelector("canvas"), g = c.getContext("2d");
    c.width = w * 2; c.height = h * 2;
    g.fillStyle = "#111"; g.fillRect(0, 0, c.width, c.height);
    for (const [k, [src, label]] of panes.entries()) {
      const img = await load(src), x = (k % 2) * w, y = Math.floor(k / 2) * h;
      g.drawImage(img, x, y, w, h);
      g.fillStyle = "rgba(0,0,0,.7)"; g.fillRect(x, y + h - 30, w, 30);
      g.fillStyle = "#fff"; g.font = "17px sans-serif"; g.fillText(label, x + 10, y + h - 9);
    }
    return c.toDataURL("image/jpeg", 0.8);
  }, panes);
  writeFileSync(file, Buffer.from(url.split(",")[1], "base64"));
  console.log(`${file}: ${group.map((s) => `#${s.block} ${s.map}`).join(" | ")}`);
}
await browser.close();
