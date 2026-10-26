// Sixfold audio: every sound is synthesised here with WebAudio, nothing is
// sampled. Rust calls the exports through wasm-bindgen (web/src/audio.rs);
// world.js reaches the same functions through `globalThis.sixfoldAudio`.
//
// Three groups, each under the master: music (the tracks), ambience (the
// regional beds), effects (sfx and the UI's clicks), all through one
// compressor. `sixfoldAudio.debug().levels` reads each group's loudness.
//
// No sound before the first user gesture: the AudioContext is created inside
// a pointerdown/keydown handler. Music runs on a look-ahead step sequencer
// clocked by the AudioContext; the context is suspended while the tab is
// hidden or the game is muted.

const KEY = "sixfold.audio";
const DEFAULTS = { master: 0.8, music: 0.5, ambience: 0.6, sfx: 0.8, muted: 0 };
const prefs = { ...DEFAULTS };
try {
  Object.assign(prefs, JSON.parse(localStorage.getItem(KEY) ?? "{}"));
} catch {}

let ctx = null, master, musicBus, sfxBus, ambBus, packBus = null, noiseBuf, timer = 0;
let want = "", wantBed = [""], intense = false;
let taps = {}, meters = null;
const players = [];
const stats = { sfx: 0, last: "", notes: 0 };

// ---------- Settings ----------

export function getPref(k) {
  return Number(prefs[k] ?? 0);
}

export function setPref(k, v) {
  prefs[k] = Number(v);
  try {
    localStorage.setItem(KEY, JSON.stringify(prefs));
  } catch {}
  applyLevels();
  wake();
}

function applyLevels() {
  if (!ctx) return;
  const t = ctx.currentTime;
  // Squared: the sliders feel linear to the ear.
  master.gain.setTargetAtTime(prefs.muted ? 0 : prefs.master ** 2, t, 0.03);
  musicBus.gain.setTargetAtTime(prefs.music ** 2, t, 0.03);
  sfxBus.gain.setTargetAtTime(prefs.sfx ** 2, t, 0.03);
  ambBus.gain.setTargetAtTime(prefs.ambience ** 2, t, 0.03);
}

// ---------- Context, gesture unlock, visibility ----------

function unlock() {
  if (!ctx) {
    const AC = globalThis.AudioContext ?? globalThis.webkitAudioContext;
    if (!AC) return;
    ctx = new AC();
    master = ctx.createGain();
    const comp = ctx.createDynamicsCompressor();
    comp.threshold.value = -14;
    comp.ratio.value = 4;
    master.connect(comp).connect(ctx.destination);
    musicBus = ctx.createGain();
    sfxBus = ctx.createGain();
    ambBus = ctx.createGain();
    for (const b of [musicBus, sfxBus, ambBus]) b.connect(master);
    taps = { music: musicBus, ambience: ambBus, sfx: sfxBus, out: comp };
    // Four seconds, so a looping bed never repeats audibly.
    noiseBuf = ctx.createBuffer(1, ctx.sampleRate * 4, ctx.sampleRate);
    const d = noiseBuf.getChannelData(0);
    for (let i = 0; i < d.length; i++) d[i] = Math.random() * 2 - 1;
    applyLevels();
    const w = want;
    want = "";
    music(w);
    ambience(...wantBed);
  }
  wake();
}

// Run when audible (visible, not muted), sleep otherwise.
function wake() {
  if (!ctx) return;
  const on = !document.hidden && !prefs.muted;
  if (on && ctx.state !== "running") ctx.resume().catch(() => {});
  if (!on && ctx.state === "running") ctx.suspend().catch(() => {});
  if (on && !timer) timer = setInterval(tick, 25);
  if (!on && timer) (clearInterval(timer), (timer = 0));
}

for (const ev of ["pointerdown", "keydown", "touchend"]) addEventListener(ev, unlock, { capture: true, passive: true });
document.addEventListener("visibilitychange", wake);

// One delegated listener for every button: a soft click, and a tick on hover (mouse only).
document.addEventListener("click", (e) => e.target.closest?.("button, summary, a[href]") && sfx("click"), true);
let hovered = null;
document.addEventListener("pointerover", (e) => {
  if (e.pointerType !== "mouse") return;
  const b = e.target.closest?.("button:not(:disabled), summary, a[href]");
  if (b && b !== hovered) sfx("hover");
  hovered = b;
});

// ---------- Building blocks ----------

const hz = (m) => 440 * 2 ** ((m - 69) / 12);

// One oscillator note. `lin` = sustained (attack, hold, release), else a plucked decay.
function tone(dest, t, f, dur, o = {}) {
  const osc = ctx.createOscillator(), g = ctx.createGain();
  const a = o.a ?? 0.005, peak = o.peak ?? 0.2;
  osc.type = o.type ?? "sine";
  osc.frequency.setValueAtTime(f, t);
  if (o.f2) osc.frequency.exponentialRampToValueAtTime(o.f2, t + (o.glide ?? dur));
  if (o.detune) osc.detune.value = o.detune;
  g.gain.setValueAtTime(0, t);
  if (o.lin) {
    g.gain.linearRampToValueAtTime(peak, t + a);
    g.gain.linearRampToValueAtTime(peak * 0.7, t + Math.max(a, dur - 0.08));
    g.gain.linearRampToValueAtTime(0, t + dur + (o.rel ?? 0.15));
  } else {
    g.gain.linearRampToValueAtTime(peak, t + a);
    g.gain.exponentialRampToValueAtTime(0.0001, t + a + dur);
  }
  osc.connect(g).connect(dest);
  osc.start(t);
  osc.stop(t + a + dur + (o.rel ?? 0.15) + 0.05);
  stats.notes++;
}

// Filtered white noise: swishes, cracks, hats, footsteps.
function noise(dest, t, dur, o = {}) {
  const src = ctx.createBufferSource(), flt = ctx.createBiquadFilter(), g = ctx.createGain();
  src.buffer = noiseBuf;
  src.loop = true;
  flt.type = o.type ?? "bandpass";
  flt.frequency.setValueAtTime(o.f ?? 1000, t);
  if (o.f2) flt.frequency.exponentialRampToValueAtTime(o.f2, t + dur);
  flt.Q.value = o.q ?? 1;
  const a = o.a ?? 0.002, peak = o.peak ?? 0.2;
  g.gain.setValueAtTime(0, t);
  g.gain.linearRampToValueAtTime(peak, t + a);
  g.gain.exponentialRampToValueAtTime(0.0001, t + a + dur);
  src.connect(flt).connect(g).connect(dest);
  src.start(t, Math.random() * 0.5);
  src.stop(t + a + dur + 0.05);
  stats.notes++;
}

