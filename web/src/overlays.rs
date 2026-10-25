//! Everything drawn over or beside the table: coin, callouts, inspector,
//! battle log, discard viewer and the game-over screen.

use leptos::prelude::*;
use sixfold_engine::campaign;
use sixfold_engine::endgame::{self, RungResult, CLEAR_PACKS, GAUNTLET};
use sixfold_engine::{card, fr, tr, Color, Event};

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
            let result = match (c.player == HUMAN, c.heads, fr()) {
                (true, true, false) => "You flipped Heads",
                (true, false, false) => "You flipped Tails",
                (false, true, false) => "Opponent flipped Heads",
                (false, false, false) => "Opponent flipped Tails",
                (true, true, true) => "Vous obtenez face",
                (true, false, true) => "Vous obtenez pile",
                (false, true, true) => "L'adversaire obtient face",
                (false, false, true) => "L'adversaire obtient pile",
            };
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
                    <p class="coin-result">{result}</p>
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
        <aside class="inspector" class:open=move || ui.sheet.get() aria-label=tr("Card details", "Détails de la carte")>
            <button class="btn ghost close" node_ref=close on:click=move |_| ui.sheet.set(false) aria-label=tr("Close card details", "Fermer les détails de la carte")>{tr("Close", "Fermer")}</button>
            {move || match shown_card() {
                None => view! { <p class="empty-note">{tr("Hover or focus a card, or press and hold it, to read it here.", "Survolez ou sélectionnez une carte, ou maintenez-la appuyée, pour la lire ici.")}</p> }.into_any(),
                Some(id) => view! {
                    <div class="inspect-body">
                        <div class="inspect-card" aria-hidden="true"><CardFace id=id /></div>
                        {move || in_play.get().map(|(player, m)| view! {
                            <div class="inplay">
                                <h3>{slot_name(player, m.slot)}</h3>
                                <p>{tr(format!("{} of {} HP", m.hp, m.max), format!("{} PV sur {}", m.hp, m.max))}
                                    {(m.stack.len() > 1).then(|| {
                                        let from = card(m.stack[m.stack.len() - 2]).label();
                                        tr(format!(", evolved from {from}"), format!(", a évolué de {from}"))
                                    })}
                                </p>
                                <div class="energy-row">
                                    {if m.energy.is_empty() {
                                        view! { <span class="none">{tr("No energy attached", "Aucune énergie attachée")}</span> }.into_any()
                                    } else {
                                        m.energy.iter().map(|&e| energy_pips(e)).collect_view().into_any()
                                    }}
                                </div>
                                <div class="badges">
                                    {m.statuses.iter().map(|&s| view! {
                                        <span class=format!("badge status {}", status_class(s))>{s.to_string()}</span>
                                    }).collect_view()}
                                    {m.trainers.iter().map(|&t| view! { <span class="badge tool">{card(t).label()}</span> }).collect_view()}
                                </div>
                                {(!m.effects.is_empty()).then(|| view! {
                                    <ul class="effects">{m.effects.iter().map(|e| view! { <li>{e.clone()}</li> }).collect_view()}</ul>
                                })}
                                {(m.power == Some(false)).then(|| view! { <p class="note">{tr("Its Power is not working right now.", "Son Pouvoir est inactif pour le moment.")}</p> })}
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
        <section class="log" class:open=move || ui.log_open.get() aria-label=tr("Battle log", "Journal du duel")>
            <h2>{tr("Battle log", "Journal du duel")}</h2>
            {move || ctl.log.with(Vec::is_empty).then(|| view! {
                <p class="empty-note">{tr("Every draw, attack and coin flip shows up here.", "Chaque pioche, attaque et pile ou face s'affiche ici.")}</p>
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
        let title = match (player == HUMAN, fr()) {
            (true, false) => "Your discard pile",
            (false, false) => "Opponent's discard pile",
            (true, true) => "Votre défausse",
            (false, true) => "Défausse adverse",
        };
        let close = move |_| ui.discard.set(None);
        Some(view! {
            <div class="scrim" on:keydown=move |e| { if e.key() == "Escape" { ui.discard.set(None) } }>
                <div class="dialog wide" role="dialog" aria-modal="true" aria-labelledby="discard-title">
                    <h2 id="discard-title">{title}</h2>
                    <p class="dialog-sub">{if cards.is_empty() {
                        tr("Empty.", "Vide.").to_string()
                    } else {
                        let n = crate::board::cards(cards.len());
                        tr(format!("{n}, newest last."), format!("{n}, la plus récente en dernier."))
                    }}</p>
                    <div class="discard-grid">
                        {cards.into_iter().map(|id| view! {
                            <button class="discard-card" aria-label=crate::cards::face_text(id) on:click=move |_| ui.open(Inspect { card: id, at: None })>
                                <CardFace id=id />
                            </button>
                        }).collect_view()}
                    </div>
                    <div class="dialog-foot">
                        <button class="btn" on:click=close autofocus>{tr("Close", "Fermer")}</button>
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
            let deck_out = g
                .log()
                .iter()
                .any(|e| matches!(e, Event::DeckOut { player } if *player == loser));
            let why = match (deck_out, left[w] == 0, loser == HUMAN) {
                (true, _, true) => tr("You couldn't draw a card.", "Vous ne pouviez plus piocher."),
                (true, _, false) => tr(
                    "Your opponent couldn't draw a card.",
                    "Votre adversaire ne pouvait plus piocher.",
                ),
                (false, true, false) => tr(
                    "You took every prize.",
                    "Vous avez pris toutes vos Récompenses.",
                ),
                (false, true, true) => tr(
                    "Your opponent took every prize.",
                    "Votre adversaire a pris toutes ses Récompenses.",
                ),
                (false, false, true) => tr(
                    "You had no creature left in play.",
                    "Vous n'aviez plus de créature en jeu.",
                ),
                (false, false, false) => tr(
                    "Your opponent had no creature left in play.",
                    "Votre adversaire n'avait plus de créature en jeu.",
                ),
            };
            Some((w, v.sudden_death, left, why))
        })
    });
    // The reward is granted once per match, when its result first shows: a
    // quick duel pays Facets by the AI's level, a campaign duel the rival's
    // reward.
    let profile = expect_context::<RwSignal<sixfold_engine::profile::Profile>>();
    let progress = expect_context::<RwSignal<sixfold_engine::overworld::Progress>>();
    let opponent = ctl
        .setup
        .with_value(|s| s.as_ref().and_then(|s| s.opponent));
    let rival = opponent.and_then(campaign::rival);
    // Whose last word the result shows: a world rival or a post-game one.
    let quoted = ctl
        .setup
        .with_value(|s| s.as_ref().and_then(|s| s.face()))
        .and_then(campaign::rival);
    let duelist = opponent.and_then(sixfold_engine::trainers::trainer);
    let post = ctl.setup.with_value(|s| s.as_ref().and_then(|s| s.post));
    let back = ctl
        .setup
        .with_value(|s| s.as_ref().map_or(Screen::Title, |s| s.back));
    let level = ctl.setup.with_value(|s| s.as_ref().map(|s| s.difficulty));
    let reward = RwSignal::new(None::<u32>);
    let campaign_reward = RwSignal::new(None::<campaign::Reward>);
    let duelist_reward = RwSignal::new(None::<campaign::Reward>);
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
                (None, _) if duelist.is_some() => {
                    let mut got = None;
                    progress.update(|pr| {
                        profile.update(|p| {
                            got = opponent.and_then(|id| campaign::record_opponent(id, won, pr, p))
                        })
                    });
                    duelist_reward.set(got);
                }
                (None, Some(Post::Tutorial)) => {}
                (None, None) => {
                    let mut got = 0;
                    progress.update(|pr| {
                        got = campaign::record_quick_duel(pr, won, level.unwrap_or_default())
                    });
                    reward.set((got > 0).then_some(got));
                }
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
                    <h2 id="result-title">{match (won, fr()) {
                        (true, false) => "You win",
                        (false, false) => "You lose",
                        (true, true) => "Victoire !",
                        (false, true) => "Défaite",
                    }}</h2>
                    <p class="dialog-sub">
                        {why} " "
                        {tr(
                            format!("Prizes left: you {}, opponent {}.", left[HUMAN], left[1 - HUMAN]),
                            format!("Récompenses restantes : vous {}, l'adversaire {}.", left[HUMAN], left[1 - HUMAN]),
                        )}
                        {sudden.then_some(tr(" Decided in sudden death.", " Décidé en mort subite."))}
                    </p>
                    {quoted.filter(|_| won).map(|r| view! {
                        <figure class="rival-quote">
                            <crate::campaign::Portrait id=r.id name=r.name />
                            <blockquote>
                                <p>{r.lines.defeat.get()}</p>
                                <footer>{r.name}</footer>
                            </blockquote>
                        </figure>
                    })}
                    {move || reward.get().map(|n| {
                        let coins = progress.with(|p| p.coins());
                        view! {
                        <div class="reward" role="status">
                            <span class="gem" aria-hidden="true"></span>
                            <p><b>{format!("+⬡ {n}")}</b>
                                <span>{tr(
                                    format!("You have ⬡ {coins}. Spend Facets on packs at the duel hall."),
                                    format!("Vous avez ⬡ {coins}. Dépensez vos Facettes en boosters à la salle des duels."),
                                )}</span></p>
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
                                        (true, true, _) => view! {
                                            <b>{tr(format!("{}'s seal!", r.name), format!("Le sceau de {} !", r.name))}</b>
                                            <span>{tr(
                                                format!("{seals} of {} seals.", campaign::SEALS),
                                                format!("{seals} sceau{} sur {}.", if seals > 1 { "x" } else { "" }, campaign::SEALS),
                                            )}</span>
                                        }.into_any(),
                                        (true, false, true) => view! {
                                            <b>{tr("You beat the Hex Warden!", "Vous avez battu la Gardienne des Six !")}</b>
                                            <span>{tr("Every seal, and the champion's title.", "Tous les sceaux, et le titre de champion.")}</span>
                                        }.into_any(),
                                        (true, ..) => view! { <b>{tr("Rematch won", "Revanche gagnée")}</b> }.into_any(),
                                        _ => view! {
                                            <b>{tr("No reward this time", "Pas de récompense cette fois")}</b>
                                            <span>{tr(format!("Talk to {} again for a rematch.", r.name), format!("Reparlez à {} pour une revanche.", r.name))}</span>
                                        }.into_any(),
                                    }}
                                    {(n > 0).then(|| view! { <span>{format!("+{}", crate::campaign::packs_label(b, n))}</span> })}
                                    {(rw.facets > 0).then(|| view! { <span>{format!("+⬡ {}", rw.facets)}</span> })}
                                    {rw.promo.map(|c| view! { <span>{tr(
                                        format!("+ the promo card {}, now in your collection", card(c).label()),
                                        format!("+ la carte promo {}, ajoutée à votre collection", card(c).label()),
                                    )}</span> })}
                                </p>
                            </div>
                        }
                    })}
                    {move || duelist_reward.get().zip(duelist).map(|(rw, t)| {
                        let coins = progress.with(|p| p.coins());
                        view! {
                            <div class="reward" role="status">
                                {(rw.facets > 0).then(|| view! { <span class="gem" aria-hidden="true"></span> })}
                                <p>{if rw.facets > 0 {
                                    view! {
                                        <b>{format!("+⬡ {}", rw.facets)}</b>
                                        <span>{tr(
                                            format!("{}'s purse. You have ⬡ {coins}.", t.name),
                                            format!("La bourse de {}. Vous avez ⬡ {coins}.", t.name),
                                        )}</span>
                                    }.into_any()
                                } else if rw.won {
                                    view! {
                                        <b>{tr("Won again", "Encore gagné")}</b>
                                        <span>{tr(format!("{} pays a purse only once.", t.name), format!("{} ne paie sa bourse qu'une fois.", t.name))}</span>
                                    }.into_any()
                                } else {
                                    view! {
                                        <b>{tr("No purse this time", "Pas de bourse cette fois")}</b>
                                        <span>{tr(format!("Talk to {} for another go.", t.name), format!("Reparlez à {} pour retenter votre chance.", t.name))}</span>
                                    }.into_any()
                                }}</p>
                            </div>
                        }
                    })}
                    {move || rung_result.get().map(|rr| {
                        let (b, n) = rr.packs;
                        view! {
                            <div class="reward" role="status">
                                <p>
                                    {match (rr.won, rr.cleared) {
                                        (true, true) => view! {
                                            <b>{tr("Gauntlet cleared!", "Épreuve réussie !")}</b>
                                            <span>{tr(
                                                format!("All {} rivals in a row. +{CLEAR_PACKS} Wild Packs.", GAUNTLET.len()),
                                                format!("Les {} rivaux d'affilée. +{CLEAR_PACKS} Boosters Sauvage.", GAUNTLET.len()),
                                            )}</span>
                                        }.into_any(),
                                        (true, false) => view! { <b>{tr(
                                            format!("Rung {} of {} cleared", rr.beaten, GAUNTLET.len()),
                                            format!("Palier {} sur {} franchi", rr.beaten, GAUNTLET.len()),
                                        )}</b> }.into_any(),
                                        _ => view! {
                                            <b>{tr("The run ends here", "La série s'arrête ici")}</b>
                                            <span>{tr("Start a new Gauntlet from the ladder.", "Relancez l'Épreuve depuis le tableau de la Gardienne.")}</span>
                                        }.into_any(),
                                    }}
                                    {(n > 0).then(|| view! { <span>{format!("+{}", crate::campaign::packs_label(b, n))}</span> })}
                                </p>
                            </div>
                        }
                    })}
                    {(post.is_some_and(|p| matches!(p, Post::Daily(_))) && won).then(|| view! {
                        <div class="reward" role="status">
                            <p>{move || match daily_pack.get() {
                                Some(b) => view! {
                                    <b>{tr("Daily duel won", "Duel du jour gagné")}</b>
                                    <span>{tr(
                                        format!("+{}. A new duel comes at midnight.", crate::campaign::packs_label(b, 1)),
                                        format!("+{}. Un nouveau duel arrive à minuit.", crate::campaign::packs_label(b, 1)),
                                    )}</span>
                                }.into_any(),
                                None => view! {
                                    <b>{tr("Daily duel won", "Duel du jour gagné")}</b>
                                    <span>{tr("Today's pack was already claimed.", "Le booster du jour a déjà été réclamé.")}</span>
                                }.into_any(),
                            }}</p>
                        </div>
                    })}
                    {match (opponent, post) {
                        (Some(_), _) => view! {
                            <div class="dialog-foot">
                                {move || if champion_now() {
                                    view! {
                                        <button class="btn primary" autofocus on:click=move |_| { ctl.stop(); screen.set(Screen::Credits(true)) }>{tr("Roll the credits", "Voir le générique")}</button>
                                    }.into_any()
                                } else {
                                    view! {
                                        <button class="btn primary" autofocus on:click=move |_| { ctl.stop(); screen.set(Screen::World) }>{tr("Back to the world", "Retour au monde")}</button>
                                    }.into_any()
                                }}
                            </div>
                        }.into_any(),
                        (None, Some(p)) => view! {
                            <div class="dialog-foot">
                                {move || (p == Post::Gauntlet && rung_result.get().is_some_and(|r| r.won && !r.cleared)).then(|| view! {
                                    <button class="btn" on:click=next_rung>{tr("Next rung", "Palier suivant")}</button>
                                })}
                                {matches!(p, Post::Daily(_)).then(|| view! {
                                    <button class="btn" on:click=rematch>{tr("Play it again", "Rejouer")}</button>
                                })}
                                <button class="btn primary" autofocus on:click=move |_| { ctl.stop(); screen.set(back) }>{tr("Back", "Retour")}</button>
                            </div>
                        }.into_any(),
                        (None, None) => view! {
                    <div class="dialog-foot">
                        {won.then(|| view! {
                            <button class="btn" on:click=move |_| { ctl.stop(); screen.set(Screen::Packs) }>{tr("Open packs", "Ouvrir des boosters")}</button>
                        })}
                        <button class="btn primary" on:click=rematch autofocus>{tr("Rematch", "Revanche")}</button>
                        <button class="btn ghost" on:click=move |_| { ctl.stop(); screen.set(Screen::Duel) }>{tr("Change decks", "Changer de deck")}</button>
                    </div>
                        }.into_any(),
                    }}
                </div>
            </div>
        })
    }
}

/// The rival's face in the duel header, and the speech bubble their lines
/// come in (see `Ctl::say`): it goes after a few seconds or on a tap, and a
/// polite live region reads it out. For rivals (world and post-game) and
/// route duelists.
#[component]
pub fn RivalFace() -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let Some(id) = ctl.setup.with_value(|s| s.as_ref().and_then(|s| s.face())) else {
        return ().into_any();
    };
    let name = campaign::opponent(id).map_or(id, |o| o.name);
    Effect::new(move |_| {
        if let Some(s) = ctl.said.get() {
            // Long enough to read: 3 s plus a little per character.
            let ms = 3000 + 45 * s.text.chars().count() as u64;
            set_timeout(
                move || {
                    if ctl.said.get_untracked().is_some_and(|n| n.key == s.key) {
                        ctl.said.set(None);
                    }
                },
                std::time::Duration::from_millis(ms.min(9000)),
            );
        }
    });
    let ko = move || ctl.said.with(|s| s.is_some_and(|s| s.ko));
    view! {
        <span class="opp-face" class:shake=ko>
            <crate::campaign::Portrait id=id name=name />
            <span class="rival-say-live" aria-live="polite">
                {move || ctl.said.get().map(|s| view! {
                    <button type="button" class="rival-say" on:click=move |_| ctl.said.set(None)
                        title=tr("Dismiss", "Masquer")>
                        <b>{name}</b>" "{s.text}
                    </button>
                })}
            </span>
        </span>
    }
    .into_any()
}
