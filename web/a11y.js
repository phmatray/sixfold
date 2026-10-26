// Focus help for the Leptos views, which add and remove dialogs and confirm
// rows on the fly:
// - `autofocus` on content added after the page loaded (browsers only honour
//   it once, on load);
// - a modal ([aria-modal="true"]) keeps Tab inside it, gets focus when it
//   opens, and gives focus back to whatever had it when it closes;
// - when the focused control disappears (the action bar after a move, a card
//   leaving the hand), focus goes to the first control of the region it was
//   in ([data-focus-home]) instead of falling back to the page.
const opener = new WeakMap();
let open = [];

const focusables = (root) =>
  [...root.querySelectorAll('button, summary, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])')]
    .filter((el) => !el.disabled && el.getClientRects().length > 0);

let home = null;
addEventListener("focusin", (e) => {
  home = e.target.closest?.("[data-focus-home]") ?? null;
});

const lost = () => !document.activeElement || document.activeElement === document.body;

const find = (node, selector) => (node.matches(selector) ? node : node.querySelector(selector));

new MutationObserver((records) => {
  for (const r of records) {
    for (const node of r.addedNodes) {
      if (!(node instanceof Element)) continue;
      const dialog = find(node, '[aria-modal="true"]');
      if (dialog && !open.includes(dialog)) {
        opener.set(dialog, document.activeElement);
        open.push(dialog);
      }
      const auto = find(node, "[autofocus]");
      if (!auto && !dialog) continue;
      requestAnimationFrame(() => {
        if (auto?.isConnected) auto.focus();
        else if (dialog?.isConnected && !dialog.contains(document.activeElement)) focusables(dialog)[0]?.focus();
      });
    }
  }
  open = open.filter((d) => {
    if (d.isConnected) return true;
    const back = opener.get(d);
    if (lost() && back?.isConnected) back.focus();
    return false;
  });
  if (lost() && home?.isConnected && !open.length) {
    const region = home;
    requestAnimationFrame(() => {
      if (lost() && region.isConnected) (focusables(region)[0] ?? region).focus({ preventScroll: true });
    });
  }
}).observe(document.body, { childList: true, subtree: true });

addEventListener("keydown", (e) => {
  const dialog = open[open.length - 1];
  if (e.key !== "Tab" || !dialog?.isConnected) return;
  const f = focusables(dialog);
  if (!f.length) return;
  const inside = dialog.contains(document.activeElement);
  if (e.shiftKey && (!inside || document.activeElement === f[0])) {
    e.preventDefault();
    f[f.length - 1].focus();
  } else if (!e.shiftKey && (!inside || document.activeElement === f[f.length - 1])) {
    e.preventDefault();
    f[0].focus();
  }
});

// Directional focus (#69, the gamepad's D-pad and stick, see gamepad.js): the
// nearest control in that direction, inside the open modal if there is one.
// With nothing focused (or `dir` null), the first control on screen.
const AXES = { up: [0, -1], down: [0, 1], left: [-1, 0], right: [1, 0] };
// The open modal on top (a <dialog> or an [aria-modal]), else the page.
window.sixfoldLayer = () =>
  [...document.querySelectorAll('dialog[open], [aria-modal="true"]')].filter((d) => d.getClientRects().length).pop() ?? document.body;
window.sixfoldMoveFocus = (dir) => {
  const root = window.sixfoldLayer();
  // checkVisibility: not inside a closed <details> or a hidden layer, which keep their boxes.
  const all = focusables(root).filter((el) => el.checkVisibility?.({ visibilityProperty: true }) ?? true);
  const cur = document.activeElement;
  const go = (el) => el?.focus({ focusVisible: true });
  if (!dir || !AXES[dir] || !cur || cur === document.body || !root.contains(cur)) {
    const a = all.find((el) => { const r = el.getBoundingClientRect(); return r.bottom > 0 && r.top < innerHeight; }) ?? all[0];
    return go(a);
  }
  // A visually hidden input (a deck's radio) stands for its label, the card around it.
  const box = (el) => {
    const r = el.getBoundingClientRect(), label = el.labels?.[0] ?? el.closest("label");
    return r.width < 4 && label ? label.getBoundingClientRect() : r;
  };
  const [dx, dy] = AXES[dir];
  const a = box(cur);
  let best = null, score = Infinity;
  for (const el of all) {
    if (el === cur || el.contains(cur) || cur.contains(el)) continue;
    const b = box(el);
    // Ahead: the centre lies that way. Gap: edge to edge along the way, and across it (0 when they line up).
    const ahead = dx ? (b.left + b.right - a.left - a.right) * dx : (b.top + b.bottom - a.top - a.bottom) * dy;
    const gap = dx ? (dx > 0 ? b.left - a.right : a.left - b.right) : dy > 0 ? b.top - a.bottom : a.top - b.bottom;
    // Overlapping by more than half (a fanned hand seen up or down) is beside, not ahead.
    const half = Math.min(dx ? a.width : a.height, dx ? b.width : b.height) / 2;
    if (ahead <= 1 || gap < -half) continue;
    // On screen but under something else (a sticky bar over a scrolled list): not a target.
    const cx = (b.left + b.right) / 2, cy = (b.top + b.bottom) / 2;
    const top = cx >= 0 && cy >= 0 && cx < innerWidth && cy < innerHeight && document.elementFromPoint(cx, cy);
    if (top && !el.contains(top) && !(el.labels?.[0] ?? el.closest("label"))?.contains(top)) continue;
    const along = Math.max(0, gap);
    const across = Math.max(0, dx ? Math.max(b.top - a.bottom, a.top - b.bottom) : Math.max(b.left - a.right, a.left - b.right));
    // In line first (the boxes overlap across the way), then the nearest.
    const s = along + across * 3 + (across > 0 ? 1e5 : 0);
    if (s < score) (score = s), (best = el);
  }
  if (best) go(best);
};
