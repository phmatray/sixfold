// Gamepad support (#69), as one small input layer over the game's own inputs:
// it polls the Gamepad API and plays each press as the key or focus move a
// keyboard player would make, so every screen keeps a single set of handlers.
//   stick / D-pad  arrows: walk in the world, else move focus to the nearest
//                  control that way (a11y.js `sixfoldMoveFocus`); left/right
//                  step a slider
//   act (A)        click the focused control (a list box: its next choice); in the world, E
//   cancel (B)     Escape, then close the open dialog, else the page's Back
//   run (X), stick pushed all the way   Shift, held
//   journal (Y), map (Select)           J, M
//   camera (R3)    C: the next outdoor camera angle (#148)
//   menu (Start)   a `sixfold-input` event the world takes (its in-game menu),
//                  else Enter (the splash's "Press to start")
//   L / R          the previous / next tab of the open tab list
// It also tracks the device last used ("keys" for keyboard and mouse, "touch",
// "pad") on <html data-input> with a `sixfold-device` event, so the Rust side
// can name the right control in its prompts, and keeps the remappable bindings
// (localStorage `sixfold.controls`, a per-browser convenience): a remapped key
// is replayed as its default one, so the handlers never learn about remapping.
//
// The pure part (mapButtons, translateKey, checkBind, label) is tested with
// `node --test web/gamepad.test.mjs`; the rest needs a browser.

