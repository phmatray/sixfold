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
export const blobMat = () => new THREE.MeshBasicMaterial({ map: blobTex, transparent: true, depthWrite: false, polygonOffset: true, polygonOffsetFactor: -2, polygonOffsetUnits: -2 });
export const blobGeo = new THREE.PlaneGeometry(1, 1).rotateX(-Math.PI / 2);

export function actor(src, tint, scale, idle) {
  const obj = src ? cloneSkinned(src.scene) : PLACEHOLDER.hero();
  prep(obj, true);
  if (tint) {
    const c = new THREE.Color(tint);
    obj.traverse((o) => o.isMesh && ((o.material = o.material.clone()), o.material.color.lerp(c, 0.45)));
  }
  obj.scale.multiplyScalar(scale);
  const blob = new THREE.Mesh(blobGeo, blobMat());
  blob.userData.shared = true;
  blob.scale.setScalar(0.8 * scale);
  const a = { obj, blob, base: obj.scale.x, mixer: null, actions: {}, clip: null, idle: 0, t: 0, turnedAt: -1, idleName: (idle ?? "idle").toLowerCase() };
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
