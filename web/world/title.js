// The title's living backdrop: a hex island at dusk, its cottage lit and a
// campfire going, embers rising, the camera on a slow orbit. Built from hex
// prisms and the world's own models (title.rs mounts it behind the menu).
//
// It gives way to the flat CSS backdrop (`.title-backdrop.flat`) when WebGL is
// missing or the first frames come in under 50 fps. With reduced motion it is
// one still frame.
import * as THREE from "three";
import { loadAsset } from "./map.js";
import { prep, rng, toon } from "./props.js";

const MODELS = ["cottage", "tree", "tree_b", "pine", "bush", "rock", "campfire", "lantern", "flowers", "waymark"];
const R = 4; // island radius, in hexes
const S = 1.05; // hex circumradius (a tile is about 1 m)

/// Start the models downloading (the splash calls this, so the title opens on them).
export const preload = () => MODELS.forEach(loadAsset);

const NIGHT = "#16233d"; // the sea fading out, close to the felt behind the menu

// Axial hex coordinates within `r` of the centre.
function hexes(r) {
  const out = [];
  for (let q = -r; q <= r; q++)
    for (let s = Math.max(-r, -q - r); s <= Math.min(r, -q + r); s++) out.push([q, s, Math.max(Math.abs(q), Math.abs(s), Math.abs(q + s))]);
  return out;
}
const at = (q, s) => [S * Math.sqrt(3) * (q + s / 2), S * 1.5 * s];

