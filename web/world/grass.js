// Grass: low-poly clumps of tapered, bent blades fanned round a centre, one
// InstancedMesh per variant per map (short clumps, taller stems, blooms). A
// clump's base takes the colour of the ground texture right under it and fades
// to a lighter, regional tip, so it grows out of the ground with no footprint;
// every normal points up, so blades light (and shadow) like the ground they
// stand on, by day and by night. Wind comes from life.js (`windify`), weighted
// by height along the blade, off with reduced motion. Scatter: soft drifts from
// value noise, denser along paths, fences, walls and cliffs, only on open tiles.
import * as THREE from "three";
import { rng } from "./props.js";
import { layer, REPEAT } from "./ground.js";
import { windify } from "./life.js";

const SIDES = [[0, -1], [1, 0], [0, 1], [-1, 0]];
const WIND = 1.1; // sway amplitude scale (tips move a few cm, gusts ~10 cm)

// Per region: tip colour, clump height (m), density, blades per clump, share of
// tall stems, bloom colours. A sand base (Gullhaven) grows dune grass instead.
const STYLE = {
  meadow: { tip: "#b8e374", h: 0.3, n: 1, blades: 9, tall: 0.04, bloom: ["#ffffff", "#ffd2b8", "#fff1a0"] },
  grove: { tip: "#8fcf5a", h: 0.34, n: 1.15, blades: 9, tall: 0.08, bloom: ["#ffffff", "#c9dcff"] },
  tide: { tip: "#9ed69a", h: 0.36, n: 1, blades: 8, tall: 0.1, bloom: ["#f6c3d2", "#ffffff"] },
  dune: { tip: "#e3d79b", h: 0.42, n: 0.8, blades: 6, tall: 0.18, thin: true, bloom: [] },
  storm: { tip: "#cdbd72", h: 0.26, n: 0.85, blades: 7, tall: 0.05, thin: true, bloom: ["#b98ad4", "#d9b3e8"] },
  dream: { tip: "#8fb2d8", h: 0.3, n: 1, blades: 7, tall: 0.06, bloom: ["#e9dcff", "#bfe8ff"] },
  ember: { tip: "#a8a874", h: 0.2, n: 0.4, blades: 5, tall: 0, thin: true, bloom: [] },
};
const styleOf = (map) => (map.ground.base === "sand" ? STYLE.dune : STYLE[map.region] ?? STYLE.meadow);

// One clump: `blades` tapered blades (base, bent middle, tip: 3 triangles each)
// leaning outward. `gh` is the height fraction (colour and wind weight).
function clumpGeo(r, blades, h, width) {
  const pos = [], gh = [], index = [];
  for (let b = 0; b < blades; b++) {
    const a = r() * Math.PI * 2; // uneven, so a clump never reads as a star
    const ca = Math.cos(a), sa = Math.sin(a);
    const t = 0.55 + r() * 0.45; // this blade's share of the clump height
    const lean = (0.08 + r() * 0.32) * h * t, w = width * (0.8 + r() * 0.4), off = 0.01 + r() * 0.05;
    const v = pos.length / 3;
    const row = (y, half, out) => {
      const cx = ca * (off + out), cz = sa * (off + out);
      pos.push(cx - sa * half, y, cz + ca * half, cx + sa * half, y, cz - ca * half);
      gh.push(y / h, y / h);
    };
    row(0, w, 0);
    row(0.55 * h * t, w * 0.6, lean * 0.3);
    pos.push(ca * (off + lean), 0.93 * h * t, sa * (off + lean));
    gh.push(0.93 * t);
    index.push(v, v + 1, v + 3, v, v + 3, v + 2, v + 2, v + 3, v + 4);
  }
  const g = new THREE.BufferGeometry();
  g.setAttribute("position", new THREE.Float32BufferAttribute(pos, 3));
  g.setAttribute("normal", new THREE.Float32BufferAttribute(pos.map((_, i) => (i % 3 === 1 ? 1 : 0)), 3));
  g.setAttribute("gh", new THREE.Float32BufferAttribute(gh, 1));
  g.setAttribute("windw", new THREE.Float32BufferAttribute(gh.map((f) => f * Math.sqrt(h / 0.3)), 1)); // life.js squares it: taller blades sway further
  g.setIndex(index);
  return g;
}

