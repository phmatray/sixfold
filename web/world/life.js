// Micro-animation: wind sway (a vertex shader on one shared clock, also used by
// the ground's drifting cloud shadows) plus a few cheap particles: dust motes
// (fireflies after dark), butterflies (roosting at night), moths round the lamps
// at night, gulls over the sea by day, chimney smoke and footstep puffs. With
// reduced motion, sway and particles are off.
//
// A map's `life` (#115) adds critters, perch birds, falling leaves, embers and docking boats, and
// micro-events; life_schedule.js decides where and when, this file draws it, instanced, one draw per kind.
import * as THREE from "three";
import { mergeGeometries } from "three/addons/utils/BufferGeometryUtils.js";
import { toon } from "./props.js";
import { lifeSchedule, calmAt, boatAt, BUDGET } from "./life_schedule.js";

export const U = { uTime: { value: 0 }, uSway: { value: 1 }, uDrift: { value: 0 } };

// Vertex wind. Weight: red channel of COLOR_0 when the model has it (0 base, 1 tip),
// else height above the ground over `height`. A gust front sweeps the map every 14 s.
// ponytail: shadows use the default depth material, so they don't sway; add a
// customDepthMaterial with the same chunk if anyone notices.
export function windify(mesh, height) {
  // A flattened model (build.js) already carries `windw` (-1 where a part had none: use the height).
  const flat = !!mesh.geometry.attributes.windw;
  const byColor = flat || !!mesh.geometry.attributes.color;
  if (byColor && !flat) mesh.geometry.setAttribute("windw", mesh.geometry.attributes.color);
  const mat = mesh.material;
  mat.onBeforeCompile = (sh) => {
    Object.assign(sh.uniforms, U, { uWindH: { value: height } });
    sh.vertexShader = sh.vertexShader
      .replace("#include <common>", `#include <common>\nuniform float uTime, uSway, uWindH;\n${byColor ? "attribute vec4 windw;" : ""}`)
      .replace(
        "#include <begin_vertex>",
        `#include <begin_vertex>
        {
          mat4 m = modelMatrix;
          #ifdef USE_INSTANCING
            m = m * instanceMatrix;
          #endif
          vec3 wp = (m * vec4(position, 1.0)).xyz;
          float wgt = ${byColor ? "windw.r < 0.0 ? clamp(wp.y / uWindH, 0.0, 1.0) : windw.r" : "clamp(wp.y / uWindH, 0.0, 1.0)"};
          wgt *= wgt;
          float ph = dot(wp.xz, vec2(0.37, 0.61));
          float sway = sin(uTime * 1.6 + ph) * 0.6 + sin(uTime * 2.7 + ph * 1.7) * 0.3;
          float front = mod(uTime, 14.0) * 5.0 - 15.0;
          float gust = exp(-pow(dot(wp.xz, vec2(0.8, 0.6)) - front, 2.0) / 10.0);
          float d = uSway * wgt * uWindH * (0.035 * sway + 0.09 * gust);
          transformed += vec3(d, 0.0, d * 0.4) * mat3(m); // world-space push, back to local
        }`,
      );
  };
  mat.customProgramCacheKey = () => (byColor ? "wind-c" : "wind-h");
}

// Still water in a model (the fountain's pool): drifting ripples and glints in
// the fragment shader, on the shared clock (frozen with reduced motion).
export function waterify(mat) {
  mat.onBeforeCompile = (sh) => {
    Object.assign(sh.uniforms, U);
    sh.vertexShader = sh.vertexShader
      .replace("#include <common>", "#include <common>\nvarying vec3 vWP;")
      .replace("#include <begin_vertex>", "#include <begin_vertex>\nvWP = (modelMatrix * vec4(position, 1.0)).xyz;");
    sh.fragmentShader = sh.fragmentShader
      .replace("#include <common>", "#include <common>\nvarying vec3 vWP;\nuniform float uDrift;")
      .replace(
        "#include <map_fragment>",
        `#include <map_fragment>
        {
          vec2 p = vWP.xz * 3.0;
          float rip = sin(p.x * 1.7 + uDrift * 2.1) * sin(p.y * 1.3 - uDrift * 1.7) * 0.5 + 0.5;
          float rip2 = sin((p.x + p.y) * 2.3 - uDrift * 2.9) * 0.5 + 0.5;
          diffuseColor.rgb *= 0.85 + 0.2 * rip;
          diffuseColor.rgb += vec3(0.9, 0.97, 1.0) * 0.35 * smoothstep(0.82, 0.98, rip * rip2 + 0.25);
        }`,
      );
  };
  mat.customProgramCacheKey = () => "water";
}

