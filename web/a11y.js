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
  [...root.querySelectorAll('button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])')]
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