// Base colour from the instance colour (the ground under it), tip from `gtip`;
// normals stay up on both faces.
function grassMaterial() {
  const mat = new THREE.MeshLambertMaterial({ side: THREE.DoubleSide });
  mat.userData.patch = (sh) => {
    sh.vertexShader = sh.vertexShader
      .replace("#include <common>", "#include <common>\nattribute float gh;\nattribute vec3 gtip;\nvarying float vGH;\nvarying vec3 vTip;")
      .replace("#include <color_vertex>", "#include <color_vertex>\nvGH = gh;\nvTip = gtip;");
    sh.fragmentShader = sh.fragmentShader
      .replace("#include <common>", "#include <common>\nvarying float vGH;\nvarying vec3 vTip;")
      .replace("#include <color_fragment>", "diffuseColor.rgb *= mix(vColor.rgb, vTip, smoothstep(0.0, 1.0, vGH));")
      .replace("#include <normal_fragment_begin>", "#include <normal_fragment_begin>\nnormal = normalize(vNormal);");
  };
  return mat;
}

const hash = (x, z) => {
  const s = Math.sin(x * 127.1 + z * 311.7) * 43758.5453;
  return s - Math.floor(s);
};
function noise(x, z) {
  const ix = Math.floor(x), iz = Math.floor(z), fx = x - ix, fz = z - iz;
  const u = fx * fx * (3 - 2 * fx), v = fz * fz * (3 - 2 * fz);
  const a = hash(ix, iz), b = hash(ix + 1, iz), c = hash(ix, iz + 1), d = hash(ix + 1, iz + 1);
  return a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v;
}
const smooth = (a, b, x) => {
  const t = Math.min(1, Math.max(0, (x - a) / (b - a)));
  return t * t * (3 - 2 * t);
};

// The base layer's colour at a world point (the ground shader samples it at xz / repeat, flipY).
async function groundSampler(base) {
  const tex = await layer(base);
  const S = 64, c = document.createElement("canvas");
  c.width = c.height = S;
  const g = c.getContext("2d", { willReadFrequently: true });
  g.drawImage(tex.image, 0, 0, S, S);
  const px = g.getImageData(0, 0, S, S).data, rep = REPEAT[base] ?? 4;
  const frac = (v) => v - Math.floor(v);
  return (x, z, col) => {
    const i = (Math.floor((1 - frac(z / rep)) * S) % S) * S + (Math.floor(frac(x / rep) * S) % S);
    return col.setRGB(px[i * 4] / 255, px[i * 4 + 1] / 255, px[i * 4 + 2] / 255, THREE.SRGBColorSpace);
  };
}

