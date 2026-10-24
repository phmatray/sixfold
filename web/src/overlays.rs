//! Everything drawn over or beside the table: coin, callouts, inspector,
//! battle log, discard viewer and the game-over screen.

use leptos::prelude::*;
use pkmntcg_engine::{card, Color, Event};

use crate::art::glyph_svg;
use crate::board::{find_mon, slot_name, status_class};
use crate::cards::{energy_pips, CardDetails, CardFace};
use crate::ctl::{Ctl, HUMAN};
use crate::{Inspect, Screen, Ui};

#[component]
pub fn CoinOverlay() -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let coin = Memo::new(move |_| ctl.fx.with(|f| f.coin.clone()));
    move || {
        coin.get().map(|c| {
            let who = if c.player == HUMAN { "You" } else { "Opponent" };
            let side = if c.heads { "heads" } else { "tails" };
            view! {
                <div class="coin-overlay" role="status" aria-live="assertive" aria-label=format!("{who} flipped a coin: {side}")>
                    <p class="coin-caption">{c.caption}</p>
                    <div class="coin-stage">
                        <div class=format!("coin land-{side}")>
                            <div class="coin-face heads" inner_html=glyph_svg(Color::Colorless)></div>
                            <div class="coin-face tails"><span class="tails-ring"></span></div>
                        </div>
                    </div>
                    <p class="coin-result">{format!("{who} flipped {}", if c.heads { "Heads" } else { "Tails" })}</p>
                </div>
            }
        })
    }
}

#[component]
pub fn CalloutOverlay() -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let callout = Memo::new(move |_| ctl.fx.with(|f| f.callout.clone()));
    move || {
        callout.get().map(|c| {
            view! {
                <div class=if c.mine { "callout mine" } else { "callout theirs" } role="status" aria-live="polite">
                    {c.card.map(|id| view! { <div class="callout-card"><CardFace id=id /></div> })}
                    <div class="callout-text">
                        <strong>{c.title}</strong>
                        <span>{c.sub}</span>
                    </div>
                </div>
            }
        })
    }
}

/// Full card text. In the sidebar on wide screens; a sheet on small ones.
#[component]
pub fn Inspector() -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let ui = expect_context::<Ui>();
    let in_play = Memo::new(move |_| {
        let at = ui.inspect.get().and_then(|i| i.at)?;
        ctl.shown
            .with(|g| g.as_ref().and_then(|g| find_mon(g, at.0, at.1)))
            .map(|m| (at.0, m))
    });
    let shown_card = move || {
        in_play
            .with(|m| m.as_ref().map(|(_, m)| m.top))
            .or_else(|| ui.inspect.get().map(|i| i.card))
    };
    let close = NodeRef::<leptos::html::Button>::new();
    Effect::new(move |_| {
        if ui.sheet.get() {
            if let Some(b) = close.get() {
                let _ = b.focus();
            }
        }
    });
    view! {
        <aside class="inspector" class:open=move || ui.sheet.get() aria-label="Card details">
            <button class="btn ghost close" node_ref=close on:click=move |_| ui.sheet.set(false) aria-label="Close card details">"Close"</button>
            {move || match shown_card() {
                None => view! { <p class="empty-note">"Hover a card, or press and hold it, to read it here."</p> }.into_any(),
                Some(id) => view! {
                    <div class="inspect-body">
                        <div class="inspect-card" aria-hidden="true"><CardFace id=id /></div>
                        {move || in_play.get().map(|(player, m)| view! {
                            <div class="inplay">
                                <h4>{slot_name(player, m.slot)}</h4>
                                <p>{format!("{} of {} HP", m.hp, m.max)}
                                    {(m.stack.len() > 1).then(|| format!(", evolved from {}", card(m.stack[m.stack.len() - 2]).name))}
                                </p>
                                <div class="energy-row">
                                    {if m.energy.is_empty() {
                                        view! { <span class="none">"No energy attached"</span> }.into_any()
                                    } else {
                                        m.energy.iter().map(|&e| energy_pips(e)).collect_view().into_any()
                                    }}
                                </div>
                                <div class="badges">
                                    {m.statuses.iter().map(|&s| view! {
                                        <span class=format!("badge status {}", status_class(s))>{s.to_string()}</span>
                                    }).collect_view()}
                                    {m.trainers.iter().map(|&t| view! { <span class="badge tool">{card(t).name}</span> }).collect_view()}
                                </div>
                                {(!m.effects.is_empty()).then(|| view! {
                                    <ul class="effects">{m.effects.iter().map(|e| view! { <li>{e.clone()}</li> }).collect_view()}</ul>
                                })}
                                {(m.power == Some(false)).then(|| view! { <p class="note">"Its Power is not working right now."</p> })}
                            </div>
                        })}
                        <CardDetails id=id />
                    </div>
                }.into_any(),
            }}
        </aside>
    }
}