const notes = (dest, t, list, step, o) => list.forEach((m, i) => tone(dest, t + i * step, hz(m), o.dur ?? step * 2, o));

// ---------- Sound effects ----------

// Rapid repeats of one sound (a 7-card draw) queue up into a riffle; too many are dropped.
const nextFree = {};
const GAP = { draw: 0.05, place: 0.06, hover: 0.04, click: 0.03, blip: 0.03, step: 0.05 };

const SFX = {
  click: (d, t) => tone(d, t, 1250, 0.03, { f2: 800, peak: 0.12 }),
  hover: (d, t) => tone(d, t, 2400, 0.012, { peak: 0.025 }),
  draw: (d, t) => noise(d, t, 0.09, { f: 3200, f2: 1400, q: 1.2, peak: 0.18 }),
  place: (d, t) => {
    noise(d, t, 0.05, { type: "lowpass", f: 1400, peak: 0.25 });
    tone(d, t, 150, 0.08, { f2: 90, peak: 0.25 });
  },
  energy: (d, t) => {
    notes(d, t, [76, 80, 83, 88], 0.055, { type: "triangle", peak: 0.12, dur: 0.22 });
    noise(d, t + 0.05, 0.3, { type: "highpass", f: 6000, peak: 0.04 });
  },
  whoosh: (d, t) => noise(d, t, 0.32, { f: 350, f2: 2600, q: 2.5, a: 0.12, peak: 0.35 }),
  impact: (d, t, n) => {
    if (!n) return tone(d, t, 120, 0.12, { f2: 80, peak: 0.15 }); // all prevented: a muffled tap
    const k = Math.min(n / 80, 1.5);
    tone(d, t, 110 + 30 * k, 0.18 + 0.2 * k, { f2: 38, peak: 0.35 + 0.25 * k });
    noise(d, t, 0.08 + 0.1 * k, { type: "lowpass", f: 2400 + 2000 * k, f2: 300, peak: 0.25 + 0.2 * k });
    if (n >= 60) tone(d, t + 0.02, 55, 0.5, { type: "triangle", peak: 0.2 }); // big hits rumble
  },
  ko: (d, t) => {
    tone(d, t, 520, 0.7, { type: "triangle", f2: 70, peak: 0.22, lin: true });
    tone(d, t + 0.05, 260, 0.6, { type: "sawtooth", f2: 45, peak: 0.05, lin: true });
    noise(d, t + 0.55, 0.4, { type: "lowpass", f: 900, f2: 120, peak: 0.3 });
    tone(d, t + 0.55, 70, 0.4, { f2: 35, peak: 0.35 });
  },
  prize: (d, t) => {
    notes(d, t, [84, 91], 0.09, { peak: 0.16, dur: 0.5 });
    notes(d, t, [96, 103], 0.09, { peak: 0.04, dur: 0.4 });
  },
  // Spin pings that speed up then settle, then the landing: bright for heads, low for tails.
  coin: (d, t, heads) => {
    const pings = [0, 0.16, 0.29, 0.39, 0.47, 0.54, 0.61, 0.69, 0.79, 0.92];
    pings.forEach((p, i) => tone(d, t + p, i % 2 ? 2900 : 3400, 0.05, { type: "triangle", peak: 0.05 }));
    const land = t + 1.15;
    noise(d, land, 0.05, { f: 5000, q: 4, peak: 0.2 });
    if (heads) notes(d, land + 0.02, [88, 95], 0.1, { type: "triangle", peak: 0.14, dur: 0.45 });
    else notes(d, land + 0.02, [81, 76], 0.12, { type: "sine", peak: 0.15, dur: 0.4 });
  },
  // 0 asleep, 1 poisoned, 2 paralyzed, 3 confused.
  status: (d, t, s) => {
    if (s === 0) notes(d, t, [79, 76, 72], 0.22, { peak: 0.12, a: 0.06, dur: 0.4, lin: true });
    else if (s === 1)
      [0, 0.07, 0.15, 0.2, 0.28].forEach((p, i) => tone(d, t + p, 260 + i * 70, 0.06, { f2: 520 + i * 90, peak: 0.13 }));
    else if (s === 2) {
      for (let i = 0; i < 7; i++) tone(d, t + i * 0.045, 900 - i * 40, 0.03, { type: "sawtooth", peak: 0.07 });
      noise(d, t, 0.3, { f: 4000, q: 6, peak: 0.08 });
    } else {
      tone(d, t, 600, 0.45, { type: "triangle", f2: 400, peak: 0.1, lin: true });
      tone(d, t + 0.1, 450, 0.45, { type: "triangle", f2: 700, peak: 0.1, lin: true });
    }
  },
  evolve: (d, t) => {
    notes(d, t, [72, 74, 76, 79, 81, 84, 86, 88, 91], 0.05, { type: "triangle", peak: 0.09, dur: 0.3 });
    noise(d, t + 0.2, 0.6, { type: "highpass", f: 7000, a: 0.2, peak: 0.06 });
    tone(d, t + 0.45, hz(96), 0.9, { peak: 0.05 });
  },
  heal: (d, t) => notes(d, t, [79, 84], 0.1, { peak: 0.1, dur: 0.35 }),
  // A pack: the tear, ten cards turning (the Rare last and slower), its shine.
  // Matches the reveal's timeline in packs.rs. n = 1: shown all at once.
  pack: (d, t, n) => {
    if (packBus) packBus.disconnect();
    packBus = ctx.createGain();
    packBus.connect(sfxBus);
    const p = packBus;
    noise(p, t, 0.35, { f: 1200, f2: 4500, q: 0.8, a: 0.05, peak: 0.3 });
    noise(p, t + 0.12, 0.2, { type: "highpass", f: 3000, peak: 0.12 });
    if (n) return SFX.rare(p, t + 0.4);
    // FLIP_AT, FLIP_STEP and RARE_AT in packs.rs.
    for (let i = 0; i < 9; i++) SFX.flip(p, t + 0.9 + i * 0.18 + 0.28);
    SFX.flip(p, t + 3.12 + 0.45);
    SFX.rare(p, t + 3.12 + 0.75);
  },
  packstop: () => {
    if (packBus) packBus.disconnect();
    packBus = null;
  },
  flip: (d, t) => noise(d, t, 0.06, { f: 2600, q: 1.5, peak: 0.16 }),
  rare: (d, t) => {
    notes(d, t, [79, 83, 86, 91], 0.07, { type: "triangle", peak: 0.1, dur: 0.9 });
    notes(d, t + 0.3, [98, 103], 0.08, { peak: 0.05, dur: 0.8 });
    noise(d, t, 0.9, { type: "highpass", f: 8000, a: 0.3, peak: 0.05 });
  },
  // n: 0 grass, 1 wood floor.
  step: (d, t, n) => {
    const v = 0.85 + Math.random() * 0.3;
    if (n) {
      tone(d, t, 200 * v, 0.05, { f2: 140, peak: 0.12 });
      noise(d, t, 0.03, { f: 1300 * v, q: 2, peak: 0.05 });
    } else noise(d, t, 0.07, { type: "lowpass", f: 1100 * v, f2: 500, peak: 0.12 });
  },
  door: (d, t) => {
    tone(d, t, 140, 0.4, { type: "sawtooth", f2: 210, peak: 0.025, a: 0.05, lin: true });
    noise(d, t, 0.4, { f: 700, f2: 1100, q: 8, a: 0.05, peak: 0.08 });
    tone(d, t + 0.42, 90, 0.12, { f2: 60, peak: 0.25 });
    noise(d, t + 0.42, 0.05, { type: "lowpass", f: 1000, peak: 0.15 });
  },
  // n: semitones above the base voice, one per character.
  blip: (d, t, n) => tone(d, t, hz(64 + n) * (0.97 + Math.random() * 0.06), 0.035, { type: "square", peak: 0.03 }),
  jingle: (d, t) => {
    notes(d, t, [84, 88, 91, 96], 0.08, { type: "triangle", peak: 0.12, dur: 0.4 });
    [0.05, 0.13, 0.21, 0.29].forEach((p) => noise(d, t + p, 0.05, { type: "highpass", f: 7000, peak: 0.06 }));
  },
  // A route duelist spots the hero: a quick rising "!" sting.
  spot: (d, t) => {
    notes(d, t, [76, 83], 0.07, { type: "square", peak: 0.06, dur: 0.12 });
    noise(d, t, 0.12, { type: "highpass", f: 5000, peak: 0.05 });
  },
  // Facets gained: small glassy chimes.
  facets: (d, t) => {
    notes(d, t, [88, 93, 100], 0.05, { type: "triangle", peak: 0.08, dur: 0.35 });
    noise(d, t + 0.1, 0.12, { type: "highpass", f: 9000, peak: 0.04 });
  },
  // Campaign: a rival's duel accepted, a seal earned, the sealed door opening.
  challenge: (d, t) => {
    noise(d, t, 0.25, { f: 400, f2: 2400, q: 2, a: 0.1, peak: 0.25 });
    notes(d, t + 0.18, [67, 74, 79], 0.07, { type: "square", peak: 0.05, dur: 0.18 });
  },
  seal: (d, t) => {
    notes(d, t, [79, 83, 86, 91], 0.09, { type: "triangle", peak: 0.12, dur: 0.6 });
    tone(d, t + 0.36, hz(55), 1.2, { peak: 0.12, lin: true, rel: 0.6 });
  },
  gate: (d, t) => {
    SFX.door(d, t);
    notes(d, t + 0.5, [72, 79, 84, 91], 0.1, { type: "triangle", peak: 0.09, dur: 0.7 });
  },
  // Fast travel: a rising shimmer, then a soft landing chord.
  travel: (d, t) => {
    noise(d, t, 0.6, { f: 600, f2: 5200, q: 3, a: 0.25, peak: 0.12 });
    notes(d, t + 0.1, [79, 83, 86, 91, 95, 98], 0.06, { type: "triangle", peak: 0.06, dur: 0.5 });
    [67, 74, 79].forEach((m) => tone(d, t + 0.55, hz(m), 0.9, { peak: 0.05, lin: true, rel: 0.5 }));
  },
  // A rest at an inn: a slow falling lullaby, then a bright morning (or evening) chime.
  rest: (d, t) => {
    notes(d, t, [79, 76, 72, 67], 0.32, { type: "triangle", peak: 0.08, dur: 0.6, lin: true });
    notes(d, t + 1.5, [84, 88, 91], 0.09, { type: "triangle", peak: 0.08, dur: 0.6 });
  },
  // A waymark woken: a struck crystal.
  waymark: (d, t) => {
    tone(d, t, hz(91), 1.4, { type: "triangle", peak: 0.09, rel: 0.6 });
    tone(d, t + 0.02, hz(98), 1.1, { peak: 0.04, rel: 0.6 });
    noise(d, t, 0.5, { type: "highpass", f: 8000, a: 0.05, peak: 0.05 });
  },
  // A relay pylon thrown: a knife switch clunks home, the coil crackles up and hums.
  spark: (d, t) => {
    noise(d, t, 0.05, { type: "lowpass", f: 900, peak: 0.25 });
    for (let i = 0; i < 7; i++) noise(d, t + 0.08 + i * 0.045 + Math.random() * 0.02, 0.03, { type: "highpass", f: 4000 + Math.random() * 3000, peak: 0.07 });
    tone(d, t + 0.1, 55, 0.9, { type: "sawtooth", f2: 110, glide: 0.4, peak: 0.03, lin: true, rel: 0.4 });
  },
  // A lift: the gate clanks shut, then (n: seconds of travel) the winch hums and its pawl
  // ratchets, and a clank at the stop.
  winch: (d, t, n) => {
    const clank = (at) => {
      noise(d, at, 0.06, { type: "lowpass", f: 1400, peak: 0.22 });
      tone(d, at, 190, 0.1, { type: "square", f2: 120, peak: 0.035 });
    };
    clank(t);
    if (!n) return;
    tone(d, t + 0.35, 52, n, { type: "sawtooth", f2: 68, glide: n * 0.4, peak: 0.035, a: 0.3, lin: true, rel: 0.3 });
    for (let k = 0.4; k < n + 0.3; k += 0.16) noise(d, t + k, 0.02, { type: "highpass", f: 2600, peak: 0.03 });
    clank(t + 0.4 + n);
  },
  // Cooling glass singing: high, slightly detuned partials ringing out.
  chime: (d, t) => {
    [96, 103, 100, 108].forEach((m, i) => tone(d, t + i * 0.22, hz(m), 1.6, { peak: 0.035, detune: (i % 2 ? 9 : -7), rel: 0.8 }));
  },
  win: (d, t) => {
    notes(d, t, [72, 76, 79], 0.12, { type: "triangle", peak: 0.15, dur: 0.3 });
    [72, 76, 79, 84].forEach((m) => tone(d, t + 0.38, hz(m), 1.4, { type: "triangle", peak: 0.09, lin: true, rel: 0.6 }));
    tone(d, t + 0.38, hz(48), 1.4, { peak: 0.2, lin: true, rel: 0.6 });
  },
  lose: (d, t) => {
    notes(d, t, [67, 63, 60], 0.28, { type: "triangle", peak: 0.12, dur: 0.4, lin: true });
    [55, 58, 62].forEach((m) => tone(d, t + 0.9, hz(m), 1.3, { peak: 0.07, lin: true, rel: 0.8 }));
  },
};

