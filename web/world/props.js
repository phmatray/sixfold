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

// Scenery that stands between the camera and the hero thins out round the hero (an
// ordered dither, down to ~30 % at the centre), above the hero's waist only; the hero
// is never drawn through anything. `CUTAWAY.hero` (the hero's centre) is set every frame
// by world.js; one shared uniform, so nothing recompiles as the hero moves.
export const CUTAWAY = { hero: { value: new THREE.Vector3(0, -1e4, 0) } };
export function fadeInto(sh) {
  sh.uniforms.uFadeHero = CUTAWAY.hero;
  sh.vertexShader = sh.vertexShader
    .replace("#include <common>", "#include <common>\nvarying vec3 vFadeW;")
    .replace("#include <project_vertex>", `#include <project_vertex>
      vec4 fadeW = vec4(transformed, 1.0);
      #ifdef USE_INSTANCING
        fadeW = instanceMatrix * fadeW;
      #endif
      vFadeW = (modelMatrix * fadeW).xyz;`);
  sh.fragmentShader = sh.fragmentShader
    .replace("#include <common>", "#include <common>\nvarying vec3 vFadeW;\nuniform vec3 uFadeHero;")
    .replace("#include <clipping_planes_fragment>", `#include <clipping_planes_fragment>
      {
        vec3 ray = uFadeHero - cameraPosition, p = vFadeW - cameraPosition;
        float len = length(ray), t = dot(p, ray) / len;
        float k = (1.0 - smoothstep(0.7, 1.15, length(p - ray * (t / len)))) * step(t, len - 0.45) * step(uFadeHero.y - 0.5, vFadeW.y);
        ivec2 q = ivec2(mod(gl_FragCoord.xy, 4.0));
        const float B[16] = float[16](0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0);
        if ((B[q.y * 4 + q.x] + 0.5) / 16.0 < k * 0.7) discard;
      }`);
}
// Opt a material into the fade, once (a wind or water hook it already has runs first).
export function fadeable(mat) {
  if (mat.onBeforeCompile.fade) return;
  const prev = mat.onBeforeCompile;
  const key = Object.hasOwn(mat, "customProgramCacheKey") && !mat.customProgramCacheKey.fade ? mat.customProgramCacheKey.bind(mat) : () => prev.toString();
  mat.onBeforeCompile = Object.assign((sh, r) => (prev.call(mat, sh, r), fadeInto(sh)), { fade: true });
  mat.customProgramCacheKey = Object.assign(() => `${key()}|fade`, { fade: true });
  mat.needsUpdate = true;
}


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

