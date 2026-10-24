//! Title screen: the hero and the main menu, plus a small settings menu.

use leptos::prelude::*;
use sixfold_engine::campaign::{seals, RIVALS, SEALS};
use sixfold_engine::decks::PREBUILT;
use sixfold_engine::overworld::Progress;
use sixfold_engine::profile::Profile;
use sixfold_engine::{card, CardId};

use crate::audio::AudioSettings;
use crate::binder::progress;
use crate::cards::{deck_star, CardFace};
use crate::saves::{SaveSettings, SlotPicker};
use crate::store::{self, Notice};
use crate::Screen;

#[component]
fn MenuItem(
    label: &'static str,
    #[prop(into)] sub: Signal<String>,
    to: Screen,
    #[prop(optional)] primary: bool,
    #[prop(optional, into)] badge: Option<Signal<u32>>,
) -> impl IntoView {
    let screen = expect_context::<RwSignal<Screen>>();
    view! {
        <li>
            <button type="button" class="menu-item" class:primary=primary on:click=move |_| screen.set(to)>
                <span class="menu-label">{label}</span>
                <span class="menu-sub">{move || sub.get()}</span>
                {badge.map(|b| view! {
                    <span class="count-badge" class:zero=move || b.get() == 0 aria-hidden="true">{move || b.get()}</span>
                })}
                <svg class="menu-chev" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 5l7 7-7 7" /></svg>
            </button>
        </li>
    }
}

#[component]
fn Settings() -> impl IntoView {
    let confirm = RwSignal::new(false);
    let done = RwSignal::new(false);
    let reset = move |_| {
        store::reset();
        confirm.set(false);
        done.set(true);
    };
    view! {
        <details class="settings" on:toggle=move |_| { confirm.set(false); done.set(false) }>
            <summary class="btn ghost">
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 7h10M18 7h2M4 17h4M12 17h8" /><circle cx="16" cy="7" r="2" /><circle cx="10" cy="17" r="2" /></svg>
                "Settings"
            </summary>
            <div class="settings-body">
                <AudioSettings />
                <SaveSettings />
                {move || if confirm.get() {
                    view! {
                        <div class="confirm" role="group" aria-labelledby="reset-q">
                            <p id="reset-q">"Delete this slot's cards, decks, packs and world progress, and start again as a new player?"</p>
                            <div class="row">
                                <button type="button" class="btn danger" on:click=reset>"Reset progress"</button>
                                <button type="button" class="btn ghost" on:click=move |_| confirm.set(false) autofocus>"Keep my progress"</button>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div class="row">
                            <button type="button" class="btn" on:click=move |_| confirm.set(true)>"Reset progress…"</button>
                            {move || done.get().then(|| view! { <span class="note" role="status">"Progress reset. You start with 3 packs."</span> })}
                        </div>
                    }.into_any()
                }}
            </div>
        </details>
    }
}

#[component]
pub fn TitleScreen() -> impl IntoView {
    let profile = expect_context::<RwSignal<Profile>>();
    let notice = expect_context::<Notice>().0;
    // Three cards fanned in the hero: each prebuilt's star, then a Basic.
    let hero: Vec<CardId> = PREBUILT
        .iter()
        .filter_map(|d| deck_star(&d.cards()))
        .chain(
            PREBUILT
                .first()
                .map(|d| d.cards())
                .unwrap_or_default()
                .into_iter()
                .filter(|&id| card(id).is_basic())
                .take(1),
        )
        .take(3)
        .collect();
    let packs = Signal::derive(move || profile.with(|p| p.wallet.total()));
    let decks = move || {
        profile.with(|p| match p.decks.len() {
            0 => "Build your first deck".to_string(),
            1 => "1 deck".to_string(),
            n => format!("{n} decks"),
        })
    };
    let found = move || {
        profile.with(|p| {
            let (have, all) = progress(&p.collection);
            format!("{have} of {all} cards found")
        })
    };
    let progress = expect_context::<RwSignal<Progress>>();
    let campaign_sub = move || {
        progress.with(|p| {
            let next = RIVALS.iter().find(|r| r.open(p) && !r.beaten(p));
            match (seals(p), next) {
                (n, _) if RIVALS.iter().all(|r| r.beaten(p)) => format!("All {n} seals. Champion!"),
                (n, Some(r)) => format!("{n} of {SEALS} seals · next: {}", r.name),
                (n, None) => format!("{n} of {SEALS} seals"),
            }
        })
    };
    let pack_sub = move || match packs.get() {
        0 => "Win a duel to earn one".to_string(),
        1 => "1 pack to open".to_string(),
        n => format!("{n} packs to open"),
    };
    view! {
        <main class="title">
            <section class="hero">
                <div class="hero-fan" aria-hidden="true">
                    {hero.into_iter().enumerate().map(|(i, id)| view! {
                        <div class="hero-card" style=format!("--i:{i}")><CardFace id=id /></div>
                    }).collect_view()}
                </div>
                <div class="hero-text">
                    <h1 class="wordmark">"Sixfold"</h1>
                    <p class="tagline">"A creature card duel with the old rules: six prizes, coin flips, and no second chances on a bad hand."</p>
                    {move || notice.get().map(|n| view! {
                        <p class="error" role="alert">{n} " "
                            <button type="button" class="btn ghost" on:click=move |_| notice.set(None)>"Dismiss"</button>
                        </p>
                    })}
                    <nav aria-label="Main menu">
                        <ul class="menu">
                            <MenuItem label="Play a duel" sub="Your deck against the AI" to=Screen::Duel primary=true />
                            <MenuItem label="Campaign" sub=Signal::derive(campaign_sub) to=Screen::Campaign />
                            <MenuItem label="Open packs" sub=Signal::derive(pack_sub) to=Screen::Packs badge=packs />
                            <MenuItem label="My decks" sub=Signal::derive(decks) to=Screen::Decks />
                            <MenuItem label="Collection" sub=Signal::derive(found) to=Screen::Collection />
                            <MenuItem label="Explore the world" sub="Walk the meadow, visit the duel hall" to=Screen::World />
                        </ul>
                    </nav>
                </div>
            </section>
            <footer class="foot">
                <span>"Original cards and art. The rules follow a classic handheld version of the game."</span>
                <div class="foot-menus">
                    <SlotPicker />
                    <Settings />
                </div>
            </footer>
        </main>
    }
}
