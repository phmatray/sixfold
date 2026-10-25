//! The match controller: owns the engine `Game`, plays each batch of events
//! back with short pauses, then shows the new state and asks the AI to move
//! when it is the AI's turn.
//!
//! Two copies of the game exist: `real` moves as soon as an action is applied;
//! `shown` (what the board renders) only catches up once the events of that
//! action have been played. During playback the board shows the old state plus
//! the effects in [`Fx`] (lunges, damage numbers, HP overrides, coins).

use std::collections::{HashMap, VecDeque};
use std::time::Duration;

use leptos::prelude::*;
use sixfold_engine::ai::{Ai, Difficulty};
use sixfold_engine::tutorial;
use sixfold_engine::{card, fr, tr, Action, CardId, Config, Event, Game, PromptKind};

pub const HUMAN: usize = 0;
pub const AI: usize = 1;

#[derive(Clone, PartialEq)]
pub struct Coin {
    pub key: u32,
    pub player: usize,
    pub heads: bool,
    pub caption: String,
}

/// A big centred message: turn banner, attack name, trainer played.
#[derive(Clone, PartialEq)]
pub struct Callout {
    pub key: u32,
    pub title: String,
    pub sub: String,
    pub card: Option<CardId>,
    pub mine: bool,
}

/// A floating number or word over a creature.
#[derive(Clone, PartialEq)]
pub struct Pop {
    pub key: u32,
    pub uid: u32,
    pub text: String,
    pub kind: &'static str,
}

#[derive(Clone, PartialEq, Default)]
pub struct Fx {
    pub lunge: Option<u32>,
    pub hit: Option<u32>,
    pub glow: Option<u32>,
    pub pops: Vec<Pop>,
    /// HP to show instead of the stale `shown` value while events play.
    pub hp: HashMap<u32, u16>,
    pub ko: Vec<u32>,
    pub coin: Option<Coin>,
    pub callout: Option<Callout>,
    /// (player, prize position) being taken.
    pub prize: Option<(usize, usize)>,
}

#[derive(Clone, PartialEq)]
pub struct LogLine {
    pub key: u32,
    pub text: String,
    pub kind: &'static str,
}

/// What the title screen picked; kept for the rematch.
#[derive(Clone)]
pub struct Setup {
    pub decks: [Vec<CardId>; 2],
    pub names: [String; 2],
    pub prizes: u8,
    pub difficulty: Difficulty,
    /// The campaign rival's id, for a duel started from the world.
    pub rival: Option<&'static str>,
    /// A post-game duel: what its result counts for.
    pub post: Option<Post>,
    /// Where Quit and the result's way out lead.
    pub back: crate::Screen,
}

/// A duel of the post-game ladder.
#[derive(Clone, Copy, PartialEq)]
pub enum Post {
    /// The next rung of the Gauntlet run under way.
    Gauntlet,
    /// The daily duel of this day number.
    Daily(u32),
    /// The guided first duel: stacked decks, a scripted opponent, no reward.
    Tutorial,
}

#[derive(Clone, Copy)]
pub struct Ctl {
    pub shown: RwSignal<Option<Game>>,
    pub busy: RwSignal<bool>,
    pub fx: RwSignal<Fx>,
    pub log: RwSignal<Vec<LogLine>>,
    /// The opponent's setup placements stay face down until the first turn.
    pub revealed: RwSignal<bool>,
    pub seed: RwSignal<u64>,
    pub error: RwSignal<Option<String>>,
    /// For screen readers: what happened since the human last had to act
    /// (the AI's whole turn, or the result of the human's own action).
    pub announce: RwSignal<String>,
    heard: StoredValue<Vec<String>>,
    pub setup: StoredValue<Option<Setup>>,
    real: StoredValue<Option<Game>>,
    ai: StoredValue<Option<Ai>>,
    queue: StoredValue<VecDeque<Event>>,
    /// Why the next coin is flipped, from the events before it.
    coin_reason: StoredValue<String>,
    /// The last reason set was the turn-start flip (a following energy
    /// discard means a Confused retreat), kept apart from the localized text.
    turn_flip: StoredValue<bool>,
    /// Bumped on every new match so stale timers from the last one stop.
    generation: StoredValue<u32>,
    counter: StoredValue<u32>,
    /// The finished match's result hasn't been rewarded yet.
    unrewarded: StoredValue<bool>,
    reduced_motion: bool,
}

