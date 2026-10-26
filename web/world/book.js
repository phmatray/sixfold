// Books (#104): one taken from a shelf flies to the camera, opens and turns its pages.
// It is drawn in a small scene of its own over the world (after the world's passes, on a
// cleared depth buffer), so the room stays where it was, dimmed behind it. The book is
// procedural: two covers on a hinge, two page blocks, a spine, and one leaf that curls
// over when a page turns. Pages are canvases painted from the text Rust hands over
// (`open`): on a wide view a spread is the heading and picture on the left, the text on
// the right; on a tall one (a phone) the right page carries all three and is centred.
// Painted at the reader's text size (`--ts`), shrunk only as far as a page needs to fit.
// Rust keeps the same text in the DOM for screen readers; this file only draws.
import * as THREE from "three";
import { toon } from "./props.js";
import { tr } from "./map.js";

const PW = 1, PH = 1.36; // a page, in book units (the cover is 3 % larger)
const BLOCK = 0.06, SKIN = 0.024; // a page block's and a cover's thickness
const CW = PW * 1.03, CH = PH * 1.03;
const FOV = 30;
// Leather and gilt per shelf; the ink of a page's heading.
const COVER = {
  rules: { leather: "#7a2e28", gilt: "#e8c46a", ink: "#7a2e28" },
  world: { leather: "#2c5a4c", gilt: "#e8c46a", ink: "#2c5a4c" },
  tales: { leather: "#4a3a6c", gilt: "#f0d9a0", ink: "#4a3a6c" },
};
const PAPER = "#f3e9d0", INK = "#2a2018", FAINT = "rgba(42, 32, 24, 0.55)";
// The six faces and the blank seventh (the atlas's region colours).
const TYPE = { ember: "#e1572f", tide: "#2f7fd0", grove: "#3f9a52", storm: "#d9a315", stone: "#a8642f", dream: "#8a4fc9", neutral: "#8d8473" };
const DUR = { take: 0.32, fly: 0.62, open: 0.55, turn: 0.6, close: 0.36, back: 0.5 }; // seconds
const DISPLAY = "'Bricolage Grotesque', 'Avenir Next', system-ui, sans-serif";
const BODY = "'Atkinson Hyperlegible', system-ui, sans-serif";
const BODY_PX = 19; // the page text's size on screen at 100 %, in CSS pixels

const ease = (t) => (t < 0.5 ? 4 * t * t * t : 1 - (-2 * t + 2) ** 3 / 2);
const clamp01 = (t) => Math.min(1, Math.max(0, t));
const lerp = THREE.MathUtils.lerp;
const textScale = () => parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--ts")) || 1;

// ---------- Painting ----------

const speckle = (() => {
  let c = null;
  return () => {
    if (c) return c;
    c = document.createElement("canvas");
    c.width = c.height = 96;
    const g = c.getContext("2d");
    let s = 7;
    const r = () => ((s = (s * 16807) % 2147483647) / 2147483647);
    for (let i = 0; i < 520; i++) {
      g.fillStyle = `rgba(${r() > 0.5 ? "120, 90, 50" : "255, 255, 255"}, ${0.03 + r() * 0.05})`;
      g.fillRect(r() * 96, r() * 96, 1 + r() * 1.5, 1 + r() * 1.5);
    }
    return c;
  };
})();

function lines(g, text, width) {
  const out = [];
  for (const para of text.split("\n")) {
    let line = "";
    for (const word of para.split(/\s+/).filter(Boolean)) {
      const next = line ? `${line} ${word}` : word;
      if (line && g.measureText(next).width > width) out.push(line), (line = word);
      else line = next;
    }
    out.push(line);
  }
  return out;
}

const hexPath = (g, cx, cy, r) => {
  g.beginPath();
  for (let i = 0; i < 6; i++) {
    const a = Math.PI / 6 + (i * Math.PI) / 3;
    g[i ? "lineTo" : "moveTo"](cx + r * Math.cos(a), cy + r * Math.sin(a));
  }
  g.closePath();
};

