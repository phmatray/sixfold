// The overworld test: tile-grid maps (./maps/<name>.json) drawn with three.js.
// `mount(host, controls, cb, start)` builds it (at `start` = "map,x,z,facing"
// when given) and returns an api for Leptos (stop, act, pause, setItems,
// preview, previewStop). `cb(event, a, b)` reports:
//   ("near", kind, text)   what the hero now faces ("" when nothing)
//   ("use", kind, text)    acted on a sign, the duel table or a locked door
//   ("at", map, "x,z,f")   the hero stands on a new tile or faces a new way
//   ("check", cond, "")    asks whether a gated warp's `if` holds (returns a bool)
//   ("talk", npcId, json)  spoke to an NPC: {"name", "dialogue"} for Leptos to run
//   ("ready", "", "")      first frame drawn;  ("error", "", message)
// Game rules (progression, dialogue effects) live in the engine crate; this file
// only moves and draws. Grid: 1 tile = 1 m; tile (x, z) covers [x, x+1] × [z, z+1].
import * as THREE from "three";
import { GLTFLoader } from "three/addons/loaders/GLTFLoader.js";
import { clone as cloneSkinned } from "three/addons/utils/SkeletonUtils.js";
import { U, windify, ambience } from "./life.js";
import { groundMesh } from "./ground.js";
import { rng, mesh, group, PLACEHOLDER, FOOT, WALKABLE, WIND_H, AUTOTILE, armsPiece, prep } from "./props.js";

const BASE = new URL("./", import.meta.url);
const INTERIOR = new Set(["counter", "shelf", "duel_table", "chair", "rug", "plant", "display_case", "lamp", "exit_mat"]);
const BATCHED = new Set(["tree", "bush", "flowers", "rock"]); // instanced: static, never animated
const STEP = 0.25, RUN_STEP = 0.125, NPC_STEP = 0.45; // seconds per tile
const TURN_SPEED = Math.PI / 2 / 0.12; // a quarter turn in 0.12 s
const TURN_HOLD = 0.12; // a press shorter than this only turns the hero
const FIDGET_AFTER = 6; // seconds idle before the hero fidgets and the camera leans in
const FADE = 350; // ms, warp fade
const DIR = { n: [0, -1], s: [0, 1], e: [1, 0], w: [-1, 0] }; // n = away from the camera
const BACK = { n: "s", s: "n", e: "w", w: "e" };
const KEYS = { w: "n", arrowup: "n", s: "s", arrowdown: "s", a: "w", arrowleft: "w", d: "e", arrowright: "e" };
const angle = (f) => Math.atan2(DIR[f][0], DIR[f][1]);
const key = (x, z) => `${x},${z}`;
const SUFFIX_ARMS = { "": [[-1, 0], [1, 0]], _corner: [[-1, 0], [0, 1]], _end: [[-1, 0]], _t: [[-1, 0], [1, 0], [0, 1]] }; // rot-0 arms
const SIDES = [[0, -1], [1, 0], [0, 1], [-1, 0]]; // N E S W, the mask bit order

// ---------- Loading (cached for the session) ----------

const loader = new GLTFLoader();
const assets = new Map();
const PERSON = /^(npc|rival)_/; // models in assets/npc/
const folder = (n) => (INTERIOR.has(n) || n.startsWith("wall") ? "interior/" : PERSON.test(n) ? "npc/" : "");
const loadAsset = (n) => {
  if (!assets.has(n)) assets.set(n, loader.loadAsync(new URL(`assets/${folder(n)}${n}.glb`, BASE).href).catch(() => null));
  return assets.get(n);
};
const maps = new Map();
const loadMap = (name) => {
  if (!maps.has(name))
    maps.set(name, fetch(new URL(`maps/${name}.json`, BASE)).then((r) => (r.ok ? r.json() : Promise.reject(new Error(`map ${name}: HTTP ${r.status}`)))));
  return maps.get(name);
};
const OPTIONAL = /_b$|_c$|_dark$/; // variants: used when present, never missed

// Every model a map can use, so they load in parallel before it is built.
function assetNames(map) {
  const names = new Set(["hero", ...map.props.map((p) => p.asset), ...(map.npcs ?? []).map((n) => n.model).filter(Boolean)]);
  const tiles = map.tiles.join("");
  const pieces = (kind) => Object.keys(SUFFIX_ARMS).forEach((s) => names.add(kind + s));
  if (tiles.includes("f")) pieces("fence");
  if (/[WV]/.test(tiles)) pieces("wall"), pieces("wall_low"), names.add("wall_window");
  if (!map.light.indoor) ["tree_b", "tree_c", "bush_dark", "grass_edge"].forEach((n) => names.add(n));
  return [...names];
}

// ---------- Mount ----------

// Sounds come from web/audio/audio.js (loaded with the wasm), when it is there.
const sound = (name, n = 0) => globalThis.sixfoldAudio?.sfx(name, n);

