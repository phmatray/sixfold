//! Everything drawn over or beside the table: coin, callouts, inspector,
//! battle log, discard viewer and the game-over screen.

use leptos::prelude::*;
use sixfold_engine::campaign;
use sixfold_engine::endgame::{self, RungResult, CLEAR_PACKS, GAUNTLET};
use sixfold_engine::{card, fr, tr, Color, Event};

use crate::art::glyph_svg;
use crate::board::{find_mon, slot_name, status_class};
use crate::cards::{energy_pips, CardDetails, CardFace};
use crate::ctl::{Ctl, Post, AI, HUMAN};
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
        // Nothing pointed at: the opponent's Active, the card you play against.
        let fallback = move || {
            let acting = ctl
                .fx
                .with(|f| f.callout.as_ref().is_some_and(|c| c.card.is_some()));
            (ui.selected.with(Option::is_none) && !acting).then(|| {
                ctl.shown.with(|g| {
                    g.as_ref()
                        .and_then(|g| g.view(HUMAN).sides[AI].active.map(|a| (AI, a.uid)))
                })
            })?
        };
        let at = match ui.inspect.get() {
            Some(i) => i.at?,
            None => fallback().filter(|_| ctl.revealed.get())?,
        };
        ctl.shown
            .with(|g| g.as_ref().and_then(|g| find_mon(g, at.0, at.1)))
            .map(|m| (at.0, m))
    });
    // The card under the pointer or focus, else the one being played.
    let shown_card = move || {
        in_play
            .with(|m| m.as_ref().map(|(_, m)| m.top))
            .or_else(|| ui.inspect.get().map(|i| i.card))
            .or_else(|| ui.selected.get().map(|k| k.0))
            .or_else(|| ctl.fx.with(|f| f.callout.as_ref().and_then(|c| c.card)))
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
                        <div class="inspect-card" aria-hidden="true" data-tilt=""><CardFace id=id /></div>
                        {move || in_play.get().map(|(player, m)| view! {
                            <div class="inplay">
                                <h3>{slot_name(player, m.slot)}</h3>
                                <p>{tr(format!("{} of {} HP", m.hp, m.max), format!("{} PV sur {}", m.hp, m.max))}
                                    {(m.stack.len() > 1).then(|| {
                                        let from = card(m.stack[m.stack.len() - 2]).label();
                                        tr(format!(", awakened from {from}"), format!(", s'est éveillé de {from}"))
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

/// The full text log, newest first: for reading back, and for screen readers.
#[component]
pub fn LogDrawer() -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let ui = expect_context::<Ui>();
    let all = RwSignal::new(false);
    let close = NodeRef::<leptos::html::Button>::new();
    let list = NodeRef::<leptos::html::Div>::new();
    Effect::new(move |_| {
        if ui.log_open.get() {
            if let Some(b) = close.get() {
                let _ = b.focus();
            }
        }
    });
    move || {
        ui.log_open.get().then(|| view! {
            <div class="sheet-scrim" on:click=move |_| ui.log_open.set(false)></div>
            <section class="drawer" role="dialog" aria-modal="true" aria-labelledby="log-title"
                on:keydown=move |e| { if e.key() == "Escape" { ui.log_open.set(false) } }>
                <header>
                    <h2 id="log-title">{tr("Duel log", "Journal du duel")}</h2>
                    <button class="icon-btn" node_ref=close aria-label=tr("Close", "Fermer") on:click=move |_| ui.log_open.set(false)>
                        <svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><path d="M6 6l12 12M18 6 6 18"/></svg>
                    </button>
                </header>
                <label class="check">
                    <input type="checkbox" prop:checked=move || all.get() on:change=move |e| all.set(event_target_checked(&e)) />
                    {tr("Every event, as text", "Chaque événement, en texte")}
                </label>
                <div class="log-scroll" node_ref=list>
                    {move || if all.get() {
                        view! {
                            <ol class="log-list">
                                {ctl.log.with(|l| l.iter().map(|l| view! { <li class=format!("l-{}", l.kind)>{l.text.clone()}</li> }).collect_view())}
                            </ol>
                        }.into_any()
                    } else {
                        view! { <crate::history::LogPlays /> }.into_any()
                    }}
                </div>
            </section>
        })
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
                    "You had no Hexon left in play.",
                    "Vous n'aviez plus de Hexon en jeu.",
                ),
                (false, false, false) => tr(
                    "Your opponent had no Hexon left in play.",
                    "Votre adversaire n'avait plus de Hexon en jeu.",
                ),
            };
            // What the cause teaches, for the "why you lost" panel.
            let cause = match (deck_out, left[w] == 0) {
                (true, _) => tr(
                    "Your deck ran out. Play fewer cards that make you draw, and keep your deck at full size.",
                    "Votre pioche s'est vidée. Jouez moins de cartes qui font piocher, et gardez un deck complet.",
                ),
                (false, true) => tr(
                    "They knocked out six of your Hexons. Spread energy over two Hexons, so one knock-out doesn't send you back to zero.",
                    "Six de vos Hexons ont été mis K.O. Répartissez l'énergie sur deux Hexons, pour qu'un K.O. ne vous ramène pas à zéro.",
                ),
                (false, false) => tr(
                    "Your Active was knocked out with no one on your Bench to take its place. Keep at least one Basic Hexon on your Bench.",
                    "Votre Actif a été mis K.O. sans personne sur le Banc pour le remplacer. Gardez toujours au moins un Hexon de Base sur le Banc.",
                ),
            };
            Some((w, v.sudden_death, left, why, cause))
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
        .and_then(campaign::opponent)
        .and_then(|o| o.lines.map(|l| (o.id, o.name, l)));
    let duelist = opponent.and_then(sixfold_engine::trainers::trainer);
    let post = ctl.setup.with_value(|s| s.as_ref().and_then(|s| s.post));
    let back = ctl
        .setup
        .with_value(|s| s.as_ref().map_or(Screen::Title, |s| s.back));
    let level = ctl.setup.with_value(|s| s.as_ref().map(|s| s.difficulty));
    let reward = RwSignal::new(None::<u32>);
    // A loss pays Facets per prize taken (campaign::CONSOLATION_PER_PRIZE).
    let consolation = RwSignal::new(None::<u32>);
    let start_prizes = ctl
        .setup
        .with_value(|s| s.as_ref().map_or(6, |s| usize::from(s.prizes)));
    // The board stays readable behind the result: "Look at the board" hides it.
    let peek = RwSignal::new(false);
    let mags = RwSignal::new(false);
    let lesson = loss_lesson(ctl);
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
            let taken = result
                .with_untracked(|r| r.as_ref().map(|r| start_prizes.saturating_sub(r.2[HUMAN])))
                .unwrap_or(0);
            if !won && !matches!(post, Some(Post::Tutorial | Post::Gauntlet | Post::Daily(_))) {
                let mut got = 0;
                progress.update(|pr| got = campaign::record_consolation(pr, taken));
                consolation.set(Some(got));
            }
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
        let (winner, sudden, left, why, cause) = result.get()?;
        let won = winner == HUMAN;
        if peek.get() {
            return Some(view! {
                <button class="btn primary show-result" on:click=move |_| peek.set(false)>{tr("Show the result", "Voir le résultat")}</button>
            }.into_any());
        }
        let lesson = lesson.clone();
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
                    {(!won).then(|| {
                        let (hints, advice) = lesson.clone();
                        view! {
                            <section class="why" aria-labelledby="why-title">
                                <h3 id="why-title">{tr("Why you lost", "Pourquoi cette défaite")}</h3>
                                <p class="why-cause">{cause}</p>
                                {(!hints.is_empty()).then(|| view! { <h4>{tr("Next time", "La prochaine fois")}</h4> })}
                                {hints.into_iter().map(|h| view! { <p>{h}</p> }).collect_view()}
                                {move || mags.get().then(|| view! {
                                    <ul class="mags">{advice.iter().map(|a| view! { <li>{a.clone()}</li> }).collect_view()}</ul>
                                })}
                            </section>
                        }
                    })}
                    {move || consolation.get().map(|n| {
                        let per = campaign::CONSOLATION_PER_PRIZE;
                        let taken = (n / per).saturating_sub(1);
                        view! {
                        <div class="reward" role="status">
                            <span class="gem" aria-hidden="true"></span>
                            <p><b>{tr(format!("+{n} Facets"), format!("+{n} Facettes"))}</b><span>{match (taken, fr()) {
                                (0, false) => "For playing it out to the end.".to_string(),
                                (0, true) => "Pour avoir joué jusqu'au bout.".to_string(),
                                (t, false) => format!("{per} for playing it out, and {per} for each of the {t} prizes you took."),
                                (t, true) => format!("{per} pour avoir joué jusqu'au bout, et {per} pour chacune des {t} Récompenses prises."),
                            }}</span></p>
                        </div>
                    }                    })}
                    {quoted.filter(|_| won).map(|(id, name, lines)| view! {
                        <figure class="rival-quote">
                            <crate::campaign::Portrait id=id name=name />
                            <blockquote>
                                <p>{lines.defeat.get()}</p>
                                <footer>{name}</footer>
                            </blockquote>
                        </figure>
                    })}
                    {move || reward.get().map(|n| {
                        let coins = progress.with(|p| p.coins());
                        view! {
                        <div class="reward" role="status">
                            <span class="gem" aria-hidden="true"></span>
                            <p><b>{tr(format!("+{n} Facets"), format!("+{n} Facettes"))}</b>
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
                                    view! { <span class="pack-mini"><crate::packs::PackArt b=b /></span> }.into_any()
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
                                            <b>{tr("No seal this time", "Pas de sceau cette fois")}</b>
                                            <span>{tr(format!("Talk to {} again for a rematch.", r.name), format!("Reparlez à {} pour une revanche.", r.name))}</span>
                                        }.into_any(),
                                    }}
                                    {(n > 0).then(|| view! { <span>{format!("+{}", crate::campaign::packs_label(b, n))}</span> })}
                                    {(rw.facets > 0).then(|| view! { <span>{tr(format!("+{} Facets", rw.facets), format!("+{} Facettes", rw.facets))}</span> })}
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
                                        <b>{tr(format!("+{} Facets", rw.facets), format!("+{} Facettes", rw.facets))}</b>
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
                                } else if t.story() && !progress.with(|p| p.has_flag("sable_door")) {
                                    // Rook's story goes on whatever the result (not his hall rematch).
                                    view! {
                                        <b>{tr("The story goes on", "L'histoire continue")}</b>
                                        <span>{tr(format!("{} will remember this one.", t.name), format!("{} s'en souviendra.", t.name))}</span>
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
                                        <button class="btn primary" autofocus on:click=move |_| { crate::go_with(Screen::Credits(true), move || ctl.stop()) }>{tr("Roll the credits", "Voir le générique")}</button>
                                    }.into_any()
                                } else {
                                    view! {
                                        <button class="btn primary" autofocus on:click=move |_| { crate::go_with(Screen::World, move || ctl.stop()) }>{tr("Back to the world", "Retour au monde")}</button>
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
                                <button class="btn primary" autofocus on:click=move |_| { crate::go_with(back, move || ctl.stop()) }>{tr("Back", "Retour")}</button>
                            </div>
                        }.into_any(),
                        (None, None) => view! {
                    <div class="dialog-foot">
                        <button class="btn primary" on:click=rematch autofocus>{tr("Rematch", "Revanche")}</button>
                        {won.then(|| view! {
                            <button class="btn" on:click=move |_| { crate::go_with(Screen::Packs, move || ctl.stop()) }>{tr("Open packs", "Ouvrir des boosters")}</button>
                        })}
                        <button class="btn ghost" on:click=move |_| { crate::go_with(Screen::Duel, move || ctl.stop()) }>{tr("Change decks", "Changer de deck")}</button>
                    </div>
                        }.into_any(),
                    }}
                    <div class="result-extra">
                        {(!won && !lesson.1.is_empty()).then(|| view! {
                            <button class="btn" aria-expanded=move || mags.get().to_string() on:click=move |_| mags.update(|m| *m = !*m)>
                                {tr("Ask Mags, the deck doctor", "Demander conseil à Mags")}
                            </button>
                        })}
                        <button class="btn ghost" on:click=move |_| peek.set(true)>{tr("Look at the board", "Revoir le plateau")}</button>
                    </div>
                </div>
            </div>
        }.into_any())
    }
}

/// What a lost duel teaches: the opponent's weakness and your deck's main
/// problem, from the deck doctor (Mags). (hints, Mags's full reading)
fn loss_lesson(ctl: Ctl) -> (Vec<String>, Vec<String>) {
    use sixfold_engine::Color;
    let Some(setup) = ctl.setup.get_value() else {
        return (vec![], vec![]);
    };
    if setup.post == Some(Post::Tutorial) {
        return (vec![], vec![]);
    }
    let mut hints = vec![];
    let mut weak: Vec<(Color, usize)> = vec![];
    for &id in &setup.decks[1] {
        if let Some(w) = card(id).creature().and_then(|c| c.weakness) {
            match weak.iter_mut().find(|(c, _)| *c == w) {
                Some(e) => e.1 += 1,
                None => weak.push((w, 1)),
            }
        }
    }
    weak.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    if let Some(&(w, _)) = weak.first() {
        let mine = setup.decks[0]
            .iter()
            .filter(|&&id| card(id).creature().is_some_and(|c| c.color == w))
            .count();
        let opp = setup.names[1].clone();
        hints.push(if mine == 0 {
            tr(
                format!("Most of {opp}'s Hexons are weak to {w}, and your deck has none: one {w} Hexon would double your damage."),
                format!("La plupart des Hexons de {opp} sont faibles face à {w}, et votre deck n'en a aucun : un Hexon {w} doublerait vos dégâts."),
            )
        } else {
            tr(
                format!("Most of {opp}'s Hexons are weak to {w}: lead with your {w} Hexons ({mine} in your deck)."),
                format!("La plupart des Hexons de {opp} sont faibles face à {w} : menez avec vos Hexons {w} ({mine} dans votre deck)."),
            )
        });
    }
    let owned = expect_context::<RwSignal<sixfold_engine::profile::Profile>>()
        .with_untracked(|p| p.collection.clone());
    let advice = sixfold_engine::doctor::diagnose(&setup.decks[0], &owned);
    let reading: Vec<String> = advice
        .iter()
        .filter(|a| a.issue != sixfold_engine::doctor::Issue::Healthy)
        .map(|a| match &a.swap {
            Some(sw) => format!("{} {}", a.text(), sw.text()),
            None => a.text(),
        })
        .collect();
    match reading.first() {
        Some(first) => hints.push(tr(format!("Mags: {first}"), format!("Mags : {first}"))),
        // Only when nothing else explains it: never "nothing wrong" after a named weakness.
        None if hints.is_empty() => hints.push(tr(
            "Mags, the deck doctor, finds nothing wrong with your deck: the draw went against you this time.",
            "Mags, la docteure des decks, ne trouve rien à redire à votre deck : la pioche vous a desservi cette fois.",
        ).into()),
        None => {}
    }
    (hints, reading)
}

/// The rival's face in the duel header, and the speech bubble their lines
/// come in (see `Ctl::say`): it goes after a few seconds or on a tap, and a
/// polite live region reads it out. For rivals (world and post-game) and
/// route duelists.
#[component]
pub fn RivalFace(
    /// Only the speech bubble: the duel table shows the portrait in the
    /// rival's station.
    #[prop(optional)]
    bubble_only: bool,
) -> impl IntoView {
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
        <span class="opp-face" class:shake=ko class:bubble-only=bubble_only>
            {(!bubble_only).then(|| view! { <crate::campaign::Portrait id=id name=name /> })}
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

/// The first-time rule tip, beside the Hexon it concerns (pinned under the
/// history strip on a phone). It never blocks play; "Got it" or Escape dismisses it.
#[component]
pub fn RuleTip() -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let style = RwSignal::new(String::new());
    Effect::new(move |_| {
        let Some((tip, player)) = ctl.tip.get() else {
            return;
        };
        // On a phone the strip lies over the opponent's side: it folds away on its own.
        if window()
            .inner_width()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(1024.0)
            <= 640.0
        {
            set_timeout(
                move || {
                    if ctl.tip.get_untracked().is_some_and(|(t, _)| t == tip) {
                        ctl.tip.set(None);
                    }
                },
                std::time::Duration::from_secs(8),
            );
        }
        request_animation_frame(move || {
            let w = window()
                .inner_width()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(1024.0);
            let sel = match player {
                HUMAN => Some(".half.me .zone .hexon-card"),
                1 => Some(".half.opp .zone .hexon-card"),
                _ => None,
            };
            let rect = sel
                .and_then(|s| document().query_selector(s).ok().flatten())
                .map(|e| e.get_bounding_client_rect());
            style.set(match rect {
                _ if w >= 1180.0 => String::new(),
                Some(_) if w <= 640.0 => {
                    "left:6px;right:6px;top:calc(62px + env(safe-area-inset-top))".into()
                }
                Some(r) if r.right() + 330.0 < w => {
                    format!("left:{}px;top:{}px", r.right() + 18.0, r.top() + 4.0)
                }
                Some(r) => format!(
                    "left:{}px;top:{}px",
                    (r.left() - 318.0).max(8.0),
                    r.top() + 4.0
                ),
                None => "left:50%;top:40%;transform:translateX(-50%)".into(),
            });
        });
    });
    move || {
        let over = ctl
            .shown
            .with(|g| g.as_ref().is_some_and(|g| g.winner().is_some()));
        ctl.tip.get().filter(|_| !over).map(|(tip, _)| view! {
            <aside class="rule-tip" role="note" aria-labelledby="tip-title" style=move || style.get()
                on:keydown=move |e| { if e.key() == "Escape" { ctl.tip.set(None) } }>
                <h3 id="tip-title">{tip.title()}</h3>
                <p>{tip.text()}</p>
                <div>
                    <span>{tr("Rule tip. The duel menu shows them again.", "Astuce. Le menu du duel les remontre.")}</span>
                    <button on:click=move |_| ctl.tip.set(None)>{tr("Got it", "Compris")}</button>
                </div>
            </aside>
        })
    }
}

/// The duel menu: sound, the opponent's speed, tips, shortcuts, the duel's
/// details (its seed) and conceding.
#[component]
pub fn DuelMenu(
    open: RwSignal<bool>,
    #[prop(into)] on_leave: Callback<()>,
    leave_label: &'static str,
) -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let tips_reset = RwSignal::new(false);
    let copied = RwSignal::new(false);
    let close = NodeRef::<leptos::html::Button>::new();
    Effect::new(move |_| {
        if open.get() {
            if let Some(b) = close.get() {
                let _ = b.focus();
            }
        }
    });
    let tutorial = ctl.tutorial();
    move || {
        open.get().then(|| {
            let seed = ctl.seed.get_untracked();
            let prizes = ctl.setup.with_value(|s| s.as_ref().map_or(6, |s| s.prizes));
            view! {
                <div class="sheet-scrim" on:click=move |_| open.set(false)></div>
                <section class="drawer" role="dialog" aria-modal="true" aria-labelledby="menu-title"
                    on:keydown=move |e| { if e.key() == "Escape" { open.set(false) } }>
                    <header>
                        <h2 id="menu-title">{tr("Duel menu", "Menu du duel")}</h2>
                        <button class="icon-btn" node_ref=close aria-label=tr("Close", "Fermer") on:click=move |_| open.set(false)>
                            <svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><path d="M6 6l12 12M18 6 6 18"/></svg>
                        </button>
                    </header>
                    <crate::audio::AudioSettings />
                    <fieldset class="seg">
                        <legend>{tr("Opponent's speed", "Vitesse de l'adversaire")}</legend>
                        <label><input type="radio" name="speed" prop:checked=move || !ctl.fast.get()
                            on:change=move |_| { ctl.fast.set(false); crate::store::set_fast_opponent(false); } />{tr("Normal", "Normale")}</label>
                        <label><input type="radio" name="speed" prop:checked=move || ctl.fast.get()
                            on:change=move |_| { ctl.fast.set(true); crate::store::set_fast_opponent(true); } />{tr("Fast", "Rapide")}</label>
                    </fieldset>
                    <p class="fine">{tr("Only the pace changes. During their turn, hold the fast-forward button for even quicker play; every move stays in the history.",
                        "Seul le rythme change. Pendant leur tour, maintenez le bouton d'accélération ; chaque coup reste dans l'historique.")}</p>
                    <button class="btn" on:click=move |_| { crate::tips::reset(); tips_reset.set(true) }>
                        {move || if tips_reset.get() { tr("Rule tips will show again", "Les astuces reviendront") } else { tr("Show rule tips again", "Remontrer les astuces") }}
                    </button>
                    // With a pad the duel is played by focus: say how (#69).
                    {move || crate::input::pad().then(|| {
                        use crate::input::{prompt, Ctrl};
                        view! {
                            <details>
                                <summary>{tr("Gamepad", "Manette")}</summary>
                                <dl class="keys">
                                    <dt><kbd>{prompt(Ctrl::Walk)}</kbd></dt><dd>{tr("Move between cards and buttons", "Passer d'une carte ou d'un bouton à l'autre")}</dd>
                                    <dt><kbd>{prompt(Ctrl::Act)}</kbd></dt><dd>{tr("Pick, play, confirm", "Choisir, jouer, valider")}</dd>
                                    <dt><kbd>{prompt(Ctrl::Cancel)}</kbd></dt><dd>{tr("Put the card back, close a panel", "Reposer la carte, fermer un panneau")}</dd>
                                    <dt><kbd>{prompt(Ctrl::Menu)}</kbd></dt><dd>{tr("This menu", "Ce menu")}</dd>
                                </dl>
                            </details>
                        }
                    })}
                    <details>
                        <summary>{tr("Keyboard shortcuts", "Raccourcis clavier")}</summary>
                        <dl class="keys">
                            <dt><kbd>"1"</kbd>" – "<kbd>"4"</kbd></dt><dd>{tr("Attack", "Attaquer")}</dd>
                            <dt><kbd>"P"</kbd></dt><dd>{tr("Use your Active's Power", "Pouvoir de votre Actif")}</dd>
                            <dt><kbd>"R"</kbd></dt><dd>{tr("Retreat", "Retraite")}</dd>
                            <dt><kbd>"E"</kbd></dt><dd>{tr("End turn (twice if you can still attack)", "Fin du tour (deux fois si vous pouvez encore attaquer)")}</dd>
                            <dt><kbd>"H"</kbd></dt><dd>{tr("Go to your hand", "Aller à votre main")}</dd>
                            <dt><kbd>"0"</kbd>" – "<kbd>"5"</kbd></dt><dd>{tr("With a card held: the Active (0) or a Bench place (1–5, from the left)", "Carte en main : l'Actif (0) ou une place du Banc (1–5, depuis la gauche)")}</dd>
                            <dt><kbd>"L"</kbd></dt><dd>{tr("The duel log", "Le journal du duel")}</dd>
                            <dt><kbd>"I"</kbd></dt><dd>{tr("Read the card", "Lire la carte")}</dd>
                            <dt><kbd>"M"</kbd></dt><dd>{tr("This menu", "Ce menu")}</dd>
                            <dt><kbd>"Esc"</kbd></dt><dd>{tr("Put the card back, close a panel", "Reposer la carte, fermer un panneau")}</dd>
                        </dl>
                    </details>
                    {(!tutorial).then(|| view! {
                        <details>
                            <summary>{tr("Duel details", "Détails du duel")}</summary>
                            <dl class="details">
                                <dt>{tr("Seed", "Graine")}</dt>
                                <dd><code>{seed}</code>
                                    <button class="btn ghost" on:click=move |_| {
                                        let _ = window().navigator().clipboard().write_text(&seed.to_string());
                                        copied.set(true);
                                    }>{move || if copied.get() { tr("Copied", "Copiée") } else { tr("Copy", "Copier") }}</button>
                                </dd>
                                <dt>{tr("Prizes", "Récompenses")}</dt><dd>{prizes}</dd>
                            </dl>
                            <p class="fine">{tr("The seed replays this exact shuffle. Add it to a bug report.", "La graine rejoue ce mélange exact. Joignez-la à un signalement de bug.")}</p>
                        </details>
                    })}
                    <div class="drawer-foot">
                        <button class="btn danger" on:click=move |_| { open.set(false); on_leave.run(()) }>{leave_label}</button>
                    </div>
                </section>
            }
        })
    }
}
