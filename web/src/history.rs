//! The play history: the last turns' plays as card tiles, newest first,
//! grouped by whose turn it was. A tile opens what that play did; the full
//! text log stays one button away.

use leptos::prelude::*;
use sixfold_engine::tr;

use crate::cards::CardFace;
use crate::ctl::{Ctl, Play, HUMAN};
use crate::Ui;

/// Turns shown in the rail.
const TURNS: usize = 2;

fn kind_icon(kind: &str) -> &'static str {
    match kind {
        "attack" => "M5 19 13 4M9.5 20.5 17 7M14 21l6-11",
        "awaken" => "M6 13.5 12 7.5l6 6M6 19.5l6-6 6 6",
        "gear" => "M7 3h7l4 4v14H7zM14 3v4h4M10 12h5M10 16h5",
        "ko" => "M12 2.5 14 9l6.5-2-4 5.3 4 5.2-6.5-2L12 22l-2-6.5-6.5 2 4-5.2-4-5.3L10 9z",
        "retreat" => "M8 5 4 9l4 4M4 9h10.5a5 5 0 0 1 0 10H9",
        "energy" => "M13.5 2 5 13.5h6L9.5 22 19 10h-6.5z",
        "power" => {
            "M12 3v4M12 17v4M3 12h4M17 12h4M6 6l2.5 2.5M15.5 15.5 18 18M6 18l2.5-2.5M15.5 8.5 18 6"
        }
        "pass" => "M5 12h10M11 7l5 5-5 5M19 6v12",
        _ => "M5 12h14M12 5v14",
    }
}

/// An attack's damage as a badge: "−30", or "Miss" when it did nothing.
fn dmg_label(n: u16) -> String {
    if n == 0 {
        tr("Miss", "Raté").to_string()
    } else {
        format!("−{n}")
    }
}

/// (side, turn, plays newest first), newest turn first.
fn groups(plays: &[Play], turns: usize) -> Vec<(usize, u32, Vec<Play>)> {
    let mut out: Vec<(usize, u32, Vec<Play>)> = vec![];
    for p in plays.iter().rev() {
        if out.last().is_some_and(|g| g.1 == p.turn) {
            if let Some(g) = out.last_mut() {
                g.2.push(p.clone());
            }
        } else if out.len() == turns {
            break;
        } else {
            out.push((p.side, p.turn, vec![p.clone()]));
        }
    }
    // Plays within a turn read in the order they happened.
    for g in &mut out {
        g.2.reverse();
    }
    out
}

/// A tile's picture: the card's illustration, else its whole face.
fn rail_art(id: sixfold_engine::CardId) -> AnyView {
    match crate::art::art_src(sixfold_engine::card(id)) {
        Some(src) => view! { <img src=src alt="" loading="lazy" decoding="async" /> }.into_any(),
        None => view! { <CardFace id=id /> }.into_any(),
    }
}

#[component]
pub fn HistoryRail() -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let ui = expect_context::<Ui>();
    let open = expect_context::<RwSignal<Option<u32>>>();
    let names = ctl.setup.get_value().map(|s| s.names).unwrap_or_default();
    let shown = Memo::new(move |_| ctl.history.with(|h| groups(h, TURNS)));
    view! {
        <nav class="rail" aria-label=tr("Play history", "Historique")>
            <h2 class="sr-only">{tr("Play history", "Historique")}</h2>
            {move || shown.with(Vec::is_empty).then(|| view! {
                <p class="rail-empty">{tr("Plays show up here.", "Les coups s'affichent ici.")}</p>
            })}
            <ol class="rail-groups">
                {move || shown.get().into_iter().map(|(side, turn, plays)| {
                    let who = if side == HUMAN { tr("You", "Vous").to_string() } else { names[1].clone() };
                    let when = tr(format!("turn {turn}"), format!("tour {turn}"));
                    // A phone's strip: a short owner and the turn ("You 3", "Ember 4").
                    let first: String = who.split_whitespace().next().unwrap_or("").chars().take(7).collect();
                    let short = format!("{first} · T{turn}");
                    view! {
                        <li class=if side == HUMAN { "rail-group me" } else { "rail-group opp" }>
                            <h3 class="rail-label"><span class="long"><b>{who.clone()}</b><span class="sr-only">" · "</span><span>{when}</span></span><span class="short" aria-hidden="true">{short}</span></h3>
                            <ol>
                                {plays.into_iter().map(|p| {
                                    let key = p.key;
                                    let label = format!("{}. {}", p.title, p.lines.join(". "));
                                    view! {
                                        <li>
                                            <button class="rail-play" data-key=key class:open=move || open.get() == Some(key)
                                                aria-label=label aria-haspopup="dialog"
                                                on:click=move |_| open.set(Some(key))>
                                                <span class="rail-art">{rail_art(p.card)}</span>
                                                <span class=format!("rail-kind k-{}", p.kind) aria-hidden="true">
                                                    <svg class="ic" viewBox="0 0 24 24"><path d=kind_icon(p.kind)/></svg>
                                                </span>
                                                {p.amount.map(|n| view! { <span class="rail-n" aria-hidden="true">{dmg_label(n)}</span> })}
                                            </button>
                                        </li>
                                    }
                                }).collect_view()}
                            </ol>
                        </li>
                    }
                }).collect_view()}
            </ol>
            <button class="rail-log" aria-haspopup="dialog" on:click=move |_| ui.log_open.set(true)>
                <svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 6h11M9 12h11M9 18h11M4.5 6h.01M4.5 12h.01M4.5 18h.01"/></svg>
                <span>{tr("Full log", "Journal")}</span>
            </button>
        </nav>
    }
}

