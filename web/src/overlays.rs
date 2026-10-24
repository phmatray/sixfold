//! Everything drawn over or beside the table: coin, callouts, inspector,
//! battle log, discard viewer and the game-over screen.

use leptos::prelude::*;
use sixfold_engine::campaign;
use sixfold_engine::endgame::{self, RungResult, CLEAR_PACKS, GAUNTLET};
use sixfold_engine::{card, Color, Event};

use crate::art::glyph_svg;
use crate::board::{find_mon, slot_name, status_class};
use crate::cards::{energy_pips, CardDetails, CardFace};
use crate::ctl::{Ctl, Post, HUMAN};
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
                <div class="coin-overlay" aria-hidden="true">
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
                <div class=if c.mine { "callout mine" } else { "callout theirs" } aria-hidden="true">
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
                None => view! { <p class="empty-note">"Hover or focus a card, or press and hold it, to read it here."</p> }.into_any(),
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
            {move || ctl.log.with(Vec::is_empty).then(|| view! {
                <p class="empty-note">"Every draw, attack and coin flip shows up here."</p>
            })}
            <ol class="log-list" node_ref=list>
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
                    <p class="dialog-sub">{if cards.is_empty() { "Empty.".to_string() } else { format!("{}, newest last.", crate::board::cards(cards.len())) }}</p>
                    <div class="discard-grid">
                        {cards.into_iter().map(|id| view! {
                            <button class="discard-card" aria-label=crate::cards::face_text(id) on:click=move |_| ui.open(Inspect { card: id, at: None })>
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
    // The reward is granted once per match, when its result first shows. A
    // campaign duel gives the rival's reward instead of the Wild pack.
    let profile = expect_context::<RwSignal<sixfold_engine::profile::Profile>>();
    let progress = expect_context::<RwSignal<sixfold_engine::overworld::Progress>>();
    let rival = ctl
        .setup
        .with_value(|s| s.as_ref().and_then(|s| s.rival))
        .and_then(campaign::rival);
    let post = ctl.setup.with_value(|s| s.as_ref().and_then(|s| s.post));
    let back = ctl
        .setup
        .with_value(|s| s.as_ref().map_or(Screen::Title, |s| s.back));
    let reward = RwSignal::new(None::<usize>);
    let campaign_reward = RwSignal::new(None::<campaign::Reward>);
    let rung_result = RwSignal::new(None::<RungResult>);
    // The daily duel's pack, if this win claimed it.
    let daily_pack = RwSignal::new(None::<usize>);
    Effect::new(move |_| {
        if let Some((winner, ..)) = result.get() {
            if !ctl.take_result() {
                return;
            }
            let won = winner == HUMAN;
            match (rival, post) {
                (None, Some(Post::Gauntlet)) => {
                    let mut got = None;
                    progress
                        .update(|pr| profile.update(|p| got = endgame::record_rung(pr, p, won)));
                    rung_result.set(got);
                }
                (None, Some(Post::Daily(day))) => {
                    let mut got = None;
                    progress.update(|pr| {
                        profile.update(|p| got = endgame::record_daily(pr, p, day, won))
                    });
                    daily_pack.set(got);
                }
                (Some(r), _) => {
                    let mut got = None;
                    progress.update(|pr| {
                        profile.update(|p| got = Some(campaign::record(r, won, pr, p)))
                    });
                    if got.as_ref().is_some_and(|g| g.seal || g.first) {
                        // After the win fanfare.
                        set_timeout(
                            || crate::audio::sfx("seal", 0.0),
                            std::time::Duration::from_millis(1200),
                        );
                    }
                    campaign_reward.set(got);
                }
                (None, None) => reward.set(profile.try_update(|p| p.record_duel(won)).flatten()),
            }
        }
    });
    // A rematch deals anew; the daily duel deals the same game again.
    let rematch = move |_| {
        if let Some(setup) = ctl.setup.get_value() {
            let seed = match setup.post {
                Some(Post::Daily(day)) => endgame::daily(day).seed,
                _ => crate::new_seed(),
            };
            ctl.start(setup, seed);
        }
    };
    // After a Gauntlet win: the next rung, with the same deck.
    let next_rung = move |_| {
        let Some(s) = ctl.setup.get_value() else {
            return;
        };
        if let Some(rung) = progress.with_untracked(endgame::next_rung) {
            let [name, _] = s.names;
            let [cards, _] = s.decks;
            let setup = crate::campaign::gauntlet_setup(rung, name, cards, s.back);
            ctl.start(setup, crate::new_seed());
        }
    };
    // The champion's first win rolls the credits on the way back to the world.
    let champion_now = move || {
        campaign_reward
            .get()
            .zip(rival)
            .is_some_and(|(rw, r)| rw.won && rw.first && !r.seal)
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
                    {move || reward.get().map(|b| {
                        let packs = profile.with(|p| p.wallet.total());
                        view! {
                        <div class="reward" role="status">
                            <span class="pack mini p-wild" aria-hidden="true">
                                <span class="pack-crimp"></span><span class="pack-crimp bottom"></span>
                            </span>
                            <p><b>{format!("+1 {}", sixfold_engine::booster::BOOSTERS[b].name)}</b>
                                <span>{format!("You have {packs} to open.", packs = if packs == 1 { "1 pack".to_string() } else { format!("{packs} packs") })}</span></p>
                        </div>
                    }})}
                    {move || campaign_reward.get().zip(rival).map(|(rw, r)| {
                        let seals = progress.with(campaign::seals);
                        let (b, n) = rw.packs;
                        view! {
                            <div class="reward campaign-reward" role="status">
                                {if rw.seal {
                                    view! { <crate::campaign::Seal r=r earned=true /> }.into_any()
                                } else if let Some(c) = rw.promo {
                                    // The Warden's promo: show the card itself.
                                    view! { <span class="reward-card" aria-hidden="true"><crate::cards::CardFace id=c /></span> }.into_any()
                                } else {
                                    view! { <span class=format!("pack mini p-{}", crate::packs::pack_theme(b)) aria-hidden="true">
                                        <span class="pack-crimp"></span><span class="pack-crimp bottom"></span>
                                    </span> }.into_any()
                                }}
                                <p>
                                    {match (rw.won, rw.seal, rw.first) {
                                        (true, true, _) => view! { <b>{format!("{}'s seal!", r.name)}</b><span>{format!("{seals} of {} seals.", campaign::SEALS)}</span> }.into_any(),
                                        (true, false, true) => view! { <b>"You beat the Hex Warden!"</b><span>"Every seal, and the champion's title."</span> }.into_any(),
                                        (true, ..) => view! { <b>"Rematch won"</b> }.into_any(),
                                        _ => view! { <b>"No reward this time"</b><span>{format!("Talk to {} again for a rematch.", r.name)}</span> }.into_any(),
                                    }}
                                    {(n > 0).then(|| view! { <span>{format!("+{}", crate::campaign::packs_label(b, n))}</span> })}
                                    {rw.promo.map(|c| view! { <span>{format!("+ the promo card {}, now in your collection", card(c).name)}</span> })}
                                </p>
                            </div>
                        }
                    })}
                    {move || rung_result.get().map(|rr| {
                        let (b, n) = rr.packs;
                        view! {
                            <div class="reward" role="status">
                                <p>
                                    {match (rr.won, rr.cleared) {
                                        (true, true) => view! { <b>"Gauntlet cleared!"</b><span>{format!("All {} rivals in a row. +{CLEAR_PACKS} Wild Packs.", GAUNTLET.len())}</span> }.into_any(),
                                        (true, false) => view! { <b>{format!("Rung {} of {} cleared", rr.beaten, GAUNTLET.len())}</b> }.into_any(),
                                        _ => view! { <b>"The run ends here"</b><span>"Start a new Gauntlet from the ladder."</span> }.into_any(),
                                    }}
                                    {(n > 0).then(|| view! { <span>{format!("+{}", crate::campaign::packs_label(b, n))}</span> })}
                                </p>
                            </div>
                        }
                    })}
                    {(post.is_some_and(|p| matches!(p, Post::Daily(_))) && won).then(|| view! {
                        <div class="reward" role="status">
                            <p>{move || match daily_pack.get() {
                                Some(b) => view! { <b>"Daily duel won"</b><span>{format!("+{}. A new duel comes at midnight.", crate::campaign::packs_label(b, 1))}</span> }.into_any(),
                                None => view! { <b>"Daily duel won"</b><span>"Today's pack was already claimed."</span> }.into_any(),
                            }}</p>
                        </div>
                    })}
                    {match (rival, post) {
                        (Some(_), _) => view! {
                            <div class="dialog-foot">
                                {move || if champion_now() {
                                    view! {
                                        <button class="btn primary" autofocus on:click=move |_| { ctl.stop(); screen.set(Screen::Credits(true)) }>"Roll the credits"</button>
                                    }.into_any()
                                } else {
                                    view! {
                                        <button class="btn primary" autofocus on:click=move |_| { ctl.stop(); screen.set(Screen::World) }>"Back to the world"</button>
                                    }.into_any()
                                }}
                            </div>
                        }.into_any(),
                        (None, Some(p)) => view! {
                            <div class="dialog-foot">
                                {move || (p == Post::Gauntlet && rung_result.get().is_some_and(|r| r.won && !r.cleared)).then(|| view! {
                                    <button class="btn" on:click=next_rung>"Next rung"</button>
                                })}
                                {matches!(p, Post::Daily(_)).then(|| view! {
                                    <button class="btn" on:click=rematch>"Play it again"</button>
                                })}
                                <button class="btn primary" autofocus on:click=move |_| { ctl.stop(); screen.set(back) }>"Back"</button>
                            </div>
                        }.into_any(),
                        (None, None) => view! {
                    <div class="dialog-foot">
                        {won.then(|| view! {
                            <button class="btn" on:click=move |_| { ctl.stop(); screen.set(Screen::Packs) }>"Open packs"</button>
                        })}
                        <button class="btn primary" on:click=rematch autofocus>"Rematch"</button>
                        <button class="btn ghost" on:click=move |_| { ctl.stop(); screen.set(Screen::Duel) }>"Change decks"</button>
                    </div>
                        }.into_any(),
                    }}
                </div>
            </div>
        })
    }
}
