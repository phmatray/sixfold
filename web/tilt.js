// Tilt and foil light for [data-tilt] frames (the inspector's card, the
// pack reveal's Rare, the sealed pack): follows the pointer on a desktop and
// a finger dragged across it on a phone. Sets, on the frame:
//   --rx, --ry  the turn in degrees, --px, --py  where the light falls
//   (-0.5 to 0.5 across the frame),
//   --o  how strong the light is (0 at rest, 1 while held),
// and the class `tilting` while it moves. A spring eases every value toward
// its target, one write per frame, and stops when they settle. Nothing runs
// with prefers-reduced-motion: the cards then rest flat in their foil.
const reduce = matchMedia("(prefers-reduced-motion: reduce)");
const MAX = 16; // degrees at the edge
const springs = new Map(); // frame -> { p: [rx, ry, px, py, o], v: [...], t: [...] }
let raf = 0;
let held = null;

const REST = [0, 0, 0, 0, 0];

// The frame's box, read once per hold: reading it on every move would force
// a style and layout pass right after each frame's writes.
const boxes = new WeakMap();
function aim(el, e) {
  let r = boxes.get(el);
  if (!r) boxes.set(el, (r = el.getBoundingClientRect()));
  const x = Math.min(Math.max((e.clientX - r.left) / r.width, 0), 1);
  const y = Math.min(Math.max((e.clientY - r.top) / r.height, 0), 1);
  set(el, [(0.5 - y) * 2 * MAX, (x - 0.5) * 2 * MAX, x - 0.5, y - 0.5, 1]);
}

function set(el, t) {
  let s = springs.get(el);
  if (!s) springs.set(el, (s = { p: REST.slice(), v: [0, 0, 0, 0, 0], t }));
  s.t = t;
  el.classList.add("tilting");
  if (!raf) raf = requestAnimationFrame(step);
}

let last = 0;
function step(now) {
  const dt = Math.min((now - (last || now)) / 16.7, 3) || 1;
  last = now;
  for (const [el, s] of springs) {
    let moving = false;
    for (let i = 0; i < 5; i++) {
      s.v[i] = (s.v[i] + (s.t[i] - s.p[i]) * 0.09 * dt) * Math.pow(0.78, dt);
      s.p[i] += s.v[i] * dt;
      if (Math.abs(s.v[i]) > 0.002 || Math.abs(s.t[i] - s.p[i]) > 0.004) moving = true;
    }
    const [rx, ry, px, py, o] = s.p;
    el.style.setProperty("--rx", rx.toFixed(2));
    el.style.setProperty("--ry", ry.toFixed(2));
    el.style.setProperty("--px", px.toFixed(4));
    el.style.setProperty("--py", py.toFixed(4));
    el.style.setProperty("--o", o.toFixed(3));
    if (!moving && s.t === REST) {
      springs.delete(el);
      el.classList.remove("tilting");
      for (const k of ["--rx", "--ry", "--px", "--py", "--o"]) el.style.removeProperty(k);
    } else if (!el.isConnected) springs.delete(el);
  }
  raf = springs.size ? requestAnimationFrame(step) : 0;
  if (!raf) last = 0;
}

function release(el) {
  if (el) boxes.delete(el);
  if (el && springs.has(el)) set(el, REST);
}

addEventListener("pointermove", (e) => {
  if (reduce.matches) return;
  const el = e.target.closest?.("[data-tilt]");
  if (held && held !== el) release(held);
  held = el;
  // A finger tilts while it is down; a mouse or pen while it hovers.
  if (el && (e.pointerType !== "touch" || e.pressure > 0 || e.buttons)) aim(el, e);
}, { passive: true });
addEventListener("pointerdown", (e) => {
  if (reduce.matches) return;
  const el = e.target.closest?.("[data-tilt]");
  if (el && e.pointerType === "touch") {
    held = el;
    aim(el, e);
  }
}, { passive: true });
for (const type of ["pointerup", "pointercancel"]) {
  addEventListener(type, (e) => {
    if (e.pointerType === "touch") {
      release(held);
      held = null;
    }
  }, { passive: true });
}
document.addEventListener("pointerleave", () => {
  release(held);
  held = null;
});
addEventListener("pointerout", (e) => {
  if (held && !held.contains(e.relatedTarget) && e.pointerType !== "touch") {
    release(held);
    held = null;
  }
}, { passive: true });
addEventListener("scroll", () => held && boxes.delete(held), { passive: true, capture: true });
addEventListener("resize", () => held && boxes.delete(held), { passive: true });