#[component]
pub fn LogPanel() -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let ui = expect_context::<Ui>();
    let list = NodeRef::<leptos::html::Ol>::new();
    Effect::new(move |_| {
        ctl.log.track();
        if let Some(el) = list.get() {
            el.set_scroll_top(el.scroll_height());
        }
    });
    view! {
        <section class="log" class:open=move || ui.log_open.get() aria-label="Battle log">
            <h2>"Battle log"</h2>
            <ol class="log-list" node_ref=list aria-live="polite" aria-relevant="additions">
                <For
                    each=move || ctl.log.get()
                    key=|l| l.key
                    children=move |l| view! { <li class=format!("l-{}", l.kind)>{l.text}</li> }
                />
            </ol>
        </section>
    }
}

#[component]
pub fn DiscardViewer() -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let ui = expect_context::<Ui>();
    move || {
        let player = ui.discard.get()?;
        let cards = ctl
            .shown
            .with(|g| {
                g.as_ref()
                    .map(|g| g.view(HUMAN).sides[player].discard.to_vec())
            })
            .unwrap_or_default();
        let whose = if player == HUMAN {
            "Your"
        } else {
            "Opponent's"
        };
        let close = move |_| ui.discard.set(None);
        Some(view! {
            <div class="scrim" on:keydown=move |e| { if e.key() == "Escape" { ui.discard.set(None) } }>
                <div class="dialog wide" role="dialog" aria-modal="true" aria-labelledby="discard-title">
                    <h2 id="discard-title">{format!("{whose} discard pile")}</h2>
                    <p class="dialog-sub">{if cards.is_empty() { "Empty.".to_string() } else { format!("{} cards, newest last.", cards.len()) }}</p>
                    <div class="discard-grid">
                        {cards.into_iter().map(|id| view! {
                            <button class="discard-card" aria-label=card(id).name on:click=move |_| ui.open(Inspect { card: id, at: None })>
                                <CardFace id=id />
                            </button>
                        }).collect_view()}
                    </div>
                    <div class="dialog-foot">
                        <button class="btn" on:click=close autofocus>"Close"</button>
                    </div>
                </div>
            </div>
        })
    }
}

#[component]
pub fn GameOver() -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let screen = expect_context::<RwSignal<Screen>>();
    let result = Memo::new(move |_| {
        ctl.shown.with(|g| {
            let g = g.as_ref()?;
            let v = g.view(HUMAN);
            let w = v.winner?;
            let left = v
                .sides
                .clone()
                .map(|s| s.prizes.iter().filter(|p| **p).count());
            let loser = 1 - w;
            let you = |p: usize| if p == HUMAN { "You" } else { "Your opponent" };
            let deck_out = g
                .log()
                .iter()
                .any(|e| matches!(e, Event::DeckOut { player } if *player == loser));
            let why = if deck_out {
                format!("{} couldn't draw a card.", you(loser))
            } else if left[w] == 0 {
                format!("{} took every prize.", you(w))
            } else {
                format!("{} had no creature left in play.", you(loser))
            };
            Some((w, v.sudden_death, left, why))
        })
    });
    let rematch = move |_| {
        if let Some(setup) = ctl.setup.get_value() {
            ctl.start(setup, crate::new_seed());
        }
    };
    move || {
        let (winner, sudden, left, why) = result.get()?;
        let won = winner == HUMAN;
        Some(view! {
            <div class="scrim end">
                <div class=if won { "dialog result won" } else { "dialog result lost" } role="dialog" aria-modal="true" aria-labelledby="result-title">
                    <h2 id="result-title">{if won { "You win" } else { "You lose" }}</h2>
                    <p class="dialog-sub">
                        {why} " "
                        {format!("Prizes left: you {}, opponent {}.", left[HUMAN], left[1 - HUMAN])}
                        {sudden.then_some(" Decided in sudden death.")}
                    </p>
                    <div class="dialog-foot">
                        <button class="btn primary" on:click=rematch autofocus>"Rematch"</button>
                        <button class="btn ghost" on:click=move |_| { ctl.stop(); screen.set(Screen::Title) }>"Change decks"</button>
                    </div>
                </div>
            </div>
        })
    }
}
