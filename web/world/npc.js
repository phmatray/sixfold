// People on a map: placing them (`spawnNpcs`), walking their `route` or a path
// they were sent on (`go`), turning between `look` directions (`updateNpc`), and
// what the hero faces and can talk to or use (`facing`). An NPC with an `if` is
// only there while it holds (`hidden` otherwise, set by build.js `refresh`). One with a
// `routine` (routine.js) walks to each activity as the clock reaches it, and is `away`
// while it is home. Idle people glance at a hero close by, and say their `barks` in passing (#114).
import { actor, place, animateActor, occupy, angle, key, DIR } from "./actors.js";
import { ACROSS } from "./props.js";
import { tr } from "./map.js";
import { activityAt, plan, seedOf } from "./routine.js";
import { sit, stand, update as seatUpdate } from "./seat.js";

const NPC_STEP = 0.45; // seconds per tile
const GO_STEP = 0.3; // walking somewhere on purpose (a duelist coming over)
const ROUTE_PAUSE = 2.5; // seconds at the end of a route's loop (x the person's own `pause`)
const STUCK = 3; // seconds blocked on a routine walk before it re-plans
const REPLANS = 3; // re-plans before it is simply placed there
const WALKERS_MAX = 6; // people walking a routine at once, per map (`walkers_max`)
const LEAVE_AFTER = 2.5; // seconds after the hour turns before a person sets off (x its own `pause`)
const JUMP = 5; // game minutes: a clock that moves further in one frame was set (a rest, a save), not ticking
const CW = { n: "e", e: "s", s: "w", w: "n" }, CCW = { n: "w", w: "s", s: "e", e: "n" };
const GLANCE_NEAR = 2, GLANCE_EVERY = 4; // tiles; seconds before a person glances anew (#114)
const BARK_NEAR = 3, BARK_EVERY = 20; // tiles; seconds between two barks of one person
export const BARK_HOLD = 2.5; // seconds a bark stays over the head

// What a person wears: the map's `wear` list, else for a route duelist one of its
// model's accessories, picked by its id (so each looks its own on a shared model).
function wearOf(def, src) {
  if (def.wear) return def.wear;
  if (!def.trainer) return [];
  const names = [];
  src.scene.traverse((o) => /^acc_/.test(o.name) && names.push(o.name.slice(4)));
  let h = 0;
  for (const ch of def.id) h = (h * 31 + ch.charCodeAt(0)) >>> 0;
  return names.length ? [names[h % names.length]] : [];
}

// NPCs: their own model when it exists, else the hero model, tinted and scaled.
export function spawnNpcs(map, got, floorAt, root) {
  return (map.npcs ?? []).map((def) => {
    const own = got[def.model];
    const n = own ? actor(own, null, 1, def.idle, def.outfit, def.hair, wearOf(def, own)) : actor(got.hero, def.tint, def.scale ?? 1, def.idle);
    // Each person's own rhythm (routine.js seedOf): where its idle clip starts, how often it looks round.
    const seed = seedOf(def.id);
    Object.assign(n, { def, seed, x: def.x, z: def.z, fy: floorAt(def.x, def.z), face: def.facing ?? "s", step: null, ri: 0, wait: 1.5, busy: false, lookT: seed.look * (0.5 + seed.phase) });
    const idle = n.actions[n.idleName];
    if (idle) idle.time = seed.phase * idle.getClip().duration;
    n.obj.rotation.y = angle(n.face);
    root.add(n.obj, n.blob);
    return n;
  });
}

// The map as routine.js plans on it: open tiles, heights, and doors by where they lead. `avoid`: a tile to
// treat as solid (the one a walk was stuck on).
const gridOf = (w, avoid) => ({
  width: w.map.width, height: w.map.height, climb: w.climb,
  open: (x, z) => !w.blocked(x, z) && !(avoid && avoid[0] === x && avoid[1] === z),
  door: (to) => {
    for (const wp of w.warps.values()) if (wp.to === to && !wp.dir) return [wp.x, wp.z];
    return null;
  },
});

