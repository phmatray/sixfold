// The overworld test: tile-grid maps (./maps/<name>.json) drawn with three.js.
// `mount(host, controls, cb, start, edit)` builds it (at `start` = "map,x,z,facing"
// when given; `edit`: a debug build with ?edit loads the map editor) and returns an
// api for Leptos (stop, act, pause, setItems, setClock, preview, previewStop, portrait, meet, cue, busy, read, page).
// `cb(event, a, b)` reports:
//   ("near", kind, text)   what the hero now faces ("" when nothing)
//   ("use", kind, text)    acted on a sign, the duel table, a locked door or a shut gate; a gated
//                          door or gate sends "kind cond" (e.g. "gate item:wading_boots"); a book
//                          on a shelf sends ("use", "book", id)
//   ("book", "closed", "") a book read (`read`) is back on its shelf
//   ("opened", text, "")   a gate on this map just opened ("A way opens: <its name>"), for the live region
//   ("at", map, "x,z,f")   the hero stands on a new tile or faces a new way
//   ("check", cond, "")    asks whether a condition (a gated warp's `if`) holds (returns a bool)
//   ("gate", cond, "wo")   asks whether a gate is open: w = was open, o = the hero is on it ("0"/"1")
//   ("clock", minute, "")  the game clock, every game hour and at each phase change
//   ("seen", map, "in"|"out") a map was entered (indoors or out)
//   ("talk", npcId, json)  spoke to an NPC (or a duelist walked up): {"name", "dialogue"} for Leptos to run
//   ("script", id, json)   a cutscene starts: {"steps"} for Leptos to run (script.js)
//   ("cue_done", id, "")   a cutscene cue sent with an `id` has finished
//   ("area", title, "")    entered an outdoor area that has a `title`
//   ("progress", "done/total", "") models loaded for the first map (the loading screen)
//   ("ready", "", "")      first frame drawn;  ("error", "", message)
// Game rules (progression, dialogue effects) live in the engine crate; the world
// only moves and draws. Grid: 1 tile = 1 m; tile (x, z) covers [x, x+1] × [z, z+1].
//
// This file is the entry (the one module the Rust side imports): mounting, the
// DOM controls, input, the hero's moves, the frame loop, the camera and the
// bridge (`cb` out, the returned api in). The rest:
//   map.js     fetch, `localize` (`*_fr`), model lists, idle prefetch of neighbours
//   build.js   a map's scene (`build`), prop animation, `refresh`, `dispose`
//   actors.js  hero and NPC rigs (`actor`, `place`, `animateActor`), grid helpers, `route`
//   npc.js     placing NPCs, their routes, `facing` (what the hero can talk to or use)
//   trainer.js route duelists: line of sight, the walk over, going quiet
//   lift.js    cages that carry the hero between two heights (the Rodspire)
//   script.js  cutscenes: a map's `scripts` start there, and it plays their cues one at a time
//   daylight.js the light at each minute of the day: sun, moon, sky, fog, lamps, glow
//   book.js    a book taken from a shelf: its flight, its pages, the hero's reach
//   ground.js, life.js, props.js  ground layers, ambient life and wind, shared building blocks
import * as THREE from "three";
import { clone as cloneSkinned } from "three/addons/utils/SkeletonUtils.js";
import { U } from "./life.js";
import { PLACEHOLDER, group, prep, CUTAWAY } from "./props.js";
import { tr, loadMap, loadAsset, assetNames, preload, PERSON, OPTIONAL } from "./map.js";
import { build, dispose, refresh, animateProps } from "./build.js";
import { DIR, BACK, angle, key, FIDGET_AFTER, actor, place, animateActor, route } from "./actors.js";
import { updateNpc, facing as facingOf } from "./npc.js";
import * as duelists from "./trainer.js";
import * as lifts from "./lift.js";
import { daylight, phaseOf, DAY } from "./daylight.js";
import { scenes } from "./script.js";
import { createBook } from "./book.js";

const STEP = 0.25, RUN_STEP = 0.17; // seconds per tile: walk 4 tiles/s, run ~6 (controllable on a phone)
const TURN_HOLD = 0.12; // a press shorter than this only turns the hero
const FADE = 350; // ms, warp fade
const KEYS = { w: "n", z: "n", arrowup: "n", s: "s", arrowdown: "s", a: "w", q: "w", arrowleft: "w", d: "e", arrowright: "e" }; // WASD and AZERTY ZQSD
const CLOCK_RATE = 1; // game minutes per real second: a day is 24 minutes (WORLD_DESIGN §7.2)
const PHASE_FADE = 20; // daylight.js cross-fades this many game minutes into a phase

// ---------- Mount ----------

// Sounds come from web/audio/audio.js (loaded with the wasm), when it is there.
const sound = (name, n = 0) => globalThis.sixfoldAudio?.sfx(name, n);