export function mount(el, controls, cb, start) {
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
  renderer.domElement.setAttribute("aria-hidden", "true");
  el.appendChild(renderer.domElement);

  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(40, 1, 0.5, 80); // tight planes: depth precision
  // Where scenery hides the hero, draw it as a flat silhouette (depth test reversed).
  const silhouette = new THREE.MeshBasicMaterial({ color: "#f3d589", transparent: true, opacity: 0.55, depthFunc: THREE.GreaterDepth, depthWrite: false, fog: false });
  let cam = { fov: 40, fovPortrait: 50, pitch: 50, distance: 12, distancePortrait: 15, lookAtOffset: [0, 0.75, 0] };
  let dist = 12, alive = true, raf = 0, paused = false, warping = false, world = null, hero = null;
  let canRun = false, runToggle = false, shift = false, lastHero = "";
  const clock = new THREE.Clock();
  const look = new THREE.Vector3(), lookVel = new THREE.Vector3(), aim = new THREE.Vector3();
  let first = true, zoom = 1, readySent = false, drawnPaused = false;
  const resize = () => {
    const w = el.clientWidth || 1, h = el.clientHeight || 1;
    renderer.setSize(w, h);
    camera.aspect = w / h;
    camera.fov = w < h ? cam.fovPortrait : cam.fov; // portrait phones: pull back and widen
    dist = w < h ? cam.distancePortrait : cam.distance;
    camera.updateProjectionMatrix();
  };
  const ro = new ResizeObserver(resize);
  ro.observe(el);

  // ---------- DOM: D-pad and RUN (in `controls`, after the header), fade, grid legend ----------
  const arrow = '<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path d="M12 5l8 11H4z" fill="currentColor"/></svg>';
  const pad = document.createElement("div");
  pad.className = "world-pad";
  pad.setAttribute("role", "group");
  pad.setAttribute("aria-label", "Walk");
  for (const [f, label] of [["n", "Up"], ["w", "Left"], ["e", "Right"], ["s", "Down"]]) {
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
  run.textContent = "Run";
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
  legend.innerHTML = '<span class="k-block">Blocked</span><span class="k-act">Look or use</span><span class="k-warp">Door</span><span class="k-npc">Person</span>';
  el.append(fade, legend);

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
  const busy = () => paused || warping || !world;
  const onKey = (e) => {
    const k = e.key.toLowerCase();
    if (e.type === "keyup") {
      if (k === "shift") shift = false;
      return release(k);
    }
    if (k === "shift") return void (shift = true);
    if (busy() || e.target.closest?.("input, select, textarea, dialog")) return;
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

  // Tap/click the ground: walk the shortest grid path there (BFS).
  const ray = new THREE.Raycaster(), ndc = new THREE.Vector2(), hit = new THREE.Vector3();
  const floor = new THREE.Plane(new THREE.Vector3(0, 1, 0), 0);
  const onTap = (e) => {
    if (busy()) return;
    const b = renderer.domElement.getBoundingClientRect();
    ndc.set(((e.clientX - b.left) / b.width) * 2 - 1, -((e.clientY - b.top) / b.height) * 2 + 1);
    ray.setFromCamera(ndc, camera);
    if (ray.ray.intersectPlane(floor, hit)) world.path = route(world, hero, Math.floor(hit.x), Math.floor(hit.z));
  };
  renderer.domElement.addEventListener("pointerdown", onTap);

  // ---------- Moving and acting ----------

  // A warp with an `if` (e.g. "seals:6") opens only while it holds.
  const opens = (wp) => !wp.if || cb("check", wp.if, "") === true;

  // What the hero faces: a person (also across a counter or a duel table), a
  // sign or table, or a door (a locked one reads like a sign).
  function facing() {
    const w = world, [dx, dz] = DIR[hero.face], fx = hero.x + dx, fz = hero.z + dz;
    const across = ["counter", "duel_table"].includes(w.propAt.get(key(fx, fz))?.asset);
    const npc = w.npcAt(fx, fz) ?? (across ? w.npcAt(fx + dx, fz + dz) : null);
    if (npc) return { kind: "npc", text: npc.def.name, npc };
    const it = w.talk.get(key(fx, fz));
    if (it) return it;
    const wp = w.warps.get(key(fx, fz));
    if (wp && !wp.dir) return opens(wp) ? { kind: "warp", text: wp.label ?? "Go in", warp: wp } : { kind: "locked", text: wp.locked ?? "It won't open." };
    return null;
  }

  // Start a step toward `f`. A door warp is entered even though its tile is solid;
  // pushing against the wall from a warp tile with that `dir` leaves the room.
  function tryStep(f, carry) {
    const w = world, x = hero.x + DIR[f][0], z = hero.z + DIR[f][1];
    hero.face = f;
    const door = w.warps.get(key(x, z));
    if (door && !door.dir && opens(door)) return startStep(x, z, carry, door);
    if (w.blocked(x, z) || w.npcAt(x, z)) {
      const here = w.warps.get(key(hero.x, hero.z));
      if (here?.dir === f) warp(here);
      return false;
    }
    return startStep(x, z, carry, null);
  }
  function startStep(x, z, carry, door) {
    const dur = canRun && (shift || runToggle) ? RUN_STEP : STEP;
    hero.step = { x, z, t: carry / dur, dur, warp: door };
    return true;
  }

  function act() {
    if (busy() || hero.step) return;
    const it = facing();
    if (!it) return;
    held.length = 0;
    if (it.kind === "npc") {
      const n = it.npc;
      n.busy = true;
      n.face = BACK[hero.face]; // turn to the hero, then speak
      setTimeout(() => alive && cb("talk", n.def.id, JSON.stringify({ name: n.def.name, dialogue: n.def.dialogue })), reduced ? 0 : 160);
    } else if (it.kind === "warp") tryStep(hero.face, 0);
    else {
      if (it.obj && !reduced) it.obj.userData.wobble = 1;
      cb("use", it.kind, it.text);
    }
  }

  async function warp(wp) {
    if (warping) return;
    warping = true;
    sound("door");
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

  // Load a map (and its models), build it, and put the hero on `spawn`.
  async function enter(name, spawn) {
    const map = await loadMap(name);
    const names = assetNames(map);
    const loaded = await Promise.all(names.map(loadAsset));
    if (!alive) return;
    const got = Object.fromEntries(names.map((n, i) => [n, loaded[i]]));
    const missing = names.filter((n) => !got[n] && !OPTIONAL.test(n) && !PERSON.test(n)); // NPCs fall back to a tinted hero
    if (missing.length) console.info(`world (${name}): placeholders for ${missing.join(", ")}`);
    if (!hero) {
      hero = actor(got.hero, null, 1, "idle");
      hero.obj.traverse((o) => o.layers.set(1)); // drawn in its own passes (see frame)
      scene.add(hero.obj, hero.blob);
    }
    const next = await build(map, got, reduced);
    if (!alive) return;
    next.name = name;
    if (world) dispose(world.root);
    world = next;
    world.hero = hero;
    scene.add(world.root);
    const L = map.light;
    renderer.setClearColor(L.background);
    scene.fog = L.fog ? new THREE.Fog(L.background, L.fog.near, L.fog.far) : null;
    el.closest(".world")?.style.setProperty("--world-bg", L.background);
    globalThis.sixfoldAudio?.music(L.indoor ? "interior" : "overworld");
    const s = spawn && !next.blocked(spawn.x, spawn.z) && !next.npcAt(spawn.x, spawn.z) ? spawn : map.spawn; // a stale saved tile falls back
    Object.assign(hero, { x: s.x, z: s.z, face: s.facing, step: null, idle: 0, turnedAt: -Infinity }); // a fresh map clock
    hero.obj.rotation.y = angle(s.facing);
    cam = map.camera;
    resize();
    first = true;
    await renderer.compileAsync(scene, camera).catch(() => {});
  }

  function fail(e) {
    console.error("world: could not build the scene", e);
    cb("error", "", String(e?.message ?? e));
  }

  // Resume at a saved "map,x,z,facing"; a map that no longer loads falls back to the overworld.
  const [sm, sx, sz, sf] = (start ?? "").split(",");
  const at = sm && DIR[sf] && Number.isInteger(+sx) && Number.isInteger(+sz) ? { x: +sx, z: +sz, facing: sf } : null;
  (at ? enter(sm, at).catch(() => enter("overworld")) : enter("overworld"))
    .then(() => {
      if (!alive) return;
      clock.getDelta();
      raf = requestAnimationFrame(frame);
    })
    .catch(fail);

  // ---------- Loop ----------
  function frame() {
    raf = requestAnimationFrame(frame);
    const dt = Math.min(clock.getDelta(), 0.05);
    if (paused) {
      // A modal is open: draw one last frame, then nothing until it closes.
      if (drawnPaused) return;
      drawnPaused = true;
    } else drawnPaused = false;
    const w = world, h = hero;
    w.now += dt;
    U.uTime.value = w.now;
    if (!reduced) U.uDrift.value = w.now;
    const prevStep = h.step;

    // Finish (or carry on) the current step.
    let carry = 0, chained = false;
    if (h.step) {
      h.step.t += dt / h.step.dur;
      if (h.step.t >= 1) {
        const done = h.step;
        carry = (done.t - 1) * done.dur;
        h.x = done.x;
        h.z = done.z;
        h.step = null;
        chained = true;
        const wp = done.warp ?? w.warps.get(key(h.x, h.z));
        if (wp) warp(wp);
      }
    }
    if (!h.step && !warping && !paused) {
      const k = held[held.length - 1];
      if (k) {
        const f = dirOf(k);
        if (f !== h.face) {
          h.face = f;
          h.turnedAt = w.now;
          if (chained) tryStep(f, carry); // already walking: turn and keep going
        } else if (chained || w.now - h.turnedAt >= TURN_HOLD) tryStep(f, carry);
      } else if (w.path.length) {
        if (!tryStep(w.path.shift(), carry)) w.path = [];
      }
    }
    if (h.step && h.step !== prevStep) {
      w.life.puff(h.x + 0.5, h.z + 0.5); // footstep dust
      sound("step", w.map.tiles[h.step.z]?.[h.step.x] === "w" ? 1 : 0); // "w": a wooden floor
    }
    h.idle = h.step || held.length ? 0 : h.idle + dt;
    h.running = !!h.step && h.step.dur < STEP;

    const [px, pz] = place(h, dt);
    animateActor(h, dt, reduced);
    for (const n of w.npcs) updateNpc(w, n, dt, paused);
    animateProps(w, dt, reduced);
    w.life.update(dt, w.now, px, pz);

    const pos = `${h.x},${h.z},${h.face}`;
    if (pos !== lastHero) (el.dataset.hero = lastHero = pos), cb("at", w.name, pos); // written only on change

    // What the hero faces, while standing still.
    const front = h.step || warping ? null : facing();
    const nk = front ? `${front.kind}:${front.text}` : "";
    if (nk !== w.nearKey) {
      w.nearKey = nk;
      cb("near", front?.kind ?? "", front?.text ?? "");
    }
    if (w.door) w.door.want = front?.kind === "warp" || h.step?.warp ? 1 : 0;

    // Camera: fixed pitch, on a soft spring behind a point a little ahead of a
    // walking hero; leans in after a while idle; indoors, kept inside the room.
    const pitch = THREE.MathUtils.degToRad(cam.pitch), off = cam.lookAtOffset;
    const ahead = h.step && !reduced ? 0.7 : 0;
    aim.set(px + off[0] + DIR[h.face][0] * ahead, off[1], pz + off[2] + DIR[h.face][1] * ahead);
    if (cam.clamp) {
      // Keep the view inside the room: the ground it shows spans [aim + top, aim + bottom]
      // along z (perspective makes the far side longer) and ±halfW along x.
      const half = THREE.MathUtils.degToRad(camera.fov / 2), hc = dist * Math.sin(pitch) + off[1], d0 = dist * Math.cos(pitch);
      const top = pitch > half ? d0 - hc / Math.tan(pitch - half) : -Infinity, bottom = d0 - hc / Math.tan(pitch + half);
      const halfW = dist * Math.tan(half) * camera.aspect;
      // A room bigger than the view: the view stays inside it. Smaller: the
      // camera still follows the hero, but only as far as keeps the whole room
      // (and its back wall, about 1 m) in view.
      const fit = (v, lo, hi) => THREE.MathUtils.clamp(v, Math.min(lo, hi), Math.max(lo, hi));
      aim.x = fit(aim.x, halfW, w.map.width - halfW);
      if (Number.isFinite(top)) aim.z = fit(aim.z, -top - 1, w.map.height - bottom);
    }
    if (first || reduced) look.copy(aim), lookVel.set(0, 0, 0);
    else {
      const k = 28, c = 2 * Math.sqrt(k) * 0.85; // slightly under-damped
      lookVel.addScaledVector(aim.clone().sub(look), k * dt).addScaledVector(lookVel, -c * dt);
      look.addScaledVector(lookVel, dt);
    }
    first = false;
    zoom += ((h.idle > FIDGET_AFTER && !reduced ? 0.88 : 1) - zoom) * Math.min(1, dt * 1.5);
    const dz = dist * zoom;
    camera.position.set(look.x, look.y + Math.sin(pitch) * dz, look.z + Math.cos(pitch) * dz);
    camera.lookAt(look);
    // Outdoors the sun (and its shadow map) follows the hero a whole tile at a
    // time: following the tweened position makes shadow edges shimmer.
    if (!w.map.light.indoor) {
      const from = w.map.light.sun.fromHero;
      w.sun.position.set(h.x + from[0], from[1], h.z + from[2]);
      w.sun.target.position.set(h.x, 0, h.z);
    }

    // Three passes: the scene without the hero; the hero's silhouette where
    // scenery hides it; then the hero itself where it shows.
    renderer.shadowMap.needsUpdate = true;
    renderer.clear();
    camera.layers.set(0);
    renderer.render(scene, camera);
    camera.layers.set(1);
    scene.overrideMaterial = silhouette;
    renderer.render(scene, camera);
    scene.overrideMaterial = null;
    renderer.render(scene, camera);
    camera.layers.set(0);
    if (!readySent) (readySent = true), cb("ready", "", "");
  }

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

  const stop = () => {
    alive = false;
    previewStop();
    cancelAnimationFrame(raf);
    ro.disconnect();
    mq.removeEventListener("change", onMotion);
    removeEventListener("keydown", onKey);
    removeEventListener("keyup", onKey);
    removeEventListener("blur", clearHeld);
    dispose(scene);
    renderer.dispose();
    renderer.domElement.remove();
    for (const n of [pad, run, fade, legend]) n.remove();
  };
  return {
    stop,
    act,
    pause(p) {
      paused = !!p;
      clearHeld();
      if (world) world.path = [];
      if (!paused) {
        clock.getDelta();
        world?.npcs.forEach((n) => (n.busy = false));
      }
    },
    setItems(list) {
      canRun = list.split(",").includes("running_shoes");
      run.hidden = !canRun;
    },
    preview,
    previewStop,
  };
}

// Free GPU memory under `root`; cached model geometry (instanced) is kept.
function dispose(root) {
  root.parent?.remove(root);
  root.traverse((o) => {
    o.userData.dispose?.();
    if (o.userData.shared) return;
    o.geometry?.dispose();
    [o.material].flat().forEach((m) => m?.dispose());
  });
}

// Shortest 4-way path from the hero to (tx, tz). A blocked target (a sign, a
// person, the door) routes to a free neighbour and ends facing it.
function route(w, hero, tx, tz) {
  const { width: W, height: H } = w.map;
  if (tx === hero.x && tz === hero.z) return [];
  const solid = (x, z) => w.blocked(x, z) || !!w.npcAt(x, z);
  const goal = solid(tx, tz) ? Object.values(DIR).map(([dx, dz]) => [tx - dx, tz - dz]).filter(([x, z]) => !solid(x, z)) : [[tx, tz]];
  if (!goal.length) return [];
  const prev = new Map([[key(hero.x, hero.z), null]]);
  const queue = [[hero.x, hero.z]];
  while (queue.length) {
    const [x, z] = queue.shift();
    if (goal.some(([gx, gz]) => gx === x && gz === z)) {
      const dirs = [];
      for (let k = key(x, z); prev.get(k); k = prev.get(k).from) dirs.unshift(prev.get(k).f);
      if (solid(tx, tz)) dirs.push(Object.keys(DIR).find((f) => x + DIR[f][0] === tx && z + DIR[f][1] === tz));
      return dirs;
    }
    for (const [f, [dx, dz]] of Object.entries(DIR)) {
      const nx = x + dx, nz = z + dz, k = key(nx, nz);
      if (nx < 0 || nz < 0 || nx >= W || nz >= H || prev.has(k) || solid(nx, nz)) continue;
      prev.set(k, { from: key(x, z), f });
      queue.push([nx, nz]);
    }
  }
  return [];
}

// ---------- Actors (the hero and NPCs share the hero model) ----------

function radial(inner, outer) {
  const c = document.createElement("canvas");
  c.width = c.height = 64;
  const g = c.getContext("2d");
  const grd = g.createRadialGradient(32, 32, 0, 32, 32, 32);
  grd.addColorStop(0, inner);
  grd.addColorStop(1, outer);
  g.fillStyle = grd;
  g.fillRect(0, 0, 64, 64);
  return new THREE.CanvasTexture(c);
}
const blobTex = radial("rgba(20,30,15,0.42)", "rgba(20,30,15,0)");
const glowTex = radial("rgba(255,255,255,0.9)", "rgba(255,255,255,0)");
// Soft contact shadow, 1.5 cm above the floor and depth-offset (never coplanar).
const blobMat = () => new THREE.MeshBasicMaterial({ map: blobTex, transparent: true, depthWrite: false, polygonOffset: true, polygonOffsetFactor: -2, polygonOffsetUnits: -2 });
const blobGeo = new THREE.PlaneGeometry(1, 1).rotateX(-Math.PI / 2);

function actor(src, tint, scale, idle) {
  const obj = src ? cloneSkinned(src.scene) : PLACEHOLDER.hero();
  prep(obj, true);
  if (tint) {
    const c = new THREE.Color(tint);
    obj.traverse((o) => o.isMesh && ((o.material = o.material.clone()), o.material.color.lerp(c, 0.45)));
  }
  obj.scale.multiplyScalar(scale);
  const blob = new THREE.Mesh(blobGeo, blobMat());
  blob.userData.shared = true;
  blob.scale.setScalar(0.8 * scale);
  const a = { obj, blob, base: obj.scale.x, mixer: null, actions: {}, clip: null, idle: 0, t: 0, turnedAt: -1, idleName: (idle ?? "idle").toLowerCase() };
  if (src?.animations.length) {
    a.mixer = new THREE.AnimationMixer(obj);
    // The first clip of each name wins: exports can carry duplicates.
    for (const clip of src.animations) a.actions[clip.name.toLowerCase()] ??= a.mixer.clipAction(clip);
    a.actions[a.idleName]?.play();
    a.clip = a.idleName;
  }
  return a;
}

// Put an actor on its tile, or between two while stepping; turn toward its facing.
function place(a, dt) {
  const t = a.step ? a.step.t : 0;
  const px = a.x + 0.5 + (a.step ? (a.step.x - a.x) * t : 0);
  const pz = a.z + 0.5 + (a.step ? (a.step.z - a.z) * t : 0);
  a.obj.position.set(px, a.obj.position.y, pz);
  a.blob.position.set(px, 0.015, pz);
  const want = angle(a.face), cur = a.obj.rotation.y;
  const turn = Math.atan2(Math.sin(want - cur), Math.cos(want - cur));
  a.obj.rotation.y = cur + Math.sign(turn) * Math.min(Math.abs(turn), TURN_SPEED * dt);
  return [px, pz];
}

function animateActor(a, dt, reduced) {
  const walking = !!a.step;
  a.t += dt;
  if (a.mixer) {
    const want = walking ? (a.running && a.actions.run ? "run" : "walk") : a.idleName;
    if (want !== a.clip && a.actions[want]) {
      a.actions[a.clip]?.fadeOut(0.15);
      a.actions[want].reset().fadeIn(0.15).play();
      a.clip = want;
    }
    if (a.actions.walk) a.actions.walk.timeScale = a.running && !a.actions.run ? 2 : 1; // no Run clip: a faster Walk
    // A fidget clip once the hero has stood still a while, if the model has one.
    const fidget = a.actions.fidget ?? a.actions.look;
    if (fidget && a.idle > FIDGET_AFTER && !a.fidgeted) {
      a.fidgeted = true;
      fidget.reset().setLoop(THREE.LoopOnce, 1).fadeIn(0.2).play();
      a.mixer.addEventListener("finished", () => fidget.fadeOut(0.3));
    }
    if (a.idle === 0) a.fidgeted = false;
    a.mixer.update(dt);
  } else if (a.obj.userData.legs) {
    const swing = walking ? Math.sin((a.t * Math.PI) / a.step.dur) * 0.6 : 0; // one leg forward per tile
    const [l, r] = a.obj.userData.legs;
    l.rotation.x = swing;
    r.rotation.x = -swing;
  }
  // A small bounce per tile, squashed at each footfall.
  const up = walking && !reduced ? Math.sin(Math.PI * a.step.t) : 0;
  const squash = walking && !reduced ? 1 - up : 0;
  a.obj.position.y = up * 0.05;
  a.obj.scale.set(a.base * (1 + squash * 0.04), a.base * (1 - squash * 0.06 + up * 0.02), a.base * (1 + squash * 0.04));
}

// NPCs idle, or walk their `route` (one direction per step) when it is free.
function updateNpc(w, n, dt, paused) {
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
      if (w.blocked(x, z) || (h.x === x && h.z === z) || (h.step?.x === x && h.step?.z === z) || w.npcAt(x, z, n)) n.wait = 0.6;
      else {
        n.step = { x, z, t: 0, dur: NPC_STEP };
        n.ri++;
        n.wait = n.ri % n.def.route.length === 0 ? 2.5 : 0.35;
      }
    }
  }
  place(n, dt);
  animateActor(n, dt, false);
}

// The house door easing open or shut, and a sign wobbling after it was read.
function animateProps(w, dt, reduced) {
  const d = w.door;
  if (d) {
    d.open = reduced ? d.want : d.open + (d.want - d.open) * Math.min(1, dt * 7);
    if (d.action) {
      d.action.time = d.open * d.action.getClip().duration;
      d.mixer.update(0);
    } else if (d.node) d.node.rotation.y = d.rest + d.open * 1.5;
  }
  for (const it of w.talk.values()) {
    const o = it.obj;
    if (!o?.userData.wobble) continue;
    o.userData.wobble = Math.max(0, o.userData.wobble - dt * 1.4);
    o.rotation.z = Math.sin(w.now * 22) * 0.07 * o.userData.wobble;
  }
}

// ---------- Building a map ----------

// Static props of one kind become InstancedMeshes: one draw per mesh of the
// model, not one per prop. `add` records a placement; `build` makes the meshes.
function batcher() {
  const kinds = new Map();
  return {
    add(name, template, matrix, color) {
      if (!kinds.has(name)) kinds.set(name, { template, list: [] });
      kinds.get(name).list.push({ matrix, color });
    },
    build(parent, windH) {
      const white = new THREE.Color("#ffffff"), tmp = new THREE.Matrix4();
      for (const [name, { template, list }] of kinds) {
        prep(template, true);
        template.updateMatrixWorld(true);
        const tinted = list.some((p) => p.color);
        template.traverse((m) => {
          if (!m.isMesh) return;
          const inst = new THREE.InstancedMesh(m.geometry, m.material, list.length);
          list.forEach(({ matrix, color }, i) => {
            inst.setMatrixAt(i, tmp.multiplyMatrices(matrix, m.matrixWorld));
            if (tinted) inst.setColorAt(i, color ?? white);
          });
          inst.castShadow = inst.receiveShadow = true;
          inst.userData.shared = true; // geometry and material belong to the cached model
          if (windH(name)) windify(inst, windH(name));
          parent.add(inst);
        });
      }
    },
  };
}

async function build(map, got, reduced) {
  const { width: W, height: H } = map;
  const L = map.light;
  const root = new THREE.Group();
  const r = rng(W * 131 + H);

  // Lights. Every light also lights layer 1 (the hero's own passes).
  const hemi = new THREE.HemisphereLight(L.hemisphere.sky, L.hemisphere.ground, L.hemisphere.intensity);
  const sun = new THREE.DirectionalLight(L.sun.color, L.sun.intensity);
  sun.castShadow = true;
  sun.shadow.mapSize.set(2048, 2048);
  const ext = L.indoor ? 9 : 16;
  Object.assign(sun.shadow.camera, { left: -ext, right: ext, top: ext, bottom: -ext, near: 1, far: 60 });
  sun.shadow.camera.layers.enableAll();
  sun.shadow.bias = -0.0005;
  sun.shadow.normalBias = 0.03;
  if (L.indoor) {
    // Window light from a fixed side of the room.
    sun.position.set(W / 2 + L.sun.fromHero[0], L.sun.fromHero[1], H / 2 + L.sun.fromHero[2]);
    sun.target.position.set(W / 2, 0, H / 2);
  }
  root.add(hemi, sun, sun.target);

  // Collision: "#" rock, "f" fence, "W" wall and "V" window tiles, plus prop footprints.
  const solid = new Uint8Array(W * H);
  const tile = (x, z) => map.tiles[z]?.[x];
  map.tiles.forEach((row, z) => [...row].forEach((c, x) => (solid[z * W + x] = "#fWV".includes(c) ? 1 : 0)));
  const blocked = (x, z) => x < 0 || z < 0 || x >= W || z >= H || solid[z * W + x] === 1;

  root.add(await groundMesh(map, (x, z) => tile(x, z) === "p"));

  const batch = batcher();
  const blobs = [];
  const BLOB = { tree: 2.2, bush: 1.1, flowers: 0.95, rock: 1.0, sign: 0.6 };
  const at = (x, z, rot, s = 1) =>
    new THREE.Matrix4().compose(new THREE.Vector3(x, 0, z), new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(0, 1, 0), rot), new THREE.Vector3(s, s, s));
  const templates = new Map();
  const tpl = (n, make) => {
    if (!templates.has(n)) templates.set(n, got[n] ? got[n].scene : make ? make() : (PLACEHOLDER[n] ?? (() => group()))());
    return templates.get(n);
  };
  const variants = (n) => [n, `${n}_b`, `${n}_c`].filter((v) => v === n || got[v]);
  const tint = (base, spread) => new THREE.Color(base).offsetHSL((r() - 0.5) * spread, 0, (r() - 0.5) * spread);

  // Props sit at their footprint's centre; rot 90/270 swaps the footprint.
  const props = [];
  const propAt = new Map();
  for (const p of map.props) {
    let [fw, fd] = FOOT[p.asset] ?? [1, 1];
    if (p.rot % 180 === 90) [fw, fd] = [fd, fw];
    const cx = p.x + fw / 2, cz = p.z + fd / 2;
    let obj = null;
    if (BATCHED.has(p.asset)) {
      // Trees vary in model, size, tint and turn; the ones on the map edge sit half a tile out.
      const isTree = p.asset === "tree";
      const kinds = variants(p.asset);
      const name = isTree ? kinds[Math.floor(r() * kinds.length)] : p.asset;
      const ox = isTree ? (p.x === 0 ? -0.5 : p.x + fw === W ? 0.5 : 0) : 0;
      const oz = isTree ? (p.z === 0 ? -0.5 : p.z + fd === H ? 0.5 : 0) : 0;
      const s = isTree ? 0.88 + r() * 0.3 : 1;
      const rot = isTree ? r() * Math.PI * 2 : THREE.MathUtils.degToRad(p.rot);
      batch.add(name, tpl(name), at(cx + ox, cz + oz, rot, s), isTree ? tint("#ffffff", 0.06) : null);
      if (!got[name]) blobs.push([cx + ox, cz + oz, BLOB[p.asset] * s]); // models bake their own shadow disc
    } else {
      obj = got[p.asset] ? got[p.asset].scene.clone(true) : (PLACEHOLDER[p.asset] ?? (() => group()))(p.x * 31 + p.z);
      obj.position.set(cx, 0, cz);
      obj.rotation.y = THREE.MathUtils.degToRad(p.rot);
      prep(obj, true);
      root.add(obj);
      if (BLOB[p.asset] && !got[p.asset]) blobs.push([cx, cz, BLOB[p.asset]]);
    }
    const rec = { ...p, obj, cx, cz };
    props.push(rec);
    for (const i of map.interactions) if (i.x >= p.x && i.x < p.x + fw && i.z >= p.z && i.z < p.z + fd) i.obj = obj;
    for (let x = p.x; x < p.x + fw; x++)
      for (let z = p.z; z < p.z + fd; z++) {
        propAt.set(key(x, z), rec);
        if (!WALKABLE.has(p.asset)) solid[z * W + x] = 1;
      }
  }

  // Fences ("f") and walls ("W", or "V" for a wall with a window) auto-tile from
  // their neighbours; walls on the south (camera-facing) row use the low cutaway pieces.
  const joins = (c, x, z) => (c === "f" ? tile(x, z) === "f" : "WV".includes(tile(x, z) ?? "-"));
  for (let z = 0; z < H; z++)
    for (let x = 0; x < W; x++) {
      const c = tile(x, z);
      if (!"fWV".includes(c ?? "-")) continue;
      const kind = c === "f" ? "fence" : z === H - 1 ? "wall_low" : "wall";
      const mask = SIDES.reduce((m, [dx, dz], i) => m | (joins(c, x + dx, z + dz) ? 1 << i : 0), 0);
      for (const [sfx, deg] of [AUTOTILE[mask], ...(mask === 15 ? [["_end", 270]] : [])]) {
        const n = c === "V" && sfx === "" ? "wall_window" : kind + sfx; // a window only fits a straight run
        batch.add(n, tpl(n, () => armsPiece(n === "wall_window" ? n : kind, SUFFIX_ARMS[sfx])), at(x + 0.5, z + 0.5, THREE.MathUtils.degToRad(deg)));
      }
    }

  const open = (x, z) => !blocked(x, z) && tile(x, z) !== "p";
  if (!L.indoor) {
    scatter(root, W, H, open);
    // Grass tufts along the path edges.
    for (let z = 0; z < H; z++)
      for (let x = 0; x < W; x++) {
        if (!open(x, z)) continue;
        for (const [dx, dz] of SIDES) {
          if (tile(x + dx, z + dz) !== "p") continue;
          for (let k = 0; k < 2; k++) {
            const along = (r() - 0.5) * 0.8;
            const ex = x + 0.5 + dx * 0.4 + (dz ? along : 0), ez = z + 0.5 + dz * 0.4 + (dx ? along : 0);
            batch.add("grass_edge", tpl("grass_edge", PLACEHOLDER.grass_edge), at(ex, ez, r() * Math.PI * 2, 0.8 + r() * 0.4), tint("#ffffff", 0.08));
          }
        }
      }
    // A denser, darker band of trees beyond the map edge hides the empty plain.
    const outer = variants("tree").concat(got.bush_dark ? ["bush_dark"] : []);
    const ring = (x, z) => {
      const n = outer[Math.floor(r() * outer.length)];
      batch.add(n, tpl(n), at(x, z, r() * Math.PI * 2, 1.05 + r() * 0.45), tint("#b9c9a8", 0.05));
    };
    for (let x = -5; x <= W + 5; x += 1.6 + r() * 0.6) ring(x, -1.6 - r() * 2.2), ring(x, H + 1.4 + r() * 2.2);
    for (let z = -2; z <= H + 2; z += 1.6 + r() * 0.6) ring(-1.6 - r() * 2.2, z), ring(W + 1.6 + r() * 2.2, z);
  }
  batch.build(root, (n) => (n === "grass_edge" ? 0.35 : WIND_H[n.replace(OPTIONAL, "")] ?? 0));

  // Contact shadows under the scenery.
  if (blobs.length) {
    const inst = new THREE.InstancedMesh(blobGeo, blobMat(), blobs.length);
    blobs.forEach(([x, z, s], i) => inst.setMatrixAt(i, at(x, z, 0, s).setPosition(x, 0.015, z)));
    inst.userData.shared = true; // blobGeo is module-wide
    root.add(inst);
  }

  // Indoors: a warm light, and a glow, on every lamp.
  if (L.indoor && L.lamp) {
    for (const p of props.filter((q) => q.asset === "lamp")) {
      const light = new THREE.PointLight(L.lamp.color, L.lamp.intensity, L.lamp.distance, 1.6);
      p.obj.updateMatrixWorld(true);
      const origin = p.obj.getObjectByName("light_origin");
      if (origin) origin.getWorldPosition(light.position);
      else light.position.set(p.cx, 1.55, p.cz);
      const glow = new THREE.Sprite(new THREE.SpriteMaterial({ map: glowTex, color: L.lamp.color, transparent: true, opacity: 0.7, blending: THREE.AdditiveBlending, depthWrite: false }));
      glow.position.copy(light.position);
      glow.scale.setScalar(1.3);
      root.add(light, glow);
    }
  }
  root.traverse((o) => o.isLight && o.layers.enableAll());

  const talk = new Map(map.interactions.map((i) => [key(i.x, i.z), i]));
  const warps = new Map((map.warps ?? []).map((wp) => [key(wp.x, wp.z), wp]));

  // NPCs: their own model when it exists, else the hero model, tinted and scaled.
  const npcs = (map.npcs ?? []).map((def) => {
    const own = got[def.model];
    const n = own ? actor(own, null, 1, def.idle) : actor(got.hero, def.tint, def.scale ?? 1, def.idle);
    Object.assign(n, { def, x: def.x, z: def.z, face: def.facing ?? "s", step: null, ri: 0, wait: 1.5, busy: false });
    n.obj.rotation.y = angle(n.face);
    root.add(n.obj, n.blob);
    return n;
  });
  const npcAt = (x, z, except) => npcs.find((n) => n !== except && ((n.x === x && n.z === z) || (n.step && n.step.x === x && n.step.z === z))) ?? null;

  const grid = debugGrid(W, H, blocked, talk, warps, npcs);
  grid.visible = false;
  root.add(grid);

  // The house door: its "Open" clip when the model has one, else a node named "door"
  // swung on its origin (the hinge). `open` eases toward `want`, 0 shut to 1 open.
  const house = props.find((p) => p.asset === "house")?.obj;
  let door = null;
  if (house) {
    const clip = got.house?.animations.find((a) => a.name.toLowerCase() === "open");
    door = { open: 0, want: 0, node: house.getObjectByName("door") ?? null, mixer: null, action: null };
    if (clip) {
      door.mixer = new THREE.AnimationMixer(house);
      door.action = door.mixer.clipAction(clip);
      door.action.play();
      door.action.paused = true;
    }
    if (door.node) door.rest = door.node.rotation.y;
  }

  root.updateMatrixWorld(true);
  const smoke = house?.getObjectByName("smoke_origin")?.getWorldPosition(new THREE.Vector3()) ?? null;
  const flowers = props.filter((p) => p.asset === "flowers").filter((_, i) => i % 3 === 0).map((p) => ({ x: p.cx, z: p.cz }));
  const life = ambience(root, { flowers, smoke }, reduced);
  return { map, root, blocked, npcAt, propAt, talk, warps, npcs, grid, sun, door, life, path: [], now: 0, nearKey: null };
}

