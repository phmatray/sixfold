//! The credits, rolled once the Hex Warden falls (and on demand from the
//! campaign screen). After the Warden they open on the finale (#67): the seal
//! stone lighting its six faces, then the champion's title. The credits are
//! plates shown one at a time, each for a while, then the next fades in; with
//! reduced motion they swap without a fade. Next (or →) moves on, ← goes back,
//! Skip (or Escape) ends them. After the Warden they end in the duel hall, by
//! the sealed door; from the campaign screen they go back there.

use std::time::Duration;

use leptos::prelude::*;
use sixfold_engine::campaign::{Rival, RIVALS};
use sixfold_engine::overworld::Progress;
use sixfold_engine::stats::{Stats, FINDS};
use sixfold_engine::{card, tr, T};

use crate::achievements::thousands;
use crate::art::{art_src, color_key};
use crate::audio::MuteToggle;
use crate::{store, Screen};

/// Where the champion stands once the credits end: the duel hall, just out of
/// the Warden's door (`web/world/maps/shop.json`, the sealed door's spawn).
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

/// The champion's title: "Champion", or once Rook's story is done
/// (`flag:rook_4`, WORLD_DESIGN §8.2) "Champion of the Seventh Face".
// ponytail: mirrors `campaign::title` from #64; use it once that lands.
fn champion_title(p: &Progress) -> T {
    if p.has_flag("rook_4") {
        T::new(
            "Champion of the Seventh Face",
            "Champion de la septième face",
        )
    } else {
        T::new("Champion", "Champion")
    }
}

