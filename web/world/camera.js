// The outdoor camera angles the player picks from (#148). Pure ES with no
// imports: world.js frames the camera with it, and tools/check-maps.mjs reads
// it to check occlusion at the lowest angle (#110's camera-ray test will run
// once per angle in ANGLES). Rooms keep their own framing.

/** The cycle order of the C key / R3. */
export const ANGLES = ["default", "higher", "lower"];

/** Pitch in degrees and distance factor of each non-default angle. "default"
 * is the map's own camera (50°). The factors keep about 13 × 9 tiles on screen. */
export const PRESETS = {
  lower: { pitch: 40, scale: 0.88 },
  higher: { pitch: 60, scale: 1.08 },
};

/** The camera block to use for a map's `camera` at `angle`. */
export function framing(mapCamera, angle, indoor) {
  const p = PRESETS[angle];
  if (indoor || !p) return mapCamera;
  return {
    ...mapCamera,
    pitch: p.pitch,
    distance: mapCamera.distance * p.scale,
    distancePortrait: mapCamera.distancePortrait * p.scale,
  };
}

/** The angle after `angle` in the cycle; an unknown one restarts at default. */
export function nextAngle(angle) {
  const i = ANGLES.indexOf(angle);
  return i < 0 ? ANGLES[0] : ANGLES[(i + 1) % ANGLES.length];
}

/** The lowest outdoor pitch, where a prop hides the most behind it. */
export function lowestPitch() {
  return Math.min(50, ...Object.values(PRESETS).map((p) => p.pitch));
}

const DEG = Math.PI / 180;
// `v` clamped between `lo` and `hi`, whichever way round they come. Not THREE.MathUtils.clamp: this file
// imports nothing, so node runs its tests and check-maps.mjs reads it.
const fit = (v, lo, hi) => Math.max(Math.min(lo, hi), Math.min(Math.max(lo, hi), v));

/** Where the camera may look in a room (a map's `camera.clamp`): `[x, z]`, the aim kept so the view stays
 * inside the room. The ground the view shows spans [aim + top, aim + bottom] along z (perspective makes the
 * far side longer) and ±halfW along x; its top edge is the top bar's lower edge (`cover`, the share of the
 * view under the bar), not the canvas's. A room bigger than the view: the view stays inside it. Smaller: the
 * camera still follows the hero, but only as far as keeps the whole room (and its back wall, about 1 m) in
 * view. A room deeper than the view leans to its back row (counters, shelves) as long as the hero stays a
 * little over the bottom edge.
 *
 * `v`: `dist` the camera's real distance (with a scene's lean-in, which lets a shot go closer to a wall,
 * #155), `pitch` in radians, `fov` (vertical, degrees), `aspect`, `cover`, `offY` the look-at height,
 * `width`/`height` the room's, `heroZ`, `wall` how far past the side walls a scene's shot may show (#155),
 * and `hold`: a scene's shot holds its own depth. */
export function clampToRoom(x, z, v) {
  const half = (v.fov / 2) * DEG, hc = v.dist * Math.sin(v.pitch) + v.offY, d0 = v.dist * Math.cos(v.pitch);
  const halfTop = Math.atan((1 - 2 * v.cover) * Math.tan(half));
  const top = v.pitch > halfTop ? d0 - hc / Math.tan(v.pitch - halfTop) : -Infinity, bottom = d0 - hc / Math.tan(v.pitch + half);
  const halfW = v.dist * Math.tan(half) * v.aspect;
  x = fit(x, halfW - v.wall, v.width - halfW + v.wall);
  if (Number.isFinite(top)) {
    const lo = -top - 1, hi = v.height - bottom;
    z = fit(z, lo, lo < hi ? (v.hold ? hi : Math.max(lo, Math.min(hi, v.heroZ + 1.2 - bottom))) : hi);
  }
  return [x, z];
}

/** How far in the camera leans, as a factor on its distance. In a scene (#155: a scene reads as one) it
 * leans in; on a place indoors (`place`: the sealed door, a stone) it comes close, so the place is the
 * subject. Resting (sitting or idle a while): in a little, but never under reduced motion. A phone held
 * upright (`tall`) already sees little across: it leans in less, so both people stay in frame (#155). */
export function leanOf({ scene, place, tall, resting, reduced }) {
  if (scene) return place ? (tall ? 0.8 : 0.55) : tall ? 0.95 : 0.85;
  return !reduced && resting ? 0.88 : 1;
}