export function mount(el, reduced) {
  const flat = () => el.classList.add("flat");
  let renderer;
  try {
    renderer = new THREE.WebGLRenderer({ antialias: true, powerPreference: "low-power" });
  } catch {
    flat();
    return { stop() {} };
  }
  renderer.setPixelRatio(Math.min(devicePixelRatio, 1.5));
  renderer.domElement.setAttribute("aria-hidden", "true");
  el.appendChild(renderer.domElement);

  const scene = new THREE.Scene();
  scene.background = new THREE.Color(NIGHT);
  scene.fog = new THREE.Fog(NIGHT, 30, 62);
  const camera = new THREE.PerspectiveCamera(30, 1, 1, 160);

  scene.add(new THREE.HemisphereLight("#7f8fc4", "#2a2233", 1.5));
  const sun = new THREE.DirectionalLight("#ffab73", 1.9);
  sun.position.set(-8, 4, -6);
  scene.add(sun);
  const fire = new THREE.PointLight("#ff9440", 6, 7, 1.6);
  scene.add(fire);

  // The island: hex prisms, sand at the rim, grass inland, a rise behind the cottage.
  const r = rng(6);
  const geo = new THREE.CylinderGeometry(S * 0.97, S * 0.97, 1, 6);
  geo.translate(0, -0.5, 0); // top face at y = 0, scaled down from there
  const side = toon("#5b4636"), sand = toon("#d6bf8a"), grass = [toon("#6f9d4c"), toon("#7aa956"), toon("#628f45")], stone = toon("#8e8a80");
  const tops = new Map();
  const island = new THREE.Group();
  for (const [q, s, d] of hexes(R)) {
    const rim = d === R;
    const rise = Math.max(0, 1.6 - Math.hypot(q + 1, s + 1.5)) * 0.35; // a low hill behind the cottage
    const h = rim ? 0.18 + r() * 0.08 : Math.round((0.4 + rise + r() * 0.12) * 8) / 8;
    const top = rim ? sand : d === R - 1 && r() < 0.25 ? stone : grass[Math.floor(r() * 3)];
    const m = new THREE.Mesh(geo, [side, top, side]);
    const [x, z] = at(q, s);
    m.position.set(x, h, z);
    m.scale.y = h + 1.2; // down under the water line
    island.add(m);
    tops.set(`${q},${s}`, [x, h, z, d]);
  }
  scene.add(island);
  const water = new THREE.Mesh(new THREE.CircleGeometry(140, 48).rotateX(-Math.PI / 2), toon("#2b5480"));
  water.position.y = 0.05;
  scene.add(water);

  // Embers from the fire, drifting up and fading.
  const N = 36;
  const embers = new THREE.Points(
    new THREE.BufferGeometry().setAttribute("position", new THREE.Float32BufferAttribute(new Float32Array(N * 3), 3)),
    new THREE.PointsMaterial({ size: 0.07, color: "#ffc070", transparent: true, opacity: 0.85, depthWrite: false, blending: THREE.AdditiveBlending }),
  );
  const seeds = Array.from({ length: N }, () => [Math.random(), Math.random() * 6.28, 0.25 + Math.random() * 0.5]);
  scene.add(embers);

  let fireAt = [0, 0, 0];
  const put = (gltf, q, s, rot = 0, scale = 1) => {
    const t = tops.get(`${q},${s}`);
    if (!gltf || !t) return null;
    const o = gltf.scene.clone(true);
    prep(o, false);
    o.position.set(t[0], t[1], t[2]);
    o.rotation.y = rot;
    o.scale.setScalar(scale);
    island.add(o);
    return o;
  };
  const loaded = Promise.all(MODELS.map(loadAsset)).then((g) => {
    const m = Object.fromEntries(MODELS.map((n, i) => [n, g[i]]));
    put(m.cottage, -1, -1, 0.5);
    put(m.campfire, 1, 0);
    put(m.lantern, -2, 1, 0.3);
    put(m.waymark, 2, -3, -0.4);
    put(m.flowers, 0, 1);
    put(m.flowers, -3, 2, 1.2);
    const trees = [[1, -3, "tree"], [2, -2, "tree_b"], [-2, -2, "pine"], [-3, 0, "tree"], [-1, -3, "pine"], [3, -1, "tree_b"], [-3, 1, "pine"], [0, -4 + 1, "tree"]];
    for (const [q, s, k] of trees) put(m[k], q, s, r() * 6.28, 0.85 + r() * 0.3);
    for (const [q, s] of [[3, 0], [-1, 3], [1, 2]]) put(m.bush, q, s, r() * 6.28);
    for (const [q, s] of [[4, -2], [-4, 3], [2, 2], [-2, 4]]) put(m.rock, q, s, r() * 6.28, 0.8);
    const t = tops.get("1,0");
    fireAt = [t[0], t[1], t[2]];
    fire.position.set(t[0], t[1] + 0.6, t[2]);
  });

  // Framing: the island sits behind the card fan (left) on a wide screen, high
  // and centred on a tall one, where the menu covers the lower part.
  const resize = () => {
    const w = el.clientWidth || 1, h = el.clientHeight || 1;
    renderer.setSize(w, h);
    camera.aspect = w / h;
    const wide = w > 900 && w > h;
    camera.fov = wide ? 26 : 50;
    camera.setViewOffset(w, h, wide ? w * 0.15 : 0, wide ? -h * 0.16 : h * 0.22, w, h);
    camera.updateProjectionMatrix();
    if (reduced) draw(0);
  };

  let angle = 0.6, raf = 0, last = 0, alive = true;
  const times = [];
  const look = new THREE.Vector3(0, 1.2, 0);
  function draw(t) {
    camera.position.set(Math.sin(angle) * 34, 13, Math.cos(angle) * 34);
    camera.lookAt(look);
    if (!reduced) {
      fire.intensity = 5 + Math.sin(t * 9.1) * 0.8 + Math.sin(t * 23.7) * 0.5;
      const p = embers.geometry.attributes.position;
      for (let i = 0; i < N; i++) {
        const [k, ph, sp] = seeds[i], y = (k + t * sp * 0.25) % 1;
        p.setXYZ(i, fireAt[0] + Math.sin(ph + t * 0.8) * 0.25 * y, fireAt[1] + 0.3 + y * 2.6, fireAt[2] + Math.cos(ph + t * 0.7) * 0.25 * y);
      }
      p.needsUpdate = true;
    }
    embers.visible = !reduced && fireAt[1] > 0;
    renderer.render(scene, camera);
  }
  const loop = (ms) => {
    if (!alive) return;
    const t = ms / 1000, dt = last ? Math.min(t - last, 0.1) : 0;
    last = t;
    angle += dt * ((Math.PI * 2) / 160); // one turn in under three minutes
    draw(t);
    // The frame-time probe: frames 30 to 120. Under 50 fps, the flat backdrop instead.
    if (dt && times.push(dt) === 120) {
      const slow = times.slice(30).sort((a, b) => a - b)[45] > 1 / 50;
      if (slow) return api.stop(), flat();
    }
    raf = requestAnimationFrame(loop);
  };
  const ro = new ResizeObserver(resize);
  ro.observe(el);
  // Fade in once the models are in (or failed: the bare island still reads).
  loaded.finally(() => {
    if (!alive) return;
    resize();
    el.classList.add("live");
    if (!reduced) raf = requestAnimationFrame(loop);
  });

  const api = {
    stop() {
      if (!alive) return;
      alive = false;
      cancelAnimationFrame(raf);
      ro.disconnect();
      scene.traverse((o) => {
        o.geometry?.dispose();
        for (const m of [o.material].flat()) m?.dispose();
      });
      renderer.forceContextLoss(); // the world screen makes its own context next
      renderer.dispose();
      renderer.domElement.remove();
      el.classList.remove("live");
    },
  };
  return api;
}
