//! The credits, rolled once the Hex Warden falls (and on demand from the
//! campaign screen, which only a champion has). They open on the finale (#67):
//! the seal stone lighting its six faces, then the champion's title, so a
//! champion can see those again from the campaign. The credits are
//! plates shown one at a time, each for a while, then the next fades in; with
//! reduced motion they swap without a fade. Next (or →) moves on, ← goes back,
//! Skip (or Escape) ends them. After the Warden they end in the duel hall, by
//! the sealed door; from the campaign screen they go back there.

use std::time::Duration;

use leptos::prelude::*;
use sixfold_engine::campaign::{self, Rival, RIVALS};
use sixfold_engine::duelists::duelist;
use sixfold_engine::overworld::Progress;
use sixfold_engine::stats::{Stats, FINDS};
use sixfold_engine::{card, tr, T};

use crate::achievements::thousands;
use crate::art::{art_src, color_key};
use crate::audio::MuteToggle;
use crate::logo::Wordmark;
use crate::{store, Screen};

/// Where the champion stands once the credits end: the duel hall, just out of
/// the Warden's door (the way back's spawn in `web/world/maps/warden.json`;
/// `engine/tests/world_data.rs` fails if it moves).
const HALL_DOOR: &str = "shop,1,6,e";

/// The people of the world, as the credits list them.
const TOWNSFOLK: &[(&str, T)] = &[
    (
        "Wynne",
        T::new("keeps the duel hall", "veille sur la salle des duels"),
    ),
    (
        "Wren",
        T::new("walks the meadow loop", "fait le tour du pré"),
    ),
    (
        "Moss",
        T::new("tends the flowerbeds", "soigne les massifs de fleurs"),
    ),
    (
        "Pip",
        T::new("practises the shuffle", "s'entraîne à mélanger"),
    ),
    (
        "Hob",
        T::new(
            "bakes in Brightwell Square",
            "fait le pain sur la Place de Clairpuits",
        ),
    ),
    (
        "Tansy",
        T::new(
            "sells apples and one proud marrow",
            "vend des pommes et une courge, sa grande fierté",
        ),
    ),
    ("Fennel", T::new("watches the herons", "observe les hérons")),
    ("Ottilie", T::new("keeps the lighthouse", "garde le phare")),
    ("Dace", T::new("waits for a bite", "attend que ça morde")),
    (
        "Tarn",
        T::new("cuts stone in the quarry", "taille la pierre à la carrière"),
    ),
];

/// What the credits list under "Made with": (name, what it did).
const TOOLS: &[(&str, T)] = &[
    (
        "Rust & WebAssembly",
        T::new(
            "the rules engine",
            "le moteur de règles",
        ),
    ),
    ("Leptos", T::new("every screen", "tous les écrans")),
    ("three.js", T::new("the world", "le monde")),
    (
        "Blender",
        T::new(
            "every prop, character and portrait, modelled by script",
            "tous les objets, personnages et portraits, modelés par script",
        ),
    ),
    (
        "FLUX 3 Image Klein, with mflux",
        T::new(
            "the card pictures, generated on our own machine from hand-written captions",
            "les illustrations des cartes, générées sur notre machine d'après des légendes écrites à la main",
        ),
    ),
    (
        "Web Audio",
        T::new(
            "every note and every sound, played live, never recorded",
            "chaque note et chaque son, joués en direct, jamais enregistrés",
        ),
    ),
    ("Trunk", T::new("the build", "la compilation")),
    ("Playwright", T::new("the browser tests", "les tests dans le navigateur")),
    (
        "Claude Code",
        T::new(
            "a coding companion, from Anthropic",
            "un compagnon de code, par Anthropic",
        ),
    ),
];

/// What ships in the game under someone else's licence: (what, the licence).
/// Checked against `web/vendor/three/LICENSE`, `web/index.html` (the fonts)
/// and `cargo tree -p sixfold-web --target wasm32-unknown-unknown -e normal`.
const LICENCES: &[(T, T)] = &[
    (T::new("three.js", "three.js"), T::new("MIT", "MIT")),
    (T::new("Leptos", "Leptos"), T::new("MIT", "MIT")),
    (
        T::new("wasm-bindgen, js-sys, web-sys", "wasm-bindgen, js-sys, web-sys"),
        T::new("MIT or Apache-2.0", "MIT ou Apache-2.0"),
    ),
    (
        T::new(
            "The other Rust crates in the WebAssembly build",
            "Les autres crates Rust de la version WebAssembly",
        ),
        T::new(
            "MIT, Apache-2.0, Zlib, BSL-1.0, Unicode-3.0 or Unlicense",
            "MIT, Apache-2.0, Zlib, BSL-1.0, Unicode-3.0 ou Unlicense",
        ),
    ),
    (
        T::new(
            "Bricolage Grotesque, Atkinson Hyperlegible",
            "Bricolage Grotesque, Atkinson Hyperlegible",
        ),
        T::new("SIL Open Font License 1.1", "SIL Open Font License 1.1"),
    ),
    (
        T::new("FLUX 3 Image Klein", "FLUX 3 Image Klein"),
        T::new(
            "the Black Forest Labs community licence, which allows its pictures in this free game",
            "la licence communautaire de Black Forest Labs, qui permet ses images dans ce jeu gratuit",
        ),
    ),
];

