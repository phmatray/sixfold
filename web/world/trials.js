// A map's trials (puzzle.js holds the rules, docs/PUZZLES.md the design), drawn and moved: crates on chalk marks,
// ice, logs and the channels they bridge, and the rune stones that put a trial back. `attach(w, ...)` after a map
// is built; world.js asks it what's in the way (`blockAt`, and `w.blocked` it wraps), pushes (`push`), what the hero
// faces (`facing`: a rune stone), uses it (`use`), whether a tile is ice (`iceAt`) and whether a goal was reached.
import * as THREE from "three";
import { mergeGeometries } from "three/addons/utils/BufferGeometryUtils.js";
import { mesh, group, toon, prep } from "./props.js";
import { DIR, glowTex } from "./actors.js";
import { tr } from "./map.js";
import * as P from "./puzzle.js";

const TEAL = "#5fe0c8"; // the trials' colour: a held mark, the rune stones, a gate's column of light
const WHITE = new THREE.Color("#ffffff");
const GLITTER = 24; // ice glitter motes, reused
const RING_S = 0.7; // a pressed mark's ring of light (s)
const BURST_S = 1.6; // a gate's column of light as it opens (s)
const LOG_R = 0.34; // a log's radius
const smooth = (t) => t * t * (3 - 2 * t);
const flat = (geo, x, z, y) => geo.rotateX(-Math.PI / 2).translate(x + 0.5, y, z + 0.5);
// Anything painted on a surface (frost on the ice, chalk on the boards, the ice and water on the floor) is pulled
// toward the camera in depth: millimetres apart, two planes otherwise flicker against each other at a distance.
const DECAL = { polygonOffset: true, polygonOffsetFactor: -2, polygonOffsetUnits: -4 };

// ---------- Models (code-built, like the other placeholders: real ones come when the prototype is kept) ----------

