//! The deck builder's workbench (#102): "Build for me", "Complete my deck",
//! the live advice, the test hand, undo, the energy readout, and the shelf of
//! unlockable decks. The thinking is the engine's (`sixfold_engine::builder`,
//! `sixfold_engine::decks::UNLOCKABLE`); this shows it and applies it.

use leptos::prelude::*;
use leptos::web_sys;
use sixfold_engine::builder::{self, Pick, Plan, Style, Suggestion, Why};
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

/// One step of history: the cards to go back to, the name to go back to,
/// and the name the deck had just after the change. The name is restored
/// only while it is still that one, so a rename made since is kept.
type Snap = (Vec<CardId>, String, String);

/// The editor's undo and redo: the deck before each change to its cards,
/// whatever made it (a tap on +, a fix, a suggestion, "Build for me"), with
/// the name it had then.
#[derive(Clone, Copy)]
pub struct Undo {
    past: RwSignal<Vec<Snap>>,
    future: RwSignal<Vec<Snap>>,
    skip: StoredValue<bool>,
}

impl Undo {
    /// Follow `draft`: every change to its cards can be taken back.
    pub fn track(draft: RwSignal<DeckList>) -> Undo {
        let u = Undo {
            past: RwSignal::new(vec![]),
            future: RwSignal::new(vec![]),
            skip: StoredValue::new(false),
        };
        Effect::new(move |prev: Option<(Vec<CardId>, String)>| {
            let now = draft.with(|d| (d.cards.clone(), d.name.clone()));
            match prev {
                Some(p) if p.0 != now.0 && u.skip.get_value() => u.skip.set_value(false),
                Some((cards, name)) if cards != now.0 => {
                    u.past.update(|v| {
                        v.push((cards, name, now.1.clone()));
                        if v.len() > UNDO_DEPTH {
                            v.remove(0);
                        }
                    });
                    u.future.update(Vec::clear);
                }
                _ => {}
            }
            now
        });
        u
    }

    pub fn can(&self) -> bool {
        self.past.with(|v| !v.is_empty())
    }

    pub fn can_redo(&self) -> bool {
        self.future.with(|v| !v.is_empty())
    }

    fn step(
        &self,
        draft: RwSignal<DeckList>,
        from: RwSignal<Vec<Snap>>,
        to: RwSignal<Vec<Snap>>,
    ) -> bool {
        let Some((cards, name, expected)) = from.try_update(|v| v.pop()).flatten() else {
            return false;
        };
        let (now_cards, now_name) = draft.with_untracked(|d| (d.cards.clone(), d.name.clone()));
        let rename = now_name == expected;
        let after = if rename {
            name.clone()
        } else {
            now_name.clone()
        };
        to.update(|v| v.push((now_cards, now_name, after)));
        self.skip.set_value(true);
        draft.update(|d| {
            d.cards = cards;
            if rename {
                d.name = name;
            }
        });
        true
    }

    /// Put the deck back as it was before the last change.
    pub fn undo(&self, draft: RwSignal<DeckList>) -> bool {
        self.step(draft, self.past, self.future)
    }

    /// Make the last undone change again.
    pub fn redo(&self, draft: RwSignal<DeckList>) -> bool {
        self.step(draft, self.future, self.past)
    }
}

/// Focus the first element `selector` finds, once the view has settled.
pub fn focus_later(selector: &'static str) {
    request_animation_frame(move || {
        if let Some(el) = document()
            .query_selector(selector)
            .ok()
            .flatten()
            .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
        {
            _ = el.focus();
        }
    });
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

/// What a chance of a Basic-less opening hand means for the player.
pub fn mulligan_word(p: f64) -> &'static str {
    if p <= 0.15 {
        tr("fine", "bien")
    } else if p <= 0.25 {
        tr("risky", "risqué")
    } else {
        tr("too high: add Basics", "trop : ajoutez des Hexons de Base")
    }
}

