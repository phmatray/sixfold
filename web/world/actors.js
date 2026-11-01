// Actors on the grid: the hero and the NPCs share the hero model (an NPC's own
// model when it has one). `actor()` makes one, `place()` puts it on its tile (or
// between two while stepping), `animateActor()` plays its clips and bounce, and
// `route()` finds the hero's way to a tapped tile. Also the grid helpers and the
// soft textures (contact shadow, glow) the scene shares.
import * as THREE from "three";
import { clone as cloneSkinned } from "three/addons/utils/SkeletonUtils.js";
import { PLACEHOLDER, prep, ACROSS } from "./props.js";

export const DIR = { n: [0, -1], s: [0, 1], e: [1, 0], w: [-1, 0] }; // n = away from the camera
export const BACK = { n: "s", s: "n", e: "w", w: "e" };
export const angle = (f) => Math.atan2(DIR[f][0], DIR[f][1]);
export const key = (x, z) => `${x},${z}`;
const TURN_SPEED = Math.PI / 2 / 0.12; // a quarter turn in 0.12 s
// Pose clips played once, held on their last frame (`a.pose`: sitting in seat.js, a book off a shelf).
const ONCE = new Set(["sitdown", "standup", "reach", "putback"]);
export const FIDGET_AFTER = 6; // seconds idle before the hero fidgets and the camera leans in

function radial(inner, outer) {
  const c = document.createElement("canvas");
  c.width = c.height = 64;
  const g = c.getContext("2d");
  const grd = g.createRadialGradient(32, 32, 0, 32, 32, 32);
  grd.addColorStop(0, inner);
  grd.addColorStop(1, outer);
  g.fillStyle = grd;
  g.fillRect(0, 0, 64, 64);
  return new THREE.CanvasTexture(c);
}
const blobTex = radial("rgba(20,30,15,0.42)", "rgba(20,30,15,0)");
export const glowTex = radial("rgba(255,255,255,0.9)", "rgba(255,255,255,0)");
// Soft contact shadow, 1.5 cm above the floor and depth-offset (never coplanar).
export const blobMat = () => new THREE.MeshBasicMaterial({ map: blobTex, transparent: true, depthWrite: false, polygonOffset: true, polygonOffsetFactor: -2, polygonOffsetUnits: -2 });
export const blobGeo = new THREE.PlaneGeometry(1, 1).rotateX(-Math.PI / 2);

