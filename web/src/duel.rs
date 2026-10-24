//! Duel setup: one of your decks (or a prebuilt) against an AI prebuilt.

use leptos::prelude::*;
use sixfold_engine::ai::Difficulty;
use sixfold_engine::decks::PREBUILT;
use sixfold_engine::profile::Profile;
use sixfold_engine::{card, CardId};

use crate::binder::PageBar;
use crate::cards::{deck_colors, deck_star, CardFace, Pip};
use crate::ctl::{Ctl, Setup};
use crate::decks::deck_problem;
use crate::Screen;

/// A deck on offer; `problem` says why it can't be played.
#[derive(Clone)]
pub struct Opt {
    pub name: String,
    pub desc: String,
    pub cards: Vec<CardId>,
    pub problem: Option<String>,
}

fn prebuilt_opts() -> Vec<Opt> {
    PREBUILT
        .iter()
        .map(|d| Opt {
            name: d.name.to_string(),
            desc: d.description.to_string(),
            cards: d.cards(),
            problem: None,
        })
        .collect()
}

/// Your saved decks first, then the prebuilts you don't already hold as-is.
pub fn my_opts(p: &Profile) -> Vec<Opt> {
    let mut held: Vec<Vec<CardId>> = p.decks.iter().map(|d| sorted(&d.cards)).collect();
    held.sort();
    p.decks
        .iter()
        .map(|d| Opt {
            name: d.name.clone(),
            desc: "Your deck".into(),
            cards: d.cards.clone(),
            problem: deck_problem(d, &p.collection),
        })
        .chain(
            prebuilt_opts()
                .into_iter()
                .filter(|o| held.binary_search(&sorted(&o.cards)).is_err())
                .map(|o| Opt {
                    desc: format!("Prebuilt. {}", o.desc),
                    ..o
                }),
        )
        .collect()
}

fn sorted(cards: &[CardId]) -> Vec<CardId> {
    let mut v = cards.to_vec();
    v.sort();
    v
}

/// The duel the world's duel table starts: the first two prebuilts.
pub fn default_setup() -> Option<Setup> {
    let (a, b) = (PREBUILT.first()?, PREBUILT.get(1).or(PREBUILT.first())?);
    Some(Setup {
        decks: [a.cards(), b.cards()],
        names: [a.name.to_string(), b.name.to_string()],
        prizes: 6,
        difficulty: Difficulty::Normal,
        rival: None,
    })
}

#[component]
pub fn DeckPicker(
    legend: &'static str,
    name: &'static str,
    opts: Vec<Opt>,
    choice: RwSignal<usize>,
) -> impl IntoView {
    view! {
        <fieldset class="deck-picker">
            <legend>{legend}</legend>
            {opts.into_iter().enumerate().map(|(i, d)| {
                let colors = deck_colors(&d.cards);
                let creatures = d.cards.iter().filter(|&&id| card(id).creature().is_some()).count();
                let id = format!("{name}-{i}");
                let for_id = id.clone();
                let off = d.problem.is_some();
                view! {
                    <label class="deck-tile" class:chosen=move || choice.get() == i class:off=off for=for_id>
                        <input type="radio" id=id name=name aria-label=d.name.clone() disabled=off
                            prop:checked=move || choice.get() == i on:change=move |_| choice.set(i) />
                        {deck_star(&d.cards).map(|s| view! { <span class="deck-star" aria-hidden="true"><CardFace id=s /></span> })}
                        <span class="deck-info">
                            <span class="deck-name">{d.name.clone()}</span>
                            <span class="deck-colors">{colors.into_iter().map(|c| view! { <Pip color=c /> }).collect_view()}</span>
                            <span class="deck-desc">{d.desc}</span>
                            {match d.problem {
                                Some(p) => view! { <span class="legal no">"Can't play: " {p}</span> }.into_any(),
                                None => view! { <span class="deck-count">{format!("{} cards, {creatures} creatures", d.cards.len())}</span> }.into_any(),
                            }}
                        </span>
                    </label>
                }
            }).collect_view()}
        </fieldset>
    }
}

#[component]
pub fn DuelSetup() -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let screen = expect_context::<RwSignal<Screen>>();
    let profile = expect_context::<RwSignal<Profile>>();
    let mine_opts = profile.with_untracked(my_opts);
    let theirs_opts = prebuilt_opts();
    // Every prebuilt is listed, as your copy or on its own, so a deck is always playable.
    let first_legal = mine_opts.iter().position(|o| o.problem.is_none());
    let mine = RwSignal::new(first_legal.unwrap_or(0));
    let theirs = RwSignal::new(if theirs_opts.len() > 1 { 1 } else { 0 });
    let seed = RwSignal::new(crate::new_seed());
    let prizes = RwSignal::new(6u8);
    let difficulty = RwSignal::new(Difficulty::Normal);
    let (m, t) = (
        StoredValue::new(mine_opts.clone()),
        StoredValue::new(theirs_opts.clone()),
    );
    let start = move || {
        let (a, b) = (
            m.with_value(|o| o[mine.get()].clone()),
            t.with_value(|o| o[theirs.get()].clone()),
        );
        if a.problem.is_some() {
            return;
        }
        let setup = Setup {
            decks: [a.cards, b.cards],
            names: [a.name, b.name],
            prizes: prizes.get(),
            difficulty: difficulty.get(),
            rival: None,
        };
        if ctl.start(setup, seed.get()) {
            screen.set(Screen::Match);
        }
    };
    view! {
        <div class="page">
            <PageBar title="Play a duel" />
            <main class="page-body">
                <form class="setup" on:submit=move |e| { e.prevent_default(); start() }>
                    <div class="pickers">
                        <DeckPicker legend="Your deck" name="mine" opts=mine_opts choice=mine />
                        <DeckPicker legend="The AI plays" name="theirs" opts=theirs_opts choice=theirs />
                    </div>
                    <p class="note">"Win to earn a Wild Pack."</p>
                    <div class="options">
                        <label class="field">
                            <span>"Seed"</span>
                            <input type="number" name="seed" min="0" inputmode="numeric"
                                prop:value=move || seed.get().to_string()
                                on:input=move |e| { if let Ok(n) = event_target_value(&e).parse() { seed.set(n) } } />
                        </label>
                        <button type="button" class="btn ghost" on:click=move |_| seed.set(crate::new_seed())>"New seed"</button>
                        <label class="field">
                            <span>"Prizes"</span>
                            <select name="prizes" on:change=move |e| { if let Ok(n) = event_target_value(&e).parse() { prizes.set(n) } }>
                                {(1..=6u8).rev().map(|n| view! { <option value=n.to_string() selected=n == 6>{n}</option> }).collect_view()}
                            </select>
                        </label>
                        <label class="field">
                            <span>"AI"</span>
                            <select name="difficulty" on:change=move |e| difficulty.set(match event_target_value(&e).as_str() {
                                "easy" => Difficulty::Easy,
                                "hard" => Difficulty::Hard,
                                _ => Difficulty::Normal,
                            })>
                                <option value="easy">"Easy: sometimes skips a play"</option>
                                <option value="normal" selected>"Normal"</option>
                                <option value="hard">"Hard: thinks a turn ahead"</option>
                            </select>
                        </label>
                        <button type="submit" class="btn primary start">"Start the duel"</button>
                    </div>
                    {move || ctl.error.get().map(|e| view! { <p class="error" role="alert">{e}</p> })}
                </form>
            </main>
        </div>
    }
}