// A type's glyph (a 24-unit SVG path) in a coloured disc.
function glyph(g, path, color, cx, cy, r) {
  g.save();
  g.fillStyle = color;
  g.beginPath();
  g.arc(cx, cy, r, 0, Math.PI * 2);
  g.fill();
  g.strokeStyle = "rgba(42, 32, 24, 0.35)";
  g.lineWidth = Math.max(1, r * 0.05);
  g.stroke();
  if (path) {
    const k = (r * 1.15) / 24;
    g.translate(cx - 12 * k, cy - 12 * k);
    g.scale(k, k);
    g.fillStyle = "#fffaf0";
    g.fill(new Path2D(path), "evenodd");
  }
  g.restore();
}

// A duel table, seen from above, labelled.
function board(g, x, y, w, h) {
  const k = Math.min(w / 10, h / 7.4);
  const ox = x + (w - k * 10) / 2, oy = y + (h - k * 7.4) / 2;
  g.save();
  g.fillStyle = "#24543f";
  g.beginPath();
  g.roundRect(ox, oy, k * 10, k * 7.4, k * 0.5);
  g.fill();
  const cw = k * 1.05, ch = cw * 1.4;
  const slot = (cx, cy, fill) => {
    g.fillStyle = fill ?? "rgba(255, 255, 255, 0.14)";
    g.strokeStyle = "rgba(255, 250, 235, 0.7)";
    g.lineWidth = Math.max(1, k * 0.05);
    g.beginPath();
    g.roundRect(cx - cw / 2, cy - ch / 2, cw, ch, k * 0.12);
    g.fill();
    g.stroke();
  };
  const label = (t, cx, cy) => {
    g.fillStyle = "#f6efd9";
    g.font = `700 ${k * 0.46}px ${BODY}`;
    g.textAlign = "center";
    g.fillText(t, cx, cy);
  };
  for (let i = 0; i < 6; i++) slot(ox + k * (0.85 + (i % 2) * 1.1), oy + k * (1.05 + Math.floor(i / 2) * 1.6), "#c9b48a");
  label(tr("Prizes", "Récompenses"), ox + k * 1.4, oy + k * 6.0);
  slot(ox + k * 5.45, oy + k * 1.85, "#e9dcc0");
  label(tr("Active", "Actif"), ox + k * 5.45, oy + k * 3.25);
  for (let i = 0; i < 5; i++) slot(ox + k * (3.15 + i * 1.15), oy + k * 5.4);
  label(tr("Bench", "Banc"), ox + k * 5.45, oy + k * 6.85);
  slot(ox + k * 9.0, oy + k * 1.85, "#5a4636");
  label(tr("Deck", "Deck"), ox + k * 9.0, oy + k * 3.25);
  slot(ox + k * 9.0, oy + k * 5.4);
  label(tr("Discard", "Défausse"), ox + k * 9.0, oy + k * 6.85);
  g.restore();
}

// The picture of a page in the box (x, y, w, h); `ready` once an image has loaded.
function picture(g, art, x, y, w, h, images, ready) {
  if (!art || w < 8 || h < 8) return 0;
  const cx = x + w / 2, cy = y + h / 2;
  if (art.kind === "img") {
    let im = images.get(art.src);
    if (!im) {
      im = new Image();
      im.decoding = "async";
      im.onload = ready;
      im.src = art.src;
      images.set(art.src, im);
    }
    const iw = Math.min(w, h * (10 / 7)), ih = iw * 0.7;
    const ix = cx - iw / 2, iy = y + (h - ih) / 2;
    g.save();
    g.fillStyle = "rgba(42, 32, 24, 0.12)";
    g.fillRect(ix + iw * 0.015, iy + iw * 0.02, iw, ih);
    g.fillStyle = "#e6dcc4";
    g.fillRect(ix, iy, iw, ih);
    if (im.complete && im.naturalWidth) g.drawImage(im, ix, iy, iw, ih);
    g.strokeStyle = INK;
    g.lineWidth = Math.max(1.5, iw * 0.006);
    g.strokeRect(ix, iy, iw, ih);
    g.restore();
    return ih;
  }
  if (art.kind === "face") {
    const r = Math.min(w, h) * 0.36;
    g.save();
    g.globalAlpha = 0.18;
    g.fillStyle = TYPE[art.face] ?? INK;
    g.beginPath();
    g.arc(cx, cy, r * 1.32, 0, Math.PI * 2);
    g.fill();
    g.restore();
    glyph(g, art.path, TYPE[art.face] ?? INK, cx, cy, r);
    return h;
  }
  if (art.kind === "faces") {
    const R = Math.min(w, h) * 0.34, r = R * 0.36;
    g.save();
    g.fillStyle = "#fbf6e8";
    g.strokeStyle = "rgba(42, 32, 24, 0.45)";
    g.lineWidth = Math.max(1, R * 0.02);
    hexPath(g, cx, cy, r * 1.05);
    g.fill();
    g.stroke();
    g.restore();
    art.faces.forEach((f, i) => {
      const a = -Math.PI / 2 + (i * Math.PI) / 3;
      glyph(g, f.path, TYPE[f.face] ?? INK, cx + R * Math.cos(a), cy + R * Math.sin(a), r);
    });
    return h;
  }
  if (art.kind === "board") {
    board(g, x, y, w, h);
    return h;
  }
  return 0;
}

