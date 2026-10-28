// The pure half of a map's ambient life (#115, its `life` and `calm`): where its critters and perch
// birds are at time `t`, and which micro-event is playing. No three.js here: life.js draws what this
// returns, and node --test (tools/life.test.mjs) and check-maps import it.

// Ground critters a map's `life.critters` may name, and when they are about.
export const CRITTERS = {
  rabbit: { when: "day" }, squirrel: { when: "day" }, duck: { when: "day" }, crab: { when: "day" },
  lizard: { when: "day" }, goat: { when: "day" }, hedgehog: { when: "night" }, glowbeetle: { when: "night" },
};
export const BUDGET = 12; // animated critters and birds in view, at most
export const GAP = 20; // s between two micro-events, at least
export const BOLT = 2.5; // tiles: a critter bolts when the hero comes this close
export const VIEW = 10; // tiles from the hero that count as in view
const LIFT = 3; // tiles: perch birds lift off together when the hero comes this close
const BACK = [20, 40]; // s before a bolted critter or a flown flock comes back
const HOP = 1.6; // tiles a critter covers in a second when it moves

const rnd = (a, b = 0) => {
  const s = Math.sin(a * 127.1 + b * 311.7) * 43758.5453;
  return s - Math.floor(s);
};
// A rect [x0, z0, x1, z1] in tiles, both ends included; x, z in world units (tile x covers [x, x + 1)).
const inRect = ([x0, z0, x1, z1], x, z) => x >= Math.min(x0, x1) && x < Math.max(x0, x1) + 1 && z >= Math.min(z0, z1) && z < Math.max(z0, z1) + 1;
export const calmAt = (map, x, z) => (map.calm ?? []).some((r) => inRect(r, x, z));
// Open ground off the paths, by the tiles alone (build.js passes a better one, that knows the props).
const openTile = (map, x, z) => /[.0-9]/.test(map.tiles[z]?.[x] ?? "#") && !"pr".includes(map.paint?.[z]?.[x] ?? "-");

// A boat on a `life.boats` cycle of `every` s: `k` is how far out along its lane it is (0 at the pier,
// 1 at the far end); it sails in, waits, sails out, and is out of sight for the rest of the cycle.
export function boatAt(every, t) {
  const f = (((t / every) % 1) + 1) % 1, ease = (u) => u * u * (3 - 2 * u);
  if (f < 0.25) return { k: 1 - ease(f / 0.25), on: true };
  if (f < 0.6) return { k: 0, on: true };
  if (f < 0.85) return { k: ease((f - 0.6) / 0.25), on: true };
  return { k: 1, on: false };
}

function setup(map, mem, t) {
  const open = mem.open ?? ((x, z) => openTile(map, x, z));
  mem.critters = (map.life.critters ?? []).flatMap((g, gi) => {
    if (!CRITTERS[g.kind]) return [];
    const [x0, z0, x1, z1] = g.zone, spots = [];
    for (let x = Math.max(0, Math.min(x0, x1)); x <= Math.min(map.width - 1, Math.max(x0, x1)); x++)
      for (let z = Math.max(0, Math.min(z0, z1)); z <= Math.min(map.height - 1, Math.max(z0, z1)); z++)
        if (open(x, z) && !calmAt(map, x + 0.5, z + 0.5)) spots.push([x + 0.5, z + 0.5]);
    if (!spots.length) return [];
    return Array.from({ length: g.n ?? 1 }, (_, i) => {
      const id = gi * 100 + i, home = spots[Math.floor(rnd(id, 1) * spots.length)];
      const near = spots.filter(([x, z]) => Math.hypot(x - home[0], z - home[1]) <= 3);
      return { id, kind: g.kind, near, period: 6 + rnd(id, 2) * 6, off: rnd(id, 3) * 30 };
    });
  });
  mem.perch = (map.life.perch ?? []).map((g, gi) => ({ id: gi, pts: (g.at ?? []).map(([x, z, y]) => [x + 0.5, z + 0.5, y ?? 0]).filter(([x, z]) => !calmAt(map, x, z)) })).filter((g) => g.pts.length);
  mem.open = open;
  mem.hid = {};
  mem.flown = {};
  mem.ev = null;
  mem.last = t - GAP + 10; // the first micro-event no sooner than 10 s in
  mem.wait = 0;
  mem.seq = 0;
}

// Where a critter is: it hops from spot to spot round its home, one hop a period, and sits between.
function critterAt(c, t) {
  const u = t + c.off, seg = Math.floor(u / c.period), f = u - seg * c.period;
  const a = c.near[Math.floor(rnd(c.id, seg - 1) * c.near.length)], b = c.near[Math.floor(rnd(c.id, seg) * c.near.length)];
  const d = Math.hypot(b[0] - a[0], b[1] - a[1]), k = d ? Math.min(1, (f * HOP) / d) : 1;
  return { x: a[0] + (b[0] - a[0]) * k, z: a[1] + (b[1] - a[1]) * k, face: d ? Math.atan2(b[0] - a[0], b[1] - a[1]) : rnd(c.id, seg) * 6.28, hop: k < 1 ? (f * 3) % 1 : 0 };
}

