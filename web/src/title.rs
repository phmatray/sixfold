//! Title screen: pick your deck and the AI's, see the seed, start.

use leptos::prelude::*;
use pkmntcg_engine::decks::{Prebuilt, PREBUILT};
use pkmntcg_engine::{card, CardId, CardKind, Color, EnergyCard};

use crate::cards::{CardFace, Pip};
use crate::ctl::{Ctl, Setup};
use crate::Screen;

/// The decks on offer. Swap this one line to change the menu.
fn decks() -> &'static [&'static Prebuilt] {
    &PREBUILT
}

/// The basic energy colours a deck runs, most-used first.
fn deck_colors(cards: &[CardId]) -> Vec<Color> {
    let mut counts: Vec<(Color, usize)> = vec![];
    for &id in cards {
        if let CardKind::Energy(EnergyCard::Basic(c)) = card(id).kind {
            match counts.iter_mut().find(|(k, _)| *k == c) {
                Some((_, n)) => *n += 1,
                None => counts.push((c, 1)),
            }
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1));
    counts.into_iter().map(|(c, _)| c).collect()
}

/// The deck's showpiece: its highest-HP creature.
fn deck_star(cards: &[CardId]) -> Option<CardId> {
    cards
        .iter()
        .copied()
        .filter(|&id| matches!(card(id).kind, CardKind::Creature(_)))
        .max_by_key(|&id| card(id).creature().map_or(0, |c| c.hp))
}

#[component]
fn DeckPicker(legend: &'static str, name: &'static str, choice: RwSignal<usize>) -> impl IntoView {
    view! {
        <fieldset class="deck-picker">
            <legend>{legend}</legend>
            {decks().iter().enumerate().map(|(i, d)| {
                let cards = d.cards();
                let colors = deck_colors(&cards);
                let creatures = cards.iter().filter(|&&id| card(id).creature().is_some()).count();
                let id = format!("{name}-{i}");
                let for_id = id.clone();
                view! {
                    <label class="deck-tile" class:chosen=move || choice.get() == i for=for_id>
                        <input type="radio" id=id name=name aria-label=d.name prop:checked=move || choice.get() == i on:change=move |_| choice.set(i) />
                        {deck_star(&cards).map(|s| view! { <span class="deck-star" aria-hidden="true"><CardFace id=s /></span> })}
                        <span class="deck-info">
                            <span class="deck-name">{d.name}</span>
                            <span class="deck-colors">{colors.into_iter().map(|c| view! { <Pip color=c /> }).collect_view()}</span>
                            <span class="deck-desc">{d.description}</span>
                            <span class="deck-count">{format!("{} cards, {creatures} creatures", cards.len())}</span>
                        </span>
                    </label>
                }
            }).collect_view()}
        </fieldset>
    }
}

#[component]
pub fn TitleScreen() -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let screen = expect_context::<RwSignal<Screen>>();
    let mine = RwSignal::new(0usize);
    let theirs = RwSignal::new(if decks().len() > 1 { 1 } else { 0 });
    let seed = RwSignal::new(crate::new_seed());
    let prizes = RwSignal::new(6u8);
    // Three cards fanned in the hero, taken from the first deck.
    let hero: Vec<CardId> = decks()
        .iter()
        .filter_map(|d| deck_star(&d.cards()))
        .chain(decks().first().map(|d| d.cards()).unwrap_or_default().into_iter().filter(|&id| card(id).is_basic()).take(1))
        .take(3)
        .collect();
    let start = move |_| {
        let (a, b) = (decks()[mine.get()], decks()[theirs.get()]);
        let setup = Setup {
            decks: [a.cards(), b.cards()],
            names: [a.name.to_string(), b.name.to_string()],
            prizes: prizes.get(),
        };
        if ctl.start(setup, seed.get()) {
            screen.set(Screen::Match);
        }
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
                </div>
            </section>
            {if decks().is_empty() {
                view! { <p class="empty-note">"No decks are available yet."</p> }.into_any()
            } else {
                view! {
                    <form class="setup" on:submit=move |e| { e.prevent_default(); start(()) }>
                        <div class="pickers">
                            <DeckPicker legend="Your deck" name="mine" choice=mine />
                            <DeckPicker legend="The AI plays" name="theirs" choice=theirs />
                        </div>
                        <div class="options">
                            <label class="field">
                                <span>"Seed"</span>
                                <input type="number" min="0" inputmode="numeric"
                                    prop:value=move || seed.get().to_string()
                                    on:input=move |e| { if let Ok(n) = event_target_value(&e).parse() { seed.set(n) } } />
                            </label>
                            <button type="button" class="btn ghost" on:click=move |_| seed.set(crate::new_seed())>"New seed"</button>
                            <label class="field">
                                <span>"Prizes"</span>
                                <select on:change=move |e| { if let Ok(n) = event_target_value(&e).parse() { prizes.set(n) } }>
                                    {(1..=6u8).rev().map(|n| view! { <option value=n.to_string() selected=n == 6>{n}</option> }).collect_view()}
                                </select>
                            </label>
                            <button type="submit" class="btn primary start">"Start the duel"</button>
                        </div>
                        {move || ctl.error.get().map(|e| view! { <p class="error" role="alert">{e}</p> })}
                    </form>
                }.into_any()
            }}
            <footer class="foot">"Original cards and art. The rules follow a classic handheld version of the game."</footer>
        </main>
    }
}
