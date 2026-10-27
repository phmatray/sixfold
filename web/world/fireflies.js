// Fireflies drawn to finds: what used to be a glint over something to find (an
// interaction's `glint`, see map.js). A few warm motes drift in lazy, wavering
// orbits round the spot, each pulsing slowly on its own beat. By day they don't
// glow: two tiny dark specks, gnats a curious eye can still catch. A hidden find
// has none by day and a single firefly at night. Once a spot's condition stops
// holding (the find is taken) its motes scatter and are gone in about a second
// (with reduced motion they hang still, and just fade).
// One point cloud for the whole map: a single draw however many finds it holds.
// Fog thins them with distance, so at night they read close by, not across the map.
import * as THREE from "three";

const PER = 3; // motes round a find (hidden: 1, and only after dark)
const BY_DAY = 2; // of them, seen by day as dark specks
const SCATTER = 1; // seconds
const DAY = new THREE.Color("#2a2614"), NIGHT = new THREE.Color("#e4ff86");

// A firefly: a small solid body in a faint halo (by day, a speck). One for every map.
let tex = null;
// The mote texture, made once (a seal stone's burst draws with it too, world.js).
export const mote = () => (tex ??= moteTex());
function moteTex() {
  const c = document.createElement("canvas");
  c.width = c.height = 64;
  const g = c.getContext("2d"), grd = g.createRadialGradient(32, 32, 0, 32, 32, 32);
  [[0, 1], [0.14, 1], [0.24, 0.4], [0.55, 0.1], [1, 0]].forEach(([at, a]) => grd.addColorStop(at, `rgba(255,255,255,${a})`));
  g.fillStyle = grd;
  g.fillRect(0, 0, 64, 64);
  return new THREE.CanvasTexture(c);
}

// The spots of a map's `interactions` (each with a `glint` condition, or a hidden
// find), added to `root`. `floorAt(x, z)` is the floor height of a tile.
export function fireflies(interactions, floorAt, root) {
  const spots = interactions
    .map((it) => {
      const hidden = it.kind === "find" && it.hidden;
      const cond = it.glint ?? (hidden ? `!flag:found_${it.id}` : null);
      if (!cond) return null;
      const at = new THREE.Vector3(it.x + 0.5, floorAt(it.x, it.z), it.z + 0.5);
      const motes = Array.from({ length: hidden ? 1 : PER }, (_, i) => ({
        i, r: 0.35 + Math.random() * 0.3, h: 0.6 + Math.random() * 0.6, w: (0.35 + Math.random() * 0.3) * (Math.random() < 0.5 ? -1 : 1),
        ph: Math.random() * 6.28, beat: 0.5 + Math.random() * 0.45, pos: new THREE.Vector3(), out: new THREE.Vector3(),
      }));
      return { it, cond, hidden, at, motes, on: undefined, gone: null };
    })
    .filter(Boolean);
  const all = spots.flatMap((s) => s.motes.map((m) => [s, m]));
  const geo = new THREE.BufferGeometry();
  geo.setAttribute("position", new THREE.Float32BufferAttribute(new Float32Array(all.length * 3), 3));
  geo.setAttribute("color", new THREE.Float32BufferAttribute(new Float32Array(all.length * 4).fill(1), 4)); // alpha per mote
  const mat = new THREE.PointsMaterial({ map: mote(), size: 0.05, vertexColors: true, transparent: true, depthWrite: false });
  const pts = new THREE.Points(geo, mat);
  pts.frustumCulled = false;
  pts.visible = all.length > 0;
  root.add(pts);

  return {
    // Re-check each spot (`holds(cond)`, refresh in build.js); one that just went off scatters.
    show(holds) {
      for (const s of spots) {
        const on = holds(s.it.if) && holds(s.cond);
        if (s.on && !on) s.gone = 0;
        if (on) s.gone = null;
        else if (s.on === undefined) s.gone = SCATTER; // off from the start: nothing to see
        s.on = on;
      }
    },
    // Each frame: `t` the map's clock, `night` 0..1 (daylight.js).
    update(dt, t, night, reduced) {
      mat.color.lerpColors(DAY, NIGHT, night);
      mat.size = 0.22 + 0.28 * night;
      mat.blending = night > 0.5 ? THREE.AdditiveBlending : THREE.NormalBlending; // they glow after dark; by day, dark specks
      const p = geo.attributes.position, c = geo.attributes.color;
      for (const s of spots) if (s.gone !== null) s.gone = Math.min(SCATTER, s.gone + dt);
      all.forEach(([s, m], k) => {
        const g = s.gone === null ? 0 : s.gone / SCATTER;
        if (g >= 1) return void c.setW(k, 0);
        const day = !s.hidden && m.i < BY_DAY ? 0.85 : 0;
        // A slow swell and fade, never a blink (steady with reduced motion).
        const pulse = reduced ? 0.8 : 0.3 + 0.7 * Math.sin(t * m.beat + m.ph) ** 2;
        c.setW(k, (day + (pulse - day) * night) * (1 - g * g)); // bright while it flies, gone at the end
        if (g > 0 && !reduced) {
          // Off and away: a dart outward and up, faster and faster.
          const d = 1.6 * g + 1.4 * g * g;
          p.setXYZ(k, m.pos.x + m.out.x * d, m.pos.y + 0.8 * g + 0.8 * g * g, m.pos.z + m.out.z * d);
          return;
        }
        // A lazy orbit that wanders: speed, radius and height all waver.
        const T = reduced ? 0 : t;
        const a = T * m.w + m.ph + 0.7 * Math.sin(T * 1.3 + m.ph * 3);
        const r = m.r + 0.12 * Math.sin(T * 0.9 + m.ph * 2);
        m.out.set(Math.cos(a), 0, Math.sin(a));
        m.pos.set(s.at.x + m.out.x * r, s.at.y + m.h + 0.18 * Math.sin(T * 0.7 + m.ph) + 0.05 * Math.sin(T * 2.9 + m.ph), s.at.z + m.out.z * r);
        p.setXYZ(k, m.pos.x, m.pos.y, m.pos.z);
      });
      p.needsUpdate = c.needsUpdate = true;
    },
  };
}
