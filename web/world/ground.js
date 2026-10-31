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
import { grassTexture, rng, fadeInto } from "./props.js";
import { raised, rise, rockOf, terrainGeometry } from "./terrain.js";
import { bandWeight } from "./ld.js";

const TEX = new URL("./textures/", import.meta.url);
export const REPEAT = { grass: 6, dirt: 3, wood_floor: 2.4, sand: 5, stone: 2.5, // metres per texture tile
  ash: 5, heath: 6, wet_sand: 5, dusk_grass: 6, moss: 5, slate: 3, cave: 4 };
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
        .catch(() => finish((FALLBACK[name] ?? FALLBACK.dirt)())),
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
// "~" and "H" (a pier) water. Anything else shows the base layer. A map's optional
// `paint` rows (same size as `tiles`) paint a layer on any tile, a raised one too.
// `ground.sand` swaps the "s" layer's texture (`"sand": "moss"`). A biome transition at an
// exit (WORLD_RULES 5.7) is a `transitions` band instead (below), not paint.
const CHANNEL = { p: 0, s: 1, "=": 1, r: 2, "~": 3, H: 3 };

// Overlay textures a map uses (the base stands in for the unused ones).
export const groundLayers = (map) => {
  const g = map.ground, t = map.tiles.join("") + (map.paint ?? []).join("");
  return [g.base, g.path ?? g.base, /[s=]/.test(t) ? g.sand ?? "sand" : g.base, t.includes("r") ? "stone" : g.base];
};

