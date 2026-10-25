// Raised ground ("1".."9" tiles) as one continuous height field, instead of a stack of
// cliff blocks: the ground mesh itself (ground.js) is a grid of S nodes per tile whose
// height is the tallest tile each node touches. A cliff is then the thin band where the
// field drops from one level to the next, inside the lower tile; the foot of that band
// is pulled in toward the lip so walls stand near-vertical. The whole grid is warped by
// smooth noise, convex corners are cut and lips bevelled, so plateaus read as rock
// shelves, not voxels. No faces overlap, so nothing z-fights. Next to a prop that has to
// sit against the wall (stairs, a climb wall, a cave mouth, a house) the warp fades out
// and the wall stands straight on the tile edge, as the block did. Collision and floor
// heights don't look at any of this: they still come from the tiles (build.js).
import * as THREE from "three";
import { FOOT } from "./props.js";

const S = 4; // grid nodes per tile
const LOOSE = /tree|pine|birch|bush|flower|rock|heather|reed|grass|willow|tuft/; // props that don't need a straight wall

// Wall strata (four bands), the darker foot and the worn lip round a plateau's top, per
// cliff set (the map's `cliff`; colours from the old blocks).
export const ROCK = {
  cliff: ["#9a663d", "#b97f4e", "#d29e62", "#e0b479", "#7d5434", "#ead6ac"],
  cliff_basalt: ["#36343d", "#45434d", "#57545f", "#4d4a52", "#2b2a30", "#9c968f"],
  cliff_heath: ["#6f716e", "#9c9c96", "#b8b6ad", "#8d8e88", "#5f6a40", "#8a9458"],
  cliff_sea: ["#b89c74", "#cdb68e", "#e8d6a6", "#c4a87e", "#4f5a3c", "#d6d29a"],
  cliff_slate: ["#4b5262", "#58606f", "#646c7b", "#525a6b", "#3a3f4a", "#8a909b"],
};
export const rockOf = (map) => ROCK[map.cliff?.[0]] ?? ROCK.cliff;

export const rise = (map, x, z) => {
  const c = map.tiles[z]?.[x];
  return c >= "1" && c <= "9" ? +c : 0; // off the map counts as open ground
};
export const raised = (map) => /[1-9]/.test(map.tiles.join(""));

// Smooth 2D value noise in [-1, 1].
function noise(seed) {
  const h = (i, j) => {
    let n = (i * 374761393 + j * 668265263 + seed * 2147483647) | 0;
    n = Math.imul(n ^ (n >>> 13), 1274126177);
    return ((n ^ (n >>> 16)) >>> 0) / 4294967295;
  };
  return (x, z) => {
    const i = Math.floor(x), j = Math.floor(z), fx = x - i, fz = z - j;
    const u = fx * fx * (3 - 2 * fx), v = fz * fz * (3 - 2 * fz);
    const a = h(i, j) + (h(i + 1, j) - h(i, j)) * u, b = h(i, j + 1) + (h(i + 1, j + 1) - h(i, j + 1)) * u;
    return (a + (b - a) * v) * 2 - 1;
  };
}