// `tint` lerps the stand-in hero toward a colour. On an NPC's own model, `outfit`
// recolours the clothes (not skin, eyes, hair, trims or glowing parts; accents take
// its opposite hue) and `hair`
// the hair, so two people on one model still look like two people; `wear` shows
// accessories (`acc_<name>` meshes, hidden otherwise).
// People are one mesh on one palette texture (build_characters.py): the swatch
// table rides in the material's extras, and a recolour repaints the swatches into
// a texture shared by everyone dressed alike. Models without one (the sheep)
// recolour their materials by name.
const BODY = /skin|eye|white|blush|sole|gold|cream|hair|trim|glow/i;
const repainted = new Map();
function palette(json, ops) {
  const id = `${json}|${ops.map((o) => o.id).join("|")}`;
  if (repainted.has(id)) return repainted.get(id);
  const { grid, cell, keys } = JSON.parse(json);
  const cv = document.createElement("canvas");
  cv.width = cv.height = grid * cell;
  const g = cv.getContext("2d"), c = new THREE.Color();
  for (const [name, [i, hex]] of Object.entries(keys)) {
    c.set(hex);
    for (const op of ops) if (op.test(name)) op.apply(c);
    g.fillStyle = `#${c.getHexString()}`;
    g.fillRect((i % grid) * cell, (grid - 1 - Math.floor(i / grid)) * cell, cell, cell); // row 0 at the bottom, as in Blender
  }
  const t = new THREE.CanvasTexture(cv);
  Object.assign(t, { colorSpace: THREE.SRGBColorSpace, flipY: false, magFilter: THREE.NearestFilter, minFilter: THREE.NearestFilter, generateMipmaps: false });
  repainted.set(id, t);
  return t;
}
export function actor(src, tint, scale, idle, outfit = null, hair = null, wear = []) {
  const obj = src ? cloneSkinned(src.scene) : PLACEHOLDER.hero();
  prep(obj, true);
  // `hue`: take the colour's hue and keep each part's own lightness (garments stay distinct).
  const op = (test, color, k, hue) => {
    const want = new THREE.Color(color), w = want.getHSL({}), had = {};
    return { test, id: `${k}${hue}${color}`, apply: (c) => (hue ? (c.getHSL(had), c.setHSL(w.h, (had.s + w.s) / 2, had.l)) : c.lerp(want, k)) };
  };
  // An `_accent` swatch takes the outfit's opposite hue, so a recoloured person keeps two colours.
  const accent = outfit && `#${new THREE.Color(outfit).offsetHSL(0.5, 0, 0).getHexString()}`;
  const ops = [tint && op(() => true, tint, 0.45), outfit && op((n) => !BODY.test(n) && !/accent/i.test(n), outfit, 0, true),
    accent && op((n) => /accent/i.test(n), accent, 0, true), hair && op((n) => /hair/i.test(n), hair, 0.85)].filter(Boolean);
  obj.traverse((o) => {
    if (!o.isMesh) return;
    if (/^acc_/.test(o.name)) o.visible = wear.includes(o.name.slice(4));
    const pal = o.material.userData?.sx_palette;
    if (pal && o.material.map) {
      const map = o.material.map;
      if (ops.length) o.material.map = palette(pal, ops);
      else if (map.minFilter !== THREE.NearestFilter) Object.assign(map, { magFilter: THREE.NearestFilter, minFilter: THREE.NearestFilter, generateMipmaps: false, needsUpdate: true });
      return;
    }
    const mine = ops.filter((p) => p.test(o.material.name));
    if (!mine.length) return;
    o.material = o.material.clone();
    for (const p of mine) p.apply(o.material.color);
  });
  obj.scale.multiplyScalar(scale);
  const blob = new THREE.Mesh(blobGeo, blobMat());
  blob.userData.shared = true;
  blob.scale.setScalar(0.8 * scale);
  const head = new THREE.Box3().setFromObject(obj).max.y;
  // `lap`: how high a seat is under a person sitting on it, from their hips (seat.js; the Sit clip's thighs rest there).
  obj.updateMatrixWorld(true);
  const hips = obj.getObjectByName("Hips")?.getWorldPosition(new THREE.Vector3()).y ?? 0.54 * scale;
  const a = { obj, blob, head: Number.isFinite(head) ? head : 1.45, base: obj.scale.x, mixer: null, actions: {}, clip: null, idle: 0, t: 0, turnedAt: -1, idleName: (idle ?? "idle").toLowerCase(),
    lap: hips * 0.87, sit: null, pose: null };
  if (src?.animations.length) {
    a.mixer = new THREE.AnimationMixer(obj);
    // The first clip of each name wins: exports can carry duplicates.
    for (const clip of src.animations) a.actions[clip.name.toLowerCase()] ??= a.mixer.clipAction(clip);
    a.actions[a.idleName]?.play();
    a.clip = a.idleName;
  }
  return a;
}

// Put an actor on its tile, or between two while stepping, at the floor height
// (`floorAt`: decks, stairs, cliff tops); turn toward its facing.
// A step into a door (`step.stoop`, props.js STOOP) goes only `in` metres past the house's front edge and up
// its stoop instead: the leading foot (LEAD ahead) finds each tread.
const LEAD = 0.1;
function stoopAt(steps, d) {
  if (d <= -LEAD) return 0;
  if (d < 0) return (steps[0] * (d + LEAD)) / LEAD;
  const i = Math.min(d / 0.25, steps.length - 1), j = Math.floor(i);
  return steps[j] + ((steps[j + 1] ?? steps[j]) - steps[j]) * (i - j);
}
export function place(a, dt, floorAt) {
  const s = a.step, stoop = s?.stoop;
  const t = s ? s.t * (stoop ? 0.5 + stoop.in : 1) : 0;
  let px = a.x + 0.5 + (s ? (s.x - a.x) * t : 0);
  let pz = a.z + 0.5 + (s ? (s.z - a.z) * t : 0);
  const y0 = floorAt(a.x, a.z);
  a.fy = !s ? y0 : stoop ? y0 + stoopAt(stoop.steps, t - 0.5 + LEAD) : y0 + (floorAt(s.x, s.z) - y0) * t;
  const sit = a.sit; // sitting (seat.js): eased from its tile onto the seat
  if (sit) (px += (sit.ax - px) * sit.s), (pz += (sit.az - pz) * sit.s), (a.fy += (sit.y - a.fy) * sit.s);
  a.obj.position.set(px, a.obj.position.y, pz);
  a.blob.position.set(px, (sit ? y0 : a.fy) + 0.015, pz);
  const want = angle(sit?.face ?? a.face), cur = a.obj.rotation.y;
  const turn = Math.atan2(Math.sin(want - cur), Math.cos(want - cur));
  a.obj.rotation.y = cur + Math.sign(turn) * Math.min(Math.abs(turn), TURN_SPEED * dt);
  return [px, pz];
}

