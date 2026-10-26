//! The deck builder's workbench (#102): "Build for me", "Complete my deck",
//! the live advice, the test hand, undo, the energy readout, and the shelf of
//! unlockable decks. The thinking is the engine's (`sixfold_engine::builder`,
//! `sixfold_engine::decks::UNLOCKABLE`); this shows it and applies it.

use leptos::prelude::*;
use leptos::web_sys;
use sixfold_engine::builder::{self, Pick, Plan, Style, Suggestion};
use sixfold_engine::decks::UNLOCKABLE;
use sixfold_engine::overworld::Progress;
use sixfold_engine::profile::Profile;
use sixfold_engine::{card, fr, tr, CardId, Color, DeckList, DECK_SIZE};
use wasm_bindgen::{JsCast, JsValue};

use crate::cards::{deck_colors, deck_star, CardFace, Pip};

/// The six colours a deck can be built in.
const COLOURS: [Color; 6] = [
    Color::Grass,
    Color::Fire,
    Color::Water,
    Color::Lightning,
    Color::Fighting,
    Color::Psychic,
];

/// Changes this far back can be undone.
const UNDO_DEPTH: usize = 50;

/// The editor's undo: the deck's cards before each change, whatever made it
/// (a tap on +, a fix, a suggestion, "Build for me").
#[derive(Clone, Copy)]
pub struct Undo {
    past: RwSignal<Vec<Vec<CardId>>>,
    skip: StoredValue<bool>,
}

impl Undo {
    /// Follow `draft`: every change to its cards can be taken back.
    pub fn track(draft: RwSignal<DeckList>) -> Undo {
        let u = Undo {
            past: RwSignal::new(vec![]),
            skip: StoredValue::new(false),
        };
        Effect::new(move |prev: Option<Vec<CardId>>| {
            let now = draft.with(|d| d.cards.clone());
            match prev {
                Some(p) if p != now && u.skip.get_value() => u.skip.set_value(false),
                Some(p) if p != now => u.past.update(|v| {
                    v.push(p);
                    if v.len() > UNDO_DEPTH {
                        v.remove(0);
                    }
                }),
                _ => {}
            }
            now
        });
        u
    }

    pub fn can(&self) -> bool {
        self.past.with(|v| !v.is_empty())
    }

    /// Put the cards back as they were before the last change.
    pub fn undo(&self, draft: RwSignal<DeckList>) -> bool {
        let Some(prev) = self.past.try_update(|v| v.pop()).flatten() else {
            return false;
        };
        self.skip.set_value(true);
        draft.update(|d| d.cards = prev);
        true
    }
}

/// Move focus to the nearest control that way (`a11y.js`, the same as the
/// gamepad's D-pad): arrow keys walk the card grid and the lists.
pub fn move_focus(dir: &str) {
    let f = js_sys::Reflect::get(&window(), &JsValue::from_str("sixfoldMoveFocus"));
    if let Ok(f) = f.and_then(|f| f.dyn_into::<js_sys::Function>()) {
        _ = f.call1(&JsValue::NULL, &JsValue::from_str(dir));
    }
}

/// The arrow keys' direction, for [`move_focus`].
pub fn arrow(key: &str) -> Option<&'static str> {
    match key {
        "ArrowUp" => Some("up"),
        "ArrowDown" => Some("down"),
        "ArrowLeft" => Some("left"),
        "ArrowRight" => Some("right"),
        _ => None,
    }
}

/// "12 %" in French (a narrow no-break space), "12%" in English.
fn pc(p: f64) -> String {
    let n = (p * 100.0).round() as u32;
    tr(format!("{n}%"), format!("{n}\u{202f}%"))
}

/// Per colour, its share of the deck's energy against its share of the
/// attack costs: the two bars should be about level.
#[component]
pub fn EnergyReadout(draft: RwSignal<DeckList>) -> impl IntoView {
    let rows = move || draft.with(|d| builder::energy_needs(&d.cards));
    view! {
        <figure class="energy-needs">
            <ul aria-label=tr("Energy by colour", "Énergie par couleur")>
                {move || {
                    let rows = rows();
                    let (all_e, all_s) = rows.iter().fold((0, 0), |(e, s), r| (e + r.1, s + r.2));
                    rows.into_iter().map(|(c, have, want)| {
                        let (e, s) = (have * 100 / all_e.max(1), want * 100 / all_s.max(1));
                        let short = s > e + 15;
                        let label = tr(
                            format!("{c}: {e}% of the energy, {s}% of the attack costs"),
                            format!("{c} : {e} % de l'énergie, {s} % des coûts d'attaque"),
                        );
                        view! {
                            <li class="need" class:short=short aria-label=label>
                                <Pip color=c />
                                <span class="need-bars" aria-hidden="true">
                                    <span class="need-have" style=format!("width:{e}%")></span>
                                    <span class="need-want" style=format!("width:{s}%")></span>
                                </span>
                                <span class="need-n" aria-hidden="true"><b>{have}</b></span>
                            </li>
                        }
                    }).collect_view()
                }}
            </ul>
            <figcaption>{tr("Energy by colour (top bar) against the attack costs (bottom bar)", "Énergie par couleur (barre du haut) face aux coûts d'attaque (barre du bas)")}</figcaption>
        </figure>
    }
}

