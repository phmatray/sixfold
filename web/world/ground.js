// One flat ground mesh per map, at y = 0. Its shader paints a base layer (grass,
// sand or wood) and up to four overlays through a per-tile mask: an RGBA8
// texture, one texel per tile (R path, G sand, B stone, A water), sampled
// bilinearly and pushed around by world-space noise so borders come out soft
// and organic instead of square. Water is drawn here too (no extra mesh): an
// animated, rippling surface with a foam line and wet sand along the shore.
// Cloud shadows drift over it outdoors. Textures load from ./textures/<name>.webp,
// with a procedural canvas fallback until those exist.
import * as THREE from "three";
import { U } from "./life.js";
import { grassTexture, rng } from "./props.js";

const TEX = new URL("./textures/", import.meta.url);
const REPEAT = { grass: 6, dirt: 3, wood_floor: 2.4, sand: 5, stone: 2.5 }; // metres per texture tile
const cache = new Map();

function finish(t) {
  t.colorSpace = THREE.SRGBColorSpace;
  t.wrapS = t.wrapT = THREE.RepeatWrapping;
  t.anisotropy = 4;
  return t;
}

// The painted texture for a layer, loaded once per session.
export function layer(name) {
  if (!cache.has(name))
    cache.set(
      name,
      new THREE.TextureLoader()
        .loadAsync(new URL(`${name}.webp`, TEX).href)
        .then(finish)
        .catch(() => finish(FALLBACK[name]())),
    );
  return cache.get(name);
}

const FALLBACK = {
  grass: grassTexture,
  sand() {
    const { c, g } = canvas("#e6cf9a");
    const r = rng(29);
    for (let i = 0; i < 400; i++) blot(g, r() * 512, r() * 512, 2 + r() * 14, r() < 0.5 ? "#d8bd84" : "#f2e0b4", 0.3);
    return new THREE.CanvasTexture(c);
  },
  stone() {
    const { c, g } = canvas("#9c968a");
    const r = rng(31);
    for (let y = 0; y < 512; y += 64)
      for (let x = (y / 64) % 2 ? 32 : 0; x < 512 + 32; x += 64) blot(g, x, y + 32, 26 + r() * 4, ["#b3ad9f", "#a8a294", "#c0b9aa"][Math.floor(r() * 3)], 1);
    return new THREE.CanvasTexture(c);
  },
  dirt() {
    const { c, g } = canvas("#c99d62");
    const r = rng(17);
    for (let i = 0; i < 160; i++) blot(g, r() * 512, r() * 512, 10 + r() * 40, r() < 0.5 ? "#b78850" : "#dcb57c", 0.25);
    for (let i = 0; i < 500; i++) blot(g, r() * 512, r() * 512, 1.5 + r() * 3, r() < 0.6 ? "#8f6a40" : "#ecd2a0", 0.5);
    return new THREE.CanvasTexture(c);
  },
  wood_floor() {
    const { c, g } = canvas("#a8743f");
    const r = rng(23);
    const plank = 512 / 10;
    for (let row = 0; row < 10; row++) {
      g.fillStyle = ["#a8743f", "#b07c45", "#9c6b3a", "#b4844d"][row % 4];
      g.fillRect(0, row * plank, 512, plank);
      g.globalAlpha = 0.18;
      g.strokeStyle = "#6e4824";
      for (let k = 0; k < 6; k++) {
        const y = row * plank + 6 + r() * (plank - 12);
        g.beginPath();
        g.moveTo(0, y);
        g.bezierCurveTo(170, y + (r() - 0.5) * 6, 340, y + (r() - 0.5) * 6, 512, y);
        g.stroke();
      }
      g.globalAlpha = 0.6;
      g.fillStyle = "#5e3c1e";
      g.fillRect(0, row * plank, 512, 2); // seam
      const cut = (row * 197) % 512;
      g.fillRect(cut, row * plank, 2, plank); // butt joint
      g.globalAlpha = 1;
    }
    return new THREE.CanvasTexture(c);
  },
};