export function animateActor(a, dt, reduced) {
  const walking = !!a.step && a.step.t < 1; // a door's last pose is held while the fade comes up
  a.t += dt;
  if (a.mixer) {
    // `pose`: a clip held over the rest (seat.js: sitdown, sit, standup; ONCE ones play once).
    const want = a.pose && a.actions[a.pose] ? a.pose : walking ? (a.running && a.actions.run ? "run" : "walk") : a.idleName;
    if (want !== a.clip && a.actions[want]) {
      const once = ONCE.has(want), fade = reduced && (a.sit || a.clip === "sit") ? 0 : 0.15; // reduced motion: sit and stand at once
      if (fade) a.actions[a.clip]?.fadeOut(fade);
      else a.actions[a.clip]?.stop();
      const next = a.actions[want].reset().setLoop(once ? THREE.LoopOnce : THREE.LoopRepeat, Infinity);
      next.clampWhenFinished = once;
      if (once && a.sit && (want === "sitdown" || want === "standup")) next.time = (want === "sitdown" ? a.sit.k : 1 - a.sit.k) * next.getClip().duration; // turned back half way: from where it is
      (fade ? next.fadeIn(fade) : next).play();
      a.clip = want;
    }
    if (a.actions.walk) a.actions.walk.timeScale = a.running && !a.actions.run ? 2 : 1; // no Run clip: a faster Walk
    // A fidget clip once the hero has stood still a while, if the model has one.
    const fidget = a.actions.fidget ?? a.actions.look;
    if (fidget && a.idle > FIDGET_AFTER && !a.fidgeted && !a.pose) {
      a.fidgeted = true;
      fidget.reset().setLoop(THREE.LoopOnce, 1).fadeIn(0.2).play();
      a.mixer.addEventListener("finished", () => fidget.fadeOut(0.3));
    }
    if (a.idle === 0) a.fidgeted = false;
    a.mixer.update(dt);
  } else if (a.obj.userData.legs) {
    const swing = walking ? Math.sin((a.t * Math.PI) / a.step.dur) * 0.6 : 0; // one leg forward per tile
    const [l, r] = a.obj.userData.legs;
    l.rotation.x = swing;
    r.rotation.x = -swing;
  }
  // A small bounce per tile, squashed at each footfall; off a drop (#117) a short hop (a plain step under reduced motion).
  const up = walking && !reduced ? Math.sin(Math.PI * a.step.t) : 0;
  const squash = walking && !reduced ? 1 - up : 0;
  a.obj.position.y = a.fy + up * (a.step?.hop ? 0.45 : 0.05);
  if (a.bubble?.visible) bubble(a, dt, reduced);
  a.obj.scale.set(a.base * (1 + squash * 0.04), a.base * (1 - squash * 0.06 + up * 0.02), a.base * (1 + squash * 0.04));
}

// Shortest 4-way path from the hero to (tx, tz). A blocked target (a sign, a
// person, the door) routes to a free neighbour, or across a counter to a person
// behind it, and ends facing it.
export function route(w, hero, tx, tz) {
  const { width: W, height: H } = w.map;
  if (tx === hero.x && tz === hero.z) return [];
  const solid = (x, z) => w.blocked(x, z) || !!w.npcAt(x, z);
  const seat = w.seats?.get(key(tx, tz)); // a seat: from in front or beside, not from behind (seat.js)
  const goal = !solid(tx, tz) ? [[tx, tz, null]] : Object.entries(DIR).flatMap(([f, [dx, dz]]) => {
    const [x, z] = [tx - dx, tz - dz];
    if (f === seat?.face) return [];
    if (!solid(x, z)) return w.climb(x, z, tx, tz) ? [[x, z, f]] : [];
    const far = [tx - 2 * dx, tz - 2 * dz, f];
    return w.npcAt(tx, tz) && ACROSS.has(w.propAt.get(key(x, z))?.asset) && !solid(far[0], far[1]) ? [far] : [];
  });
  if (!goal.length) return [];
  const prev = new Map([[key(hero.x, hero.z), null]]);
  const queue = [[hero.x, hero.z]];
  while (queue.length) {
    const [x, z] = queue.shift();
    const g = goal.find(([gx, gz]) => gx === x && gz === z);
    if (g) {
      const dirs = [];
      for (let k = key(x, z); prev.get(k); k = prev.get(k).from) dirs.unshift(prev.get(k).f);
      if (g[2]) dirs.push(g[2]); // turn to face it
      return dirs;
    }
    for (const [f, [dx, dz]] of Object.entries(DIR)) {
      const nx = x + dx, nz = z + dz, k = key(nx, nz);
      if (nx < 0 || nz < 0 || nx >= W || nz >= H || prev.has(k) || solid(nx, nz) || !w.climb(x, z, nx, nz)) continue;
      prev.set(k, { from: key(x, z), f });
      queue.push([nx, nz]);
    }
  }
  return [];
}

