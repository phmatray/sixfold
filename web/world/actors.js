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
// Drawn after the hero (camera layer 3): people and anything too low to hide the hero
// (grass, rocks, fences, crates). They cover the hero by plain depth, so only tall
// static scenery (roofs, canopies, cliffs, walls) draws the hero's silhouette (world.js).
export const LATE = 3;
export const late = (o) => (o.traverse((c) => c.layers.set(LATE)), o);
export const blobMat = () => new THREE.MeshBasicMaterial({ map: blobTex, transparent: true, depthWrite: false, polygonOffset: true, polygonOffsetFactor: -2, polygonOffsetUnits: -2 });
export const blobGeo = new THREE.PlaneGeometry(1, 1).rotateX(-Math.PI / 2);

// `tint` lerps the stand-in hero toward a colour. On an NPC's own model, `outfit`
// recolours the clothes (not skin, eyes, hair or trims) and `hair` the hair, so
// two people on one model still look like two people.
const BODY = /skin|eye|white|blush|sole|gold|cream|hair/i;
export function actor(src, tint, scale, idle, outfit = null, hair = null) {
  const obj = src ? cloneSkinned(src.scene) : PLACEHOLDER.hero();
  prep(obj, true);
  // `hue`: take the colour's hue and keep each part's own lightness (garments stay distinct).
  const recolor = (test, color, k, hue) => {
    const c = new THREE.Color(color), want = c.getHSL({}), had = {};
    obj.traverse((o) => {
      if (!o.isMesh || !test(o.material.name)) return;
      o.material = o.material.clone();
      if (hue) o.material.color.getHSL(had), o.material.color.setHSL(want.h, (had.s + want.s) / 2, had.l);
      else o.material.color.lerp(c, k);
    });
  };
  if (tint) recolor(() => true, tint, 0.45);
  if (outfit) recolor((n) => !BODY.test(n), outfit, 0, true);
  if (hair) recolor((n) => /hair/i.test(n), hair, 0.85);
  obj.scale.multiplyScalar(scale);
  const blob = new THREE.Mesh(blobGeo, blobMat());
  blob.userData.shared = true;
  blob.scale.setScalar(0.8 * scale);
  const head = new THREE.Box3().setFromObject(obj).max.y;
  const a = { obj, blob, head: Number.isFinite(head) ? head : 1.45, base: obj.scale.x, mixer: null, actions: {}, clip: null, idle: 0, t: 0, turnedAt: -1, idleName: (idle ?? "idle").toLowerCase() };
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
export function place(a, dt, floorAt) {
  const t = a.step ? a.step.t : 0;
  const px = a.x + 0.5 + (a.step ? (a.step.x - a.x) * t : 0);
  const pz = a.z + 0.5 + (a.step ? (a.step.z - a.z) * t : 0);
  const y0 = floorAt(a.x, a.z);
  a.fy = a.step ? y0 + (floorAt(a.step.x, a.step.z) - y0) * t : y0;
  a.obj.position.set(px, a.obj.position.y, pz);
  a.blob.position.set(px, a.fy + 0.015, pz);
  const want = angle(a.face), cur = a.obj.rotation.y;
  const turn = Math.atan2(Math.sin(want - cur), Math.cos(want - cur));
  a.obj.rotation.y = cur + Math.sign(turn) * Math.min(Math.abs(turn), TURN_SPEED * dt);
  return [px, pz];
}

export function animateActor(a, dt, reduced) {
  const walking = !!a.step;
  a.t += dt;
  if (a.mixer) {
    const want = walking ? (a.running && a.actions.run ? "run" : "walk") : a.idleName;
    if (want !== a.clip && a.actions[want]) {
      a.actions[a.clip]?.fadeOut(0.15);
      a.actions[want].reset().fadeIn(0.15).play();
      a.clip = want;
    }
    if (a.actions.walk) a.actions.walk.timeScale = a.running && !a.actions.run ? 2 : 1; // no Run clip: a faster Walk
    // A fidget clip once the hero has stood still a while, if the model has one.
    const fidget = a.actions.fidget ?? a.actions.look;
    if (fidget && a.idle > FIDGET_AFTER && !a.fidgeted) {
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
  // A small bounce per tile, squashed at each footfall.
  const up = walking && !reduced ? Math.sin(Math.PI * a.step.t) : 0;
  const squash = walking && !reduced ? 1 - up : 0;
  a.obj.position.y = a.fy + up * 0.05;
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
  const goal = !solid(tx, tz) ? [[tx, tz, null]] : Object.entries(DIR).flatMap(([f, [dx, dz]]) => {
    const [x, z] = [tx - dx, tz - dz];
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
// (kinds as `EMOTES` in engine/src/overworld.rs). Drawn over everything, so scenery never hides it.
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
const POP = 0.2, FADE_OUT = 0.25;
// Pop a bubble over `a` for `hold` seconds (then it fades).
export function emote(a, kind, hold = 1.2) {
  if (!a.bubble) {
    a.bubble = new THREE.Sprite(new THREE.SpriteMaterial({ transparent: true, depthTest: false, depthWrite: false, fog: false }));
    a.bubble.renderOrder = 20;
    a.bubble.userData.shared = true; // module-wide textures; the sprite geometry is three's own
  }
  if (a.bubble.parent !== a.obj.parent) a.obj.parent?.add(a.bubble);
  a.bubble.material.map = bubbleTex(kind);
  a.bubble.material.needsUpdate = true;
  Object.assign(a.bubble.userData, { t: 0, hold });
  a.bubble.visible = true;
  bubble(a, 0, false);
}
function bubble(a, dt, reduced) {
  const b = a.bubble, u = b.userData;
  u.t += dt;
  const t = u.t, pop = reduced || t > POP ? 1 : t < POP * 0.6 ? (t / (POP * 0.6)) * 1.25 : 1.25 - ((t - POP * 0.6) / (POP * 0.4)) * 0.25;
  b.material.opacity = THREE.MathUtils.clamp(1 - (t - u.hold) / FADE_OUT, 0, 1);
  if (b.material.opacity === 0) b.visible = false;
  b.scale.setScalar(0.74 * pop);
  b.position.set(a.obj.position.x, a.obj.position.y + a.head + 0.42, a.obj.position.z);
}