/// What the credits list under "Made with": (name, what it did).
const TOOLS: &[(&str, T)] = &[
    (
        "Rust & WebAssembly",
        T::new(
            "the rules engine and every screen",
            "le moteur de règles et tous les écrans",
        ),
    ),
    ("Leptos", T::new("the screens", "les écrans")),
    ("three.js", T::new("the world", "le monde")),
    (
        "Blender",
        T::new(
            "every prop, character and portrait, modelled by script",
            "chaque objet, personnage et portrait, modelés par script",
        ),
    ),
    (
        "FLUX 3 Image Klein",
        T::new(
            "the card pictures, drawn on our own machine from hand-written captions",
            "les illustrations des cartes, dessinées sur notre machine d'après des légendes écrites à la main",
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

/// The run so far, in one sentence: up to five of the stats, the ones that
/// aren't zero. Read only; None before anything happened.
fn run_line(s: &Stats) -> Option<impl IntoView> {
    let n = |k: &str| s.get(k);
    let fact = |v: u64, en: String, fr: String| {
        (v > 0).then(|| view! { <b>{thousands(v)}</b>{tr(en, fr)} }.into_any())
    };
    let facts: Vec<AnyView> = [
        fact(n("wins"), " duels won".into(), " duels gagnés".into()),
        fact(n("kos"), " knock-outs".into(), " K.O.".into()),
        fact(n("steps"), " steps walked".into(), " pas".into()),
        fact(
            n("finds"),
            format!(" of {FINDS} finds"),
            format!(" trouvailles sur {FINDS}"),
        ),
        fact(
            n("packs"),
            " packs opened".into(),
            " boosters ouverts".into(),
        ),
    ]
    .into_iter()
    .flatten()
    .collect();
    (!facts.is_empty()).then(|| view! { <p class="credits-run">{and_list(facts)}"."</p> })
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
        let tone = r.colors.first().map_or("colorless", |&c| color_key(c));
        let ((x0, y0), (x1, y1), (x2, y2), (x3, y3)) = (v(k, 34.0), v(k, 100.0), v(k + 1, 100.0), v(k + 1, 34.0));
        view! {
            <path class=format!("stone-face t-{tone}") style=format!("--i: {k}")
                d=format!("M{x0:.1},{y0:.1} L{x1:.1},{y1:.1} L{x2:.1},{y2:.1} L{x3:.1},{y3:.1} Z") />
        }
    });
    view! {
        <svg class="stone" viewBox="-112 -112 224 224" aria-hidden="true">
            <defs>
                <radialGradient id="stone-sheen" cx="35%" cy="25%" r="80%">
                    <stop offset="0" stop-color="#fff" stop-opacity="0.32" />
                    <stop offset="0.55" stop-color="#fff" stop-opacity="0" />
                </radialGradient>
            </defs>
            <polygon class="stone-ring" points=hex(104.0) />
            {faces.collect_view()}
            <polygon class="stone-seventh" points=hex(34.0) />
            <polygon class="stone-sheen" points=hex(100.0) fill="url(#stone-sheen)" />
        </svg>
    }
}

#[component]
pub fn Credits(won: bool) -> impl IntoView {
    let screen = expect_context::<RwSignal<Screen>>();
    let progress = expect_context::<RwSignal<Progress>>();
    let stats = expect_context::<RwSignal<Stats>>();
    // After the Warden: back into the world, in the hall (the save written now).
    let done = move || {
        if won {
            store::set_world_at(HALL_DOOR);
            screen.set(Screen::World);
        } else {
            screen.set(Screen::Campaign);
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

    let title = progress.with_untracked(champion_title);
    let seal_rivals = || RIVALS.iter().filter(|r| r.seal);
    // The plates, in order: (how long each shows in ms, what it shows). The
    // last one stays until Continue.
    let mut plates: Vec<(u32, AnyView)> = vec![];
    if won {
        plates.push((
            RING_AT + 1900,
            view! {
                {seal_stone()}
                <p class="credits-line">{tr("Six faces, all lit.", "Six faces, toutes allumées.")}</p>
            }
            .into_any(),
        ));
        plates.push((
            4600,
            view! {
                <h2 class="wordmark champion-title" class:long={title.en.len() > 10}>{title.get()}</h2>
                <p class="credits-line">{tr("Six seals and the Warden's chair.", "Six sceaux et le fauteuil de la Gardienne.")}</p>
            }
            .into_any(),
        ));
    }
    plates.push((
        5000,
        view! {
            <h1 class="wordmark">"Sixfold"</h1>
            <p class="credits-line">{if won {
                tr("Six seals, one Warden, and a new champion.", "Six sceaux, une Gardienne… et une nouvelle légende.")
            } else {
                tr("The people and the tools behind it.", "Ceux qui l'ont fait, et avec quoi.")
            }}</p>
        }
        .into_any(),
    ));
    if let Some(line) = stats.with_untracked(run_line) {
        plates.push((
            7000,
            view! {
                <h2>{tr("Your run", "Votre partie")}</h2>
                <ul class="seals credits-seals" aria-hidden="true">
                    {seal_rivals().map(|r| view! {
                        <li><crate::campaign::Seal r=r earned=progress.with_untracked(|p| r.beaten(p)) /></li>
                    }).collect_view()}
                </ul>
                {line}
            }
            .into_any(),
        ));
    }
    let face = |r: &'static Rival| {
        view! {
            <li class=format!("cast-one t-{}", tone(r))>
                <img src=format!("world/assets/portraits/{}.webp", r.id) alt="" width="256" height="256" decoding="async" />
                <b>{r.name}</b>
                <span>{r.title.get()}</span>
            </li>
        }
    };
    plates.push((
        8000,
        view! {
            <h2>{tr("The rivals", "Les rivaux")}</h2>
            <ul class="cast">{seal_rivals().map(face).collect_view()}</ul>
            <ul class="cast warden">{RIVALS.iter().filter(|r| !r.seal).map(face).collect_view()}</ul>
        }
        .into_any(),
    ));
    plates.push((
        6500,
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
        8000,
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
        view! {
            <h2>{tr("A game by Atypical Play", "Un jeu Atypical Play")}</h2>
            <ul class="credits-list solo">
                <li><b>"Philippe Matray"</b><span>{tr("direction, game design, code, writing and cards", "direction, conception, code, écriture et cartes")}</span></li>
            </ul>
        }
        .into_any(),
    ));
    plates.push((
        8000,
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
        9000,
        view! {
            <h2>{tr("Licences", "Licences")}</h2>
            <ul class="credits-list licences">
                {LICENCES.iter().map(|(name, what)| view! {
                    <li><b>{name.get()}</b><span>{what.get()}</span></li>
                }).collect_view()}
            </ul>
            <p class="credits-line small">{tr("Sixfold's own code is MIT too.", "Le code de Sixfold est lui aussi sous licence MIT.")}</p>
        }
        .into_any(),
    ));
    plates.push((
        0,
        view! {
            <p class="credits-end">{tr("Thank you for playing.", "Merci d'avoir joué.")}</p>
            <p class="credits-line">{tr(
                "Every Hexon, card, place, picture and sound here is original. Only the rules come from a classic handheld card game.",
                "Chaque Hexon, carte, lieu, image et son de ce jeu est original. Seules les règles viennent d'un grand classique du jeu de cartes sur console portable.",
            )}</p>
        }
        .into_any(),
    ));
    let durations: Vec<u32> = plates.iter().map(|(ms, _)| *ms).collect();
    let n = plates.len();
    let stone = won.then_some(0);

    // The plate on show. Each change bumps `armed`, so a timer armed for an
    // earlier plate (before a Next, Back or Skip) does nothing.
    let at = RwSignal::new(0usize);
    let armed = StoredValue::new(0u32);
    let show = move |i: usize| {
        armed.update_value(|t| *t += 1);
        at.set(i.min(n - 1));
    };
    let later = move |ms: u32, f: Box<dyn FnOnce()>| {
        let t = armed.get_value();
        set_timeout(
            move || {
                if armed.try_get_value() == Some(t) {
                    f()
                }
            },
            Duration::from_millis(u64::from(ms)),
        );
    };
    Effect::new(move |_| {
        let i = at.get();
        // The stone rings a bell per face as it lights, then all at once.
        if stone == Some(i) {
            if !still {
                for k in 0..6 {
                    later(
                        FACE_AT + k * FACE_GAP,
                        Box::new(move || crate::audio::sfx("face", f64::from(k))),
                    );
                }
            }
            later(
                if still { 200 } else { RING_AT },
                Box::new(|| crate::audio::sfx("face", 6.0)),
            );
        }
        if let Some(&ms) = durations.get(i).filter(|&&ms| ms > 0 && i + 1 < n) {
            later(ms, Box::new(move || show(i + 1)));
        }
    });

    let last = move || at.get() + 1 == n;
    // Next disappears on the last plate: focus goes on to Continue, not lost.
    let primary = NodeRef::<leptos::html::Button>::new();
    let next = move |_| {
        show(at.get_untracked() + 1);
        if at.get_untracked() + 1 == n {
            if let Some(b) = primary.get_untracked() {
                _ = b.focus();
            }
        }
    };
    let on_key = move |e: leptos::ev::KeyboardEvent| match e.key().as_str() {
        "Escape" => {
            e.prevent_default();
            done();
        }
        "ArrowRight" if !last() => show(at.get_untracked() + 1),
        "ArrowLeft" if at.get_untracked() > 0 => show(at.get_untracked() - 1),
        _ => {}
    };
    view! {
        <main class="credits" class:still=still on:keydown=on_key aria-label=tr("Credits", "Générique")>
            <ol class="credits-pips" aria-hidden="true">
                {(0..n).map(|i| view! { <li class:on=move || at.get() == i class:past=move || { at.get() > i }></li> }).collect_view()}
            </ol>
            <div class="credits-stage" aria-live="polite">
                {plates.into_iter().enumerate().map(|(i, (_, v))| view! {
                    <div class="credits-plate" class:on=move || at.get() == i
                        aria-hidden=move || (at.get() != i).to_string() inert=move || at.get() != i>
                        {v}
                    </div>
                }).collect_view()}
            </div>
            <div class="credits-foot">
                <MuteToggle />
                {move || (!last()).then(|| view! {
                    <button type="button" class="btn ghost" on:click=next>{tr("Next", "Suivant")}</button>
                })}
                // Focus starts on Skip (see `focus_heading`), so Enter works straight away.
                <button type="button" class="btn primary" data-screen-focus node_ref=primary on:click=move |_| done()>
                    {move || if last() { tr("Continue", "Continuer") } else { tr("Skip the credits", "Passer le générique") }}
                </button>
            </div>
        </main>
    }
}
