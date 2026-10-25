// People on a map: placing them (`spawnNpcs`), walking their `route`
// (`updateNpc`), and what the hero faces and can talk to or use (`facing`).
import { actor, place, animateActor, angle, key, DIR } from "./actors.js";
import { ACROSS } from "./props.js";
import { tr } from "./map.js";

const NPC_STEP = 0.45; // seconds per tile

// NPCs: their own model when it exists, else the hero model, tinted and scaled.
export function spawnNpcs(map, got, floorAt, root) {
  return (map.npcs ?? []).map((def) => {
    const own = got[def.model];
    const n = own ? actor(own, null, 1, def.idle) : actor(got.hero, def.tint, def.scale ?? 1, def.idle);
    Object.assign(n, { def, x: def.x, z: def.z, fy: floorAt(def.x, def.z), face: def.facing ?? "s", step: null, ri: 0, wait: 1.5, busy: false });
    n.obj.rotation.y = angle(n.face);
    root.add(n.obj, n.blob);
    return n;
  });
}

// NPCs idle, or walk their `route` (one direction per step) when it is free.
export function updateNpc(w, n, dt, paused) {
  if (n.step) {
    n.step.t += dt / n.step.dur;
    if (n.step.t >= 1) {
      n.x = n.step.x;
      n.z = n.step.z;
      n.step = null;
    }
  }
  if (!n.step && n.def.route && !n.busy && !paused) {
    n.wait -= dt;
    if (n.wait <= 0) {
      const f = n.def.route[n.ri % n.def.route.length];
      const x = n.x + DIR[f][0], z = n.z + DIR[f][1];
      const h = w.hero;
      n.face = f;
      if (w.blocked(x, z) || !w.climb(n.x, n.z, x, z) || (h.x === x && h.z === z) || (h.step?.x === x && h.step?.z === z) || w.npcAt(x, z, n)) n.wait = 0.6;
      else {
        n.step = { x, z, t: 0, dur: NPC_STEP };
        n.ri++;
        n.wait = n.ri % n.def.route.length === 0 ? 2.5 : 0.35;
      }
    }
  }
  place(n, dt, w.floorAt);
  animateActor(n, dt, false);
}

// What the hero faces: a person (also across a counter or a duel table), a
// sign or table, or a door (a locked one reads like a sign). `opens(warp)`:
// whether a gated door's `if` holds.
export function facing(w, hero, opens) {
  const [dx, dz] = DIR[hero.face], fx = hero.x + dx, fz = hero.z + dz;
  const across = ACROSS.has(w.propAt.get(key(fx, fz))?.asset);
  const npc = w.npcAt(fx, fz) ?? (across ? w.npcAt(fx + dx, fz + dz) : null);
  if (npc) return { kind: "npc", text: npc.def.name, npc };
  const it = w.talk.get(key(fx, fz));
  if (it) return it;
  const wp = w.warps.get(key(fx, fz));
  if (wp && !wp.dir) return opens(wp) ? { kind: "warp", text: wp.label ?? tr("Go in", "Entrer"), warp: wp } : { kind: "locked", text: wp.locked ?? tr("It won't open.", "Ça ne s'ouvre pas.") };
  return null;
}