// White blobs on black (or one soft dot when n = 1), tiling when `wrap`.
function softTexture(size, n, rMin, rMax, wrap) {
  const c = document.createElement("canvas");
  c.width = c.height = size;
  const g = c.getContext("2d");
  if (n > 1) {
    // Clouds are read from the red channel, so they need an opaque black base;
    // the single dot (sprites, motes) keeps a transparent one.
    g.fillStyle = "#000";
    g.fillRect(0, 0, size, size);
  }
  let seed = 11;
  const r = () => ((seed = (seed * 16807) % 2147483647) / 2147483647);
  for (let i = 0; i < n; i++) {
    const x = n === 1 ? size / 2 : r() * size, y = n === 1 ? size / 2 : r() * size, rad = rMin + r() * (rMax - rMin);
    for (const dx of wrap ? [-size, 0, size] : [0])
      for (const dy of wrap ? [-size, 0, size] : [0]) {
        const grd = g.createRadialGradient(x + dx, y + dy, 0, x + dx, y + dy, rad);
        grd.addColorStop(0, "rgba(255,255,255,0.55)");
        grd.addColorStop(1, "rgba(255,255,255,0)");
        g.fillStyle = grd;
        g.fillRect(0, 0, size, size);
      }
  }
  const t = new THREE.CanvasTexture(c);
  if (wrap) t.wrapS = t.wrapT = THREE.RepeatWrapping;
  return t;
}

// Critters and perch birds: a few primitives merged into one vertex-coloured mesh each (a few hundred
// triangles, front +Z, feet at 0), drawn as one InstancedMesh per kind.
const part = (g, color, at, s = [1, 1, 1]) => {
  g.scale(...s).translate(...at);
  const c = new THREE.Color(color).toArray(), n = g.attributes.position.count;
  return g.setAttribute("color", new THREE.Float32BufferAttribute(Array.from({ length: n * 3 }, (_, i) => c[i % 3]), 3));
};
const ball = (r) => new THREE.SphereGeometry(r, 8, 6);
const box = (x, y, z) => new THREE.BoxGeometry(x, y, z);
const cone = (r, h, tilt) => new THREE.ConeGeometry(r, h, 6).rotateX(tilt);
const pair = (f) => [f(1), f(-1)];
const MODELS = {
  rabbit: () => [part(ball(0.09), "#9b8a76", [0, 0.09, 0], [1, 0.85, 1.3]), part(ball(0.06), "#9b8a76", [0, 0.16, 0.1]),
    ...pair((s) => part(box(0.02, 0.09, 0.035), "#c4b29c", [0.025 * s, 0.24, 0.09])), part(ball(0.03), "#f4f0e8", [0, 0.11, -0.12])],
  squirrel: () => [part(ball(0.07), "#b0623a", [0, 0.07, 0], [1, 0.9, 1.4]), part(ball(0.045), "#b0623a", [0, 0.12, 0.08]), part(ball(0.07), "#c4784a", [0, 0.17, -0.1], [0.8, 1.6, 0.8])],
  duck: () => [part(ball(0.1), "#8a6a4a", [0, 0.08, 0], [1, 0.75, 1.4]), part(ball(0.05), "#2f6b45", [0, 0.17, 0.1]), part(box(0.04, 0.015, 0.05), "#e8a23a", [0, 0.165, 0.155])],
  crab: () => [part(ball(0.08), "#d8553a", [0, 0.05, 0], [1.4, 0.5, 1]), ...pair((s) => part(ball(0.035), "#e0664a", [0.11 * s, 0.05, 0.07])), ...pair((s) => part(box(0.012, 0.04, 0.012), "#2b2230", [0.025 * s, 0.1, 0.05]))],
  lizard: () => [part(ball(0.05), "#6b8f3a", [0, 0.03, 0], [0.8, 0.5, 2.4]), part(ball(0.035), "#6b8f3a", [0, 0.04, 0.13], [1, 0.7, 1.3]), part(cone(0.025, 0.18, -Math.PI / 2), "#5a7a30", [0, 0.03, -0.2])],
  goat: () => [part(box(0.22, 0.2, 0.4), "#e8e2d6", [0, 0.3, 0]), ...[-1, 1].flatMap((a) => pair((s) => part(box(0.05, 0.2, 0.05), "#cfc6b6", [0.07 * s, 0.1, 0.14 * a]))),
    part(box(0.1, 0.12, 0.16), "#e8e2d6", [0, 0.45, 0.24]), ...pair((s) => part(cone(0.02, 0.1, -0.4), "#6b5a48", [0.03 * s, 0.55, 0.2]))],
  hedgehog: () => [part(ball(0.09), "#5a4636", [0, 0.06, 0], [1, 0.7, 1.3]), part(ball(0.035), "#c9a98a", [0, 0.05, 0.12])],
  glowbeetle: () => [part(ball(0.035), "#d9ff8a", [0, 0.03, 0], [1, 0.6, 1.3])],
  bird: () => [part(ball(0.06), "#6d5f55", [0, 0.06, 0], [1, 0.9, 1.5]), part(ball(0.04), "#6d5f55", [0, 0.11, 0.07]), part(cone(0.012, 0.04, Math.PI / 2), "#e8a23a", [0, 0.11, 0.12]),
    part(box(0.2, 0.01, 0.06), "#4d4038", [0, 0.08, -0.01])],
};
const LEAF = ["#c9a33a", "#d9772f", "#9bb84a", "#e6c25a", "#b5532f"];

