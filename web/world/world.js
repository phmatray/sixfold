// Overworld test scene on a tile grid. `mount(el, cb)` builds it inside `el` and
// returns a stop function. `cb(event, kind, text)` reports to Leptos:
//   ("near", kind, text) when the hero faces an interaction tile ("near", "", "" when not),
//   ("use", kind, text) when the player acts on it.
// The layout comes from ./map.json; props load from ./assets/<name>.glb (see
// ASSETS.md), and a missing file falls back to the placeholder below.
// Grid: 1 tile = 1 m; tile (x, z) covers [x, x+1] × [z, z+1].
import * as THREE from "three";
import { GLTFLoader } from "three/addons/loaders/GLTFLoader.js";
import { U, windify, cloudify, ambience } from "./life.js";

const ASSETS = new URL("./assets/", import.meta.url);
const MAP = new URL("./map.json", import.meta.url);
const NAMES = ["house", "tree", "bush", "flowers", "rock", "sign", "fence", "fence_corner", "fence_end", "fence_t", "path", "path_b", "path_c", "hero"];
const OPTIONAL = new Set(["path_b", "path_c"]); // extra dirt-tile variants
// Footprint in tiles [along X, along Z] at rot 0. Everything but path blocks.
const FOOT = { house: [7, 5], tree: [2, 2], bush: [1, 1], flowers: [1, 1], rock: [1, 1], sign: [1, 1], path: [1, 1] };
const STEP = 0.25; // seconds per tile
const TURN_SPEED = Math.PI / 2 / 0.12; // a quarter turn in 0.12 s
const WIND_H = { tree: 3.5, bush: 0.8, flowers: 0.4 }; // sway reference height per prop
const FIDGET_AFTER = 6; // seconds idle before the hero fidgets and the camera leans in
const TURN_HOLD = 0.12; // a press shorter than this only turns the hero
const DIR = { n: [0, -1], s: [0, 1], e: [1, 0], w: [-1, 0] }; // n = away from the camera
const KEYS = { w: "n", arrowup: "n", s: "s", arrowdown: "s", a: "w", arrowleft: "w", d: "e", arrowright: "e" };
const angle = (f) => Math.atan2(DIR[f][0], DIR[f][1]);
// Fence auto-tiling: neighbour mask (N=1, E=2, S=4, W=8) -> [piece, rot]. At rot 0:
// fence = arms W+E, fence_corner = W+S, fence_end = W, fence_t = W+E+S. Mask 15 adds an end toward N.
// Same table in ASSETS.md and in the Blender scene script.
const FENCE = [
  ["fence", 0], ["fence_end", 270], ["fence_end", 180], ["fence_corner", 180],
  ["fence_end", 90], ["fence", 90], ["fence_corner", 90], ["fence_t", 90],
  ["fence_end", 0], ["fence_corner", 270], ["fence", 0], ["fence_t", 180],
  ["fence_corner", 0], ["fence_t", 270], ["fence_t", 0], ["fence_t", 0],
];

