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
use pkmntcg_engine::{card, Action, CardId, Config, Event, Game, PromptKind, Status};

pub const HUMAN: usize = 0;
pub const AI: usize = 1;

/// The AI opponent's move. The only place the web app decides for the AI.
pub fn ai_move(game: &Game, _me: usize) -> Action {
    // Placeholder until `pkmntcg_engine::ai` lands: play cards, then attack,
    // else end the turn. Skips Retreat and Powers so it can't loop.
    let actions = game.legal_actions();
    let other = |a: &&Action| {
        !matches!(
            a,
            Action::Retreat | Action::UsePower(_) | Action::Attack(_) | Action::Done | Action::Cancel
        )
    };
    let pick = actions
        .iter()
        .find(other)
        .or_else(|| actions.iter().find(|a| matches!(a, Action::Attack(_))))
        .or_else(|| actions.iter().find(|a| matches!(a, Action::Done)))
        .or(actions.first());
    *pick.expect("legal_actions is never empty while the game runs")
}

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
    pub setup: StoredValue<Option<Setup>>,
    real: StoredValue<Option<Game>>,
    queue: StoredValue<VecDeque<Event>>,
    /// Bumped on every new match so stale timers from the last one stop.
    generation: StoredValue<u32>,
    counter: StoredValue<u32>,
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
            setup: StoredValue::new(None),
            real: StoredValue::new(None),
            queue: StoredValue::new(VecDeque::new()),
            generation: StoredValue::new(0),
            counter: StoredValue::new(0),
            reduced_motion,
        }
    }

    fn next_key(&self) -> u32 {
        self.counter.update_value(|c| *c += 1);
        self.counter.get_value()
    }

    /// Start a match; returns false (and sets `error`) if the engine refuses it.
    pub fn start(&self, setup: Setup, seed: u64) -> bool {
        let game = match Game::new(setup.decks.clone(), seed, Config { prizes: setup.prizes }) {
            Ok(g) => g,
            Err(e) => {
                self.error.set(Some(format!("Can't start this match: {e:?}")));
                return false;
            }
        };
        self.generation.update_value(|g| *g += 1);
        self.error.set(None);
        self.seed.set(seed);
        self.setup.set_value(Some(setup));
        self.log.set(vec![]);
        self.fx.set(Fx::default());
        self.revealed.set(false);
        self.busy.set(true);
        let events = game.log().to_vec();
        self.real.set_value(Some(game.clone()));
        self.shown.set(Some(game));
        self.queue.set_value(events.into());
        self.step(self.generation.get_value());
        true
    }

    /// Leave the match (title screen): stops every pending timer.
    pub fn stop(&self) {
        self.generation.update_value(|g| *g += 1);
        self.queue.set_value(VecDeque::new());
        self.busy.set(false);
        self.shown.set(None);
        self.real.set_value(None);
    }

    /// The human clicked one of their legal actions.
    pub fn act(&self, action: Action) {
        let mine = self.real.with_value(|g| {
            g.as_ref()
                .is_some_and(|g| g.view(HUMAN).waiting_on == Some(HUMAN))
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
            Some(Err(e)) => self.error.set(Some(format!("The engine refused {action:?}: {e:?}"))),
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
        self.fx.set(Fx::default());
        self.busy.set(false);
        let ai_turn = self.real.with_value(|g| {
            g.as_ref()
                .is_some_and(|g| g.winner().is_none() && g.view(HUMAN).waiting_on == Some(AI))
        });
        if !ai_turn {
            return;
        }
        let setup = self.real.with_value(|g| {
            g.as_ref().and_then(|g| g.prompt()).is_some_and(|p| {
                matches!(p.kind, PromptKind::ChooseActive | PromptKind::ChooseBench)
            })
        });
        let think = if setup { 350 } else { 750 };
        set_timeout(
            move || {
                if self.generation.get_value() != generation {
                    return;
                }
                let action = self.real.with_value(|g| g.as_ref().map(|g| ai_move(g, AI)));
                if let Some(action) = action {
                    self.run(action);
                }
            },
            Duration::from_millis(think),
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
            self.fx.update(|f| f.pops.push(Pop { key, uid, text, kind }));
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
        let key = self.next_key();
        let text = event.describe(HUMAN);
        let kind = log_kind(event);
        let previous = self.log.with_untracked(|l| l.last().map(|l| l.text.clone()));
        self.log.update(|l| {
            l.push(LogLine { key, text, kind });
            if l.len() > 300 {
                l.drain(..l.len() - 300);
            }
        });
        let you = |p: usize| p == HUMAN;
        let who = |p: usize| if p == HUMAN { "You" } else { "Opponent" };
        match event {
            Event::Mulligan { player, .. } => {
                self.callout(
                    "Mulligan".into(),
                    format!("{} had no Basic creature and reshuffled", who(*player)),
                    None,
                    you(*player),
                );
                1300
            }
            Event::SetupRestarted => {
                self.callout("Redeal".into(), "Neither player had a Basic creature".into(), None, true);
                1200
            }
            Event::CoinFlip { player, heads } => {
                let caption = previous.unwrap_or_default();
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
                let title = if you(*player) { "You go first" } else { "Opponent goes first" };
                self.callout(title.into(), "The first player draws and may attack".into(), None, you(*player));
                1300
            }
            Event::TurnStarted { player, turn } => {
                self.revealed.set(true);
                self.fx.update(|f| f.coin = None);
                let title = if you(*player) { "Your turn" } else { "Opponent's turn" };
                self.callout(title.into(), format!("Turn {turn}"), None, you(*player));
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
                    .map_or("Attack", |a| a.name);
                self.fx.update(|f| f.lunge = Some(*uid));
                self.callout(name.into(), card(*creature).name.into(), None, you(*player));
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
                    .map_or("Power", |p| p.name);
                self.fx.update(|f| f.glow = Some(*uid));
                self.callout(name.into(), format!("{}'s Power", card(*creature).name), None, you(*player));
                1000
            }
            Event::TrainerPlayed { player, card: c } => {
                let sub = format!("{} played a trainer", who(*player));
                self.callout(card(*c).name.into(), sub, Some(*c), you(*player));
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
                    text.push_str(" resisted");
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
                self.pop(self.active_uid(*player), status_word(*status).into(), "status");
                700
            }
            Event::ConditionRemoved { player, status } => {
                let text = format!("{} wore off", status_word(*status));
                self.pop(self.active_uid(*player), text, "cure");
                500
            }
            Event::AttackFailed { player } => {
                self.fx.update(|f| f.lunge = None);
                self.pop(self.active_uid(*player), "Failed".into(), "none");
                700
            }
            Event::AttackNegated { player } => {
                self.pop(self.active_uid(*player), "Protected".into(), "none");
                700
            }
            Event::RetreatFailed { player } => {
                self.pop(self.active_uid(*player), "Can't retreat".into(), "none");
                800
            }
            Event::EnergyDiscarded { .. } | Event::HandDiscarded { .. } | Event::Devolved { .. } => 300,
            Event::EffectApplied { .. } => 250,
            Event::DeckOut { player } => {
                self.callout("Out of cards".into(), format!("{} couldn't draw", who(*player)), None, you(*player));
                1400
            }
            Event::SuddenDeath => {
                self.revealed.set(false);
                self.callout("Sudden death".into(), "A tie. New setup, 1 prize each".into(), None, true);
                2000
            }
            _ => 0,
        }
    }
}

fn status_word(s: Status) -> &'static str {
    match s {
        Status::Confused => "Confused",
        Status::Asleep => "Asleep",
        Status::Paralyzed => "Paralyzed",
        Status::Poisoned => "Poisoned",
        Status::DoublePoisoned => "Double-poisoned",
    }
}

fn log_kind(e: &Event) -> &'static str {
    match e {
        Event::TurnStarted { .. } | Event::SuddenDeath => "turn",
        Event::Damage { .. } => "dmg",
        Event::KnockedOut { .. } | Event::GameOver { .. } | Event::DeckOut { .. } => "ko",
        Event::CoinFlip { .. } => "coin",
        Event::AttackUsed { .. } | Event::PowerUsed { .. } => "act",
        Event::Shuffled { .. } | Event::CardDrawn { .. } | Event::BetweenTurns | Event::TurnEnded { .. } => "quiet",
        _ => "",
    }
}