impl Ctl {
    pub fn new() -> Ctl {
        let reduced_motion = window()
            .match_media("(prefers-reduced-motion: reduce)")
            .ok()
            .flatten()
            .is_some_and(|m| m.matches());
        Ctl {
            shown: RwSignal::new(None),
            busy: RwSignal::new(false),
            fx: RwSignal::new(Fx::default()),
            log: RwSignal::new(vec![]),
            revealed: RwSignal::new(false),
            seed: RwSignal::new(0),
            error: RwSignal::new(None),
            announce: RwSignal::new(String::new()),
            heard: StoredValue::new(vec![]),
            setup: StoredValue::new(None),
            real: StoredValue::new(None),
            ai: StoredValue::new(None),
            queue: StoredValue::new(VecDeque::new()),
            coin_reason: StoredValue::new(String::new()),
            turn_flip: StoredValue::new(false),
            generation: StoredValue::new(0),
            counter: StoredValue::new(0),
            unrewarded: StoredValue::new(false),
            reduced_motion,
        }
    }

    fn next_key(&self) -> u32 {
        self.counter.update_value(|c| *c += 1);
        self.counter.get_value()
    }

    /// Start a match; returns false (and sets `error`) if the engine refuses it.
    pub fn start(&self, setup: Setup, seed: u64) -> bool {
        let game = if setup.post == Some(Post::Tutorial) {
            Ok(tutorial::game())
        } else {
            Game::new(
                setup.decks.clone(),
                seed,
                Config {
                    prizes: setup.prizes,
                },
            )
        };
        let game = match game {
            Ok(g) => g,
            Err(e) => {
                self.error.set(Some(tr(
                    format!("Can't start this match: {e:?}"),
                    format!("Impossible de lancer ce duel : {e:?}"),
                )));
                return false;
            }
        };
        self.generation.update_value(|g| *g += 1);
        self.unrewarded.set_value(true);
        self.error.set(None);
        self.seed.set(seed);
        // Its own seed, so the AI's random choices don't mirror the game's coins.
        self.ai
            .set_value(Some(Ai::new(seed ^ 0x5eed_0a11, setup.difficulty)));
        self.setup.set_value(Some(setup));
        self.log.set(vec![]);
        self.heard.set_value(vec![]);
        self.announce.set(String::new());
        self.fx.set(Fx::default());
        self.revealed.set(false);
        self.busy.set(true);
        let events = game.log().to_vec();
        self.real.set_value(Some(game.clone()));
        self.shown.set(Some(game));
        self.queue.set_value(events.into());
        crate::audio::music("duel");
        self.step(self.generation.get_value());
        true
    }

    /// True once per match: whoever shows its result grants the reward.
    pub fn take_result(&self) -> bool {
        let first = self.unrewarded.get_value();
        self.unrewarded.set_value(false);
        first
    }

    /// Leave the match (title screen): stops every pending timer.
    pub fn stop(&self) {
        self.generation.update_value(|g| *g += 1);
        self.queue.set_value(VecDeque::new());
        self.busy.set(false);
        self.shown.set(None);
        self.real.set_value(None);
    }

    pub fn tutorial(&self) -> bool {
        self.setup
            .with_value(|s| s.as_ref().is_some_and(|s| s.post == Some(Post::Tutorial)))
    }

