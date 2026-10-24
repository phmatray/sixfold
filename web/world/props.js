// Shared building blocks: seeded RNG, toon materials, placeholder models (same
// footprints, origins and +Z facing as the asset contract in ASSETS.md), the
// fence/wall auto-tile table, and `prep` for loaded models.
import * as THREE from "three";

// Small seeded RNG so the meadow looks the same every visit.
export function rng(seed) {
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
export const toon = (color, extra = {}) => new THREE.MeshToonMaterial({ color, gradientMap: ramp, ...extra });


export function grassTexture() {
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

export const mesh = (geo, mat, x = 0, y = 0, z = 0) => {
  const m = new THREE.Mesh(geo, mat);
  m.position.set(x, y, z);
  return m;
};
export const group = (...kids) => {
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

export const PLACEHOLDER = {
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

  // ----- Interior (assets/interior/) -----
  counter() {
    const wood = toon("#9a6a40");
    return group(
      mesh(new THREE.BoxGeometry(2.9, 0.95, 0.8), wood, 0, 0.475, 0),
      mesh(new THREE.BoxGeometry(3.0, 0.08, 0.95), toon("#c99a62"), 0, 0.99, 0),
      mesh(new THREE.BoxGeometry(0.5, 0.25, 0.35), toon("#e8d7b0"), 0.8, 1.15, 0), // a till
    );
  },
  shelf() {
    const g = group(mesh(new THREE.BoxGeometry(1.9, 2.0, 0.45), toon("#7e5636"), 0, 1.0, -0.25));
    const goods = ["#e2553a", "#f3d589", "#4f9fd6", "#79c26a", "#c77dd8"];
    for (let row = 0; row < 3; row++)
      for (let i = 0; i < 5; i++) g.add(mesh(new THREE.BoxGeometry(0.26, 0.3, 0.22), toon(goods[(i + row * 2) % 5]), -0.7 + i * 0.35, 0.45 + row * 0.6, -0.05));
    return g;
  },
  duel_table() {
    const wood = toon("#8b5e3a"), leg = (x, z) => mesh(new THREE.BoxGeometry(0.1, 0.72, 0.1), wood, x, 0.36, z);
    const g = group(mesh(new THREE.BoxGeometry(1.85, 0.08, 0.85), toon("#2f6b4f"), 0, 0.76, 0), leg(-0.8, -0.32), leg(0.8, -0.32), leg(-0.8, 0.32), leg(0.8, 0.32));
    for (const [x, z, r] of [[-0.45, 0, 0.1], [-0.2, 0.05, -0.2], [0.35, -0.05, 0.25], [0.6, 0.08, -0.1]]) {
      const c = mesh(new THREE.BoxGeometry(0.18, 0.012, 0.26), toon("#f4efe2"), x, 0.81, z);
      c.rotation.y = r;
      g.add(c);
    }
    return g;
  },
  chair() {
    const wood = toon("#a0703f");
    return group(mesh(new THREE.BoxGeometry(0.5, 0.06, 0.5), wood, 0, 0.45, 0), mesh(new THREE.BoxGeometry(0.5, 0.55, 0.06), wood, 0, 0.75, -0.22),
      ...[[-0.2, -0.2], [0.2, -0.2], [-0.2, 0.2], [0.2, 0.2]].map(([x, z]) => mesh(new THREE.BoxGeometry(0.06, 0.45, 0.06), wood, x, 0.22, z)));
  },
  rug() {
    // 2 cm thick, so its top never shares a plane with the floor.
    return group(mesh(new THREE.BoxGeometry(2.8, 0.02, 1.8), toon("#b0473c"), 0, 0.011, 0), mesh(new THREE.BoxGeometry(2.2, 0.022, 1.2), toon("#e2b65a"), 0, 0.012, 0));
  },
  plant() {
    return group(mesh(new THREE.CylinderGeometry(0.22, 0.17, 0.4, 10), toon("#c5683f"), 0, 0.2, 0), ...blobs(toon("#3f9a3a"), [[0.32, 0, 0.62, 0], [0.24, 0.15, 0.85, 0.05], [0.22, -0.14, 0.8, -0.05]]));
  },
  display_case() {
    const g = group(mesh(new THREE.BoxGeometry(0.85, 0.8, 0.7), toon("#7e5636"), 0, 0.4, 0), mesh(new THREE.BoxGeometry(0.3, 0.12, 0.2), toon("#f3d589"), 0, 0.86, 0));
    g.add(mesh(new THREE.BoxGeometry(0.8, 0.45, 0.65), new THREE.MeshLambertMaterial({ color: "#cfefff", transparent: true, opacity: 0.25, depthWrite: false }), 0, 1.03, 0));
    return g;
  },
  lamp() {
    return group(mesh(new THREE.CylinderGeometry(0.18, 0.22, 0.06, 12), toon("#5b4a3a"), 0, 0.03, 0), mesh(new THREE.CylinderGeometry(0.03, 0.03, 1.4, 6), toon("#5b4a3a"), 0, 0.72, 0),
      mesh(new THREE.CylinderGeometry(0.16, 0.3, 0.32, 12, 1, true), new THREE.MeshBasicMaterial({ color: "#ffd9a0", side: THREE.DoubleSide }), 0, 1.5, 0));
  },
  exit_mat() {
    return group(mesh(new THREE.BoxGeometry(0.9, 0.02, 0.62), toon("#5d7a4a"), 0, 0.011, 0));
  },
  // A few blades for the edge of a path (until grass_edge.glb exists).
  grass_edge() {
    const m = toon("#4fae3a");
    const g = group();
    for (let i = 0; i < 4; i++) {
      const b = mesh(new THREE.ConeGeometry(0.045, 0.3 + i * 0.04, 3).translate(0, 0.16 + i * 0.02, 0), m, (i - 1.5) * 0.06, 0, ((i * 7) % 3 - 1) * 0.04);
      b.rotation.z = (i - 1.5) * 0.18;
      g.add(b);
    }
    return g;
  },
  // ----- Items -----
  shoes() {
    const shoe = (x) => group(mesh(new THREE.BoxGeometry(0.16, 0.12, 0.34), toon("#e2553a"), 0, 0.08, 0), mesh(new THREE.BoxGeometry(0.17, 0.04, 0.36), toon("#f4efe2"), 0, 0.02, 0.01), mesh(new THREE.BoxGeometry(0.14, 0.03, 0.08), toon("#f3d589"), 0, 0.15, 0.06)).translateX(x);
    return group(shoe(-0.11), shoe(0.11));
  },
};

// A post plus arms toward each neighbour ([dx, dz] in world space, so no rotation
// needed): fence rails, or wall slabs (full height, or low on the camera side).
export function armsPiece(kind, arms) {
  if (kind === "fence") {
    const wood = toon("#c89a62");
    const g = group(mesh(new THREE.BoxGeometry(0.14, 0.9, 0.14), wood, 0, 0.45, 0));
    for (const [dx, dz] of arms)
      for (const y of [0.33, 0.66]) g.add(mesh(new THREE.BoxGeometry(dx ? 0.5 : 0.05, 0.09, dz ? 0.5 : 0.05), wood, dx * 0.25, y, dz * 0.25));
    return g;
  }
  const h = kind === "wall_low" ? 0.45 : 2.6, t = 0.3;
  if (kind === "wall_window") {
    const g = armsPiece("wall", arms);
    g.add(mesh(new THREE.BoxGeometry(0.7, 0.8, t + 0.04), toon("#bfe6ff"), 0, 1.55, 0)); // the pane
    return g;
  }
  const plaster = toon(kind === "wall_low" ? "#c9a77c" : "#efe1c6");
  const g = group(mesh(new THREE.BoxGeometry(t, h, t), plaster, 0, h / 2, 0));
  for (const [dx, dz] of arms) g.add(mesh(new THREE.BoxGeometry(dx ? 0.5 : t, h, dz ? 0.5 : t), plaster, dx * 0.25, h / 2, dz * 0.25));
  g.add(mesh(new THREE.BoxGeometry(t + 0.04, 0.06, t + 0.04), toon("#8a5f3c"), 0, h + 0.03, 0)); // trim on top
  for (const [dx, dz] of arms) g.add(mesh(new THREE.BoxGeometry(dx ? 0.5 : t + 0.04, 0.06, dz ? 0.5 : t + 0.04), toon("#8a5f3c"), dx * 0.25, h + 0.03, dz * 0.25));
  return g;
}

// Footprint in tiles [along X, along Z] at rot 0.
export const FOOT = {
  house: [7, 5], tree: [2, 2], bush: [1, 1], flowers: [1, 1], rock: [1, 1], sign: [1, 1],
  counter: [3, 1], shelf: [2, 1], duel_table: [2, 1], chair: [1, 1], rug: [3, 2], plant: [1, 1],
  display_case: [1, 1], lamp: [1, 1], exit_mat: [1, 1],
};
// Props the hero walks over; every other prop blocks its footprint.
export const WALKABLE = new Set(["rug", "exit_mat"]);
// Sway reference height per prop (life.js wind).
export const WIND_H = { tree: 3.5, bush: 0.8, flowers: 0.4 };

// Auto-tiling for fences ("f") and walls ("W"): neighbour mask (N=1, E=2, S=4, W=8)
// -> [piece suffix, rot]. At rot 0: "" = arms W+E, _corner = W+S, _end = W,
// _t = W+E+S. Mask 15 adds an _end toward N. Same table in ASSETS.md, in the
// Blender scene script, and checked by tools/check-fence-table.mjs.
// (world.js places the pieces.)
export const AUTOTILE = [
  ["", 0], ["_end", 270], ["_end", 180], ["_corner", 180],
  ["_end", 90], ["", 90], ["_corner", 90], ["_t", 90],
  ["_end", 0], ["_corner", 270], ["", 0], ["_t", 180],
  ["_corner", 0], ["_t", 270], ["_t", 0], ["_t", 0],
];
// Shadows on, and loaded glTF materials swapped to toon so they match the placeholders.
export function prep(root, cast) {
  root.traverse((o) => {
    if (!o.isMesh) return;
    o.castShadow = cast;
    o.receiveShadow = true;
    if (!(o.material instanceof THREE.MeshToonMaterial) && !(o.material instanceof THREE.MeshLambertMaterial) && !(o.material instanceof THREE.MeshBasicMaterial)) {
      const m = o.material;
      o.material = toon(m.color ?? "#ffffff", { map: m.map ?? null, transparent: m.transparent, alphaTest: m.alphaTest, opacity: m.opacity });
    }
    // COLOR_0 is a wind weight (read by the wind shader), never a tint.
    o.material.vertexColors = false;
  });
}