// One page. `side` "L" or "R"; `spread` a page of the book ({heading, text, art}) or null
// (the endpapers); `single`: the tall layout (the right page carries everything).
function paintPage(c, { side, spread, single, num, family, ts, cssH, images, ready }) {
  const g = c.getContext("2d"), W = c.width, H = c.height;
  const look = COVER[family] ?? COVER.world;
  g.save();
  g.fillStyle = PAPER;
  g.fillRect(0, 0, W, H);
  g.fillStyle = g.createPattern(speckle(), "repeat");
  g.fillRect(0, 0, W, H);
  // The gutter's shade and a darker outer edge.
  const inner = side === "R" ? 0 : W;
  const gut = g.createLinearGradient(inner, 0, side === "R" ? W * 0.16 : W * 0.84, 0);
  gut.addColorStop(0, "rgba(70, 48, 24, 0.30)");
  gut.addColorStop(1, "rgba(70, 48, 24, 0)");
  g.fillStyle = gut;
  g.fillRect(0, 0, W, H);
  const edge = g.createLinearGradient(W - inner, 0, side === "R" ? W * 0.94 : W * 0.06, 0);
  edge.addColorStop(0, "rgba(70, 48, 24, 0.10)");
  edge.addColorStop(1, "rgba(70, 48, 24, 0)");
  g.fillStyle = edge;
  g.fillRect(0, 0, W, H);

  const px = H / cssH; // canvas pixels per CSS pixel on screen
  const mo = W * 0.1, mi = W * 0.12, top = H * 0.075, bottom = H * 0.085;
  const x0 = side === "R" ? mi : mo, x1 = W - (side === "R" ? mo : mi), tw = x1 - x0;
  g.textBaseline = "alphabetic";
  if (!spread) {
    // Endpapers: a gilt hexagon, nothing to read.
    g.strokeStyle = look.ink;
    g.globalAlpha = 0.25;
    g.lineWidth = W * 0.006;
    hexPath(g, W / 2, H / 2, W * 0.16);
    g.stroke();
    g.restore();
    return;
  }
  if (num) {
    g.fillStyle = FAINT;
    g.font = `italic 400 ${Math.round(13 * px)}px ${BODY}`;
    g.textAlign = "center";
    g.fillText(String(num), (x0 + x1) / 2, H - bottom * 0.45);
  }
  const heading = (y, size) => {
    g.fillStyle = look.ink;
    g.font = `700 ${size}px ${DISPLAY}`;
    g.textAlign = "left";
    const hl = lines(g, spread.heading, tw);
    for (const l of hl) (y += size * 1.12), g.fillText(l, x0, y);
    y += size * 0.45;
    g.strokeStyle = look.ink;
    g.globalAlpha = 0.5;
    g.lineWidth = Math.max(1, size * 0.06);
    g.beginPath();
    g.moveTo(x0, y);
    g.lineTo(x0 + tw * 0.22, y);
    g.stroke();
    g.globalAlpha = 1;
    return y + size * 0.6;
  };
  // The body, at the reader's size; smaller only as far as it must be to fit in `room`.
  const body = (y, room) => {
    let size = BODY_PX * ts * px, out;
    for (;;) {
      g.font = `400 ${size}px ${BODY}`;
      out = lines(g, spread.text, tw);
      if (out.length * size * 1.45 <= room || size < 11 * px) break;
      size *= 0.94;
    }
    g.fillStyle = INK;
    g.textAlign = "left";
    for (const l of out) (y += size * 1.45), g.fillText(l, x0, y);
    return size;
  };
  const hSize = Math.round(Math.max(26 * px, 24 * ts * px));
  if (side === "L" && !single) {
    const y = heading(top, hSize);
    picture(g, spread.art, x0, y + H * 0.02, tw, H - bottom - y - H * 0.06, images, ready);
  } else if (side === "L") {
    // Tall layout: the left page is mostly out of view; its picture keeps it from looking blank.
    picture(g, spread.art, x0, top, tw, H - top - bottom, images, ready);
  } else if (single) {
    let y = heading(top, hSize);
    const room = H - bottom - y;
    // Room for the picture only if the text still fits at the reader's size beside it.
    g.font = `400 ${BODY_PX * ts * px}px ${BODY}`;
    const need = lines(g, spread.text, tw).length * BODY_PX * ts * px * 1.45;
    const pic = spread.art ? Math.min(room - need - H * 0.02, H * 0.26) : 0;
    if (pic > H * 0.1) y += picture(g, spread.art, x0, y, tw, pic, images, ready) + H * 0.025;
    body(y, H - bottom - y);
  } else body(top, H - top - bottom);
  g.restore();
}