    /// The human clicked one of their legal actions (in the tutorial, one
    /// the lesson's step allows).
    pub fn act(&self, action: Action) {
        let tutorial = self.tutorial();
        let mine = self.real.with_value(|g| {
            g.as_ref().is_some_and(|g| {
                g.view(HUMAN).waiting_on == Some(HUMAN)
                    && (!tutorial || tutorial::allows(g, action))
            })
        });
        if mine && !self.busy.get_untracked() {
            self.run(action);
        }
    }

    fn run(&self, action: Action) {
        let result = self
            .real
            .try_update_value(|g| g.as_mut().map(|g| g.apply(action)))
            .flatten();
        match result {
            Some(Ok(events)) => {
                self.busy.set(true);
                self.queue.update_value(|q| q.extend(events));
                self.step(self.generation.get_value());
            }
            Some(Err(e)) => self.error.set(Some(tr(
                format!("The engine refused {action:?}: {e:?}"),
                format!("Le moteur a refusé {action:?} : {e:?}"),
            ))),
            None => {}
        }
    }

    /// Play queued events until one needs a pause; commit when the queue is empty.
    fn step(self, generation: u32) {
        loop {
            if self.generation.get_value() != generation {
                return;
            }
            let Some(event) = self.queue.try_update_value(|q| q.pop_front()).flatten() else {
                self.commit(generation);
                return;
            };
            let ms = self.play(&event);
            if ms > 0 {
                let ms = if self.reduced_motion { ms * 6 / 10 } else { ms };
                set_timeout(move || self.step(generation), Duration::from_millis(ms));
                return;
            }
        }
    }

    fn commit(self, generation: u32) {
        self.shown.set(self.real.get_value());
        self.shown
            .with_untracked(|g| g.as_ref().map(crate::audio::duel_state));
        self.fx.set(Fx::default());
        self.busy.set(false);
        let ai_turn = self.real.with_value(|g| {
            g.as_ref()
                .is_some_and(|g| g.winner().is_none() && g.view(HUMAN).waiting_on == Some(AI))
        });
        if !ai_turn {
            let heard = self
                .heard
                .try_update_value(std::mem::take)
                .unwrap_or_default();
            if !heard.is_empty() {
                self.announce.set(summary(&heard));
            }
            #[cfg(debug_assertions)]
            self.autoplay(generation);
            return;
        }
        let setup = self.real.with_value(|g| {
            g.as_ref().and_then(|g| g.prompt()).is_some_and(|p| {
                matches!(p.kind, PromptKind::ChooseActive | PromptKind::ChooseBench)
            })
        });
        let think = if setup { 350 } else { 750 };
        let tutorial = self.tutorial();
        set_timeout(
            move || {
                if self.generation.get_value() != generation {
                    return;
                }
                let action = if tutorial {
                    self.real.with_value(|g| g.as_ref().map(tutorial::opponent))
                } else {
                    self.ai
                        .try_update_value(|ai| {
                            let ai = ai.as_mut()?;
                            self.real
                                .with_value(|g| g.as_ref().map(|g| ai.choose_action(g, AI)))
                        })
                        .flatten()
                };
                if let Some(action) = action {
                    self.run(action);
                }
            },
            Duration::from_millis(think),
        );
    }

    /// Debug builds only (`trunk serve`): with `?autoplay` in the URL the Hard
    /// AI plays the human's seat too, so a whole duel, a Warden win and the
    /// credits after it can be checked in the browser.
    #[cfg(debug_assertions)]
    fn autoplay(self, generation: u32) {
        let on = window()
            .location()
            .search()
            .is_ok_and(|s| s.contains("autoplay"));
        let mine = self.real.with_value(|g| {
            g.as_ref()
                .is_some_and(|g| g.winner().is_none() && g.view(HUMAN).waiting_on == Some(HUMAN))
        });
        if !on || !mine {
            return;
        }
        set_timeout(
            move || {
                if self.generation.get_value() != generation {
                    return;
                }
                let seed = self.seed.get_untracked() ^ u64::from(self.next_key());
                let action = self.real.with_value(|g| {
                    g.as_ref()
                        .map(|g| Ai::new(seed, Difficulty::Hard).choose_action(g, HUMAN))
                });
                if let Some(action) = action {
                    self.run(action);
                }
            },
            Duration::from_millis(50),
        );
    }