/// Per colour, its share of the deck's energy against its share of the
/// attack costs: the two should be about level.
#[component]
pub fn EnergyReadout(draft: RwSignal<DeckList>) -> impl IntoView {
    let rows = move || draft.with(|d| builder::energy_needs(&d.cards));
    view! {
        <figure class="energy-needs">
            <figcaption>{tr("Energy for each colour, against what its attacks cost", "L'énergie de chaque couleur, face au coût de ses attaques")}</figcaption>
            <ul aria-label=tr("Energy by colour", "Énergie par couleur")>
                {move || {
                    let rows = rows();
                    let (all_e, all_s) = rows.iter().fold((0, 0), |(e, s), r| (e + r.1, s + r.2));
                    rows.into_iter().map(|(c, have, want)| {
                        let (e, s) = (have * 100 / all_e.max(1), want * 100 / all_s.max(1));
                        let short = s > e + 15;
                        view! {
                            <li class="need" class:short=short>
                                <Pip color=c />
                                <span class="need-bars" aria-hidden="true">
                                    <span class="need-have" style=format!("width:{e}%")></span>
                                    <span class="need-want" style=format!("width:{s}%")></span>
                                </span>
                                <span class="need-n">
                                    {tr(format!("{have} energy ({e}%) · costs {s}%"), format!("{have} énergies ({e}\u{202f}%) · coûts {s}\u{202f}%"))}
                                    {short.then(|| view! { <b class="need-flag">{tr(" · short", " · trop peu")}</b> })}
                                </span>
                            </li>
                        }
                    }).collect_view()
                }}
            </ul>
        </figure>
    }
}