export function sfx(name, n = 0) {
  const f = SFX[name];
  if (!ctx || ctx.state !== "running" || !f) return;
  const now = ctx.currentTime;
  let t = now;
  const gap = GAP[name];
  if (gap) {
    t = Math.max(now, nextFree[name] ?? 0);
    if (t - now > 0.4) return;
    nextFree[name] = t + gap;
  }
  stats.sfx++;
  stats.last = name;
  f(sfxBus, t + 0.005, n);
}

// ---------- Ambience: a bed per region, by day and by night ----------
//
// A bed is a few looping noise layers, each filtered and slowly swelling
// (wind, surf, a forge's roar), plus sparse voices at random intervals (a gull,
// a chime, a drip). Layer: filter `type`, `f` Hz, `q`, `gain`; `lfo` [Hz, depth
// 0..1] swells the level, `sweep` [Hz, depth 0..1] moves the filter.
// Voice: [name, at least, at most] seconds between two.

const rnd = (a, b) => a + Math.random() * (b - a);

const breeze = (gain, f = 500) => ({ type: "bandpass", f, q: 0.6, gain, lfo: [0.07, 0.7], sweep: [0.05, 0.4] });
const wind = (gain, f = 800) => ({ type: "bandpass", f, q: 1.4, gain, lfo: [0.11, 0.85], sweep: [0.09, 0.6] });
const surf = (gain) => ({ type: "lowpass", f: 700, q: 0.4, gain, lfo: [0.085, 0.95], sweep: [0.085, 0.5] });
const spray = (gain) => ({ type: "highpass", f: 3200, q: 0.5, gain, lfo: [0.085, 0.95] });
const roar = (gain, f = 260) => ({ type: "lowpass", f, q: 0.8, gain, lfo: [0.05, 0.3] });
// A choir of insects: a narrow band, beating fast.
const insects = (gain, f = 4800) => ({ type: "bandpass", f, q: 9, gain, lfo: [7, 0.9] });