/// The live advice: up to six changes, each with its reason and one tap to
/// apply (undoable).
#[component]
pub fn Advisor(draft: RwSignal<DeckList>, msg: RwSignal<Option<String>>) -> impl IntoView {
    let profile = expect_context::<RwSignal<Profile>>();
    let tips = Memo::new(move |_| {
        profile.with(|p| draft.with(|d| builder::suggestions(&d.cards, &p.collection)))
    });
    let apply = move |s: Suggestion| {
        let done = profile.with_untracked(|p| {
            draft
                .try_update(|d| s.apply(d, &p.collection))
                .unwrap_or(false)
        });
        if !done {
            return;
        }
        crate::audio::sfx("place", 0.0);
        msg.set(Some(tr(
            format!("{}. Undo takes it back.", s.title()),
            format!("{}. « Annuler » revient en arrière.", s.title()),
        )));
        // The row just went: on to the next tip, or the status line.
        request_animation_frame(|| {
            let doc = document();
            let next = doc
                .query_selector(".advisor .btn.apply")
                .ok()
                .flatten()
                .or_else(|| doc.query_selector(".editor-msg").ok().flatten());
            if let Some(el) = next.and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) {
                _ = el.focus();
            }
        });
    };
    view! {
        <section class="advisor" aria-labelledby="advice-title">
            <h2 class="checks-title" id="advice-title">
                {tr("Suggestions", "Suggestions")}
                {move || { let n = tips.with(Vec::len); (n > 0).then(|| view! { <span class="count">{n}</span> }) }}
            </h2>
            {move || {
                let list = tips.get();
                if list.is_empty() {
                    let full = draft.with(|d| d.cards.len() == DECK_SIZE);
                    return view! {
                        <p class="advice-none">{if full {
                            tr("Nothing to suggest: the lines are whole, the Basics and the energy are where they should be.", "Rien à suggérer : les lignées sont complètes, les Hexons de Base et l'énergie sont à leur place.")
                        } else {
                            tr("Add cards, or let “Build for me” start the deck.", "Ajoutez des cartes, ou laissez « Construire pour moi » lancer le deck.")
                        }}</p>
                    }.into_any();
                }
                view! {
                    <ul class="advice-list" data-focus-home="">
                        {list.into_iter().map(|s| {
                            let (title, text) = (s.title(), s.text());
                            let label = tr(format!("Apply: {title}"), format!("Appliquer : {title}"));
                            let kind = match (s.out, s.into) {
                                (Some(_), Some(_)) => "swap",
                                (None, _) => "add",
                                _ => "remove",
                            };
                            let face = s.into.or(s.out);
                            view! {
                                <li class=format!("advice-item tip {kind}")>
                                    {face.map(|id| view! { <span class="tip-face" aria-hidden="true"><CardFace id=id /></span> })}
                                    <div class="tip-body">
                                        <b class="tip-title">{title}</b>
                                        <p>{text}</p>
                                    </div>
                                    <button type="button" class="btn apply" aria-label=label on:click=move |_| apply(s.clone())>
                                        {tr("Apply", "Appliquer")}
                                    </button>
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                }.into_any()
            }}
        </section>
    }
}

/// The deck's main colours (its Hexons'), most-played first, at most two.
fn hexon_colours(cards: &[CardId]) -> Vec<Color> {
    let mut by: Vec<(Color, usize)> = vec![];
    for c in cards.iter().filter_map(|&id| card(id).creature()) {
        if c.color == Color::Colorless {
            continue;
        }
        match by.iter_mut().find(|(k, _)| *k == c.color) {
            Some((_, n)) => *n += 1,
            None => by.push((c.color, 1)),
        }
    }
    by.sort_by_key(|c| std::cmp::Reverse(c.1));
    by.into_iter().take(2).map(|c| c.0).collect()
}

/// A dialog over the editor; Escape (or B) closes it and nothing else.
fn esc_closes(open: RwSignal<bool>) -> impl Fn(leptos::ev::KeyboardEvent) + Copy {
    move |e| {
        if e.key() == "Escape" {
            e.prevent_default();
            e.stop_propagation();
            open.set(false);
        }
    }
}

/// "Build for me": colours (or a surprise) and a style, a live preview of
/// the deck with why each part is there, then use it (undoable).
#[component]
pub fn BuildSheet(
    open: RwSignal<bool>,
    draft: RwSignal<DeckList>,
    msg: RwSignal<Option<String>>,
) -> impl IntoView {
    let profile = expect_context::<RwSignal<Profile>>();
    let start = draft.with_untracked(|d| hexon_colours(&d.cards));
    let surprise = RwSignal::new(start.is_empty().then(crate::new_seed));
    let colours = RwSignal::new(start);
    let style = RwSignal::new(Style::Balanced);
    let plan = move || Plan {
        pick: match surprise.get() {
            Some(s) => Pick::Surprise(s),
            None => Pick::Colors(colours.get()),
        },
        style: style.get(),
    };
    let built = Memo::new(move |_| {
        let plan = plan();
        profile.with(|p| builder::auto_build(&p.collection, &plan))
    });
    let had = draft.with_untracked(|d| d.cards.len());
    let toggle = move |c: Color| {
        surprise.set(None);
        colours.update(|v| {
            if let Some(i) = v.iter().position(|&x| x == c) {
                v.remove(i);
            } else {
                v.push(c);
                if v.len() > 2 {
                    v.remove(0);
                }
            }
        });
        if colours.with_untracked(Vec::is_empty) {
            surprise.set(Some(crate::new_seed()));
        }
    };
    let use_it = move |_| {
        let b = built.get_untracked();
        let colours = deck_colors(&b.cards);
        let style = style.get_untracked();
        draft.update(|d| {
            d.cards = b.cards;
            let untitled = d.name.trim().is_empty()
                || d.name.starts_with("New deck")
                || d.name.starts_with("Nouveau deck");
            if untitled {
                let names: Vec<String> = colours.iter().take(2).map(|c| c.to_string()).collect();
                d.name = format!("{} {}", names.join("–"), style.label());
            }
        });
        crate::audio::sfx("place", 0.0);
        msg.set(Some(
            tr(
                "Built a 60-card deck from your cards. Undo brings yours back.",
                "Deck de 60 cartes construit avec vos cartes. « Annuler » rend le vôtre.",
            )
            .into(),
        ));
        open.set(false);
    };
    view! {
        <div class="scrim" on:keydown=esc_closes(open)>
            <div class="dialog wide build-sheet" role="dialog" aria-modal="true" aria-labelledby="build-title">
                <h2 id="build-title">{tr("Build for me", "Construire pour moi")}</h2>
                <p class="dialog-sub">{tr(
                    "Pick one or two colours, or let the builder surprise you, and a way to play. It uses only cards you own and says why each one is in.",
                    "Choisissez une ou deux couleurs, ou laissez-vous surprendre, et une façon de jouer. Seules vos cartes sont utilisées, et chacune a sa raison d'être.",
                )}</p>
                <fieldset class="build-colours">
                    <legend>{tr("Colours (up to two)", "Couleurs (deux au plus)")}</legend>
                    <div class="colour-row">
                        {COLOURS.into_iter().map(|c| view! {
                            <button type="button" class="chip colour" aria-pressed=move || colours.with(|v| v.contains(&c)).to_string()
                                on:click=move |_| toggle(c)>
                                <Pip color=c />{c.to_string()}
                            </button>
                        }).collect_view()}
                        <button type="button" class="chip surprise" aria-pressed=move || surprise.get().is_some().to_string()
                            on:click=move |_| { colours.set(vec![]); surprise.set(Some(crate::new_seed())) }>
                            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 7h3l10 10h3M4 17h3l3-3M14 10l3-3h3M18 4l2 3-2 3M18 14l2 3-2 3" /></svg>
                            {move || if surprise.get().is_some() { tr("Surprise me again", "Encore une surprise") } else { tr("Surprise me", "Surprenez-moi") }}
                        </button>
                    </div>
                </fieldset>
                <fieldset class="seg build-style">
                    <legend>{tr("Style", "Style")}</legend>
                    {Style::ALL.into_iter().map(|s| view! {
                        <label>
                            <input type="radio" name="build-style" prop:checked=move || style.get() == s on:change=move |_| style.set(s) />
                            {s.label()}
                        </label>
                    }).collect_view()}
                    <p class="style-blurb">{move || style.get().blurb()}</p>
                </fieldset>
                <section class="build-preview" aria-labelledby="preview-title" aria-live="polite">
                    {move || {
                        let b = built.get();
                        let count = |keep: fn(&sixfold_engine::CardDef) -> bool| b.cards.iter().filter(|&&id| keep(card(id))).count();
                        let hexons = count(|c| c.creature().is_some());
                        let trainers = count(|c| matches!(c.kind, sixfold_engine::CardKind::Trainer(_)));
                        let energy = b.cards.len() - hexons - trainers;
                        let odds = builder::opening(&b.cards);
                        let colours = deck_colors(&b.cards);
                        let summary = tr(
                            format!("{hexons} Hexons · {trainers} trainers · {energy} energy · {} of opening hands lack a Basic", pc(odds.mulligan)),
                            format!("{hexons} Hexons · {trainers} Dresseurs · {energy} énergies · {} des mains de départ sans Hexon de Base", pc(odds.mulligan)),
                        );
                        view! {
                            <header class="preview-head">
                                {deck_star(&b.cards).map(|s| view! { <span class="deck-star" aria-hidden="true"><CardFace id=s /></span> })}
                                <div>
                                    <h3 id="preview-title">{tr("The deck", "Le deck")} <span class="deck-colors">{colours.into_iter().map(|c| view! { <Pip color=c /> }).collect_view()}</span></h3>
                                    <p class="preview-sum">{summary}</p>
                                </div>
                            </header>
                            <ul class="reasons">
                                {b.reasons.iter().map(|r| view! { <li>{r.text()}</li> }).collect_view()}
                            </ul>
                        }
                    }}
                </section>
                {(had > 0).then(|| view! {
                    <p class="note">{tr(
                        format!("This replaces the {had} cards in the deck now; Undo brings them back."),
                        format!("Ceci remplace les {had} cartes du deck ; « Annuler » les rend."),
                    )}</p>
                })}
                <div class="dialog-foot">
                    <button type="button" class="btn ghost" on:click=move |_| open.set(false)>{tr("Keep my deck", "Garder mon deck")}</button>
                    <button type="button" class="btn primary" autofocus on:click=use_it>{tr("Use this deck", "Prendre ce deck")}</button>
                </div>
            </div>
        </div>
    }
}

/// The deck test: seven cards off a shuffled deck, whether it would have to
/// redraw, and the odds over every hand.
#[component]
pub fn TestSheet(open: RwSignal<bool>, draft: RwSignal<DeckList>) -> impl IntoView {
    let seed = RwSignal::new(crate::new_seed());
    let hand = move || draft.with(|d| builder::sample_hand(&d.cards, seed.get()));
    let odds = move || draft.with(|d| builder::opening(&d.cards));
    view! {
        <div class="scrim" on:keydown=esc_closes(open)>
            <div class="dialog wide test-sheet" role="dialog" aria-modal="true" aria-labelledby="test-title">
                <h2 id="test-title">{tr("Test a hand", "Tester une main")}</h2>
                <p class="dialog-sub">{tr("Seven cards off the top of the shuffled deck, as a duel starts.", "Sept cartes du dessus du deck mélangé, comme au début d'un duel.")}</p>
                <ul class="test-hand" aria-label=tr("Your opening hand", "Votre main de départ")>
                    {move || hand().into_iter().map(|id| view! { <li class:basic=card(id).is_basic()><CardFace id=id /></li> }).collect_view()}
                </ul>
                <p class="test-verdict" role="status">{move || {
                    let n = hand().iter().filter(|&&id| card(id).is_basic()).count();
                    match n {
                        0 => tr("No Basic Hexon: you would show this hand and draw a new one (a mulligan).", "Aucun Hexon de Base : vous montreriez cette main et en piocheriez une autre (un mulligan).").to_string(),
                        1 => tr("1 Basic Hexon to start with.", "1 Hexon de Base pour commencer.").to_string(),
                        n => tr(format!("{n} Basic Hexons to start with."), format!("{n} Hexons de Base pour commencer.")),
                    }
                }}</p>
                <dl class="odds">
                    <div><dt>{tr("Hands with no Basic", "Mains sans Hexon de Base")}</dt><dd>{move || pc(odds().mulligan)}</dd></div>
                    <div><dt>{tr("Basics in a hand, on average", "Hexons de Base par main, en moyenne")}</dt><dd>{move || {
                        let b = odds().basics;
                        if fr() { format!("{b:.1}").replace('.', ",") } else { format!("{b:.1}") }
                    }}</dd></div>
                    <div><dt>{tr("Hands with an energy", "Mains avec une énergie")}</dt><dd>{move || pc(odds().energy)}</dd></div>
                </dl>
                <div class="dialog-foot">
                    <button type="button" class="btn ghost" on:click=move |_| open.set(false)>{tr("Close", "Fermer")}</button>
                    <button type="button" class="btn primary" autofocus on:click=move |_| { crate::audio::sfx("flip", 0.0); seed.update(|s| *s = s.wrapping_add(1)) }>
                        {tr("Draw again", "Repiocher")}
                    </button>
                </div>
            </div>
        </div>
    }
}

/// The deck shelf: the prebuilt and rival decks, unlocked ones ready to copy
/// (on loan: playable in a Quick duel as they are), locked ones saying how
/// to unlock them.
#[component]
pub fn DeckShelf() -> impl IntoView {
    let profile = expect_context::<RwSignal<Profile>>();
    let progress = expect_context::<RwSignal<Progress>>();
    let note = RwSignal::new(None::<String>);
    let got = move || progress.with(|p| UNLOCKABLE.iter().filter(|u| u.unlocked(p)).count());
    view! {
        <section class="deck-shelf" aria-labelledby="shelf-title">
            <div class="shelf-head">
                <h2 id="shelf-title">{tr("Deck shelf", "L'étagère à decks")}</h2>
                <p class="shelf-count">{move || tr(
                    format!("{} of {} unlocked", got(), UNLOCKABLE.len()),
                    format!("{} sur {} débloqués", got(), UNLOCKABLE.len()),
                )}</p>
            </div>
            <p class="shelf-sub">{tr(
                "Decks you unlock are lent to you: play them as they are in a Quick duel, or copy one to your decks and build toward it.",
                "Les decks débloqués vous sont prêtés : jouez-les tels quels en duel rapide, ou copiez-en un dans vos decks pour vous en approcher.",
            )}</p>
            <ul class="shelf-grid" data-focus-home="">
                {UNLOCKABLE.iter().map(|u| {
                    let open = move || progress.with(|p| u.unlocked(p));
                    let name = u.deck.name.to_string();
                    let cards = u.deck.cards();
                    let colours = deck_colors(&cards);
                    let star = deck_star(&cards);
                    let copy = {
                        let name = name.clone();
                        move |_| {
                            let mine = profile.try_update(|p| {
                                let n = crate::decks::unique_name(p, &name);
                                p.decks.push(DeckList { name: n.clone(), cards: u.deck.cards() });
                                (n, p.collection.missing(&u.deck.cards()).iter().map(|m| m.1).sum::<usize>())
                            });
                            if let Some((n, missing)) = mine {
                                crate::audio::sfx("place", 0.0);
                                note.set(Some(match missing {
                                    0 => tr(format!("“{n}” is in your decks, ready to play."), format!("« {n} » est dans vos decks, prêt à jouer.")),
                                    k => tr(
                                        format!("“{n}” is in your decks. You own all but {k} of its cards: open it and “Complete my deck” fills the gaps."),
                                        format!("« {n} » est dans vos decks. Il vous manque {k} de ses cartes : ouvrez-le, « Compléter mon deck » comble les trous."),
                                    ),
                                }));
                            }
                        }
                    };
                    view! {
                        <li class="shelf-deck" class:locked=move || !open()>
                            <span class="deck-star" aria-hidden="true">{star.map(|s| view! { <CardFace id=s /> })}</span>
                            <div class="shelf-info">
                                <h3>{name.clone()}</h3>
                                <span class="deck-colors">{colours.into_iter().map(|c| view! { <Pip color=c /> }).collect_view()}</span>
                                {move || if open() {
                                    let copy = copy.clone();
                                    view! {
                                        <p class="shelf-desc">{u.deck.description.to_string()}</p>
                                        <button type="button" class="btn" on:click=copy
                                            aria-label=tr(format!("Copy {} to my decks", u.deck.name), format!("Copier {} dans mes decks", u.deck.name))>
                                            {tr("Copy to my decks", "Copier dans mes decks")}
                                        </button>
                                    }.into_any()
                                } else {
                                    view! {
                                        <p class="shelf-lock">
                                            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 11V8a5 5 0 0 1 10 0v3M5 11h14v10H5z" /></svg>
                                            <span><span class="sr-only">{tr("Locked. ", "Verrouillé. ")}</span>{u.how.to_string()}</span>
                                        </p>
                                    }.into_any()
                                }}
                            </div>
                        </li>
                    }
                }).collect_view()}
            </ul>
            <p class="shelf-note" role="status" aria-live="polite">{move || note.get()}</p>
        </section>
    }
}
