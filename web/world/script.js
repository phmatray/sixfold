// Cutscenes (WORLD_DESIGN §8.3, §9.1): a map's `scripts` start here, on entering the map (`on: "enter"`) or
// on stepping onto one of its `at` tiles, while the script's `if` holds. The script goes to Rust
// (`cb("script", id, json)`), which runs its steps: it shows the `say` lines in the dialogue box, keeps
// the state effects for the end, and hands the cues here one at a time (`play`), waiting for
// `cb("cue_done", id)` before the next. `end(rest)` closes the scene: the cue under way and every cue
// in `rest` (what a skip jumps over) finish at once, so nobody is left mid-walk, and the camera goes
// back to the hero. While a scene plays the hero takes no input (world.js `busy`). Dialogue cues (no `id`)
// play here too, unanswered.
//
// Cues (engine `Cue`): walk (a path of n/e/s/w; an NPC at its walking pace, the hero at its own),
// face (n/e/s/w, or `hero`: toward where the hero stands), cam (eased by world.js's camera spring, a cut
// under reduced motion), wait, sfx, music, emote, warp and stone (the map's seal stone wakes until the scene
// ends; `seal` lights a keeper's face of it for good). A walk that can't go on (someone stands in the way) arrives anyway after a while.
import { DIR, angle, emote } from "./actors.js";

const EMOTE_HOLD = 600; // ms an emote holds the scene, so it reads
const CAM_MAX = 2.5; // s: a pan longer than this is cut short

