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
  const closed = open.filter((d) => !d.isConnected);
  open = open.filter((d) => d.isConnected);
  // Focus goes back to what opened a closed modal, unless another one is open
  // (a menu that gave way to its confirmation): that one takes it.
  for (const d of closed) {
    const back = opener.get(d);
    if (lost() && back?.isConnected && !open.length) back.focus();
  }
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

// Escape (a pad's B) folds the open <details> that focus is in or on (the title's
// Options, a section), and focus goes back to its summary: back, one step at a time.
addEventListener("keydown", (e) => {
  if (e.key !== "Escape" || e.defaultPrevented) return;
  const d = document.activeElement?.closest?.("details[open]");
  if (!d || d.closest('dialog, [aria-modal="true"]')) return;
  e.preventDefault();
  d.open = false;
  d.querySelector(":scope > summary")?.focus();
});
// Escape that nothing else took, on a menu screen: its Back (as a pad's B does).
addEventListener("keydown", (e) => {
  if (e.key !== "Escape" || e.defaultPrevented || e.target.closest?.('input:not([type="checkbox"], [type="radio"], [type="range"]), select, textarea')) return;
  if (document.querySelector('dialog[open], [aria-modal="true"]')) return;
  const back = document.querySelector(".page-bar .back");
  if (back?.getClientRects().length) e.preventDefault(), back.click();
});

// The arrows move the cursor on menu screens too (#100), as a pad's D-pad does: the
// nearest control that way. Not where arrows already mean something: text fields,
// sliders, selects, a radio group, a tab list, the world's walking and the duel's board.
const DIRS = { ArrowUp: "up", ArrowDown: "down", ArrowLeft: "left", ArrowRight: "right" };
addEventListener("keydown", (e) => {
  const dir = DIRS[e.key];
  if (!dir || e.defaultPrevented || e.altKey || e.ctrlKey || e.metaKey) return;
  const t = e.target;
  if (t.closest?.('input:not([type="checkbox"], [type="radio"]), select, textarea, [role="tab"], [role="listbox"], [role="grid"], [contenteditable="true"]')) return;
  const layered = window.sixfoldLayer() !== document.body;
  if (!layered && document.querySelector(".world-stage, .duel")) return;
  e.preventDefault();
  window.sixfoldMoveFocus(t === document.body ? null : dir);
  // Within a set of choices (the decks on offer), the choice follows the cursor,
  // as a radio group's arrows do; the arrows also lead out of it.
  const to = document.activeElement;
  if (t.type === "radio" && to !== t && to?.type === "radio" && to.name === t.name && !to.disabled) to.click();
});

// The cursor (#100): a painted hand beside whatever the keys or a pad focus, the
// one focus marker (style.css drops the ring where it points). A pointer click
// shows none, as a ring wouldn't. It lives in the top layer (a popover), so it
// shows over open dialogs, and follows its control while that moves or scrolls.
// The duel's board, the world's scene and the cards keep their own focus looks.
(() => {
  const hand = document.createElement("div");
  hand.className = "focus-hand";
  hand.setAttribute("aria-hidden", "true");
  const top = "showPopover" in hand;
  if (top) hand.popover = "manual";
  document.body.append(hand);
  document.documentElement.classList.add("hand-cursor");
  const OWN = ".duel .mat *, .world-stage, .world-stage *, .card, .hexon-card, .hand-card, .atlas-post, .talk-box, .press-start, h1, h2, main, [tabindex='-1']";
  let at = null, raf = 0;
  const shown = (el) => el.matches(":focus-visible") || (document.documentElement.dataset.input === "pad" && el.matches(":focus"));
  // A visually hidden input (a deck's radio, a segment) stands for its label.
  const box = (el) => {
    const r = el.getBoundingClientRect(), label = el.labels?.[0] ?? el.closest("label");
    return r.width < 4 && label ? label.getBoundingClientRect() : r;
  };
  const place = () => {
    raf = 0;
    if (!at?.isConnected || document.activeElement !== at) return hide();
    const r = box(at);
    // Left of the control, pointing at it; from the right when it starts at the
    // screen's edge; else as far left as the screen allows.
    const right = r.left < 24 && r.right <= innerWidth - 44;
    const x = right ? r.right + 2 : Math.max(2, r.left - 42), y = r.top + Math.min(r.height, 60) / 2 - 16;
    hand.classList.toggle("flip", right);
    hand.style.translate = `${Math.round(x)}px ${Math.round(y)}px`;
    raf = requestAnimationFrame(place);
  };
  const hide = () => {
    cancelAnimationFrame(raf);
    raf = 0;
    at?.removeAttribute("data-hand");
    at = null;
    hand.classList.remove("on");
  };
  addEventListener("focusin", (e) => {
    const el = e.target;
    if (!(el instanceof Element) || el.matches(OWN) || !shown(el)) return hide();
    at?.removeAttribute("data-hand");
    at = el;
    el.setAttribute("data-hand", "");
    // Back on top of whatever opened since (a dialog joins the top layer after it).
    if (top) {
      if (hand.matches(":popover-open")) hand.hidePopover();
      hand.showPopover();
    }
    if (!hand.classList.contains("on")) {
      // Appears where it points, then glides from there.
      hand.classList.add("on", "fresh");
      requestAnimationFrame(() => requestAnimationFrame(() => hand.classList.remove("fresh")));
    }
    if (!raf) place();
  });
  addEventListener("focusout", (e) => e.target === at && requestAnimationFrame(() => document.activeElement !== at && hide()));
  addEventListener("pointerdown", (e) => e.pointerType && hide(), true);
})();