    fn active_uid(&self, player: usize) -> Option<u32> {
        self.shown.with_untracked(|g| {
            g.as_ref()
                .and_then(|g| g.view(player).sides[player].active.map(|a| a.uid))
        })
    }

    fn shown_hp(&self, uid: u32) -> Option<u16> {
        self.shown.with_untracked(|g| {
            let v = g.as_ref()?.view(HUMAN);
            v.sides
                .iter()
                .flat_map(|s| s.active.into_iter().chain(s.bench.iter()))
                .find(|c| c.uid == uid)
                .map(|c| c.hp)
        })
    }

    fn pop(&self, uid: Option<u32>, text: String, kind: &'static str) {
        if let Some(uid) = uid {
            let key = self.next_key();
            self.fx.update(|f| {
                f.pops.push(Pop {
                    key,
                    uid,
                    text,
                    kind,
                })
            });
        }
    }

    fn callout(&self, title: String, sub: String, card: Option<CardId>, mine: bool) {
        let key = self.next_key();
        self.fx.update(|f| {
            f.callout = Some(Callout {
                key,
                title,
                sub,
                card,
                mine,
            })
        });
    }

    /// Log one event, start its effect, and return how long to pause (ms).
    fn play(&self, event: &Event) -> u64 {
        crate::audio::duel_event(event);
        let key = self.next_key();
        let text = event.describe(HUMAN);
        let kind = log_kind(event);
        if !matches!(
            event,
            Event::Shuffled { .. } | Event::BetweenTurns | Event::TurnEnded { .. }
        ) && !text.is_empty()
        {
            self.heard.update_value(|h| h.push(text.clone()));
        }
        let reason = match event {
            Event::PrizesSet { .. } => Some(tr("Who goes first?", "Qui commence ?").to_string()),
            Event::TurnStarted { .. } => Some(tr("Coin flip", "Pile ou face").into()),
            Event::BetweenTurns => Some(
                tr(
                    "Asleep: heads wakes it up",
                    "Endormie : face, elle se réveille",
                )
                .into(),
            ),
            Event::EnergyDiscarded { .. } if self.turn_flip.get_value() => Some(
                tr(
                    "Confused: heads lets it retreat",
                    "Confuse : face, elle bat en retraite",
                )
                .into(),
            ),
            Event::AttackUsed {
                creature, attack, ..
            } => card(*creature)
                .creature()
                .and_then(|c| c.attacks.get(*attack))
                .map(|a| a.label().to_string()),
            Event::PowerUsed { creature, .. } => card(*creature)
                .creature()
                .and_then(|c| c.power.as_ref())
                .map(|p| p.label().to_string()),
            _ => None,
        };
        if let Some(r) = reason {
            self.coin_reason.set_value(r);
            self.turn_flip
                .set_value(matches!(event, Event::TurnStarted { .. }));
        }
        self.log.update(|l| {
            l.push(LogLine { key, text, kind });
            if l.len() > 300 {
                l.drain(..l.len() - 300);
            }
        });
        let you = |p: usize| p == HUMAN;
        match event {
            Event::Mulligan { player, .. } => {
                self.callout(
                    "Mulligan".into(),
                    match (you(*player), fr()) {
                        (true, false) => "You had no Basic creature and reshuffled",
                        (false, false) => "Opponent had no Basic creature and reshuffled",
                        (true, true) => "Aucune créature de Base : vous remélangez",
                        (false, true) => "Aucune créature de Base : l'adversaire remélange",
                    }
                    .into(),
                    None,
                    you(*player),
                );
                1300
            }
            Event::SetupRestarted => {
                self.callout(
                    tr("Redeal", "Nouvelle donne").into(),
                    tr(
                        "Neither player had a Basic creature",
                        "Aucun joueur n'avait de créature de Base",
                    )
                    .into(),
                    None,
                    true,
                );
                1200
            }
            Event::CoinFlip { player, heads } => {
                let caption = self.coin_reason.get_value();
                let key = self.next_key();
                self.fx.update(|f| {
                    f.callout = None;
                    f.coin = Some(Coin {
                        key,
                        player: *player,
                        heads: *heads,
                        caption,
                    })
                });
                1700
            }
            Event::FirstPlayer { player } => {
                self.fx.update(|f| f.coin = None);
                let title = if you(*player) {
                    tr("You go first", "Vous commencez")
                } else {
                    tr("Opponent goes first", "L'adversaire commence")
                };
                self.callout(
                    title.into(),
                    tr(
                        "The first player draws and may attack",
                        "Le premier joueur pioche et peut attaquer",
                    )
                    .into(),
                    None,
                    you(*player),
                );
                1300
            }
            Event::TurnStarted { player, turn } => {
                self.revealed.set(true);
                self.fx.update(|f| f.coin = None);
                let title = if you(*player) {
                    tr("Your turn", "Votre tour")
                } else {
                    tr("Opponent's turn", "Tour adverse")
                };
                // Counted from 1 for people (the engine counts from 0).
                let sub = tr(format!("Turn {}", turn + 1), format!("Tour {}", turn + 1));
                self.callout(title.into(), sub, None, you(*player));
                1000
            }
            Event::AttackUsed {
                player,
                attack,
                uid,
                creature,
            } => {
                let name = card(*creature)
                    .creature()
                    .and_then(|c| c.attacks.get(*attack))
                    .map_or(tr("Attack", "Attaque"), |a| a.label());
                self.fx.update(|f| f.lunge = Some(*uid));
                self.callout(
                    name.into(),
                    card(*creature).label().into(),
                    None,
                    you(*player),
                );
                900
            }
            Event::PowerUsed {
                player,
                uid,
                creature,
                ..
            } => {
                let name = card(*creature)
                    .creature()
                    .and_then(|c| c.power.as_ref())
                    .map_or(tr("Power", "Pouvoir"), |p| p.label());
                self.fx.update(|f| f.glow = Some(*uid));
                let owner = card(*creature).label();
                self.callout(
                    name.into(),
                    tr(format!("{owner}'s Power"), format!("Pouvoir de {owner}")),
                    None,
                    you(*player),
                );
                1000
            }
            Event::TrainerPlayed { player, card: c } => {
                let sub = match (you(*player), fr()) {
                    (true, false) => "You played a trainer",
                    (false, false) => "Opponent played a trainer",
                    (true, true) => "Vous jouez une carte Dresseur",
                    (false, true) => "L'adversaire joue une carte Dresseur",
                };
                self.callout(card(*c).label().into(), sub.into(), Some(*c), you(*player));
                if you(*player) {
                    700
                } else {
                    1400
                }
            }
            Event::Evolved { uid, .. } => {
                self.fx.update(|f| f.glow = Some(*uid));
                500
            }
            Event::Damage {
                uid,
                amount,
                hp,
                weakness,
                resistance,
                ..
            } => {
                let mut text = format!("−{amount}");
                if *weakness {
                    text.push_str(" ×2");
                }
                if *resistance {
                    text.push_str(tr(" resisted", " résisté"));
                }
                let kind = if *amount == 0 { "none" } else { "dmg" };
                self.fx.update(|f| {
                    f.lunge = None;
                    f.hit = Some(*uid);
                    f.hp.insert(*uid, *hp);
                });
                self.pop(Some(*uid), text, kind);
                850
            }
            Event::Healed { uid, amount, .. } => {
                let now = self.fx.with_untracked(|f| f.hp.get(uid).copied());
                if let Some(hp) = now.or_else(|| self.shown_hp(*uid)) {
                    self.fx.update(|f| {
                        f.hp.insert(*uid, hp + amount);
                    });
                }
                self.fx.update(|f| f.glow = Some(*uid));
                self.pop(Some(*uid), format!("+{amount}"), "heal");
                700
            }
            Event::KnockedOut { uid, .. } => {
                self.fx.update(|f| f.ko.push(*uid));
                950
            }
            Event::PrizeTaken { player, prize, .. } => {
                self.fx.update(|f| f.prize = Some((*player, *prize)));
                600
            }
            Event::ConditionApplied { player, status } => {
                self.pop(self.active_uid(*player), status.to_string(), "status");
                700
            }
            Event::ConditionRemoved { player, status } => {
                let text = tr(format!("{status} wore off"), format!("N'est plus {status}"));
                self.pop(self.active_uid(*player), text, "cure");
                500
            }
            Event::AttackFailed { player } => {
                self.fx.update(|f| f.lunge = None);
                self.pop(
                    self.active_uid(*player),
                    tr("Failed", "Raté").into(),
                    "none",
                );
                700
            }
            Event::AttackNegated { player } => {
                self.pop(
                    self.active_uid(*player),
                    tr("Protected", "Protégée").into(),
                    "none",
                );
                700
            }
            Event::RetreatFailed { player } => {
                self.pop(
                    self.active_uid(*player),
                    tr("Can't retreat", "Retraite impossible").into(),
                    "none",
                );
                800
            }
            Event::EnergyDiscarded { .. }
            | Event::HandDiscarded { .. }
            | Event::Devolved { .. } => 300,
            Event::EffectApplied { .. } => 250,
            Event::DeckOut { player } => {
                self.callout(
                    tr("Out of cards", "Plus de cartes").into(),
                    match (you(*player), fr()) {
                        (true, false) => "You couldn't draw",
                        (false, false) => "Opponent couldn't draw",
                        (true, true) => "Vous ne pouvez plus piocher",
                        (false, true) => "L'adversaire ne peut plus piocher",
                    }
                    .into(),
                    None,
                    you(*player),
                );
                1400
            }
            Event::SuddenDeath => {
                self.revealed.set(false);
                self.callout(
                    tr("Sudden death", "Mort subite").into(),
                    tr(
                        "A tie. New setup, 1 prize each",
                        "Égalité. Nouvelle mise en place, 1 Récompense chacun",
                    )
                    .into(),
                    None,
                    true,
                );
                2000
            }
            _ => 0,
        }
    }
}

