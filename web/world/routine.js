// NPC routines (WORLD_RULES §9): what a person does at each hour (`activityAt`), the way there (`plan`) and
// what keeps two people out of step (`seedOf`). Pure — no three.js, no DOM — so `node --test
// tools/routine.test.mjs` runs it; npc.js walks what `plan` returns.
const DIRS = { n: [0, -1], s: [0, 1], e: [1, 0], w: [-1, 0] };
const DAY = 1440;
export const PHASE_START = { dawn: 300, day: 480, dusk: 1080, night: 1260 }; // as daylight.js
export const DOES = ["idle", "work", "sit", "talk", "sweep", "read", "look"];

// A routine entry's `from` in game minutes: "HH:MM" or a phase name; NaN when it is neither.
export function minuteOf(from) {
  if (Object.hasOwn(PHASE_START, from ?? "")) return PHASE_START[from];
  const m = /^([01]\d|2[0-3]):([0-5]\d)$/.exec(from ?? "");
  return m ? +m[1] * 60 + +m[2] : NaN;
}

// The entry in force at `minute`: the last one started today, else (before the day's first) the day's last one,
// still running from last night.
export function activityAt(routine, minute) {
  const now = ((minute % DAY) + DAY) % DAY;
  let today = null, last = null;
  for (const a of routine) {
    const t = minuteOf(a.from);
    if (t <= now && !(minuteOf(today?.from) > t)) today = a;
    if (!(minuteOf(last?.from) > t)) last = a;
  }
  return today ?? last;
}

// Where `npc` ({ x, z, def: { routine } }) goes at `minute`: `{ act, to, steps }`, `steps` the 4-way path to
// the tile `to`. `steps: null` means place it on `to` at once: on `arrive` (the map was just entered, so it is
// already there) or when there is no way. A `home` act goes beside its door (`grid.door(name)`; the door
// itself is solid); `to: null` when there is no such door. `grid`: `{ width, height, open(x, z),
// climb?(x0, z0, x1, z1), door?(name) }`; other people are not in it (they block when walking, not here).
export function plan(npc, minute, grid, arrive = false) {
  const act = activityAt(npc.def.routine, minute);
  const at = act.home ? grid.door?.(act.home) : act.at;
  if (!at) return { act, to: null, steps: null };
  const goals = grid.open(at[0], at[1]) ? [at] : Object.values(DIRS).map(([dx, dz]) => [at[0] + dx, at[1] + dz]).filter(([x, z]) => grid.open(x, z));
  if (!goals.length) return { act, to: null, steps: null };
  if (arrive) return { act, to: goals[0], steps: null };
  const path = bfs(grid, npc.x, npc.z, goals);
  return path ? { act, ...path } : { act, to: goals[0], steps: null };
}

function bfs(grid, x0, z0, goals) {
  const key = (x, z) => z * grid.width + x;
  const goal = new Set(goals.map(([x, z]) => key(x, z)));
  const prev = new Map([[key(x0, z0), null]]);
  const queue = [[x0, z0]];
  for (let i = 0; i < queue.length; i++) {
    const [x, z] = queue[i];
    if (goal.has(key(x, z))) {
      const steps = [];
      for (let k = key(x, z); prev.get(k); k = prev.get(k).from) steps.unshift(prev.get(k).f);
      return { to: [x, z], steps };
    }
    for (const [f, [dx, dz]] of Object.entries(DIRS)) {
      const nx = x + dx, nz = z + dz, k = key(nx, nz);
      if (nx < 0 || nz < 0 || nx >= grid.width || nz >= grid.height || prev.has(k) || !grid.open(nx, nz) || (grid.climb && !grid.climb(x, z, nx, nz))) continue;
      prev.set(k, { from: key(x, z), f });
      queue.push([nx, nz]);
    }
  }
  return null;
}

// A person's own rhythm, from its id: where its idle clip starts (`phase`, 0..1), how often it turns its head
// (`look`, 2-4 s) and how long it pauses at each stop (`pause`, a factor of 0.6-1.4).
export function seedOf(id) {
  let h = 2166136261;
  for (const ch of id) h = Math.imul(h ^ ch.charCodeAt(0), 16777619) >>> 0; // FNV-1a
  const next = () => ((h = Math.imul(h ^ (h >>> 15), 2246822507) >>> 0), (h ^= h >>> 13), (h >>> 0) / 4294967296);
  return { phase: next(), look: 2 + 2 * next(), pause: 0.6 + 0.8 * next() };
}
