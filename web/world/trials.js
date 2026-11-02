// A map's trials (puzzle.js holds the rules), drawn and moved: ice, plates, boulders pushed by hand, pillars
// shoved from afar with Reach, and the rune stones that put a trial back. `attach(w, ...)` after a map is built;
// world.js asks it what the hero faces (`facing`), pushes (`push`), uses (`use`) and slides on (`iceAt`).
import * as THREE from "three";
import { mergeGeometries } from "three/addons/utils/BufferGeometryUtils.js";
import { mesh, group, toon, prep } from "./props.js";
import { DIR, angle, glowTex } from "./actors.js";
import { tr } from "./map.js";
import * as P from "./puzzle.js";

const TEAL = "#5fe0c8"; // Reach's colour: the pillars' bands, the hand, a held plate
const WHITE = new THREE.Color("#ffffff");
const BEAM = 10; // motes in the trail behind Reach's hand
const CAST_S = 0.8, HIT = 0.38, PUSH_S = 0.45; // Reach: the whole cast, when the hand strikes, the pillar's slide (s)
const HAND = 1.5; // the hand at full size (it grows from a third of that on its way out)
const GUIDE = 7; // motes in the aim line to a pillar in reach
const GLITTER = 24; // ice glitter motes, reused
const RING_S = 0.7; // a pressed plate's ring of light (s)
const BURST_S = 1.6; // a gate's column of light as it opens (s)
const smooth = (t) => t * t * (3 - 2 * t);

// A round, rough boulder (a squashed icosahedron, faceted), as wide as a tile less a hand.
const boulder = () => {
  const o = mesh(new THREE.IcosahedronGeometry(0.47, 1).scale(1, 0.9, 1), toon("#b8ab95", { flatShading: true }), 0, 0.43, 0);
  o.rotation.set(0.3, 0.7, 0.1);
  return group(o);
};
const pillar = () => {
  const band = mesh(new THREE.CylinderGeometry(0.345, 0.345, 0.1, 8), new THREE.MeshBasicMaterial({ color: TEAL, transparent: true }), 0, 1.0, 0);
  const g = group(
    mesh(new THREE.CylinderGeometry(0.3, 0.36, 1.4, 8), toon("#8c97a0", { flatShading: true }), 0, 0.7, 0),
    mesh(new THREE.CylinderGeometry(0.38, 0.38, 0.12, 8), toon("#6f7a83"), 0, 1.46, 0),
    band,
  );
  g.userData.band = band;
  return g;
};
// A pale stone disc with a teal inlay, dim until a block holds it down, then lit.
const IDLE = "#2f6e66";
const plate = () => {
  const core = mesh(new THREE.CylinderGeometry(0.3, 0.3, 0.02, 24), new THREE.MeshBasicMaterial({ color: IDLE }), 0, 0.08, 0);
  const g = group(mesh(new THREE.CylinderGeometry(0.44, 0.48, 0.07, 24), toon("#c9bfa9"), 0, 0.035, 0), core);
  g.userData.core = core;
  return g;
};
// A low stone with a ring cut in its face, lit while its trial can still be put back.
const rune = () => group(
  mesh(new THREE.BoxGeometry(0.62, 0.7, 0.5), toon("#7d776d", { flatShading: true }), 0, 0.35, 0),
  mesh(new THREE.TorusGeometry(0.16, 0.035, 6, 20), new THREE.MeshBasicMaterial({ color: TEAL }), 0, 0.42, 0.26),
);
const flat = (geo, x, z, y) => geo.rotateX(-Math.PI / 2).translate(x + 0.5, y, z + 0.5);
// A trial's ice: one seamless pale-blue sheet a few millimetres over the floor, glossy and lit a little from within
// so it reads as ice in a dim cave, with a few faint frost scratches. It runs under the rocks too (their rounded
// cliffs don't fill their tiles), and the frost only over open ice.
function iceSheet(s, open) {
  const tiles = [...s.ice].map((k) => k.split(",").map(Number));
  if (!tiles.length) return null;
  const sheet = new THREE.Mesh(mergeGeometries(tiles.map(([x, z]) => flat(new THREE.PlaneGeometry(1, 1), x, z, 0.012))),
    new THREE.MeshStandardMaterial({ color: "#a9def5", emissive: "#5fb8e0", emissiveIntensity: 0.3, roughness: 0.08, metalness: 0.1 }));
  sheet.receiveShadow = true;
  let seed = 7;
  const r = () => ((seed = (seed * 16807) % 2147483647) / 2147483647);
  const scratches = tiles.filter(([x, z]) => open(x, z) && r() < 0.3).map(([x, z]) =>
    flat(new THREE.PlaneGeometry(0.25 + r() * 0.3, 0.02).rotateZ(r() * Math.PI), x + (r() - 0.5) * 0.5, z + (r() - 0.5) * 0.5, 0.014));
  if (!scratches.length) return sheet;
  const frost = new THREE.Mesh(mergeGeometries(scratches), new THREE.MeshBasicMaterial({ color: "#eefaff", transparent: true, opacity: 0.35, depthWrite: false }));
  return group(sheet, frost);
}
// Pale flagstones (one mesh): the dry tiles in the ice, a place to stop, and the dais the pillars stand on. On a
// dais's border a stone stops short of the edge, inside the cliff's rounded lip, rather than hanging over it.
const LIP = 0.2;
function flagstones(tiles, floorAt) {
  if (!tiles.length) return null;
  const set = new Set(tiles.map(String));
  const stone = ([x, z]) => {
    const raised = floorAt(x, z) > 0, cut = (dx, dz) => (raised && !set.has(`${x + dx},${z + dz}`) ? LIP : 0);
    const [w0, w1, n0, n1] = [cut(-1, 0), cut(1, 0), cut(0, -1), cut(0, 1)];
    // A box in the tile's local plane (x across, y toward the camera before flat() lays it down), trimmed per side.
    return flat(new THREE.BoxGeometry(0.94 - w0 - w1, 0.94 - n0 - n1, 0.04).translate((w0 - w1) / 2, (n1 - n0) / 2, 0), x, z, floorAt(x, z) + 0.02);
  };
  const m = new THREE.Mesh(mergeGeometries(tiles.map(stone)), toon("#b3aa98"));
  m.receiveShadow = true;
  return m;
}