// The six seals' colours in the campaign's order (the .t-* swatches in style.css).
export const SEAL_ORDER = ["#3f9a52", "#2f7fd0", "#e1572f", "#e2ad19", "#a8642f", "#8a4fc9"];

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
  // The sealed door to the Warden (#155), set in the duel hall's wall: a dark leaf in a stone frame, a brass
  // hexagon on it, and over it a stone medallion standing out from the wall, toward the camera, with six
  // hexagons carved round a brass one, one per seal. As the door rings, `inlay_<i>` lights in that seal's
  // colour (world.js `ringDoor`, in the campaign's order). The leaf is a `door`: it swings when taken.
  seal_door() {
    const stone = toon("#9a9080"), brass = toon("#f3d589");
    const hex = (r, x, y, z, mat) => {
      const m = mesh(new THREE.CylinderGeometry(r, r, 0.05, 6), mat, x, y, z);
      m.rotation.set(Math.PI / 6, 0, Math.PI / 2); // flat to the leaf's side, toward the camera once turned
      return m;
    };
    const leaf = group(mesh(new THREE.BoxGeometry(0.8, 2.15, 0.12), toon("#4a3426"), 0.4, 1.075, 0));
    const boss = mesh(new THREE.CylinderGeometry(0.14, 0.14, 0.04, 6), brass, 0.4, 1.3, 0.07);
    boss.rotation.x = Math.PI / 2;
    leaf.add(boss);
    leaf.name = "door";
    leaf.position.set(-0.4, 0, 0.02);
    const [cy, cz] = [2.2, 0.48];
    const medal = group(hex(0.36, 0, cy, cz, stone), hex(0.09, -0.03, cy, cz, brass));
    SEAL_ORDER.forEach((c, i) => {
      const a = (i * Math.PI) / 3, m = hex(0.065, -0.035, cy + 0.21 * Math.cos(a), cz + 0.21 * Math.sin(a), toon("#2a1d14"));
      m.name = `inlay_${i}`;
      m.userData.lit = c;
      medal.add(m);
    });
    return group(leaf, medal, mesh(new THREE.BoxGeometry(1.0, 0.45, 0.4), stone, 0, 2.375, 0),
      mesh(new THREE.BoxGeometry(0.12, 2.15, 0.4), stone, -0.46, 1.075, 0), mesh(new THREE.BoxGeometry(0.12, 2.15, 0.4), stone, 0.46, 1.075, 0));
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

// Footprint in tiles [along X, along Z] at rot 0 (1 x 1 when not listed).
export const FOOT = {
  house: [7, 5], tree: [2, 2], counter: [3, 1], shelf: [2, 1], duel_table: [2, 1], rug: [3, 2],
  bridge: [2, 4], lighthouse: [2, 2], boat: [2, 1], stall: [2, 1], stall_b: [2, 1], cottage: [5, 4],
  workshop: [3, 3], fountain: [3, 3], bench: [2, 1],
  oven: [2, 1], printing_press: [2, 1], bookcase: [2, 1], bed: [2, 1], workbench: [2, 1], light_box: [2, 1],
  curio_cabinet: [2, 1], seal_stone: [2, 2], seal_stone_carved: [2, 2], cave_mouth: [2, 1], log: [2, 1],
  ferry: [4, 2], cave_mouth_slate: [2, 1],
  kiln: [4, 4], glass_furnace: [2, 2], glass_rack: [2, 1], lava_glass: [2, 2], // Ember
  observatory: [5, 5], lift_tower: [2, 1], rodspire: [3, 3], telescope: [2, 2], // Storm
  terrace_house: [4, 3], // Stone (Cinderwick's terraces use it too)
  harbour_house: [5, 4], bell_tower: [3, 3], net_rack: [2, 1], chart_table: [2, 1], // Tide
  willow: [2, 2], wishing_tree: [2, 2], moonwell: [3, 3], lake_house: [4, 4], wisteria_arch: [2, 1], cloth_rack: [2, 1], // Dream
  standing_tree: [3, 3], great_root: [4, 2], treehouse: [4, 4], fallen_trunk: [4, 3], fallen_trunk_end: [4, 3], // Grove
  cobbler_bench: [2, 1], boot_rack: [2, 1],
  headframe: [3, 3], terrace_house: [4, 3], minecart: [2, 1], // Stone
  hold_wall: [2, 1], tally_board: [2, 1],
};
// Props the hero walks over; every other prop blocks its footprint.
export const WALKABLE = new Set(["rug", "exit_mat", "tidepool", "wisteria_arch", "rail", "support_beam", "oil_lantern"]); // the arch and the mine frames: walked under
// Seats (seat.js): [the seat's top, how far in front of each tile's centre the hips go (along the prop's
// facing, +Z at rot 0)], in m. Every footprint tile is a place: a bench seats two. `seat: false` on a map's prop: scenery.
export const SEAT = { bench: [0.47, 0.02], chair: [0.52, 0.06] };
// People are spoken to across these (a person behind a counter or a duel table).
export const ACROSS = new Set(["counter", "duel_table", "stall", "stall_b", "curio_cabinet"]);
// Decks: walkable even over water, with the floor height the hero stands at, per
// footprint row along the prop's own Z (front row first; one value for every row).
export const DECK = { boardwalk: [0.1], pier: [0.3], bridge: [0.12, 0.24, 0.24, 0.12], stairs: [0.5], climb_wall: [0.5], climb_wall_basalt: [0.5], stairs_basalt: [0.5], stairs_heath: [0.5],
  lantern_raft: [0.1], ladder: [1.5], canopy_deck: [0.1], rope_bridge: [0.1], stairs_slate: [0.5], climb_wall_slate: [0.5], climb_wall_root: [0.5] };
// Doors: a warp on the front row of one of these props. The hero walks `in` metres past the footprint's front
// edge, up its stoop and through the door as it swings open (build.js `doors`: every house model has a hinged
// `door` leaf), and the fade comes up there. `steps`: tread heights every 0.25 m from that edge (the last one
// holds on), read off the models. Every prop a door warp sits on needs an entry (tools/check-maps.mjs), so a new
// house can't have the hero wade through its steps.
export const STOOP = { house: { in: 0.5, steps: [0.14, 0.28, 0.28] }, cottage: { in: 0.8, steps: [0, 0, 0.13, 0.22] },
  terrace_house: { in: 0.4, steps: [0.13, 0.13, 0.19] }, harbour_house: { in: 1.0, steps: [0.17, 0.34, 0.68] },
  lake_house: { in: 0.75, steps: [0, 0.3, 0.34] }, workshop: { in: 0.5, steps: [0, 0, 0.14] }, observatory: { in: 0.55, steps: [0, 0.28, 0.28] },
  moonwell: { in: 0.5, steps: [0.14, 0.14, 0.14] }, cave_mouth: { in: 0.5, steps: [0.05, 0.05, 0.05] }, cave_mouth_slate: { in: 0.5, steps: [0, 0, 0] },
  fallen_trunk: { in: 0.5, steps: [0, 0, 0] }, seal_door: { in: 0.4, steps: [0, 0, 0] } };
// Ladders join their front and back neighbours whatever the height (floor 0 in front, a 3 m deck behind).
export const LADDER = new Set(["ladder"]);
// Afloat: they bob on the water (never instanced).
export const BOB = new Set(["boat", "ferry", "buoy", "floating_lantern", "lantern_raft"]);
// Sway reference height per prop (life.js wind).
export const WIND_H = { tree: 3.5, bush: 0.8, flowers: 0.4, pine: 3.8, birch: 3.6, reeds: 0.9, ash_pine: 3.1, heather: 0.55, chime_post: 2.3,
  willow: 3.8, wishing_tree: 3.8, wisteria_arch: 2.3, standing_tree: 5.5, cairn: 1.2 };
// Rough height of a model, for the generic placeholder box of a model not yet built.
export const TALL = { cliff: 1, cliff_b: 1, boardwalk: 0.1, pier: 0.3, bridge: 0.24, stairs: 0.5, lighthouse: 7, cottage: 4.5, workshop: 3.8, pine: 3.8, birch: 3.6,
  waymark: 1.7, inn_sign: 1.9, ember_vent: 1.7, rubble: 1.4, ford_stones: 0.1, lantern_raft: 0.1, climb_wall: 1, climb_wall_basalt: 1, ladder: 3.4,
  ferry: 2.1, buoy: 1.4, floating_lantern: 0.6,
  kiln: 4.5, glass_furnace: 2.8, glass_rack: 1.8, lava_glass: 0.3, ash_pine: 3.1, cliff_basalt: 1, cliff_basalt_b: 1, stairs_basalt: 0.5,
  glow_mushroom: 0.35, glow_mushroom_tall: 0.9, glow_shelf: 0.62, observatory: 5.7, lift_tower: 9.6, lift_cage: 2.6, rodspire: 12.1, pylon: 3.2, chime_post: 2.6, heather: 0.5, wall_dry: 0.95, cliff_heath: 1, cliff_heath_b: 1, stairs_heath: 0.5, telescope: 2.2,
  harbour_house: 4.3, bell_tower: 6.8, net_rack: 1.45, cliff_sea: 1, cliff_sea_b: 1, tidepool: 0.07, rock_sea: 0.55, bollard: 0.6, crab_pots: 0.45,
  willow: 3.8, wishing_tree: 3.8, moonwell: 3.7, lake_house: 4.6, wisteria_arch: 2.7, moonflower: 0.5, hedge: 1.3,
  standing_tree: 6, great_root: 3, treehouse: 7.6, fallen_trunk: 2.9, fallen_trunk_end: 4.1, canopy_deck: 0.1, rope_bridge: 0.1, beehive: 0.9, mushroom_ring: 0.45,
  climb_wall_root: 1, campfire: 0.5,
  cliff_slate: 1, cliff_slate_b: 1, stairs_slate: 0.5, headframe: 6.1, terrace_house: 5, minecart: 1.1, menhir: 2.7, support_beam: 2.1, cave_mouth_slate: 2.2,
  rail: 0.07, cairn: 1.45, crystal_cluster: 1, climb_wall_slate: 1, oil_lantern: 2.1, oil_lantern_post: 1.75, anvil: 0.85,
  cobbler_bench: 1, boot_rack: 2, hold_wall: 2.6, rope_coil: 0.9, tally_board: 2.2, soup_cauldron: 1.3 };

// Auto-tiling for fences ("f") and walls ("W"): neighbour mask (N=1, E=2, S=4, W=8)
// -> [piece suffix, rot]. At rot 0: "" = arms W+E, _corner = W+S, _end = W,
// _t = W+E+S. Mask 15 adds an _end toward N. Same table in ASSETS.md, in the
// Blender scene script, and checked by tools/check-fence-table.mjs.
// (build.js places the pieces.)
export const AUTOTILE = [
  ["", 0], ["_end", 270], ["_end", 180], ["_corner", 180],
  ["_end", 90], ["", 90], ["_corner", 90], ["_t", 90],
  ["_end", 0], ["_corner", 270], ["", 0], ["_t", 180],
  ["_corner", 0], ["_t", 270], ["_t", 0], ["_t", 0],
];
// Each piece's arms at rot 0, as [dx, dz] (the placeholder pieces, and the model names a map needs).
export const SUFFIX_ARMS = { "": [[-1, 0], [1, 0]], _corner: [[-1, 0], [0, 1]], _end: [[-1, 0]], _t: [[-1, 0], [1, 0], [0, 1]] };
// Shadows on, and loaded glTF materials swapped to toon so they match the placeholders.
export function prep(root, cast) {
  root.traverse((o) => {
    if (!o.isMesh) return;
    o.castShadow = cast;
    o.receiveShadow = true;
    if (!(o.material instanceof THREE.MeshToonMaterial) && !(o.material instanceof THREE.MeshLambertMaterial) && !(o.material instanceof THREE.MeshBasicMaterial)) {
      const m = o.material;
      o.material = toon(m.color ?? "#ffffff", { name: m.name, map: m.map ?? null, transparent: m.transparent, alphaTest: m.alphaTest, opacity: m.opacity });
      o.material.userData = m.userData; // a person's palette table (actors.js)
    }
    // COLOR_0 is a wind weight (read by the wind shader), never a tint.
    o.material.vertexColors = false;
    // A `glow` part (a lamp's glass, the oven's mouth) shines a little on its own.
    if (/glow/i.test(o.material.name) && o.material.emissive) o.material.emissive.copy(o.material.color).multiplyScalar(0.55);
    // The baked contact shadow (two discs a few mm over the ground): pulled toward the
    // camera in depth and never written there or cast, so it can't flicker against the
    // ground or itself.
    if (/contact/i.test(o.material.name)) {
      o.castShadow = false;
      Object.assign(o.material, { depthWrite: false, polygonOffset: true, polygonOffsetFactor: -2, polygonOffsetUnits: /_in/.test(o.material.name) ? -8 : -4 });
      o.renderOrder = /_in/.test(o.material.name) ? 1 : 0;
    }
  });
}

// A model that is not there yet: a plain box of its footprint, so maps work before the art lands.
export function placeholderBox(name) {
  const [w, d] = FOOT[name] ?? [1, 1], h = TALL[name] ?? 0.8;
  return group(mesh(new THREE.BoxGeometry(w * 0.96, h, d * 0.96), toon(name.startsWith("cliff") ? "#b48a5e" : "#a08a70"), 0, h / 2, 0));
}
