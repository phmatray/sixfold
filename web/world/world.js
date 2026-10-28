// The overworld test: tile-grid maps (./maps/<name>.json) drawn with three.js.
// `mount(host, controls, cb, start, edit, debug)` builds it (at `start` = "map,x,z,facing"
// when given; `edit`/`debug`: a debug build with ?edit loads the map editor, with ?debug
// the level-design overlays, debug.js) and returns an
// api for Leptos (stop, act, pause, setItems, setSeals, setClock, setCamera, preview, previewStop, portrait, meet, cue, busy, read, page).
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
//   ("bark", name, line)   a person said a line in passing (npc.js `barkOf`), for the live region
//   ("camera", angle, "")  the C key (or a pad's R3) picked the next outdoor camera angle (camera.js)
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
//   duelist.js route duelists: line of sight, the walk over, going quiet
//   lift.js    cages that carry the hero between two heights (the Rodspire)
//   seat.js    sitting on benches and chairs (the hero, and people placed sitting)
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
import { mote } from "./fireflies.js";
import { DIR, BACK, angle, key, FIDGET_AFTER, actor, place, animateActor, route, say } from "./actors.js";
import { updateNpc, barkOf, BARK_HOLD, facing as facingOf } from "./npc.js";
import * as duelists from "./duelist.js";
import * as lifts from "./lift.js";
import * as seats from "./seat.js";
import { daylight, phaseOf, DAY } from "./daylight.js";
import { scenes } from "./script.js";
import { createBook } from "./book.js";
import { ANGLES, framing, nextAngle } from "./camera.js";

const STEP = 0.25, RUN_STEP = 0.17; // seconds per tile: walk 4 tiles/s, run ~6 (controllable on a phone)
const TURN_HOLD = 0.12; // a press shorter than this only turns the hero
const FADE = 350; // ms, warp fade
const KEYS = { w: "n", z: "n", arrowup: "n", s: "s", arrowdown: "s", a: "w", q: "w", arrowleft: "w", d: "e", arrowright: "e" }; // WASD and AZERTY ZQSD
const CLOCK_RATE = 1; // game minutes per real second: a day is 24 minutes (WORLD_DESIGN §7.2)
const PHASE_FADE = 20; // daylight.js cross-fades this many game minutes into a phase
const WAKE_MS = 600; // a seal stone wakes (and goes back to rest) over this long
const STONE_LIGHT = 3; // its light at full wake
// The seal-stone face whose colour is the region's, by the map's `region` (stone:wake).
const REGION_FACE = { ember: "fire", storm: "lightning", tide: "water", dream: "psychic", grove: "grass", stone: "fighting" };

