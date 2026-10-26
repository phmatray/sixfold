//! The credits, rolled once the Hex Warden falls (and on demand from the
//! campaign screen). After the Warden they open on the finale (#67): the seal
//! stone lighting its six faces, then the champion's title. The credits are
//! plates shown one at a time, each for a while, then the next fades in; with
//! reduced motion they swap without a fade. Next (or →) moves on, ← goes back,
//! Skip (or Escape) ends them. After the Warden they end in the duel hall, by
//! the sealed door; from the campaign screen they go back there.

use std::time::Duration;

use leptos::prelude::*;
use sixfold_engine::campaign::RIVALS;
use sixfold_engine::overworld::Progress;
use sixfold_engine::{tr, T};

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
        let tone = r.colors.first().map_or("colorless", |&c| crate::art::color_key(c));
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
    let ended = RwSignal::new(still);
    // The plates, in order: (how long each shows in ms, what it shows).
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
                <h1 class="wordmark champion-title" class:long={title.en.len() > 10}>{title.get()}</h1>
                <p class="credits-line">{tr("Six seals and the Warden's chair.", "Six sceaux et le fauteuil de la Gardienne.")}</p>
            }
            .into_any(),
        ));
    }
    plates.push((
        0,
        view! {
            <div class="credits-roll" class:ended=move || ended.get()
                on:animationend=move |_| ended.set(true)>
                <h2 id="credits-title" class="wordmark">"Sixfold"</h2>
                {won.then(|| view! {
                    <p class="credits-lead">{tr("Six seals, one Warden, and a new champion.", "Six sceaux, une Gardienne… et une nouvelle légende.")}</p>
                    <ul class="seals credits-seals" aria-hidden="true">
                        {RIVALS.iter().filter(|r| r.seal).map(|r| view! {
                            <li><crate::campaign::Seal r=r earned=true /></li>
                        }).collect_view()}
                    </ul>
                })}
                <section>
                    <h2>{tr("The rivals", "Les rivaux")}</h2>
                    <ul>
                        {RIVALS.iter().map(|r| view! {
                            <li><b>{r.name}</b>" "<span>{r.title.get()}</span></li>
                        }).collect_view()}
                    </ul>
                </section>
                <section>
                    <h2>{tr("The townsfolk", "Les gens du coin")}</h2>
                    <ul>
                        {TOWNSFOLK.iter().map(|(name, what)| view! {
                            <li><b>{*name}</b>" "<span>{what.get()}</span></li>
                        }).collect_view()}
                    </ul>
                </section>
                <section>
                    <h2>{tr("Made by", "Créé par")}</h2>
                    <ul><li><b>"Philippe Matray"</b>" "<span>{tr("design, code and cards", "conception, code et cartes")}</span></li></ul>
                </section>
                <section>
                    <h2>{tr("Built with", "Fait avec")}</h2>
                    <ul>
                        <li><b>"Rust"</b>" "<span>{tr("and WebAssembly", "et WebAssembly")}</span></li>
                        <li><b>"Leptos"</b>" "<span>{tr("for the screens", "pour les écrans")}</span></li>
                        <li><b>"three.js"</b>" "<span>{tr("for the world", "pour le monde")}</span></li>
                        <li><b>"Blender"</b>" "<span>{tr("for every prop, made by script", "pour chaque objet du décor, modelé par script")}</span></li>
                    </ul>
                </section>
                <section>
                    <p>{tr("Every Hexon, card, place, picture and sound here is original. Only the rules come from a classic handheld card game.", "Chaque Hexon, carte, lieu, image et son de ce jeu est original. Seules les règles viennent d'un grand classique du jeu de cartes sur console portable.")}</p>
                </section>
                <p class="credits-end">{tr("Thank you for playing.", "Merci d'avoir joué.")}</p>
            </div>
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
                    <button type="button" class="btn ghost" on:click=move |_| show(at.get_untracked() + 1)>{tr("Next", "Suivant")}</button>
                })}
                // Focus starts on Skip (see `focus_heading`), so Enter works straight away.
                <button type="button" class="btn primary" data-screen-focus on:click=move |_| done()>
                    {move || if last() && ended.get() { tr("Continue", "Continuer") } else { tr("Skip the credits", "Passer le générique") }}
                </button>
            </div>
        </main>
    }
}
