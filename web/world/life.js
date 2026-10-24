// Micro-animation for the meadow: wind sway and cloud shadows (shaders on one
// shared clock), plus a few cheap particles: dust motes, butterflies, chimney
// smoke and footstep puffs. With reduced motion, sway and particles are off.
import * as THREE from "three";

export const U = { uTime: { value: 0 }, uSway: { value: 1 }, uDrift: { value: 0 } };

// Vertex wind. Weight: red channel of COLOR_0 when the model has it (0 base, 1 tip),
// else height above the ground over `height`. A gust front sweeps the map every 14 s.
// ponytail: shadows use the default depth material, so they don't sway; add a
// customDepthMaterial with the same chunk if anyone notices.
export function windify(mesh, height) {
  const byColor = !!mesh.geometry.attributes.color;
  if (byColor && !mesh.geometry.attributes.windw) mesh.geometry.setAttribute("windw", mesh.geometry.attributes.color);
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
          float wgt = ${byColor ? "windw.r" : "clamp(wp.y / uWindH, 0.0, 1.0)"};
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

// Soft cloud shadows drifting over the ground: a tiling noise multiplied into its colour.
export function cloudify(mat) {
  const tex = softTexture(256, 14, 40, 110, true);
  mat.onBeforeCompile = (sh) => {
    Object.assign(sh.uniforms, U, { uCloud: { value: tex } });
    sh.vertexShader = sh.vertexShader
      .replace("#include <common>", "#include <common>\nvarying vec2 vCloud;")
      .replace("#include <begin_vertex>", "#include <begin_vertex>\nvCloud = (modelMatrix * vec4(position, 1.0)).xz;");
    sh.fragmentShader = sh.fragmentShader
      .replace("#include <common>", "#include <common>\nuniform float uDrift;\nuniform sampler2D uCloud;\nvarying vec2 vCloud;")
      .replace(
        "#include <map_fragment>",
        "#include <map_fragment>\ndiffuseColor.rgb *= 1.0 - 0.2 * texture2D(uCloud, vCloud * 0.03 + uDrift * vec2(0.012, 0.007)).r;",
      );
  };
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

// Particles. `spots.flowers` = flower bed centres, `spots.smoke` = chimney top or null.
export function ambience(scene, spots, reduced) {
  if (reduced) return { update() {}, puff() {} };
  const dot = softTexture(64, 1, 30, 30, false);
  const sprite = (color, opacity) => {
    const s = new THREE.Sprite(new THREE.SpriteMaterial({ map: dot, color, transparent: true, opacity, depthWrite: false, blending: THREE.AdditiveBlending }));
    scene.add(s);
    return s;
  };

  // Dust motes catching the light, kept in a box around the hero.
  const MOTES = 60, box = 14;
  const motes = new THREE.Points(
    new THREE.BufferGeometry().setAttribute("position", new THREE.Float32BufferAttribute(new Float32Array(MOTES * 3), 3)),
    new THREE.PointsMaterial({ size: 0.07, map: dot, color: "#fff4c2", transparent: true, opacity: 0.8, depthWrite: false, blending: THREE.AdditiveBlending }),
  );
  const seeds = Array.from({ length: MOTES }, () => [Math.random() * box, Math.random() * 3 + 0.3, Math.random() * box, Math.random() * 6.28]);
  motes.frustumCulled = false;
  scene.add(motes);

  // Butterflies: two hinged wings, looping over a flower bed each.
  const flies = spots.flowers.slice(0, 4).map((c, i) => {
    const wingGeo = new THREE.PlaneGeometry(0.13, 0.1).translate(0.065, 0, 0).rotateX(-Math.PI / 2);
    const mat = new THREE.MeshBasicMaterial({ color: ["#fff3a6", "#ffffff", "#ffc2e0", "#bfe3ff"][i % 4], side: THREE.DoubleSide });
    const l = new THREE.Mesh(wingGeo, mat), r = new THREE.Mesh(wingGeo, mat);
    r.scale.x = -1;
    const g = new THREE.Group();
    g.add(l, r);
    scene.add(g);
    return { g, l, r, c, k: i * 1.7, last: new THREE.Vector3() };
  });

  // Chimney smoke: a few soft puffs rising and fading.
  const smoke = spots.smoke ? Array.from({ length: 7 }, () => sprite("#f6f2ea", 0)) : [];
  smoke.forEach((s) => (s.material.blending = THREE.NormalBlending));

  // Footstep dust: a small pool reused round-robin.
  const dust = Array.from({ length: 10 }, () => {
    const s = sprite("#d9b98a", 0);
    s.material.blending = THREE.NormalBlending;
    return { s, age: 1 };
  });
  let next = 0;

  return {
    puff(x, z) {
      for (let i = 0; i < 2; i++) {
        const d = dust[next++ % dust.length];
        d.age = 0;
        d.s.position.set(x + (Math.random() - 0.5) * 0.25, 0.08, z + (Math.random() - 0.5) * 0.25);
      }
    },
    update(dt, t, hx, hz) {
      const p = motes.geometry.attributes.position;
      seeds.forEach(([sx, sy, sz, ph], i) => {
        const wrap = (v, c) => c - box / 2 + ((((v - c + box / 2) % box) + box) % box);
        p.setXYZ(i, wrap(sx + Math.sin(t * 0.3 + ph) * 0.6 + t * 0.15, hx), sy + Math.sin(t * 0.7 + ph) * 0.25, wrap(sz + Math.cos(t * 0.25 + ph) * 0.6, hz));
      });
      p.needsUpdate = true;

      for (const f of flies) {
        const a = t * 0.6 + f.k;
        f.g.position.set(f.c.x + Math.sin(a) * 1.3, 0.55 + Math.sin(t * 2.3 + f.k) * 0.2, f.c.z + Math.sin(a * 1.6) * 0.9);
        const v = f.g.position.clone().sub(f.last);
        if (v.lengthSq() > 1e-8) f.g.rotation.y = Math.atan2(v.x, v.z);
        f.last.copy(f.g.position);
        const flap = Math.sin(t * 22 + f.k) * 0.9;
        f.l.rotation.z = flap;
        f.r.rotation.z = -flap;
      }

      const L = 4.2;
      smoke.forEach((s, i) => {
        const age = (t + (i * L) / smoke.length) % L, k = age / L;
        s.position.set(spots.smoke.x + age * 0.18 + Math.sin(t + i) * 0.05, spots.smoke.y + age * 0.5, spots.smoke.z - age * 0.06);
        s.scale.setScalar(0.35 + age * 0.3);
        s.material.opacity = 0.55 * Math.min(1, k * 6) * (1 - k);
      });

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
