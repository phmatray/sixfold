// A camera shot authored on a Theatre.js timeline (#253): web/world/shots/<name>.json is a Theatre state whose sheet
// has one object, `camera` (x, y, z: where it stands; tx, ty, tz: what it looks at; fov). The core is vendored
// (web/vendor/theatre) and imported only when a shot first plays, so a game that never plays one never loads it.
import { CUT_HOLD } from "./script.js";

const PROPS = { x: 0, y: 0, z: 0, tx: 0, ty: 0, tz: 0, fov: 40 };
let core = null;

// playShot(name, { reduced }) -> { pose(), done, finish() }.
// pose(): the camera now ({ x, y, z, tx, ty, tz, fov }), null until the shot has loaded (or when it never will).
// done: resolves when the shot has played out, was finished, or could not load (one warning, then at once).
// finish(): jump to the end now and resolve. Under reduced motion nothing plays: the end is held for CUT_HOLD.
export function playShot(name, { reduced = false } = {}) {
  let obj = null, seq = null, len = 0, over = false, resolve;
  const done = new Promise((r) => (resolve = r));
  const stop = () => ((over = true), resolve());
  const finish = () => {
    if (seq) seq.position = len;
    stop();
  };
  (async () => {
    try {
      core ??= await import("@theatre/core");
      const res = await fetch("world/shots/" + name + ".json");
      if (!res.ok) throw new Error("HTTP " + res.status);
      const state = await res.json();
      len = state.sheetsById[name].sequence.length;
      const project = core.getProject("sixfold", { state });
      await project.ready; // a sequence plays only on a project that is ready
      const sheet = project.sheet(name);
      const o = sheet.object("camera", PROPS);
      seq = sheet.sequence;
      if (over) seq.position = len; // finished while loading: the end is the pose
      else if (reduced) {
        seq.position = len;
        setTimeout(stop, CUT_HOLD * 1000);
      } else seq.play({ range: [0, len] }).then(stop, stop);
      obj = o;
    } catch (e) {
      console.warn("world: no shot", name, e);
      stop();
    }
  })();
  return { pose: () => (obj ? { ...obj.value } : null), done, finish };
}