// Small seeded RNG so the meadow looks the same every visit.
function rng(seed) {
  return () => {
    seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

// ---------- Materials ----------

const ramp = (() => {
  const t = new THREE.DataTexture(new Uint8Array([90, 160, 215, 255]), 4, 1, THREE.RedFormat);
  t.minFilter = t.magFilter = THREE.NearestFilter;
  t.needsUpdate = true;
  return t;
})();
const toon = (color, extra = {}) => new THREE.MeshToonMaterial({ color, gradientMap: ramp, ...extra });

function grassTexture() {
  const c = document.createElement("canvas");
  c.width = c.height = 512;
  const g = c.getContext("2d");
  g.fillStyle = "#4fa83a";
  g.fillRect(0, 0, 512, 512);
  const r = rng(7);
  const blob = (x, y, rx, ry, a, col) => {
    g.globalAlpha = a;
    g.fillStyle = col;
    // Drawn wrapped so the texture tiles without seams.
    for (const dx of [-512, 0, 512])
      for (const dy of [-512, 0, 512]) {
        g.beginPath();
        g.ellipse(x + dx, y + dy, rx, ry, 0.3, 0, Math.PI * 2);
        g.fill();
      }
  };
  for (let i = 0; i < 260; i++) blob(r() * 512, r() * 512, 20 + r() * 60, 12 + r() * 34, 0.12, r() < 0.5 ? "#3a8a2c" : "#6cc24a");
  for (let i = 0; i < 900; i++) blob(r() * 512, r() * 512, 2 + r() * 6, 1 + r() * 3, 0.25, r() < 0.5 ? "#2f7a26" : "#86d65c");
  const t = new THREE.CanvasTexture(c);
  t.colorSpace = THREE.SRGBColorSpace;
  t.wrapS = t.wrapT = THREE.RepeatWrapping;
  t.anisotropy = 4;
  return t;
}

// ---------- Placeholders: same footprints, origins (footprint centre) and facing (+Z) as the contract ----------

const mesh = (geo, mat, x = 0, y = 0, z = 0) => {
  const m = new THREE.Mesh(geo, mat);
  m.position.set(x, y, z);
  return m;
};
const group = (...kids) => {
  const g = new THREE.Group();
  kids.forEach((k) => g.add(k));
  return g;
};
const blobs = (mat, list) => list.map(([r, x, y, z]) => mesh(new THREE.IcosahedronGeometry(r, 1), mat, x, y, z));

// The placeholder door hangs on a hinge at its left edge, like the contract asks.
function door() {
  const hinge = group(mesh(new THREE.BoxGeometry(0.9, 2.0, 0.14), toon("#2f7175"), 0.45, 0, 0));
  hinge.name = "door";
  hinge.position.set(-0.45, 1.3, 2.32);
  return hinge;
}

const PLACEHOLDER = {
  house() {
    const wall = toon("#efe4cc"), wood = toon("#7a5a3c"), glass = toon("#9fd3f0");
    const roofGeo = new THREE.ExtrudeGeometry(
      new THREE.Shape([new THREE.Vector2(-2.75, 0), new THREE.Vector2(2.75, 0), new THREE.Vector2(0, 2.0)]),
      { depth: 7.3, bevelEnabled: false },
    );
    roofGeo.translate(0, 0, -3.65);
    roofGeo.rotateY(Math.PI / 2); // ridge runs along X, slopes face the camera
    const hex = new THREE.CylinderGeometry(0.42, 0.42, 0.08, 6);
    hex.rotateX(Math.PI / 2);
    hex.rotateZ(Math.PI / 6);
    const inner = hex.clone().scale(0.55, 0.55, 1.3);
    return group(
      mesh(new THREE.BoxGeometry(7, 0.3, 5), toon("#b7a58a"), 0, 0.15, 0),
      mesh(new THREE.BoxGeometry(6.6, 2.9, 4.6), wall, 0, 1.6, 0),
      mesh(roofGeo, toon("#d0663f"), 0, 3.05, 0),
      mesh(new THREE.BoxGeometry(0.6, 1.4, 0.6), toon("#a9533a"), 2, 4.1, -0.6),
      door(),
      mesh(new THREE.BoxGeometry(1.2, 0.14, 0.5), wood, 0, 2.4, 2.45),
      mesh(new THREE.BoxGeometry(1.3, 1.0, 0.1), glass, -2.1, 1.85, 2.32),
      mesh(new THREE.BoxGeometry(1.3, 1.0, 0.1), glass, 2.1, 1.85, 2.32),
      mesh(new THREE.BoxGeometry(1.5, 0.16, 0.3), wood, -2.1, 1.28, 2.4),
      mesh(new THREE.BoxGeometry(1.5, 0.16, 0.3), wood, 2.1, 1.28, 2.4),
      mesh(hex, toon("#2d5fae"), 0, 2.85, 2.34), // the Sixfold hex
      mesh(inner, toon("#f3d589"), 0, 2.85, 2.34),
      Object.assign(new THREE.Object3D(), { name: "smoke_origin" }).translateX(2).translateY(4.85).translateZ(-0.6),
    );
  },
  tree() {
    const leaf = toon("#3f9a3a"), leaf2 = toon("#5bb547");
    return group(
      mesh(new THREE.CylinderGeometry(0.16, 0.24, 1.6, 7), toon("#7a5536"), 0, 0.8, 0),
      ...blobs(leaf, [[1.0, 0, 2.2, 0], [0.75, -0.45, 2.7, -0.3]]),
      ...blobs(leaf2, [[0.7, 0.45, 2.9, 0.2]]),
    );
  },
  bush() {
    return group(...blobs(toon("#3e9b36"), [[0.36, 0, 0.32, 0], [0.26, 0.24, 0.24, 0.1], [0.25, -0.24, 0.22, -0.05]]));
  },
  flowers() {
    const g = group(mesh(new THREE.SphereGeometry(0.4, 10, 6).scale(1, 0.35, 1), toon("#3b8f2e"), 0, 0.08, 0));
    const petal = toon("#ffb38f"), heart = toon("#fff0b0");
    for (let i = 0; i < 5; i++) {
      const a = (i / 5) * Math.PI * 2, x = Math.cos(a) * 0.22, z = Math.sin(a) * 0.22;
      g.add(mesh(new THREE.SphereGeometry(0.1, 8, 6).scale(1, 0.5, 1), petal, x, 0.2, z));
      g.add(mesh(new THREE.SphereGeometry(0.04, 6, 4), heart, x, 0.25, z));
    }
    return g;
  },
  rock() {
    return group(mesh(new THREE.DodecahedronGeometry(0.42, 0).scale(1.1, 0.75, 1), toon("#9a9a8e"), 0, 0.28, 0));
  },
  sign() {
    const wood = toon("#8a6440");
    return group(
      mesh(new THREE.BoxGeometry(0.12, 1.1, 0.12), wood, 0, 0.55, 0),
      mesh(new THREE.BoxGeometry(0.85, 0.55, 0.08), toon("#f2e6cc"), 0, 1.0, 0.08),
      mesh(new THREE.BoxGeometry(0.95, 0.08, 0.12), wood, 0, 1.3, 0.08),
    );
  },
  // A post plus two rails toward each arm ([dx, dz] in world space, so no rotation needed).
  fence(arms) {
    const wood = toon("#c89a62");
    const g = group(mesh(new THREE.BoxGeometry(0.14, 0.9, 0.14), wood, 0, 0.45, 0));
    for (const [dx, dz] of arms)
      for (const y of [0.33, 0.66]) g.add(mesh(new THREE.BoxGeometry(dx ? 0.5 : 0.05, 0.09, dz ? 0.5 : 0.05), wood, dx * 0.25, y, dz * 0.25));
    return g;
  },
  path(seed) {
    const r = rng(seed), pts = [];
    for (let i = 0; i < 14; i++) {
      const a = (i / 14) * Math.PI * 2, d = 0.6 + r() * 0.12;
      // A rounded square, so neighbouring tiles join into one path.
      const k = d / Math.max(Math.abs(Math.cos(a)), Math.abs(Math.sin(a))) ** 0.6;
      pts.push(new THREE.Vector2(Math.cos(a) * Math.min(k, 0.72), Math.sin(a) * Math.min(k, 0.72)));
    }
    const geo = new THREE.ShapeGeometry(new THREE.Shape(pts));
    geo.rotateX(-Math.PI / 2);
    return group(mesh(geo, new THREE.MeshLambertMaterial({ color: "#d9b06a" }), 0, 0.02, 0));
  },
  hero() {
    const skin = toon("#f6cfa8"), dark = toon("#3b2a22"), cap = toon("#2b4c8c");
    const legL = mesh(new THREE.CylinderGeometry(0.07, 0.07, 0.3, 6), dark, -0.1, 0.15, 0);
    const legR = legL.clone();
    legR.position.x = 0.1;
    const g = group(
      legL, legR,
      mesh(new THREE.CapsuleGeometry(0.2, 0.18, 4, 10), toon("#e2553a"), 0, 0.45, 0),
      mesh(new THREE.BoxGeometry(0.26, 0.26, 0.12), toon("#2f7175"), 0, 0.47, -0.2), // backpack
      mesh(new THREE.SphereGeometry(0.3, 16, 12), skin, 0, 0.84, 0),
      mesh(new THREE.SphereGeometry(0.315, 16, 12, 0, Math.PI * 2, 0, Math.PI * 0.5), cap, 0, 0.88, -0.02),
      mesh(new THREE.BoxGeometry(0.34, 0.04, 0.2), cap, 0, 0.9, 0.3), // brim, faces +Z
    );
    g.userData.legs = [legL, legR];
    g.scale.setScalar(1.27); // drawn ~1.15 m, the contract hero is ~1.45 m
    return g;
  },
};

// ---------- Mount ----------

export function mount(el, cb) {
  const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const renderer = new THREE.WebGLRenderer({ antialias: true });
  renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = THREE.PCFSoftShadowMap;
  el.appendChild(renderer.domElement);

  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(40, 1, 0.5, 80); // tight planes: depth precision for flat tiles
  let cam = null, dist = 12, alive = true, raf = 0;
  const resize = () => {
    const w = el.clientWidth || 1, h = el.clientHeight || 1;
    renderer.setSize(w, h);
    camera.aspect = w / h;
    if (cam) {
      // Portrait phones see less width, so pull back and widen a little.
      camera.fov = w < h ? cam.fovPortrait : cam.fov;
      dist = w < h ? cam.distancePortrait : cam.distance;
    }
    camera.updateProjectionMatrix();
  };
  const ro = new ResizeObserver(resize);
  ro.observe(el);

  // D-pad for touch screens (CSS hides it where there is a hover pointer).
  const pad = document.createElement("div");
  pad.className = "world-pad";
  for (const [f, label, arrow] of [["n", "Up", "▲"], ["w", "Left", "◀"], ["e", "Right", "▶"], ["s", "Down", "▼"]]) {
    const b = document.createElement("button");
    b.type = "button";
    b.textContent = arrow;
    b.className = `pad-${f}`;
    b.setAttribute("aria-label", label);
    b.dataset.dir = f;
    pad.appendChild(b);
  }
  el.appendChild(pad);

  // ---------- Input state (filled once the map is in) ----------
  const held = []; // pressed inputs, newest last: key names or "pad-<dir>"
  const dirOf = (k) => KEYS[k] ?? k.slice(4);
  const press = (k) => {
    const i = held.indexOf(k);
    if (i >= 0) held.splice(i, 1);
    held.push(k);
    world && (world.path = []);
  };
  const release = (k) => {
    const i = held.indexOf(k);
    if (i >= 0) held.splice(i, 1);
  };
  let world = null;

  const onKey = (e) => {
    const k = e.key.toLowerCase();
    if (e.type === "keyup") return release(k);
    if (e.target.closest?.(".scrim, input, select, textarea")) return; // a dialog has the keys
    if (KEYS[k]) {
      e.preventDefault();
      if (!e.repeat) press(k);
    } else if ((k === "e" || k === "enter" || k === " ") && world?.near && !e.repeat) {
      e.preventDefault();
      act();
    } else if (k === "g" && world && !e.repeat) {
      world.grid.visible = !world.grid.visible;
    }
  };
  const clearHeld = () => (held.length = 0);
  // Act on the faced tile: the door swings open first; a sign wobbles.
  const act = () => {
    const it = world?.near;
    if (!it) return;
    held.length = 0;
    if (it.kind === "duel") {
      world.door.want = 1;
      setTimeout(() => alive && cb("use", it.kind, it.text), reduced ? 0 : 450);
      return;
    }
    if (it.obj && !reduced) it.obj.userData.wobble = 1;
    cb("use", it.kind, it.text);
  };
  const padDown = (e) => {
    const f = e.target.dataset?.dir;
    if (!f) return;
    e.preventDefault();
    press(`pad-${f}`);
    try {
      e.target.setPointerCapture(e.pointerId); // keep the release even if the finger slides off
    } catch {}
  };
  const padUp = (e) => e.target.dataset?.dir && release(`pad-${e.target.dataset.dir}`);
  addEventListener("keydown", onKey);
  addEventListener("keyup", onKey);
  addEventListener("blur", clearHeld);
  pad.addEventListener("pointerdown", padDown);
  for (const t of ["pointerup", "pointercancel", "lostpointercapture"]) pad.addEventListener(t, padUp);
  pad.addEventListener("contextmenu", (e) => e.preventDefault());

  // Tap/click the ground: walk the shortest grid path there (BFS).
  const ray = new THREE.Raycaster(), ndc = new THREE.Vector2(), hit = new THREE.Vector3();
  const ground0 = new THREE.Plane(new THREE.Vector3(0, 1, 0), 0);
  const onTap = (e) => {
    if (!world) return;
    const b = renderer.domElement.getBoundingClientRect();
    ndc.set(((e.clientX - b.left) / b.width) * 2 - 1, -((e.clientY - b.top) / b.height) * 2 + 1);
    ray.setFromCamera(ndc, camera);
    if (ray.ray.intersectPlane(ground0, hit)) world.path = route(world, Math.floor(hit.x), Math.floor(hit.z));
  };
  renderer.domElement.addEventListener("pointerdown", onTap);

  // ---------- Load the map and assets, then build ----------
  const loader = new GLTFLoader();
  Promise.all([
    fetch(MAP).then((r) => r.json()),
    ...NAMES.map((n) => loader.loadAsync(new URL(`${n}.glb`, ASSETS).href).catch(() => null)),
  ])
    .then(([map, ...gltfs]) => {
      if (!alive) return;
      const got = Object.fromEntries(NAMES.map((n, i) => [n, gltfs[i]]));
      const missing = NAMES.filter((n) => !got[n] && !OPTIONAL.has(n));
      if (missing.length) console.info(`world: placeholders for ${missing.join(", ")}`);
      world = build(scene, map, got, reduced);
      cam = map.camera;
      resize();
      clock.getDelta();
      raf = requestAnimationFrame(frame);
    })
    .catch((e) => {
      console.error("world: could not build the scene", e);
      cb("error", "", String(e));
    });

  // ---------- Loop ----------
  const clock = new THREE.Clock();
  const look = new THREE.Vector3(), lookVel = new THREE.Vector3(), aim = new THREE.Vector3();
  let first = true, zoom = 1;
  U.uSway.value = reduced ? 0 : 1;
  function frame() {
    raf = requestAnimationFrame(frame);
    const dt = Math.min(clock.getDelta(), 0.05);
    const w = world, h = w.hero;
    w.now += dt;
    U.uTime.value = w.now;
    if (!reduced) U.uDrift.value = w.now;
    const prevStep = h.step;

    // Finish (or carry on) the current step.
    let carry = 0, chained = false;
    if (h.step) {
      h.step.t += dt / STEP;
      if (h.step.t >= 1) {
        carry = (h.step.t - 1) * STEP;
        h.x = h.step.x;
        h.z = h.step.z;
        h.step = null;
        chained = true;
      }
    }
    if (!h.step) {
      const k = held[held.length - 1];
      if (k) {
        const f = dirOf(k);
        if (f !== h.face) {
          h.face = f;
          h.turnedAt = w.now;
          if (chained) tryStep(w, f, carry); // already walking: turn and keep going
        } else if (chained || w.now - h.turnedAt >= TURN_HOLD) tryStep(w, f, carry);
      } else if (w.path.length) {
        h.face = w.path.shift();
        if (!tryStep(w, h.face, carry)) w.path = [];
      }
    }

    if (h.step && h.step !== prevStep) w.life.puff(h.x + 0.5, h.z + 0.5); // footstep dust
    h.idle = h.step || held.length ? 0 : h.idle + dt;

    // Place the hero: tile centre, or between two while stepping.
    const t = h.step ? h.step.t : 0;
    const px = h.x + 0.5 + (h.step ? (h.step.x - h.x) * t : 0);
    const pz = h.z + 0.5 + (h.step ? (h.step.z - h.z) * t : 0);
    h.obj.position.set(px, 0, pz);
    const want = angle(h.face), cur = h.obj.rotation.y;
    const turn = Math.atan2(Math.sin(want - cur), Math.cos(want - cur));
    h.obj.rotation.y = cur + Math.sign(turn) * Math.min(Math.abs(turn), TURN_SPEED * dt);
    animateHero(h, dt, reduced);
    animateProps(w, dt, reduced);
    w.life.update(dt, w.now, px, pz);

    el.dataset.hero = `${h.x},${h.z},${h.face}`; // for tests and debugging

    // Interaction: the tile in front of the hero, while standing still.
    const front = h.step ? null : w.talk.get(`${h.x + DIR[h.face][0]},${h.z + DIR[h.face][1]}`) ?? null;
    if (front !== w.near) {
      w.near = front;
      if (front?.kind !== "duel") w.door.want = 0; // walked off: the door swings shut
      cb("near", front?.kind ?? "", front?.text ?? "");
    }

    // Camera: fixed pitch, on a soft spring behind a point a little ahead of a
    // walking hero; leans in after a while idle. Rigid with reduced motion.
    const pitch = THREE.MathUtils.degToRad(cam.pitch), off = cam.lookAtOffset;
    const ahead = h.step && !reduced ? 0.7 : 0;
    aim.set(px + off[0] + DIR[h.face][0] * ahead, off[1], pz + off[2] + DIR[h.face][1] * ahead);
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
    // The sun (and its shadow map) moves a whole tile at a time: following the
    // tweened position makes shadow edges shimmer.
    const s = w.sun, from = w.sunFrom;
    s.position.set(h.x + from[0], from[1], h.z + from[2]);
    s.target.position.set(h.x, 0, h.z);

    renderer.render(scene, camera);
  }

  const stop = () => {
    alive = false;
    cancelAnimationFrame(raf);
    ro.disconnect();
    removeEventListener("keydown", onKey);
    removeEventListener("keyup", onKey);
    removeEventListener("blur", clearHeld);
    scene.traverse((o) => {
      o.geometry?.dispose();
      [o.material].flat().forEach((m) => m?.dispose());
    });
    renderer.dispose();
    renderer.domElement.remove();
    pad.remove();
  };
  return { stop, act };
}

// Start a one-tile step toward `f` if that tile is free; otherwise just face it.
function tryStep(w, f, carry) {
  const h = w.hero, x = h.x + DIR[f][0], z = h.z + DIR[f][1];
  h.face = f;
  if (w.blocked(x, z)) return false;
  h.step = { x, z, t: carry / STEP };
  return true;
}

// Shortest 4-way path from the hero to (tx, tz). A blocked target (a sign, the
// door) routes to a free neighbour and ends facing it.
function route(w, tx, tz) {
  const { width: W, height: H } = w.map, h = w.hero;
  if (tx === h.x && tz === h.z) return [];
  const goal = w.blocked(tx, tz)
    ? Object.values(DIR).map(([dx, dz]) => [tx - dx, tz - dz]).filter(([x, z]) => !w.blocked(x, z))
    : [[tx, tz]];
  if (!goal.length) return [];
  const prev = new Map([[`${h.x},${h.z}`, null]]);
  const queue = [[h.x, h.z]];
  while (queue.length) {
    const [x, z] = queue.shift();
    const g = goal.find(([gx, gz]) => gx === x && gz === z);
    if (g) {
      const dirs = [];
      for (let k = `${x},${z}`; prev.get(k); k = prev.get(k).from) dirs.unshift(prev.get(k).f);
      if (w.blocked(tx, tz)) dirs.push(Object.keys(DIR).find((f) => x + DIR[f][0] === tx && z + DIR[f][1] === tz));
      return dirs;
    }
    for (const [f, [dx, dz]] of Object.entries(DIR)) {
      const nx = x + dx, nz = z + dz, k = `${nx},${nz}`;
      if (nx < 0 || nz < 0 || nx >= W || nz >= H || prev.has(k) || w.blocked(nx, nz)) continue;
      prev.set(k, { from: `${x},${z}`, f });
      queue.push([nx, nz]);
    }
  }
  return [];
}

// Build the scene from map.json: ground, lights, props, scatter, hero, debug grid.
function build(scene, map, got, reduced) {
  const { width: W, height: H } = map;
  const L = map.light;
  scene.background = new THREE.Color(L.background);
  scene.fog = new THREE.Fog(L.background, L.fog.near, L.fog.far);
  scene.add(new THREE.HemisphereLight(L.hemisphere.sky, L.hemisphere.ground, L.hemisphere.intensity));
  const sun = new THREE.DirectionalLight(L.sun.color, L.sun.intensity);
  sun.castShadow = true;
  sun.shadow.mapSize.set(2048, 2048);
  Object.assign(sun.shadow.camera, { left: -16, right: 16, top: 16, bottom: -16, near: 1, far: 60 });
  sun.shadow.bias = -0.0005;
  sun.shadow.normalBias = 0.03;
  scene.add(sun, sun.target);

  const size = Math.max(W, H) + 40;
  const grass = grassTexture();
  grass.repeat.set(size / 6, size / 6);
  const ground = new THREE.Mesh(new THREE.PlaneGeometry(size, size), new THREE.MeshLambertMaterial({ map: grass }));
  cloudify(ground.material);
  ground.rotation.x = -Math.PI / 2;
  ground.position.set(W / 2, 0, H / 2);
  ground.receiveShadow = true;
  scene.add(ground);

  const solid = new Uint8Array(W * H);
  map.tiles.forEach((row, z) => [...row].forEach((c, x) => (solid[z * W + x] = c === "#" || c === "f" ? 1 : 0)));
  const blocked = (x, z) => x < 0 || z < 0 || x >= W || z >= H || solid[z * W + x] === 1;
  const make = (n, seed) => (got[n] ? got[n].scene.clone(true) : PLACEHOLDER[n](seed));

  // Each prop sits at its footprint's centre; rot 90/270 swaps the footprint.
  const props = [];
  for (const p of map.props) {
    let [fw, fd] = FOOT[p.asset];
    if (p.rot % 180 === 90) [fw, fd] = [fd, fw];
    const o = make(p.asset, p.x * 31 + p.z);
    o.position.set(p.x + fw / 2, 0, p.z + fd / 2);
    o.rotation.y = THREE.MathUtils.degToRad(p.rot);
    prep(o, true);
    if (WIND_H[p.asset]) o.traverse((m) => m.isMesh && windify(m, WIND_H[p.asset]));
    scene.add(o);
    props.push({ ...p, obj: o, cx: o.position.x, cz: o.position.z });
    for (const i of map.interactions) if (i.x >= p.x && i.x < p.x + fw && i.z >= p.z && i.z < p.z + fd) i.obj = o;
    if (p.asset !== "path") for (let x = p.x; x < p.x + fw; x++) for (let z = p.z; z < p.z + fd; z++) solid[z * W + x] = 1;
  }
  const isFence = (x, z) => map.tiles[z]?.[x] === "f";
  const r = rng(5);
  map.tiles.forEach((row, z) =>
    [...row].forEach((c, x) => {
      if (c === "f") return scene.add(fenceTile(got, x, z, isFence));
      if (c !== "p") return;
      const variants = ["path", "path_b", "path_c"].filter((n) => got[n]);
      const o = variants.length ? variants[(x * 7 + z * 13) % variants.length] : null;
      const tile = o ? got[o].scene.clone(true) : PLACEHOLDER.path(x * 31 + z);
      tile.position.set(x + 0.5, 0, z + 0.5);
      tile.rotation.y = Math.floor(r() * 4) * (Math.PI / 2);
      prep(tile, false);
      // Flat on the ground: pull it toward the camera in depth so it never z-fights the grass.
      tile.traverse((m) => m.material && Object.assign(m.material, { polygonOffset: true, polygonOffsetFactor: -2, polygonOffsetUnits: -2 }));
      scene.add(tile);
    }),
  );
  const isPath = (x, z) => map.tiles[z]?.[x] === "p";
  scatter(scene, W, H, (x, z) => !blocked(x, z) && !isPath(x, z));

  const s = map.spawn;
  const obj = got.hero ? got.hero.scene : PLACEHOLDER.hero();
  prep(obj, true);
  scene.add(obj);
  const hero = { x: s.x, z: s.z, face: s.facing, turnedAt: -1, step: null, obj, mixer: null, actions: {}, walking: false, t: 0, idle: 0, base: obj.scale.x };
  obj.rotation.y = angle(s.facing);
  if (got.hero?.animations.length) {
    hero.mixer = new THREE.AnimationMixer(obj);
    for (const clip of got.hero.animations) hero.actions[clip.name.toLowerCase()] = hero.mixer.clipAction(clip);
    hero.actions.idle?.play();
  }

  const talk = new Map(map.interactions.map((i) => [`${i.x},${i.z}`, i]));
  const grid = debugGrid(W, H, blocked, talk);
  grid.visible = false;
  scene.add(grid);

  // The house door: its "Open" clip when the model has one, else a node named "door"
  // swung on its origin (the hinge). `open` eases toward `want`, 0 shut to 1 open.
  const house = props.find((p) => p.asset === "house")?.obj;
  const clip = got.house?.animations.find((a) => a.name.toLowerCase() === "open");
  const doorState = { open: 0, want: 0, node: house?.getObjectByName("door") ?? null, mixer: null, action: null };
  if (house && clip) {
    doorState.mixer = new THREE.AnimationMixer(house);
    doorState.action = doorState.mixer.clipAction(clip);
    doorState.action.play();
    doorState.action.paused = true;
  }
  if (doorState.node) doorState.rest = doorState.node.rotation.y;

  scene.updateMatrixWorld(true);
  const smoke = house?.getObjectByName("smoke_origin")?.getWorldPosition(new THREE.Vector3()) ?? null;
  const flowers = props.filter((p) => p.asset === "flowers").filter((_, i) => i % 3 === 0).map((p) => ({ x: p.cx, z: p.cz }));
  const life = ambience(scene, { flowers, smoke }, reduced);
  return { map, blocked, hero, talk, grid, sun, sunFrom: L.sun.fromHero, near: null, path: [], now: 0, door: doorState, life };
}

// One fence tile, its piece and rotation picked from the fence neighbours.
function fenceTile(got, x, z, isFence) {
  const sides = ["n", "e", "s", "w"];
  const mask = sides.reduce((m, f, i) => m | (isFence(x + DIR[f][0], z + DIR[f][1]) ? 1 << i : 0), 0);
  const [piece, rot] = FENCE[mask];
  const extra = mask === 15 ? [["fence_end", 270]] : [];
  const g = group();
  for (const [n, deg] of [[piece, rot], ...extra]) {
    let o;
    if (got[n]) {
      o = got[n].scene.clone(true);
      o.rotation.y = THREE.MathUtils.degToRad(deg);
    } else {
      const arms = mask === 0 ? [] : sides.filter((_, i) => mask & (1 << i)).map((f) => DIR[f]);
      o = PLACEHOLDER.fence(n === piece ? arms : []);
    }
    g.add(o);
  }
  g.position.set(x + 0.5, 0, z + 0.5);
  prep(g, true);
  return g;
}

// G toggles this: tile lines, red = blocked, gold = interaction.
function debugGrid(W, H, blocked, talk) {
  const pts = [];
  for (let x = 0; x <= W; x++) pts.push(x, 0.04, 0, x, 0.04, H);
  for (let z = 0; z <= H; z++) pts.push(0, 0.04, z, W, 0.04, z);
  const geo = new THREE.BufferGeometry();
  geo.setAttribute("position", new THREE.Float32BufferAttribute(pts, 3));
  const g = group(new THREE.LineSegments(geo, new THREE.LineBasicMaterial({ color: "#ffffff", transparent: true, opacity: 0.55 })));
  const quad = new THREE.PlaneGeometry(0.92, 0.92).rotateX(-Math.PI / 2);
  const flat = { transparent: true, depthWrite: false, polygonOffset: true, polygonOffsetFactor: -4, polygonOffsetUnits: -4 };
  const red = new THREE.MeshBasicMaterial({ color: "#ff3b30", opacity: 0.35, ...flat });
  const gold = new THREE.MeshBasicMaterial({ color: "#ffd60a", opacity: 0.6, ...flat });
  for (let x = 0; x < W; x++)
    for (let z = 0; z < H; z++) {
      const m = talk.has(`${x},${z}`) ? gold : blocked(x, z) ? red : null;
      if (m) g.add(mesh(quad, m, x + 0.5, 0.05, z + 0.5));
    }
  return g;
}

// Shadows on, and loaded glTF materials swapped to toon so they match the placeholders.
function prep(root, cast) {
  root.traverse((o) => {
    if (!o.isMesh) return;
    o.castShadow = cast;
    o.receiveShadow = true;
    if (!(o.material instanceof THREE.MeshToonMaterial) && !(o.material instanceof THREE.MeshLambertMaterial)) {
      const m = o.material;
      o.material = toon(m.color ?? "#ffffff", { map: m.map ?? null, transparent: m.transparent, alphaTest: m.alphaTest });
    }
  });
}

function animateHero(h, dt, reduced) {
  const walking = !!h.step;
  h.t += dt;
  if (h.mixer) {
    if (walking !== h.walking) {
      const [from, to] = walking ? ["idle", "walk"] : ["walk", "idle"];
      h.actions[from]?.fadeOut(0.15);
      h.actions[to]?.reset().fadeIn(0.15).play();
    }
    // A fidget clip once the hero has stood still a while, if the model has one.
    const fidget = h.actions.fidget ?? h.actions.look;
    if (fidget && h.idle > FIDGET_AFTER && !h.fidgeted) {
      h.fidgeted = true;
      fidget.reset().setLoop(THREE.LoopOnce, 1).fadeIn(0.2).play();
      fidget.getMixer().addEventListener("finished", () => fidget.fadeOut(0.3));
    }
    if (h.idle === 0) h.fidgeted = false;
    h.mixer.update(dt);
  } else if (h.obj.userData.legs) {
    const swing = walking ? Math.sin((h.t * Math.PI) / STEP) * 0.6 : 0; // one leg forward per tile
    const [l, r] = h.obj.userData.legs;
    l.rotation.x = swing;
    r.rotation.x = -swing;
  }
  h.walking = walking;
  // A small bounce per tile, squashed at each footfall.
  const up = walking && !reduced ? Math.sin(Math.PI * h.step.t) : 0;
  const squash = walking && !reduced ? 1 - up : 0;
  h.obj.position.y = up * 0.05;
  h.obj.scale.set(h.base * (1 + squash * 0.04), h.base * (1 - squash * 0.06 + up * 0.02), h.base * (1 + squash * 0.04));
}

// The door easing open or shut, and the sign wobbling after it was read.
function animateProps(w, dt, reduced) {
  const d = w.door;
  d.open = reduced ? d.want : d.open + (d.want - d.open) * Math.min(1, dt * 7);
  if (d.action) {
    d.action.time = d.open * d.action.getClip().duration;
    d.mixer.update(0);
  } else if (d.node) d.node.rotation.y = d.rest + d.open * 1.5;
  for (const it of w.talk.values()) {
    const o = it.obj;
    if (!o?.userData.wobble) continue;
    o.userData.wobble = Math.max(0, o.userData.wobble - dt * 1.4);
    o.rotation.z = Math.sin(w.now * 22) * 0.07 * o.userData.wobble;
  }
}

// Grass tufts and tiny flowers, instanced, only on open grass tiles.
function scatter(scene, W, H, open) {
  const r = rng(99);
  const blade = new THREE.ConeGeometry(0.05, 0.32, 3);
  blade.translate(0, 0.16, 0);
  const tufts = new THREE.InstancedMesh(blade, new THREE.MeshLambertMaterial(), 2000);
  const bloom = new THREE.InstancedMesh(new THREE.SphereGeometry(0.06, 6, 4), new THREE.MeshLambertMaterial(), 220);
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
  scene.add(tufts, bloom);
}