// A wooden crate: planks, a darker frame round its top and foot, a cross brace on the side the camera sees.
function crate() {
  const wood = toon("#ddb077"), frame = toon("#8a5f38"); // warm and light: the crates that move (the walls are dark green stacks)
  const g = group(mesh(new THREE.BoxGeometry(0.8, 0.8, 0.8), wood, 0, 0.42, 0));
  // The frames stand proud of the box (its top at 0.82): no two faces share a plane, so nothing flickers.
  for (const y of [0.06, 0.8]) g.add(mesh(new THREE.BoxGeometry(0.86, 0.08, 0.86), frame, 0, y, 0));
  for (const s of [-1, 1]) {
    const brace = mesh(new THREE.BoxGeometry(0.1, 0.98, 0.04), frame, 0, 0.42, 0.41);
    brace.rotation.z = s * Math.PI / 4;
    g.add(brace);
  }
  return g;
}
// A block of pale stone (the ice cave's crates): a squared boulder, faceted.
function stone() {
  const o = mesh(new THREE.BoxGeometry(0.84, 0.78, 0.84, 1, 1, 1), toon("#a6b3bc", { flatShading: true }), 0, 0.39, 0);
  return group(o, mesh(new THREE.BoxGeometry(0.7, 0.06, 0.7), toon("#c7d2d9"), 0, 0.8, 0));
}
// A log lying along +x, LOG tiles long: bark, the pale cut ends with a ring. Its own group rolls (`roll`).
function log() {
  const len = P.LOG - 0.1, bark = toon("#7a5634"), cut = toon("#dcbf8a");
  const body = mesh(new THREE.CylinderGeometry(LOG_R, LOG_R, len, 10).rotateZ(Math.PI / 2), bark);
  const ends = [-1, 1].map((s) => mesh(new THREE.CylinderGeometry(LOG_R * 0.92, LOG_R * 0.92, 0.02, 10).rotateZ(Math.PI / 2), cut, s * (len / 2 + 0.012), 0, 0)); // just out from the bark's caps
  const knot = mesh(new THREE.BoxGeometry(0.16, 0.08, 0.1), bark, 0.3, LOG_R - 0.01, 0); // shows the roll
  const roll = group(body, ...ends, knot);
  const g = group(roll);
  g.userData.roll = roll;
  return g;
}
// A mark a crate goes on: a disc of pale stone with a teal inlay (`look: "stone"`), or a chalk square on the depot's
// boards (`look: "crate"`). Dim until a crate holds it down, then lit.
const IDLE = "#2f6e66", CHALK = "#ece5d2";
function mark(look) {
  if (look === "crate") {
    const m = new THREE.MeshBasicMaterial({ color: CHALK, transparent: true, opacity: 0.85, ...DECAL });
    const side = (w, d, x, z) => mesh(new THREE.BoxGeometry(w, 0.01, d), m, x, 0.012, z);
    const g = group(side(0.84, 0.06, 0, -0.39), side(0.84, 0.06, 0, 0.39), side(0.06, 0.84, -0.39, 0), side(0.06, 0.84, 0.39, 0),
      mesh(new THREE.BoxGeometry(0.5, 0.01, 0.05).rotateY(Math.PI / 4), m, 0, 0.012, 0), mesh(new THREE.BoxGeometry(0.5, 0.01, 0.05).rotateY(-Math.PI / 4), m, 0, 0.012, 0));
    g.userData.core = { material: m };
    g.userData.idle = CHALK;
    return g;
  }
  const core = mesh(new THREE.CylinderGeometry(0.3, 0.3, 0.02, 24), new THREE.MeshBasicMaterial({ color: IDLE }), 0, 0.08, 0);
  const g = group(mesh(new THREE.CylinderGeometry(0.44, 0.48, 0.07, 24), toon("#c9bfa9"), 0, 0.035, 0), core);
  g.userData.core = core;
  g.userData.idle = IDLE;
  return g;
}
// A low stone with a ring cut in its face, lit while its trial can still be put back.
const rune = () => group(
  mesh(new THREE.BoxGeometry(0.62, 0.7, 0.5), toon("#7d776d", { flatShading: true }), 0, 0.35, 0),
  mesh(new THREE.TorusGeometry(0.16, 0.035, 6, 20), new THREE.MeshBasicMaterial({ color: TEAL }), 0, 0.42, 0.26),
);
// The ice: one seamless pale-blue sheet a few millimetres over the floor, glossy and lit a little from within so it
// reads as ice in a dim cave, and a few faint frost scratches over the open ice. It runs under the rocks too (their
// rounded cliffs don't fill their tiles).
function iceSheet(s, open) {
  const tiles = [...s.ice].map((k) => k.split(",").map(Number));
  if (!tiles.length) return null;
  const sheet = new THREE.Mesh(mergeGeometries(tiles.map(([x, z]) => flat(new THREE.PlaneGeometry(1, 1), x, z, 0.012))),
    new THREE.MeshStandardMaterial({ color: "#a9def5", emissive: "#5fb8e0", emissiveIntensity: 0.3, roughness: 0.08, metalness: 0.1, polygonOffset: true, polygonOffsetFactor: -1, polygonOffsetUnits: -2 }));
  sheet.receiveShadow = true;
  let seed = 7;
  const r = () => ((seed = (seed * 16807) % 2147483647) / 2147483647);
  const scratches = tiles.filter(([x, z]) => open(x, z) && r() < 0.3).map(([x, z]) =>
    flat(new THREE.PlaneGeometry(0.25 + r() * 0.3, 0.02).rotateZ(r() * Math.PI), x + (r() - 0.5) * 0.5, z + (r() - 0.5) * 0.5, 0.012));
  if (!scratches.length) return sheet;
  const frost = new THREE.Mesh(mergeGeometries(scratches), new THREE.MeshBasicMaterial({ color: "#eefaff", transparent: true, opacity: 0.35, depthWrite: false, ...DECAL }));
  return group(sheet, frost);
}
// Pale flagstones (one mesh): the dry tiles in the ice, where a slide stops.
function flagstones(tiles) {
  if (!tiles.length) return null;
  const m = new THREE.Mesh(mergeGeometries(tiles.map(([x, z]) => flat(new THREE.BoxGeometry(0.94, 0.94, 0.04), x, z, 0.02))), toon("#b3aa98"));
  m.receiveShadow = true;
  return m;
}
// The channels: running water a little over the floor (the ground can't be cut), dark at the banks.
function water(s) {
  const tiles = [...s.channel].map((k) => k.split(",").map(Number));
  if (!tiles.length) return null;
  const m = new THREE.Mesh(mergeGeometries(tiles.map(([x, z]) => flat(new THREE.PlaneGeometry(1, 1), x, z, 0.03))),
    new THREE.MeshStandardMaterial({ color: "#2f6f8c", emissive: "#1d4f66", emissiveIntensity: 0.5, roughness: 0.15, metalness: 0.1, transparent: true, opacity: 0.92, ...DECAL }));
  m.receiveShadow = true;
  return m;
}
// A vertical fade for a column of light: solid at its foot, gone at its top (an alpha map, made once).
let fadeTex = null;
function fadeUp() {
  if (fadeTex) return fadeTex;
  const c = document.createElement("canvas");
  (c.width = 4), (c.height = 64);
  const g = c.getContext("2d"), grad = g.createLinearGradient(0, 0, 0, 64);
  grad.addColorStop(0, "#000"), grad.addColorStop(0.3, "#aaa"), grad.addColorStop(1, "#fff"); // canvas top = the column's top
  (g.fillStyle = grad), g.fillRect(0, 0, 4, 64);
  return (fadeTex = new THREE.CanvasTexture(c));
}