// A map's seal stone, ready to wake in a scene (script.js `stone:wake`): its coloured faces (the vertices
// build.js notes in the stone's merged mesh, `seal`: recoloured, so waking costs no draw call) and, outdoors,
// one real light at it, made with the map at 0 so waking it never recompiles a shader. With the hero's
// lantern and the three live lamps that is five, under the six (WORLD_DESIGN P1). The region's colour is
// its own face's; a map with no region wakes each face in its own.
// A face whose seal is held (`held`, the colour keys setSeals gives) stays lit; the others rest dormant (#153).
function stoneRig(w, held) {
  const faces = [], box = new THREE.Box3();
  w.root.updateMatrixWorld(true);
  w.root.traverse((o) => o.isMesh && o.userData.seal && (faces.push(...o.userData.seal.map((f) => ({ ...f, attr: o.geometry.attributes.color }))), box.expandByObject(o)));
  if (!faces.length) return null;
  const color = faces.find((f) => f.colour === REGION_FACE[w.map.region])?.color.clone() ?? null;
  let light = null;
  if (!w.map.light.indoor) {
    light = new THREE.PointLight(color ?? "#ffe2a0", 0, 7, 1.5);
    w.root.worldToLocal(box.getCenter(light.position).add(new THREE.Vector3(0, 0.6, 1))); // a little over it, toward the camera
    light.layers.enableAll();
    w.root.add(light);
  }
  for (const f of faces) f.dormant = f.color.clone().lerp(DORMANT, DORMANT_K);
  const at = w.root.worldToLocal(box.getCenter(new THREE.Vector3()));
  // The seal's motes, made with the map and drawn once unseen, so the seal won on stage compiles no shader.
  const geo = new THREE.BufferGeometry().setAttribute("position", new THREE.BufferAttribute(new Float32Array(BURST * 3), 3));
  const pts = new THREE.Points(geo, new THREE.PointsMaterial({ map: mote(), size: 0.42, transparent: true, opacity: 0, depthWrite: false }));
  pts.frustumCulled = false;
  w.root.add(pts);
  const burst = { pts, vel: new Float32Array(BURST * 3), t: BURST_S - 1e-3 }; // its first frame draws it at opacity 0
  return { faces, color, light, at, held, k: 0, want: 0, dirty: true, burst };
}
// Ease the stone toward its want (a cut under reduced motion): the faces' glow and the light together.
function stoneStep(s, dt, reduced) {
  if (s?.burst) burstStep(s, dt);
  if (!s || (s.k === s.want && !s.dirty)) return;
  s.dirty = false;
  s.k = reduced ? s.want : s.k + Math.sign(s.want - s.k) * Math.min(Math.abs(s.want - s.k), (dt * 1000) / WAKE_MS);
  const e = s.k * s.k * (3 - 2 * s.k);
  // Each face takes on the region's colour, brighter than paint (a map with no region: each its own); a held
  // seal's face stays lit in its own colour through it all, and the others rest dormant.
  for (const f of s.faces) {
    if (s.held.has(f.colour)) tmpColor.copy(f.color).multiplyScalar(LIT);
    else tmpColor.copy(f.dormant).lerp(tmpColor2.copy(s.color ?? f.color).multiplyScalar(GLOW), e);
    for (let i = f.start; i < f.start + f.count; i++) f.attr.setXYZ(i, tmpColor.r, tmpColor.g, tmpColor.b);
    f.attr.needsUpdate = true;
  }
  if (s.light) s.light.intensity = STONE_LIGHT * e;
}
const tmpColor = new THREE.Color(), tmpColor2 = new THREE.Color();
const GLOW = 1.8; // an awake face: its colour this much over paint (vertex colours may pass 1)
const LIT = 1.5; // a held seal's face, lit for good: its colour this much over paint, with no light (#153)
// A face whose seal isn't held yet rests dormant: its paint sunk most of the way into the stone.
const DORMANT = new THREE.Color("#9c968b"), DORMANT_K = 0.75;
const BURST = 60, BURST_S = 1.2; // the motes a seal won on stage sends up, and for how long (s)
const SEAL_KEYS = ["grass", "water", "fire", "lightning", "fighting", "psychic"]; // the bells' order (props.js SEAL_ORDER)
const WHITE = new THREE.Color("#ffffff");
// A seal won on stage (`stone:seal`): its face lights for good and the rest of the glow gathers into it
// (the wake fades), while BURST motes in its colour rise off the stone and fade over BURST_S, one draw
// call. Under reduced motion the face just lights: no motes.
function sealFace(s, colour, reduced) {
  s.held.add(colour);
  s.dirty = true;
  s.want = 0;
  const f = s.faces.find((f) => f.colour === colour);
  if (reduced || !f) return;
  const b = s.burst, pos = b.pts.geometry.attributes.position;
  for (let i = 0; i < BURST; i++) {
    const a = Math.random() * Math.PI * 2, r = 0.4 + Math.random() * 1.2;
    pos.array.set([s.at.x, s.at.y + 0.3, s.at.z], i * 3);
    b.vel.set([Math.cos(a) * r, 1 + Math.random() * 1.8, Math.sin(a) * r], i * 3);
  }
  b.pts.material.color.copy(f.color).lerp(WHITE, 0.25);
  b.pts.visible = true;
  b.t = 0;
}
function burstStep(s, dt) {
  const b = s.burst, p = b.pts.geometry.attributes.position;
  if (b.t >= BURST_S) return void (b.pts.visible = false); // spent, or (at the map's start) drawn once unseen
  b.t += dt;
  for (let i = 0; i < BURST * 3; i += 3) {
    b.vel[i + 1] -= dt * 1.2; // they slow as they rise
    for (let k = 0; k < 3; k++) p.array[i + k] += b.vel[i + k] * dt;
  }
  p.needsUpdate = true;
  b.pts.material.opacity = Math.max(0, 1 - (b.t / BURST_S) ** 2); // a fade, never a blink
}

// ---------- Mount ----------

// Sounds come from web/audio/audio.js (loaded with the wasm), when it is there.
const sound = (name, n = 0) => globalThis.sixfoldAudio?.sfx(name, n);
// The hero's bust (an object URL), outliving the world it was baked in: the duel table shows it (#177).
const HERO_BUST = "\u0000hero"; // its place in the bust queue, never an NPC's id
let heroBust = "";