function paintCover(c, title, family) {
  const g = c.getContext("2d"), W = c.width, H = c.height, look = COVER[family] ?? COVER.world;
  g.fillStyle = look.leather;
  g.fillRect(0, 0, W, H);
  g.fillStyle = g.createPattern(speckle(), "repeat");
  g.fillRect(0, 0, W, H);
  const shade = g.createRadialGradient(W * 0.45, H * 0.4, W * 0.1, W * 0.5, H * 0.5, W * 0.85);
  shade.addColorStop(0, "rgba(255, 255, 255, 0.08)");
  shade.addColorStop(1, "rgba(0, 0, 0, 0.28)");
  g.fillStyle = shade;
  g.fillRect(0, 0, W, H);
  g.strokeStyle = look.gilt;
  g.lineWidth = W * 0.012;
  g.strokeRect(W * 0.07, H * 0.055, W * 0.86, H * 0.89);
  g.lineWidth = W * 0.005;
  g.strokeRect(W * 0.1, H * 0.075, W * 0.8, H * 0.85);
  g.lineWidth = W * 0.012;
  hexPath(g, W / 2, H * 0.3, W * 0.13);
  g.stroke();
  hexPath(g, W / 2, H * 0.3, W * 0.07);
  g.fillStyle = look.gilt;
  g.fill();
  g.fillStyle = look.gilt;
  g.textAlign = "center";
  let size = W * 0.095;
  g.font = `700 ${size}px ${DISPLAY}`;
  let ls = lines(g, title, W * 0.7);
  while (ls.length > 3 && size > W * 0.06) (size *= 0.9), (g.font = `700 ${size}px ${DISPLAY}`), (ls = lines(g, title, W * 0.7));
  ls.forEach((l, i) => g.fillText(l, W / 2, H * 0.53 + i * size * 1.15));
}

function stripes(across) {
  const c = document.createElement("canvas");
  c.width = c.height = 64;
  const g = c.getContext("2d");
  g.fillStyle = "#efe3c4";
  g.fillRect(0, 0, 64, 64);
  g.fillStyle = "rgba(120, 95, 60, 0.35)";
  for (let i = 0; i < 64; i += 4) across ? g.fillRect(i, 0, 1, 64) : g.fillRect(0, i, 64, 1);
  const t = new THREE.CanvasTexture(c);
  t.colorSpace = THREE.SRGBColorSpace;
  return t;
}

// ---------- The book ----------