// env: world(), hero(), reduced(), settled() (the camera reached its target), warp(wp) -> Promise,
// sound(name), tune() (the map's music back), stone(state, colour) (the seal stone's look, null at rest;
// "seal" lights that colour's face for good),
// moved() (the hero's tile is reported again), holds(cond), cb.
export function scenes(env) {
  let playing = false, cur = null, cam = null;
  const actorOf = (id) => (id === "hero" ? env.hero() : env.world()?.npcs.find((n) => n.def.id === id && !n.hidden));
  const isHero = (a) => a === env.hero();
  // A `face` cue's direction: `hero` turns toward the hero (along the longer axis).
  const toward = (a, f) => {
    if (f !== "hero") return f;
    const h = env.hero(), dx = h.x - a.x, dz = h.z - a.z;
    return Math.abs(dx) > Math.abs(dz) ? (dx > 0 ? "e" : "w") : dz > 0 ? "s" : "n";
  };

  function start(s) {
    const w = env.world();
    playing = true;
    cur = cam = null;
    w.path = [];
    // Each `say` carries its speaker's name (the hero's own lines have none).
    const steps = (s.steps ?? []).map((st) => (st.say ? { ...st, name: st.say === "hero" ? "" : (actorOf(st.say)?.def.name ?? "") } : st));
    env.cb("script", s.id ?? "", JSON.stringify({ steps }));
    return true;
  }
  const on = (s, x, z) => s.at?.some(([ax, az]) => ax === x && az === z);
  // The map's first `enter` script that holds (with `at`: only arriving there).
  const due = (w) => {
    const h = env.hero();
    return playing ? null : (w.map.scripts ?? []).find((s) => s.on === "enter" && (!s.at || on(s, h.x, h.z)) && env.holds(s.if));
  };
  // A map is up (the hero placed, no fade): that script starts.
  const entered = (w) => {
    const s = due(w);
    return !!s && start(s);
  };
  // The hero just stepped onto (x, z): a script with `at` tiles and no `on`.
  const stepped = (w, x, z) => !playing && (w.map.scripts ?? []).some((s) => !s.on && on(s, x, z) && env.holds(s.if) && start(s));

  // Where `a` ends up walking `path` from where it stands: each step taken while the way is open.
  function arrive(a, path) {
    const w = env.world();
    if (a.step) (a.x = a.step.x), (a.z = a.step.z), (a.step = null);
    for (const f of path) {
      const x = a.x + DIR[f][0], z = a.z + DIR[f][1], h = env.hero();
      a.face = f;
      if (w.blocked(x, z) || !w.climb(a.x, a.z, x, z) || w.npcAt(x, z, a) || (!isHero(a) && h.x === x && h.z === z)) break;
      (a.x = x), (a.z = z);
    }
    a.obj.rotation.y = angle(a.face);
    if (isHero(a)) w.path = [];
    else a.go = [];
  }

  // Play one cue; `cb("cue_done", id)` once it has finished (never for a cue a skip cut short).
  function play(c) {
    const id = c.id, me = { id, c, t: 0 };
    me.done = () => setTimeout(() => cur === me && ((cur = null), id !== undefined && env.cb("cue_done", String(id), "")));
    cur = me;
    const a = c.actor !== undefined ? actorOf(c.actor) : null;
    if (c.walk !== undefined) {
      if (!a) return me.done();
      me.a = a;
      me.limit = c.walk.length * 0.6 + 2;
      if (isHero(a)) env.world().path = [...c.walk];
      else a.go = [...c.walk];
    } else if (c.face !== undefined) {
      if (a) a.face = toward(a, c.face);
      me.done();
    } else if (c.cam !== undefined) {
      cam = c.cam === "hero" ? null : c.cam;
    } else if (c.wait !== undefined) setTimeout(me.done, c.wait);
    else if (c.sfx) env.sound(c.sfx), me.done();
    else if (c.music) globalThis.sixfoldAudio?.music(c.music), me.done();
    else if (c.stone) env.stone(c.stone, c.colour), me.done();
    else if (c.emote) {
      if (a) emote(a, c.emote);
      setTimeout(me.done, a ? EMOTE_HOLD : 0);
    } else if (c.warp) {
      const { map, x, z, facing } = c.warp;
      env.warp({ to: map, spawn: { x, z, facing }, travel: true }).finally(me.done);
    } else me.done();
  }

  // Every frame (not while paused): has the cue under way finished?
  function update(dt) {
    const me = cur;
    if (!me || me.finished) return;
    me.t += dt;
    if (me.c.walk !== undefined) {
      const a = me.a, moving = a.step || (isHero(a) ? env.world().path.length : a.go?.length);
      if (moving && me.t < me.limit) return;
      if (moving) arrive(a, isHero(a) ? env.world().path : a.go); // stuck: arrive anyway
    } else if (me.c.cam === undefined || (!env.reduced() && !env.settled() && me.t < CAM_MAX)) return;
    me.finished = true;
    me.done();
  }

  // The camera's look-at point while a `cam` cue holds it: [x, floor, z], or null for the hero.
  function aim() {
    if (!playing || !cam) return null;
    const w = env.world();
    if (Array.isArray(cam)) return [cam[0] + 0.5, w.floorAt(cam[0], cam[1]), cam[1] + 0.5];
    const a = actorOf(cam);
    return a ? [a.obj.position.x, a.fy, a.obj.position.z] : null;
  }

  // The scene is over (played out, or skipped): finish what's under way, jump over `rest`, unlock.
  function end(rest = []) {
    const me = cur;
    cur = cam = null;
    if (me?.c.walk !== undefined) arrive(me.a, isHero(me.a) ? env.world().path : me.a.go);
    for (const c of rest) {
      const a = c.actor !== undefined ? actorOf(c.actor) : null;
      if (c.walk !== undefined && a) arrive(a, c.walk);
      else if (c.face !== undefined && a) (a.face = toward(a, c.face)), (a.obj.rotation.y = angle(a.face));
      else if (c.warp) env.warp({ to: c.warp.map, spawn: c.warp, travel: true });
    }
    playing = false;
    env.stone(null);
    env.tune();
    env.moved();
  }

  return { get playing() { return playing; }, due, entered, stepped, play, update, aim, end };
}