// What to draw at `t` (s, paused with the world) for a hero at world `hero.x`, `hero.z` (`hero.night`
// 0..1). `mem` is the map's memory between calls (who bolted, the last micro-event); start it `{}`, or
// with `open(x, z)` for open ground. Returns `spawns` (nearest first, at most BUDGET, never in a calm
// rect): { id, kind, x, z, y?, face, hop?, bolt?, fly? } — `bolt`/`fly` 0..1 while running off or
// lifting off — and `event`: null or { kind: "cross" | "flock" | "gust", t0, dur, … }.
export function lifeSchedule(map, t, hero, mem = {}) {
  if (!map.life) return { spawns: [], event: null };
  if (!mem.critters) setup(map, mem, t);
  const night = (hero.night ?? 0) > 0.5, dist = (x, z) => Math.hypot(x - hero.x, z - hero.z), all = [];

  for (const c of mem.critters) {
    if ((CRITTERS[c.kind].when === "night") !== night) continue;
    const p = critterAt(c, t), h = mem.hid[c.id];
    if (h) {
      if (t < h.t0 + 0.8) {
        const k = (t - h.t0) / 0.8;
        all.push({ id: c.id, kind: c.kind, x: h.x + h.dx * k * 1.5, z: h.z + h.dz * k * 1.5, face: Math.atan2(h.dx, h.dz), hop: (k * 3) % 1, bolt: k });
        continue;
      }
      if (t < h.back) continue;
      if (dist(p.x, p.z) < BOLT * 1.5) {
        h.back = t + 5; // the hero is still there: stay hidden a while longer
        continue;
      }
      delete mem.hid[c.id];
    }
    const d = dist(p.x, p.z);
    if (d < BOLT) {
      const away = { dx: (p.x - hero.x) / (d || 1), dz: (p.z - hero.z) / (d || 1) };
      mem.hid[c.id] = { t0: t, back: t + BACK[0] + rnd(c.id, Math.floor(t)) * (BACK[1] - BACK[0]), x: p.x, z: p.z, ...away };
      all.push({ id: c.id, kind: c.kind, x: p.x, z: p.z, face: Math.atan2(away.dx, away.dz), bolt: 0 });
      continue;
    }
    all.push({ id: c.id, kind: c.kind, ...p });
  }

  const lift = (g) => (mem.flown[g.id] = { t0: t, back: t + BACK[0] + rnd(g.id, Math.floor(t)) * (BACK[1] - BACK[0]), face: rnd(g.id, t) * 6.28 });
  for (const g of night ? [] : mem.perch) {
    const f = mem.flown[g.id];
    if (f) {
      if (t < f.t0 + 2.5) {
        g.pts.forEach(([x, z, y], i) => all.push({ id: `p${g.id}.${i}`, kind: "bird", x, z, y, face: f.face, fly: (t - f.t0) / 2.5 }));
        continue;
      }
      if (t < f.back) continue;
      if (g.pts.some(([x, z]) => dist(x, z) < LIFT * 1.5)) {
        f.back = t + 5;
        continue;
      }
      delete mem.flown[g.id];
    }
    if (g.pts.some(([x, z]) => dist(x, z) < LIFT)) lift(g);
    g.pts.forEach(([x, z, y], i) => all.push({ id: `p${g.id}.${i}`, kind: "bird", x, z, y, face: rnd(g.id * 10 + i, Math.floor(t / 4)) * 6.28, ...(mem.flown[g.id] ? { fly: 0 } : {}) }));
  }

  // Micro-events: one at a time, GAP s apart at least (and a random while more), never near a calm hero.
  if (mem.ev && t >= mem.ev.t0 + mem.ev.dur) mem.ev = null;
  if (!mem.ev && !calmAt(map, hero.x, hero.z) && t - mem.last >= GAP + mem.wait) {
    const s = mem.seq, kinds = [...new Set(mem.critters.map((c) => c.kind))].filter((k) => (CRITTERS[k].when === "night") === night);
    const pick = [];
    if (kinds.length)
      for (let i = 0; i < 4; i++) {
        // A critter darts across, three tiles from the hero, on open ground out of any calm.
        const a = (rnd(s, 10 + i) + i / 4) * 6.28, cx = hero.x + Math.cos(a) * 3, cz = hero.z + Math.sin(a) * 3;
        const ux = -Math.sin(a) * 2.5, uz = Math.cos(a) * 2.5;
        if (![[cx, cz], [cx - ux, cz - uz], [cx + ux, cz + uz]].every(([x, z]) => mem.open(Math.floor(x), Math.floor(z)) && !calmAt(map, x, z))) continue;
        pick.push({ kind: "cross", critter: kinds[Math.floor(rnd(s, 20) * kinds.length)], dur: 2.5, x0: cx - ux, z0: cz - uz, x1: cx + ux, z1: cz + uz });
        break;
      }
    const flock = night ? null : mem.perch.find((g) => !mem.flown[g.id] && g.pts.some(([x, z]) => dist(x, z) < VIEW * 0.8));
    if (flock) pick.push({ kind: "flock", dur: 2.5, group: flock.id });
    if (map.life.leaves || map.life.embers) pick.push({ kind: "gust", dur: 4 });
    if (pick.length) {
      mem.ev = { ...pick[Math.floor(rnd(s, 30) * pick.length)], t0: t };
      if (mem.ev.kind === "flock") lift(flock);
      mem.last = t;
      mem.wait = rnd(s, 40) * 15;
      mem.seq++;
    }
  }
  const ev = mem.ev;
  if (ev?.kind === "cross") {
    const k = (t - ev.t0) / ev.dur;
    all.push({ id: "cross", kind: ev.critter, x: ev.x0 + (ev.x1 - ev.x0) * k, z: ev.z0 + (ev.z1 - ev.z0) * k, face: Math.atan2(ev.x1 - ev.x0, ev.z1 - ev.z0), hop: (k * 8) % 1 });
  }

  const spawns = all.filter((s) => !calmAt(map, s.x, s.z) && dist(s.x, s.z) <= VIEW).sort((a, b) => dist(a.x, a.z) - dist(b.x, b.z)).slice(0, BUDGET);
  return { spawns, event: ev };
}