/// A rival's colour class (`t-fire` …), from their first colour.
fn tone(r: &Rival) -> &'static str {
    r.colors.first().map_or("colorless", |&c| color_key(c))
}

/// "a, b, c and d" (or "et").
fn and_list(parts: Vec<AnyView>) -> Vec<AnyView> {
    let n = parts.len();
    let mut out = vec![];
    for (i, p) in parts.into_iter().enumerate() {
        if i > 0 {
            out.push(
                if i + 1 == n {
                    tr(" and ", " et ")
                } else {
                    ", "
                }
                .into_any(),
            );
        }
        out.push(p);
    }
    out
}

/// The run so far, in one sentence: the stats that aren't zero (a save from
/// before 1.7 may have only its duels won). Read only.
fn run_line(s: &Stats) -> impl IntoView {
    let n = |k: &str| s.get(k);
    let facts: Vec<(u64, String, String)> = vec![
        (n("wins"), " duels won".into(), " duels gagnés".into()),
        (n("kos"), " knock-outs".into(), " K.O.".into()),
        (n("steps"), " steps walked".into(), " pas".into()),
        (
            n("finds"),
            format!(" of {FINDS} finds"),
            format!(" trouvailles sur {FINDS}"),
        ),
        (
            n("packs"),
            " packs opened".into(),
            " boosters ouverts".into(),
        ),
    ];
    // A champion has won at least once; "0 steps walked" is no boast.
    let nonzero = facts.iter().filter(|f| f.0 > 0).count();
    let facts: Vec<AnyView> = facts
        .into_iter()
        .filter(|f| f.0 > 0 || nonzero == 0)
        .map(|(v, en, fr)| view! { <b>{thousands(v)}</b>{tr(en, fr)} }.into_any())
        .collect();
    view! { <p class="credits-run">{and_list(facts)}"."</p> }
}

/// When the seal stone's faces light (ms after its plate shows), one per
/// seal, and when the whole stone rings.
const FACE_AT: u32 = 500;
const FACE_GAP: u32 = 450;
const RING_AT: u32 = FACE_AT + 6 * FACE_GAP + 150;

/// The seal stone: a hexagon of six faces, one per seal in the rivals' order
/// and colours, around the seventh, smooth and blank.
fn seal_stone() -> impl IntoView {
    // Pointy-top hexagon, radius 100 round (0, 0): vertex k at -90° + 60°·k.
    let v = |k: usize, r: f64| {
        let a = (-90.0 + 60.0 * k as f64).to_radians();
        (r * a.cos(), r * a.sin())
    };
    let hex = |r: f64| {
        (0..6)
            .map(|k| {
                let (x, y) = v(k, r);
                format!("{x:.1},{y:.1}")
            })
            .collect::<Vec<_>>()
            .join(" ")
    };
    let faces = RIVALS.iter().filter(|r| r.seal).enumerate().map(|(k, r)| {
        let tone = tone(r);
        let ((x0, y0), (x1, y1), (x2, y2), (x3, y3)) = (v(k, 34.0), v(k, 100.0), v(k + 1, 100.0), v(k + 1, 34.0));
        view! {
            <path class=format!("stone-face t-{tone}") style=format!("--i: {k}")
                d=format!("M{x0:.1},{y0:.1} L{x1:.1},{y1:.1} L{x2:.1},{y2:.1} L{x3:.1},{y3:.1} Z") />
        }
    });
    view! {
        // The CSS lights each face when its bell rings: one set of timings, from here.
        <svg class="stone" viewBox="-112 -112 224 224" aria-hidden="true"
            style=format!("--face-at: {FACE_AT}ms; --face-gap: {FACE_GAP}ms; --ring-at: {RING_AT}ms")>
            <defs>
                <radialGradient id="stone-sheen" cx="35%" cy="25%" r="80%">
                    <stop offset="0" stop-color="#fff" stop-opacity="0.32" />
                    <stop offset="0.55" stop-color="#fff" stop-opacity="0" />
                </radialGradient>
                <linearGradient id="stone-shade" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0.45" stop-color="#000" stop-opacity="0" />
                    <stop offset="1" stop-color="#000" stop-opacity="0.32" />
                </linearGradient>
            </defs>
            <polygon class="stone-ring" points=hex(104.0) />
            {faces.collect_view()}
            <polygon class="stone-seventh" points=hex(34.0) />
            <polygon class="stone-sheen" points=hex(100.0) fill="url(#stone-sheen)" />
            <polygon class="stone-sheen" points=hex(100.0) fill="url(#stone-shade)" />
            <polygon class="stone-rim" points=hex(100.0) />
        </svg>
    }
}

