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