export function mount(el, controls, cb, start, edit = false, debug = false) {
  const mq = matchMedia("(prefers-reduced-motion: reduce)");
  let reduced = mq.matches;
  const onMotion = (e) => ((reduced = e.matches), (U.uSway.value = reduced ? 0 : 1));
  mq.addEventListener("change", onMotion);
  U.uSway.value = reduced ? 0 : 1;

  const renderer = new THREE.WebGLRenderer({ antialias: true });
  renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = THREE.PCFShadowMap; // soft since r182 (PCFSoftShadowMap is gone)
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
  // The outdoor angle the player picked (camera.js, #148), and the pitch and distance easing toward it
  // (only after a new angle: a rotated phone or sixfoldCam still cut).
  let camAngle = "default", pitchNow = 50, distNow = 12, camEase = false;
  let dist = 12, alive = true, raf = 0, paused = false, warping = false, world = null, hero = null;
  let canRun = false, runToggle = false, shift = false, lastHero = "";
  const sealsHeld = new Set(); // the seals held (setSeals): their stone faces stay lit
  // The clock (the engine owns the saved minute; Rust hands it over with setClock).
  let minute = 480, lastHour = -1, lastPhase = "", bgAt = 0;
  const clock = new THREE.Timer();
  const look = new THREE.Vector3(), lookVel = new THREE.Vector3(), aim = new THREE.Vector3();
  let first = true, zoom = 1, readySent = false, drawnPaused = false, cover = 0, quiet = false;
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
  // A scene's caption (a reveal's line, the door's bells, #155): a paper plate, never mistaken for the area's
  // name on its dark pill.
  // A talk box hides it (style.css `.scene-caption`).
  const caption = document.createElement("div");
  caption.className = "scene-caption";
  caption.setAttribute("aria-hidden", "true");
  Object.assign(caption.style, {
    position: "absolute", zIndex: 2, left: "50%", bottom: "18%", transform: "translateX(-50%)", pointerEvents: "none",
    padding: "10px 20px", borderRadius: "12px", background: "var(--paper)", color: "var(--toon)", border: "2.5px solid var(--toon)",
    boxShadow: "0 3px 0 var(--toon)", font: "600 clamp(15px, 2vw, 19px) 'Bricolage Grotesque', system-ui, sans-serif",
    width: "max-content", maxWidth: "calc(100% - 32px)", textAlign: "center", textWrap: "balance", opacity: 0, transition: "opacity 0.4s ease",
  });
  let captionTimer = 0;
  // Placed from what it names on screen (the seal stone, or the door's medallion): beside it, on the side
  // with more room, never over it (#155).
  const subjectY = () => {
    const o = world?.root.getObjectByName("inlay_0") ?? null, s = world?.stone;
    const p = o ? o.getWorldPosition(new THREE.Vector3()) : s ? world.root.localToWorld(s.at.clone()) : null;
    if (!p) return null;
    // Its top and its foot on screen (a stone stands about 2 m tall; the medallion is small).
    const y = (q) => ((1 - q.project(camera).y) / 2) * el.clientHeight;
    const [up, down] = o ? [0.5, 0.5] : [1.3, 1.0];
    return { mid: y(p.clone()), top: y(p.clone().setY(p.y + up)), foot: y(p.clone().setY(p.y - down)) };
  };
  // Placed again every frame while it shows (#155): the camera is often still moving onto what it names.
  const placeCaption = (fresh = false) => {
    const at = subjectY(), H = el.clientHeight, gap = 28;
    if (at && fresh) caption.above = at.mid > H / 2; // the side is picked once per caption: it never flips mid-move
    if (!at) Object.assign(caption.style, { top: "auto", bottom: "18%" });
    else if (caption.above) Object.assign(caption.style, { top: "auto", bottom: `${Math.round(Math.min(H * 0.85, Math.max(16, H - at.top + gap)))}px` });
    else Object.assign(caption.style, { bottom: "auto", top: `${Math.round(Math.min(H * 0.85, at.foot + gap))}px` });
  };
  const showCaption = (text, ms) => {
    placeCaption(true);
    caption.textContent = text;
    caption.style.opacity = 1;
    clearTimeout(captionTimer);
    captionTimer = setTimeout(() => (caption.style.opacity = 0), ms);
  };
  // A scene's end takes its caption with it (a skip, a cut back under reduced motion).
  const clearCaption = () => (clearTimeout(captionTimer), (caption.style.opacity = 0));
  el.append(fade, legend, banner, caption);

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
    // (web/gamepad.js) moves focus with the arrows nothing takes. One pressed during
    // a warp is kept (the warp drops only those held before it): the hero walks once it is in.
    if (busy() && !(warping && KEYS[k])) return void (KEYS[k] && e.preventDefault());
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
    } else if (k === "c" && !e.repeat && !e.ctrlKey && !e.metaKey && !e.altKey && !world?.map.light.indoor) { // not a copy (Ctrl/Cmd+C); rooms keep their own view, so C changes nothing there (#183)
      setCamera(nextAngle(camAngle));
      cb("camera", camAngle, "");
    }
  };
  // Escape (or a pad's B) while sitting stands up, before it can open the menu.
  const onEsc = (e) => {
    if (e.key !== "Escape" || !hero?.sit || paused || e.repeat) return;
    e.preventDefault();
    e.stopImmediatePropagation();
    seats.stand(hero);
  };
  // The outdoor camera angle (#148): rooms keep their own framing; the frame eases to it.
  const setCamera = (a) => {
    camAngle = ANGLES.includes(a) ? a : "default";
    if (world) (cam = framing(world.map.camera, camAngle, !!world.map.light.indoor)), resize(), (camEase = true);
  };
  const clearHeld = () => ((held.length = 0), (shift = false));
  const padDir = (e) => e.target.closest?.("[data-dir]")?.dataset.dir;
  const padDown = (e) => {
    const f = padDir(e);
    if (!f || (busy() && !warping)) return; // pressed during a warp: kept, as a key is
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
    if (hero.sit) return seats.stand(hero);
    tryStep(f, 0);
  };
  addEventListener("keydown", onKey);
  addEventListener("keydown", onEsc, true);
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
  let stopEditor = () => {}, editing = () => false, stopDebug = () => {}; // the map editor (?edit) and overlays (?debug), when loaded
  // A finger walks by tapping only when the player chose it in Options (web/src/store.rs):
  // otherwise a tap that misses the pad's buttons would send the hero across the map.
  const tapWalk = () => {
    try {
      return localStorage.getItem("sixfold.tap-walk") === "1";
    } catch {
      return false;
    }
  };
  const onTap = (e) => {
    if (busy() || e.button || (e.pointerType !== "mouse" && !tapWalk())) return;
    if (hero.sit) return seats.stand(hero); // a tap stands up
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
    else if (path.length && (npc || lift || world.talk.has(key(tx, tz)) || world.seats.has(key(tx, tz)) || world.gateAt.get(key(tx, tz))?.open === false)) path.push("act");
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
  // The room's lamps (lights, glows, shades) at `k` of their strength: the door's bells dim them so the inlays
  // are the brightest thing in the room (#155); the scene's end brings them back.
  const dimLamps = (k) => world?.root.traverse((o) => {
    const v = o.userData.lamp;
    if (v === undefined) return;
    if (o.isLight) o.intensity = v * k;
    else if (o.isSprite) o.material.opacity = v * k;
    else if (o.material?.color) {
      o.material.color.copy(v).multiplyScalar(0.35 + 0.65 * k);
      if (o.material.emissive) o.material.emissiveIntensity = k;
    }
  });
  // The sealed door's `n`th inlay lights in its seal's colour, for good (props.js seal_door).
  const ringDoor = (n) => {
    const m = world?.root.getObjectByName(`inlay_${n}`);
    if (m) m.material.color.set(m.userData.lit), m.material.emissive.set(m.userData.lit).multiplyScalar(0.7);
  };
  const recheck = (animate) => {
    refresh(world, holds, gate, animate);
    if (world.gateSound !== undefined) sound(world.gateSound ? "gate" : "door"), (world.gateSound = undefined);
    if (holds("flag:door_warden") || holds("beat:warden")) for (let i = 0; i < 6; i++) ringDoor(i); // the sealed door, rung
    // The gates that just opened, named on screen and in the live region (once: they opened now);
    // two at once (both vents at nightfall) share one banner rather than the second hiding the first.
    const names = [...new Set(world.opened?.splice(0))];
    if (names.length) {
      const list = names.join(tr(" and ", " et "));
      const text = names.length > 1 ? tr(`Ways open: ${list}`, `Des passages s'ouvrent\u00a0: ${list}`) : tr(`A way opens: ${list}`, `Un passage s'ouvre\u00a0: ${list}`);
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
    // At a new hour what is out may change too (`hour:`, #114: a shop shuts, a house lights its windows).
    if ((phase !== lastPhase || hour !== lastHour) && lastPhase) recheck(true);
    if (phase !== lastPhase && lastPhase) cut.playing || tune(); // a scene's `music:` holds until it ends
    lastHour = hour;
    lastPhase = phase;
  };
  // Cutscenes (script.js): the camera reports when it has reached what it was sent to look at.
  const BELLS = tr(["one", "two", "three", "four", "five", "six"], ["un", "deux", "trois", "quatre", "cinq", "six"]);
  const cut = scenes({
    world: () => world, hero: () => hero, reduced: () => reduced, holds, cb, sound, tune, warp,
    caption: (en, fr) => {
      const text = tr(en, fr);
      showCaption(text, 3400);
      cb("area", text, "");
    },
    // The door's bells (#154), seen and read too (#155): an inlay lights per bell, and the count builds up on screen;
    // the live region gets the whole count once.
    ended: () => (dimLamps(1), clearCaption()),
    bell: (n) => {
      // The sixth bell (#155): the sealed door swings open and its light spills into the hall, for as long as
      // the hall stays loaded (a cut under reduced motion, like any door).
      if (n >= 5) {
        const wp = [...(world?.warps.values() ?? [])].find((w) => w.to === "warden");
        if (wp?.door) wp.door.held = true;
        const spill = world?.root.getObjectByName("door_light");
        if (spill) spill.intensity = 4;
      }
      // Sable's own stone answers the door's bells (#155): each rings up a face of it, motes in that seal's colour.
      const door = world?.root.getObjectByName(`inlay_${n}`);
      if (!door && world?.stone) return void sealFace(world.stone, SEAL_KEYS[n], reduced);
      if (n === 0) dimLamps(0.12);
      ringDoor(n);
      const text = tr("The door rings: ", "La porte sonne\u00a0: ") + BELLS.slice(0, n + 1).join("… ") + (n >= 5 ? "." : "…");
      showCaption(text, n >= 5 ? 2600 : 1200);
      if (n >= 5) cb("area", text, "");
    },
    stone: (state, face) => {
      const s = world?.stone;
      if (!s) return;
      if (state === "seal") sealFace(s, face, reduced);
      else s.want = state === "wake" ? 1 : 0;
    },
    settled: () => look.distanceTo(aim) < 0.12 && lookVel.lengthSq() < 0.04,
    moved: () => (lastHero = ""), // the hero's tile is saved again (not during a scene: a reload replays it)
  });
  const opens = (wp) => holds(wp.if) && holds(wp.open); // `open`: a shop's hours (#114)
  // A find already taken on a seat (coins under a bench) gives the place back to sitting.
  const spent = (it) => it?.kind === "find" && holds(`flag:found_${it.id}`);
  const facing = () => {
    const it = hero.sit ? null : lifts.facing(world, hero) ?? facingOf(world, hero, opens);
    // Taken, on a seat: the seat (or, from behind it, nothing) rather than "Nothing else here".
    if (spent(it) && world.seats.has(key(hero.x + DIR[hero.face][0], hero.z + DIR[hero.face][1]))) return seats.facing(world, hero);
    return it ?? seats.facing(world, hero);
  };

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
    else if (it.kind === "seat") (world.path = []), seats.sit(hero, it.place, reduced);
    else if (it.kind === "stand") seats.stand(hero);
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
    seats.attach(next);
    next.name = name;
    duelists.enterMap(name);
    // Gates first see the hero where it arrives (a gate it stands on lets it stay).
    if (spawn && Number.isInteger(spawn.x)) next.spawnAt = spawn;
    refresh(next, holds, gate);
    if (world) dispose(world.root);
    world = next;
    world.hero = null;
    world.day = daylight(world, reduced);
    world.stone = stoneRig(world, sealsHeld);
    scene.add(world.root);
    const L = map.light;
    scene.fog = L.indoor ? null : new THREE.Fog(L.background, L.fog?.near ?? 60, L.fog?.far ?? 90);
    lastPhase = phaseOf(minute);
    world.scriptsDue = true; // an `enter` cutscene starts once the map is on screen
    cb("clock", String(Math.floor(minute)), "");
    cb("seen", name, L.indoor ? "in" : "out");
    const s = spawn && !next.blocked(spawn.x, spawn.z) && !next.npcAt(spawn.x, spawn.z) ? spawn : map.spawn; // a stale saved tile falls back
    Object.assign(hero, { x: s.x, z: s.z, face: s.facing, step: null, idle: 0, turnedAt: -Infinity, sit: null, pose: null }); // a fresh map clock; standing
    hero.obj.rotation.y = angle(s.facing);
    // A scene about to play on arrival that opens on its own music keeps what is playing until it
    // starts: no flip to the map's track in between (#177: the sealed door's bells into Sable's room).
    if (!cut.due(world)?.steps?.[0]?.do?.some((d) => d.startsWith("music:"))) tune();
    world.hero = hero;
    sky(world.day.update(minute, s.x + 0.5, s.z + 0.5, 0), true);
    cam = framing(map.camera, camAngle, !!map.light.indoor);
    resize();
    first = true;
    await renderer.compileAsync(scene, camera).catch(() => {});
    // The area's name, except back from a duel or when a scene is about to play on arrival (#155): the
    // moment is the scene's, not the map's. The live region still says where you are.
    if (map.title && !L.indoor && !quiet && !cut.due(world)) showBanner(map.title);
    if (map.title && !L.indoor) cb("area", map.title, "");
    quiet = false;
    preload(map, () => alive);
    bakeBusts(s);
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
  // A fifth field `back`: the world comes back from a duel (no area banner on that arrival).
  const [sm, sx, sz, sf, back] = (start ?? "").split(",");
  quiet = back === "back";
  const at = sm && DIR[sf] && Number.isInteger(+sx) && Number.isInteger(+sz) ? { x: +sx, z: +sz, facing: sf } : null;
  // The first build counts as a warp: travel or an edge warp while its materials compile would dispose them
  // mid-compile (three's compileAsync then never settles and the world stays on "Loading").
  warping = true;
  (at ? enter(sm, at).catch(() => enter("overworld")) : enter("overworld"))
    .finally(() => (warping = false))
    .then(() => {
      if (!alive) return;
      clock.update();
      raf = requestAnimationFrame(frame);
      if (edit)
        import("./editor.js").then((m) => {
          if (!alive) return;
          const tool = m.attach({ el, canvas: renderer.domElement, world: () => world, tileAt, rebuild: () => enter(world.name, { x: hero.x, z: hero.z, facing: hero.face }) });
          stopEditor = tool.stop;
          editing = tool.editing;
        });
      if (debug)
        import("./debug.js")
          .then((m) => alive && (stopDebug = m.attach({ world: () => world, scene, camera, el }).stop))
          .catch((e) => console.error("world: could not load the debug overlays", e));
    })
    .catch(fail);

  // ---------- Loop ----------
  let stats = { calls: 0, triangles: 0 };
  function frame() {
    raf = requestAnimationFrame(frame);
    const dt = Math.min(clock.update().getDelta(), 0.05);
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
    // Sitting: a move (key, stick, pad, a walk sent) or a scene stands the hero up; it walks on once up.
    if (h.sit && (held.length || w.path.length || cut.playing)) (w.path = []), seats.stand(h);
    seats.update(h, dt, reduced);
    if (!h.step && !warping && !paused && !w.engage && !h.sit) {
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
    book.pose(h);
    for (const n of w.npcs) updateNpc(w, n, dt, paused, minute);
    // A line said in passing (#114): never in a scene, a talk, a duel or with a sheet open (all `busy`);
    // one at a time on a map, so two never talk over each other in the bubbles or the live region.
    if (!busy() && !editing() && w.now >= (w.barkNext ?? 0)) for (const n of w.npcs) {
      const line = barkOf(w, n, holds);
      if (line) {
        say(n, line, BARK_HOLD), cb("bark", n.def.name, line);
        w.barkNext = w.now + BARK_HOLD + 1;
        break;
      }
    }
    animateProps(w, dt, reduced);
    stoneStep(w.stone, dt, reduced);
    const lit = w.day.update(minute, px, pz, dt);
    sky(lit);
    w.life.update(dt, w.now, px, pz, lit.night, cut.playing);
    w.fireflies.update(dt, w.now, lit.night, reduced);

    const pos = `${h.x},${h.z},${h.face}`;
    const inside = h.x >= 0 && h.z >= 0 && h.x < w.map.width && h.z < w.map.height; // not mid-way off an edge
    // Written only on change, and never mid-scene: a reload then replays the cutscene from where it began.
    if (pos !== lastHero && inside && !cut.playing) (el.dataset.hero = lastHero = pos), cb("at", w.name, pos);

    // What the hero faces, while standing still.
    // Only what pressing it would do: none while the world can't act (a cutscene, a duelist
    // coming over, a sheet open), so the prompt never offers a press that does nothing.
    const front = h.step || busy() ? null : facing();
    // A book is named by Rust, from its id; a sign, find or shut gate by its own name when it has one.
    const said = front?.kind === "book" ? front.id : ["sign", "find", "gate"].includes(front?.kind) ? (front.name ?? "") : front?.text;
    const nk = front ? `${front.kind}:${said}` : "";
    if (nk !== w.nearKey) {
      w.nearKey = nk;
      cb("near", front?.kind ?? "", said ?? "");
    }
    for (const d of w.doors) d.want = d.held || d === (front?.warp ?? h.step?.warp)?.door ? 1 : 0; // the door ahead opens (or one a scene holds open)

    // Camera: fixed pitch, on a soft spring behind a point a little ahead of a
    // walking hero; leans in after a while idle; indoors, kept inside the room.
    // A new angle (#148) eases the pitch and distance over about half a second (a cut on a new map or under reduced motion).
    const ease = first || reduced || !camEase ? 1 : Math.min(1, dt * 6);
    pitchNow += (cam.pitch - pitchNow) * ease;
    distNow += (dist - distNow) * ease;
    if (Math.abs(cam.pitch - pitchNow) < 0.01 && Math.abs(dist - distNow) < 0.001) camEase = false;
    const pitch = THREE.MathUtils.degToRad(pitchNow), off = cam.lookAtOffset;
    const ahead = h.step && !reduced ? 0.7 : 0;
    const shot = cut.aim(); // a cutscene's `cam` target
    if (shot) aim.set(shot[0] + off[0], off[1] + shot[1], shot[2] + off[2]);
    else aim.set(px + off[0] + DIR[h.face][0] * ahead, off[1] + h.fy, pz + off[2] + DIR[h.face][1] * ahead);
    // A scene's line on screen (#155): the talk box covers the bottom of the frame, so the shot moves down the
    // ground by as much, and whoever speaks and whoever listens both stand above it.
    if (cut.playing) {
      const box = el.closest(".world")?.querySelector(".talk-box")?.getBoundingClientRect();
      const H = el.clientHeight;
      if (box && H) {
        const half = THREE.MathUtils.degToRad(camera.fov / 2), span = (2 * distNow * zoom * Math.tan(half)) / Math.sin(pitch);
        aim.z += (Math.max(0, H - box.top) / H) * span * 0.9;
      }
    }
    if (cam.clamp) {
      // Keep the view inside the room: the ground it shows spans [aim + top, aim + bottom]
      // along z (perspective makes the far side longer) and ±halfW along x.
      // The top edge is the top bar's lower edge, not the canvas's.
      // At the camera's real distance: a scene's lean-in (zoom) lets the shot go closer to a wall (#155).
      const dzc = distNow * zoom;
      const half = THREE.MathUtils.degToRad(camera.fov / 2), hc = dzc * Math.sin(pitch) + off[1], d0 = dzc * Math.cos(pitch);
      const halfTop = Math.atan((1 - 2 * cover) * Math.tan(half));
      const top = pitch > halfTop ? d0 - hc / Math.tan(pitch - halfTop) : -Infinity, bottom = d0 - hc / Math.tan(pitch + half);
      const halfW = dzc * Math.tan(half) * camera.aspect;
      // A room bigger than the view: the view stays inside it. Smaller: the
      // camera still follows the hero, but only as far as keeps the whole room
      // (and its back wall, about 1 m) in view.
      const fit = (v, lo, hi) => THREE.MathUtils.clamp(v, Math.min(lo, hi), Math.max(lo, hi));
      // A scene's shot of a tile by the wall (the sealed door) may take in the wall's own thickness, no more,
      // so what it shows isn't cut by the frame's edge (#155).
      // An indoor place shot (the door) may show up to 1.2 m past the wall, so the place sits about a third in.
      const wall = shot ? (cut.onTile() && w.map.light.indoor ? 1.2 : 0.6) : 0;
      aim.x = fit(aim.x, halfW - wall, w.map.width - halfW + wall);
      if (Number.isFinite(top)) {
        // A room deeper than the view leans to its back row (counters, shelves) as
        // long as the hero stays a little over the bottom edge; a scene's shot holds its own depth.
        const lo = -top - 1, hi = w.map.height - bottom;
        aim.z = fit(aim.z, lo, lo < hi ? (shot ? hi : Math.max(lo, Math.min(hi, pz + 1.2 - bottom))) : hi);
      }
    }
    if (caption.style.opacity === "1") placeCaption();
    if (first || reduced) look.copy(aim), lookVel.set(0, 0, 0);
    else {
      const k = cut.playing ? 10 : 28, c = 2 * Math.sqrt(k) * (cut.playing ? 1 : 0.85); // slightly under-damped; a scene pans slowly
      lookVel.addScaledVector(aim.clone().sub(look), k * dt).addScaledVector(lookVel, -c * dt);
      look.addScaledVector(lookVel, dt);
    }
    first = false;
    if (!paused) cut.update(dt);
    // In a scene (#155: a scene reads as one) the camera leans in; on a place indoors (the sealed door, a
    // stone) it comes close, so the place is the subject. Sitting or idle a while: in a little. Reduced motion
    // cuts to a scene's distance and skips the idle lean.
    // A phone held upright already sees little across: it leans in less, so both people stay in frame (#155).
    const tall = camera.aspect < 1;
    const lean = cut.playing ? (cut.onTile() && w.map.light.indoor ? (tall ? 0.8 : 0.55) : tall ? 0.95 : 0.85) : !reduced && (h.sit || h.idle > FIDGET_AFTER) ? 0.88 : 1;
    zoom = reduced ? lean : zoom + (lean - zoom) * Math.min(1, dt * 1.5);
    const dz = distNow * zoom;
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
  el.sixfoldStats = () => ({ ...stats, area: world?.name ?? "", minute: Math.floor(minute), phase: phaseOf(minute), at: hero && [hero.x, hero.z, hero.face], sitting: !!hero?.sit, camera: { angle: camAngle, pitch: pitchNow },
    people: world?.npcs.filter((n) => !n.hidden).map((n) => ({ id: n.def.id, x: n.x, z: n.z, face: n.glance ?? n.face, do: n.activity?.do ?? null, away: !!n.away, walking: !!n.walk, sitting: !!n.sit })) ?? [] });
  // A map's ambient life (#115) as drawn last frame: critters per kind, leaves, embers, boats, the micro-event; null without any.
  el.sixfoldLife = () => world?.life.stats?.() ?? null;

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

  // A speaker's bust for the dialogue box: their own rig in their own outfit and hair, drawn once
  // per map visit by the world's own renderer into a small target (no second WebGL context), ahead
  // of time at idle once the map is up, the people nearest the hero first. A talk that comes
  // sooner shows a placeholder ("pending") and the bust follows (cb "bust", id, url).
  const BUST = 256;
  let bustRT = null, busts = new Map(), bustsOf = null, bustQ = [], bustWant = new Set(), baking = false, heroBaked = false;
  const idle = (f) => (globalThis.requestIdleCallback ?? ((g) => setTimeout(g, 50)))(f, { timeout: 1500 });
  const painted = () => new Promise((r) => requestAnimationFrame(() => setTimeout(r))); // the next frame is on screen
  function bustsReset() {
    for (const u of busts.values()) URL.revokeObjectURL(u);
    (busts = new Map()), (bustsOf = world), (bustQ = []), bustWant.clear();
  }
  // After a map loads: queue its people, nearest first, and bake them one per idle slot.
  function bakeBusts(s) {
    bustsReset();
    // Only where shaders compile off the page's thread: without KHR_parallel_shader_compile (software GL, CI)
    // each bake stalls the page for seconds, mid-walk or under a menu, so a bust is drawn when its talk opens.
    if (!renderer.extensions.has("KHR_parallel_shader_compile")) return;
    const far = (n) => (n.hidden ? 1e9 : Math.hypot(n.x - s.x, n.z - s.z));
    bustQ = world.npcs.filter((n) => n.obj).sort((a, b) => far(a) - far(b)).map((n) => n.def.id);
    // The hero first, once a visit (the outfit worn now): a busy map's people can take longer than a walk
    // to the next duel. A talk still jumps the queue.
    if (!heroBaked) bustQ.unshift(HERO_BUST);
    idle(bakeNext);
  }
  async function bakeNext() {
    if (baking || !alive || warping || !bustQ.length || bustsOf !== world) return; // a warp's end queues the next map's
    baking = true;
    const w = world, id = bustQ.shift();
    let url = "";
    try {
      url = await bake(id === HERO_BUST ? hero : w.npcs.find((p) => p.def.id === id));
    } catch (e) {
      console.warn("world: no bust for", id, e);
    }
    baking = false;
    if (!alive) return void (url && URL.revokeObjectURL(url));
    if (id === HERO_BUST && url) {
      // The hero's own face, for the duel table (#177): kept past the world, the last one let go.
      if (heroBust) URL.revokeObjectURL(heroBust);
      (heroBust = url), (heroBaked = true), cb("hero_bust", url, "");
    } else if (bustsOf !== w) url && URL.revokeObjectURL(url); // baked for the map just left
    else {
      if (url) busts.set(id, url);
      if (bustWant.delete(id) && url) cb("bust", id, url); // else the placeholder stays
    }
    if (bustQ.length) bustWant.size ? bakeNext() : idle(bakeNext); // a talk waiting: at once
  }
  async function bake(n) {
    if (!n?.obj) return "";
    const s = new THREE.Scene();
    const key = new THREE.DirectionalLight("#fff4dc", 2.6), rim = new THREE.DirectionalLight("#cfe0ff", 1.8);
    key.position.set(-1.5, 2.5, 3);
    rim.position.set(2, 1.5, -2.5);
    const o = cloneSkinned(n.obj); // in the pose they hold then
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
    cam.layers.enableAll(); // the hero lives on layer 1 (no sun shadow)
    cam.position.set(0, cy, 0).addScaledVector(dir, h / 2 / Math.tan(THREE.MathUtils.degToRad(11)));
    cam.lookAt(0, cy, 0);
    // An sRGB target: the same colours a canvas showed. Shaders are built for it (the target
    // set while they compile), in parallel where the browser can.
    bustRT ??= new THREE.WebGLRenderTarget(BUST, BUST, { samples: 4, colorSpace: THREE.SRGBColorSpace });
    renderer.setRenderTarget(bustRT);
    const ready = renderer.compileAsync(s, cam);
    renderer.setRenderTarget(null);
    await ready;
    await painted(); // a talk opened meanwhile shows first (software GL links shaders on first draw: slow)
    if (!alive || warping || bustsOf !== world) return ""; // never in a warp (the next map's busts come after it), nor once the world is gone
    const sky = renderer.getClearColor(new THREE.Color()), skyA = renderer.getClearAlpha();
    renderer.setRenderTarget(bustRT);
    renderer.setClearColor(0x000000, 0);
    renderer.clear();
    renderer.render(s, cam);
    renderer.setRenderTarget(null);
    renderer.setClearColor(sky, skyA);
    o.traverse((m) => m.skeleton?.dispose()); // the clone's bone texture
    const px = new Uint8Array(BUST * BUST * 4);
    await renderer.readRenderTargetPixelsAsync(bustRT, 0, 0, BUST, BUST, px);
    // GL rows run bottom-up and the edges come premultiplied: flipped and divided out for a 2D canvas.
    const img = new ImageData(BUST, BUST), d = img.data, row = BUST * 4;
    for (let y = 0; y < BUST; y++)
      for (let i = 0, f = (BUST - 1 - y) * row, t = y * row; i < row; i += 4) {
        const a = px[f + i + 3], k = a ? 255 / a : 0;
        d[t + i] = px[f + i] * k;
        d[t + i + 1] = px[f + i + 1] * k;
        d[t + i + 2] = px[f + i + 2] * k;
        d[t + i + 3] = a;
      }
    const cv = document.createElement("canvas");
    cv.width = cv.height = BUST;
    cv.getContext("2d").putImageData(img, 0, 0);
    const blob = await new Promise((r) => cv.toBlob(r, "image/webp", 0.85));
    return blob ? URL.createObjectURL(blob) : "";
  }
  // The face for a talk: the bust's URL, "pending" while it is baking, "" for no one on this map.
  function portrait(id) {
    const n = world?.npcs.find((p) => p.def.id === id);
    if (!n?.obj) return "";
    if (bustsOf !== world) bustsReset();
    if (busts.has(id)) return busts.get(id);
    bustQ = [id, ...bustQ.filter((q) => q !== id)];
    bustWant.add(id);
    painted().then(bakeNext); // the talk shows first
    return "pending";
  }

  const stop = () => {
    alive = false;
    globalThis.sixfoldAudio?.ambience("");
    bustsReset();
    bustRT?.dispose();
    stopEditor();
    stopDebug();
    previewStop();
    book.dispose();
    cancelAnimationFrame(raf);
    ro.disconnect();
    mq.removeEventListener("change", onMotion);
    removeEventListener("keydown", onKey);
    removeEventListener("keydown", onEsc, true);
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
    setCamera,
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
        clock.update();
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
    // The map's seal stone on screen, [x, y] in CSS pixels: where a seal won on stage starts its flight (#155).
    stoneAt() {
      const s = world?.stone;
      if (!s) return [];
      const v = world.root.localToWorld(s.at.clone()).project(camera), r = el.getBoundingClientRect();
      return [r.left + ((v.x + 1) / 2) * r.width, r.top + ((1 - v.y) / 2) * r.height];
    },
    cue(json) {
      const c = JSON.parse(json);
      if (c.end) return void (world && cut.end(c.end));
      if (c.caption) return void showCaption(c.caption, 2800); // a seal won on stage, said on screen (#155)
      if (c.warp && c.id === undefined) {
        const { map, x, z, facing } = c.warp;
        if (world && !warping) warp({ to: map, spawn: { x, z, facing }, travel: !!c.travel });
        return;
      }
      if (world) cut.play(c); // one cue player (script.js), for dialogue and cutscenes alike
      else if (c.id !== undefined) cb("cue_done", String(c.id), "");
    },
    // The seals held, as colour keys (`water,grass`): those faces of any seal stone stay lit (#153).
    setSeals(csv) {
      sealsHeld.clear();
      for (const c of csv.split(",")) if (c) sealsHeld.add(c);
      if (world?.stone) world.stone.dirty = true;
    },
    setItems(list) {
      canRun = list.split(",").includes("running_shoes");
      run.hidden = !canRun;
    },
    preview,
    previewStop,
    portrait,
    replay: (id) => !!world && !warping && cut.replay(id),
  };
}