function canvas(base) {
  const c = document.createElement("canvas");
  c.width = c.height = 512;
  const g = c.getContext("2d");
  g.fillStyle = base;
  g.fillRect(0, 0, 512, 512);
  return { c, g };
}

function blot(g, x, y, rad, color, alpha) {
  g.globalAlpha = alpha;
  g.fillStyle = color;
  for (const dx of [-512, 0, 512])
    for (const dy of [-512, 0, 512]) {
      g.beginPath();
      g.ellipse(x + dx, y + dy, rad, rad * 0.7, 0.4, 0, Math.PI * 2);
      g.fill();
    }
  g.globalAlpha = 1;
}

// Tiling cloud noise, white blobs on black (read from the red channel).
const clouds = (() => {
  const c = document.createElement("canvas");
  c.width = c.height = 256;
  const g = c.getContext("2d");
  g.fillStyle = "#000";
  g.fillRect(0, 0, 256, 256);
  const r = rng(11);
  for (let i = 0; i < 14; i++) {
    const x = r() * 256, y = r() * 256, rad = 40 + r() * 70;
    for (const dx of [-256, 0, 256])
      for (const dy of [-256, 0, 256]) {
        const grd = g.createRadialGradient(x + dx, y + dy, 0, x + dx, y + dy, rad);
        grd.addColorStop(0, "rgba(255,255,255,0.55)");
        grd.addColorStop(1, "rgba(255,255,255,0)");
        g.fillStyle = grd;
        g.fillRect(0, 0, 256, 256);
      }
  }
  const t = new THREE.CanvasTexture(c);
  t.wrapS = t.wrapT = THREE.RepeatWrapping;
  return t;
})();

// Ground layers per tile character: the overlay channel each one paints.
// "p" path (the map's `ground.path` layer), "s" and "=" sand, "r" stone,
// "~" and "H" (a pier) water. Anything else shows the base layer.
const CHANNEL = { p: 0, s: 1, "=": 1, r: 2, "~": 3, H: 3 };

// Overlay textures a map uses (the base stands in for the unused ones).
export const groundLayers = (map) => {
  const g = map.ground, t = map.tiles.join("");
  return [g.base, g.path ?? g.base, /[s=]/.test(t) ? "sand" : g.base, t.includes("r") ? "stone" : g.base];
};