// ---------- Emotes: a bubble over a head ----------
// "!" when a route duelist spots the hero, and the dialogue cue `emote:<actor>:<kind>`
// (kinds as `EMOTES` in meta/src/overworld.rs). Drawn over everything, so scenery never hides it.
const INK = "#0d1a2c", GLYPH = { bang: "#e2553a", what: "#2d5fae", dots: INK, heart: "#e0457b", note: "#2f7175" };
const bubbles = new Map();
function bubbleTex(kind) {
  if (bubbles.has(kind)) return bubbles.get(kind);
  const c = document.createElement("canvas");
  c.width = c.height = 128;
  const g = c.getContext("2d");
  g.lineJoin = "round";
  g.lineWidth = 7;
  g.strokeStyle = INK;
  g.fillStyle = "#fffaf0";
  g.beginPath(); // a round bubble with a tail toward the head below
  g.roundRect(14, 10, 100, 86, 34);
  g.moveTo(54, 94);
  g.lineTo(64, 118);
  g.lineTo(76, 94);
  g.stroke();
  g.fill();
  g.fillRect(52, 86, 26, 10);
  g.fillStyle = g.strokeStyle = GLYPH[kind] ?? INK;
  g.beginPath();
  if (kind === "bang") g.roundRect(55, 22, 18, 46, 8), g.arc(64, 81, 9, 0, Math.PI * 2);
  else if (kind === "dots") for (const x of [38, 64, 90]) g.moveTo(x + 8, 54), g.arc(x, 54, 8, 0, Math.PI * 2);
  else if (kind === "heart") g.moveTo(64, 82), g.bezierCurveTo(20, 54, 32, 18, 64, 38), g.bezierCurveTo(96, 18, 108, 54, 64, 82);
  else if (kind === "note") {
    g.ellipse(52, 72, 13, 10, -0.4, 0, Math.PI * 2);
    g.rect(58, 26, 8, 46);
    g.moveTo(62, 26), g.quadraticCurveTo(84, 30, 86, 50), g.quadraticCurveTo(76, 40, 64, 40);
  } else {
    // "what": a question mark
    g.lineWidth = 12;
    g.lineCap = "round";
    g.arc(64, 40, 16, Math.PI * 1.1, Math.PI * 0.45);
    g.lineTo(64, 62);
    g.stroke();
    g.beginPath();
    g.arc(64, 81, 8, 0, Math.PI * 2);
  }
  g.fill();
  const t = new THREE.CanvasTexture(c);
  t.colorSpace = THREE.SRGBColorSpace;
  bubbles.set(kind, t);
  return t;
}
// A bark (npc.js): its words in a bubble of the same make, wrapped to a few short lines.
function sayTex(text) {
  const id = `say:${text}`;
  if (bubbles.has(id)) return bubbles.get(id);
  const c = document.createElement("canvas"), font = "600 30px system-ui, sans-serif", WRAP = 340, LINE = 38;
  let g = c.getContext("2d");
  g.font = font;
  const lines = [];
  for (const word of text.split(" ")) {
    const l = lines.length ? `${lines[lines.length - 1]} ${word}` : "";
    if (l && g.measureText(l).width <= WRAP) lines[lines.length - 1] = l;
    else lines.push(word);
  }
  c.width = Math.ceil(Math.max(...lines.map((l) => g.measureText(l).width)) + 60);
  c.height = lines.length * LINE + 64;
  g = c.getContext("2d"); // resized: the context starts over
  const w = c.width, h = c.height - 26, mid = w / 2;
  Object.assign(g, { lineJoin: "round", lineWidth: 7, strokeStyle: INK, fillStyle: "#fffaf0" });
  g.beginPath();
  g.roundRect(4, 4, w - 8, h - 4, 26);
  g.moveTo(mid - 10, h - 2);
  g.lineTo(mid, c.height - 4);
  g.lineTo(mid + 12, h - 2);
  g.stroke();
  g.fill();
  g.fillRect(mid - 12, h - 10, 26, 10);
  Object.assign(g, { font, fillStyle: INK, textAlign: "center", textBaseline: "middle" });
  lines.forEach((l, i) => g.fillText(l, mid, 4 + (h - 4) / 2 + (i - (lines.length - 1) / 2) * LINE));
  const t = new THREE.CanvasTexture(c);
  t.colorSpace = THREE.SRGBColorSpace;
  t.userData.size = [w / 128, c.height / 128];
  bubbles.set(id, t);
  return t;
}
const POP = 0.2, FADE_OUT = 0.25;
// Pop a bubble over `a` for `hold` seconds (then it fades).
export const emote = (a, kind, hold = 1.2) => show(a, bubbleTex(kind), hold);
// Say `text` in a bubble over `a` for `hold` seconds (a bark: no dialogue box).
export const say = (a, text, hold) => show(a, sayTex(text), hold);
function show(a, map, hold) {
  if (!a.bubble) {
    a.bubble = new THREE.Sprite(new THREE.SpriteMaterial({ transparent: true, depthTest: false, depthWrite: false, fog: false }));
    a.bubble.renderOrder = 20;
    a.bubble.userData.shared = true; // module-wide textures; the sprite geometry is three's own
  }
  if (a.bubble.parent !== a.obj.parent) a.obj.parent?.add(a.bubble);
  a.bubble.material.map = map;
  a.bubble.material.needsUpdate = true;
  Object.assign(a.bubble.userData, { t: 0, hold, size: map.userData.size ?? [1, 1] });
  a.bubble.visible = true;
  bubble(a, 0, false);
}
function bubble(a, dt, reduced) {
  const b = a.bubble, u = b.userData;
  u.t += dt;
  const t = u.t, pop = reduced || t > POP ? 1 : t < POP * 0.6 ? (t / (POP * 0.6)) * 1.25 : 1.25 - ((t - POP * 0.6) / (POP * 0.4)) * 0.25;
  b.material.opacity = reduced ? +(t < u.hold) : THREE.MathUtils.clamp(1 - (t - u.hold) / FADE_OUT, 0, 1); // reduced motion: in and out as cuts
  if (b.material.opacity === 0) b.visible = false;
  const [sw, sh] = u.size;
  b.scale.set(0.74 * pop * sw, 0.74 * pop * sh, 1);
  b.position.set(a.obj.position.x, a.obj.position.y + a.head + 0.05 + 0.37 * sh, a.obj.position.z);
}

