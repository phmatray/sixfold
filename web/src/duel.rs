//! Duel setup: one of your decks (or a prebuilt) against an AI prebuilt.

use leptos::prelude::*;
use sixfold_engine::ai::Difficulty;
use sixfold_engine::decks::PREBUILT;
use sixfold_engine::profile::Profile;
use sixfold_engine::{card, tr, CardId};

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
            desc: tr("Your deck", "Votre deck").into(),
            cards: d.cards.clone(),
            problem: deck_problem(d, &p.collection),
        })
        .chain(
            prebuilt_opts()
                .into_iter()
                .filter(|o| held.binary_search(&sorted(&o.cards)).is_err())
                .map(|o| Opt {
                    desc: tr(
                        format!("Prebuilt. {}", o.desc),
                        format!("Préconstruit. {}", o.desc),
                    ),
                    ..o
                }),
        )
        .collect()
}

/// "60 cards, 14 creatures", with French plurals (0 and 1 singular).
fn deck_count(cards: usize, creatures: usize) -> String {
    if sixfold_engine::fr() {
        let s = |n: usize| if n > 1 { "s" } else { "" };
        format!(
            "{cards} carte{}, {creatures} créature{}",
            s(cards),
            s(creatures)
        )
    } else {
        format!("{cards} cards, {creatures} creatures")
    }
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
        post: None,
        back: Screen::Title,
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
                                Some(p) => view! { <span class="legal no">{tr("Can't play: ", "Injouable : ")} {p}</span> }.into_any(),
                                None => view! { <span class="deck-count">{deck_count(d.cards.len(), creatures)}</span> }.into_any(),
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
    // Ember Rush, a plain attacking deck: a gentler first opponent than a
    // control deck that opens with sleep and paralysis.
    let theirs = RwSignal::new(0);
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
            post: None,
            back: Screen::Title,
        };
        if ctl.start(setup, seed.get()) {
            screen.set(Screen::Match);
        }
    };
    view! {
        <div class="page">
            <PageBar title=tr("Play a duel", "Jouer un duel") />
            <main class="page-body">
                <form class="setup" on:submit=move |e| { e.prevent_default(); start() }>
                    <details class="how-to">
                        <summary>{tr("How a duel works", "Comment se joue un duel")}</summary>
                        <ol>
                            <li>{tr("Put a Basic creature in your Active spot, and more Basics on your Bench (up to 5).", "Placez une créature de Base en position Active, et d'autres créatures de Base sur votre Banc (5 au plus).")}</li>
                            <li>{tr("Each turn you draw a card, may attach one energy, play Basics and trainers, evolve, and retreat.", "À chaque tour, vous piochez une carte ; vous pouvez attacher une énergie, jouer des créatures de Base et des Dresseurs, faire évoluer et battre en retraite.")}</li>
                            <li>{tr("Attack when the Active has the energy an attack shows. Attacking ends your turn.", "Attaquez quand votre Active porte l'énergie indiquée par l'attaque. Attaquer termine votre tour.")}</li>
                            <li>{tr("Knock out a creature to take a prize. Take all your prizes, or leave the opponent with no creature in play, to win.", "Mettez une créature K.O. pour prendre une Récompense. Pour gagner, prenez toutes vos Récompenses ou ne laissez aucune créature en jeu à l'adversaire.")}</li>
                        </ol>
                        <p class="note">{tr("Tap or hover any card to read it. Asleep and Paralyzed creatures can't attack or retreat; Confused ones may hurt themselves.", "Touchez ou survolez une carte pour la lire. Une créature Endormie ou Paralysée ne peut ni attaquer ni battre en retraite ; une créature Confuse peut se blesser elle-même.")}</p>
                    </details>
                    <div class="pickers">
                        <DeckPicker legend=tr("Your deck", "Votre deck") name="mine" opts=mine_opts choice=mine />
                        <DeckPicker legend=tr("The AI plays", "Deck de l'IA") name="theirs" opts=theirs_opts choice=theirs />
                    </div>
                    <p class="note">{tr("Win to earn a Wild Pack.", "Gagnez pour obtenir un Booster Sauvage.")}</p>
                    <div class="options">
                        <details class="more-options">
                            <summary>{tr("Seed and prizes", "Graine et Récompenses")}</summary>
                            <div class="more-body">
                                <label class="field">
                                    <span>{tr("Seed", "Graine")}</span>
                                    <input type="number" name="seed" min="0" inputmode="numeric"
                                        prop:value=move || seed.get().to_string()
                                        on:input=move |e| { if let Ok(n) = event_target_value(&e).parse() { seed.set(n) } } />
                                </label>
                                <button type="button" class="btn ghost" on:click=move |_| seed.set(crate::new_seed())>{tr("New seed", "Nouvelle graine")}</button>
                                <label class="field">
                                    <span>{tr("Prizes", "Récompenses")}</span>
                                    <select name="prizes" on:change=move |e| { if let Ok(n) = event_target_value(&e).parse() { prizes.set(n) } }>
                                        {(1..=6u8).rev().map(|n| view! { <option value=n.to_string() selected=n == 6>{n}</option> }).collect_view()}
                                    </select>
                                </label>
                            </div>
                        </details>
                        <label class="field">
                            <span>{tr("AI", "IA")}</span>
                            <select name="difficulty" on:change=move |e| difficulty.set(match event_target_value(&e).as_str() {
                                "easy" => Difficulty::Easy,
                                "hard" => Difficulty::Hard,
                                _ => Difficulty::Normal,
                            })>
                                <option value="easy">{tr("Easy: sometimes skips a play", "Facile : laisse parfois passer un coup")}</option>
                                <option value="normal" selected>"Normal"</option>
                                <option value="hard">{tr("Hard: thinks a turn ahead", "Difficile : anticipe d'un tour")}</option>
                            </select>
                        </label>
                        <button type="submit" class="btn primary start">{tr("Start the duel", "Lancer le duel")}</button>
                    </div>
                    {move || ctl.error.get().map(|e| view! { <p class="error" role="alert">{e}</p> })}
                </form>
            </main>
        </div>
    }
}