const BEDS = {
  // The meadow (and the default for any outdoor map).
  overworld: {
    day: { layers: [breeze(0.05)], voices: [["bird", 2, 7], ["leaves", 6, 14]] },
    night: { layers: [breeze(0.035, 380), insects(0.006)], voices: [["cricket", 0.8, 2.5], ["owl", 18, 40]] },
  },
  // Ember: the forges' roar, coals crackling, steam from the vents.
  ember: {
    day: { layers: [roar(0.06), breeze(0.02, 700)], voices: [["crackle", 0.8, 2.5], ["vent", 8, 18]] },
    night: { layers: [roar(0.06, 200)], voices: [["crackle", 1.2, 3.5], ["vent", 10, 22], ["cricket", 3, 7]] },
  },
  // Storm: wind over slate, chimes, thunder far off.
  storm: {
    day: { layers: [wind(0.06)], voices: [["chime", 4, 10], ["thunder", 25, 50]] },
    night: { layers: [wind(0.07, 600)], voices: [["chime", 5, 12], ["thunder", 18, 40]] },
  },
  // Tide: surf and gulls; at night the swell and a buoy's bell.
  tide: {
    day: { layers: [surf(0.06), spray(0.009)], voices: [["gull", 5, 12]] },
    night: { layers: [surf(0.05), spray(0.006)], voices: [["bell", 12, 25]] },
  },
  // Dream: a hush, high shimmer by day, a whole choir of insects at night.
  dream: {
    day: { layers: [breeze(0.035, 900)], voices: [["shimmer", 5, 11], ["bird", 8, 18]] },
    night: { layers: [insects(0.02), insects(0.01, 6200), breeze(0.02, 700)], voices: [["cricket", 0.6, 1.8], ["shimmer", 7, 15]] },
  },
  // Grove: leaves and birdsong; frogs and an owl at night.
  grove: {
    day: { layers: [breeze(0.035, 1400)], voices: [["bird", 1.2, 4], ["leaves", 3, 8]] },
    night: { layers: [breeze(0.022, 900), insects(0.005)], voices: [["frog", 2, 6], ["owl", 12, 30], ["cricket", 1, 3]] },
  },
  // Stone: wind on the ledges, distant picks and drips; the quarry rests at night.
  stone: {
    day: { layers: [wind(0.06, 450)], voices: [["pick", 5, 12], ["drip", 3, 8]] },
    night: { layers: [wind(0.05, 380)], voices: [["drip", 3, 7], ["owl", 20, 45]] },
  },
  // Caves: a low rumble and drips with an echo.
  cave: { day: { layers: [roar(0.08, 110)], voices: [["drip", 1.5, 5]] } },
  // Indoors: room tone, the odd creak of a board.
  interior: { day: { layers: [roar(0.04, 180)], voices: [["creak", 10, 25]] } },
};

