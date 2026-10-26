// Route duelists (WORLD_DESIGN §4.1): an NPC with a `trainer` block watches a
// straight line of tiles while it is unbeaten, there and not suppressed. When the
// hero stands on that line: "!" over its head, it walks over until it stands by the
// hero, and they talk (its `challenge` branch, which offers the duel). Turned down,
// or after the post-duel talk, it walks back to its post and stops watching until
// the hero leaves the map. world.js calls `watch` whenever the hero stands still,
// `update` every frame, `release` when a dialog closes and `meet` back from a duel.
import { DIR, BACK, emote, route } from "./actors.js";

export const SIGHT = 3; // tiles, when the block gives no `sight` (1..5; 0: talk to duel, never watches)
const BANG = 0.6; // seconds the "!" shows before the duelist sets off

// Duelists that stopped watching: per map, kept while the world is rebuilt around
// a duel (this module outlives a mount), forgotten when the hero enters another map.
const quiet = { map: "", ids: new Set() };
export function enterMap(name) {
  if (quiet.map !== name) (quiet.map = name), quiet.ids.clear();
}

// The tiles `n` watches: from its tile along `face`, up to `sight`, for as long as
// each step is one the hero could take (walkable, no prop, no person, at most a
// small climb: a duelist on a ledge can't see the floor below).
export function sightLine(w, n, face = n.face) {
  const [dx, dz] = DIR[face], line = [];
  let x = n.x, z = n.z;
  for (let i = 0; i < (n.def.trainer.sight ?? SIGHT); i++) {
    const nx = x + dx, nz = z + dz;
    if (w.blocked(nx, nz) || !w.climb(x, z, nx, nz) || w.npcAt(nx, nz, n)) break;
    line.push([nx, nz]);
    (x = nx), (z = nz);
  }
  return line;
}

// The duelist that sees the hero (the nearest, when two do), or null.
export function watch(w, hero, holds) {
  let best = null, near = Infinity;
  for (const n of w.npcs) {
    if (!n.def.trainer || n.hidden || n.busy || n.step || n.go?.length || quiet.ids.has(n.def.id)) continue;
    const i = sightLine(w, n).findIndex(([x, z]) => x === hero.x && z === hero.z);
    if (i >= 0 && i < near && holds(`!beat:${n.def.id}`)) (best = n), (near = i);
  }
  return best;
}

// Spotted: "!", and the hero turns to look. The walk starts once the "!" has shown.
export function spot(w, n, hero, sound) {
  const steps = Math.abs(hero.x - n.x) + Math.abs(hero.z - n.z) - 1;
  n.busy = true;
  w.engage = { n, post: { x: n.x, z: n.z, face: n.face }, wait: BANG, path: Array(steps).fill(n.face) };
  emote(n, "bang", BANG + 0.4);
  sound("spot");
  hero.face = BACK[n.face];
}

// While engaged: wait out the "!", walk over, then `talk(n)` once. A talk that
// opened no dialog after a moment (the world never paused) lets go, so input never stays locked.
export function update(w, dt, talk) {
  const e = w.engage;
  if (e?.talked && w.now - (e.at ??= w.now) > 1.5) return release(w);
  if (!e || e.talked || (e.wait -= dt) > 0) return;
  if (e.path) (e.n.go = e.path), (e.path = null);
  if (e.n.go.length || e.n.step) return;
  e.talked = true;
  talk(e.n);
}

// A dialog closed with the world still up (Not now, or after the post-duel line):
// the duelist stops watching and walks back to its post.
export function release(w) {
  const e = w.engage;
  if (!e) return;
  w.engage = null;
  quiet.ids.add(e.n.def.id);
  e.n.go = route(w, e.n, e.post.x, e.post.z);
  e.n.faceAfter = e.post.face;
}

// Talking to a duelist directly counts as meeting it: afterwards it goes quiet too.
export function engaged(w, n, face) {
  if (n.def.trainer) w.engage ??= { n, post: { x: n.x, z: n.z, face }, talked: true };
}

// Back from a duel against `id`: a duelist that walked over before the duel stands
// by the hero again (the world was rebuilt with it at its post), and so does anyone
// a cutscene brought to the hero (Rook); after the talk they walk back. Returns the NPC.
export function meet(w, hero, id) {
  const n = w.npcs.find((m) => m.def.id === id && !m.hidden);
  if (!n) return null;
  const [dx, dz] = DIR[hero.face], x = hero.x + dx, z = hero.z + dz;
  if ((n.x !== x || n.z !== z) && !w.blocked(x, z) && !w.npcAt(x, z)) {
    w.engage = { n, post: { x: n.x, z: n.z, face: n.face }, talked: true };
    Object.assign(n, { x, z, face: BACK[hero.face], step: null });
  }
  return n;
}
