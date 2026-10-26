// Sitting on benches and chairs (`SEAT` in props.js). Every tile of a seat's footprint is a place (a
// bench seats two), facing the way the prop faces, its back to the backrest. The hero sits from the
// tile in front of a place or beside it (never from behind), and that tile stays the hero's while it
// sits: nobody walks onto it, it is what the game saves ("at"), and standing up lands back on it, so
// the hero is never left inside the furniture and a reload stands it in front of the seat.
// A map's prop with `seat: false` is scenery; a person with `sit: true` on a place sits there.
// Sitting: turn round, then SitDown while backing onto the seat; Sit loops; StandUp undoes it.
// Reduced motion cuts straight to the end of each. Route duelists leave a sitting hero be (trainer.js).
import { DIR, key, angle } from "./actors.js";
import { FOOT, SEAT } from "./props.js";
import { tr } from "./map.js";

const FACE = { 0: "s", 90: "e", 180: "n", 270: "w" };
const DOWN = 14 / 24, UP = 14 / 24; // seconds: the SitDown and StandUp clips
const ease = (t) => t * t * (3 - 2 * t);

// The places on a map: tile key -> { x, z, ax, az (where the hips go), top (the seat), face }.
export function places(map, floorAt) {
  const out = new Map();
  for (const p of map.props) {
    const s = SEAT[p.asset];
    if (!s || p.seat === false || p.if) continue;
    let [fw, fd] = FOOT[p.asset] ?? [1, 1];
    if (p.rot % 180 === 90) [fw, fd] = [fd, fw];
    const face = FACE[(((p.rot ?? 0) % 360) + 360) % 360] ?? "s", [dx, dz] = DIR[face];
    for (let x = p.x; x < p.x + fw; x++)
      for (let z = p.z; z < p.z + fd; z++)
        out.set(key(x, z), { x, z, ax: x + 0.5 + dx * s[1], az: z + 0.5 + dz * s[1], top: (p.y ?? floorAt(x, z)) + s[0], face });
  }
  return out;
}

// What the hero can do here: stand up while sitting; sit on the free place it faces, from in front or beside.
export function facing(w, hero) {
  if (hero.sit) return { kind: "stand", text: tr("Stand up", "Se lever") };
  const [dx, dz] = DIR[hero.face], fx = hero.x + dx, fz = hero.z + dz;
  const pl = w.seats.get(key(fx, fz));
  if (!pl || hero.face === pl.face || w.npcAt(fx, fz) || !w.climb(hero.x, hero.z, fx, fz)) return null;
  return { kind: "seat", text: tr("Sit", "S'asseoir"), place: pl };
}

// Sit `a` (the hero, or a person placed sitting: `now`) on place `pl`.
export function sit(a, pl, reduced, now = false) {
  const cut = reduced || now;
  a.sit = { ax: pl.ax, az: pl.az, y: pl.top - a.lap, face: pl.face, k: cut ? 1 : 0, s: cut ? 1 : 0, want: 1 };
  a.face = pl.face;
  if (cut) a.obj.rotation.y = angle(pl.face);
  a.pose = cut ? "sit" : null;
}

export function stand(a) {
  if (a.sit) a.sit.want = 0;
}

// Turn first, then down onto the seat (or up off it); `a.pose` is the clip animateActor plays.
export function update(a, dt, reduced) {
  const s = a.sit;
  if (!s) return;
  const off = Math.abs(Math.atan2(Math.sin(angle(s.face) - a.obj.rotation.y), Math.cos(angle(s.face) - a.obj.rotation.y)));
  if (reduced) s.k = s.want;
  else if (s.want === 0 || off < 0.3) s.k = Math.min(1, Math.max(0, s.k + (s.want ? dt / DOWN : -dt / UP)));
  s.s = ease(s.k);
  if (s.want) a.pose = s.k === 0 ? null : s.k < 1 ? "sitdown" : "sit";
  else if (s.k > 0) a.pose = "standup";
  else (a.sit = null), (a.pose = null);
}

// A built map's places (`w.seats`), and the people it puts sitting on them.
export function attach(w) {
  w.seats = places(w.map, w.floorAt);
  for (const n of w.npcs) {
    const pl = n.def.sit && w.seats.get(key(n.x, n.z));
    if (pl) sit(n, pl, true, true);
  }
}