// `solved(def)`: a trial was just solved (world.js reveals its gate and runs its `solved` lines, which set its flag).
export function attach(w, holds, { reduced, sound, solved }) {
  const defs = w.map.trials;
  if (!defs?.length) return null;
  const s = P.trials(defs, (d) => holds(`flag:${d.flag}`));
  const put = (o, x, z) => (prep(o, true), o.position.set(x + 0.5, w.floorAt(x, z), z + 0.5), w.root.add(o), o);
  for (const o of [iceSheet(s, (x, z) => w.floorAt(x, z) === 0 && !w.blocked(x, z)), flagstones(defs.flatMap((d) => d.dry ?? [])), water(s)]) if (o) w.root.add(o);
  for (const p of s.plates) p.obj = put(mark(p.trial.look), p.x, p.z);
  for (const b of s.blocks) {
    b.home = [b.x, b.z, b.axis];
    b.obj = put(b.kind === "log" ? log() : b.trial.look === "stone" ? stone() : crate(), b.x, b.z);
  }
  const runes = defs.filter((d) => d.reset).map((d) => ({ d, x: d.reset[0], z: d.reset[1], obj: put(rune(), ...d.reset) }));
  // What waits at a goal worth a word (a trial with a goal and `solved` lines: the lost card): a card face down,
  // and a glint over it, seen from the entrance (the Lens of Curiosity), gone once it's found.
  for (const d of defs) if (d.goal && d.flag && holds(`flag:${d.flag}`)) s.solved.add(d); // found on an earlier visit
  const prizes = defs.filter((d) => d.goal && d.solved && !s.solved.has(d)).map((d) => {
    const card = mesh(new THREE.BoxGeometry(0.3, 0.012, 0.42), toon("#3f5fa8"), 0.05, 0.03, 0);
    card.rotation.y = 0.5;
    card.add(mesh(new THREE.BoxGeometry(0.24, 0.002, 0.36), toon("#e8dcc0"), 0, 0.007, 0));
    const obj = put(group(card), ...d.goal);
    return { d, obj, glint: null };
  });
  const runeAt = (x, z) => runes.find((r) => r.x === x && r.z === z);
  // Blocks, rune stones and open channels are solid (tap-to-walk routes round them too).
  const base = w.blocked;
  w.blocked = (x, z) => base(x, z) || !!P.blockAt(s, x, z) || !!runeAt(x, z) || P.gapAt(s, x, z);

  // ---------- Effects (under reduced motion only the lights change: no rings, motes, dust or roll) ----------
  const fx = group();
  w.root.add(fx);
  const mote = (color, size) => {
    const m = new THREE.Sprite(new THREE.SpriteMaterial({ map: glowTex, color, transparent: true, opacity: 0, depthWrite: false, blending: THREE.AdditiveBlending }));
    m.scale.setScalar(size);
    return fx.add(m), m;
  };
  const rings = [], bursts = [];
  const glitter = Array.from({ length: GLITTER }, () => Object.assign(mote("#e8f8ff", 0.14), { life: 0 }));
  for (const p of prizes) (p.glint = mote("#fff6d8", 0.6)), p.glint.position.copy(p.obj.position).setY(p.obj.position.y + 0.35);
  let gi = 0, glitterAt = 0, wasSliding = false;
  for (const p of s.plates) p.held = !!P.blockAt(s, p.x, p.z);

  const locked = (b) => s.solved.has(b.trial); // a solved trial's crates stay on their marks
  // Where a block stands in the scene: its tiles' middle, a log's lying along its axis (a sunk one low in the water).
  const placeOf = (b, x, z) => {
    if (b.kind !== "log") return [x + 0.5, w.floorAt(x, z), z + 0.5];
    const along = (P.LOG - 1) / 2;
    return b.axis === "x" ? [x + 0.5 + along, LOG_R, z + 0.5] : [x + 0.5, LOG_R, z + 0.5 + along];
  };
  const center = (x, z, y = 0) => new THREE.Vector3(x + 0.5, w.floorAt(x, z) + y, z + 0.5);
  const drawLog = (b) => b.obj.rotation.set(0, b.axis === "x" ? 0 : Math.PI / 2, 0);
  for (const b of s.blocks) if (b.kind === "log") drawLog(b), b.obj.position.set(...placeOf(b, b.x, b.z));

  return {
    iceAt: (x, z) => P.iceAt(s, x, z),
    blockAt: (x, z) => P.blockAt(s, x, z),
    // The hero walks into a block: a crate moves one tile (on over ice), a log rolls across its length until it
    // stops (into a channel its size). `room(ax, az, bx, bz)`: a step there would be allowed. True when it moved
    // (the hero follows into the tile it pushed); `step`: a tile's time in seconds.
    push(b, f, room, step) {
      if (locked(b)) return false;
      const to = P.shove(s, b, f, room);
      if (!to) return false;
      const from = [b.x, b.z], done = P.move(s, b, to);
      b.anim = { from, t: reduced() ? 1 : 0, dur: step * (b.kind === "log" ? 0.8 : 1) * to.n + (to.sunk ? 0.25 : 0), done, sunk: to.sunk, f };
      return true;
    },
    // A rune stone in front of the hero.
    facing(hero) {
      const [dx, dz] = DIR[hero.face], r = runeAt(hero.x + dx, hero.z + dz);
      return r ? { kind: "reset", text: tr("Touch the rune stone", "Toucher la pierre runique"), rune: r } : null;
    },
    // Put a trial's blocks back where they started (unless it's solved), with a puff where each one was and is.
    // It won't while one of them is still moving (a crate sliding onto the last mark would leave the trial solved
    // with its lines never run), nor while the hero stands where a block would come back to, or on a log's bridge.
    use(it, hero) {
      const mine = s.blocks.filter((b) => b.trial === it.rune.d && !locked(b));
      const on = ([x, z]) => x === hero.x && z === hero.z;
      if (mine.some((b) => b.anim || P.cells({ ...b, x: b.home[0], z: b.home[1] }).some(on) || (b.sunk && P.cells(b).some(on)))) return void sound("refuse");
      sound("whoosh");
      for (const b of mine) {
        w.life.puff(...placeOf(b, b.x, b.z).filter((_, i) => i !== 1));
        P.place(s, b, b.home[0], b.home[1]);
        if (b.kind === "log") b.obj.userData.roll.rotation.x = 0;
        w.life.puff(...placeOf(b, b.x, b.z).filter((_, i) => i !== 1));
      }
      for (const p of s.plates) p.held = !!P.blockAt(s, p.x, p.z);
    },
    // The hero stands on (x, z): a trial whose goal that is is solved (the end of the ice, the far bank).
    reached(x, z) {
      for (const d of defs) {
        if (!d.goal || !d.solved || s.solved.has(d) || d.goal[0] !== x || d.goal[1] !== z) continue;
        s.solved.add(d);
        for (const p of prizes.filter((p) => p.d === d)) (p.obj.visible = false), (p.glint.visible = false); // picked up
        solved(d);
      }
    },
    // A trial's gate opening (world.js, as its reveal watches): a column of light rises out of the doorway as its
    // seal sinks, with motes drifting up, and fades.
    burst(x, z) {
      if (reduced()) return;
      const col = new THREE.Mesh(new THREE.CylinderGeometry(0.36, 0.5, 3.2, 20, 1, true).translate(0, 1.6, 0), // clear of the walls either side
        new THREE.MeshBasicMaterial({ color: TEAL, alphaMap: fadeUp(), transparent: true, opacity: 0, depthWrite: false, side: THREE.DoubleSide, blending: THREE.AdditiveBlending }));
      col.position.copy(center(x, z));
      fx.add(col);
      const motes = Array.from({ length: 14 }, () => Object.assign(mote(TEAL, 0.18 + Math.random() * 0.14), {
        v: 0.8 + Math.random() * 1.4, a: Math.random() * Math.PI * 2, r: 0.15 + Math.random() * 0.35, h: Math.random() * 0.4 }));
      bursts.push({ col, motes, t: 0 });
    },
    update(dt, hero) {
      for (const b of s.blocks) {
        const m = b.anim;
        let [x, y, z] = placeOf(b, b.x, b.z);
        if (m) {
          m.t = Math.min(1, m.t + dt / m.dur);
          const e = smooth(m.t), [fx0, , fz0] = placeOf(b, ...m.from);
          x = fx0 + (x - fx0) * e;
          z = fz0 + (z - fz0) * e;
          if (b.kind === "log") {
            // It rolls (one turn per 2πr travelled), and a log going into a channel drops in at the end.
            const d = Math.hypot(x - fx0, z - fz0), [dx, dz] = DIR[m.f];
            b.obj.userData.roll.rotation.x = (b.axis === "x" ? dz : -dx) * (d / LOG_R) + (m.roll0 ??= b.obj.userData.roll.rotation.x);
            if (m.sunk) y = LOG_R - Math.max(0, (m.t - 0.8) / 0.2) * (LOG_R + 0.12);
          }
          if (m.t >= 1) {
            b.anim = null;
            w.life.puff(x, z); // dust as it settles (a splash, for a log in the water)
            sound(m.sunk ? "draw" : "place");
            if (m.done) solved(m.done);
          }
        } else if (b.sunk) y = -0.12;
        b.obj.position.set(x, y, z);
      }
      // Marks: lit while held; pressed just now, a flash and a ring of light (and two bright notes).
      for (const p of s.plates) {
        const on = P.blockAt(s, p.x, p.z), held = !!on && !on.anim;
        if (held && !p.held) {
          sound("heal");
          p.flash = 1;
          if (!reduced()) rings.push(ring(p.x, p.z));
        }
        p.held = held;
        p.flash = Math.max(0, (p.flash ?? 0) - dt * 2);
        p.obj.userData.core.material.color.set(held ? TEAL : p.obj.userData.idle).lerp(WHITE, p.flash);
      }
      for (const r of [...rings]) {
        r.t += dt / RING_S;
        r.obj.scale.setScalar(1 + r.t * 1.6);
        r.obj.material.opacity = 0.8 * (1 - r.t) ** 2;
        if (r.t >= 1) fx.remove(r.obj), r.obj.geometry.dispose(), r.obj.material.dispose(), rings.splice(rings.indexOf(r), 1);
      }
      for (const r of runes) r.obj.children[1].material.color.set(s.solved.has(r.d) ? "#3a3c3a" : TEAL);
      // The prize's glint: a slow pulse (steady under reduced motion), so it reads as "something's there".
      for (const p of prizes) if (p.glint.visible) p.glint.material.opacity = reduced() ? 0.8 : 0.45 + 0.45 * Math.sin(w.now * 2.4) ** 2;
      for (const b of [...bursts]) {
        const k = (b.t += dt) / BURST_S;
        b.col.material.opacity = 0.7 * Math.sin(Math.min(1, k) * Math.PI) ** 0.7;
        b.col.scale.set(1 + 0.3 * k, 0.4 + 0.8 * Math.min(1, k * 2), 1 + 0.3 * k);
        for (const m of b.motes) {
          m.position.copy(b.col.position).add(new THREE.Vector3(Math.cos(m.a + b.t * 2) * m.r, b.h + m.v * b.t, Math.sin(m.a + b.t * 2) * m.r));
          m.material.opacity = 0.9 * Math.max(0, 1 - k) * Math.min(1, b.t * 4);
        }
        if (k >= 1) {
          for (const o of [b.col, ...b.motes]) fx.remove(o), o.geometry?.dispose(), o.material.dispose();
          bursts.splice(bursts.indexOf(b), 1);
        }
      }
      // The ice: a swish as a slide starts, glitter behind the hero while it lasts.
      const sliding = !!hero?.step?.slide;
      if (sliding && !wasSliding) sound("draw");
      wasSliding = sliding;
      if (sliding && !reduced() && (glitterAt -= dt) <= 0) {
        glitterAt = 0.035;
        const g = glitter[gi++ % GLITTER], p = hero.obj.position;
        g.position.set(p.x + (Math.random() - 0.5) * 0.4, 0.06, p.z + (Math.random() - 0.5) * 0.4);
        g.life = 1;
      }
      for (const g of glitter) {
        if (g.life <= 0) continue;
        g.life = Math.max(0, g.life - dt / 0.6);
        g.position.y += dt * 0.35;
        g.material.opacity = g.life * 0.9;
      }
    },
  };

  // A ring of light flat on a mark, spreading and fading over RING_S.
  function ring(x, z) {
    const obj = new THREE.Mesh(new THREE.TorusGeometry(0.42, 0.035, 6, 40).rotateX(Math.PI / 2), new THREE.MeshBasicMaterial({ color: TEAL, transparent: true, depthWrite: false, blending: THREE.AdditiveBlending }));
    obj.position.copy(center(x, z, 0.1));
    fx.add(obj);
    return { obj, t: 0 };
  }
}