/// What one play did: the card, its target and the engine's sentences.
#[component]
pub fn PlayPopup() -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let open = expect_context::<RwSignal<Option<u32>>>();
    let names = ctl.setup.get_value().map(|s| s.names).unwrap_or_default();
    let play = Memo::new(move |_| {
        let key = open.get()?;
        ctl.history
            .with(|h| h.iter().find(|p| p.key == key).cloned())
    });
    let close = NodeRef::<leptos::html::Button>::new();
    // A choice you must make closes the detail: never two sheets at once.
    Effect::new(move |_| {
        let asked = ctl.shown.with(|g| {
            g.as_ref()
                .and_then(|g| {
                    g.view(HUMAN)
                        .prompt
                        .map(|p| crate::controls::is_modal(&p.kind))
                })
                .unwrap_or(false)
        });
        if asked {
            open.set(None);
        }
    });
    Effect::new(move |_| {
        if play.with(Option::is_some) {
            if let Some(b) = close.get() {
                let _ = b.focus();
            }
        }
    });
    move || {
        let p = play.get()?;
        let who = if p.side == HUMAN {
            tr("You", "Vous").to_string()
        } else {
            names[1].clone()
        };
        Some(view! {
            <div class="sheet-scrim" on:click=move |_| open.set(None)></div>
            <section class="play-pop" role="dialog" aria-labelledby="play-title"
                on:keydown=move |e| { if e.key() == "Escape" { open.set(None) } }>
                <header>
                    <p class="play-who">{tr(format!("{who} · turn {}", p.turn), format!("{who} · tour {}", p.turn))}</p>
                    <button class="icon-btn" node_ref=close aria-label=tr("Close", "Fermer") on:click=move |_| open.set(None)>
                        <svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><path d="M6 6l12 12M18 6 6 18"/></svg>
                    </button>
                </header>
                <div class="play-cards" aria-hidden="true">
                    <span class="play-face"><CardFace id=p.card /></span>
                    {p.target.map(|t| view! {
                        <svg class="ic play-arrow" viewBox="0 0 24 24"><path d="M4 12h15M14 6.5l5.5 5.5-5.5 5.5"/></svg>
                        <span class="play-face" class:hit=p.amount.is_some()>
                            <CardFace id=t />
                            {p.amount.map(|n| view! { <span class="play-dmg">{dmg_label(n)}</span> })}
                        </span>
                    })}
                </div>
                <h3 class="play-title" id="play-title">{p.title.clone()}</h3>
                <ul class="play-lines">{p.lines.iter().map(|l| view! { <li>{l.clone()}</li> }).collect_view()}</ul>
            </section>
        })
    }
}

/// The whole duel's plays for the log drawer: one block per turn, newest
/// first, each play a picture, its title and what came of it.
#[component]
pub fn LogPlays() -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let names = ctl.setup.get_value().map(|s| s.names).unwrap_or_default();
    let all = Memo::new(move |_| {
        // Newest turn first, like the rail; a turn's plays in the order they happened.
        ctl.history.with(|h| groups(h, usize::MAX))
    });
    view! {
        {move || all.with(Vec::is_empty).then(|| view! {
            <p class="fine">{tr("No plays yet.", "Aucun coup pour l'instant.")}</p>
        })}
        <ol class="log-turns">
            {move || all.get().into_iter().map(|(side, turn, plays)| {
                let who = if side == HUMAN { tr("You", "Vous").to_string() } else { names[1].clone() };
                view! {
                    <li class=if side == HUMAN { "log-turn me" } else { "log-turn opp" }>
                        <h3>{tr(format!("Turn {turn} · {who}"), format!("Tour {turn} · {who}"))}</h3>
                        <ol>
                            {plays.into_iter().map(|p| view! {
                                <li class="log-play">
                                    <span class="log-thumb" aria-hidden="true">
                                        <span class="rail-art">{rail_art(p.card)}</span>
                                        <span class=format!("rail-kind k-{}", p.kind)>
                                            <svg class="ic" viewBox="0 0 24 24"><path d=kind_icon(p.kind)/></svg>
                                        </span>
                                        {p.amount.map(|n| view! { <span class="rail-n">{dmg_label(n)}</span> })}
                                    </span>
                                    <div>
                                        <b>{p.title.clone()}</b>
                                        {p.lines.iter().map(|l| view! { <p>{l.clone()}</p> }).collect_view()}
                                    </div>
                                </li>
                            }).collect_view()}
                        </ol>
                    </li>
                }
            }).collect_view()}
        </ol>
    }
}
