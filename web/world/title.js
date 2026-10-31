// The title's living backdrop: the real world, full-bleed. A slow cinematic camera
// drifts over a few recognisable places of the regions (SHOTS), by day and by night,
// cross-fading from one to the next. Each place is the world's own map, built by the
// world's own code (map.js, build.js, daylight.js), with its wind, water, smoke,
// lamps, people and fireflies; only the camera is the title's.
//
// It loads progressively: the flat CSS backdrop (`.title-backdrop.flat` look, under
// the canvas) holds until the first place is built, and the next place loads while
// the current one plays. It gives way to the flat backdrop for good when WebGL is
// missing or the first frames come in under 45 fps. With reduced motion it is one
// still frame of the first place.
//
// `el.sixfoldTitle` (for screenshots and the console): `.shot(i)` holds shot i,
// `.play()` resumes the reel.
import * as THREE from "three";
import { U } from "./life.js";
import { loadMap, loadAsset, assetNames, tr } from "./map.js";
import { build, dispose, refresh, animateProps } from "./build.js";
import { updateNpc } from "./npc.js";
import { daylight, phaseOf } from "./daylight.js";
import { weak } from "./tier.js"; // a device that can't carry a whole map, known before trying

// A place: the map, the hour (game minute), what the camera looks at (tile x, z, and a
// height), and the camera's path round it: bearing (degrees, 0 = from the south, the
// world's own view), pitch and distance, each from → to over the shot.
const SHOTS = [
  { map: "overworld", minute: 620, at: [23, 14, 1], yaw: [-34, -22], pitch: [31, 29], dist: [36, 32] }, // the meadow, mid-morning
  { map: "gullhaven", minute: 800, at: [34, 14, 0.5], yaw: [42, 28], pitch: [29, 27], dist: [36, 32] }, // the harbour, afternoon
  { map: "cinderwick", minute: 1150, at: [22, 15, 1], yaw: [-26, -14], pitch: [31, 29], dist: [32, 28] }, // the kilns at dusk
  { map: "lanternmere", minute: 1330, at: [28, 18, 0.3], yaw: [16, 4], pitch: [31, 29], dist: [28, 25] }, // the lantern lake at night
  { map: "square", minute: 400, at: [25, 13, 1], yaw: [-20, -8], pitch: [31, 29], dist: [31, 28] }, // the town square at dawn
];
const HOLD = 13; // seconds on a place
const FADE = 1.6; // seconds of cross-fade

const holds = (cond) => !cond; // only what is always there: no story state on the title
const nobody = { x: -99, z: -99, step: null }; // the people's paths are never blocked by a hero

/// Start the first place downloading (the splash calls this, so the title opens on it).
export const preload = () => fetchShot(SHOTS[0]).catch(() => {});

function fetchShot(s) {
  return loadMap(s.map).then((map) => {
    const names = assetNames(map).filter((n) => n !== "hero");
    return Promise.all(names.map(loadAsset)).then((g) => ({ map, got: Object.fromEntries(names.map((n, i) => [n, g[i]])) }));
  });
}


// The device can't carry the backdrop (no WebGL, or `weak`): known from the first title,
// so a later one (after the world, a reset, the lesson) goes flat without a new context.
// Creating one waits on the GPU process: on a software renderer still busy with the
// world's last frames, that froze the page for seconds on the way back to the title.
let flatForGood = false;