// Particles, added to `scene` (the map's root group). `spots.flowers` = flower
// bed centres, `spots.smoke` = smoke sources (chimneys, kilns, vents). `spots.life`
// (a map with `life`, build.js): { map, open, floorAt, trees, embers, boats }.
export function ambience(scene, spots, reduced) {
  if (reduced) return { update() {}, puff() {}, puffs() {} };
  const dot = softTexture(64, 1, 30, 30, false);
  const sprite = (color, opacity) => {
    const s = new THREE.Sprite(new THREE.SpriteMaterial({ map: dot, color, transparent: true, opacity, depthWrite: false, blending: THREE.AdditiveBlending }));
    scene.add(s);
    return s;
  };

  // Dust motes catching the light, kept in a box around the hero.
  const MOTES = 60, box = 14;
  const motes = new THREE.Points(
    new THREE.BufferGeometry()
      .setAttribute("position", new THREE.Float32BufferAttribute(new Float32Array(MOTES * 3), 3))
      .setAttribute("color", new THREE.Float32BufferAttribute(new Float32Array(MOTES * 3).fill(1), 3)),
    new THREE.PointsMaterial({ size: 0.07, map: dot, color: "#fff4c2", vertexColors: true, transparent: true, opacity: 0.8, depthWrite: false, blending: THREE.AdditiveBlending }),
  );
  const seeds = Array.from({ length: MOTES }, () => [Math.random() * box, Math.random() * 3 + 0.3, Math.random() * box, Math.random() * 6.28]);
  motes.geometry.boundingSphere = new THREE.Sphere(new THREE.Vector3(), 500); // in every real view (not world.js's shadowEye, far below)
  scene.add(motes);

  // Butterflies: a dark body and two small toon wings, looping over a flower bed each.
  const wingGeo = new THREE.CircleGeometry(0.045, 8).scale(1, 1.3, 1).translate(0.04, 0, 0).rotateX(-Math.PI / 2);
  const bodyGeo = new THREE.CapsuleGeometry(0.008, 0.05, 2, 6).rotateX(Math.PI / 2);
  const bodyMat = toon("#2b2230");
  const flies = spots.flowers.slice(0, 4).map((c, i) => {
    const mat = toon(["#ff9d3c", "#ffd23f", "#6fb7ff", "#ff7aa8"][i % 4], { side: THREE.DoubleSide });
    const l = new THREE.Mesh(wingGeo, mat), r = new THREE.Mesh(wingGeo, mat);
    r.scale.x = -1;
    const g = new THREE.Group();
    g.add(l, r, new THREE.Mesh(bodyGeo, bodyMat));
    scene.add(g);
    return { g, l, r, c, k: i * 1.7, last: new THREE.Vector3() };
  });

  // Moths: pale wings fluttering round the street lamps nearest the hero, after dark.
  const mothMat = toon("#efe6d2", { side: THREE.DoubleSide, emissive: "#3a3426" });
  const mothWing = new THREE.CircleGeometry(0.03, 6).scale(1, 1.2, 1).translate(0.026, 0, 0).rotateX(-Math.PI / 2);
  const moths = (spots.lamps ?? []).length
    ? Array.from({ length: 8 }, (_, i) => {
        const l = new THREE.Mesh(mothWing, mothMat), r = new THREE.Mesh(mothWing, mothMat);
        r.scale.x = -1;
        const g = new THREE.Group();
        g.add(l, r);
        g.visible = false;
        scene.add(g);
        return { g, l, r, k: i * 2.39, lamp: null };
      })
    : [];
  let mothPick = 0;

  // Gulls (`spots.gulls` of them, a map's `gulls`): wide glides high over the hero's
  // part of the map, a few lazy flaps, gone to roost at night.
  const gullMat = toon("#f7f7f2"), tipMat = toon("#3b4048");
  const gullWing = new THREE.BoxGeometry(0.36, 0.015, 0.11).translate(0.18, 0, 0);
  const gullTip = new THREE.BoxGeometry(0.1, 0.017, 0.09).translate(0.31, 0.002, 0);
  const gullBody = new THREE.CapsuleGeometry(0.045, 0.2, 2, 6).rotateX(Math.PI / 2);
  const gulls = Array.from({ length: spots.gulls ?? 0 }, (_, i) => {
    const wing = (side) => {
      const w = new THREE.Group();
      w.add(new THREE.Mesh(gullWing, gullMat), new THREE.Mesh(gullTip, tipMat));
      w.scale.x = side;
      return w;
    };
    const l = wing(1), r = wing(-1), g = new THREE.Group();
    g.add(new THREE.Mesh(gullBody, gullMat), l, r);
    scene.add(g);
    return { g, l, r, k: i * 2.1, rad: 3.5 + (i % 3) * 1.6, h: 5.2 + (i % 2) * 1.3, ox: ((i * 37) % 9) - 4.5, oz: ((i * 53) % 7) - 5, cx: null, cz: 0, last: new THREE.Vector3() };
  });

  // Smoke: soft puffs rising and fading, from each source in `spots.smoke`
  // ({ at, kind, on }): a chimney's pale curl, a kiln's darker soot, a vent's
  // quick white steam (only while `on()` holds, for a gate's vents).
  const LOOK = { chimney: ["#f6f2ea", 7, 4.2, 0.5, 0.55], soot: ["#8a7f78", 8, 5.2, 0.62, 0.6], steam: ["#ffffff", 5, 1.8, 1.1, 0.7] };
  const smoke = (spots.smoke ?? []).flatMap((src, j) => {
    const [color, n, life, rise, alpha] = LOOK[src.kind] ?? LOOK.chimney;
    return Array.from({ length: n }, (_, i) => {
      const s = sprite(color, 0);
      s.material.blending = THREE.NormalBlending;
      return { s, src, i, n, life, rise, alpha, ph: j * 1.3 };
    });
  });

  // Footstep dust: a small pool reused round-robin.
  const dust = Array.from({ length: 10 }, () => {
    const s = sprite("#d9b98a", 0);
    s.material.blending = THREE.NormalBlending;
    return { s, age: 1 };
  });
  let next = 0;

  const life = spots.life && lively(scene, spots.life);
  const day = new THREE.Color("#fff4c2"), fly = new THREE.Color("#d9ff8a");
  const mm = motes.material;
  return {
    stats: () => life?.stats() ?? null,
    // Dust where a gate changed: a rubble pile falling, rafts drifting in.
    puffs(tiles) {
      for (const t of tiles.slice(0, 5)) this.puff(t.x + 0.5, t.z + 0.5);
    },
    puff(x, z) {
      for (let i = 0; i < 2; i++) {
        const d = dust[next++ % dust.length];
        d.age = 0;
        d.s.position.set(x + (Math.random() - 0.5) * 0.25, 0.08, z + (Math.random() - 0.5) * 0.25);
      }
    },
    // `night` 0..1: motes turn into slow, low fireflies that pulse; butterflies go to roost.
    // `scene`: a cutscene plays, and the gulls keep out of its frame (#155).
    update(dt, t, hx, hz, night = 0, scene = false) {
      const p = motes.geometry.attributes.position;
      const drift = 0.15 * (1 - night * 0.7), low = 1 - night * 0.5;
      seeds.forEach(([sx, sy, sz, ph], i) => {
        const wrap = (v, c) => c - box / 2 + ((((v - c + box / 2) % box) + box) % box);
        p.setXYZ(i, wrap(sx + Math.sin(t * 0.3 + ph) * 0.6 + t * drift, hx), sy * low + Math.sin(t * 0.7 + ph) * 0.25, wrap(sz + Math.cos(t * 0.25 + ph) * 0.6, hz));
      });
      p.needsUpdate = true;
      if (life) motes.geometry.setDrawRange(0, life.update(dt, t, hx, hz, night) ? MOTES / 2 : MOTES); // halved in a calm place
      mm.color.lerpColors(day, fly, night);
      mm.size = 0.07 + 0.07 * night;
      // Each firefly glows and fades on its own beat (additive: dark is unseen).
      const c = motes.geometry.attributes.color;
      if (night > 0 || c.getX(0) !== 1) {
        seeds.forEach(([, , , ph], i) => {
          const pulse = Math.max(0, Math.sin(t * (0.9 + (ph % 1) * 0.8) + ph * 5));
          const k = 1 - night + night * pulse * pulse;
          c.setXYZ(i, k, k, k);
        });
        c.needsUpdate = true;
      }

      for (const f of flies) {
        f.g.visible = night < 0.5;
        if (!f.g.visible) continue;
        const a = t * 0.6 + f.k;
        f.g.position.set(f.c.x + Math.sin(a) * 1.3, 0.55 + Math.sin(t * 2.3 + f.k) * 0.2, f.c.z + Math.sin(a * 1.6) * 0.9);
        const v = f.g.position.clone().sub(f.last);
        if (v.lengthSq() > 1e-8) f.g.rotation.y = Math.atan2(v.x, v.z);
        f.last.copy(f.g.position);
        const flap = Math.sin(t * 11 + f.k) * 0.8;
        f.l.rotation.z = flap;
        f.r.rotation.z = -flap;
      }

      // Each moth keeps to one of the four lamps nearest the hero (re-picked each second).
      if (moths.length) {
        const on = night > 0.4;
        if (on && t > mothPick) {
          mothPick = t + 1;
          const near = spots.lamps.filter((l) => l.y < 4 && Math.hypot(l.x - hx, l.z - hz) < 16).sort((a, b) => Math.hypot(a.x - hx, a.z - hz) - Math.hypot(b.x - hx, b.z - hz)).slice(0, 4);
          moths.forEach((m, i) => (m.lamp = near.length ? near[i % near.length] : null));
        }
        for (const m of moths) {
          m.g.visible = on && !!m.lamp;
          if (!m.g.visible) continue;
          const a = t * (2.1 + (m.k % 1.3)) + m.k;
          m.g.position.set(m.lamp.x + Math.sin(a) * 0.32 + Math.sin(a * 2.7) * 0.08, m.lamp.y - 0.05 + Math.sin(a * 1.7) * 0.18, m.lamp.z + Math.cos(a * 1.3) * 0.32);
          m.g.rotation.y = a + Math.PI / 2;
          const flap = Math.sin(t * 26 + m.k) * 0.9;
          m.l.rotation.z = flap;
          m.r.rotation.z = -flap;
        }
      }

      for (const gl of gulls) {
        gl.g.visible = night < 0.5 && !scene;
        if (!gl.g.visible) continue;
        // The circle's centre trails the hero slowly, so the gulls stay in view without following.
        if (gl.cx === null) (gl.cx = hx + gl.ox), (gl.cz = hz + gl.oz);
        gl.cx += (hx + gl.ox - gl.cx) * Math.min(1, dt * 0.08);
        gl.cz += (hz + gl.oz - gl.cz) * Math.min(1, dt * 0.08);
        const a = t * (0.32 + (gl.k % 0.7) * 0.1) + gl.k;
        gl.g.position.set(gl.cx + Math.cos(a) * gl.rad, gl.h + Math.sin(a * 2.3) * 0.35, gl.cz + Math.sin(a) * gl.rad * 0.7);
        const v = gl.g.position.clone().sub(gl.last);
        if (v.lengthSq() > 1e-8) gl.g.rotation.y = Math.atan2(v.x, v.z);
        gl.last.copy(gl.g.position);
        gl.g.rotation.z = -0.35; // banked into the turn
        const beat = Math.max(0, Math.sin(t * 0.5 + gl.k)) ** 8; // mostly gliding, now and then a few flaps
        const flap = 0.12 + beat * Math.sin(t * 9 + gl.k) * 0.55;
        gl.l.rotation.z = flap;
        gl.r.rotation.z = -flap;
      }

      for (const p of smoke) {
        const { s, src, i, n, life: L } = p;
        const on = !src.on || src.on();
        if (!on && s.material.opacity < 0.01) {
          s.visible = false;
          continue;
        }
        s.visible = true;
        const age = (t + p.ph + (i * L) / n) % L, k = age / L;
        s.position.set(src.at.x + age * 0.18 + Math.sin(t + i) * 0.05, src.at.y + age * p.rise, src.at.z - age * 0.06);
        s.scale.setScalar(0.35 + age * 0.3 * (p.rise > 1 ? 0.6 : 1));
        const want = on ? p.alpha * Math.min(1, k * 6) * (1 - k) : 0;
        s.material.opacity += (want - s.material.opacity) * Math.min(1, dt * (on ? 30 : 3));
      }

      for (const d of dust) {
        if (d.age >= 1) continue;
        d.age = Math.min(1, d.age + dt / 0.45);
        d.s.scale.setScalar(0.15 + d.age * 0.35);
        d.s.position.y += dt * 0.25;
        d.s.material.opacity = 0.55 * (1 - d.age);
      }
    },
  };
}

