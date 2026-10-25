// Lifts: a cage that carries the hero between two heights of one map (the Rodspire's).
// A map's `lifts`: [{ id, x, z, top, bottom?, head?, cage?, rot?, if?, locked? }]
//   x, z    the shaft tile the cage stands on (a tower prop's footprint may cover it: it stays walkable)
//   top     the floor height of the upper landing; `bottom` the lower one (default: the tile's own)
//   if      the power: while it fails the cage stays put and `locked` is said
//   head    where the cables end, at the sheave (default top + 3.1); `cage` the model (lift_cage); `rot`
// The landings are the shaft's neighbours at either height that a gate of the cage faces; the hero
// only gets in or out there, through the open gate. The cage model: `gate*` nodes hinged on their +X
// edge (they swing out), `cable*` nodes 1 m tall from their origin, stretched up to `head`.
// Inside, the hero can ride (E, the prompt, or a tap on the cage); on a landing it isn't at, call it.
// The hero's tile never changes during a ride (no "at": nothing is saved mid-way), and a reload on
// the shaft tile starts at the bottom.
import * as THREE from "three";
import { DIR, BACK } from "./actors.js";
import { prep, placeholderBox } from "./props.js";
import { tr } from "./map.js";

const DECK = 0.1; // the cage floor over its base
const CLIMB = 0.6; // as build.js: the most one step may rise or drop
const SWING = 1.66; // how far a gate swings open (rad)
const SHUT = 0.35, SETTLE = 0.55, OPEN = 0.35; // seconds: gate shuts, bob at the stop, gate opens
const PER_M = 0.5; // travel seconds per metre (the 6 m Rodspire: 3 s)
const ease = (t) => t * t * (3 - 2 * t);
const LEVELS = ["bottom", "top"];

// Put each lift's cage in `w.root` and lay the shaft tiles over the map's collision and floor
// heights. The cage waits on the side the hero arrives on (`spawn`).
export function attach(w, got, spawn) {
  w.lifts = [];
  if (!w.map.lifts?.length) return;
  const { floorAt, climb, blocked } = w;
  const v = new THREE.Vector3();
  for (const def of w.map.lifts) {
    const { x, z, top } = def, bottom = def.bottom ?? floorAt(x, z);
    const obj = got[def.cage ?? "lift_cage"]?.scene.clone(true) ?? placeholderBox("lift_cage");
    prep(obj, true);
    obj.position.set(x + 0.5, bottom, z + 0.5);
    obj.rotation.y = THREE.MathUtils.degToRad(def.rot ?? 0);
    obj.updateMatrixWorld(true);
    const l = { def, x, z, bottom, top, head: def.head ?? top + 3.1, obj, gates: [], cables: [], y: bottom, at: "bottom" };
    obj.traverse((o) => {
      if (/^gate/.test(o.name)) {
        o.getWorldPosition(v).sub(obj.position);
        const dir = Math.abs(v.x) > Math.abs(v.z) ? (v.x > 0 ? "e" : "w") : v.z > 0 ? "s" : "n";
        l.gates.push({ o, dir, rest: o.rotation.y, sign: Math.sign(o.position.z) || 1, open: 0 });
      } else if (/^cable/.test(o.name)) l.cables.push(o);
    });
    const landings = (y) => l.gates.map((g) => g.dir).filter((f) => {
      const nx = x + DIR[f][0], nz = z + DIR[f][1];
      return !blocked(nx, nz) && Math.abs(floorAt(nx, nz) - (y + DECK)) <= CLIMB;
    });
    l.doors = { bottom: landings(bottom), top: landings(top) };
    const s = spawn ?? w.map.spawn;
    if (s && !(s.x === x && s.z === z) && floorAt(s.x, s.z) > (bottom + top) / 2) (l.at = "top"), (l.y = top);
    for (const g of l.gates) g.open = l.doors[l.at].includes(g.dir) ? 1 : 0;
    pose(l);
    w.root.add(obj);
    w.lifts.push(l);
  }
  const on = (x, z) => w.lifts.find((l) => l.x === x && l.z === z);
  w.floorAt = (x, z) => {
    const l = on(x, z);
    return l ? l.y + DECK : floorAt(x, z);
  };
  w.blocked = (x, z) => !on(x, z) && blocked(x, z);
  // Into or out of a cage only through a gate that's open, at the landing it stands at.
  w.climb = (ax, az, bx, bz) => {
    const a = on(ax, az), l = a ?? on(bx, bz);
    if (!l) return climb(ax, az, bx, bz);
    const [ox, oz] = a ? [bx, bz] : [ax, az];
    const f = Object.keys(DIR).find((d) => l.x + DIR[d][0] === ox && l.z + DIR[d][1] === oz);
    return !!l.at && l.doors[l.at].includes(f);
  };
}