export function mount(el) {
  const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const flat = () => el.classList.add("flat");
  if (flatForGood) {
    flat();
    return { stop() {} };
  }
  let renderer;
  try {
    renderer = new THREE.WebGLRenderer({ antialias: true, powerPreference: "high-performance" });
  } catch {
    flatForGood = true;
    flat();
    return { stop() {} };
  }
  if (weak(renderer)) {
    flatForGood = true;
    renderer.forceContextLoss();
    renderer.dispose();
    flat();
    return { stop() {} };
  }
  U.uSway.value = reduced ? 0 : 1;
  renderer.setPixelRatio(Math.min(devicePixelRatio, 1.5));
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = THREE.PCFShadowMap; // soft since r182 (PCFSoftShadowMap is gone)
  renderer.shadowMap.autoUpdate = false;
  renderer.autoClear = false;
  const canvas = renderer.domElement;
  canvas.setAttribute("aria-hidden", "true");
  // The cross-fade: the last frame of a place, held on a 2D canvas over the new one and faded out.
  const veil = document.createElement("canvas");
  veil.className = "title-veil";
  veil.setAttribute("aria-hidden", "true");
  // The place's name and hour, like a film's location card.
  const place = document.createElement("p");
  place.className = "title-place";
  place.setAttribute("aria-hidden", "true");
  el.append(canvas, veil, place);
  const HOURS = { dawn: tr("Dawn", "Aube"), day: tr("Day", "Jour"), dusk: tr("Dusk", "Crépuscule"), night: tr("Night", "Nuit") };
  const vctx = veil.getContext("2d");

  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(34, 1, 0.5, 90);
  // As in world.js: the shadow pass through a camera that holds every layer (the
  // terrain's shadow-only caster is on layer 2) and sees nothing itself.
  const shadowEye = new THREE.OrthographicCamera(-0.01, 0.01, 0.01, -0.01, 0.1, 0.2);
  shadowEye.position.set(0, -1e4, 0);
  shadowEye.updateMatrixWorld();
  shadowEye.layers.enableAll();

  let wide = true;
  const resize = () => {
    const w = el.clientWidth || 1, h = el.clientHeight || 1;
    renderer.setSize(w, h);
    veil.width = canvas.width;
    veil.height = canvas.height;
    camera.aspect = w / h;
    wide = w > h;
    // The place sits clear of the menu: right of it on a wide screen, above it on a tall one.
    camera.fov = wide ? 34 : 52;
    camera.setViewOffset(w, h, wide ? -w * 0.14 : 0, wide ? 0 : h * 0.16, w, h);
    camera.updateProjectionMatrix();
    if (reduced && world) draw(0);
  };

  let world = null, shot = 0, t0 = 0, held = -1, alive = true, raf = 0, last = 0, fade = 0, minute = 0;
  const look = new THREE.Vector3();
  const ease = (k) => k * k * (3 - 2 * k);

  // Build place `i` (its models already fetched), replacing the one on screen.
  async function show(i) {
    const s = SHOTS[i];
    const { map, got } = await fetchShot(s);
    if (!alive) return false;
    const next = await build(map, got, reduced);
    if (!alive) return dispose(next.root), false;
    next.hero = nobody;
    refresh(next, holds, () => false);
    next.day = daylight(next, reduced);
    if (world) dispose(world.root);
    world = next;
    scene.add(world.root);
    const L = map.light;
    scene.fog = new THREE.Fog(L.background, 40, 80);
    shot = i;
    minute = s.minute;
    t0 = performance.now() / 1000;
    place.innerHTML = `<b>${map.title ?? ""}</b><span>${HOURS[phaseOf(minute)]}</span>`;
    draw(0, 0);
    // A still of the place, for the menu screens behind their panels (`--world-still`).
    canvas.toBlob((b) => {
      if (!b || !alive) return;
      const root = document.documentElement, old = root.dataset.worldStill, url = URL.createObjectURL(b);
      root.dataset.worldStill = url;
      root.style.setProperty("--world-still", `url("${url}")`);
      if (old) setTimeout(() => URL.revokeObjectURL(old), 1000);
    }, "image/jpeg", 0.8);
    await renderer.compileAsync(scene, camera).catch(() => {});
    globalThis.sixfoldAudio?.ambience(map.ambience ?? map.music ?? "overworld", phaseOf(minute));
    return true;
  }

  // One frame of the current place, `k` (0..1) of the way through its shot.
  function draw(k, dt = 0) {
    const s = SHOTS[shot], w = world;
    const lerp = (a) => a[0] + (a[1] - a[0]) * ease(k);
    const yaw = THREE.MathUtils.degToRad(lerp(s.yaw)), pitch = THREE.MathUtils.degToRad(lerp(s.pitch)), d = lerp(s.dist);
    look.set(s.at[0] + 0.5, s.at[2], s.at[1] + 0.5);
    camera.position.set(look.x + Math.sin(yaw) * Math.cos(pitch) * d, look.y + Math.sin(pitch) * d, look.z + Math.cos(yaw) * Math.cos(pitch) * d);
    camera.lookAt(look);
    w.now += dt;
    U.uTime.value = w.now;
    if (!reduced) U.uDrift.value = w.now;
    for (const n of w.npcs) updateNpc(w, n, dt, false);
    animateProps(w, dt, reduced);
    const lit = w.day.update(minute, look.x, look.z, dt);
    w.life.update(dt, w.now, look.x, look.z, lit.night);
    w.fireflies.update(dt, w.now, lit.night, reduced);
    // The fog keeps the far ground soft without eating the place: kept past the camera's distance.
    renderer.setClearColor(lit.color ?? lit.bg);
    scene.fog.color.copy(lit.color ?? lit.bg);
    scene.fog.near = Math.max(lit.near, d + 6);
    scene.fog.far = Math.max(lit.far, d + 40);
    w.sun.position.set(look.x + lit.from.x, lit.from.y, look.z + lit.from.z);
    w.sun.target.position.set(look.x, 0, look.z);
    renderer.shadowMap.needsUpdate = true;
    renderer.render(scene, shadowEye);
    renderer.clear();
    renderer.render(scene, camera);
  }

  // Hold the frame just drawn on the veil (same task as the render: the buffer is still there).
  const cover = () => {
    vctx.clearRect(0, 0, veil.width, veil.height);
    vctx.drawImage(canvas, 0, 0);
    fade = 1;
    veil.style.opacity = 1;
  };

  // The frame-time probe: frames 30 to 120 of the first place. Under 45 fps (a weak
  // GPU, a battery saver capping the page), the flat backdrop instead.
  const times = [];
  let switching = false;
  const loop = (ms) => {
    if (!alive) return;
    raf = requestAnimationFrame(loop);
    const t = ms / 1000, dt = last ? Math.min(t - last, 0.1) : 0;
    last = t;
    if (switching) return;
    const age = t - t0, k = held >= 0 ? 0.5 : Math.min(1, age / HOLD);
    draw(k, dt);
    if (fade > 0) (fade = Math.max(0, fade - dt / FADE)), (veil.style.opacity = ease(fade));
    if (times.length < 120 && dt && times.push(dt) === 120) {
      if (times.slice(30).sort((a, b) => a - b)[45] > 1 / 45) return void (api.stop(), flat());
    }
    if (held < 0 && age >= HOLD) {
      switching = true;
      cover();
      show((shot + 1) % SHOTS.length)
        .catch(() => {}) // a place that fails to load: the next one
        .finally(() => (switching = false));
    }
  };

  const ro = new ResizeObserver(resize);
  ro.observe(el);
  show(0)
    .then((ok) => {
      if (!ok || !alive) return;
      resize();
      el.classList.add("live");
      if (reduced) draw(0.5);
      else raf = requestAnimationFrame(loop);
      // The next place downloads while this one plays.
      if (!reduced) fetchShot(SHOTS[1]).catch(() => {});
    })
    .catch(() => (alive ? (api.stop(), flat()) : null));

  el.sixfoldTitle = {
    shot: async (i) => ((held = i), await show(i), draw(0.5)),
    play: () => ((held = -1), (t0 = performance.now() / 1000)),
    shots: SHOTS,
  };

  const api = {
    stop() {
      if (!alive) return;
      alive = false;
      cancelAnimationFrame(raf);
      ro.disconnect();
      if (world) dispose(world.root);
      renderer.forceContextLoss(); // the world screen makes its own context next
      renderer.dispose();
      canvas.remove();
      veil.remove();
      el.classList.remove("live");
    },
  };
  return api;
}