// `solved(def)`: a trial was just solved (world.js runs its `solved` dialogue, which sets its flag).
export function attach(w, holds, { reduced, sound, solved }) {
  const defs = w.map.trials;
  if (!defs?.length) return null;
  const s = P.trials(defs, (d) => holds(`flag:${d.flag}`));
  const put = (o, x, z) => (prep(o, true), o.position.set(x + 0.5, w.floorAt(x, z), z + 0.5), w.root.add(o), o);
  const ice = iceSheet(s, (x, z) => w.floorAt(x, z) === 0 && !w.blocked(x, z));
  if (ice) w.root.add(ice);
  // Dry tiles, and a dais's tiles at its own height (not the rocks standing on it).
  const dais = defs.flatMap((d) => (d.dais ?? []).flatMap(([x0, z0, x1, z1]) => {
    const out = [];
    for (let z = z0; z <= z1; z++) for (let x = x0; x <= x1; x++) if (w.floorAt(x, z) === w.floorAt(x0, z0) && !w.blocked(x, z)) out.push([x, z]);
    return out;
  }));
  const stones = flagstones([...defs.flatMap((d) => d.dry ?? []), ...dais], w.floorAt);
  if (stones) w.root.add(stones);
  for (const p of s.plates) p.obj = put(plate(), p.x, p.z);
  for (const b of s.blocks) (b.home = [b.x, b.z]), (b.obj = put(b.kind === "boulder" ? boulder() : pillar(), b.x, b.z));
  const runes = defs.filter((d) => d.reset).map((d) => ({ d, x: d.reset[0], z: d.reset[1], obj: put(rune(), ...d.reset) }));
  const runeAt = (x, z) => runes.find((r) => r.x === x && r.z === z);
  // Blocks and rune stones are solid (tap-to-walk routes round them too).
  const base = w.blocked;
  w.blocked = (x, z) => base(x, z) || !!P.blockAt(s, x, z) || !!runeAt(x, z);

  // ---------- Effects (under reduced motion only the lights change: no hand, motes, rings or dust) ----------
  const fx = group();
  w.root.add(fx);
  const mote = (color = TEAL, size = 0.3) => {
    const m = new THREE.Sprite(new THREE.SpriteMaterial({ map: glowTex, color, transparent: true, opacity: 0, depthWrite: false, blending: THREE.AdditiveBlending }));
    m.scale.setScalar(size);
    return fx.add(m), m;
  };
  // Reach: the hand flies out, strikes and fades (`cast`), a trail of motes behind it and a glow at the hero's chest.
  const handMat = new THREE.MeshBasicMaterial({ color: TEAL, transparent: true, opacity: 0, depthWrite: false, side: THREE.DoubleSide, blending: THREE.AdditiveBlending });
  const edgeMat = new THREE.LineBasicMaterial({ color: "#e4fff8", transparent: true, opacity: 0, depthWrite: false, blending: THREE.AdditiveBlending });
  const hand = reachHand(handMat, edgeMat);
  hand.visible = false;
  fx.add(hand);
  const trail = Array.from({ length: BEAM }, () => mote(TEAL, 0.3)), aura = mote(TEAL, 1.1);
  let cast = null;
  // The aim: when a pillar is in Reach, a dotted line of motes flows along the floor to it.
  const guide = Array.from({ length: GUIDE }, () => mote(TEAL, 0.16));
  // A plate pressed: a ring of light spreads from it. A gate opened: a column of light (`burst`).
  const rings = [], bursts = [];
  // The ice: glitter kicked up behind a sliding hero.
  const glitter = Array.from({ length: GLITTER }, () => Object.assign(mote("#e8f8ff", 0.14), { life: 0 }));
  let gi = 0, glitterAt = 0, wasSliding = false;

  const locked = (b) => s.solved.has(b.trial); // a solved trial's blocks stay on their plates
  // Slide `b` to (x, z) over `dur` seconds, after `delay`; the trial it solves is told when it lands.
  const slideBlock = (b, x, z, dur, delay = 0) => {
    const from = [b.x, b.z], done = P.move(s, b, x, z);
    b.anim = { from, t: reduced() ? 1 : -delay / dur, dur, done };
  };
  const wobble = (b) => !reduced() && (b.shake = 1);
  const center = (x, z, y = 0) => new THREE.Vector3(x + 0.5, w.floorAt(x, z) + y, z + 0.5);
  // The pillar Reach would shove now (the hero facing it, standing still), or null.
  const aimed = (hero) => {
    if (!hero || hero.step || hero.sit) return null;
    const y = w.floorAt(hero.x, hero.z);
    const see = (x, z) => x >= 0 && z >= 0 && x < w.map.width && z < w.map.height && !"#fWV".includes(w.map.tiles[z][x]) && w.floorAt(x, z) - y <= 1.5;
    const b = P.reachTarget(s, hero.x, hero.z, hero.face, see);
    return b && !locked(b) ? b : null;
  };
  for (const p of s.plates) p.held = !!P.blockAt(s, p.x, p.z);

  return {
    iceAt: (x, z) => P.iceAt(s, x, z),
    blockAt: (x, z) => P.blockAt(s, x, z),
    // The hero walks into a boulder: it rolls one tile on if there is room. `room(ax, az, bx, bz)`: a step
    // there would be allowed. True when it moved (the hero follows into its tile).
    push(b, f, room, dur) {
      const to = b.kind === "boulder" && !locked(b) && P.shove(s, b, f, room);
      if (!to) return false;
      slideBlock(b, ...to, dur); // it thuds as it settles (update)
      return true;
    },
    // What the hero can use from here that the rest of the world doesn't know: a rune stone in front, or a
    // pillar in line for Reach.
    facing(hero) {
      const [dx, dz] = DIR[hero.face], r = runeAt(hero.x + dx, hero.z + dz);
      if (r) return { kind: "reset", text: tr("Touch the rune stone", "Toucher la pierre runique"), rune: r };
      const b = !cast && aimed(hero);
      return b ? { kind: "reach", text: tr("Reach for the pillar", "Pousser le pilier par l'Allonge"), block: b } : null;
    },
    busy: () => !!cast, // the hand is out: the hero waits for it
    // A trial's gate opening (world.js, as its reveal watches): a column of light rises out of the doorway as the
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
    use(it, hero, room) {
      if (it.kind === "reset") {
        sound("whoosh");
        for (const b of s.blocks) {
          if (b.trial !== it.rune.d || locked(b) || (b.x === b.home[0] && b.z === b.home[1])) continue;
          w.life.puff(b.x + 0.5, b.z + 0.5);
          P.move(s, b, ...b.home);
          b.anim = null;
          w.life.puff(b.x + 0.5, b.z + 0.5);
        }
        for (const p of s.plates) p.held = !!P.blockAt(s, p.x, p.z);
        return;
      }
      const b = it.block, to = P.shove(s, b, hero.face, room);
      sound("energy");
      setTimeout(() => sound("whoosh"), reduced() ? 0 : 120);
      if (reduced()) return void (to ? (slideBlock(b, ...to, 0.45), sound("place")) : sound("refuse"));
      // The hand leaves the hero's chest, flies to just short of the pillar's face and strikes it at HIT; the
      // pillar slides from then on, the hand pushing behind it as it fades.
      const [dx, dz] = DIR[hero.face];
      const a = center(hero.x, hero.z, 0.95).add(new THREE.Vector3(dx * 0.35, 0, dz * 0.35));
      const c = center(b.x, b.z, 0.85).sub(new THREE.Vector3(dx * 0.62, 0, dz * 0.62));
      hand.rotation.y = angle(hero.face);
      cast = { a, c, d: new THREE.Vector3(dx, 0, dz), t: 0, b, ok: !!to, struck: false };
      if (to) slideBlock(b, ...to, PUSH_S, HIT);
    },
    update(dt, hero) {
      for (const b of s.blocks) {
        const m = b.anim;
        let x = b.x, z = b.z;
        const y = w.floorAt(b.x, b.z);
        if (m) {
          m.t = Math.min(1, m.t + dt / m.dur);
          const e = smooth(Math.max(0, m.t));
          x = m.from[0] + (b.x - m.from[0]) * e;
          z = m.from[1] + (b.z - m.from[1]) * e;
          if (b.kind === "boulder") b.obj.children[0].rotation.x += dt * 4 * (1 - e); // a roll
          if (m.t >= 1) {
            b.anim = null;
            w.life.puff(b.x + 0.5, b.z + 0.5); // dust as it settles
            sound("place");
            if (m.done) solved(m.done);
          }
        }
        if (b.shake) (b.shake = Math.max(0, b.shake - dt * 2.5)), (x += Math.sin(w.now * 40) * 0.04 * b.shake);
        b.obj.position.set(x + 0.5, y, z + 0.5);
      }
      // Plates: lit while held; pressed just now, a flash and a ring of light (and two bright notes).
      for (const p of s.plates) {
        const held = !!P.blockAt(s, p.x, p.z) && !P.blockAt(s, p.x, p.z).anim;
        if (held && !p.held) {
          sound("heal");
          p.flash = 1;
          if (!reduced()) rings.push(ring(p.x, p.z));
        }
        p.held = held;
        p.flash = Math.max(0, (p.flash ?? 0) - dt * 2);
        p.obj.userData.core.material.color.set(held ? TEAL : IDLE).lerp(WHITE, p.flash);
      }
      for (const r of rings) {
        r.t += dt / RING_S;
        r.obj.scale.setScalar(1 + r.t * 1.6);
        r.obj.material.opacity = 0.8 * (1 - r.t) ** 2;
      }
      for (const r of rings.filter((r) => r.t >= 1)) fx.remove(r.obj), r.obj.geometry.dispose(), r.obj.material.dispose(), rings.splice(rings.indexOf(r), 1);
      for (const r of runes) r.obj.children[1].material.color.set(s.solved.has(r.d) ? "#3a3c3a" : TEAL);
      for (const b of [...bursts]) {
        const k = (b.t += dt) / BURST_S;
        b.col.material.opacity = 0.7 * Math.sin(Math.min(1, k) * Math.PI) ** 0.7;
        b.col.scale.set(1 + 0.3 * k, 0.4 + 0.8 * Math.min(1, k * 2), 1 + 0.3 * k);
        for (const m of b.motes) {
          const y = b.h + m.v * b.t;
          m.position.copy(b.col.position).add(new THREE.Vector3(Math.cos(m.a + b.t * 2) * m.r, y, Math.sin(m.a + b.t * 2) * m.r));
          m.material.opacity = 0.9 * Math.max(0, 1 - k) * Math.min(1, b.t * 4);
        }
        if (k >= 1) {
          for (const o of [b.col, ...b.motes]) fx.remove(o), o.geometry?.dispose(), o.material.dispose();
          bursts.splice(bursts.indexOf(b), 1);
        }
      }

      // Reach: the cast, then the aim.
      if (cast) castStep(dt);
      const target = cast ? null : aimed(hero);
      for (const b of s.blocks) {
        const band = b.obj.userData.band;
        if (!band) continue;
        b.flash = Math.max(0, (b.flash ?? 0) - dt * 2.5);
        const lit = locked(b) || b === target;
        band.material.opacity = lit ? 1 : 0.55 + 0.35 * Math.sin(w.now * 3);
        band.material.color.set(TEAL).lerp(WHITE, Math.max(b.flash, b === target ? 0.25 + 0.15 * Math.sin(w.now * 6) : 0));
        band.scale.setScalar(1 + 0.12 * b.flash);
      }
      const from = hero && center(hero.x, hero.z, 0.06), to = target && center(target.x, target.z, 0.06);
      guide.forEach((m, i) => {
        if (!to || reduced()) return void (m.material.opacity = 0);
        const k = (i / GUIDE + w.now * 0.7) % 1;
        m.position.lerpVectors(from, to, 0.12 + k * 0.76);
        m.material.opacity = 0.55 * Math.sin(k * Math.PI);
      });

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

  // A ring of light flat on a plate, spreading and fading over RING_S.
  function ring(x, z) {
    const obj = new THREE.Mesh(new THREE.TorusGeometry(0.42, 0.035, 6, 40).rotateX(Math.PI / 2), new THREE.MeshBasicMaterial({ color: TEAL, transparent: true, depthWrite: false, blending: THREE.AdditiveBlending }));
    obj.position.copy(center(x, z, 0.1));
    fx.add(obj);
    return { obj, t: 0 };
  }
  // The hand's flight (0..HIT, eased, growing from small), its strike (a short thrust and the pillar's band
  // flashing; a pillar that can't move shakes instead), then its follow-through and fade (to CAST_S).
  function castStep(dt) {
    const k = (cast.t += dt) / CAST_S, hit = HIT / CAST_S;
    const p = new THREE.Vector3();
    let size, alpha;
    if (k < hit) {
      const e = smooth(k / hit);
      p.lerpVectors(cast.a, cast.c, e);
      p.y += Math.sin(e * Math.PI) * 0.25; // a little arc
      (size = HAND * (0.3 + 0.7 * e)), (alpha = 0.8 * Math.min(1, (k / hit) * 3));
    } else {
      const e = (k - hit) / (1 - hit);
      if (!cast.struck) {
        cast.struck = true;
        cast.b.flash = 1;
        if (!cast.ok) wobble(cast.b), sound("refuse");
      }
      p.copy(cast.c).addScaledVector(cast.d, cast.ok ? Math.min(1, e * 1.4) * 0.9 : Math.sin(Math.min(1, e * 3) * Math.PI) * 0.12);
      (size = HAND * (1 + 0.15 * e)), (alpha = 0.8 * (1 - e) ** 1.5);
    }
    hand.visible = true;
    hand.position.copy(p);
    hand.scale.setScalar(size);
    handMat.opacity = alpha * 0.38; // the fill and its halo overlap (additive): a brighter core, a soft rim
    edgeMat.opacity = Math.min(1, alpha * 1.25);
    trail.forEach((m, i) => {
      m.position.lerpVectors(cast.a, p, (i + 1) / (BEAM + 1));
      m.material.opacity = alpha * 0.6 * (0.4 + 0.6 * (i / BEAM));
    });
    aura.position.copy(cast.a).addScaledVector(cast.d, -0.35);
    aura.material.opacity = 0.5 * Math.max(0, 1 - k * 1.3);
    if (k >= 1) {
      cast = null;
      hand.visible = false;
      handMat.opacity = edgeMat.opacity = 0;
      for (const m of [...trail, aura]) m.material.opacity = 0;
    }
  }
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

// Reach's hand: the silhouette of an open hand (a flat shape: four rounded fingers, a thumb out to the side),
// a see-through additive teal fill (`mat`), one bright outline (`edge`) and a faint halo a size up. Drawn fingers
// up (+y), then laid nearly flat with the fingers pointing ahead (+z, the group turned to the hero's facing) and
// tilted up a little, palm down: from the camera's height it reads whole whichever way it pushes, never edge-on.
const TILT = 0.45; // radians the fingers rise off flat
function handShape() {
  const s = new THREE.Shape(), base = 0.1;
  s.moveTo(-0.17, -0.22);
  s.quadraticCurveTo(-0.235, -0.05, -0.21, base);
  // Pinky to index, left to right: each finger up its left side, round its tip, down to the gap before the next.
  for (const [cx, top] of [[-0.165, 0.33], [-0.058, 0.44], [0.05, 0.46], [0.155, 0.4]]) {
    const r = 0.046;
    s.lineTo(cx - r, top);
    s.absarc(cx, top, r, Math.PI, 0, true);
    s.lineTo(cx + r, base + 0.02);
  }
  s.lineTo(0.215, 0.03); // the thumb: out and up, a rounded tip, back to the wrist
  s.lineTo(0.33, 0.14);
  s.quadraticCurveTo(0.405, 0.18, 0.39, 0.1);
  s.lineTo(0.235, -0.1);
  s.lineTo(0.17, -0.22);
  s.closePath();
  return s;
}
function reachHand(mat, edge) {
  const shape = handShape(), fill = new THREE.ShapeGeometry(shape, 8);
  const halo = new THREE.Mesh(fill, mat);
  halo.scale.setScalar(1.14);
  halo.position.z = -0.01;
  const palm = group(halo, new THREE.Mesh(fill, mat), new THREE.LineLoop(new THREE.BufferGeometry().setFromPoints(shape.getPoints(8)), edge));
  palm.rotation.x = Math.PI / 2 - TILT;
  return group(palm);
}