// Bed voices: (dest, time). Quiet by design: they sit under music and dialogue.
const BED_VOICES = {
  bird: (d, t) => {
    const f = rnd(2600, 3800), n = 2 + Math.floor(Math.random() * 4);
    for (let i = 0; i < n; i++) tone(d, t + i * rnd(0.08, 0.14), f * rnd(0.9, 1.15), 0.06, { f2: f * rnd(1.1, 1.4), peak: 0.035 });
  },
  leaves: (d, t) => noise(d, t, rnd(0.5, 1), { f: 2400, q: 0.8, a: 0.25, peak: 0.03 }),
  cricket: (d, t) => {
    const f = rnd(4200, 4700);
    for (let i = 0; i < 3; i++) tone(d, t + i * 0.06, f, 0.025, { peak: 0.012 });
  },
  owl: (d, t) => {
    tone(d, t, 410, 0.3, { f2: 370, peak: 0.03, a: 0.06, lin: true, rel: 0.2 });
    tone(d, t + 0.6, 400, 0.45, { f2: 350, peak: 0.03, a: 0.08, lin: true, rel: 0.25 });
  },
  frog: (d, t) => {
    for (let i = 0; i < 2; i++) tone(d, t + i * 0.16, 210, 0.07, { type: "triangle", f2: 160, peak: 0.04 });
  },
  crackle: (d, t) => {
    for (let i = 0, n = 3 + Math.floor(Math.random() * 6); i < n; i++) noise(d, t + rnd(0, 0.6), 0.012, { type: "highpass", f: rnd(2500, 6000), peak: rnd(0.02, 0.06) });
  },
  vent: (d, t) => noise(d, t, 1.6, { type: "highpass", f: 2600, f2: 1800, a: 0.6, peak: 0.03 }),
  chime: (d, t) => {
    for (let i = 0, n = 2 + Math.floor(Math.random() * 3); i < n; i++)
      tone(d, t + i * rnd(0.12, 0.3), hz(84 + [0, 2, 4, 7, 9, 12][Math.floor(Math.random() * 6)]), 1.6, { peak: 0.018, detune: rnd(-8, 8) });
  },
  thunder: (d, t) => noise(d, t, 3, { type: "lowpass", f: 260, f2: 50, a: 0.5, peak: 0.12 }),
  gull: (d, t) => {
    for (let i = 0, n = 2 + Math.floor(Math.random() * 2); i < n; i++)
      tone(d, t + i * 0.32, rnd(1300, 1600), 0.25, { type: "triangle", f2: 850, glide: 0.25, peak: 0.022, a: 0.03 });
  },
  bell: (d, t) => {
    tone(d, t, hz(69), 2.6, { peak: 0.02 });
    tone(d, t, hz(69) * 2.76, 1.6, { peak: 0.008 });
  },
  shimmer: (d, t) => notes(d, t, [93, 98, 100, 105].sort(() => Math.random() - 0.5), 0.18, { peak: 0.012, dur: 1.4 }),
  drip: (d, t) => {
    const f = rnd(900, 1500);
    tone(d, t, f, 0.05, { f2: f * 1.8, glide: 0.04, peak: 0.04 });
    tone(d, t + 0.28, f, 0.05, { f2: f * 1.8, glide: 0.04, peak: 0.012 }); // its echo
  },
  pick: (d, t) => {
    for (let i = 0; i < 3; i++) {
      const at = t + i * rnd(0.8, 1.1);
      noise(d, at, 0.06, { f: 2800, q: 9, peak: 0.03 });
      noise(d, at + 0.3, 0.06, { f: 2800, q: 9, peak: 0.01 });
    }
  },
  creak: (d, t) => {
    tone(d, t, 150, 0.35, { type: "sawtooth", f2: 210, peak: 0.006, a: 0.05, lin: true });
    noise(d, t, 0.35, { f: 900, f2: 1200, q: 9, a: 0.05, peak: 0.02 });
  },
};

const beds = [];
let bed = null;

function startBed(key, def, t) {
  const out = ctx.createGain(), stop = [];
  out.gain.setValueAtTime(0, t);
  out.gain.setTargetAtTime(1, t, 0.35);
  out.connect(ambBus);
  for (const L of def.layers) {
    const src = ctx.createBufferSource(), flt = ctx.createBiquadFilter(), g = ctx.createGain();
    src.buffer = noiseBuf;
    src.loop = true;
    flt.type = L.type;
    flt.frequency.value = L.f;
    flt.Q.value = L.q;
    const [rate, depth] = L.lfo ?? [0, 0];
    g.gain.value = L.gain * (1 - depth / 2);
    src.connect(flt).connect(g).connect(out);
    src.start(t, rnd(0, 4));
    stop.push(src);
    // A slow sine on the level and one on the filter, each at its own drifting rate.
    for (const [r, amt, param] of [[rate, (L.gain * depth) / 2, g.gain], [L.sweep?.[0], L.f * (L.sweep?.[1] ?? 0), flt.frequency]]) {
      if (!r || !amt) continue;
      const lfo = ctx.createOscillator(), lg = ctx.createGain();
      lfo.frequency.value = r * rnd(0.8, 1.2);
      lg.gain.value = amt;
      lfo.connect(lg).connect(param);
      lfo.start(t, 0);
      stop.push(lfo);
    }
  }
  const voices = (def.voices ?? []).map(([v, lo, hi]) => ({ v, lo, hi, next: t + rnd(0.5, hi) }));
  const b = { key, out, stop, voices, endAt: 0 };
  beds.push(b);
  return b;
}

/// Cross-fade to a region's bed ("tide", "cave", "interior", … or "" for none) at a phase
/// ("dawn", "day", "dusk", "night"): the night has its own bed, the rest share the day's.
export function ambience(name, phase = "day") {
  if (!ctx) return void (wantBed = [name, phase]);
  const set = BEDS[name], half = phase === "night" && set?.night ? "night" : "day";
  const key = set ? `${name}:${half}` : "";
  if ((bed?.key ?? "") === key) return;
  const t = ctx.currentTime;
  if (bed) {
    bed.out.gain.setTargetAtTime(0, t, 0.35);
    bed.endAt = t + 1.5;
  }
  bed = set ? startBed(key, set[half], t) : null;
}

function tickBeds(now) {
  for (let i = beds.length - 1; i >= 0; i--) {
    const b = beds[i];
    if (b.endAt && now > b.endAt) {
      b.stop.forEach((s) => s.stop());
      b.out.disconnect();
      beds.splice(i, 1);
      continue;
    }
    if (b.endAt) continue;
    for (const v of b.voices) {
      if (v.next < now - 1) v.next = now + rnd(v.lo, v.hi); // the timer stalled (a hidden tab): don't burst
      while (v.next < now + LOOKAHEAD) {
        BED_VOICES[v.v](b.out, v.next);
        v.next += rnd(v.lo, v.hi);
      }
    }
  }
}

