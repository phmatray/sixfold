// One flat ground mesh per map, at y = 0. Its shader blends two painted layers
// (grass and dirt outside, wood inside) through a per-tile mask: an R8 texture,
// one texel per tile, sampled bilinearly and pushed around by world-space noise
// so path edges come out soft and organic instead of square. Cloud shadows
// drift over it outdoors. Textures load from ./textures/<name>.webp, with a
// procedural canvas fallback until those exist.
import * as THREE from "three";
import { U } from "./life.js";
import { grassTexture, rng } from "./props.js";

const TEX = new URL("./textures/", import.meta.url);
const REPEAT = { grass: 6, dirt: 3, wood_floor: 2.4 }; // metres per texture tile
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

// `mask(x, z)` is 1 where the second layer shows (path tiles), else 0.
export async function groundMesh(map, mask) {
  const { width: W, height: H } = map;
  const gd = map.ground;
  const [a, b] = await Promise.all([layer(gd.base), layer(gd.path ?? gd.base)]);
  const data = new Uint8Array(W * H);
  for (let z = 0; z < H; z++) for (let x = 0; x < W; x++) data[z * W + x] = mask(x, z) ? 255 : 0;
  const maskTex = new THREE.DataTexture(data, W, H, THREE.RedFormat, THREE.UnsignedByteType);
  maskTex.magFilter = maskTex.minFilter = THREE.LinearFilter;
  maskTex.needsUpdate = true;

  const m = gd.margin ?? 0;
  const mat = new THREE.MeshLambertMaterial();
  mat.onBeforeCompile = (sh) => {
    Object.assign(sh.uniforms, U, {
      uA: { value: a }, uB: { value: b }, uMask: { value: maskTex }, uCloud: { value: clouds },
      uSize: { value: new THREE.Vector2(W, H) },
      uRep: { value: new THREE.Vector2(REPEAT[gd.base] ?? 4, REPEAT[gd.path ?? gd.base] ?? 4) },
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
        uniform sampler2D uA, uB, uMask, uCloud;
        uniform vec2 uSize, uRep;
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
          float m = texture2D(uMask, (vXZ + warp * 0.55) / uSize).r;
          float k = smoothstep(0.4, 0.6, m + (n - 0.5) * 0.35);
          vec3 col = mix(texture2D(uA, vXZ / uRep.x).rgb, texture2D(uB, vXZ / uRep.y).rgb, k);
          col *= 1.0 - 0.7 * k * (1.0 - k) * (0.6 + 0.4 * n); // a darker, damp rim along the border
          col *= 1.0 - 0.2 * uClouds * texture2D(uCloud, vXZ * 0.03 + uDrift * vec2(0.012, 0.007)).r;
          diffuseColor.rgb *= col;
        }`,
      );
  };
  const ground = new THREE.Mesh(new THREE.PlaneGeometry(W + 2 * m, H + 2 * m), mat);
  ground.rotation.x = -Math.PI / 2;
  ground.position.set(W / 2, 0, H / 2);
  ground.receiveShadow = true;
  ground.userData.dispose = () => maskTex.dispose();
  return ground;
}