// G toggles this: tile lines; red = blocked, gold = look/use, blue = door, green = person.
function debugGrid(W, H, blocked, talk, warps, npcs) {
  const pts = [];
  for (let x = 0; x <= W; x++) pts.push(x, 0.04, 0, x, 0.04, H);
  for (let z = 0; z <= H; z++) pts.push(0, 0.04, z, W, 0.04, z);
  const geo = new THREE.BufferGeometry();
  geo.setAttribute("position", new THREE.Float32BufferAttribute(pts, 3));
  const g = group(new THREE.LineSegments(geo, new THREE.LineBasicMaterial({ color: "#ffffff", transparent: true, opacity: 0.7 })));
  const quad = new THREE.PlaneGeometry(0.86, 0.86).rotateX(-Math.PI / 2);
  const flat = { transparent: true, depthWrite: false, polygonOffset: true, polygonOffsetFactor: -4, polygonOffsetUnits: -4 };
  const mat = (color, opacity) => new THREE.MeshBasicMaterial({ color, opacity, ...flat });
  const red = mat("#ff2d55", 0.6), gold = mat("#ffd60a", 0.75), blue = mat("#3fa9ff", 0.75), green = mat("#34e07a", 0.75);
  for (let x = 0; x < W; x++)
    for (let z = 0; z < H; z++) {
      const k = key(x, z);
      const m = warps.has(k) ? blue : talk.has(k) ? gold : npcs.some((n) => n.def.x === x && n.def.z === z) ? green : blocked(x, z) ? red : null;
      if (m) g.add(mesh(quad, m, x + 0.5, 0.05, z + 0.5));
    }
  return g;
}