// ---------- Music: a tiny step sequencer ----------
//
// A track is 16 steps (sixteenth notes) per bar. `chords` holds the scale
// degree of each bar's chord (a seventh, voiced an octave under `root`).
// Patterns are strings, one character per step: "." rest, "-" holds the note.
// lead: a scale degree in "0123456789abcdef", where "3" is the tonic (`root`).
// bass: "x" chord root, "5" its fifth, "o" its octave (two octaves down).
// keys: "x" a soft chord stab. Drums: "x" a hit.

// A scale degree past the last note climbs an octave (pentatonic has five, the others seven).
const SCALES = { major: [0, 2, 4, 5, 7, 9, 11], minor: [0, 2, 3, 5, 7, 8, 10], harmonic: [0, 2, 3, 5, 7, 8, 11], dorian: [0, 2, 3, 5, 7, 9, 10], pentatonic: [0, 2, 4, 7, 9] };
const DIGITS = "0123456789abcdef";

const TRACKS = {
  // Title and menus: an unhurried evening by the fire.
  title: {
    bpm: 76, root: 65, scale: "major", chords: [0, 5, 3, 4, 0, 5, 1, 4],
    lead:
      "7---5-6-7-----.." + "8---7-5-3-------" + "6---5-6-8---7---" + "4-----2-4-5-7---" +
      "7---5-6-7---a---" + "9---8-7-8-------" + "6-5-4-5-6---8---" + "7-------4---2---",
    bass: "x-------5---x---",
    drums: { kick: "x.......x.......", shaker: "....x.......x..." },
    // A touch under the others: it was 3 dB louder than the world and the duels.
    mix: { lead: 0.075, pad: 0.03, bass: 0.105, keys: 0.025 },
  },
  // The duel: a little more urgent, still friendly. Intensifies on the last prize.
  duel: {
    bpm: 108, root: 69, scale: "minor", chords: [0, 5, 2, 6, 0, 5, 3, 4],
    lead:
      "7-.7-.5-6-7-a-9-" + "8---7-5-3---5---" + "7-.7-.9-8-7-5-7-" + "9-------6---4---" +
      "7-.7-.5-6-7-a-c-" + "a---9-8-7---5---" + "6-8-a-8-6-5-6-8-" + "7-------4-2-0---",
    bass: "x.x.x.x.x.x.5.o.",
    drums: { kick: "x.......x.x.....", snare: "....x.......x...", hat: "..x...x...x...x." },
    hot: { shaker: "xxxxxxxxxxxxxxxx", arp: "x.x.x.x.x.x.x.x." },
    mix: { lead: 0.08, pad: 0.03, bass: 0.14, keys: 0.03 },
  },
  // The meadow: a walking tune with offbeat keys.
  overworld: {
    bpm: 96, root: 67, scale: "major", chords: [0, 3, 0, 4, 5, 3, 1, 4],
    lead:
      "3-5-7---8-7-5---" + "6-8-a---8-6-5---" + "3-5-7---a-9-8-7-" + "4-------2---0---" +
      "5-7-8---7-5-1---" + "6-5-6-8-a---8---" + "4-6-8-6-4-3-4-6-" + "7---------------",
    bass: "x...5...x...5.o.",
    keys: "..x...x...x...x.",
    drums: { kick: "x.......x.......", shaker: "x.x.x.x.x.x.x.x." },
    mix: { lead: 0.08, pad: 0.025, bass: 0.13, keys: 0.025 },
  },
};
// The six regions (WORLD_DESIGN §14.3), each in its own scale and pace; a map names its track with `music`.
Object.assign(TRACKS, {
  // Ember: dorian, a stoker's swing.
  ember: {
    bpm: 100, root: 62, scale: "dorian", chords: [0, 3, 0, 4, 0, 3, 6, 4],
    lead:
      "3-.3-5-6-7---6-5" + "4---3-1-3-------" + "3-.3-5-6-7-9-a--" + "9---7-6-7-------" +
      "a-.a-9-7-8---7-6" + "7---6-5-6-------" + "5-6-7-5-6-4-5-3-" + "3-------1---3---",
    bass: "x..x....5..x..o.",
    drums: { kick: "x.....x...x.....", snare: "....x.......x...", shaker: "..x...x...x...x." },
    mix: { lead: 0.075, pad: 0.025, bass: 0.14, keys: 0.025 },
  },
  // Storm: minor and quick, hats like rain on slate.
  storm: {
    bpm: 112, root: 64, scale: "minor", chords: [0, 5, 6, 4, 0, 5, 3, 4],
    lead:
      "7-7-a-7-9-7-6-5-" + "6---5-3-5-------" + "7-7-a-7-c-a-9-7-" + "8-------9---a---" +
      "a-a-c-a-9-7-6-7-" + "8---7-6-5-------" + "6-7-8-6-7-5-6-4-" + "3---------------",
    bass: "x.x...x.x.x...5.",
    keys: "x...x...x...x...",
    drums: { kick: "x.......x.......", snare: "....x.......x...", hat: "x.xxx.xxx.xxx.xx" },
    mix: { lead: 0.07, pad: 0.03, bass: 0.13, keys: 0.02 },
  },
  // Tide: pentatonic, slow and rocking, the shaker for surf.
  tide: {
    bpm: 88, root: 67, scale: "pentatonic", chords: [0, 3, 1, 4, 0, 3, 2, 4],
    lead:
      "5---6-7---8-7---" + "5-----3-4-------" + "5---6-7---a-8---" + "7---------------" +
      "8---7-5---6-5---" + "3-----4-5-------" + "6---5-3---4-3---" + "3---------------",
    bass: "x.....5.....x...",
    keys: "...x.....x....x.",
    drums: { kick: "x...........x...", shaker: "x..x..x..x..x..." },
    mix: { lead: 0.08, pad: 0.03, bass: 0.12, keys: 0.025 },
  },
  // Dream: pentatonic, slow, high and soft, no drums but a brush.
  dream: {
    bpm: 72, root: 69, scale: "pentatonic", chords: [0, 2, 3, 1, 0, 2, 4, 1], cutoff: 4200,
    lead:
      "a-------8---7---" + "8-------5-------" + "7---8---a---c---" + "a---------------" +
      "8-------7---5---" + "6-------4-------" + "5---6---7---5---" + "3---------------",
    bass: "x...............",
    keys: "x.x.x.x.x.x.x.x.",
    drums: { shaker: "........x......." },
    mix: { lead: 0.07, pad: 0.04, bass: 0.1, keys: 0.018 },
  },
  // Grove: dorian, a walking tune among roots.
  grove: {
    bpm: 92, root: 65, scale: "dorian", chords: [0, 6, 3, 4, 0, 6, 1, 4],
    lead:
      "3-5-7-5-6---5-3-" + "4-6-8-6-7-------" + "5-7-8-a-9-8-7-5-" + "6-------4---2---" +
      "3-5-7-5-6---8-7-" + "6-5-4-3-4-------" + "5-4-3-2-3-4-5-6-" + "3---------------",
    bass: "x...5...x...5...",
    keys: "..x...x...x...x.",
    drums: { kick: "x.......x.......", shaker: "x.xxx.xxx.xxx.xx" },
    mix: { lead: 0.075, pad: 0.025, bass: 0.13, keys: 0.022 },
  },
  // Stone: low minor, heavy steps.
  stone: {
    bpm: 84, root: 57, scale: "minor", chords: [0, 0, 5, 4, 0, 3, 5, 4],
    lead:
      "3---3-5-6---5-3-" + "2---------------" + "3---3-5-7---6-5-" + "6-------5-------" +
      "7---7-6-5---6-7-" + "8-------7-------" + "6---5-3-2---3-2-" + "0-------3-------",
    bass: "x.......x...5...",
    drums: { kick: "x.......x..x....", snare: "........x......." },
    mix: { lead: 0.085, pad: 0.03, bass: 0.15, keys: 0.02 },
  },
  // Caves (a `dark` map): a slow minor drone, filtered, a drip for a beat.
  cave: {
    bpm: 66, root: 60, scale: "minor", chords: [0, 5, 3, 4, 0, 5, 6, 4], cutoff: 1100,
    lead:
      "7-------6-------" + "5-------3-------" + "4-------5-------" + "3---------------" +
      "a-------9-------" + "7-------6-------" + "5-------4-------" + "3---------------",
    bass: "x...............",
    drums: { shaker: "......x........." },
    mix: { lead: 0.06, pad: 0.04, bass: 0.1, keys: 0.02 },
  },
});
// Indoors: the meadow tune, slower, softer and filtered, with no drums.
TRACKS.interior = {
  ...TRACKS.overworld, bpm: 80, cutoff: 1500,
  drums: { shaker: "....x.......x..." },
  mix: { lead: 0.06, pad: 0.03, bass: 0.1, keys: 0.03 },
};
// Duels by stakes. "duel" (above) is the ordinary one: route duelists, quick duels.
// A rival's theme is one tune played in their colour's region: its key, scale (a
// pentatonic colour sings it differently), pace and keys. The Warden's is its own.
const HOT = { shaker: "xxxxxxxxxxxxxxxx", arp: "x.x.x.x.x.x.x.x." };
const RIVAL = {
  chords: [0, 5, 3, 4, 0, 5, 6, 4],
  lead:
    "3-5-7-.7-8-7-5-." + "6---5-3-5-------" + "3-5-7-.7-a-9-7-." + "8-------7---5---" +
    "a-.a-9-7-8-.8-7-" + "6---7-8-7-------" + "5-6-7-8-9-8-7-6-" + "7-------3---.3..",
  bass: "x.x.xx.x.x.x5.o.",
  drums: { kick: "x...x...x...x...", snare: "....x.......x.x.", hat: "..x...x...x...x." },
  hot: HOT,
  mix: { lead: 0.08, pad: 0.028, bass: 0.14, keys: 0.022 },
};
for (const [k, root, bpm] of [["ember", 62, 120], ["storm", 64, 124], ["tide", 67, 108], ["dream", 69, 104], ["grove", 65, 114], ["stone", 59, 110]])
  TRACKS[`duel_rival_${k}`] = { ...RIVAL, root, bpm, scale: TRACKS[k].scale, keys: TRACKS[k].keys };
