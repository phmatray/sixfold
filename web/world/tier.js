// The graphics tier (#225): how much post-processing the world and the title draw.
// Pure ES with no imports (node --test reads it): the world (look.js), the title
// and the Graphics setting share one verdict. "high" is tone mapping, ambient
// occlusion, bloom and the grade; "low" the tone mapping and grade only; "off"
// today's plain frame. The player's choice ("auto" or a tier) is a device
// preference, not part of a save: Rust writes it (store.rs), this reads it.

export const TIERS = ["high", "low", "off"];
export const KEY = "sixfold.graphics";

/** The tier to draw: a chosen one is kept; Auto (or nothing saved, or junk) is High, Low on a weak device. */
export function pickTier(saved, weak) {
  return TIERS.includes(saved) ? saved : weak ? "low" : "high";
}

/** One step down, for a device that turns out slower than it looked. */
export function downgrade(tier) {
  return TIERS[Math.min(TIERS.indexOf(tier) + 1, TIERS.length - 1)] ?? "off";
}

/** The saved choice, or null (nothing saved, or storage missing or blocked). */
export function readSaved() {
  try {
    return globalThis.localStorage?.getItem(KEY) ?? null;
  } catch {
    return null;
  }
}

/** A device that can't carry a whole map at a good frame rate, known before trying:
 * a software renderer, little memory or few cores, or data saving asked for. */
export function weak(renderer) {
  const gl = renderer.getContext(), info = gl.getExtension("WEBGL_debug_renderer_info");
  const name = info ? String(gl.getParameter(info.UNMASKED_RENDERER_WEBGL)) : "";
  return /swiftshader|llvmpipe|software|basic render/i.test(name) || navigator.deviceMemory < 4 || navigator.hardwareConcurrency < 4 || !!navigator.connection?.saveData;
}