// `tile(x, z)` is the map's tile character. `bands` (build.js): the map's biome transitions (#118), each with the
// map next door; the base blends toward that map's base across the band, half and half at the edge (so the two
// sides meet), patchy through the noise. Up to three bands with a base of their own, each a channel of one more
// per-tile mask and one more texture; a map without them pays nothing.
export async function groundMesh(map, tile, bands = []) {
  const { width: W, height: H } = map;
  const gd = map.ground;
  const names = groundLayers(map);
  const [a, b, c, d] = await Promise.all(names.map(layer));
  const blend = bands.filter((t) => t.next.ground.base !== gd.base).slice(0, 3);
  const eTex = await Promise.all(blend.map((t) => layer(t.next.ground.base)));
  let bandTex = null;
  if (blend.length) {
    const wts = new Uint8Array(W * H * 4);
    for (let z = 0; z < H; z++) for (let x = 0; x < W; x++) blend.forEach((t, i) => (wts[(z * W + x) * 4 + i] = Math.round(bandWeight(map, t, x + 0.5, z + 0.5) * 255)));
    bandTex = new THREE.DataTexture(wts, W, H, THREE.RGBAFormat, THREE.UnsignedByteType);
    bandTex.magFilter = bandTex.minFilter = THREE.LinearFilter;
    bandTex.needsUpdate = true;
  }
  const data = new Uint8Array(W * H * 4);
  for (let z = 0; z < H; z++) for (let x = 0; x < W; x++) {
    for (const ch of [CHANNEL[tile(x, z)], CHANNEL[map.paint?.[z]?.[x]]]) if (ch !== undefined) data[(z * W + x) * 4 + ch] = 255;
  }
  const maskTex = new THREE.DataTexture(data, W, H, THREE.RGBAFormat, THREE.UnsignedByteType);
  maskTex.magFilter = maskTex.minFilter = THREE.LinearFilter;
  maskTex.needsUpdate = true;
  // A tide gate (build.js) floods its tiles while shut, so its map always draws water.
  const water = (map.gates ?? []).some((g) => g.tide) || data.some((v, i) => i % 4 === 3 && v);
  // Raised ground (terrain.js): each tile's level, so a layer only paints at its own
  // tile's height (no path or water creeping over a lip), and the rock of the walls.
  const high = raised(map);
  const levels = new Uint8Array(W * H * 4);
  for (let z = 0; z < H; z++) for (let x = 0; x < W; x++) levels[(z * W + x) * 4] = rise(map, x, z) * 25;
  const levelTex = new THREE.DataTexture(levels, W, H, THREE.RGBAFormat, THREE.UnsignedByteType);
  levelTex.needsUpdate = true;

  const m = gd.margin ?? 0;
  const mat = new THREE.MeshLambertMaterial();
  mat.onBeforeCompile = (sh) => {
    Object.assign(sh.uniforms, U, {
      uA: { value: a }, uB: { value: b }, uC: { value: c }, uD: { value: d }, uMask: { value: maskTex }, uCloud: { value: clouds },
      uSize: { value: new THREE.Vector2(W, H) },
      uRep: { value: new THREE.Vector4(...names.map((n) => REPEAT[n] ?? 4)) },
      uClouds: { value: map.light.indoor ? 0 : 1 },
      uLevel: { value: levelTex }, uRock: { value: rockOf(map).map((c) => new THREE.Color(c)) },
      uBand: { value: bandTex }, uRepE: { value: new THREE.Vector3(...[0, 1, 2].map((i) => REPEAT[blend[i]?.next.ground.base] ?? 4)) },
    });
    eTex.forEach((t, i) => (sh.uniforms[`uE${i}`] = { value: t }));
    sh.vertexShader = sh.vertexShader
      .replace("#include <common>", `#include <common>\nvarying vec2 vXZ;\nvarying float vY;${high ? "\nattribute vec2 aRim;\nvarying vec2 vRim;" : ""}`)
      .replace("#include <begin_vertex>", `#include <begin_vertex>\nvec4 wp4 = modelMatrix * vec4(position, 1.0);\nvXZ = wp4.xz;\nvY = wp4.y;${high ? "\nvRim = aRim;" : ""}`);
    sh.fragmentShader = sh.fragmentShader
      .replace(
        "#include <common>",
        `#include <common>
        varying vec2 vXZ;
        varying float vY;
        ${high ? "varying vec2 vRim;" : ""}
        uniform sampler2D uA, uB, uC, uD, uMask, uCloud, uLevel;
        uniform vec3 uRock[6];
        uniform vec2 uSize;
        uniform vec4 uRep;
        uniform float uDrift, uClouds;
        ${blend.length ? `uniform sampler2D uBand${blend.map((_, i) => `, uE${i}`).join("")};\n        uniform vec3 uRepE;` : ""}
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
          ${high ? `
          float lvl = texture2D(uLevel, (floor(vXZ) + 0.5) / uSize).r * 10.2; // 255 / 25
          m *= 1.0 - smoothstep(0.15, 0.4, abs(vY - lvl));` : ""}
          vec4 k = smoothstep(0.4, 0.6, m + (n - 0.5) * 0.35);
          vec3 col = texture2D(uA, vXZ / uRep.x).rgb;
          ${blend.length ? `
          // Biome transitions: the next map's base, in patches that thin out from the edge.
          vec4 tb = texture2D(uBand, (vXZ + warp * 0.55) / uSize);
          float nb = vnoise(vXZ * 0.8 + 11.3) * 0.6 + n * 0.4;
          ${blend.map((_, i) => `col = mix(col, texture2D(uE${i}, vXZ / uRepE.${"xyz"[i]}).rgb, smoothstep(0.25, 0.75, tb.${"rgb"[i]} * 0.5 + 0.5 - nb) * smoothstep(0.0, 0.15, tb.${"rgb"[i]}));`).join("\n          ")}` : ""}
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
          ${high ? `
          // Rock where the field is steep: wavy strata by height, a darker foot.
          vec3 wp = vec3(vXZ.x, vY, vXZ.y);
          float steep = 1.0 - smoothstep(0.5, 0.78, abs(normalize(cross(dFdx(wp), dFdy(wp))).y));
          float band = mod(floor(vY * 2.3 + n * 0.7 + vnoise(vXZ * 0.31) * 1.6), 4.0);
          vec3 rock = band < 0.5 ? uRock[0] : band < 1.5 ? uRock[1] : band < 2.5 ? uRock[2] : uRock[3];
          rock *= 0.86 + 0.28 * vnoise(vec2(vXZ.x + vXZ.y, vY) * vec2(2.7, 7.0));
          // A worn lip round a plateau's top, a contact shadow along a wall's foot and
          // darker rock at the bottom of the wall (terrain.js marks those nodes: aRim).
          float level = 1.0 - steep;
          col = mix(col, uRock[5], level * smoothstep(0.42, 0.5, vRim.x + (n - 0.5) * 0.45) * 0.75);
          col *= 1.0 - level * smoothstep(0.0, 0.8, vRim.y) * 0.35;
          col *= 1.0 + level * min(vY, 3.0) * 0.045; // higher ground a touch lighter, so levels read from above
          rock = mix(rock, uRock[4], vRim.y * 0.7);
          col = mix(col, rock, steep);` : ""}
          col *= 1.0 - 0.2 * uClouds * texture2D(uCloud, vXZ * 0.03 + uDrift * vec2(0.012, 0.007)).r;
          diffuseColor.rgb *= col;
        }`,
      );
    if (high) fadeInto(sh); // a cliff lip between the camera and the hero thins out
  };
  mat.customProgramCacheKey = () => (water ? "ground-w" : "ground") + (high ? "-h" : "") + (blend.length ? `-b${blend.length}` : "");
  let ground;
  if (high) {
    // One mesh for the ground and every plateau (flat-shaded, so the walls facet like
    // the props). Only its raised part casts a shadow, through a copy on layer 2 that
    // the shadow pass draws and the camera (layers 0 and 1) never does.
    const { geo, shadow } = terrainGeometry(map);
    mat.flatShading = true;
    ground = new THREE.Mesh(geo, mat);
    const caster = new THREE.Mesh(shadow, new THREE.MeshBasicMaterial());
    caster.castShadow = true;
    caster.layers.set(2);
    ground.add(caster);
  } else {
    ground = new THREE.Mesh(new THREE.PlaneGeometry(W + 2 * m, H + 2 * m), mat);
    ground.rotation.x = -Math.PI / 2;
    ground.position.set(W / 2, 0, H / 2);
  }
  ground.receiveShadow = true;
  ground.userData.dispose = () => (maskTex.dispose(), levelTex.dispose(), bandTex?.dispose());
  // The tide: these tiles' water, 0 (the flats bare) to 1 (under the sea).
  ground.userData.water = (tiles, level) => {
    const v = Math.round(level * 255);
    for (const i of tiles) data[i * 4 + 3] = v;
    maskTex.needsUpdate = true;
  };
  return ground;
}
