//! The credits, rolled once the Hex Warden falls (and on demand from the
//! campaign screen). The text scrolls by itself; with reduced motion it is a
//! still page. Skip, Escape or the end of the roll moves on.

use leptos::prelude::*;
use sixfold_engine::campaign::RIVALS;

use crate::Screen;

/// The people of the world, as the credits list them.
const TOWNSFOLK: &[(&str, &str)] = &[
    ("Juniper", "keeps the duel hall"),
    ("Wren", "walks the meadow loop"),
    ("Moss", "tends the flowerbeds"),
    ("Pip", "practises the shuffle"),
    ("Hob", "bakes in Brightwell Square"),
    ("Tansy", "sells apples and one proud marrow"),
    ("Fennel", "watches the herons"),
    ("Ottilie", "keeps the lighthouse"),
    ("Dace", "waits for a bite"),
    ("Flint", "cuts stone in the quarry"),
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
                    <p class="credits-lead">"Six seals, one Warden, and a new champion."</p>
                    <ul class="seals credits-seals" aria-hidden="true">
                        {RIVALS.iter().filter(|r| r.seal).map(|r| view! {
                            <li><crate::campaign::Seal r=r earned=true /></li>
                        }).collect_view()}
                    </ul>
                })}
                <section>
                    <h2>"The rivals"</h2>
                    <ul>
                        {RIVALS.iter().map(|r| view! {
                            <li><b>{r.name}</b>" "<span>{r.title}</span></li>
                        }).collect_view()}
                    </ul>
                </section>
                <section>
                    <h2>"The townsfolk"</h2>
                    <ul>
                        {TOWNSFOLK.iter().map(|(name, what)| view! {
                            <li><b>{*name}</b>" "<span>{*what}</span></li>
                        }).collect_view()}
                    </ul>
                </section>
                <section>
                    <h2>"Made by"</h2>
                    <ul><li><b>"Philippe Matray"</b>" "<span>"design, code and cards"</span></li></ul>
                </section>
                <section>
                    <h2>"Built with"</h2>
                    <ul>
                        <li><b>"Rust"</b>" "<span>"and WebAssembly"</span></li>
                        <li><b>"Leptos"</b>" "<span>"for the screens"</span></li>
                        <li><b>"three.js"</b>" "<span>"for the world"</span></li>
                        <li><b>"Blender"</b>" "<span>"for every prop, made by script"</span></li>
                    </ul>
                </section>
                <section>
                    <p>"Every creature, card, place, picture and sound here is original. Only the rules come from a classic handheld card game."</p>
                </section>
                <p class="credits-end">"Thank you for playing."</p>
            </div>
            <div class="credits-foot">
                // Focus starts on Skip (see `focus_heading`), so Enter works straight away.
                <button type="button" class="btn primary" data-screen-focus on:click=move |_| done()>
                    {move || if ended.get() { "Continue" } else { "Skip the credits" }}
                </button>
            </div>
        </main>
    }
}
