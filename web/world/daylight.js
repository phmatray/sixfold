// Day and night (WORLD_DESIGN §7.2). The clock is the engine's (engine/src/clock.rs:
// dawn 05:00, day 08:00, dusk 18:00, night 21:00); world.js runs it, a game minute
// per real second, and this module turns a minute into light. By day a map wears its
// own `light` block; dawn, dusk and night are derived from it (a map may override any
// of them with `light_dawn` / `light_dusk` / `light_night`, the same shape as `light`).
// Phases cross-fade over 20 game minutes, 20 s (at once with reduced motion).
//
// Night is moonlight, not darkness: a cool moon from the camera's side and a blue
// sky fill keep paths, edges and people readable; fog takes the sky's colour; `glow`
// parts (lamps, kilns, crystals) and window `glass` light up warm. Only the hero's
// lantern and the LIVE nearest street lamps are real point lights (WORLD_DESIGN P1:
// six at most), a fixed set per map so no shader ever recompiles at dusk; every
// other lamp is emissive, with a halo and a pool of light on the ground.
// Indoors only the window light's colour follows the clock. A `dark` map (a cave)
// has no sun at any hour: the hero's lantern is the light.
import * as THREE from "three";
import { glowTex } from "./actors.js";

export const DAY = 1440;
const START = { dawn: 300, day: 480, dusk: 1080, night: 1260 };
const ORDER = ["dawn", "day", "dusk", "night"];
const FADE = 20; // game minutes
const LIVE = 3; // street lamps with a real light, besides the hero's lantern
export const phaseOf = (minute) => {
  const m = ((minute % DAY) + DAY) % DAY;
  return m < START.dawn || m >= START.night ? "night" : m < START.day ? "dawn" : m < START.dusk ? "day" : "dusk";
};

const C = (c) => new THREE.Color(c);
const mix = (a, b, t) => C(a).lerp(C(b), t);
const WARM = C("#ffc46e"); // a lit window
const DARK_PANE = C("#1f2c4c"); // a window seen from inside at night

// A `light` block (or part of one) over a preset.
function over(p, L) {
  if (!L) return p;
  const q = { ...p };
  if (L.sun?.color) q.sun = C(L.sun.color);
  if (L.sun?.intensity !== undefined) q.sunI = L.sun.intensity;
  if (L.sun?.fromHero) q.from = new THREE.Vector3(...L.sun.fromHero);
  if (L.hemisphere?.sky) q.sky = C(L.hemisphere.sky);
  if (L.hemisphere?.ground) q.ground = C(L.hemisphere.ground);
  if (L.hemisphere?.intensity !== undefined) q.hemiI = L.hemisphere.intensity;
  if (L.background) q.bg = C(L.background);
  if (L.fog) (q.near = L.fog.near), (q.far = L.fog.far);
  return q;
}

