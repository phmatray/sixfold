//! The component sheet (`?sheet`): every control of the kit, in each state, on
//! one screen. For reviewing the kit; not reachable from the game.

use leptos::prelude::*;

#[component]
fn States(class: &'static str, label: &'static str) -> impl IntoView {
    let states = [
        ("", "Rest"),
        ("is-hover", "Hover"),
        ("is-active", "Pressed"),
        ("is-focus", "Focus"),
    ];
    view! {
        <div class="kit-states">
            {states.into_iter().map(|(s, name)| view! {
                <figure>
                    <button type="button" class=format!("btn {class} {s}")>{label}</button>
                    <figcaption>{name}</figcaption>
                </figure>
            }).collect_view()}
            <figure>
                <button type="button" class=format!("btn {class}") disabled>{label}</button>
                <figcaption>"Disabled"</figcaption>
            </figure>
        </div>
    }
}

#[component]
pub fn KitSheet() -> impl IntoView {
    let row = |icon: &'static str, label: &'static str, sub: &'static str, state: &'static str| {
        view! {
            <li><button type="button" class=format!("menu-item {state}")>
                <svg class="menu-icon" viewBox="0 0 24 24" aria-hidden="true"><path d=icon /></svg>
                <span class="menu-label">{label}</span>
                <span class="menu-sub">{sub}</span>
            </button></li>
        }
    };
    view! {
        <div class="page kit-sheet">
            <header class="topbar page-bar">
                <button type="button" class="btn ghost back">
                    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M15 5l-7 7 7 7" /></svg>
                    "Back"
                </button>
                <h1 class="page-title">"Component kit"</h1>
            </header>
            <main class="page-body">
                <section class="kit-block">
                    <h2>"Primary"</h2>
                    <States class="primary" label="Start the duel" />
                    <h2>"Secondary"</h2>
                    <States class="" label="New seed" />
                </section>
                <div class="kit-cols">
                    <section class="kit-block kit-panel settings-body">
                        <h2>"Panel"</h2>
                        <div class="menu-tabs" role="tablist" aria-label="Tabs">
                            <button type="button" role="tab" aria-selected="true">"Achievements"</button>
                            <button type="button" role="tab" aria-selected="false">"Stats"</button>
                            <button type="button" role="tab" aria-selected="false" class="is-hover">"Duels"</button>
                        </div>
                        <fieldset class="audio-settings">
                            <legend>"Sound"</legend>
                            <label class="vol-mute"><input type="checkbox" checked /> "Music on the title"</label>
                            <label class="vol-mute"><input type="checkbox" /> "Mute all sound"</label>
                            <label class="vol"><span>"Music"</span><input type="range" min="0" max="100" value="60" style="--fill: 60%" /><span class="vol-pct">"60%"</span></label>
                            <label class="vol"><span>"Effects"</span><input type="range" min="0" max="100" value="80" style="--fill: 80%" /><span class="vol-pct">"80%"</span></label>
                        </fieldset>
                        <div class="options-row">
                            <label class="field"><span>"Opponent"</span>
                                <select><option>"Laid-back"</option><option selected>"Steady"</option><option>"Formidable"</option></select>
                            </label>
                            <fieldset class="seg"><legend>"Prizes"</legend>
                                <label><input type="radio" name="kp" />"4"</label>
                                <label><input type="radio" name="kp" checked />"6"</label>
                            </fieldset>
                        </div>
                    </section>
                    <section class="kit-block">
                        <h2>"List"</h2>
                        <ul class="menu kit-list">
                            {row("M3.5 8.2l6.3-1.9 3.4 11.3-6.3 1.9zM13.2 4.5l6.6.9-1.6 11.6-3.4-.5", "Play a duel", "Your deck against a practice opponent", "")}
                            {row("M6 21V3.5M6 4h12l-2.5 4.5L18 13H6", "Campaign", "2 of 6 seals · next: Bramble", "is-focus")}
                            {row("M8 3.5h11v15H8zM5 6.5v14h11", "My decks", "3 decks", "is-hover")}
                            {row("M4 4h7v7H4zM13 4h7v7h-7zM4 13h7v7H4zM13 13h7v7h-7z", "Collection", "17 of 104 cards found", "")}
                        </ul>
                        <p class="menu-help">"2 of 6 seals · next: Bramble"</p>
                        <div class="kit-states">
                            <button type="button" class="btn ghost">"Ghost"</button>
                            <button type="button" class="btn danger">"Reset progress"</button>
                        </div>
                    </section>
                </div>
            </main>
        </div>
    }
}
