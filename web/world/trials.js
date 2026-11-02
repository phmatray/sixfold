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
      for (const p of s.plates) p.obj.userData.core.material.color.set(P.blockAt(s, p.x, p.z) ? TEAL : IDLE);
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