// The four looks of a map.
function presets(map) {
  const L = map.light;
  const day = over({ lamps: 0, glow: 0, glass: 0, night: 0, near: 60, far: 90 }, L);
  if (L.indoor) {
    // The window light only: warm at dawn and dusk, moonlit at night; panes go dark.
    const tint = (c, k, i, glass = 0) => ({ ...day, sun: mix(day.sun, c, k), sunI: day.sunI * i, glass });
    return { day, dawn: tint("#ffc8a0", 0.6, 0.85), dusk: tint("#ff9e6b", 0.6, 0.8, 0.5), night: tint("#7f95e0", 0.85, 0.55, 1) };
  }
  if (map.dark) {
    const cave = { ...day, sunI: 0, sky: C("#3b3a52"), ground: C("#0d0b12"), hemiI: 0.55, bg: C("#07060b"), near: 7, far: 20, lamps: 1, glow: 1, glass: 1, night: 0.6 };
    return { day: cave, dawn: cave, dusk: cave, night: cave };
  }
  // Dawn is cool and rosy (dusk is the warm one): a pink sun low in the east, lilac shadows.
  const dawn = { ...day, sun: C("#ffbfae"), sunI: day.sunI * 0.62, from: new THREE.Vector3(14, 8, 4),
    sky: mix(day.sky, "#d9c4ee", 0.65), ground: mix(day.ground, "#5a4a78", 0.5), hemiI: day.hemiI * 0.8,
    bg: mix(day.bg, "#e8c6d8", 0.72), near: day.near * 0.9, far: day.far * 0.95, lamps: 0.2, glow: 0.3, glass: 0.25, night: 0.15 };
  const dusk = { ...day, sun: C("#ff9e6b"), sunI: day.sunI * 0.6, from: new THREE.Vector3(-14, 7, 4),
    sky: mix(day.sky, "#ffae8c", 0.55), ground: mix(day.ground, "#4a2f4c", 0.45), hemiI: day.hemiI * 0.78,
    bg: mix(day.bg, "#e59a7a", 0.75), near: day.near * 0.85, far: day.far * 0.9, lamps: 0.6, glow: 0.75, glass: 0.7, night: 0.35 };
  // The moon shines from the camera's side, so faces turned to the player stay lit.
  const night = { ...day, sun: C("#a9bcff"), sunI: day.sunI * 0.45, from: new THREE.Vector3(-7, 18, 10),
    sky: C("#6f83c4"), ground: C("#232a47"), hemiI: day.hemiI * 0.74,
    bg: C("#1b2236"), near: 13, far: 33, lamps: 1, glow: 1, glass: 1, night: 1 };
  return { day, dawn: over(dawn, map.light_dawn), dusk: over(dusk, map.light_dusk), night: over(night, map.light_night) };
}

function lerp(out, a, b, t) {
  for (const k of ["sun", "sky", "ground", "bg"]) out[k].lerpColors(a[k], b[k], t);
  out.from.lerpVectors(a.from, b.from, t);
  for (const k of ["sunI", "hemiI", "near", "far", "lamps", "glow", "glass", "night"]) out[k] = a[k] + (b[k] - a[k]) * t;
}