// The seat a `sit` activity's `at` names (seat.js places), if it is one: walked onto last, then sat on.
const seatOf = (w, act) => (act.do === "sit" && act.at && w.seats?.get(key(act.at[0], act.at[1]))) || null;

// Put `n` where its activity is, at once: on entering the map, when the clock was set, or when a walk can't get there.
function settle(w, n, p) {
  Object.assign(n, { activity: p.act, walk: null, go: [], step: null, sit: null, pose: null, due: undefined });
  const to = seatOf(w, p.act) ? p.act.at : p.to;
  if (to) [n.x, n.z] = to;
  arrive(w, n, true);
}

// At its activity: face its way, sit if it sits on a seat, or go in if it went home.
function arrive(w, n, now = false) {
  const act = n.activity, seat = seatOf(w, act);
  n.away = !!act.home;
  if (act.face) n.face = act.face;
  if (seat && n.x === act.at[0] && n.z === act.at[1]) sit(n, seat, w.reduced, now);
  if (now) n.obj.rotation.y = angle(n.face);
  n.lookT = n.seed.look * n.seed.pause;
}

// Walk the way `p` found (a seat is solid: the walk ends beside it, then steps onto it). `false` when `p` is
// already there.
function walkPlan(w, n, p, replans = 0) {
  const seat = seatOf(w, p.act), [sx, sz] = p.act.at ?? [];
  const onto = seat && Object.keys(DIR).find((f) => p.to[0] + DIR[f][0] === sx && p.to[1] + DIR[f][1] === sz);
  if (!p.steps.length && !onto) return false;
  n.go = onto ? [...p.steps, onto] : [...p.steps];
  n.walk = { to: onto ? [sx, sz] : p.to, seat: onto ? key(sx, sz) : null, t: 0, replans };
  return true;
}

// Leave for the activity in force, unless no walker is free or (coming out) its door is taken.
function leave(w, n, minute) {
  if (w.npcs.filter((m) => m.walk).length >= (w.map.walkers_max ?? WALKERS_MAX)) return;
  const h = w.hero;
  if (n.away && ((h.x === n.x && h.z === n.z) || (h.step?.x === n.x && h.step?.z === n.z) || w.npcAt(n.x, n.z, n))) return;
  if (n.sit) return stand(n); // up first (seat.js), then off
  const p = plan(n, minute, gridOf(w));
  n.away = false;
  n.due = undefined;
  if (!p.steps) return settle(w, n, p);
  n.activity = p.act;
  if (!walkPlan(w, n, p)) arrive(w, n);
}

// A routine: placed on entering (or when the clock is set), walking when the activity changes while you watch,
// each person after its own pause (seedOf), so a whole street doesn't set off on the same frame.
function routine(w, n, dt, minute) {
  const d = (((minute - (n.min ?? minute)) % 1440) + 1440) % 1440;
  n.min = minute;
  if (n.activity === undefined || d > JUMP) return settle(w, n, plan(n, minute, gridOf(w), true));
  if (n.walk && !n.step && !n.go?.length) {
    // The walk is over (or a scene took the person elsewhere): it is there.
    n.walk = null;
    return arrive(w, n);
  }
  if (activityAt(n.def.routine, minute) === n.activity || n.walk || n.busy || n.step || n.go?.length) return;
  if ((n.due ??= LEAVE_AFTER * n.seed.pause) > 0) n.due -= dt;
  else leave(w, n, minute);
}

// Stuck on a routine walk: after STUCK s, plan round the tile in the way; after REPLANS of those, just place it.
// Its own place taken (the hero stands on it): it waits there for it.
function unstick(w, n, dt, x, z, minute) {
  if ((n.walk.t += dt) < STUCK) return;
  n.walk.t = 0;
  const h = w.hero, [tx, tz] = n.walk.to;
  if ((h.x === tx && h.z === tz) || w.npcAt(tx, tz, n)) return;
  const replans = n.walk.replans + 1;
  const p = replans > REPLANS ? null : plan(n, minute, gridOf(w, [x, z]));
  if (p?.steps && walkPlan(w, n, p, replans)) return;
  settle(w, n, { act: n.activity, to: n.walk.to });
}