export const ACTIONS = ["act", "cancel", "run", "menu", "journal", "map", "camera"];
// The standard mapping: 0-3 face buttons (bottom, right, left, top), 4/5
// shoulders, 6/7 triggers, 8 select, 9 start, 10/11 stick clicks, 12-15 D-pad.
export const PAD_DEFAULTS = { act: 0, cancel: 1, run: 2, journal: 3, map: 8, menu: 9, camera: 11 };
export const KEY_DEFAULTS = { act: "e", cancel: "Escape", run: "Shift", journal: "j", map: "m", menu: "Escape", camera: "c" };
const PAD_FIXED = [4, 5, 12, 13, 14, 15]; // tabs and the D-pad always work
const KEY_FIXED = ["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "w", "a", "s", "d", "z", "q", "g", "Tab", "Enter", " ", "Control", "Meta", "Alt"];
const DEAD = 0.5, FULL = 0.9; // stick: past DEAD it walks, past FULL it runs
const REPEAT_AFTER = 400, REPEAT_EVERY = 130; // ms, a held direction in menus

const norm = (k) => (typeof k === "string" && k.length === 1 ? k.toLowerCase() : k);
const pressed = (b) => (typeof b === "object" && b ? b.pressed || b.value > 0.5 : b > 0.5);

/** The inputs a pad holds right now: actions, "up"/"down"/"left"/"right", "tabPrev"/"tabNext". */
export function mapButtons(state, pad = PAD_DEFAULTS) {
  const on = (i) => pressed(state.buttons?.[i]);
  const out = new Set(ACTIONS.filter((a) => on(pad[a])));
  if (on(4)) out.add("tabPrev");
  if (on(5)) out.add("tabNext");
  const dirs = { up: on(12), down: on(13), left: on(14), right: on(15) };
  const [x = 0, y = 0] = state.axes ?? [];
  const mag = Math.hypot(x, y);
  if (mag > DEAD) {
    if (Math.abs(x) > Math.abs(y)) dirs[x < 0 ? "left" : "right"] = true;
    else dirs[y < 0 ? "up" : "down"] = true;
    if (mag > FULL) out.add("run");
  }
  for (const d in dirs) if (dirs[d]) out.add(d);
  return [...out];
}

/** A physical key as the game should hear it: its default key when remapped,
 *  `null` to drop it (a default key moved elsewhere), `undefined` to leave it. */
export function translateKey(key, keys = KEY_DEFAULTS) {
  const k = norm(key);
  if (k === "Escape") return undefined; // always closes and goes back, whatever the bindings
  const bound = ACTIONS.find((a) => norm(keys[a]) === k);
  if (bound) return norm(KEY_DEFAULTS[bound]) === k ? undefined : KEY_DEFAULTS[bound];
  return ACTIONS.some((a) => norm(KEY_DEFAULTS[a]) === k) ? null : undefined;
}

/** Why `value` can't be bound to `action` ("reserved" or "taken:<action>"), or "" when it can. */
export function checkBind(binds, device, action, value) {
  if (device === "pad") {
    if (PAD_FIXED.includes(value)) return "reserved";
  } else {
    value = norm(value);
    if (KEY_FIXED.includes(value) || (value === "Escape" && action !== "cancel" && action !== "menu")) return "reserved";
  }
  const map = binds[device];
  // On a keyboard, Escape is both back and the menu (the world opens it when nothing is open).
  const shared = (a) => device === "keys" && [a, action].every((x) => x === "cancel" || x === "menu");
  const other = ACTIONS.find((a) => a !== action && !shared(a) && norm(map[a]) === value);
  return other ? `taken:${other}` : "";
}

const PAD_NAMES = ["A", "B", "X", "Y", "L", "R", "L2", "R2", "Select", "Start", "L3", "R3", "↑", "↓", "←", "→"];
const KEY_NAMES = {
  Escape: ["Esc", "Échap"], Shift: ["Shift", "Maj"], " ": ["Space", "Espace"], Enter: ["Enter", "Entrée"],
  Backspace: ["Backspace", "Retour arrière"], ArrowUp: ["↑", "↑"], ArrowDown: ["↓", "↓"], ArrowLeft: ["←", "←"], ArrowRight: ["→", "→"],
};

/** What to call a control on screen: a key, a pad button, or "" for touch (there is no key to name). */
export function label(device, action, fr, binds = { keys: KEY_DEFAULTS, pad: PAD_DEFAULTS }) {
  const t = (en, f) => (fr ? f : en);
  if (device === "pad") {
    if (action === "walk") return t("Stick or D-pad", "Stick ou croix");
    if (action === "confirm") action = "act";
    const b = binds.pad[action];
    return PAD_NAMES[b] ?? t(`Button ${b}`, `Bouton ${b}`);
  }
  if (device !== "keys") return "";
  if (action === "walk") return t("Arrows or WASD", "Flèches ou ZQSD");
  if (action === "confirm") return t("Enter", "Entrée");
  const k = binds.keys[action];
  return KEY_NAMES[k]?.[fr ? 1 : 0] ?? (k.length === 1 ? k.toUpperCase() : k);
}

// ---------- In the browser ----------

const STORE = "sixfold.controls";
let binds = { keys: { ...KEY_DEFAULTS }, pad: { ...PAD_DEFAULTS } };
let device = "keys", capturing = null, started = false;
const ours = new WeakSet(); // the key events this module plays, which it must not translate again

const save = () => {
  try {
    localStorage.setItem(STORE, JSON.stringify(binds));
  } catch {}
};
const changed = (result) => dispatchEvent(new CustomEvent("sixfold-bindings", { detail: { result } }));

export function currentDevice() {
  return device;
}
function setDevice(d) {
  if (d === device) return;
  device = d;
  document.documentElement.dataset.input = d;
  dispatchEvent(new Event("sixfold-device"));
}

/** The label for `action` on `device` (the one in use when omitted), with the current bindings. */
export function prompt(action, dev, fr) {
  return label(dev || device, action, fr, binds);
}

export function keyOf(action) {
  return binds.keys[action] ?? "";
}

/** Bind the next key or pad button pressed to `action`; `sixfold-bindings` reports the outcome. */
export function capture(dev, action) {
  capturing = { dev, action };
}
export function stopCapture() {
  capturing = null;
}
function bind(value) {
  const { dev, action } = capturing;
  capturing = null;
  const why = checkBind(binds, dev, action, value);
  if (!why) {
    binds[dev][action] = dev === "pad" ? value : norm(value);
    save();
  }
  changed(why);
}
export function resetBindings(dev) {
  binds[dev] = { ...(dev === "pad" ? PAD_DEFAULTS : KEY_DEFAULTS) };
  save();
  changed("");
}

// Play a key on the focused element (where a real one would go); true when a handler took it.
function send(type, key, repeat = false, target = document.activeElement ?? document.body) {
  const e = new KeyboardEvent(type, { key, bubbles: true, cancelable: true, repeat });
  ours.add(e);
  target.dispatchEvent(e);
  return e.defaultPrevented;
}

const ARROWS = { up: "ArrowUp", down: "ArrowDown", left: "ArrowLeft", right: "ArrowRight" };
const shown = (el) => el.getClientRects().length > 0;
const layer = () => globalThis.sixfoldLayer?.() ?? document.body; // a11y.js: the open modal, else the page
const world = () => document.querySelector(".world-stage");
// In the world, A is E (talk, use) while focus is on the scene or its act prompt; any other
// control (Retry, the top bar, Skip) is clicked like everywhere else.
const onStage = (el) => world() && (!el || el === document.body || !!el.closest?.(".world-stage, .world-prompt"));

// A direction nothing took: step a slider, else move focus.
function nudge(dir) {
  const el = document.activeElement;
  const step = dir === "left" ? -1 : dir === "right" ? 1 : 0;
  if (step && el?.matches?.('input[type="range"]')) {
    step < 0 ? el.stepDown() : el.stepUp();
    for (const t of ["input", "change"]) el.dispatchEvent(new Event(t, { bubbles: true }));
  } else globalThis.sixfoldMoveFocus?.(dir);
}

function activate() {
  const el = document.activeElement;
  if (onStage(el)) return void (send("keydown", KEY_DEFAULTS.act), send("keyup", KEY_DEFAULTS.act));
  if (el && el !== document.body && el.matches('button, a[href], summary, input[type="checkbox"], input[type="radio"], [role="button"], [role="tab"], [role="option"]')) {
    return el.click();
  }
  // A list box can't open from a script: A steps to its next choice instead.
  if (el?.tagName === "SELECT" && el.options.length) {
    el.selectedIndex = (el.selectedIndex + 1) % el.options.length;
    return void el.dispatchEvent(new Event("change", { bubbles: true }));
  }
  if (el?.matches?.("input, select, textarea")) return;
  if (!send("keydown", "Enter")) globalThis.sixfoldMoveFocus?.(null); // nothing focused yet: wake focus up
}

function back() {
  const el = document.activeElement ?? document.body;
  const layered = document.querySelector('dialog[open], [aria-modal="true"], .inspector.open');
  if (send("keydown", "Escape")) return;
  // A native modal closes on a real Escape only; a played one asks it the same way.
  const dialog = [...document.querySelectorAll("dialog[open]")].pop();
  if (dialog) {
    const c = new Event("cancel", { cancelable: true });
    dialog.dispatchEvent(c);
    if (!c.defaultPrevented) dialog.close();
    return;
  }
  const details = el.closest?.("details[open]");
  if (details) {
    details.open = false;
    return details.querySelector("summary")?.focus();
  }
  if (!layered) [...document.querySelectorAll(".back")].find((b) => b.matches("button") && shown(b))?.click();
}

function tab(step) {
  const list = layer().querySelector('[role="tablist"]');
  const tabs = [...(list?.querySelectorAll('[role="tab"]') ?? [])].filter(shown);
  if (!tabs.length) return;
  const at = tabs.findIndex((t) => t.getAttribute("aria-selected") === "true");
  const to = tabs[(at + step + tabs.length) % tabs.length];
  to.click();
  to.focus();
}

// Up and down leave a tab row (whose keys cycle its tabs; L and R do that on a pad).
const across = (a) => (a === "up" || a === "down") && document.activeElement?.closest?.('[role="tablist"]');

function press(a) {
  if (ARROWS[a]) return void ((across(a) || !send("keydown", ARROWS[a])) && nudge(a));
  const e = new CustomEvent("sixfold-input", { cancelable: true, detail: { action: a, device: "pad" } });
  if (!dispatchEvent(e)) return; // taken (the world's menu)
  if (a === "act") activate();
  else if (a === "cancel") back();
  else if (a === "run") send("keydown", KEY_DEFAULTS.run);
  else if (a === "journal" || a === "map" || a === "camera") send("keydown", KEY_DEFAULTS[a]), send("keyup", KEY_DEFAULTS[a]);
  else if (a === "menu") send("keydown", "Enter"); // off the world: the splash's "Press to start"
  else if (a === "tabPrev" || a === "tabNext") tab(a === "tabNext" ? 1 : -1);
}
function release(a) {
  if (ARROWS[a]) send("keyup", ARROWS[a]);
  else if (a === "run") send("keyup", KEY_DEFAULTS.run);
}

let held = new Set(), since = {}, raf = 0, last = [];
function poll(now) {
  raf = 0;
  const pads = [...(navigator.getGamepads?.() ?? [])].filter((p) => p?.connected);
  const buttons = pads.flatMap((p) => p.buttons.map((b, i) => (pressed(b) ? i : -1))).filter((i) => i >= 0);
  const binding = capturing?.dev === "pad";
  if (binding) {
    const fresh = buttons.find((i) => !last.includes(i));
    if (fresh !== undefined) bind(fresh);
  }
  const next = new Set(pads.flatMap((p) => mapButtons(p, binds.pad)));
  if (next.size || buttons.length) setDevice("pad");
  // While binding, presses only bind; what is still held after waits for its release.
  if (!binding) {
    for (const a of next) if (!held.has(a)) press(a), (since[a] = now);
    for (const a of held) if (!next.has(a)) release(a);
    // A direction held in a menu repeats, like a key; the world ignores repeats.
    for (const a of next) {
      if (ARROWS[a] && now - since[a] > REPEAT_AFTER) {
        since[a] = now - REPEAT_AFTER + REPEAT_EVERY;
        if (across(a) || !send("keydown", ARROWS[a], true)) nudge(a);
      }
    }
  }
  held = next;
  last = buttons;
  // Unplugged mid-walk: the loop above has just released everything; stop until one comes back.
  if (pads.length) raf = requestAnimationFrame(poll);
}
const wake = () => raf || (raf = requestAnimationFrame(poll));

function onKey(e) {
  if (ours.has(e)) return;
  if (e.type === "keydown") setDevice("keys");
  if (capturing?.dev === "keys" && e.type === "keydown") {
    // Tab still moves on (and gives up waiting): the panel stays keyboard-usable.
    if (e.key === "Tab") return void ((capturing = null), changed("cancelled"));
    e.preventDefault();
    e.stopImmediatePropagation();
    if (e.repeat) return;
    if (e.key === "Escape" && capturing.action !== "cancel" && capturing.action !== "menu") return void ((capturing = null), changed("cancelled"));
    return bind(e.key);
  }
  // Remapping is the world's (its controls); the duel and the menus keep their own keys.
  if (!world() || e.ctrlKey || e.metaKey || e.altKey || e.target.closest?.("input, select, textarea")) return;
  const to = translateKey(e.key, binds.keys);
  if (to === undefined) return;
  e.preventDefault();
  e.stopImmediatePropagation();
  if (to) send(e.type, to, e.repeat, e.target);
}

/** Install the layer (once): listeners, saved bindings, the first device guess. */
export function start() {
  if (started) return;
  started = true;
  try {
    const saved = JSON.parse(localStorage.getItem(STORE) ?? "{}");
    // Only well-typed values: a broken entry must not break every prompt.
    for (const a of ACTIONS) {
      if (typeof saved.keys?.[a] === "string" && saved.keys[a]) binds.keys[a] = saved.keys[a];
      if (Number.isInteger(saved.pad?.[a])) binds.pad[a] = saved.pad[a];
    }
  } catch {}
  device = matchMedia("(pointer: coarse)").matches ? "touch" : "keys";
  document.documentElement.dataset.input = device;
  addEventListener("keydown", onKey, true);
  addEventListener("keyup", onKey, true);
  addEventListener("pointerdown", (e) => setDevice(e.pointerType === "mouse" ? "keys" : "touch"), true);
  addEventListener("gamepadconnected", wake);
  addEventListener("blur", () => (held = new Set())); // back in the tab, a held direction presses again
  wake();
}