/// Run `f` in `ms`, unless the credits moved on meanwhile (`armed` changed).
fn later(armed: StoredValue<u32>, ms: u32, f: impl FnOnce() + 'static) {
    let t = armed.get_value();
    set_timeout(
        move || {
            if armed.try_get_value() == Some(t) {
                f()
            }
        },
        Duration::from_millis(u64::from(ms)),
    );
}

#[component]
pub fn Credits(won: bool) -> impl IntoView {
    let progress = expect_context::<RwSignal<Progress>>();
    let stats = expect_context::<RwSignal<Stats>>();
    // After the Warden: back into the world, in the hall (the save written now).
    // Saved now too, so a reload during the credits comes back in the hall, not
    // in the Warden's room with the scene spent (the stone and the title stay
    // on the campaign screen's "Watch the credits").
    if won {
        store::set_world_at(HALL_DOOR);
    }
    let done = move || {
        if won {
            store::set_world_at(HALL_DOOR);
            crate::go(Screen::World);
        } else {
            crate::go(Screen::Campaign);
        }
    };
    // With reduced motion the plates swap without a fade and the stone is lit at once.
    let still = web_sys::window()
        .and_then(|w| {
            w.match_media("(prefers-reduced-motion: reduce)")
                .ok()
                .flatten()
        })
        .is_some_and(|m| m.matches());

    // "Champion", or "Champion of the Seventh Face" once Rook's story is done.
    let title = progress
        .with_untracked(campaign::title)
        .unwrap_or(T::new("Champion", "Champion"));
    let seal_rivals = || RIVALS.iter().filter(|r| r.seal);
    let primary = NodeRef::<leptos::html::Button>::new();
    // The plates, in order: (how long each shows in ms, its name for the
    // status line, what it shows). The last one stays until Continue.
    let mut plates: Vec<(u32, String, AnyView)> = vec![];
    let lit = tr("Six faces, all lit.", "Six faces, toutes allumées.");
    // The peak: the stone stays lit a few seconds once the ring has bloomed.
    plates.push((
        RING_AT + 4200,
        lit.into(),
        view! {
            {seal_stone()}
            // It waits for the ring (its own copy of the timing: `--ring-at` is set on the stone).
            <p class="credits-line" style=format!("--ring-at: {RING_AT}ms")>{lit}</p>
        }
        .into_any(),
    ));
    plates.push((
        4600,
        title.get().into(),
        view! {
            <h2 class="wordmark champion-title" class:long={title.en.len() > 10}>{title.get()}</h2>
            <p class="credits-line">{tr("Six seals and the Warden's chair.", "Six sceaux et le fauteuil de la Gardienne.")}</p>
        }
        .into_any(),
    ));
    plates.push((
        5000,
        "Sixfold".into(),
        view! {
            <h1 class="wordmark"><Wordmark /></h1>
            <p class="credits-line">{tr("The people and the tools behind it.", "Ceux qui l'ont fait, et avec quoi.")}</p>
        }
        .into_any(),
    ));
    let heading = |en: &'static str, fr: &'static str| tr(en, fr).to_string();
    plates.push((
        7000,
        heading("Your run", "Votre partie"),
        view! {
            <h2>{tr("Your run", "Votre partie")}</h2>
            <ul class="seals credits-seals" aria-hidden="true">
                {seal_rivals().map(|r| view! {
                    <li><crate::campaign::Seal r=r earned=progress.with_untracked(|p| r.beaten(p)) /></li>
                }).collect_view()}
            </ul>
            {stats.with_untracked(run_line)}
        }
        .into_any(),
    ));
    // A portrait in its colour's ring: (portrait of, name, title, colour class).
    let face = |id: &str, name: &'static str, title: T, tone: &str| {
        view! {
            <li class=format!("cast-one t-{tone}")>
                {crate::campaign::portrait_url(id).map(|src| view! { <img src=src alt="" width="256" height="256" decoding="async" /> })}
                <b>{name}</b>
                <span>{title.get()}</span>
            </li>
        }
    };
    let rival = move |r: &'static Rival| face(r.id, r.name, r.title, tone(r));
    plates.push((
        9000,
        heading("The rivals", "Les rivaux"),
        view! {
            <h2>{tr("The rivals", "Les rivaux")}</h2>
            <ul class="cast">{seal_rivals().map(rival).collect_view()}</ul>
            // Rook (no colour, no seal) beside the Warden.
            <ul class="cast last">
                {duelist("rook_4").map(|t| face(t.id, t.name, t.class, "colorless"))}
                {RIVALS.iter().filter(|r| !r.seal).map(rival).collect_view()}
            </ul>
        }
        .into_any(),
    ));
    plates.push((
        7000,
        heading("The Hexons", "Les Hexons"),
        view! {
            <h2>{tr("The Hexons", "Les Hexons")}</h2>
            <ul class="hexons">
                {RIVALS.iter().enumerate().map(|(i, r)| {
                    let c = card(r.promo());
                    view! {
                        <li style=format!("--i: {i}")>
                            {art_src(c).map(|src| view! { <img src=src alt="" width="800" height="560" decoding="async" /> })}
                            <span>{c.label()}</span>
                        </li>
                    }
                }).collect_view()}
            </ul>
            <p class="credits-line">{tr(
                "Seventy-eight Hexons, each one invented for this game. These seven are the rivals' own.",
                "Soixante-dix-huit Hexons, tous inventés pour ce jeu. Ces sept-là appartiennent aux rivaux.",
            )}</p>
        }
        .into_any(),
    ));
    plates.push((
        10000,
        heading("The townsfolk", "Les gens du coin"),
        view! {
            <h2>{tr("The townsfolk", "Les gens du coin")}</h2>
            <ul class="credits-list">
                {TOWNSFOLK.iter().map(|(name, what)| view! {
                    <li><b>{*name}</b><span>{what.get()}</span></li>
                }).collect_view()}
            </ul>
        }
        .into_any(),
    ));
    plates.push((
        6000,
        heading("A game by Atypical Play", "Un jeu Atypical Play"),
        view! {
            <h2>{tr("A game by Atypical Play", "Un jeu Atypical Play")}</h2>
            <ul class="credits-list solo">
                <li><b>"Philippe Matray"</b><span>{tr("direction, game design, code, writing and cards", "direction, conception, code, écriture et cartes")}</span></li>
            </ul>
        }
        .into_any(),
    ));
    plates.push((
        13000,
        heading("Made with", "Fait avec"),
        view! {
            <h2>{tr("Made with", "Fait avec")}</h2>
            <ul class="credits-list">
                {TOOLS.iter().map(|(name, what)| view! {
                    <li><b>{*name}</b><span>{what.get()}</span></li>
                }).collect_view()}
            </ul>
        }
        .into_any(),
    ));
    plates.push((
        14000,
        heading("Licences", "Licences"),
        view! {
            <h2>{tr("Licences", "Licences")}</h2>
            <ul class="credits-list licences">
                {LICENCES.iter().map(|(name, what)| view! {
                    <li><b>{name.get()}</b><span>{what.get()}</span></li>
                }).collect_view()}
            </ul>
            <p class="credits-line small">{tr(
                "Sixfold's own code is MIT too. Every Hexon, card, place, picture and sound here is original; only the rules come from a classic handheld card game.",
                "Le code de Sixfold est lui aussi sous licence MIT. Chaque Hexon, carte, lieu, image et son de ce jeu est original\u{202f}; seules les règles viennent d'un grand classique du jeu de cartes sur console portable.",
            )}</p>
        }
        .into_any(),
    ));
    let thanks = tr("Thank you for playing.", "Merci d'avoir joué.");
    plates.push((
        0,
        thanks.into(),
        view! {
            <p class="credits-end">{thanks}</p>
            <button type="button" class="btn primary credits-continue" node_ref=primary on:click=move |_| done()>
                {tr("Continue", "Continuer")}
            </button>
        }
        .into_any(),
    ));
    let durations: Vec<u32> = plates.iter().map(|p| p.0).collect();
    let names: Vec<String> = plates.iter().map(|p| p.1.clone()).collect();
    let n = plates.len();

    // The plate on show, and whether the plates wait for Next (Pause; from the
    // start with reduced motion, read at the reader's pace). Each change bumps
    // `armed`, so a timer armed before a Next, Back, Pause or Skip does nothing.
    let at = RwSignal::new(0usize);
    let paused = RwSignal::new(still);
    let armed = StoredValue::new(0u32);
    // The same plate again (Back on the first) changes nothing: no bells rung twice.
    let show = move |i: usize| {
        let i = i.min(n - 1);
        if i != at.get_untracked() {
            armed.update_value(|t| *t += 1);
            at.set(i);
        }
    };
    // A memo: the foot's buttons are rebuilt only when it flips, not on every plate.
    let last = Memo::new(move |_| at.get() + 1 == n);
    Effect::new(move |_| {
        let i = at.get();
        // The stone rings a bell per face as it lights, then all at once.
        if i == 0 {
            if !still {
                for k in 0..6 {
                    later(armed, FACE_AT + k * FACE_GAP, move || {
                        crate::audio::sfx("face", f64::from(k))
                    });
                }
            }
            later(armed, if still { 200 } else { RING_AT }, || {
                crate::audio::sfx("face", 6.0)
            });
        }
        // The last plate: Continue takes the focus (Next and Skip are gone).
        if i + 1 == n {
            request_animation_frame(move || {
                if let Some(b) = primary.get_untracked() {
                    _ = b.focus();
                }
            });
        }
    });
    Effect::new(move |_| {
        let i = at.get();
        if paused.get() {
            return;
        }
        if let Some(&ms) = durations.get(i).filter(|&&ms| ms > 0 && i + 1 < n) {
            later(armed, ms, move || show(i + 1));
        }
    });
    let pause = move |_| {
        armed.update_value(|t| *t += 1);
        paused.update(|p| *p = !*p);
    };
    // On the window, so the keys (and a pad's, which gamepad.js sends as keys)
    // still work once a click on a plate has taken the focus off the buttons.
    let keys = window_event_listener(leptos::ev::keydown, move |e| match e.key().as_str() {
        "Escape" => {
            e.prevent_default();
            done();
        }
        "ArrowRight" if !last.get_untracked() => show(at.get_untracked() + 1),
        "ArrowLeft" => show(at.get_untracked().saturating_sub(1)),
        _ => {}
    });
    on_cleanup(move || keys.remove());
    let status = move || {
        let i = at.get();
        tr(
            format!("{} of {n}: {}", i + 1, names[i]),
            format!("{} sur {n}\u{a0}: {}", i + 1, names[i]),
        )
    };
    view! {
        <main class="credits" class:still=still aria-label=tr("Credits", "Générique")>
            <ol class="credits-pips" aria-hidden="true">
                {(0..n).map(|i| view! { <li class:on=move || at.get() == i class:past=move || { at.get() > i }></li> }).collect_view()}
            </ol>
            <p class="sr-only" role="status">{status}</p>
            <div class="credits-stage">
                {plates.into_iter().enumerate().map(|(i, (_, _, v))| view! {
                    <div class="credits-plate" class:on=move || at.get() == i
                        aria-hidden=move || (at.get() != i).to_string() inert=move || at.get() != i>
                        {v}
                    </div>
                }).collect_view()}
            </div>
            <div class="credits-foot">
                <MuteToggle />
                {move || (!last.get()).then(|| view! {
                    <button type="button" class="btn ghost icon" aria-pressed=move || paused.get().to_string()
                        aria-label=tr("Pause the credits", "Mettre le générique en pause") on:click=pause>
                        <svg viewBox="0 0 24 24" aria-hidden="true">
                            {move || if paused.get() {
                                view! { <path d="M8 5v14l11-7z" /> }.into_any()
                            } else {
                                view! { <path d="M7 5h3v14H7zM14 5h3v14h-3z" /> }.into_any()
                            }}
                        </svg>
                    </button>
                })}
                <button type="button" class="btn ghost icon" aria-label=tr("Back", "Retour")
                    aria-disabled=move || (at.get() == 0).to_string() on:click=move |_| show(at.get_untracked().saturating_sub(1))>
                    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M15 5l-7 7 7 7" /></svg>
                </button>
                {move || (!last.get()).then(|| view! {
                    <button type="button" class="btn ghost icon" aria-label=tr("Next", "Suivant")
                        on:click=move |_| show(at.get_untracked() + 1)>
                        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 5l7 7-7 7" /></svg>
                    </button>
                    // Focus starts on Skip (see `focus_heading`), so Enter works straight away.
                    <button type="button" class="btn ghost" data-screen-focus aria-keyshortcuts="Escape" on:click=move |_| done()>
                        {tr("Skip the credits", "Passer le générique")}
                        <kbd aria-hidden="true">{move || crate::input::prompt(crate::input::Ctrl::Cancel)}</kbd>
                    </button>
                })}
            </div>
        </main>
    }
}