// The terrain grid for a map with raised tiles, in world coordinates, and the
// indices of its raised part (the only part that needs to cast a shadow).
export function terrainGeometry(map) {
  const { width: W, height: H } = map;
  const m = map.ground.margin ?? 0;
  const inMap = (x, z) => x >= 0 && z >= 0 && x < W && z < H;
  // Tiles whose walls stay straight: under a built prop, or a gate's.
  const pin = new Uint8Array(W * H);
  const mark = (x, z) => inMap(x, z) && (pin[z * W + x] = 1);
  for (const p of map.props) {
    if (LOOSE.test(p.asset)) continue;
    let [fw, fd] = FOOT[p.asset] ?? [1, 1];
    if (p.rot % 180 === 90) [fw, fd] = [fd, fw];
    for (let x = p.x; x < p.x + fw; x++) for (let z = p.z; z < p.z + fd; z++) mark(x, z);
  }
  for (const g of map.gates ?? []) if (g.prop || g.prop_closed || g.prop_open) for (const [x, z] of g.tiles) mark(x, z);

  // Node positions along one axis: the margin, a foot row just outside the map, then S per tile.
  const axis = (n) => [...(m ? [-m, -1 / S] : []), ...Array.from({ length: n * S + 1 }, (_, i) => i / S), ...(m ? [n + 1 / S, n + m] : [])];
  const xs = axis(W), zs = axis(H), NX = xs.length, NZ = zs.length;
  // The tiles a coordinate touches (two on a tile edge, else one).
  const touch = (c, n) => (c < 0 || c > n ? [] : Number.isInteger(c) ? [c - 1, c].filter((t) => t >= 0 && t < n) : [Math.floor(c)]);
  // Pinned nodes (touching a pinned tile, or off the map) take the tallest tile they touch.
  const hgt = new Float32Array(NX * NZ), amp0 = new Float32Array(NX * NZ);
  for (let j = 0; j < NZ; j++)
    for (let i = 0; i < NX; i++) {
      const tx = touch(xs[i], W), tz = touch(zs[j], H), k = j * NX + i;
      let top = 0, pinned = !tx.length || !tz.length;
      for (const x of tx) for (const z of tz) (top = Math.max(top, rise(map, x, z))), (pinned ||= !!pin[z * W + x]);
      hgt[k] = top;
      amp0[k] = pinned ? 0 : 1;
    }
  // Fade the roughness out round pinned nodes (a 3×3 average, zero on them).
  const amp = new Float32Array(NX * NZ);
  for (let j = 0; j < NZ; j++)
    for (let i = 0; i < NX; i++) {
      const k = j * NX + i;
      if (!amp0[k]) continue;
      let s = 0, n = 0;
      for (let dj = -1; dj <= 1; dj++)
        for (let di = -1; di <= 1; di++) {
          const a = i + di, b = j + dj;
          if (a >= 0 && b >= 0 && a < NX && b < NZ) (s += amp0[b * NX + a]), n++;
        }
      amp[k] = s / n;
    }

  // Everywhere else a plateau's outline is a contour: "at least level L" (1 or 0 per
  // tile) interpolated between tile centres, plus a little noise, cut at 0.5. Tile
  // centres keep their own level and straight edges stay on the tile edge, but a
  // convex corner is rounded off, a concave one filled in, and a staircase of tiles
  // becomes one slanting line.
  const lev = new Uint8Array(W * H);
  for (let z = 0; z < H; z++) for (let x = 0; x < W; x++) lev[z * W + x] = rise(map, x, z);
  const top = Math.max(...lev), no = noise(7), nf = noise(29), nh = noise(41);
  const at = (L, x, z) => (inMap(x, z) && lev[z * W + x] >= L ? 1 : 0);
  const jit = new Float32Array(NX * NZ); // the noise added at each node (taken as flat round it)
  const occ = (L, x, z, k) => {
    const u = x - 0.5, v = z - 0.5, i = Math.floor(u), j = Math.floor(v), fu = u - i, fv = v - j;
    return (at(L, i, j) * (1 - fu) + at(L, i + 1, j) * fu) * (1 - fv) + (at(L, i, j + 1) * (1 - fu) + at(L, i + 1, j + 1) * fu) * fv + jit[k];
  };
  const grad = (L, x, z, k) => [occ(L, x + 0.02, z, k) - occ(L, x - 0.02, z, k), occ(L, x, z + 0.02, k) - occ(L, x, z - 0.02, k)].map((g) => g / 0.04);
  for (let j = 0; j < NZ; j++)
    for (let i = 0; i < NX; i++) {
      const k = j * NX + i;
      if (!amp0[k]) continue;
      jit[k] = no(xs[i] * 1.1, zs[j] * 1.1) * 0.14 * amp[k];
      let h = 0;
      while (h < top && occ(h + 1, xs[i], zs[j], k) >= 0.5) h++;
      hgt[k] = h;
    }
  const around = (i, j, f) => {
    for (let dj = -1; dj <= 1; dj++)
      for (let di = -1; di <= 1; di++) {
        const a = i + di, b = j + dj;
        if ((di || dj) && a >= 0 && b >= 0 && a < NX && b < NZ) f(a, b, b * NX + a, di && dj ? 0.5 : 1);
      }
  };

  const pos = new Float32Array(NX * NZ * 3), foot = new Uint8Array(NX * NZ), lips = new Uint8Array(NX * NZ);
  for (let j = 0; j < NZ; j++)
    for (let i = 0; i < NX; i++) {
      const k = j * NX + i, X = xs[i], Z = zs[j], a = amp[k];
      let x = X, z = Z, y = hgt[k], lower = false, dx = 0, dz = 0;
      around(i, j, (a2, b2, q, wgt) => {
        if (hgt[q] < y) lower = true;
        else if (hgt[q] > y) (dx += (xs[a2] - X) * wgt), (dz += (zs[b2] - Z) * wgt);
      });
      if (lower && y > 0) lips[k] = 1;
      else if (dx || dz) foot[k] = 1;
      if (!amp0[k]) {
        // Pinned: a foot is pulled nearly onto the tile edge, so the wall stands straight there.
        const len = Math.hypot(dx, dz);
        if (foot[k]) (x += (dx / len) * 0.92 / S), (z += (dz / len) * 0.92 / S);
      } else if (lips[k] || foot[k]) {
        // A lip slides out onto its contour; a foot slides in to just short of it, so the
        // wall between them stands near-vertical (a little rougher where `nf` says so).
        const L = lips[k] ? y : y + 1, [gx, gz] = grad(L, X, Z, k), g = Math.hypot(gx, gz) || 1;
        const o = occ(L, X, Z, k);
        const rough = nh(X * 9, Z * 9) * 0.5 + 0.5; // node to node: facets
        const d = lips[k] ? -Math.min((o - 0.5) / g, 0.6 / S) : Math.max(0, Math.min((0.5 - o) / g - 0.02 - 0.06 * (nf(X * 2.1, Z * 2.1) * 0.5 + 0.5) - 0.08 * rough, 1.3 / S));
        (x += (gx / g) * d), (z += (gz / g) * d);
        if (lips[k]) y -= (0.05 + 0.07 * rough) * a; // a bevelled, chipped lip
      }
      pos.set([x, y, z], k * 3);
    }

  // aRim: x, a lip node (1) and its inner neighbours (a worn band on the top); y, a foot
  // node (1) and its outer neighbours (a contact shadow on the ground below).
  const rim = new Float32Array(NX * NZ * 2);
  for (let j = 0; j < NZ; j++)
    for (let i = 0; i < NX; i++) {
      const k = j * NX + i;
      if (xs[i] < 0 || zs[j] < 0 || xs[i] > W || zs[j] > H) continue; // off the map: its cells run out into the wide margin
      let lip = lips[k] && hgt[k] > 0 ? 1 : 0, ao = foot[k];
      for (const [di, dj] of [[1, 0], [-1, 0], [0, 1], [0, -1]]) {
        const a = i + di, b = j + dj, q = b * NX + a;
        if (a < 0 || b < 0 || a >= NX || b >= NZ || hgt[q] !== hgt[k]) continue;
        if (lips[q] && hgt[q] > 0) lip = Math.max(lip, 0.45);
        if (foot[q]) ao = Math.max(ao, 0.4);
      }
      rim.set([lip, ao], k * 2);
    }

  // Two triangles a cell, split along the diagonal whose ends are closer in height.
  const index = [], cast = [];
  for (let j = 0; j < NZ - 1; j++)
    for (let i = 0; i < NX - 1; i++) {
      const a = j * NX + i, b = a + 1, c = a + NX, d = c + 1;
      const tris = Math.abs(hgt[a] - hgt[d]) <= Math.abs(hgt[b] - hgt[c]) ? [a, c, d, a, d, b] : [a, c, b, b, c, d];
      index.push(...tris);
      if (hgt[a] || hgt[b] || hgt[c] || hgt[d]) cast.push(...tris);
    }
  const geo = new THREE.BufferGeometry();
  geo.setAttribute("position", new THREE.BufferAttribute(pos, 3));
  geo.setAttribute("aRim", new THREE.BufferAttribute(rim, 2));
  geo.setIndex(index);
  geo.computeVertexNormals();
  const shadow = new THREE.BufferGeometry();
  shadow.setAttribute("position", geo.attributes.position);
  shadow.setAttribute("normal", geo.attributes.normal);
  shadow.setIndex(cast);
  return { geo, shadow };
}