TRACKS.duel_boss = {
  bpm: 126, root: 62, scale: "harmonic", chords: [0, 0, 5, 4, 0, 3, 5, 4],
  lead:
    "3---4-5-6-5-4-3-" + "2-------3-------" + "3---4-5-7-6-5-4-" + "5-------6---7---" +
    "a---9-8-7---8-9-" + "a-------9-------" + "8-7-6-5-6-5-4-2-" + "3-------.3.3.3..",
  bass: "x.x.x.x.x.x.x5o.",
  keys: "x..x..x...x..x..",
  drums: { kick: "x...x...x...x...", snare: "....x.......x...", hat: "x.x.x.x.x.x.x.x." },
  hot: HOT,
  mix: { lead: 0.08, pad: 0.03, bass: 0.15, keys: 0.025 },
};
// After dark, every outdoor track has a night variant made the same way: slower,
// filtered, half the drums (only the kick and a soft shaker), the lead held back.
for (const k of ["overworld", "ember", "storm", "tide", "dream", "grove", "stone"]) {
  const t = TRACKS[k], d = t.drums;
  TRACKS[`${k}_night`] = {
    ...t, bpm: Math.round(t.bpm * 0.88), cutoff: Math.min(t.cutoff ?? 9000, 1700),
    drums: { ...(d.kick && { kick: d.kick.replace(/(?<=x.*)x/g, ".") }), shaker: "....x.......x..." },
    keys: undefined,
    mix: { ...t.mix, lead: t.mix.lead * 0.75, pad: t.mix.pad * 1.3 },
  };
}