// NPCs idle, walk their `route` (one direction per step) when it is free, or keep their routine (above).
export function updateNpc(w, n, dt, paused, minute = 480) {
  if (n.hidden) {
    n.obj.visible = n.blob.visible = false;
    n.activity = undefined; // back in view: placed where its routine puts it
    if (n.walk) (n.walk = null), (n.go = []); // and its walker's place is free
    return;
  }
  if (n.def.routine && !paused) routine(w, n, dt, minute);
  n.obj.visible = n.blob.visible = !n.away;
  if (n.away) return;
  if (n.step) {
    n.step.t += dt / n.step.dur;
    if (n.step.t >= 1) {
      n.x = n.step.x;
      n.z = n.step.z;
      n.step = null;
    }
  }
  const h = w.hero;
  const heroSeat = h.sit && key(Math.floor(h.sit.ax), Math.floor(h.sit.az)); // a seat the hero sits on is taken
  const free = (x, z) => (!w.blocked(x, z) || (n.walk?.seat === key(x, z) && heroSeat !== key(x, z))) && w.climb(n.x, n.z, x, z) && !(h.x === x && h.z === z) && !(h.step?.x === x && h.step?.z === z) && !w.npcAt(x, z, n);
  if (!n.step && n.go?.length && !paused && !(n.walk && n.busy)) {
    // Sent somewhere: one step at a time, waiting while the way is taken (talked to on a routine walk: stops).
    const f = n.go[0], x = n.x + DIR[f][0], z = n.z + DIR[f][1];
    n.face = f;
    if (free(x, z)) {
      n.step = { x, z, t: 0, dur: n.walk ? NPC_STEP : GO_STEP };
      n.go.shift();
      if (n.walk) n.walk.t = 0;
    } else if (n.walk) unstick(w, n, dt, x, z, minute);
  } else if (!n.step && n.faceAfter && !n.go?.length) {
    n.face = n.faceAfter; // arrived: face the way it was sent to face
    n.faceAfter = null;
  }
  if (!n.step && !n.go?.length && n.def.route && !n.busy && !paused) {
    n.wait -= dt;
    if (n.wait <= 0) {
      const f = n.def.route[n.ri % n.def.route.length];
      const x = n.x + DIR[f][0], z = n.z + DIR[f][1];
      n.face = f;
      if (!free(x, z)) n.wait = 0.6;
      else {
        n.step = { x, z, t: 0, dur: NPC_STEP };
        n.ri++;
        n.wait = n.ri % n.def.route.length === 0 ? ROUTE_PAUSE * n.seed.pause : 0.35;
      }
    }
  }
  // A `look` list, or a routine's `look` activity (round about its facing); each at the person's own interval.
  const act = !n.walk && n.activity, f = act?.face ?? n.def.facing ?? "s";
  const look = n.def.trainer?.look ?? n.def.look ?? (act?.do === "look" ? [CCW[f], f, CW[f], f] : null);
  if (look?.length > 1 && !n.step && !n.go?.length && !n.busy && !paused && (n.lookT -= dt) <= 0) {
    n.lookT = n.seed.look;
    n.li = ((n.li ?? 0) + 1) % look.length; // a routine's list repeats its facing: counted, not looked up
    n.face = look[n.def.routine ? n.li : (look.indexOf(n.face) + 1) % look.length];
  }
  if (act?.with && !n.busy && !n.step) {
    // Talking with someone: face them (along the longer way).
    const o = w.npcs.find((m) => m.def.id === act.with && !m.hidden && !m.away);
    const dx = o ? o.x - n.x : 0, dz = o ? o.z - n.z : 0;
    if (dx || dz) n.face = Math.abs(dx) > Math.abs(dz) ? (dx > 0 ? "e" : "w") : dz > 0 ? "s" : "n";
  }
  if (n.sit) seatUpdate(n, dt, w.reduced);
  glance(w, n, paused);
  const face = n.face; // a glance only turns the body: its own facing (a routine's, a look's) is kept for after
  if (n.glance) n.face = n.glance;
  place(n, dt, w.floorAt);
  if (w.reduced && n.glance !== n.glanced) n.obj.rotation.y = angle(n.face); // reduced motion: the turn is a cut
  n.glanced = n.glance;
  n.face = face;
  occupy(n, null);
  animateActor(n, dt, w.reduced);
  occupy(n, n.busy ? null : act?.do, w.reduced, n.seed.phase);
}