// The light of a built map `w` (build.js), added to its root. `update(minute, hero)`
// each frame; it returns the current look for world.js (background, fog, sun direction).
export function daylight(w, reduced) {
  const P = presets(w.map);
  const indoor = !!w.map.light.indoor, dark = !!w.map.dark;
  const cur = { ...P.day, sun: C(0), sky: C(0), ground: C(0), bg: C(0), from: new THREE.Vector3() };
  const root = w.root;

  // The hero's lantern (outdoors and in caves), and a fixed few for the nearest lamps.
  const low = w.lamps.filter((p) => p.y < 3.2);
  const lantern = indoor ? null : new THREE.PointLight("#ffc98a", 0, dark ? 10 : 7, 1.5);
  const pool = indoor ? [] : low.slice(0, LIVE).map(() => new THREE.PointLight("#ffbf73", 0, 8.5, 1.4));
  for (const l of [lantern, ...pool]) if (l) (l.layers.enableAll(), root.add(l));

  // Every lamp glows on its own: a halo, and (low ones) light pooled on the ground.
  const halos = w.lamps.map((p) => {
    const s = new THREE.Sprite(new THREE.SpriteMaterial({ map: glowTex, color: "#ffd59a", transparent: true, opacity: 0, blending: THREE.AdditiveBlending, depthWrite: false }));
    s.position.copy(p);
    s.scale.setScalar(1.1);
    s.visible = false;
    root.add(s);
    return s;
  });
  let pools = null;
  if (low.length && !indoor) {
    const mat = new THREE.MeshBasicMaterial({ map: glowTex, color: "#ffb35c", transparent: true, opacity: 0, blending: THREE.AdditiveBlending, depthWrite: false,
      polygonOffset: true, polygonOffsetFactor: -3, polygonOffsetUnits: -3 });
    pools = new THREE.InstancedMesh(new THREE.PlaneGeometry(5, 5).rotateX(-Math.PI / 2), mat, low.length);
    low.forEach((p, i) => pools.setMatrixAt(i, new THREE.Matrix4().makeTranslation(p.x, w.floorAt(Math.floor(p.x), Math.floor(p.z)) + 0.03, p.z)));
    pools.visible = false;
    root.add(pools);
  }

  // Materials that light up: `glow` parts and window `glass` (models and instances alike).
  const glows = new Set(), glass = new Set(), crystals = new Set(w.waymarks.flatMap((m) => m.mats));
  root.traverse((o) => {
    if (!o.isMesh || !o.material?.name) return;
    const m = o.material;
    if (crystals.has(m)) return;
    if (/glow|lantern_glass/i.test(m.name)) glows.add(m);
    else if (/glass/i.test(m.name)) glass.add(m);
    m.userData.base ??= m.color.clone();
  });
  for (const m of crystals) m.userData.base ??= m.color.clone();

  let lastKey = "", pick = 0, near = [];
  const tmp = new THREE.Color();
  return {
    // The light at `minute` around the hero at (hx, hz); `dt` in real seconds.
    update(minute, hx, hz, dt) {
      const p = phaseOf(minute), prev = ORDER[(ORDER.indexOf(p) + 3) % 4];
      const since = (((minute - START[p]) % DAY) + DAY) % DAY;
      const k = reduced ? 1 : Math.min(1, since / FADE);
      lerp(cur, P[prev], P[p], k * k * (3 - 2 * k));

      w.sun.color.copy(cur.sun);
      w.sun.intensity = cur.sunI;
      w.hemi.color.copy(cur.sky);
      w.hemi.groundColor.copy(cur.ground);
      w.hemi.intensity = cur.hemiI;

      // Materials only when the look moved (a fade, a waymark lit) or something asked.
      const key = `${cur.glow.toFixed(3)}|${cur.glass.toFixed(3)}|${w.waymarks.map((m) => `${m.lit}${m.flash.toFixed(2)}`).join()}`;
      if (key !== lastKey || w.lightDirty) {
        lastKey = key;
        w.lightDirty = false;
        for (const m of glows) m.emissive?.copy(m.userData.base).multiplyScalar(0.55 + 0.8 * cur.glow);
        for (const m of glass) {
          if (indoor) m.color.lerpColors(m.userData.base, DARK_PANE, cur.glass * 0.85), m.emissive?.setScalar(0);
          else m.color.lerpColors(m.userData.base, WARM, cur.glass * 0.9), m.emissive?.copy(WARM).multiplyScalar(0.95 * cur.glass);
        }
        for (const wm of w.waymarks)
          for (const m of wm.mats) {
            const on = wm.lit ? 1 : 0;
            m.color.copy(m.userData.base).multiplyScalar(0.45 + 0.55 * on);
            m.emissive?.copy(m.userData.base).multiplyScalar(0.1 + on * (0.6 + 0.8 * cur.glow) + wm.flash * 2.2);
          }
      }

      // Halos and light pools by every lamp; real light on the nearest few.
      const on = cur.lamps > 0.01;
      for (const h of halos) (h.visible = on), (h.material.opacity = 0.85 * cur.glow);
      if (pools) (pools.visible = on), (pools.material.opacity = 0.42 * cur.lamps);
      if (lantern) {
        // Held high and a little forward, so it lights the ground round the hero, not its hair.
        lantern.intensity = dark ? 9 : 2.6 * cur.lamps;
        lantern.position.set(hx, w.floorAt(Math.floor(hx), Math.floor(hz)) + 2.6, hz + 0.7);
      }
      if (pool.length) {
        if ((pick -= dt) <= 0 || !on) {
          pick = 0.25;
          near = low.map((q) => [q, (q.x - hx) ** 2 + (q.z - hz) ** 2]).sort((a, b) => a[1] - b[1]);
        }
        // A lamp about to lose its light to a nearer one is already faded out: no pop.
        const edge = near[LIVE] ? Math.sqrt(near[LIVE][1]) : Infinity;
        pool.forEach((l, i) => {
          const n = near[i];
          if (!n) return void (l.intensity = 0);
          l.position.copy(n[0]);
          const fade = Math.min(1, (edge - Math.sqrt(n[1])) / 2.5);
          l.intensity = on ? 7 * cur.lamps * fade : 0;
        });
      }
      return { bg: cur.bg, near: cur.near, far: cur.far, from: cur.from, night: cur.night, phase: p, color: tmp.copy(cur.bg) };
    },
  };
}