// Precompute every step's notes once, so the scheduler only reads arrays.
function compile(def) {
  const sc = SCALES[def.scale], n7 = sc.length;
  const deg = (d) => def.root + sc[((d % n7) + n7) % n7] + 12 * Math.floor(d / n7);
  const n = def.chords.length * 16;
  const ev = Array.from({ length: n }, () => []), hot = Array.from({ length: n }, () => []);
  const held = (s, i) => {
    let k = 1;
    while (s[i + k] === "-") k++;
    return k;
  };
  def.chords.forEach((c, bar) => {
    const chord = [0, 2, 4, 6].map((i) => hz(deg(c + i) - 12));
    const at = (i) => ev[bar * 16 + i];
    at(0).push(["pad", chord, 16]);
    for (let i = 0; i < 16; i++) {
      const b = { x: 0, 5: 4, o: 7 }[def.bass[i]];
      if (b !== undefined) at(i).push(["bass", hz(deg(c + b) - 24), held(def.bass, i)]);
      if (def.keys?.[i] === "x") at(i).push(["keys", chord, 1]);
      for (const [k, p] of Object.entries(def.drums)) if (p[i] === "x") at(i).push([k, 0, 1]);
      for (const [k, p] of Object.entries(def.hot ?? {}))
        if (p[i] === "x") hot[bar * 16 + i].push(k === "arp" ? ["arp", hz(deg(c + 2 * ((i >> 1) % 4))), 1] : [k, 0, 1]);
    }
  });
  for (let i = 0; i < n; i++) {
    const d = DIGITS.indexOf(def.lead[i]);
    if (d >= 0) ev[i].push(["lead", hz(deg(d - 3)), held(def.lead, i)]);
  }
  return { ...def, ev, hot: def.hot ? hot : null };
}
for (const k in TRACKS) TRACKS[k] = compile(TRACKS[k]);

const VOICES = {
  pad: (d, t, f, len, m) => {
    for (const x of f) {
      tone(d, t, x, len, { type: "triangle", peak: m.pad, a: 0.4, lin: true, rel: 0.6 });
      tone(d, t, x, len, { peak: m.pad * 0.7, a: 0.5, detune: 7, lin: true, rel: 0.6 });
    }
  },
  lead: (d, t, f, len, m) => {
    tone(d, t, f, len, { type: "triangle", peak: m.lead, a: 0.015, lin: true, rel: 0.12 });
    tone(d, t, f * 2, len * 0.6, { peak: m.lead * 0.15 });
  },
  bass: (d, t, f, len, m) => tone(d, t, f, Math.max(len * 0.9, 0.12), { type: "triangle", peak: m.bass, a: 0.008, lin: true, rel: 0.06 }),
  keys: (d, t, f, len, m) => {
    for (const x of f) tone(d, t, x * 2, 0.25, { peak: m.keys });
  },
  arp: (d, t, f) => tone(d, t, f, 0.12, { type: "triangle", peak: 0.03 }),
  kick: (d, t) => tone(d, t, 120, 0.16, { f2: 42, glide: 0.1, peak: 0.3 }),
  snare: (d, t) => {
    noise(d, t, 0.12, { f: 1800, q: 0.7, peak: 0.1 });
    tone(d, t, 190, 0.06, { peak: 0.06 });
  },
  hat: (d, t) => noise(d, t, 0.035, { type: "highpass", f: 7500, peak: 0.05 }),
  shaker: (d, t) => noise(d, t, 0.05, { type: "highpass", f: 5500, a: 0.015, peak: 0.025 }),
};

function startPlayer(name, t) {
  const def = TRACKS[name], out = ctx.createGain(), flt = ctx.createBiquadFilter();
  flt.type = "lowpass";
  flt.frequency.value = def.cutoff ?? 9000;
  out.gain.setValueAtTime(0, t);
  out.gain.setTargetAtTime(1, t, 0.5);
  out.connect(flt).connect(musicBus);
  const p = { name, def, out, flt, step: 0, next: t + 0.1, endAt: 0 };
  players.push(p);
  return p;
}

let current = null, duelTrack = "duel";

/// The theme "duel" plays from now on ("duel", "duel_rival_<region>", "duel_boss"); set as each duel starts.
export function duelTheme(name) {
  duelTrack = TRACKS[name] ? name : "duel";
}

/// Crossfade to a track ("title", "duel", "overworld", "interior", a region's, "<track>_night"), or to silence ("").
export function music(name) {
  if (name === "duel") name = duelTrack;
  if (!ctx) return void (want = name);
  if ((current?.name ?? "") === name) return;
  const t = ctx.currentTime;
  if (current) {
    current.out.gain.setTargetAtTime(0, t, 0.4);
    current.endAt = t + 2.5;
  }
  current = TRACKS[name] ? startPlayer(name, t) : null;
  if (name.startsWith("duel")) intense = false;
}

/// 1: the duel is down to someone's last prize (busier drums, an arpeggio, a touch faster).
export function intensity(level) {
  intense = level > 0;
}

const LOOKAHEAD = 0.15;
function tick() {
  const now = ctx.currentTime;
  tickBeds(now);
  for (let i = players.length - 1; i >= 0; i--) {
    const p = players[i];
    if (p.endAt && now > p.endAt) {
      p.flt.disconnect();
      players.splice(i, 1);
      continue;
    }
    const hot = intense && p.def.hot;
    const spb = 60 / (p.def.bpm * (hot ? 1.05 : 1)) / 4;
    if (p.next < now - 0.3) p.next = now + 0.05; // the timer stalled: rejoin the clock, don't burst
    while (p.next < now + LOOKAHEAD) {
      if (!p.endAt) {
        for (const e of p.def.ev[p.step]) VOICES[e[0]](p.out, p.next, e[1], e[2] * spb, p.def.mix);
        if (hot) for (const e of p.def.hot[p.step]) VOICES[e[0]](p.out, p.next, e[1], e[2] * spb, p.def.mix);
      }
      p.step = (p.step + 1) % p.def.ev.length;
      p.next += spb;
    }
  }
}

/// What is playing, for debugging from the console: `sixfoldAudio.debug()`.
function debug() {
  return {
    state: ctx?.state ?? "none",
    tracks: players.map((p) => p.name + (p.endAt ? " (fading)" : "")),
    beds: beds.map((b) => b.key + (b.endAt ? " (fading)" : "")),
    intense,
    prefs: { ...prefs },
    levels: levels(),
    ...stats,
  };
}

// RMS of the last ~40 ms per group, in dBFS (-inf is silence): each group after its slider, `out` after the master.
function levels() {
  // Tapped on first use only: each group after its slider, and what leaves the speakers.
  if (!meters && ctx) meters = Object.fromEntries(Object.entries(taps).map(([k, n]) => [k, n.connect(ctx.createAnalyser())]));
  const buf = new Float32Array(2048);
  return Object.fromEntries(
    Object.entries(meters ?? {}).map(([k, a]) => {
      a.getFloatTimeDomainData(buf);
      const rms = Math.sqrt(buf.reduce((s, x) => s + x * x, 0) / buf.length);
      return [k, Math.round(20 * Math.log10(rms) * 10) / 10];
    }),
  );
}

globalThis.sixfoldAudio = { sfx, music, ambience, intensity, debug };