export function createBook({ reduced, sound }) {
  const scene = new THREE.Scene();
  const cam = new THREE.PerspectiveCamera(FOV, 1, 0.05, 60);
  scene.add(cam);
  scene.add(new THREE.HemisphereLight("#fff4df", "#5a4632", 1.7));
  const sun = new THREE.DirectionalLight("#fff1d8", 1.6);
  sun.position.set(-2, 4, 5);
  scene.add(sun);
  // The world behind, dimmed: a sheet far behind the book, filling the view.
  const DIM_AT = 50;
  const dim = new THREE.Mesh(new THREE.PlaneGeometry(1, 1), new THREE.MeshBasicMaterial({ color: "#0b0805", transparent: true, opacity: 0, depthWrite: false }));
  dim.position.z = -DIM_AT;
  cam.add(dim);

  const edgesU = stripes(true), edgesV = stripes(false);
  const paper = toon("#efe3c4");
  const blockMats = [new THREE.MeshToonMaterial({ map: edgesU }), new THREE.MeshToonMaterial({ map: edgesU }), new THREE.MeshToonMaterial({ map: edgesV }), new THREE.MeshToonMaterial({ map: edgesV }), paper, paper];
  const leather = toon("#7a2e28");
  const coverCanvas = document.createElement("canvas");
  coverCanvas.width = 512;
  coverCanvas.height = Math.round(512 * (CH / CW));
  const coverTex = new THREE.CanvasTexture(coverCanvas);
  coverTex.colorSpace = THREE.SRGBColorSpace;

  // The hinge: `right` lies still; `left` turns about the spine (y axis) from π (shut,
  // lying on top) to 0 (open flat), rising by two blocks so it lands on the right half.
  const root = new THREE.Group(), lift = new THREE.Group(), right = new THREE.Group(), left = new THREE.Group();
  root.add(lift);
  lift.add(right, left);
  const half = (g, sign) => {
    const cover = new THREE.Mesh(new THREE.BoxGeometry(CW, CH, SKIN), leather);
    cover.position.set((sign * CW) / 2, 0, -SKIN / 2);
    const block = new THREE.Mesh(new THREE.BoxGeometry(PW * 0.98, PH * 0.98, BLOCK), blockMats);
    block.position.set(sign * (PW * 0.49 + 0.004), 0, BLOCK / 2);
    const face = new THREE.Mesh(new THREE.PlaneGeometry(PW * 0.98, PH * 0.98), new THREE.MeshBasicMaterial({ color: "#ffffff" }));
    face.position.set(sign * (PW * 0.49 + 0.004), 0, BLOCK + 0.0015);
    g.add(cover, block, face);
    return face;
  };
  const faceR = half(right, 1), faceL = half(left, -1);
  // The front cover's outside, with the title (the left cover's back).
  const title = new THREE.Mesh(new THREE.PlaneGeometry(CW, CH), new THREE.MeshToonMaterial({ map: coverTex }));
  title.rotation.y = Math.PI;
  title.position.set(-CW / 2, 0, -SKIN - 0.0015);
  left.add(title);
  const spine = new THREE.Mesh(new THREE.BoxGeometry(SKIN, CH, 1), leather);
  lift.add(spine);
  // The turning leaf: a strip of columns bent about the spine; front and back faces.
  const COLS = 22;
  const leafGeo = new THREE.PlaneGeometry(PW * 0.98, PH * 0.98, COLS, 1);
  leafGeo.translate(PW * 0.49, 0, 0);
  const rest = Float32Array.from(leafGeo.attributes.position.array);
  const leafFront = new THREE.Mesh(leafGeo, new THREE.MeshBasicMaterial({ color: "#ffffff", side: THREE.FrontSide }));
  const leafBack = new THREE.Mesh(leafGeo, new THREE.MeshBasicMaterial({ color: "#ffffff", side: THREE.BackSide }));
  const leaf = new THREE.Group();
  leaf.add(leafFront, leafBack);
  leaf.position.set(0.004, 0, BLOCK + 0.004);
  leaf.visible = false;
  lift.add(leaf);
  root.visible = false;
  scene.add(root);

  let data = null, at = 0, single = false, aspect = 1, viewH = 1;
  let state = null; // { kind, t, ... } the animation under way
  let shut = Math.PI; // the hinge angle
  let from = null; // where the book came from, in this camera's space
  let hero = null;
  const images = new Map();
  const pages = new Map(); // "i:L" -> { canvas, tex }
  const look = (p) => ({ pos: p.pos.clone(), rot: p.rot.clone(), scale: p.scale, dim: p.dim, shut: p.shut });

  // ---- Pages ----
  const cssPageH = () => (PH / (2 * readDist() * Math.tan(THREE.MathUtils.degToRad(FOV / 2)))) * viewH;
  const texSize = () => {
    const h = THREE.MathUtils.clamp(Math.round(cssPageH() * Math.min(devicePixelRatio, 2) * 1.1), 640, 1400);
    return [Math.round(h * (PW / PH)), h];
  };
  function tex(i, side) {
    const k = `${i}:${side}`;
    let p = pages.get(k);
    if (p) return p.tex;
    const [w, h] = texSize();
    const canvas = document.createElement("canvas");
    canvas.width = w;
    canvas.height = h;
    const t = new THREE.CanvasTexture(canvas);
    t.colorSpace = THREE.SRGBColorSpace;
    t.anisotropy = 4;
    const paint = () => {
      const spread = data?.pages[i] ?? null;
      paintPage(canvas, { side, spread, single, num: spread ? 2 * i + (side === "R" ? 2 : 1) : 0, family: data?.family, ts: textScale(), cssH: cssPageH(), images, ready: () => {
        if (pages.get(k) !== p) return;
        paint();
        if (back?.source === t.source) back.needsUpdate = true; // the leaf's mirrored copy
      } });
      t.needsUpdate = true;
    };
    p = { tex: t };
    pages.set(k, p);
    paint();
    // Only what can be on screen is kept: this spread and the one a turn reveals.
    for (const [key, q] of pages) {
      const j = +key.split(":")[0];
      if (Math.abs(j - i) > 1 && Math.abs(j - at) > 1) q.tex.dispose(), pages.delete(key);
    }
    return t;
  }
  let back = null; // the leaf's mirrored copy of a page, replaced at each turn
  const flipped = (t) => {
    back?.dispose();
    const b = (back = t.clone());
    b.repeat.x = -1;
    b.offset.x = 1;
    b.needsUpdate = true;
    return b;
  };
  const show = (mat, t) => {
    if (mat.map !== t) (mat.map = t), (mat.needsUpdate = true);
  };
  function settle() {
    show(faceL.material, tex(at, "L"));
    show(faceR.material, tex(at, "R"));
    leaf.visible = false;
    // The next spread is painted ahead, so its picture has loaded by the time it turns over.
    if (data && at + 1 < data.pages.length) tex(at + 1, "L"), tex(at + 1, "R");
  }
  function repaint() {
    for (const q of pages.values()) q.tex.dispose();
    pages.clear();
    if (data) settle();
  }

  // ---- Poses (this camera's space) ----
  const readDist = () => {
    const t = 2 * Math.tan(THREE.MathUtils.degToRad(FOV / 2));
    // A wide view fits the open spread (with room for the controls above and below);
    // a tall one fits the right page.
    return single ? Math.max((CW * 1.06) / (t * aspect), CH / (t * 0.74)) : Math.max((2 * CW) / (t * aspect * 0.9), CH / (t * 0.76));
  };
  const reading = () => ({ pos: new THREE.Vector3(single ? -PW / 2 : 0, single ? 0.02 : 0, -readDist()), rot: new THREE.Euler(-0.07, 0, 0), scale: 1, dim: 1, shut: 0 });
  // Shut, the book is the right half: a wide view centres it as it opens.
  const place = (p) => {
    root.position.copy(p.pos);
    root.rotation.copy(p.rot);
    root.scale.setScalar(p.scale);
    dim.material.opacity = 0.62 * p.dim;
    shut = p.shut;
    const s = shut / Math.PI;
    lift.position.x = single ? 0 : -(PW / 2) * s;
    left.rotation.y = shut;
    left.position.z = 2 * BLOCK * s;
    spine.scale.z = BLOCK + (BLOCK + SKIN) * s + SKIN;
    spine.position.set(-SKIN / 2, 0, (BLOCK + (BLOCK + SKIN) * s - SKIN) / 2);
    root.visible = true;
  };
  const current = () => ({ pos: root.position.clone(), rot: root.rotation.clone(), scale: root.scale.x, dim: dim.material.opacity / 0.62, shut });
  const mix = (a, b, k) => ({
    pos: a.pos.clone().lerp(b.pos, k),
    rot: new THREE.Euler(lerp(a.rot.x, b.rot.x, k), lerp(a.rot.y, b.rot.y, k), lerp(a.rot.z, b.rot.z, k)),
    scale: lerp(a.scale, b.scale, k),
    dim: lerp(a.dim, b.dim, k),
    shut: lerp(a.shut, b.shut, k),
  });
  // On the shelf: small, far, spine out; from the Journal: below the view.
  const origin = () => {
    const d = 7, t = Math.tan(THREE.MathUtils.degToRad(FOV / 2)) * d;
    const pos = from ? new THREE.Vector3(from.x * t * aspect, from.y * t, -d) : new THREE.Vector3(0, -t * 1.5, -readDist() * 1.2);
    return { pos, rot: new THREE.Euler(0, from ? Math.PI * 0.38 : 0, 0), scale: from ? 0.16 : 1, dim: 0, shut: Math.PI };
  };

  // ---- The leaf ----
  function bend(phi, dir) {
    const p = leafGeo.attributes.position, du = (PW * 0.98) / COLS, lag = 0.85 * Math.sin(phi);
    const xs = [0], zs = [0];
    for (let c = 1; c <= COLS; c++) {
      const a = phi - dir * lag * ((c - 0.5) / COLS);
      xs.push(xs[c - 1] + Math.cos(a) * du);
      zs.push(zs[c - 1] + Math.sin(a) * du);
    }
    for (let i = 0; i < p.count; i++) {
      const c = Math.round(rest[i * 3] / du);
      p.setXYZ(i, xs[c], rest[i * 3 + 1], zs[c] + 0.002 * Math.sin(phi));
    }
    p.needsUpdate = true;
    leafGeo.computeBoundingSphere();
    const shade = 1 - 0.22 * Math.sin(phi);
    leafFront.material.color.setScalar(shade);
    leafBack.material.color.setScalar(shade);
  }

  // ---- The hero's arms (no clip: offsets over whatever the rig's clip poses) ----
  // Two poses, blended by weight: `reach` (the right arm up to the shelf, elbow soft) and
  // `hold` (both arms forward, elbows bent: the book at the chest). Each bone is set from a
  // base every frame, never stacked: three's mixer only writes a bone when its clip's value
  // changes, so a bone still holding last frame's result keeps last frame's base. Angles are
  // about the bone's own X (negative: forward and up) and Z (toward the body's middle for the
  // right arm when positive, the left when negative).
  const POSE = {
    reach: { ArmR: [-2.05, -0.12], ForeR: [-0.45, 0], ArmL: [0, 0], ForeL: [0, 0] },
    hold: { ArmR: [-0.85, 0.32], ForeR: [-1.15, 0], ArmL: [-0.85, -0.32], ForeL: [-1.15, 0] },
  };
  const want = { reach: 0, hold: 0 }, have = { reach: 0, hold: 0 };
  const qx = new THREE.Quaternion(), qz = new THREE.Quaternion(), X = new THREE.Vector3(1, 0, 0), Z = new THREE.Vector3(0, 0, 1);
  let bones = [];
  const aim = (reach, hold) => ((want.reach = reduced() ? 0 : reach), (want.hold = reduced() ? 0 : hold));
  function pose(h, dt) {
    if (h !== hero) {
      hero = h;
      const find = (n) => h?.obj.getObjectByName(n) ?? h?.obj.getObjectByName(`${n.slice(0, -1)}.${n.slice(-1)}`);
      bones = Object.keys(POSE.reach).map((name) => ({ name, bone: find(name), base: new THREE.Quaternion(), set: null })).filter((b) => b.bone);
    }
    const k = 1 - Math.exp(-dt * 8); // eased toward the wanted weights, never snapped
    for (const w of ["reach", "hold"]) have[w] += (want[w] - have[w]) * k;
    const rest = !want.reach && !want.hold && have.reach < 0.002 && have.hold < 0.002;
    for (const b of bones) {
      const q = b.bone.quaternion;
      if (!b.set || !q.equals(b.set)) b.base.copy(q);
      if (rest) {
        if (b.set) q.copy(b.base), (b.set = null); // released: the clip has the bone again
        continue;
      }
      const r = POSE.reach[b.name], o = POSE.hold[b.name];
      const x = r[0] * have.reach + o[0] * have.hold, z = r[1] * have.reach + o[1] * have.hold;
      q.copy(b.base).multiply(qx.setFromAxisAngle(X, x)).multiply(qz.setFromAxisAngle(Z, z));
      (b.set ??= new THREE.Quaternion()).copy(q);
    }
    if (rest) have.reach = have.hold = 0;
  }

  // ---- Animation ----
  const run = (kind, dur, extra = {}) => (state = { kind, t: 0, dur, ...extra });
  function update(dt) {
    if (!state) return;
    state.t += dt / state.dur / (globalThis.sixfoldSlow ?? 1); // `sixfoldSlow = 8` in the console: slow motion, for checking frames
    const k = clamp01(state.t), e = ease(k);
    switch (state.kind) {
      case "take": // the arm goes up, the book slides out toward the hero
        place(mix(state.a, { ...state.a, pos: state.a.pos.clone().add(new THREE.Vector3(0, 0.15, 1.2)) }, e));
        if (k >= 1) aim(0, 1), run("fly", DUR.fly, { a: current() }); // brought to the chest as it flies
        break;
      case "fly": {
        const b = { ...reading(), shut: Math.PI }, p = mix(state.a, b, e);
        p.pos.y += Math.sin(Math.PI * k) * 0.35; // a little arc
        p.rot.z = Math.sin(Math.PI * k) * 0.12;
        place(p);
        if (k >= 1) sound("flip"), run("open", DUR.open, { a: current() });
        break;
      }
      case "open":
        place(mix(state.a, reading(), e));
        if (k >= 1) state = null;
        break;
      case "turn": {
        const phi = state.dir > 0 ? Math.PI * e : Math.PI * (1 - e);
        bend(phi, state.dir);
        if (k >= 1) (state = null), settle();
        break;
      }
      case "close":
        place(mix(state.a, { ...state.a, shut: Math.PI }, e));
        if (k >= 1) aim(from ? 1 : 0, 0), run("back", DUR.back, { a: current() }); // up to the shelf again
        break;
      case "back":
        place(mix(state.a, origin(), e));
        if (k >= 1) finish();
        break;
    }
  }
  function finish() {
    state = null;
    aim(0, 0); // and the arms come down
    root.visible = false;
    dim.material.opacity = 0;
    data = null;
    for (const p of pages.values()) p.tex.dispose();
    pages.clear();
    const done = closed;
    closed = null;
    done?.();
  }
  let closed = null;

  return {
    // `book`: {id, family, title, pages: [{heading, text, art}]}; `ndc`: the shelf on screen, or null.
    open(book, ndc) {
      if (data) finish();
      data = book;
      at = 0;
      from = ndc;
      paintCover(coverCanvas, book.title, book.family);
      coverTex.needsUpdate = true;
      leather.color.set((COVER[book.family] ?? COVER.world).leather);
      settle();
      if (reduced()) return place(reading());
      sound("draw");
      aim(from ? 1 : 0, from ? 0 : 1);
      place(origin());
      run("take", from ? DUR.take : 0.001, { a: origin() });
    },
    page(n) {
      if (!data || n === at || n < 0 || n >= data.pages.length) return;
      if (state?.kind === "turn") (state = null), settle();
      const dir = Math.sign(n - at), was = at;
      if (reduced() || state || Math.abs(n - was) > 1) return void ((at = n), settle());
      // The leaf carries the page being turned over; under it, the page it uncovers.
      if (dir > 0) {
        show(leafFront.material, tex(was, "R"));
        show(leafBack.material, flipped(tex(n, "L")));
        at = n;
        show(faceR.material, tex(n, "R"));
      } else {
        show(leafFront.material, tex(n, "R"));
        show(leafBack.material, flipped(tex(was, "L")));
        at = n;
        show(faceL.material, tex(n, "L"));
      }
      bend(dir > 0 ? 0 : Math.PI, dir);
      leaf.visible = true;
      sound("flip");
      run("turn", DUR.turn, { dir });
    },
    // Shut it and put it back; `done` once it is out of sight.
    close(done) {
      if (!data) return void done?.();
      closed = done;
      leaf.visible = false;
      if (reduced()) return finish();
      sound("place");
      run("close", DUR.close, { a: current() });
    },
    live: () => !!data,
    busy: () => !!data && !!state && state.kind !== "turn",
    pose,
    resize(w, h) {
      const tall = w < h;
      aspect = w / h;
      viewH = h;
      cam.aspect = aspect;
      cam.updateProjectionMatrix();
      const t = 2 * Math.tan(THREE.MathUtils.degToRad(FOV / 2));
      dim.scale.set(t * aspect * 1.1 * DIM_AT, t * 1.1 * DIM_AT, 1);
      if (tall !== single) (single = tall), repaint();
      if (data && !state) place(reading());
    },
    render(renderer, dt) {
      update(dt);
      if (!root.visible) return;
      renderer.clearDepth();
      renderer.render(scene, cam);
    },
    dispose() {
      for (const p of pages.values()) p.tex.dispose();
      scene.traverse((o) => {
        o.geometry?.dispose();
        for (const m of [o.material].flat()) m?.map?.dispose(), m?.dispose();
      });
    },
  };
}