// Glances (#114): an idle person (not walking, sitting, talking or a duelist, which watches its line) turns to the
// hero within GLANCE_NEAR tiles in front or beside it, at most every GLANCE_EVERY s; back to its own facing once the
// hero has gone. `glance: false` opts out.
const duels = (n) => (n.duels ??= !!n.def.trainer || JSON.stringify(n.def.dialogue ?? "").includes("duel:"));
function glance(w, n, paused) {
  const h = w.hero, dx = h.x - n.x, dz = h.z - n.z, [fx, fz] = DIR[n.face];
  const near = Math.abs(dx) + Math.abs(dz) <= GLANCE_NEAR && (dx || dz) && (n.glance || dx * fx + dz * fz >= 0); // not from behind
  if (!near || paused || n.busy || n.step || n.go?.length || n.walk || n.sit || n.def.route || n.def.glance === false || duels(n)) return void (n.glance = null);
  if (w.now < (n.glanceNext ?? 0)) return; // too soon: keeps its last glance (or none)
  n.glanceNext = w.now + GLANCE_EVERY;
  n.glance = Math.abs(dx) >= Math.abs(dz) ? (dx > 0 ? "e" : "w") : dz > 0 ? "s" : "n";
}

// Barks (#114): within BARK_NEAR tiles, the first of a person's `barks` whose `if` holds and that it hasn't said
// this session; then BARK_EVERY s before its next. world.js shows it and only asks when nothing else is going on.
const barked = new Set(); // "map:id:i", never saved: barks don't fill the save
export function barkOf(w, n, holds) {
  const h = w.hero;
  if (!n.def.barks || n.hidden || n.away || w.now < (n.barkNext ?? 0) || Math.abs(h.x - n.x) + Math.abs(h.z - n.z) > BARK_NEAR) return null;
  const i = n.def.barks.findIndex((b, i) => !barked.has(`${w.name}:${n.def.id}:${i}`) && holds(b.if));
  n.barkNext = w.now + (i < 0 ? 1 : BARK_EVERY); // none due: asked again in a second, not every frame
  if (i < 0) return null;
  barked.add(`${w.name}:${n.def.id}:${i}`);
  return n.def.barks[i].say;
}

// What the hero faces: a person (also across a counter or a duel table), a
// sign or table, a shut gate (its `text`), or a door (a locked one reads like a sign). `opens(warp)`:
// whether a gated door's `if` (and its hours, `open`) holds; one shut only by its hours reads "closed".
export function facing(w, hero, opens) {
  const [dx, dz] = DIR[hero.face], fx = hero.x + dx, fz = hero.z + dz;
  const across = ACROSS.has(w.propAt.get(key(fx, fz))?.asset);
  const npc = w.npcAt(fx, fz) ?? (across ? w.npcAt(fx + dx, fz + dz) : null);
  if (npc) return { kind: "npc", text: npc.def.name, npc };
  const it = w.talk.get(key(fx, fz));
  if (it) return it;
  const gate = w.gateAt.get(key(fx, fz));
  if (gate && !gate.open) return { kind: "gate", name: gate.def.name, cond: gate.def.if, text: gate.def.text ?? tr("There's no way through here yet.", "Pas de passage ici, pour l'instant.") };
  const wp = w.warps.get(key(fx, fz));
  if (wp && !wp.dir) return opens(wp) ? { kind: "warp", text: wp.label ?? tr("Go in", "Entrer"), warp: wp } : wp.open && opens({ if: wp.if }) // only its hours keep it shut: a shop after closing time (#114)
      ? { kind: "closed", text: wp.locked ?? tr("Closed. Come back in the day.", "Fermé. Revenez dans la journée.") }
      : { kind: "locked", cond: wp.if, text: wp.locked ?? tr("It won't open.", "Ça ne s'ouvre pas.") };
  return null;
}