// A map's `life` (#115): critters and perch birds where lifeSchedule puts them, leaves falling under
// its trees (more in a gust), embers rising from its kilns and vents, boats docking at its piers.
// `L`: { map, open(x, z), floorAt(x, z), trees: [{ x, z }], embers: smoke sources, boats: [{ x, z, dx, dz, len, every }] }.
// `update` returns whether the hero stands in a calm place (particles there are halved).
function lively(scene, L) {
  const life = L.map.life, mem = { open: L.open }, o = new THREE.Object3D();
  const kinds = {};
  for (const k of new Set([...(life.critters ?? []).map((c) => c.kind), ...((life.perch ?? []).length ? ["bird"] : [])])) {
    if (!MODELS[k]) continue;
    const mat = toon("#ffffff", { vertexColors: true, ...(k === "glowbeetle" ? { emissive: "#9fd34a" } : {}) });
    const m = new THREE.InstancedMesh(mergeGeometries(MODELS[k]()).scale(1.6, 1.6, 1.6), mat, BUDGET); // storybook-sized next to the hero
    Object.assign(m, { count: 0, visible: false, frustumCulled: false });
    scene.add((kinds[k] = m));
  }

  // Leaves: one pool, each falling from a canopy near the hero to the ground, lying there a moment.
  const leafOn = life.leaves && L.trees.length;
  const leafMesh = new THREE.InstancedMesh(new THREE.PlaneGeometry(0.15, 0.1), toon("#ffffff", { side: THREE.DoubleSide }), leafOn ? 36 : 1);
  const leaves = Array.from({ length: leafOn ? 36 : 0 }, (_, i) => (leafMesh.setColorAt(i, new THREE.Color(LEAF[i % LEAF.length])), { age: 1, life: 1, x: 0, y: 0, z: 0, ph: i * 1.3 }));
  Object.assign(leafMesh, { count: 0, visible: false, frustumCulled: false });
  scene.add(leafMesh);

  // Embers: additive points rising and fading over the kilns and vents.
  const EMB = life.embers && L.embers.length ? 30 : 0;
  const ember = new THREE.Points(
    new THREE.BufferGeometry().setAttribute("position", new THREE.Float32BufferAttribute(new Float32Array(Math.max(1, EMB) * 3), 3)),
    new THREE.PointsMaterial({ size: 0.14, color: "#ffb04a", transparent: true, opacity: 0.9, depthWrite: false, blending: THREE.AdditiveBlending }),
  );
  ember.frustumCulled = false;
  ember.visible = EMB > 0;
  scene.add(ember);
  const embers = Array.from({ length: EMB }, (_, i) => ({ age: 1, life: 2.5, x: 0, y: -9, z: 0, ph: i * 2.1 }));

  // Boats: the map's own rowing boat model (long along +X, turned to run along +Z here), or a plain
  // hull where the map has none, out along the lane from the pier and back.
  const hullMat = toon("#7a5434");
  const boats = (L.boats ?? []).map((b) => {
    const g = new THREE.Group(), hull = L.boatModel?.clone(true) ?? new THREE.Mesh(new THREE.BoxGeometry(1.4, 0.22, 0.55).translate(0, 0.08, 0), hullMat);
    hull.rotation.y = -Math.PI / 2;
    g.add(hull);
    scene.add(g);
    return { ...b, g };
  });

  let acc = 0, emAcc = 0, near = [], pick = -1, last = null;
  return {
    stats: () => ({ critters: Object.fromEntries(Object.entries(kinds).map(([k, m]) => [k, m.count])), leaves: leafMesh.count, embers: embers.filter((e) => e.age < 1).length,
      boats: boats.map((b) => ({ ...boatAt(b.every, last?.t ?? 0), x: +b.g.position.x.toFixed(2), z: +b.g.position.z.toFixed(2) })), event: last?.event?.kind ?? null, calm: !!last?.quiet }),
    update(dt, t, hx, hz, night) {
      const quiet = calmAt(L.map, hx, hz), { spawns, event } = lifeSchedule(L.map, t, { x: hx, z: hz, night }, mem);
      last = { t, event, quiet };
      for (const m of Object.values(kinds)) m.count = 0;
      for (const s of spawns) {
        const m = kinds[s.kind];
        if (!m || m.count >= BUDGET) continue;
        let y = (s.y ?? 0) + L.floorAt(Math.floor(s.x), Math.floor(s.z)), x = s.x, z = s.z, k = 1, roll = 0;
        if (s.hop) y += Math.sin(s.hop * Math.PI) * 0.1;
        if (s.bolt !== undefined && s.bolt > 0.6) k = (1 - s.bolt) / 0.4; // gone to ground
        if (s.fly !== undefined) {
          (y += s.fly * s.fly * 4), (x += Math.sin(s.face) * s.fly * 3), (z += Math.cos(s.face) * s.fly * 3);
          roll = Math.sin(t * 22 + s.x) * 0.35;
        }
        o.position.set(x, y, z);
        o.rotation.set(0, s.face, roll);
        o.scale.setScalar(k);
        o.updateMatrix();
        m.setMatrixAt(m.count++, o.matrix);
      }
      for (const m of Object.values(kinds)) (m.visible = m.count > 0), (m.instanceMatrix.needsUpdate = true);

      // The trees and kilns near the hero, re-picked each second.
      if (t > pick) {
        pick = t + 1;
        near = L.trees.filter((p) => Math.hypot(p.x - hx, p.z - hz) < 12);
      }
      const gust = event?.kind === "gust" ? 1 : 0, half = quiet ? 0.5 : 1;
      if (leaves.length) {
        acc += dt * life.leaves.rate * Math.min(near.length, 10) * half * (1 + gust * 3);
        while (acc >= 1 && near.length) {
          acc -= 1;
          const tr = near[Math.floor(Math.random() * near.length)], free = leaves.find((l) => l.age >= l.life);
          // The 14 s gust front of the wind shader (windify) shakes more leaves off.
          const front = (t % 14) * 5 - 15, g = Math.exp(-((tr.x * 0.8 + tr.z * 0.6 - front) ** 2) / 10);
          if (free) Object.assign(free, { age: 0, x: tr.x + (Math.random() - 0.5) * 1.4, y: 1.7 + Math.random() * 1.1, z: tr.z + (Math.random() - 0.5) * 1.4, g: Math.max(g, gust) });
          if (g > 0.5) acc += 1;
        }
        let n = 0;
        for (const l of leaves) {
          if (l.age >= l.life) continue;
          l.age += dt;
          const ground = L.floorAt(Math.floor(l.x), Math.floor(l.z)) + 0.02;
          if (l.y > ground) {
            const push = 0.25 + l.g * 1.2;
            (l.y = Math.max(ground, l.y - dt * 0.45)), (l.x += dt * (push * 0.8 + Math.sin(l.age * 3 + l.ph) * 0.3)), (l.z += dt * push * 0.6);
            l.life = l.age + 1.5; // then lie a moment on the ground
          }
          o.position.set(l.x, l.y, l.z);
          o.rotation.set(l.y > ground ? l.age * 4 + l.ph : -Math.PI / 2, l.age * 2.3 + l.ph, 0);
          o.scale.setScalar(Math.min(1, (l.life - l.age) / 0.5));
          o.updateMatrix();
          leafMesh.setMatrixAt(n++, o.matrix);
        }
        leafMesh.count = n;
        leafMesh.visible = n > 0;
        leafMesh.instanceMatrix.needsUpdate = true;
      }

      if (EMB) {
        const srcs = L.embers.filter((s) => (!s.on || s.on()) && Math.hypot(s.at.x - hx, s.at.z - hz) < 14);
        emAcc += dt * 8 * half * (1 + gust);
        const pos = ember.geometry.attributes.position;
        embers.forEach((e, i) => {
          if (e.age >= 1 && emAcc >= 1 && srcs.length) {
            emAcc -= 1;
            const s = srcs[Math.floor(Math.random() * srcs.length)];
            Object.assign(e, { age: 0, x: s.at.x + (Math.random() - 0.5) * 0.3, y: s.at.y, z: s.at.z + (Math.random() - 0.5) * 0.3 });
          }
          if (e.age < 1) {
            e.age += dt / e.life;
            (e.y += dt * 0.9), (e.x += dt * (Math.sin(t * 2 + e.ph) * 0.25 + 0.15 + gust * 0.6)), (e.z += dt * Math.cos(t * 1.7 + e.ph) * 0.2);
          }
          pos.setXYZ(i, e.x, e.age < 1 ? e.y : -9, e.z);
        });
        emAcc = Math.min(emAcc, 2);
        pos.needsUpdate = true;
        ember.material.opacity = 0.9 - 0.4 * (1 - night);
      }

      for (const b of boats) {
        const { k, on } = boatAt(b.every, t), out = ((t / b.every) % 1) >= 0.6;
        b.g.visible = on;
        b.g.position.set(b.x + b.dx * b.len * k, Math.sin(t * 1.4 + b.x) * 0.035, b.z + b.dz * b.len * k);
        b.g.rotation.set(Math.sin(t * 1.1) * 0.03, Math.atan2(b.dx, b.dz) + (out ? 0 : Math.PI), 0);
      }
      return quiet;
    },
  };
}