/// The live advice (`tips`, the engine's suggestions for the deck): each
/// change with its card, its reason and one tap to apply (undoable).
#[component]
pub fn Advisor(
    draft: RwSignal<DeckList>,
    tips: Memo<Vec<Suggestion>>,
    msg: RwSignal<Option<String>>,
) -> impl IntoView {
    let profile = expect_context::<RwSignal<Profile>>();
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
            format!("{}. « Annuler » revient en arrière.", s.title()),
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
                    return full.then(|| view! {
                        <p class="advice-none">{tr("Nothing to suggest: the lines are whole, the Basics and the energy are where they should be.", "Rien à suggérer : les lignées sont complètes, les Hexons de Base et l'énergie sont à leur place.")}</p>
                    }).into_any();
                }
                view! {
                    <ul class="advice-list" data-focus-home="">
                        {list.into_iter().map(|s| {
                            let (title, text) = (s.title(), s.text());
                            let label = tr(format!("Apply: {title}"), format!("Appliquer : {title}"));
                            let kind = match (s.out, s.into) {
                                (Some(_), Some(_)) => "swap",
                                (None, _) => "add",
                                _ => "remove",
                            };
                            let face = s.into.or(s.out).map(|id| {
                                let (peek, open) = crate::binder::inspect_handlers(id);
                                let name = card(id).label();
                                view! {
                                    <button type="button" class="tip-face" aria-label=tr(format!("{name}, details"), format!("{name}, détails"))
                                        on:click=move |_| open() on:mouseenter=move |_| peek() on:focus=move |_| peek()>
                                        <CardFace id=id />
                                    </button>
                                }
                            });
                            view! {
                                <li class=format!("advice-item tip {kind}")>
                                    {face}
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

/// Close a sheet and give focus back to the tool that opened it.
fn close(open: RwSignal<bool>, opener: &'static str) {
    open.set(false);
    focus_later(opener);
}

/// A sheet over the editor; Escape (or B) closes it and nothing else.
fn esc_closes(
    open: RwSignal<bool>,
    opener: &'static str,
) -> impl Fn(leptos::ev::KeyboardEvent) + Copy {
    move |e| {
        if e.key() == "Escape" {
            e.prevent_default();
            e.stop_propagation();
            close(open, opener);
        }
    }
}

/// The tools the sheets come back to.
const BUILD_TOOL: &str = ".bench-actions .btn.build";
const TEST_TOOL: &str = ".bench-actions .btn.test";

/// Copies per card.
fn counts(cards: &[CardId]) -> std::collections::BTreeMap<CardId, i32> {
    let mut m = std::collections::BTreeMap::new();
    for &id in cards {
        *m.entry(id).or_insert(0) += 1;
    }
    m
}

/// What a built deck changes from the deck in the editor: "Same as your
/// deck", or the biggest changes ("+2 Ember Energy, −2 Tide Energy").
fn diff_line(from: &[CardId], to: &[CardId]) -> String {
    let (a, b) = (counts(from), counts(to));
    let mut d: Vec<(CardId, i32)> = a
        .keys()
        .chain(b.keys())
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .map(|id| {
            (
                id,
                b.get(&id).copied().unwrap_or(0) - a.get(&id).copied().unwrap_or(0),
            )
        })
        .filter(|&(_, n)| n != 0)
        .collect();
    if d.is_empty() {
        return tr(
            "Same cards as your deck.",
            "Les mêmes cartes que votre deck.",
        )
        .into();
    }
    if from.is_empty() {
        return tr(
            "A new deck from your cards.",
            "Un nouveau deck fait de vos cartes.",
        )
        .into();
    }
    d.sort_by_key(|&(id, n)| (std::cmp::Reverse(n.abs()), id));
    let shown: Vec<String> = d
        .iter()
        .take(4)
        .map(|&(id, n)| {
            format!(
                "{}{} {}",
                if n > 0 { "+" } else { "−" },
                n.abs(),
                card(id).label()
            )
        })
        .collect();
    let more = d.len().saturating_sub(4);
    let tail = match more {
        0 => String::new(),
        k => tr(format!(" and {k} more"), format!(" et {k} de plus")),
    };
    tr(
        format!("Changes from your deck: {}{tail}.", shown.join(", ")),
        format!(
            "Changements par rapport à votre deck : {}{tail}.",
            shown.join(", ")
        ),
    )
}

/// The group a reason belongs to in the preview: Hexons, trainers, energy.
fn group(why: &Why) -> usize {
    match why {
        Why::Trainer(_) => 1,
        Why::Energy(..) | Why::Twofold => 2,
        _ => 0,
    }
}

/// "Build for me": colours (or a surprise) and a style, a live preview of
/// the deck with what it changes and why each part is there, then use it
/// (undoable).
#[component]
pub fn BuildSheet(
    open: RwSignal<bool>,
    draft: RwSignal<DeckList>,
    msg: RwSignal<Option<String>>,
) -> impl IntoView {
    let profile = expect_context::<RwSignal<Profile>>();
    // Hexon cards owned of each colour, and whether one is a Basic.
    let owned: Vec<(Color, usize, bool)> = profile.with_untracked(|p| {
        COLOURS
            .iter()
            .map(|&c| {
                let of = |basic: bool| {
                    p.collection
                        .iter()
                        .filter(|&(id, _)| {
                            card(id).creature().is_some_and(|cr| cr.color == c)
                                && (!basic || card(id).is_basic())
                        })
                        .map(|(_, n)| n)
                        .sum::<usize>()
                };
                (c, of(false), of(true) > 0)
            })
            .collect()
    });
    let start: Vec<Color> = draft
        .with_untracked(|d| hexon_colours(&d.cards))
        .into_iter()
        .filter(|c| owned.iter().any(|o| o.0 == *c && o.2))
        .collect();
    let surprise = RwSignal::new(start.is_empty().then(crate::new_seed));
    let colours = RwSignal::new(start);
    let style = RwSignal::new(Style::Balanced);
    let refused = RwSignal::new(None::<String>);
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
    let toggle = move |c: Color, can: bool| {
        if !can {
            refused.set(Some(tr(
                format!("You own no Basic {c} Hexon yet, so there is no {c} deck to build: {c} packs have them."),
                format!("Vous n'avez encore aucun Hexon {c} de Base, donc pas de deck {c} possible : les boosters {c} en contiennent."),
            )));
            return;
        }
        refused.set(None);
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
        let names: Vec<String> = colours.iter().take(2).map(|c| c.to_string()).collect();
        let auto = profile.with_untracked(|p| {
            crate::decks::unique_name(p, &format!("{} {}", names.join("–"), style.label()))
        });
        draft.update(|d| {
            d.cards = b.cards;
            let untitled = d.name.trim().is_empty()
                || d.name.starts_with("New deck")
                || d.name.starts_with("Nouveau deck");
            if untitled {
                d.name = auto;
            }
        });
        crate::audio::sfx("place", 0.0);
        msg.set(Some(
            tr(
                "Built a 60-card deck from your cards. Undo brings yours back.",
                "Deck de 60 cartes construit avec vos cartes. « Annuler » rend le vôtre.",
            )
            .into(),
        ));
        close(open, BUILD_TOOL);
    };
    focus_later("#build-title");
    let diff = Memo::new(move |_| {
        let b = built.get();
        draft.with_untracked(|d| diff_line(&d.cards, &b.cards))
    });
    view! {
        <div class="scrim" on:keydown=esc_closes(open, BUILD_TOOL)>
            <div class="dialog wide build-sheet" role="dialog" aria-modal="true" aria-labelledby="build-title">
                <div class="sheet-scroll">
                <h2 id="build-title" tabindex="-1">{tr("Build for me", "Construire pour moi")}</h2>
                <p class="dialog-sub">{tr(
                    "Pick one or two colours, or let the builder surprise you, and a way to play. It uses only cards you own and says why each one is in.",
                    "Choisissez une ou deux couleurs, ou laissez-vous surprendre, et une façon de jouer. Seules vos cartes sont utilisées, et chacune a sa raison d'être.",
                )}</p>
                <fieldset class="build-colours">
                    <legend>{tr("Colours (up to two), with the Hexon cards you own", "Couleurs (deux au plus), avec vos cartes Hexon")}</legend>
                    <div class="colour-row">
                        {owned.iter().map(|&(c, n, can)| view! {
                            <button type="button" class="chip colour" aria-disabled=(!can).to_string()
                                aria-pressed=move || colours.with(|v| v.contains(&c)).to_string()
                                aria-label=tr(format!("{c}, {n} Hexon cards owned"), format!("{c}, {n} cartes Hexon possédées"))
                                on:click=move |_| toggle(c, can)>
                                <Pip color=c />{c.to_string()}<span class="chip-n" aria-hidden="true">{n}</span>
                            </button>
                        }).collect_view()}
                        <button type="button" class="chip surprise" aria-pressed=move || surprise.get().is_some().to_string()
                            on:click=move |_| { refused.set(None); colours.set(vec![]); surprise.set(Some(crate::new_seed())) }>
                            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 7h3l10 10h3M4 17h3l3-3M14 10l3-3h3M18 4l2 3-2 3M18 14l2 3-2 3" /></svg>
                            {move || if surprise.get().is_some() { tr("Surprise me again", "Encore une surprise") } else { tr("Surprise me", "Surprenez-moi") }}
                        </button>
                    </div>
                    <p class="refused" role="status">{move || refused.get()}</p>
                </fieldset>
                <fieldset class="seg build-style">
                    <legend>{tr("Style", "Style")}</legend>
                    <div class="seg-tray">
                    {Style::ALL.into_iter().map(|s| view! {
                        <label>
                            <input type="radio" name="build-style" prop:checked=move || style.get() == s on:change=move |_| style.set(s) />
                            {s.label()}
                        </label>
                    }).collect_view()}
                    </div>
                    <p class="style-blurb">{move || style.get().blurb()}</p>
                </fieldset>
                <section class="build-preview" aria-labelledby="preview-title">
                    <p class="preview-diff" role="status">{move || diff.get()}</p>
                    {move || {
                        let b = built.get();
                        let asked = if surprise.get().is_none() { colours.get() } else { vec![] };
                        let count = |keep: fn(&sixfold_engine::CardDef) -> bool| b.cards.iter().filter(|&&id| keep(card(id))).count();
                        let hexons = count(|c| c.creature().is_some());
                        let trainers = count(|c| matches!(c.kind, sixfold_engine::CardKind::Trainer(_)));
                        let energy = b.cards.len() - hexons - trainers;
                        let odds = builder::opening(&b.cards);
                        let colours = deck_colors(&b.cards);
                        let names: Vec<String> = colours.iter().map(|c| c.to_string()).collect();
                        let has = hexon_colours(&b.cards);
                        let missing: Vec<String> = asked.iter().filter(|c| !has.contains(c)).map(|c| c.to_string()).collect();
                        let summary = tr(
                            format!("{hexons} Hexons · {trainers} trainers · {energy} energy · {} of opening hands lack a Basic ({})", pc(odds.mulligan), mulligan_word(odds.mulligan)),
                            format!("{hexons} Hexons · {trainers} Dresseurs · {energy} énergies · {} des mains de départ sans Hexon de Base ({})", pc(odds.mulligan), mulligan_word(odds.mulligan)),
                        );
                        let groups = [tr("Hexons", "Hexons"), tr("Trainers", "Dresseurs"), tr("Energy", "Énergie")];
                        view! {
                            <header class="preview-head">
                                {deck_star(&b.cards).map(|s| view! { <span class="deck-star" aria-hidden="true"><CardFace id=s /></span> })}
                                <div>
                                    <h3 id="preview-title">
                                        {if names.is_empty() { tr("Your deck", "Votre deck").to_string() } else { tr(format!("Your {} deck", names.join("–")), format!("Votre deck {}", names.join("–"))) }}
                                        <span class="deck-colors">{colours.into_iter().map(|c| view! { <Pip color=c /> }).collect_view()}</span>
                                    </h3>
                                    <p class="preview-sum">{summary}</p>
                                </div>
                            </header>
                            {(!missing.is_empty()).then(|| view! {
                                <p class="preview-warn">{tr(
                                    format!("Too few {} Hexons in your collection to build on: the deck leaves them out.", missing.join(" / ")),
                                    format!("Trop peu d'Hexons {} dans votre collection pour bâtir dessus : le deck s'en passe.", missing.join(" / ")),
                                )}</p>
                            })}
                            {(odds.mulligan > 0.25 || energy > 28).then(|| view! {
                                <p class="preview-warn">{tr(
                                    "Your cards in these colours make a thin deck. Try other colours, or open packs for more.",
                                    "Vos cartes de ces couleurs donnent un deck maigre. Essayez d'autres couleurs, ou ouvrez des boosters.",
                                )}</p>
                            })}
                            {groups.into_iter().enumerate().map(|(g, name)| {
                                let lines: Vec<String> = b.reasons.iter().filter(|r| group(&r.why) == g).map(|r| r.text()).collect();
                                (!lines.is_empty()).then(|| view! {
                                    <h4 class="reasons-group">{name}</h4>
                                    <ul class="reasons">{lines.into_iter().map(|t| view! { <li>{t}</li> }).collect_view()}</ul>
                                })
                            }).collect_view()}
                        }
                    }}
                </section>
                    {(had > 0).then(|| view! {
                        <p class="note replace-note">{tr(
                            format!("Replaces the {had} cards in the deck; Undo brings them back."),
                            format!("Remplace les {had} cartes du deck ; « Annuler » les rend."),
                        )}</p>
                    })}
                </div>
                <div class="dialog-foot sheet-foot">
                    <button type="button" class="btn ghost" on:click=move |_| close(open, BUILD_TOOL)>{tr("Keep my deck", "Garder mon deck")}</button>
                    <button type="button" class="btn primary" on:click=use_it>{tr("Use this deck", "Prendre ce deck")}</button>
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
    focus_later("#test-title");
    view! {
        <div class="scrim" on:keydown=esc_closes(open, TEST_TOOL)>
            <div class="dialog wide test-sheet" role="dialog" aria-modal="true" aria-labelledby="test-title">
                <h2 id="test-title" tabindex="-1">{tr("Test a hand", "Tester une main")}</h2>
                <p class="dialog-sub">{tr("Seven cards off the top of the shuffled deck, as a duel starts.", "Sept cartes du dessus du deck mélangé, comme au début d'un duel.")}</p>
                <ul class="test-hand" aria-label=tr("Your opening hand", "Votre main de départ")>
                    {move || hand().into_iter().map(|id| view! { <li class:basic=card(id).is_basic()><CardFace id=id /></li> }).collect_view()}
                </ul>
                <p class="test-verdict" role="status">{move || {
                    let n = hand().iter().filter(|&&id| card(id).is_basic()).count();
                    match n {
                        0 => tr("No Basic Hexon: you would show this hand and draw a new one (a mulligan).", "Aucun Hexon de Base : vous montreriez cette main et en piocheriez une autre (un mulligan).").to_string(),
                        1 => tr("1 Basic Hexon to start with.", "1 Hexon de Base pour commencer.").to_string(),
                        n => tr(format!("{n} Basic Hexons to start with."), format!("{n} Hexons de Base pour commencer.")),
                    }
                }}</p>
                <dl class="odds">
                    <div><dt>{tr("Hands with no Basic", "Mains sans Hexon de Base")}</dt><dd>{move || pc(odds().mulligan)}<span class="odds-word">{move || mulligan_word(odds().mulligan)}</span></dd></div>
                    <div><dt>{tr("Basics in a hand, on average", "Hexons de Base par main, en moyenne")}</dt><dd>{move || {
                        let b = odds().basics;
                        if fr() { format!("{b:.1}").replace('.', ",") } else { format!("{b:.1}") }
                    }}</dd></div>
                    <div><dt>{tr("Hands with an energy", "Mains avec une énergie")}</dt><dd>{move || pc(odds().energy)}</dd></div>
                </dl>
                <div class="dialog-foot">
                    <button type="button" class="btn ghost" on:click=move |_| close(open, TEST_TOOL)>{tr("Close", "Fermer")}</button>
                    <button type="button" class="btn primary" on:click=move |_| { crate::audio::sfx("flip", 0.0); seed.update(|s| *s = s.wrapping_add(1)) }>
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
                "Decks you unlock are lent to you: they join your decks under “Play a duel”, as they are. Copy one here to change it and build toward it.",
                "Les decks débloqués vous sont prêtés : ils rejoignent vos decks dans « Jouer un duel », tels quels. Copiez-en un ici pour le modifier et vous en approcher.",
            )}</p>
            <ul class="shelf-grid" data-focus-home="">
                {move || {
                    // Unlocked first, then the rest in the order a playthrough meets them.
                    let mut shelf: Vec<_> = UNLOCKABLE.iter().collect();
                    progress.with(|p| shelf.sort_by_key(|u| !u.unlocked(p)));
                    shelf.into_iter().map(|u| {
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
                                    0 => tr(format!("“{n}” is in your decks, ready to play."), format!("« {n} » est dans vos decks, prêt à jouer.")),
                                    k => tr(
                                        format!("“{n}” is in your decks. You own all but {k} of its cards: open it and “Complete my deck” fills the gaps."),
                                        format!("« {n} » est dans vos decks. Il vous manque {k} de ses cartes : ouvrez-le, « Compléter mon deck » comble les trous."),
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
                                <details class="shelf-peek">
                                    <summary>{tr("See its cards", "Voir ses cartes")}</summary>
                                    <ul>{u.deck.list.iter().map(|&(id, n)| view! { <li><b>{n}</b>" " {card(id).label()}</li> }).collect_view()}</ul>
                                </details>
                            </div>
                        </li>
                    }
                }).collect_view()
                }}
            </ul>
            <p class="shelf-note" role="status" aria-live="polite">{move || note.get()}</p>
        </section>
    }
}