// The lift at a tile, if any.
export const at = (w, x, z) => w.lifts?.find((l) => l.x === x && l.z === z) ?? null;

// What the lift offers the hero: a ride from inside, a call from a landing the cage isn't at.
export function facing(w, h) {
  if (!w.lifts?.length || w.ride) return null;
  const inside = at(w, h.x, h.z);
  if (inside) return { kind: "lift", text: inside.at === "top" ? tr("Ride the lift down", "Descendre par l'ascenseur") : tr("Ride the lift up", "Monter par l'ascenseur"), lift: inside };
  const [dx, dz] = DIR[h.face], l = at(w, h.x + dx, h.z + dz);
  if (!l) return null;
  const mine = LEVELS.find((k) => l.doors[k].includes(BACK[h.face]) && Math.abs(w.floorAt(h.x, h.z) - (l[k] + DECK)) <= CLIMB);
  return mine && mine !== l.at ? { kind: "lift", text: tr("Call the lift", "Appeler l'ascenseur"), lift: l, call: true } : null;
}

// Ride (or call) it. `io`: holds(cond), say(text), sound(name, n), fade(on), step(dir), reduced.
export function use(w, h, it, io) {
  const l = it.lift;
  if (!io.holds(l.def.if)) return io.say(l.def.locked ?? tr("The lift has no power.", "L'ascenseur n'a pas de courant."));
  const to = l.at === "top" ? "bottom" : "top";
  const travel = io.reduced ? 0.45 : Math.max(1.6, Math.abs(l[to] - l.y) * PER_M);
  w.ride = { l, from: l.y, to, travel, rider: it.call ? null : h, phase: "shut", t: 0, io };
  l.at = null;
  io.sound("winch", io.reduced ? 0 : travel);
}

// Every frame: gates swing, the cage moves through a ride's phases (shut, travel, settle, open),
// and at the end the rider steps off through the gate that opened.
export function update(w, dt) {
  const r = w.ride;
  if (r) {
    const { l, io } = r, target = l[r.to], reduced = io.reduced;
    r.t += dt;
    if (r.rider) r.rider.idle = 0; // no lean-in mid-ride
    if (r.phase === "shut" && (reduced || r.t >= SHUT)) {
      (r.phase = "travel"), (r.t = 0);
      if (reduced) io.fade(true);
    }
    if (r.phase === "travel") {
      const k = Math.min(1, r.t / r.travel);
      l.y = reduced ? (k < 1 ? r.from : target) : r.from + (target - r.from) * ease(k);
      if (k >= 1) {
        (r.phase = reduced ? "open" : "settle"), (r.t = 0);
        if (reduced) io.fade(false);
      }
    }
    if (r.phase === "settle") {
      const k = Math.min(1, r.t / SETTLE);
      l.y = target + 0.05 * Math.sign(target - r.from) * Math.sin(k * Math.PI * 2) * (1 - k) ** 2; // a little bob, settling
      if (k >= 1) (l.y = target), (r.phase = "open"), (r.t = 0);
    }
    if (r.phase === "open" && (reduced || r.t >= OPEN)) {
      l.at = r.to;
      w.ride = null;
      const out = r.rider && [r.rider.face, ...l.doors[r.to]].find((f) => l.doors[r.to].includes(f));
      if (out) io.step(out);
    }
  }
  for (const l of w.lifts ?? []) {
    const opening = w.ride?.l === l && w.ride.phase === "open" ? w.ride.to : l.at;
    for (const g of l.gates) {
      const want = opening && l.doors[opening].includes(g.dir) ? 1 : 0;
      g.open = r?.io.reduced ? want : g.open + Math.sign(want - g.open) * Math.min(Math.abs(want - g.open), dt / 0.3);
    }
    pose(l);
  }
}

function pose(l) {
  l.obj.position.y = l.y;
  for (const g of l.gates) g.o.rotation.y = g.rest + g.sign * SWING * ease(g.open);
  for (const c of l.cables) {
    let y = l.y;
    for (let o = c; o && o !== l.obj; o = o.parent) y += o.position.y; // its origin's height (no turned parents)
    c.scale.y = Math.max(0.01, l.head - y);
  }
}