// ---------- Occupations: a routine's `do` (npc.js), laid over the idle clip ----------
// A small motion on the Torso and Head bones, as [pitch, yaw] radians: `work` bends to a task, `sweep` sways at
// the waist, `read` looks down, `talk` nods. Called with no kind before the mixer poses the bones (it takes the
// last frame's motion off, so a bone no clip animates never drifts) and with the kind after. Off under reduced motion.
const OCCUPY = {
  work: (t) => ({ Torso: [0.16 + 0.08 * Math.sin(t * 3.2), 0], Head: [0.2, 0] }),
  sweep: (t) => ({ Torso: [0.2, 0.28 * Math.sin(t * 2.4)], Head: [0.15, 0] }),
  read: (t) => ({ Torso: [0.06, 0], Head: [0.38 + 0.03 * Math.sin(t * 0.7), 0] }),
  talk: (t) => ({ Head: [0.06 * Math.sin(t * 5.5), 0.12 * Math.sin(t * 1.3)] }),
};
const euler = new THREE.Euler();
export function occupy(a, kind, reduced, phase = 0) {
  const on = (a.occupied ??= []);
  for (const [b, q] of on) b.quaternion.multiply(q.invert());
  on.length = 0;
  const f = kind && !reduced && a.mixer && !a.step && !a.sit && OCCUPY[kind];
  if (!f) return;
  for (const [name, [pitch, yaw]] of Object.entries(f(a.t + phase * 10))) {
    const b = ((a.bones ??= {})[name] ??= a.obj.getObjectByName(name));
    if (!b) continue;
    const q = ((a.boneQ ??= {})[name] ??= new THREE.Quaternion()).setFromEuler(euler.set(pitch, yaw, 0));
    b.quaternion.multiply(q);
    on.push([b, q]);
  }
}