// `tile(x, z)` is the map's tile character.
export async function groundMesh(map, tile) {
  const { width: W, height: H } = map;
  const gd = map.ground;
  const names = groundLayers(map);
  const [a, b, c, d] = await Promise.all(names.map(layer));
  const data = new Uint8Array(W * H * 4);
  for (let z = 0; z < H; z++) for (let x = 0; x < W; x++) {
    const ch = CHANNEL[tile(x, z)];
    if (ch !== undefined) data[(z * W + x) * 4 + ch] = 255;
  }
  const maskTex = new THREE.DataTexture(data, W, H, THREE.RGBAFormat, THREE.UnsignedByteType);
  maskTex.magFilter = maskTex.minFilter = THREE.LinearFilter;
  maskTex.needsUpdate = true;
  const water = data.some((v, i) => i % 4 === 3 && v);

  const m = gd.margin ?? 0;
  const mat = new THREE.MeshLambertMaterial();
  mat.onBeforeCompile = (sh) => {
    Object.assign(sh.uniforms, U, {
      uA: { value: a }, uB: { value: b }, uC: { value: c }, uD: { value: d }, uMask: { value: maskTex }, uCloud: { value: clouds },
      uSize: { value: new THREE.Vector2(W, H) },
      uRep: { value: new THREE.Vector4(...names.map((n) => REPEAT[n] ?? 4)) },
      uClouds: { value: map.light.indoor ? 0 : 1 },
    });
    sh.vertexShader = sh.vertexShader
      .replace("#include <common>", "#include <common>\nvarying vec2 vXZ;")
      .replace("#include <begin_vertex>", "#include <begin_vertex>\nvXZ = (modelMatrix * vec4(position, 1.0)).xz;");
    sh.fragmentShader = sh.fragmentShader
      .replace(
        "#include <common>",
        `#include <common>
        varying vec2 vXZ;
        uniform sampler2D uA, uB, uC, uD, uMask, uCloud;
        uniform vec2 uSize;
        uniform vec4 uRep;
        uniform float uDrift, uClouds;
        float h21(vec2 p) { return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453); }
        float vnoise(vec2 p) {
          vec2 i = floor(p), f = fract(p), u = f * f * (3.0 - 2.0 * f);
          return mix(mix(h21(i), h21(i + vec2(1, 0)), u.x), mix(h21(i + vec2(0, 1)), h21(i + vec2(1, 1)), u.x), u.y);
        }`,
      )
      .replace(
        "#include <map_fragment>",
        `{
          float n = vnoise(vXZ * 1.7) * 0.65 + vnoise(vXZ * 4.3) * 0.35;
          vec2 warp = vec2(vnoise(vXZ * 0.9 + 7.1), vnoise(vXZ * 0.9 - 3.7)) - 0.5;
          vec4 m = texture2D(uMask, (vXZ + warp * 0.55) / uSize);
          vec4 k = smoothstep(0.4, 0.6, m + (n - 0.5) * 0.35);
          vec3 col = texture2D(uA, vXZ / uRep.x).rgb;
          col = mix(col, texture2D(uC, vXZ / uRep.z).rgb, k.g);
          col = mix(col, texture2D(uB, vXZ / uRep.y).rgb, k.r);
          col = mix(col, texture2D(uD, vXZ / uRep.w).rgb, k.b);
          float rim = max(max(k.r * (1.0 - k.r), k.b * (1.0 - k.b)), 0.0);
          col *= 1.0 - 0.7 * rim * (0.6 + 0.4 * n); // a darker, damp rim along path borders
          ${water ? `
          // Water: wet sand toward the shore, a foam line, then shallow to deep with drifting ripples.
          float w = m.a + (n - 0.5) * 0.3;
          col *= 1.0 - 0.28 * smoothstep(0.18, 0.42, w);
          float r1 = vnoise(vXZ * 1.1 + uDrift * vec2(0.21, 0.13)), r2 = vnoise(vXZ * 2.3 - uDrift * vec2(0.17, 0.27));
          float rip = r1 * 0.6 + r2 * 0.4;
          vec3 wat = mix(vec3(0.36, 0.74, 0.76), vec3(0.13, 0.45, 0.6), smoothstep(0.55, 1.0, m.a) * (0.75 + 0.25 * rip));
          wat += vec3(0.85, 0.95, 1.0) * 0.32 * smoothstep(0.72, 0.9, rip) * smoothstep(0.6, 0.9, m.a);
          float edge = smoothstep(0.42, 0.5, w);
          col = mix(col, wat, edge);
          float foam = smoothstep(0.1, 0.0, abs(w - 0.5 - 0.035 * sin(uDrift * 1.3 + n * 6.0)));
          col = mix(col, vec3(0.96, 0.98, 0.95), foam * 0.8);` : ""}
          col *= 1.0 - 0.2 * uClouds * texture2D(uCloud, vXZ * 0.03 + uDrift * vec2(0.012, 0.007)).r;
          diffuseColor.rgb *= col;
        }`,
      );
  };
  mat.customProgramCacheKey = () => (water ? "ground-w" : "ground");
  const ground = new THREE.Mesh(new THREE.PlaneGeometry(W + 2 * m, H + 2 * m), mat);
  ground.rotation.x = -Math.PI / 2;
  ground.position.set(W / 2, 0, H / 2);
  ground.receiveShadow = true;
  ground.userData.dispose = () => maskTex.dispose();
  return ground;
}
