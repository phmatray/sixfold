// A map's trials (puzzle.js holds the rules), drawn and moved: ice, plates, boulders pushed by hand, pillars
// shoved from afar with Reach, and the rune stones that put a trial back. `attach(w, ...)` after a map is built;
// world.js asks it what the hero faces (`facing`), pushes (`push`), uses (`use`) and slides on (`iceAt`).
import * as THREE from "three";
import { mergeGeometries } from "three/addons/utils/BufferGeometryUtils.js";
import { mesh, group, toon, prep } from "./props.js";
import { DIR, glowTex } from "./actors.js";
import { tr } from "./map.js";
import * as P from "./puzzle.js";

const TEAL = "#5fe0c8"; // Reach's colour: the pillars' bands, the beam, a held plate
const BEAM = 10, BEAM_S = 0.6; // the beam's motes and how long it shows (s)
const smooth = (t) => t * t * (3 - 2 * t);

const boulder = () => {
  const o = mesh(new THREE.DodecahedronGeometry(0.44, 0), toon("#a39a8b", { flatShading: true }), 0, 0.42, 0);
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
const plate = () => {
  const core = mesh(new THREE.CylinderGeometry(0.3, 0.3, 0.02, 24), new THREE.MeshBasicMaterial({ color: "#3a3c3a" }), 0, 0.07, 0);
  const g = group(mesh(new THREE.CylinderGeometry(0.42, 0.46, 0.06, 24), toon("#8b857a"), 0, 0.03, 0), core);
  g.userData.core = core;
  return g;
};
// A low stone with a ring cut in its face, lit while its trial can still be put back.
const rune = () => group(
  mesh(new THREE.BoxGeometry(0.62, 0.7, 0.5), toon("#7d776d", { flatShading: true }), 0, 0.35, 0),
  mesh(new THREE.TorusGeometry(0.16, 0.035, 6, 20), new THREE.MeshBasicMaterial({ color: TEAL }), 0, 0.42, 0.26),
);
// One mesh for a trial's ice: a pale sheet a few millimetres over the floor, a glint under the sun.
function iceSheet(s, floorAt) {
  const tiles = [...s.ice].map((k) => k.split(",").map(Number));
  if (!tiles.length) return null;
  const geo = mergeGeometries(tiles.map(([x, z]) => new THREE.PlaneGeometry(0.98, 0.98).rotateX(-Math.PI / 2).translate(x + 0.5, floorAt(x, z) + 0.012, z + 0.5)));
  const m = new THREE.Mesh(geo, new THREE.MeshStandardMaterial({ color: "#cdeefc", roughness: 0.15, metalness: 0.05, transparent: true, opacity: 0.85 }));
  m.receiveShadow = true;
  return m;
}

// `solved(def)`: a trial was just solved (world.js runs its `solved` dialogue, which sets its flag).
export function attach(w, holds, { reduced, sound, solved }) {
  const defs = w.map.trials;
  if (!defs?.length) return null;
  const s = P.trials(defs, (d) => holds(`flag:${d.flag}`));
  const put = (o, x, z) => (prep(o, true), o.position.set(x + 0.5, w.floorAt(x, z), z + 0.5), w.root.add(o), o);
  const ice = iceSheet(s, w.floorAt);
  if (ice) w.root.add(ice);
  for (const p of s.plates) p.obj = put(plate(), p.x, p.z);
  for (const b of s.blocks) (b.home = [b.x, b.z]), (b.obj = put(b.kind === "boulder" ? boulder() : pillar(), b.x, b.z));
  const runes = defs.filter((d) => d.reset).map((d) => ({ d, x: d.reset[0], z: d.reset[1], obj: put(rune(), ...d.reset) }));
  const runeAt = (x, z) => runes.find((r) => r.x === x && r.z === z);
  // Blocks and rune stones are solid (tap-to-walk routes round them too).
  const base = w.blocked;
  w.blocked = (x, z) => base(x, z) || !!P.blockAt(s, x, z) || !!runeAt(x, z);

  // The beam Reach sends: motes from the hero's hands to the pillar, fading.
  const beam = group(...Array.from({ length: BEAM }, () => new THREE.Sprite(new THREE.SpriteMaterial({ map: glowTex, color: TEAL, transparent: true, depthWrite: false, blending: THREE.AdditiveBlending }))));
  beam.visible = false;
  w.root.add(beam);
  let beamT = BEAM_S;

  const locked = (b) => s.solved.has(b.trial); // a solved trial's blocks stay on their plates
  // Slide `b` to (x, z) over `dur` seconds; the trial it solves is told when it lands.
  const slideBlock = (b, x, z, dur) => {
    const from = [b.x, b.z], done = P.move(s, b, x, z);
    b.anim = { from, t: reduced() ? 1 : 0, dur, done };
  };
  const wobble = (b) => !reduced() && (b.shake = 1);

  return {
    iceAt: (x, z) => P.iceAt(s, x, z),
    blockAt: (x, z) => P.blockAt(s, x, z),
    // The hero walks into a boulder: it rolls one tile on if there is room. `room(ax, az, bx, bz)`: a step
    // there would be allowed. True when it moved (the hero follows into its tile).
    push(b, f, room, dur) {
      const to = b.kind === "boulder" && !locked(b) && P.shove(s, b, f, room);
      if (!to) return false;
      slideBlock(b, ...to, dur);
      sound("place");
      return true;
    },
    // What the hero can use from here that the rest of the world doesn't know: a rune stone in front, or a
    // pillar in line for Reach.
    facing(hero) {
      const [dx, dz] = DIR[hero.face], r = runeAt(hero.x + dx, hero.z + dz);
      if (r) return { kind: "reset", text: tr("Touch the rune stone", "Toucher la pierre runique"), rune: r };
      const y = w.floorAt(hero.x, hero.z);
      const see = (x, z) => x >= 0 && z >= 0 && x < w.map.width && z < w.map.height && !"#fWV".includes(w.map.tiles[z][x]) && w.floorAt(x, z) - y <= 1.5;
      const b = P.reachTarget(s, hero.x, hero.z, hero.face, see);
      return b && !locked(b) ? { kind: "reach", text: tr("Reach for the pillar", "Pousser le pilier par l'Allonge"), block: b } : null;
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
        return;
      }
      const b = it.block, to = P.shove(s, b, hero.face, room);
      // The beam, from the hero's chest to the pillar's band.
      const a = new THREE.Vector3(hero.x + 0.5, w.floorAt(hero.x, hero.z) + 0.9, hero.z + 0.5);
      const c = new THREE.Vector3(b.x + 0.5, w.floorAt(b.x, b.z) + 1.0, b.z + 0.5);
      beam.children.forEach((m, i) => m.position.lerpVectors(a, c, (i + 1) / BEAM));
      if (!reduced()) (beam.visible = true), (beamT = 0);
      sound("whoosh");
      if (!to) return void (wobble(b), sound("refuse"));
      slideBlock(b, ...to, 0.45);
      setTimeout(() => sound("place"), reduced() ? 0 : 300);
    },
    update(dt) {
      for (const b of s.blocks) {
        const m = b.anim;
        let x = b.x, z = b.z, y = w.floorAt(b.x, b.z);
        if (m) {
          m.t = Math.min(1, m.t + dt / m.dur);
          const e = smooth(m.t);
          x = m.from[0] + (b.x - m.from[0]) * e;
          z = m.from[1] + (b.z - m.from[1]) * e;
          if (b.kind === "boulder") b.obj.children[0].rotation.x += dt * 4 * (1 - e); // a roll
          if (m.t >= 1) (b.anim = null), m.done && solved(m.done);
        }
        if (b.shake) (b.shake = Math.max(0, b.shake - dt * 2.5)), (x += Math.sin(w.now * 40) * 0.04 * b.shake);
        b.obj.position.set(x + 0.5, y, z + 0.5);
        const band = b.obj.userData.band;
        if (band) band.material.opacity = locked(b) ? 1 : 0.6 + 0.4 * Math.sin(w.now * 3);
      }
      for (const p of s.plates) p.obj.userData.core.material.color.set(P.blockAt(s, p.x, p.z) ? TEAL : "#3a3c3a");
      for (const r of runes) r.obj.children[1].material.color.set(s.solved.has(r.d) ? "#3a3c3a" : TEAL);
      if (beam.visible) {
        beamT += dt;
        const k = Math.max(0, 1 - beamT / BEAM_S);
        beam.children.forEach((m, i) => (m.material.opacity = k * (0.5 + 0.5 * Math.sin(i + w.now * 20)), m.scale.setScalar(0.35 + 0.25 * k)));
        if (k === 0) beam.visible = false;
      }
    },
  };
}