// `open(x, z)`: grass may grow on this tile; `tile(x, z)`: its map character.
// `density` scales the whole field (the map's `ground.tufts` / 1800); `rise(x, z)`: the
// tile's ground height, so tufts stand on plateau tops too.
export async function grass(root, map, tile, open, density = 1, rise = () => 0) {
  const st = styleOf(map), k = Math.sqrt(density) * st.n; // sqrt: a sparse map still gets drifts
  const { width: W, height: H } = map;
  const r = rng(W * 977 + H * 31 + 5);
  const ground = await groundSampler(map.ground.base);
  const short = [], tall = [], blooms = [];
  const poisson = (l) => Math.floor(l) + (r() < l % 1 ? 1 : 0);
  let y = 0; // the current tile's height
  const put = (x, z, edge) => {
    const list = r() < st.tall * (edge ? 2.5 : 1) ? tall : short;
    list.push({ x, y, z, s: edge ? 1.05 + r() * 0.35 : 0.75 + r() * 0.5 });
    if (st.bloom.length && !edge && r() < 0.07) blooms.push({ x: x + (r() - 0.5) * 0.1, y, z: z + (r() - 0.5) * 0.1 });
  };
  for (let z = 0; z < H; z++)
    for (let x = 0; x < W; x++) {
      if (!open(x, z)) continue;
      y = rise(x, z);
      // Soft drifts: patches of thick grass, thin lawn between.
      const drift = smooth(0.38, 0.78, noise(x * 0.3, z * 0.3) * 0.7 + noise(x * 0.85 + 9.1, z * 0.85 - 3.3) * 0.3);
      for (let n = poisson(k * (0.12 + 3.2 * drift)); n--; ) {
        const cx = x + 0.1 + r() * 0.8, cz = z + 0.1 + r() * 0.8;
        for (let m = 1 + Math.floor(r() * 4 * drift); m--; ) put(cx + (r() - 0.5) * 0.3, cz + (r() - 0.5) * 0.3, false);
      }
      // Borders: a fringe along paths, thicker at the foot of fences, walls, cliffs and props.
      for (const [dx, dz] of SIDES) {
        const t = tile(x + dx, z + dz);
        if (open(x + dx, z + dz) || "~H".includes(t ?? "-")) continue;
        const path = "pr".includes(t ?? "-");
        for (let n = poisson(k * (path ? 3.2 : 3)); n--; ) {
          const along = r() - 0.5, depth = path ? 0.34 + r() * 0.14 : 0.32 + r() * 0.16;
          put(x + 0.5 + dx * depth + (dz ? along : 0), z + 0.5 + dz * depth + (dx ? along : 0), true);
        }
      }
    }

  const mat = grassMaterial();
  const tip = new THREE.Color(st.tip), col = new THREE.Color(), m = new THREE.Matrix4(), q = new THREE.Quaternion(), e = new THREE.Euler(), s = new THREE.Vector3(), v = new THREE.Vector3();
  const field = (list, geo) => {
    if (!list.length) return;
    const inst = new THREE.InstancedMesh(geo, mat, list.length);
    const tips = new Float32Array(list.length * 3);
    list.forEach((p, i) => {
      e.set((r() - 0.5) * 0.25, r() * Math.PI * 2, (r() - 0.5) * 0.25);
      const xz = 0.85 + r() * 0.35;
      inst.setMatrixAt(i, m.compose(v.set(p.x, p.y, p.z), q.setFromEuler(e), s.set(xz, p.s, xz)));
      inst.setColorAt(i, ground(p.x, p.z, col));
      col.copy(tip).offsetHSL((r() - 0.5) * 0.04, (r() - 0.5) * 0.1, (r() - 0.5) * 0.1);
      tips.set([col.r, col.g, col.b], i * 3);
    });
    geo.setAttribute("gtip", new THREE.InstancedBufferAttribute(tips, 3));
    inst.computeBoundingSphere();
    inst.receiveShadow = true;
    windify(inst, WIND);
    root.add(inst);
  };
  field(short, clumpGeo(rng(3), st.blades, st.h, st.thin ? 0.026 : 0.036));
  field(tall, clumpGeo(rng(4), 4, st.h * 2.1, 0.02));
  // windify set the wind; add the colour gradient on top (one program for both meshes).
  const wind = mat.onBeforeCompile;
  mat.onBeforeCompile = (sh, gl) => (wind(sh, gl), mat.userData.patch(sh));
  mat.customProgramCacheKey = () => "grass";

  // Blooms: tiny flower heads resting in some clumps, swaying with their tips.
  if (blooms.length) {
    const geo = new THREE.SphereGeometry(0.045, 6, 4);
    geo.setAttribute("windw", new THREE.Float32BufferAttribute(new Array(geo.attributes.position.count).fill(0.8), 1));
    const bloom = new THREE.InstancedMesh(geo, new THREE.MeshLambertMaterial(), blooms.length);
    blooms.forEach((p, i) => {
      bloom.setMatrixAt(i, m.compose(v.set(p.x, p.y + st.h * (0.6 + r() * 0.3), p.z), q.identity(), s.set(1, 0.6, 1)));
      bloom.setColorAt(i, col.set(st.bloom[Math.floor(r() * st.bloom.length)]));
    });
    bloom.computeBoundingSphere();
    bloom.receiveShadow = true;
    windify(bloom, WIND);
    root.add(bloom);
  }
}
