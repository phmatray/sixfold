//! The credits, rolled once the Hex Warden falls (and on demand from the
//! campaign screen). The text scrolls by itself; with reduced motion it is a
//! still page. Skip, Escape or the end of the roll moves on.

use leptos::prelude::*;
use sixfold_engine::campaign::RIVALS;
use sixfold_engine::{tr, T};

use crate::Screen;

/// The people of the world, as the credits list them.
const TOWNSFOLK: &[(&str, T)] = &[
    (
        "Juniper",
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
        "Flint",
        T::new("cuts stone in the quarry", "taille la pierre à la carrière"),
    ),
];

#[component]
pub fn Credits(won: bool) -> impl IntoView {
    let screen = expect_context::<RwSignal<Screen>>();
    let next = if won { Screen::World } else { Screen::Campaign };
    let done = move || screen.set(next);
    let on_key = move |e: leptos::ev::KeyboardEvent| {
        if e.key() == "Escape" {
            e.prevent_default();
            done();
        }
    };
    // With reduced motion the roll is a still page, read at the reader's pace.
    let still = web_sys::window()
        .and_then(|w| {
            w.match_media("(prefers-reduced-motion: reduce)")
                .ok()
                .flatten()
        })
        .is_some_and(|m| m.matches());
    let ended = RwSignal::new(still);
    view! {
        <main class="credits" on:keydown=on_key aria-labelledby="credits-title">
            <div class="credits-roll" class:ended=move || ended.get()
                on:animationend=move |_| ended.set(true)>
                <h1 id="credits-title" class="wordmark">"Sixfold"</h1>
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
                    <p>{tr("Every creature, card, place, picture and sound here is original. Only the rules come from a classic handheld card game.", "Chaque créature, carte, lieu, image et son de ce jeu est original. Seules les règles viennent d'un grand classique du jeu de cartes sur console portable.")}</p>
                </section>
                <p class="credits-end">{tr("Thank you for playing.", "Merci d'avoir joué.")}</p>
            </div>
            <div class="credits-foot">
                // Focus starts on Skip (see `focus_heading`), so Enter works straight away.
                <button type="button" class="btn primary" data-screen-focus on:click=move |_| done()>
                    {move || if ended.get() { tr("Continue", "Continuer") } else { tr("Skip the credits", "Passer le générique") }}
                </button>
            </div>
        </main>
    }
}