export function mount(el, controls, cb, start, edit = false) {
  const mq = matchMedia("(prefers-reduced-motion: reduce)");
  let reduced = mq.matches;
  const onMotion = (e) => ((reduced = e.matches), (U.uSway.value = reduced ? 0 : 1));
  mq.addEventListener("change", onMotion);
  U.uSway.value = reduced ? 0 : 1;

  const renderer = new THREE.WebGLRenderer({ antialias: true });
  renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = THREE.PCFSoftShadowMap;
  renderer.shadowMap.autoUpdate = false; // updated once per frame, before the three passes
  renderer.autoClear = false;
  renderer.info.autoReset = false; // counted over the whole frame (shadows and the three passes)
  renderer.domElement.setAttribute("aria-hidden", "true");
  el.appendChild(renderer.domElement);

  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(40, 1, 0.5, 80); // tight planes: depth precision
  camera.layers.enable(1); // the hero (layer 1: left out of the sun's shadow pass)
  // three's shadow pass draws only what the rendering camera's layers hold, so the
  // shadows are drawn first through a camera that sees nothing itself and holds every
  // layer (the terrain's shadow-only caster, layer 2) but the hero's.
  const shadowEye = new THREE.OrthographicCamera(-0.01, 0.01, 0.01, -0.01, 0.1, 0.2);
  shadowEye.position.set(0, -1e4, 0);
  shadowEye.updateMatrixWorld();
  shadowEye.layers.enableAll();
  shadowEye.layers.disable(1);
  let cam = { fov: 40, fovPortrait: 50, pitch: 50, distance: 12, distancePortrait: 15, lookAtOffset: [0, 0.75, 0] };
  let dist = 12, alive = true, raf = 0, paused = false, warping = false, world = null, hero = null;
  let canRun = false, runToggle = false, shift = false, lastHero = "";
  // The clock (the engine owns the saved minute; Rust hands it over with setClock).
  let minute = 480, lastHour = -1, lastPhase = "", bgAt = 0;
  const clock = new THREE.Clock();
  const look = new THREE.Vector3(), lookVel = new THREE.Vector3(), aim = new THREE.Vector3();
  let first = true, zoom = 1, readySent = false, drawnPaused = false, cover = 0;
  const resize = () => {
    const w = el.clientWidth || 1, h = el.clientHeight || 1;
    // The share of the view under the top bar: a room's back row must clear it.
    const bar = document.querySelector(".world-top");
    cover = bar ? THREE.MathUtils.clamp((bar.getBoundingClientRect().bottom - el.getBoundingClientRect().top) / h, 0, 0.4) : 0;
    renderer.setSize(w, h);
    camera.aspect = w / h;
    camera.fov = w < h ? cam.fovPortrait : cam.fov; // portrait phones: pull back and widen
    dist = w < h ? cam.distancePortrait : cam.distance;
    camera.updateProjectionMatrix();
    book.resize(w, h);
  };
  const book = createBook({ reduced: () => reduced, sound });
  let shelf = null; // the book spot last used: where a book read from the world comes from
  const ro = new ResizeObserver(resize);
  ro.observe(el);

  // ---------- DOM: D-pad and RUN (in `controls`, after the header), fade, grid legend ----------
  const arrow = '<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path d="M12 5l8 11H4z" fill="currentColor"/></svg>';
  const pad = document.createElement("div");
  pad.className = "world-pad";
  pad.setAttribute("role", "group");
  pad.setAttribute("aria-label", tr("Walk", "Marcher"));
  for (const [f, label] of [["n", tr("Up", "Haut")], ["w", tr("Left", "Gauche")], ["e", tr("Right", "Droite")], ["s", tr("Down", "Bas")]]) {
    const b = document.createElement("button");
    b.type = "button";
    b.innerHTML = arrow;
    b.className = `pad-${f}`;
    b.setAttribute("aria-label", label);
    b.dataset.dir = f;
    pad.appendChild(b);
  }
  const run = document.createElement("button");
  run.type = "button";
  run.className = "world-run";
  run.textContent = tr("Run", "Courir");
  run.hidden = true;
  run.setAttribute("aria-pressed", "false");
  run.addEventListener("click", () => {
    runToggle = !runToggle;
    run.setAttribute("aria-pressed", String(runToggle));
  });
  controls.append(pad, run);
  const fade = document.createElement("div");
  fade.className = "world-fade";
  const legend = document.createElement("div");
  legend.className = "world-legend";
  legend.hidden = true;
  legend.innerHTML = tr('<span class="k-block">Blocked</span><span class="k-act">Look or use</span><span class="k-warp">Door</span><span class="k-npc">Person</span>',
    '<span class="k-block">Bloqué</span><span class="k-act">Regarder ou utiliser</span><span class="k-warp">Porte</span><span class="k-npc">Personne</span>');
  // The area's name, shown for a moment on the way in; also a gate that just opened.
  const banner = document.createElement("div");
  banner.setAttribute("aria-hidden", "true");
  Object.assign(banner.style, {
    position: "absolute", zIndex: 2, left: "50%", top: "22%", transform: "translateX(-50%)", pointerEvents: "none",
    padding: "8px 18px", borderRadius: "999px", background: "rgba(13, 26, 44, 0.72)", color: "#f6ecd2",
    font: "700 clamp(16px, 2.4vw, 22px) 'Bricolage Grotesque', system-ui, sans-serif", letterSpacing: "0.02em",
    width: "max-content", maxWidth: "calc(100% - 32px)", textAlign: "center", opacity: 0, transition: "opacity 0.5s ease",
  });
  let bannerTimer = 0;
  const showBanner = (text, ms = 2200) => {
    banner.textContent = text;
    banner.style.opacity = 1;
    clearTimeout(bannerTimer);
    bannerTimer = setTimeout(() => (banner.style.opacity = 0), ms);
  };
  el.append(fade, legend, banner);

  // ---------- Input ----------
  const held = []; // pressed inputs, newest last: key names or "pad-<dir>"
  const dirOf = (k) => KEYS[k] ?? k.slice(4);
  const press = (k) => {
    const i = held.indexOf(k);
    if (i >= 0) held.splice(i, 1);
    held.push(k);
    if (world) world.path = [];
  };
  const release = (k) => {
    const i = held.indexOf(k);
    if (i >= 0) held.splice(i, 1);
  };
  const busy = () => paused || warping || !world || !!world.engage || !!world.ride || cut.playing || book.live(); // a duelist coming over, a lift ride, a cutscene, a book lock input
  const onKey = (e) => {
    const k = e.key.toLowerCase();
    if (e.type === "keyup") {
      if (k === "shift") shift = false;
      return release(k);
    }
    if (k === "shift") return void (shift = true);
    if (e.target.closest?.("input, select, textarea, dialog")) return;
    // The arrows stay the world's while it is busy too (a warp, a lift): a pad
    // (web/gamepad.js) moves focus with the arrows nothing takes.
    if (busy()) return void (KEYS[k] && e.preventDefault());
    // Enter and Space belong to whatever button has focus; only on the stage do
    // they act. E is the dedicated action key.
    const onStage = e.target === document.body || !!e.target.closest?.(".world-stage");
    if (KEYS[k]) {
      e.preventDefault();
      if (!e.repeat) press(k);
    } else if ((k === "e" || ((k === "enter" || k === " ") && onStage)) && !e.repeat) {
      e.preventDefault();
      act();
    } else if (k === "g" && !e.repeat) {
      world.grid.visible = !world.grid.visible;
      legend.hidden = !world.grid.visible;
    }
  };
  const clearHeld = () => ((held.length = 0), (shift = false));
  const padDir = (e) => e.target.closest?.("[data-dir]")?.dataset.dir;
  const padDown = (e) => {
    const f = padDir(e);
    if (!f || busy()) return;
    e.preventDefault();
    press(`pad-${f}`);
    try {
      e.target.setPointerCapture(e.pointerId); // keep the release even if the finger slides off
    } catch {}
  };
  const padUp = (e) => {
    const f = padDir(e);
    if (f) release(`pad-${f}`);
  };
  // Keyboard or switch activation of a pad button (a click with no pointer): one step.
  const padClick = (e) => {
    const f = padDir(e);
    if (!f || e.detail !== 0 || busy() || hero.step) return;
    tryStep(f, 0);
  };
  addEventListener("keydown", onKey);
  addEventListener("keyup", onKey);
  addEventListener("blur", clearHeld);
  pad.addEventListener("pointerdown", padDown);
  for (const t of ["pointerup", "pointercancel", "lostpointercapture"]) pad.addEventListener(t, padUp);
  pad.addEventListener("click", padClick);
  pad.addEventListener("contextmenu", (e) => e.preventDefault());

  // Tap/click the ground: walk the shortest grid path there (BFS). Tap a person,
  // a sign or a table: walk up to it, face it, then talk or use it (like E).
  const ray = new THREE.Raycaster(), ndc = new THREE.Vector2(), hit = new THREE.Vector3();
  const floor = new THREE.Plane(new THREE.Vector3(0, 1, 0), 0);
  const box = new THREE.Box3();
  // The tile under the ray: a person or something to use first (a standing box
  // per tile, so tapping a head or a signboard counts), else the floor.
  const tapped = () => {
    const w = world;
    let best = null, bestD = Infinity;
    const test = (x, z, cx, cz, top) => {
      const y = w.floorAt(x, z);
      box.min.set(cx - 0.5, y, cz - 0.5);
      box.max.set(cx + 0.5, y + top, cz + 0.5);
      if (ray.ray.intersectBox(box, hit) && hit.distanceTo(ray.ray.origin) < bestD) (bestD = hit.distanceTo(ray.ray.origin)), (best = [x, z]);
    };
    for (const n of w.npcs) test(n.x, n.z, n.obj.position.x, n.obj.position.z, 1.7);
    for (const i of w.talk.values()) test(i.x, i.z, i.x + 0.5, i.z + 0.5, 1.1);
    for (const wp of w.warps.values()) if (!wp.dir) test(wp.x, wp.z, wp.x + 0.5, wp.z + 0.5, 2.2); // a door: walking into it enters
    for (const l of w.lifts) test(l.x, l.z, l.x + 0.5, l.z + 0.5, 2.4); // a lift's cage: walk in and ride
    if (best) return best;
    // Raised ground first: the highest level whose tile under the ray stands at that height.
    for (const y of w.levels) {
      floor.constant = -y;
      if (!ray.ray.intersectPlane(floor, hit)) continue;
      const tx = Math.floor(hit.x), tz = Math.floor(hit.z);
      if (y && Math.abs(w.floorAt(tx, tz) - y) > 0.3) continue;
      return [tx, tz];
    }
    return null;
  };
  // The floor tile under a screen point (the editor's pointer).
  const tileAt = (clientX, clientY) => {
    const b = renderer.domElement.getBoundingClientRect();
    ndc.set(((clientX - b.left) / b.width) * 2 - 1, -((clientY - b.top) / b.height) * 2 + 1);
    ray.setFromCamera(ndc, camera);
    for (const y of world.levels) {
      floor.constant = -y;
      if (!ray.ray.intersectPlane(floor, hit)) continue;
      const tx = Math.floor(hit.x), tz = Math.floor(hit.z);
      if (!y || Math.abs(world.floorAt(tx, tz) - y) <= 0.3) return [tx, tz];
    }
    return null;
  };
  let stopEditor = () => {}, editing = () => false; // the map editor (?edit), when loaded
  const onTap = (e) => {
    if (busy() || e.button) return;
    const b = renderer.domElement.getBoundingClientRect();
    ndc.set(((e.clientX - b.left) / b.width) * 2 - 1, -((e.clientY - b.top) / b.height) * 2 + 1);
    ray.setFromCamera(ndc, camera);
    const t = tapped();
    if (!t) return;
    world.path = walkTo(t[0], t[1], world.npcAt(...t));
  };
  // Ends facing it (route's last move turns into the solid tile); "act" then uses it.
  // `path.npc`: a person who may walk off, so the path is re-planned at every step.
  const walkTo = (tx, tz, npc) => {
    const path = route(world, hero, tx, tz), lift = lifts.at(world, tx, tz);
    if (lift && tx === hero.x && tz === hero.z) path.push("act"); // already in the cage: ride
    else if (path.length && (npc || lift || world.talk.has(key(tx, tz)) || world.gateAt.get(key(tx, tz))?.open === false)) path.push("act");
    path.npc = npc;
    return path;
  };
  renderer.domElement.addEventListener("pointerdown", onTap);
  renderer.domElement.addEventListener("contextmenu", (e) => e.preventDefault()); // long-press on the scene

  // ---------- Moving and acting ----------

  // A condition (a gated warp's `if`, e.g. "seals:6"; a glint) holds when Rust says so.
  const holds = (cond) => !cond || cb("check", cond, "") === true;
  // A gate's state is the engine's rule too (it never shuts under the hero).
  const gate = (cond, was, on) => cb("gate", cond, `${was ? 1 : 0}${on ? 1 : 0}`) === true;
  const recheck = (animate) => {
    refresh(world, holds, gate, animate);
    if (world.gateSound !== undefined) sound(world.gateSound ? "gate" : "door"), (world.gateSound = undefined);
    // The gates that just opened, named on screen and in the live region (once: they opened now);
    // two at once (both vents at nightfall) share one banner rather than the second hiding the first.
    const names = [...new Set(world.opened?.splice(0))];
    if (names.length) {
      const list = names.join(tr(" and ", " et "));
      const text = names.length > 1 ? tr(`Ways open: ${list}`, `Des passages s'ouvrent : ${list}`) : tr(`A way opens: ${list}`, `Un passage s'ouvre : ${list}`);
      showBanner(text, 4000), cb("opened", text, "");
    }
  };
  // The music for the map and the hour: a region's track, its night variant after dark;
  // under it the region's ambience bed (a map may name another with `ambience`).
  const tune = () => {
    const m = world.map, phase = phaseOf(minute);
    const track = m.light.indoor ? "interior" : m.dark ? "cave" : m.music ?? "overworld";
    globalThis.sixfoldAudio?.music(!m.light.indoor && !m.dark && phase === "night" ? `${track}_night` : track);
    globalThis.sixfoldAudio?.ambience(m.ambience ?? track, phase);
  };
  // Tell Rust the time (it saves it); at a new phase, what is out at this hour changes.
  const tick = (dt) => {
    minute = (minute + dt * CLOCK_RATE) % DAY;
    const hour = Math.floor(minute / 60), phase = phaseOf(minute);
    if (phase !== lastPhase || hour !== lastHour) cb("clock", String(Math.floor(minute)), "");
    if (phase !== lastPhase && lastPhase) recheck(true), cut.playing || tune(); // a scene's `music:` holds until it ends
    lastHour = hour;
    lastPhase = phase;
  };
  // Cutscenes (script.js): the camera reports when it has reached what it was sent to look at.
  const cut = scenes({
    world: () => world, hero: () => hero, reduced: () => reduced, holds, cb, sound, tune, warp,
    settled: () => look.distanceTo(aim) < 0.12 && lookVel.lengthSq() < 0.04,
    moved: () => (lastHero = ""), // the hero's tile is saved again (not during a scene: a reload replays it)
  });
  const opens = (wp) => holds(wp.if);
  const facing = () => lifts.facing(world, hero) ?? facingOf(world, hero, opens);

  // Start a step toward `f`. A door warp is entered even though its tile is solid;
  // pushing against the wall from a warp tile with that `dir` leaves the room.
  function tryStep(f, carry) {
    const w = world, x = hero.x + DIR[f][0], z = hero.z + DIR[f][1];
    hero.face = f;
    const free = !cut.playing; // a cutscene's walk never leaves the map (a `warp:` cue does), so skipping lands the same
    const door = free && w.warps.get(key(x, z));
    if (door && !door.dir && opens(door)) return startStep(x, z, carry, door);
    const exit = free && w.exitAt(x, z);
    if (exit) {
      // Off the edge into the next area: take the step while the fade comes up.
      warp({ to: exit.to, edge: true, spawn: arrival(w, exit, f) });
      return startStep(x, z, carry, null);
    }
    if (w.blocked(x, z) || w.npcAt(x, z) || !w.climb(hero.x, hero.z, x, z)) {
      const here = free && w.warps.get(key(hero.x, hero.z));
      if (here?.dir === f) warp(here);
      return false;
    }
    return startStep(x, z, carry, null);
  }
  function startStep(x, z, carry, door) {
    const stoop = door?.stoop; // a house door: only as far as its door, up its steps (props.js STOOP)
    const run = canRun && (shift || runToggle) && !cut.playing; // a scene walks
    const dur = (run ? RUN_STEP : STEP) * (stoop ? 0.5 + stoop.in : 1);
    hero.step = { x, z, t: carry / dur, dur, run, warp: door, stoop };
    return true;
  }

  function act() {
    if (busy() || hero.step) return;
    const it = facing();
    if (!it) return;
    held.length = 0;
    if (it.kind === "npc") talkTo(it.npc);
    else if (it.kind === "warp") tryStep(hero.face, 0);
    else if (it.kind === "lift") {
      world.path = [];
      lifts.use(world, hero, it, { holds, reduced, sound, say: (t) => cb("use", "locked", t), fade: (on) => fade.classList.toggle("on", on), step: (f) => tryStep(f, 0) });
    }
    else if (it.kind === "book") {
      shelf = it;
      cb("use", "book", it.id);
    } else {
      if (it.obj && !reduced) it.obj.userData.wobble = 1;
      // A spot with its own `dialogue` (a lost item, a letter box) talks like a person.
      if (it.dialogue) cb("talk", it.id, JSON.stringify({ name: it.name ?? "", dialogue: it.dialogue }));
      else cb("use", it.cond ? `${it.kind} ${it.cond}` : it.kind, it.text);
    }
  }

  // Turn a person to the hero, then speak (Leptos runs the dialogue).
  function talkTo(n) {
    duelists.engaged(world, n, n.face);
    n.busy = true;
    n.face = BACK[hero.face];
    setTimeout(() => alive && cb("talk", n.def.id, JSON.stringify({ name: n.def.name, dialogue: n.def.dialogue })), reduced ? 0 : 160);
  }

  // Where the hero comes in after walking off `w` through `exit` (heading `f`): on the
  // matching edge of the next map, at the same offset along the exit that leads back.
  function arrival(w, exit, f) {
    const along = "ns".includes(exit.side) ? hero.x : hero.z;
    return (map) => {
      const back = (map.exits ?? []).find((e) => e.side === BACK[exit.side] && e.to === w.name);
      if (!back) return map.spawn;
      const c = Math.min(back.span[1], back.span[0] + along - exit.span[0]);
      const { width: W, height: H } = map;
      const xz = { n: [c, H - 1], s: [c, 0], e: [0, c], w: [W - 1, c] }[exit.side];
      return { x: xz[0], z: xz[1], facing: f };
    };
  }

  async function warp(wp) {
    if (warping) return;
    warping = true;
    if (!wp.edge) sound(wp.travel ? "travel" : wp.if ? "gate" : "door"); // a gated door opening sounds grander
    held.length = 0;
    world.path = [];
    fade.classList.add("on");
    await new Promise((r) => setTimeout(r, reduced ? 0 : FADE));
    try {
      await enter(wp.to, wp.spawn);
    } catch (e) {
      fail(e);
    }
    fade.classList.remove("on");
    warping = false;
  }

  // Load a map (and its models), build it, and put the hero on `spawn` (or on
  // `spawn(map)` for an edge exit). Only this map is built; its neighbours are
  // fetched lazily once it is up, so the next walk off the edge is quick.
  async function enter(name, spawn) {
    const map = await loadMap(name);
    if (typeof spawn === "function") spawn = spawn(map);
    const names = assetNames(map);
    // The loading screen counts the first map's models in.
    let count = 0;
    const counted = (n) => (readySent ? loadAsset(n) : loadAsset(n).finally(() => cb("progress", `${++count}/${names.length}`, "")));
    if (!readySent) cb("progress", `0/${names.length}`, "");
    const loaded = await Promise.all(names.map(counted));
    if (!alive) return;
    const got = Object.fromEntries(names.map((n, i) => [n, loaded[i]]));
    const missing = names.filter((n) => !got[n] && !OPTIONAL.test(n) && !PERSON.test(n)); // NPCs fall back to a tinted hero
    if (missing.length) console.info(`world (${name}): placeholders for ${missing.join(", ")}`);
    if (!hero) {
      hero = actor(got.hero, null, 1, "idle");
      hero.obj.traverse((o) => o.layers.set(1)); // casts no sun shadow (its blob is its shadow; see shadowEye)
      scene.add(hero.obj, hero.blob);
    }
    const next = await build(map, got, reduced);
    if (!alive) return;
    lifts.attach(next, got, spawn);
    next.name = name;
    duelists.enterMap(name);
    // Gates first see the hero where it arrives (a gate it stands on lets it stay).
    if (spawn && Number.isInteger(spawn.x)) next.spawnAt = spawn;
    refresh(next, holds, gate);
    if (world) dispose(world.root);
    world = next;
    world.hero = null;
    world.day = daylight(world, reduced);
    scene.add(world.root);
    const L = map.light;
    scene.fog = L.indoor ? null : new THREE.Fog(L.background, L.fog?.near ?? 60, L.fog?.far ?? 90);
    lastPhase = phaseOf(minute);
    world.scriptsDue = true; // an `enter` cutscene starts once the map is on screen
    cb("clock", String(Math.floor(minute)), "");
    cb("seen", name, L.indoor ? "in" : "out");
    tune();
    const s = spawn && !next.blocked(spawn.x, spawn.z) && !next.npcAt(spawn.x, spawn.z) ? spawn : map.spawn; // a stale saved tile falls back
    Object.assign(hero, { x: s.x, z: s.z, face: s.facing, step: null, idle: 0, turnedAt: -Infinity }); // a fresh map clock
    hero.obj.rotation.y = angle(s.facing);
    world.hero = hero;
    sky(world.day.update(minute, s.x + 0.5, s.z + 0.5, 0), true);
    cam = map.camera;
    resize();
    first = true;
    await renderer.compileAsync(scene, camera).catch(() => {});
    if (map.title && !L.indoor) showBanner(map.title), cb("area", map.title, "");
    preload(map, () => alive);
  }

  // The sky behind the scene and the fog: one colour (fog fades into the sky).
  function sky(look, now = false) {
    renderer.setClearColor(look.color ?? look.bg); // `color`: the sky, lit by a lightning flash
    if (scene.fog) scene.fog.color.copy(look.color ?? look.bg), (scene.fog.near = look.near), (scene.fog.far = look.far);
    if (now || world.now - bgAt > 1) (bgAt = world.now), el.closest(".world")?.style.setProperty("--world-bg", `#${look.bg.getHexString()}`);
  }

  function fail(e) {
    console.error("world: could not build the scene", e);
    cb("error", "", String(e?.message ?? e));
  }

  // Resume at a saved "map,x,z,facing"; a map that no longer loads falls back to the overworld.
  const [sm, sx, sz, sf] = (start ?? "").split(",");
  const at = sm && DIR[sf] && Number.isInteger(+sx) && Number.isInteger(+sz) ? { x: +sx, z: +sz, facing: sf } : null;
  // The first build counts as a warp: travel or an edge warp while its materials compile would dispose them
  // mid-compile (three's compileAsync then never settles and the world stays on "Loading").
  warping = true;
  (at ? enter(sm, at).catch(() => enter("overworld")) : enter("overworld"))
    .finally(() => (warping = false))
    .then(() => {
      if (!alive) return;
      clock.getDelta();
      raf = requestAnimationFrame(frame);
      if (edit)
        import("./editor.js").then((m) => {
          if (!alive) return;
          const tool = m.attach({ el, canvas: renderer.domElement, world: () => world, tileAt, rebuild: () => enter(world.name, { x: hero.x, z: hero.z, facing: hero.face }) });
          stopEditor = tool.stop;
          editing = tool.editing;
        });
    })
    .catch(fail);

  // ---------- Loop ----------
  let stats = { calls: 0, triangles: 0 };
  function frame() {
    raf = requestAnimationFrame(frame);
    const dt = Math.min(clock.getDelta(), 0.05);
    if (paused && !book.live()) {
      // A modal is open: draw one last frame, then nothing until it closes (a book being read draws on).
      if (drawnPaused) return;
      drawnPaused = true;
    } else drawnPaused = false;
    const w = world, h = hero;
    w.now += dt;
    if (!paused) tick(dt);
    U.uTime.value = w.now;
    if (!reduced) U.uDrift.value = w.now;
    const prevStep = h.step;

    // Finish (or carry on) the current step.
    let carry = 0, chained = false, spotted = null;
    if (h.step) {
      h.step.t += dt / h.step.dur;
      if (h.step.t >= 1 && h.step.stoop) {
        // Into a house: hold the last pose, on its steps at the door, while the fade comes up (`enter`
        // places the hero inside; if that fails, it steps back where it was).
        const s = h.step;
        s.t = 1;
        if (!s.going) (s.going = true), warp(s.warp).finally(() => h.step === s && (h.step = null));
      } else if (h.step.t >= 1) {
        const done = h.step, left = w.gateAt.get(key(h.x, h.z));
        carry = (done.t - 1) * done.dur;
        h.x = done.x;
        h.z = done.z;
        h.step = null;
        chained = true;
        w.calm = false; // a step taken: duelists may look again
        // Off a gate that only stayed open for the hero (rafts at dawn): it may shut now.
        if (left?.open && w.gateAt.get(key(h.x, h.z)) !== left) recheck(true);
        const wp = done.warp ?? w.warps.get(key(h.x, h.z));
        if (wp && !cut.playing) warp(wp);
        else if (!wp) cut.stepped(w, h.x, h.z);
      }
    }
    if (w.scriptsDue && !warping && !paused && !h.step && !cut.playing) (w.scriptsDue = false), cut.entered(w); // after a scene that warped here, too
    // Standing on a tile (a step just ended, or still): does a duelist see the hero?
    // After one was dealt with, the next waits for the hero's next step.
    if (!h.step && !warping && !paused && !w.engage && !w.ride && !w.calm && !cut.playing && !editing() && (spotted = duelists.watch(w, h, holds))) {
      held.length = 0;
      w.path = [];
      duelists.spot(w, spotted, h, sound);
    }
    if (!paused) duelists.update(w, dt, talkTo);
    if (!h.step && !warping && !paused && !w.engage) {
      const k = cut.playing ? null : held[held.length - 1]; // a cutscene walks the hero itself
      if (k) {
        const f = dirOf(k);
        if (f !== h.face) {
          h.face = f;
          h.turnedAt = w.now;
          if (chained) tryStep(f, carry); // already walking: turn and keep going
        } else if (chained || w.now - h.turnedAt >= TURN_HOLD) tryStep(f, carry);
      } else if (w.path.length) {
        const npc = w.path.npc, near = npc && Object.keys(DIR).find((d) => h.x + DIR[d][0] === npc.x && h.z + DIR[d][1] === npc.z);
        if (near) {
          h.face = near; // a walking person within reach: turn and talk
          w.path = ["act"];
        } else if (npc && w.path[0] !== "act") w.path = walkTo(npc.x, npc.z, npc);
        const f = w.path.shift();
        if (f === "act") act();
        // Blocked: stop, unless that was the turn into what was tapped (then use it),
        // or a person walked into the way (re-planned on the next frame).
        else if (f && !tryStep(f, carry) && w.path[0] !== "act" && !npc) w.path = [];
      }
    }
    if (h.step && h.step !== prevStep) {
      w.life.puff(h.x + 0.5, h.z + 0.5); // footstep dust
      sound("step", w.map.tiles[h.step.z]?.[h.step.x] === "w" ? 1 : 0); // "w": a wooden floor
    }
    h.idle = h.step || held.length ? 0 : h.idle + dt;
    h.running = !!h.step?.run;

    lifts.update(w, dt);
    const [px, pz] = place(h, dt, w.floorAt);
    animateActor(h, dt, reduced);
    book.pose(h, dt);
    for (const n of w.npcs) updateNpc(w, n, dt, paused);
    animateProps(w, dt, reduced);
    const lit = w.day.update(minute, px, pz, dt);
    sky(lit);
    w.life.update(dt, w.now, px, pz, lit.night);
    w.fireflies.update(dt, w.now, lit.night, reduced);

    const pos = `${h.x},${h.z},${h.face}`;
    const inside = h.x >= 0 && h.z >= 0 && h.x < w.map.width && h.z < w.map.height; // not mid-way off an edge
    // Written only on change, and never mid-scene: a reload then replays the cutscene from where it began.
    if (pos !== lastHero && inside && !cut.playing) (el.dataset.hero = lastHero = pos), cb("at", w.name, pos);

    // What the hero faces, while standing still.
    const front = h.step || warping ? null : facing();
    const said = front?.kind === "book" ? front.id : front?.text; // a book is named by Rust, from its id
    const nk = front ? `${front.kind}:${said}` : "";
    if (nk !== w.nearKey) {
      w.nearKey = nk;
      cb("near", front?.kind ?? "", said ?? "");
    }
    if (w.door) w.door.want = front?.kind === "warp" || h.step?.warp ? 1 : 0;

    // Camera: fixed pitch, on a soft spring behind a point a little ahead of a
    // walking hero; leans in after a while idle; indoors, kept inside the room.
    const pitch = THREE.MathUtils.degToRad(cam.pitch), off = cam.lookAtOffset;
    const ahead = h.step && !reduced ? 0.7 : 0;
    const shot = cut.aim(); // a cutscene's `cam` target
    if (shot) aim.set(shot[0] + off[0], off[1] + shot[1], shot[2] + off[2]);
    else aim.set(px + off[0] + DIR[h.face][0] * ahead, off[1] + h.fy, pz + off[2] + DIR[h.face][1] * ahead);
    if (cam.clamp) {
      // Keep the view inside the room: the ground it shows spans [aim + top, aim + bottom]
      // along z (perspective makes the far side longer) and ±halfW along x.
      // The top edge is the top bar's lower edge, not the canvas's.
      const half = THREE.MathUtils.degToRad(camera.fov / 2), hc = dist * Math.sin(pitch) + off[1], d0 = dist * Math.cos(pitch);
      const halfTop = Math.atan((1 - 2 * cover) * Math.tan(half));
      const top = pitch > halfTop ? d0 - hc / Math.tan(pitch - halfTop) : -Infinity, bottom = d0 - hc / Math.tan(pitch + half);
      const halfW = dist * Math.tan(half) * camera.aspect;
      // A room bigger than the view: the view stays inside it. Smaller: the
      // camera still follows the hero, but only as far as keeps the whole room
      // (and its back wall, about 1 m) in view.
      const fit = (v, lo, hi) => THREE.MathUtils.clamp(v, Math.min(lo, hi), Math.max(lo, hi));
      aim.x = fit(aim.x, halfW, w.map.width - halfW);
      if (Number.isFinite(top)) {
        // A room deeper than the view leans to its back row (counters, shelves) as
        // long as the hero stays a little over the bottom edge.
        const lo = -top - 1, hi = w.map.height - bottom;
        aim.z = fit(aim.z, lo, lo < hi ? Math.max(lo, Math.min(hi, pz + 1.2 - bottom)) : hi);
      }
    }
    if (first || reduced) look.copy(aim), lookVel.set(0, 0, 0);
    else {
      const k = cut.playing ? 10 : 28, c = 2 * Math.sqrt(k) * (cut.playing ? 1 : 0.85); // slightly under-damped; a scene pans slowly
      lookVel.addScaledVector(aim.clone().sub(look), k * dt).addScaledVector(lookVel, -c * dt);
      look.addScaledVector(lookVel, dt);
    }
    first = false;
    if (!paused) cut.update(dt);
    zoom += ((h.idle > FIDGET_AFTER && !reduced ? 0.88 : 1) - zoom) * Math.min(1, dt * 1.5);
    const dz = dist * zoom;
    camera.position.set(look.x, look.y + Math.sin(pitch) * dz, look.z + Math.cos(pitch) * dz);
    camera.lookAt(look);
    // Outdoors the sun (and its shadow map) follows the hero a whole tile at a
    // time: following the tweened position makes shadow edges shimmer.
    if (!w.map.light.indoor) {
      const from = lit.from;
      w.sun.position.set(h.x + from.x, from.y, h.z + from.z);
      w.sun.target.position.set(h.x, 0, h.z);
    }

    // The shadows (shadowEye), then the scene. Tall scenery between the camera and the
    // hero thins out round the hero (props.js `fadeable`); the hero is never drawn through it.
    // Kept through a warp's fade too: switching it off there popped the thinned scenery back in front of the door.
    if (hero) CUTAWAY.hero.value.set(hero.obj.position.x, hero.obj.position.y + 0.8, hero.obj.position.z);
    else CUTAWAY.hero.value.set(0, -1e4, 0);
    renderer.shadowMap.needsUpdate = true;
    renderer.info.reset();
    renderer.render(scene, shadowEye);
    renderer.clear();
    renderer.render(scene, camera);
    book.render(renderer, dt);
    stats = { calls: renderer.info.render.calls, triangles: renderer.info.render.triangles };
    if (!readySent) (readySent = true), cb("ready", "", "");
  }
  // The clock, from the save or an inn's rest (`minute` at a phase's start: shown
  // already turned, as if the fade happened while the hero slept).
  function setClock(m) {
    const at = ((+m % DAY) + DAY) % DAY;
    minute = Object.values({ dawn: 300, day: 480, dusk: 1080, night: 1260 }).includes(at) ? at + PHASE_FADE : at;
    if (!world) return;
    lastPhase = phaseOf(minute);
    lastHour = Math.floor(minute / 60);
    recheck(false);
    tune();
  }
  // For trying the hours out from the console: `el.sixfoldClock(21 * 60)` (Rust hears it at once).
  el.sixfoldClock = (m) => (cb("clock", String(Math.floor(m)), ""), setClock(m));
  el.sixfoldWarp = (to, x, z, facing = "s") => warp({ to, spawn: { x, z, facing }, travel: true }); // and anywhere: `el.sixfoldWarp("square", 12, 8)`
  // Walks there as a tap would (the same route, collisions, heights, gates and ladders); `[]` when there's no way.
  el.sixfoldWalk = (x, z) => (busy() ? [] : [...(world.path = walkTo(x, z, world.npcAt(x, z)))]);
  // For measuring: draw calls and triangles of the last frame (shadows included), and the area.
  // For reviewing a whole map: `el.sixfoldCam({ distance: 40, pitch: 62 })` pulls the camera back
  // (no fog, a far plane to match) until the next map loads.
  el.sixfoldCam = (o) => (Object.assign((cam = { ...cam, ...o }), { distancePortrait: o.distance ?? cam.distancePortrait }), (camera.far = 200), (scene.fog = null), resize());
  el.sixfoldStats = () => ({ ...stats, area: world?.name ?? "", minute: Math.floor(minute), phase: phaseOf(minute), at: hero && [hero.x, hero.z, hero.face] });

  // Item popup preview: the model turning in its own small canvas.
  let pv = null;
  function preview(box, name) {
    previewStop();
    const bw = box.clientWidth || 160, bh = box.clientHeight || 160;
    const r = new THREE.WebGLRenderer({ antialias: true, alpha: true });
    r.setPixelRatio(Math.min(devicePixelRatio, 2));
    r.setSize(bw, bh);
    r.domElement.setAttribute("aria-hidden", "true");
    box.appendChild(r.domElement);
    const s = new THREE.Scene();
    const sun = new THREE.DirectionalLight("#fff4dc", 2.4);
    sun.position.set(2, 3, 4);
    s.add(new THREE.HemisphereLight("#ffffff", "#8a7a66", 2), sun);
    const c = new THREE.PerspectiveCamera(30, bw / bh, 0.05, 20);
    c.position.set(0, 0.4, 1.9);
    c.lookAt(0, 0, 0);
    const me = (pv = { r, raf: 0, o: null });
    loadAsset(name).then((g) => {
      if (pv !== me) return;
      const o = g ? g.scene.clone(true) : (PLACEHOLDER[name] ?? (() => group()))();
      prep(o, false);
      const box3 = new THREE.Box3().setFromObject(o), size = box3.getSize(new THREE.Vector3());
      const k = 0.7 / Math.max(size.x, size.y, size.z, 0.01);
      o.scale.multiplyScalar(k);
      o.position.sub(box3.getCenter(new THREE.Vector3()).multiplyScalar(k));
      me.o = group(o);
      me.o.rotation.x = 0.25;
      s.add(me.o);
    });
    const loop = () => {
      me.raf = requestAnimationFrame(loop);
      if (me.o && !reduced) me.o.rotation.y += 0.02;
      r.render(s, c);
    };
    me.raf = requestAnimationFrame(loop);
  }
  function previewStop() {
    if (!pv) return;
    cancelAnimationFrame(pv.raf);
    pv.r.dispose();
    pv.r.domElement.remove();
    pv = null;
  }

  // A speaker's bust for the dialogue box: their own rig in their own outfit and hair, in the pose
  // they hold now, drawn once per map visit by a small offscreen renderer.
  // The renderer is dropped on each new map, so the people it uploaded never pile up.
  let bustR = null, busts = new Map(), bustsOf = null;
  function portrait(id) {
    const n = world?.npcs.find((p) => p.def.id === id);
    if (!n?.obj) return "";
    if (bustsOf !== world) {
      // A new map: drop the last one's busts and the people uploaded to draw them.
      bustR?.dispose(), bustR?.forceContextLoss();
      (bustR = null), (busts = new Map()), (bustsOf = world);
    }
    if (busts.has(id)) return busts.get(id);
    const S = 256;
    bustR ??= new THREE.WebGLRenderer({ antialias: true, alpha: true, preserveDrawingBuffer: true });
    bustR.setPixelRatio(1);
    bustR.setSize(S, S, false);
    const s = new THREE.Scene();
    const key = new THREE.DirectionalLight("#fff4dc", 2.6), rim = new THREE.DirectionalLight("#cfe0ff", 1.8);
    key.position.set(-1.5, 2.5, 3);
    rim.position.set(2, 1.5, -2.5);
    const o = cloneSkinned(n.obj);
    o.position.set(0, 0, 0);
    o.rotation.set(0, 0, 0); // facing +z, toward the camera
    s.add(new THREE.HemisphereLight("#ffffff", "#6a5a4a", 1.8), key, rim, o);
    o.updateMatrixWorld(true);
    const box = new THREE.Box3().setFromObject(o), size = box.getSize(new THREE.Vector3());
    // People: head and shoulders (the upper half: they're big-headed); a beast wider than it is tall: all of it.
    const beast = Math.max(size.x, size.z) > size.y;
    const h = beast ? Math.max(size.y, size.z) * 1.15 : size.y * 0.5;
    const cy = beast ? box.getCenter(new THREE.Vector3()).y : box.max.y - h * 0.5;
    const cam = new THREE.PerspectiveCamera(22, 1, 0.05, 50);
    const dir = new THREE.Vector3(0.42, 0.14, 1).normalize();
    cam.position.set(0, cy, 0).addScaledVector(dir, h / 2 / Math.tan(THREE.MathUtils.degToRad(11)));
    cam.lookAt(0, cy, 0);
    bustR.render(s, cam);
    const url = bustR.domElement.toDataURL("image/webp", 0.85);
    busts.set(id, url);
    return url;
  }

  const stop = () => {
    alive = false;
    globalThis.sixfoldAudio?.ambience("");
    bustR?.dispose();
    bustR?.forceContextLoss();
    stopEditor();
    previewStop();
    book.dispose();
    cancelAnimationFrame(raf);
    ro.disconnect();
    mq.removeEventListener("change", onMotion);
    removeEventListener("keydown", onKey);
    removeEventListener("keyup", onKey);
    removeEventListener("blur", clearHeld);
    dispose(scene);
    renderer.dispose();
    renderer.domElement.remove();
    clearTimeout(bannerTimer);
    for (const n of [pad, run, fade, legend, banner]) n.remove();
  };
  return {
    stop,
    act,
    busy: () => warping || !world || !!world.engage || !!world.ride || cut.playing || book.live(), // the in-game menu waits for these
    // A book to read ({id, family, title, pages: [{heading, text, art}], shelf}): from the shelf
    // last used when `shelf` is true, else it rises into view. null puts it back ("book closed" after).
    read(b) {
      if (!b) return void book.close(() => alive && cb("book", "closed", ""));
      let ndc = null;
      if (b.shelf && shelf && world) {
        const v = new THREE.Vector3(shelf.x + 0.5, world.floorAt(shelf.x, shelf.z) + 1.15, shelf.z + 0.95).project(camera);
        ndc = { x: v.x, y: v.y };
      }
      book.open(b, ndc);
    },
    // Where the open book is on screen ([left, top, right, bottom], CSS pixels): taps outside it don't turn pages.
    bookRect: () => book.rect(),
    // Turn to page `n` (a spread), from the open book's current one.
    page: (n) => book.page(n),
    pause(p) {
      paused = !!p;
      clearHeld();
      if (world) world.path = [];
      if (paused && world) cb("clock", String(Math.floor(minute)), ""); // a duel may follow: keep the time
      if (!paused && world) {
        clock.getDelta();
        world.npcs.forEach((n) => (n.busy = false));
        if (world.engage?.talked) (duelists.release(world), (world.calm = true)); // the talk with a duelist is over
        recheck(true); // a find may be gone now, a gate open
      }
    },
    setClock,
    // Back from a duel against `id`: that person (a duelist walks back over) says its post-duel line,
    // unless a scene is due on this map (the finale after the Warden): the scene speaks instead.
    meet(id) {
      if (!world || warping || cut.playing || (world.scriptsDue && world.hero && cut.due(world))) return;
      const n = duelists.meet(world, hero, id);
      if (n && facing()?.npc === n) talkTo(n);
    },
    // A dialogue cue (engine `Cue`), e.g. {"emote": "heart", "actor": "moss"}; actor "hero" is the hero.
    // {"warp": {"map", "x", "z", "facing"}, "travel": true}: fade and load (a ferry, fast travel).
    // {"sfx": "spark"}: a sound. A cutscene's cues carry an `id` (script.js answers `cue_done`), and
    // {"end": [cues]} closes the scene, jumping over those cues (a skip).
    cue(json) {
      const c = JSON.parse(json);
      if (c.end) return void (world && cut.end(c.end));
      if (c.warp && c.id === undefined) {
        const { map, x, z, facing } = c.warp;
        if (world && !warping) warp({ to: map, spawn: { x, z, facing }, travel: !!c.travel });
        return;
      }
      if (world) cut.play(c); // one cue player (script.js), for dialogue and cutscenes alike
      else if (c.id !== undefined) cb("cue_done", String(c.id), "");
    },
    setItems(list) {
      canRun = list.split(",").includes("running_shoes");
      run.hidden = !canRun;
    },
    preview,
    previewStop,
    portrait,
  };
}