// Grass tufts and tiny flowers, instanced, only on open grass tiles.
function scatter(root, W, H, open) {
  const r = rng(99);
  const blade = new THREE.ConeGeometry(0.05, 0.32, 3);
  blade.translate(0, 0.16, 0);
  const tufts = new THREE.InstancedMesh(blade, new THREE.MeshLambertMaterial(), 1800);
  const bloom = new THREE.InstancedMesh(new THREE.SphereGeometry(0.06, 6, 4), new THREE.MeshLambertMaterial(), 200);
  const m = new THREE.Matrix4(), q = new THREE.Quaternion(), e = new THREE.Euler(), s = new THREE.Vector3(), v = new THREE.Vector3(), col = new THREE.Color();
  const spot = () => {
    for (;;) {
      const x = r() * W, z = r() * H;
      if (open(Math.floor(x), Math.floor(z))) return [x, z];
    }
  };
  for (let i = 0; i < tufts.count; ) {
    const [cx, cz] = spot();
    for (let k = 0; k < 8 && i < tufts.count; k++, i++) {
      e.set((r() - 0.5) * 0.6, r() * 6, (r() - 0.5) * 0.6);
      s.setScalar(0.7 + r() * 0.8);
      m.compose(v.set(cx + (r() - 0.5) * 0.35, 0, cz + (r() - 0.5) * 0.35), q.setFromEuler(e), s);
      tufts.setMatrixAt(i, m);
      tufts.setColorAt(i, col.set(r() < 0.5 ? "#3c9a2f" : "#6cc44a"));
    }
  }
  const petals = ["#ffffff", "#ffd2b8", "#fff1a0"];
  for (let j = 0; j < bloom.count; j++) {
    const [x, z] = spot();
    m.compose(v.set(x, 0.07, z), q.identity(), s.set(1, 0.6, 1));
    bloom.setMatrixAt(j, m);
    bloom.setColorAt(j, col.set(petals[Math.floor(r() * 3)]));
  }
  tufts.receiveShadow = bloom.receiveShadow = true;
  windify(tufts, 0.32);
  windify(bloom, 0.12);
  root.add(tufts, bloom);
}