/// The lines heard since the human last acted, as one sentence each; a line
/// repeated in a row ("Opponent drew a card") is said once with a count.
fn summary(lines: &[String]) -> String {
    let mut out: Vec<String> = vec![];
    let mut i = 0;
    while i < lines.len() {
        let n = lines[i..].iter().take_while(|l| **l == lines[i]).count();
        let line = lines[i].trim_end_matches('.');
        // "You win!" keeps its own mark rather than gaining a full stop.
        let stop = if line.ends_with(['!', '?']) { "" } else { "." };
        out.push(if n > 1 {
            tr(
                format!("{line} ({n} times){stop}"),
                format!("{line} ({n} fois){stop}"),
            )
        } else {
            format!("{line}{stop}")
        });
        i += n;
    }
    out.join(" ")
}

fn log_kind(e: &Event) -> &'static str {
    match e {
        Event::TurnStarted { .. } | Event::SuddenDeath => "turn",
        Event::Damage { .. } => "dmg",
        Event::KnockedOut { .. } | Event::GameOver { .. } | Event::DeckOut { .. } => "ko",
        Event::CoinFlip { .. } => "coin",
        Event::AttackUsed { .. } | Event::PowerUsed { .. } => "act",
        Event::Shuffled { .. }
        | Event::CardDrawn { .. }
        | Event::BetweenTurns
        | Event::TurnEnded { .. } => "quiet",
        _ => "",
    }
}
