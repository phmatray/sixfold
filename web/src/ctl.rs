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
use sixfold_engine::ai::{Ai, Difficulty, Style};
use sixfold_engine::campaign::{self, RivalLines, SealDuel};
use sixfold_engine::overworld::Progress;
use sixfold_engine::tutorial;
use sixfold_engine::{
    card, fr, summarise_turn, tr, Action, CardId, Config, Event, Game, PromptKind,
};

pub const HUMAN: usize = 0;
pub const AI: usize = 1;

#[derive(Clone, PartialEq)]
pub struct Coin {
    pub key: u32,
    pub player: usize,
    pub heads: bool,
    pub caption: String,
    /// The flip that settles an attack: it arcs higher.
    pub decide: bool,
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
    /// Damage done (its number grows with it), else 0.
    pub amount: u16,
    /// Beside the number: weakness "×2", resistance "−30" (class, text).
    pub stamps: Vec<(&'static str, &'static str)>,
}

/// A big moment's extra weight (WORLD_DESIGN §12.1), on top of the event's
/// own effect and sound: what the duel wears while it plays (`data-beat`),
/// a sound (name, variant) and a hold before the event (the KO's hit-stop).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Beat {
    pub class: &'static str,
    pub sfx: Option<(&'static str, f64)>,
    pub hold_ms: u32,
}

/// The beat of `event`, if it is a big moment. `next` is the event after it,
/// `attack` says an attack is resolving, `left` is what its taker has left
/// once a prize lands, `seal` that this prize would win a seal (or the title).
pub fn beat(
    event: &Event,
    next: Option<&Event>,
    attack: bool,
    left: usize,
    seal: bool,
) -> Option<Beat> {
    let b = |class, sfx, hold_ms| {
        Some(Beat {
            class,
            sfx,
            hold_ms,
        })
    };
    match event {
        Event::KnockedOut { .. } => b("ko", Some(("crack", 0.0)), 80),
        // The winning prize flies at half speed; a seal's holds first, to its sting.
        Event::PrizeTaken { .. } if left == 0 && seal => {
            b("last-prize seal-final", Some(("stinger", 0.0)), 240)
        }
        Event::PrizeTaken { .. } if left == 0 => b("last-prize", Some(("fly", 1.0)), 0),
        Event::PrizeTaken { .. } => b("prize", Some(("fly", 0.0)), 0),
        // The last flip of an attack's run of flips settles it.
        Event::CoinFlip { .. } if attack && !matches!(next, Some(Event::CoinFlip { .. })) => {
            b("decide", None, 0)
        }
        _ => None,
    }
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
    /// The beat playing, for `data-beat` on the duel.
    pub beat: Option<&'static str>,
    /// A hit-stop: every animation on the table holds still.
    pub frozen: bool,
}

/// A rival's duel line in the speech bubble; `ko` for their first knock-out.
#[derive(Clone, Copy, PartialEq)]
pub struct Said {
    pub key: u32,
    pub text: &'static str,
    pub ko: bool,
}

/// One play in the history rail: who acted, the card to show and what
/// came of it (the engine's own sentences, in order).
#[derive(Clone, PartialEq)]
pub struct Play {
    pub key: u32,
    /// Whose turn it was.
    pub side: usize,
    /// Counted from 1 for people.
    pub turn: u32,
    /// attack, power, trainer, energy, awaken, retreat, bench or ko.
    pub kind: &'static str,
    pub card: CardId,
    /// The Hexon it went to or hit.
    pub target: Option<CardId>,
    /// Total damage an attack did.
    pub amount: Option<u16>,
    pub title: String,
    pub lines: Vec<String>,
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
    /// How the AI leans: a rival's own style, else Balanced.
    pub style: Style,
    /// The rival's or route duelist's id, for a duel started from the world.
    pub opponent: Option<&'static str>,
    /// A post-game duel's rival (Gauntlet, daily duel): their face and lines,
    /// with none of a world duel's rewards.
    pub rival: Option<&'static str>,
    /// A post-game duel: what its result counts for.
    pub post: Option<Post>,
    /// Where Quit and the result's way out lead.
    pub back: crate::Screen,
}

impl Setup {
    /// Whose face and lines the duel shows: the world opponent, else the
    /// post-game rival.
    pub fn face(&self) -> Option<&'static str> {
        self.opponent.or(self.rival)
    }
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
    /// The plays of this duel, oldest first (the rail shows the latest).
    pub history: RwSignal<Vec<Play>>,
    /// (turn player, turn) of the turn being played back.
    turn_of: StoredValue<(usize, u32)>,
    /// The opponent's moves play faster (a saved preference).
    pub fast: RwSignal<bool>,
    /// Held down: the opponent's turn runs at a tenth of the pace (display only).
    pub rush: RwSignal<bool>,
    /// Whose turn the playback has reached (set the moment a turn starts,
    /// before the table catches up).
    pub turn_side: RwSignal<Option<usize>>,
    /// The turn the current rule tip came up on (it goes a full round later).
    tip_turn: StoredValue<u32>,
    /// Energy discards since the last play began: a retreat's payment comes
    /// before the retreat itself, so the retreat takes them back.
    discards: StoredValue<Vec<String>>,
    /// The rule tip to show now, and whose Active it points at.
    pub tip: RwSignal<Option<(crate::tips::Tip, usize)>>,
    /// The opponent's setup placements stay face down until the first turn.
    pub revealed: RwSignal<bool>,
    pub seed: RwSignal<u64>,
    pub error: RwSignal<Option<String>>,
    /// For screen readers: what happened since the human last had to act
    /// (the AI's whole turn, or the result of the human's own action).
    pub announce: RwSignal<String>,
    /// The rival's line being said, if any (see [`Ctl::say`]).
    pub said: RwSignal<Option<Said>>,
    /// An energy just attached: (key, uid, colour key). Its own signal, so the
    /// spark outlives the playback without holding it up.
    pub spark: RwSignal<Option<(u32, u32, &'static str)>>,
    /// An attack is resolving (its coins can settle it).
    attacking: StoredValue<bool>,
    /// Prizes each side took in the batch playing (`shown` doesn't know yet).
    taken: StoredValue<[usize; 2]>,
    /// The rival lines already said this duel, one bit each.
    spoken: StoredValue<u8>,
    /// What happened since the human last acted, for [`Ctl::announce`].
    heard: StoredValue<Vec<Event>>,
    /// Rule tips that came up meanwhile, read after it.
    heard_tips: StoredValue<Vec<String>>,
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
    /// The seal (or the champion's title) this match is for, if any.
    seal: RwSignal<Option<SealDuel>>,
    /// The seal duel's title card is up: the playback waits for [`Ctl::begin`].
    pub card: RwSignal<bool>,
    progress: Option<RwSignal<Progress>>,
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
            history: RwSignal::new(vec![]),
            turn_of: StoredValue::new((0, 0)),
            fast: RwSignal::new(crate::store::fast_opponent()),
            rush: RwSignal::new(false),
            tip_turn: StoredValue::new(0),
            turn_side: RwSignal::new(None),
            discards: StoredValue::new(vec![]),
            tip: RwSignal::new(None),
            revealed: RwSignal::new(false),
            seed: RwSignal::new(0),
            error: RwSignal::new(None),
            announce: RwSignal::new(String::new()),
            said: RwSignal::new(None),
            spark: RwSignal::new(None),
            attacking: StoredValue::new(false),
            taken: StoredValue::new([0; 2]),
            spoken: StoredValue::new(0),
            heard: StoredValue::new(vec![]),
            heard_tips: StoredValue::new(vec![]),
            setup: StoredValue::new(None),
            real: StoredValue::new(None),
            ai: StoredValue::new(None),
            queue: StoredValue::new(VecDeque::new()),
            coin_reason: StoredValue::new(String::new()),
            turn_flip: StoredValue::new(false),
            generation: StoredValue::new(0),
            counter: StoredValue::new(0),
            unrewarded: StoredValue::new(false),
            seal: RwSignal::new(None),
            card: RwSignal::new(false),
            progress: use_context::<RwSignal<Progress>>(),
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
        self.ai.set_value(Some(
            Ai::new(seed ^ 0x5eed_0a11, setup.difficulty).with_style(setup.style),
        ));
        crate::audio::duel_theme(&setup);
        // From the progress now, so a rematch after the win is a plain duel.
        let seal = self
            .progress
            .and_then(|p| p.with_untracked(|p| campaign::seal_duel(setup.opponent, p)));
        self.setup.set_value(Some(setup));
        self.log.set(vec![]);
        self.turn_side.set(None);
        self.history.set(vec![]);
        self.turn_of.set_value((0, 0));
        self.tip.set(None);
        self.heard.set_value(vec![]);
        self.heard_tips.set_value(vec![]);
        self.announce.set(String::new());
        self.said.set(None);
        self.spoken.set_value(0);
        self.taken.set_value([0; 2]);
        self.attacking.set_value(false);
        self.fx.set(Fx::default());
        self.revealed.set(false);
        self.busy.set(true);
        let events = game.log().to_vec();
        self.real.set_value(Some(game.clone()));
        self.shown.set(Some(game));
        self.queue.set_value(events.into());
        crate::audio::music("duel");
        self.seal.set(seal);
        self.card.set(seal.is_some());
        match seal {
            // The title card holds the opening until it is dismissed.
            Some(s) => self.announce.set(format!(
                "{}. {}, {}.",
                s.heading(),
                s.rival.name,
                s.rival.title
            )),
            None => self.step(self.generation.get_value()),
        }
        true
    }

    /// The seal this match is for, if any.
    pub fn seal(&self) -> Option<SealDuel> {
        self.seal.get()
    }

    /// The title card is done: the duel begins (once).
    pub fn begin(&self) {
        if self.card.get_untracked() {
            self.card.set(false);
            self.step(self.generation.get_value());
        }
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
        self.card.set(false);
        self.shown.set(None);
        self.real.set_value(None);
    }

    /// The rival's duel lines, in a world or post-game duel against a rival
    /// or Rook (route duelists, quick duels and the tutorial have none).
    pub fn lines(&self) -> Option<&'static RivalLines> {
        self.setup
            .with_value(|s| s.as_ref()?.face())
            .and_then(campaign::lines)
    }

    /// Say one of the rival's lines, at most once per duel (`bit`: 1 intro,
    /// 2 first KO, 4 on the ropes).
    fn say(&self, bit: u8, pick: fn(&RivalLines) -> sixfold_engine::T) {
        let Some(lines) = self.lines() else {
            return;
        };
        if self.spoken.get_value() & bit != 0 {
            return;
        }
        self.spoken.update_value(|s| *s |= bit);
        let key = self.next_key();
        self.said.set(Some(Said {
            key,
            text: pick(lines).get(),
            ko: bit == 2,
        }));
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

    /// The engine said no: the refuse sound and the reason.
    fn refused(&self, why: String) {
        crate::audio::sfx("refuse", 0.0);
        self.error.set(Some(why));
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
            Some(Err(e)) => self.refused(tr(
                format!("The engine refused {action:?}: {e:?}"),
                format!("Le moteur a refusé {action:?} : {e:?}"),
            )),
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
            let beat = self.beat_of(&event);
            if let Some(hold) = beat.map(|b| u64::from(b.hold_ms)).filter(|h| *h > 0) {
                // Hit-stop: the table freezes on the blow, then the event lands.
                self.fx.update(|f| {
                    f.frozen = true;
                    f.beat = None;
                });
                set_timeout(
                    move || {
                        if self.generation.get_value() != generation {
                            return;
                        }
                        self.fx.update(|f| f.frozen = false);
                        if !self.show(&event, beat, generation) {
                            self.step(generation);
                        }
                    },
                    Duration::from_millis(self.pace(hold)),
                );
                return;
            }
            if self.show(&event, beat, generation) {
                return;
            }
        }
    }

    /// The beat of the next event to play, from the replay's own state.
    fn beat_of(&self, event: &Event) -> Option<Beat> {
        let (left, mine) = match event {
            Event::PrizeTaken { player, .. } => (
                self.prizes_left(*player).saturating_sub(1),
                *player == HUMAN,
            ),
            _ => (usize::MAX, false),
        };
        let next = self.queue.with_value(|q| q.front().cloned());
        // Only the player's winning prize wins the seal; the keeper's is a plain last prize.
        let seal = mine && self.seal.with_untracked(Option::is_some);
        beat(event, next.as_ref(), self.attacking.get_value(), left, seal)
    }

    /// The prizes `player` has left at this point of the playback.
    fn prizes_left(&self, player: usize) -> usize {
        let shown = self.shown.with_untracked(|g| {
            g.as_ref().map_or(0, |g| {
                g.view(HUMAN).sides[player]
                    .prizes
                    .iter()
                    .filter(|p| **p)
                    .count()
            })
        });
        shown.saturating_sub(self.taken.get_value()[player])
    }

    /// Play one event and its beat; true if a pause was scheduled.
    fn show(self, event: &Event, beat: Option<Beat>, generation: u32) -> bool {
        let ms = self.play(event, beat);
        if let Some((name, n)) = beat.and_then(|b| b.sfx) {
            crate::audio::sfx(name, n);
        }
        if ms == 0 {
            return false;
        }
        self.fx.update(|f| f.beat = beat.map(|b| b.class));
        let ms = if self.reduced_motion { ms * 6 / 10 } else { ms };
        let ms = self.pace(ms);
        set_timeout(move || self.step(generation), Duration::from_millis(ms));
        true
    }

    fn commit(self, generation: u32) {
        self.shown.set(self.real.get_value());
        self.shown
            .with_untracked(|g| g.as_ref().map(crate::audio::duel_state));
        self.fx.set(Fx::default());
        self.taken.set_value([0; 2]);
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
            let mut said = summarise_turn(&heard, HUMAN);
            for tip in self
                .heard_tips
                .try_update_value(std::mem::take)
                .unwrap_or_default()
            {
                said = format!("{said} {tip}").trim_start().to_string();
            }
            if !said.is_empty() {
                self.announce.set(said);
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
        let think = self.pace(if setup { 350 } else { 750 });
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
    /// credits after it can be checked in the browser. `?autoplay=lose` only
    /// ends the turn, to check a lost duel's screen.
    #[cfg(debug_assertions)]
    fn autoplay(self, generation: u32) {
        let search = window().location().search().unwrap_or_default();
        let on = search.contains("autoplay");
        let lose = search.contains("autoplay=lose");
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
                    g.as_ref().map(|g| {
                        if lose && g.legal_actions().contains(&Action::Done) {
                            Action::Done
                        } else {
                            Ai::new(seed, Difficulty::Hard).choose_action(g, HUMAN)
                        }
                    })
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
        self.pop_with(uid, text, kind, 0, vec![]);
    }

    fn pop_with(
        &self,
        uid: Option<u32>,
        text: String,
        kind: &'static str,
        amount: u16,
        stamps: Vec<(&'static str, &'static str)>,
    ) {
        if let Some(uid) = uid {
            let key = self.next_key();
            self.fx.update(|f| {
                f.pops.push(Pop {
                    key,
                    uid,
                    text,
                    kind,
                    amount,
                    stamps,
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

    /// Pauses shrink to a third while the opponent plays, if the player
    /// asked for a faster opponent.
    fn pace(&self, ms: u64) -> u64 {
        let theirs = self.turn_of.get_value().0 == AI && self.revealed.get_untracked();
        ms / slowdown(theirs, self.rush.get_untracked(), self.fast.get_untracked())
    }

    /// `pace` as a factor, for the table's juice (`--pace` in the CSS):
    /// under Fast or a held fast-forward its beats play quicker too.
    pub fn pace_factor(&self) -> f32 {
        let theirs = self.turn_side.get() == Some(AI) && self.revealed.get();
        1.0 / slowdown(theirs, self.rush.get(), self.fast.get()) as f32
    }

    /// Point a rule tip at `player`'s Active the first time its rule comes up
    /// (never in the guided duel, which explains as it goes).
    pub fn tip_once(&self, tip: crate::tips::Tip, player: usize) {
        if self.tutorial() || tip.seen() || self.tip.get_untracked().is_some() {
            return;
        }
        tip.mark_seen();
        self.tip_turn.set_value(self.turn_of.get_value().1);
        self.heard_tips
            .update_value(|h| h.push(format!("{}: {}", tip.title(), tip.text())));
        self.tip.set(Some((tip, player)));
    }

    /// Add the event to the play history: a play starts a new entry, what
    /// follows from it (damage, a knock-out's details, a coin) joins it.
    fn record(&self, event: &Event, text: &str, kind: &'static str) {
        use crate::tips::Tip;
        match event {
            Event::FirstPlayer { player } if *player == HUMAN => {
                self.tip_once(Tip::FirstTurn, HUMAN)
            }
            Event::ConditionApplied { player, status } => {
                self.tip_once(Tip::for_status(*status), *player)
            }
            Event::SuddenDeath => self.tip_once(Tip::SuddenDeath, 2),
            _ => {}
        }
        if let Event::TurnStarted { player, turn } = event {
            // A turn with no play still leaves a trace: "passed".
            let (side, last) = self.turn_of.get_value();
            let played = self
                .history
                .with_untracked(|h| h.last().is_some_and(|p| p.turn == last));
            let face = self.shown.with_untracked(|g| {
                g.as_ref()
                    .and_then(|g| g.view(HUMAN).sides[side].active.map(|a| a.top()))
            });
            if let (true, false, Some(c)) = (last > 0, played, face) {
                let key = self.next_key();
                self.history.update(|h| {
                    h.push(Play {
                        key,
                        side,
                        turn: last,
                        kind: "pass",
                        card: c,
                        target: None,
                        amount: None,
                        title: tr("No play", "Aucun coup").into(),
                        lines: vec![tr(
                            "Drew a card and ended the turn.",
                            "A pioché une carte et fini son tour.",
                        )
                        .into()],
                    })
                });
            }
            self.turn_of.set_value((*player, turn + 1));
            self.turn_side.set(Some(*player));
            if self.tip.get_untracked().is_some() && turn + 1 >= self.tip_turn.get_value() + 2 {
                self.tip.set(None);
            }
            return;
        }
        let (side, turn) = self.turn_of.get_value();
        let play = |kind: &'static str, card: CardId, target: Option<CardId>, title: String| Play {
            key: self.next_key(),
            side,
            turn,
            kind,
            card,
            target,
            amount: None,
            title,
            lines: vec![],
        };
        let new = match event {
            Event::AttackUsed {
                attack, creature, ..
            } => Some(play(
                "attack",
                *creature,
                None,
                card(*creature)
                    .creature()
                    .and_then(|c| c.attacks.get(*attack))
                    .map_or(tr("Attack", "Attaque"), |a| a.label())
                    .into(),
            )),
            Event::PowerUsed { creature, .. } => Some(play(
                "power",
                *creature,
                None,
                card(*creature)
                    .creature()
                    .and_then(|c| c.power.as_ref())
                    .map_or(tr("Power", "Pouvoir"), |p| p.label())
                    .into(),
            )),
            Event::TrainerPlayed { card: c, .. } => {
                Some(play("trainer", *c, None, card(*c).label().into()))
            }
            Event::EnergyAttached {
                creature, card: c, ..
            } => Some(play("energy", *c, Some(*creature), card(*c).label().into())),
            Event::Evolved { card: c, .. } => {
                Some(play("awaken", *c, None, tr("Awakened", "Éveil").into()))
            }
            Event::Retreated { creature, .. } => Some(play(
                "retreat",
                *creature,
                None,
                tr("Retreat", "Retraite").into(),
            )),
            Event::KnockedOut { card: c, .. } => {
                Some(play("ko", *c, None, tr("Knocked out", "K.O.").into()))
            }
            Event::Placed { card: c, .. } if turn > 0 => {
                Some(play("bench", *c, None, tr("Benched", "Sur le Banc").into()))
            }
            _ => None,
        };
        let is_discard = matches!(event, Event::EnergyDiscarded { .. });
        let retreat = matches!(event, Event::Retreated { .. });
        let moved = if new.is_some() {
            self.discards.with_value(|d| d.clone())
        } else {
            vec![]
        };
        if new.is_some() {
            self.discards.set_value(vec![]);
        }
        self.history.update(|h| {
            if let Some(mut p) = new {
                if retreat && !moved.is_empty() {
                    if let Some(prev) = h.last_mut() {
                        prev.lines.retain(|l| !moved.contains(l));
                    }
                    p.lines.extend(moved.iter().cloned());
                }
                h.push(p);
                if h.len() > 600 {
                    h.drain(..h.len() - 600);
                }
            }
            let Some(last) = h.last_mut().filter(|l| l.turn == turn) else {
                return;
            };
            if let Event::Damage {
                creature, amount, ..
            } = event
            {
                // The tile shows what the attack did to its target; other
                // Hexons it hit are in its lines.
                if last.kind == "attack" && last.target.is_none_or(|t| t == *creature) {
                    last.amount = Some(last.amount.unwrap_or(0) + amount);
                    last.target.get_or_insert(*creature);
                }
            }
            if !text.is_empty() && kind != "quiet" && kind != "turn" {
                last.lines.push(text.to_string());
            }
            if is_discard && !text.is_empty() {
                self.discards.update_value(|d| d.push(text.to_string()));
            }
        });
    }

    /// Log one event, start its effect, and return how long to pause (ms).
    fn play(&self, event: &Event, beat: Option<Beat>) -> u64 {
        crate::audio::duel_event(event);
        match event {
            Event::AttackUsed { .. } => self.attacking.set_value(true),
            Event::CoinFlip { .. } => {}
            // Anything after the flips means the attack's coins are done.
            _ => self.attacking.set_value(false),
        }
        let key = self.next_key();
        let text = event.describe(HUMAN);
        let kind = log_kind(event);
        self.heard.update_value(|h| h.push(event.clone()));
        let reason = match event {
            Event::PrizesSet { .. } => Some(tr("Who goes first?", "Qui commence ?").to_string()),
            Event::TurnStarted { .. } => Some(tr("Coin flip", "Pile ou face").into()),
            Event::BetweenTurns => Some(
                tr(
                    "Asleep: heads wakes it up",
                    "Endormi : face, il se réveille",
                )
                .into(),
            ),
            Event::EnergyDiscarded { .. } if self.turn_flip.get_value() => Some(
                tr(
                    "Confused: heads lets it retreat",
                    "Confus : face, il bat en retraite",
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
        self.record(event, &text, kind);
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
                        (true, false) => "You had no Basic Hexon and reshuffled",
                        (false, false) => "Opponent had no Basic Hexon and reshuffled",
                        (true, true) => "Aucun Hexon de Base : vous remélangez",
                        (false, true) => "Aucun Hexon de Base : l'adversaire remélange",
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
                        "Neither player had a Basic Hexon",
                        "Aucun joueur n'avait de Hexon de Base",
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
                        decide: beat.is_some_and(|b| b.class == "decide"),
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
                        "The first player draws but can't attack this turn",
                        "Le premier joueur pioche mais ne peut pas attaquer ce tour",
                    )
                    .into(),
                    None,
                    you(*player),
                );
                1300
            }
            Event::TurnStarted { player, turn } => {
                self.say(1, |l| l.intro);
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
                // R-DMG-01: weakness doubles, resistance takes 30 off.
                let stamps = [
                    (*weakness, ("weak", "×2")),
                    (*resistance, ("resist", "−30")),
                ]
                .into_iter()
                .filter_map(|(on, s)| on.then_some(s))
                .collect();
                let kind = if *amount == 0 { "none" } else { "dmg" };
                self.fx.update(|f| {
                    f.lunge = None;
                    f.hit = Some(*uid);
                    f.hp.insert(*uid, *hp);
                });
                self.pop_with(Some(*uid), format!("−{amount}"), kind, *amount, stamps);
                // A knock-out blow hands over to its KO while it still shakes
                // (the KO's hit-stop freezes it).
                let ko_next = self.queue.with_value(
                    |q| matches!(q.front(), Some(Event::KnockedOut { uid: u, .. }) if u == uid),
                );
                if *hp == 0 && ko_next {
                    260
                } else {
                    850
                }
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
            Event::KnockedOut { uid, player, .. } => {
                if *player == HUMAN {
                    self.say(2, |l| l.first_ko);
                }
                self.fx.update(|f| f.ko.push(*uid));
                950
            }
            Event::PrizeTaken { player, prize, .. } => {
                // On the ropes: the rival is one prize from losing (`real` is
                // already past this batch, so it knows the game goes on).
                let ropes = *player == HUMAN
                    && self.real.with_value(|g| {
                        g.as_ref().is_some_and(|g| {
                            g.winner().is_none()
                                && g.view(HUMAN).sides[HUMAN]
                                    .prizes
                                    .iter()
                                    .filter(|p| **p)
                                    .count()
                                    == 1
                        })
                    });
                if ropes {
                    self.say(4, |l| l.ropes);
                }
                self.taken.update_value(|t| t[*player] += 1);
                // Down to the last prize: the music's hot layer comes in now.
                if self.prizes_left(*player) == 1 {
                    crate::audio::intensity(1.0);
                }
                self.fx.update(|f| f.prize = Some((*player, *prize)));
                // The winning prize flies at half speed (a seal's slower still);
                // it chimes as it lands in the hand (86% into prize-fly).
                let ms = match beat.map(|b| b.class.split(' ').collect::<Vec<_>>()) {
                    Some(c) if c.contains(&"seal-final") => 1600,
                    Some(c) if c.contains(&"last-prize") => 1200,
                    _ => 600,
                };
                let land = if self.reduced_motion {
                    ms * 6 / 10
                } else {
                    ms * 86 / 100
                };
                let generation = self.generation.get_value();
                let (me, mine) = (*self, *player == HUMAN);
                set_timeout(
                    move || {
                        // Only your own prize is a reward; theirs just flies.
                        if me.generation.get_value() == generation && mine {
                            crate::audio::sfx("reward", 0.0);
                        }
                    },
                    Duration::from_millis(self.pace(land)),
                );
                ms
            }
            Event::EnergyAttached { uid, card: c, .. } => {
                let tone =
                    crate::art::card_color(card(*c)).map_or("colorless", crate::art::color_key);
                let key = self.next_key();
                self.spark.set(Some((key, *uid, tone)));
                let spark = self.spark;
                set_timeout(
                    move || {
                        if spark.get_untracked().is_some_and(|s| s.0 == key) {
                            spark.set(None);
                        }
                    },
                    Duration::from_millis(700),
                );
                0
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

/// How much quicker the opponent's turn plays: a tenth of the time held
/// down, a third with Fast on.
fn slowdown(theirs: bool, rush: bool, fast: bool) -> u64 {
    match (theirs, rush, fast) {
        (true, true, _) => 10,
        (true, false, true) => 3,
        _ => 1,
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beats_follow_the_events() {
        let flip = Event::CoinFlip {
            player: 0,
            heads: true,
        };
        let prize = Event::PrizeTaken {
            player: 0,
            prize: 0,
            card: CardId(0),
        };
        // Only the last flip of an attack settles it; a turn's flip never does.
        assert_eq!(beat(&flip, Some(&flip), true, 9, false), None);
        assert_eq!(
            beat(&flip, None, true, 9, false).map(|b| b.class),
            Some("decide")
        );
        assert_eq!(beat(&flip, None, false, 9, false), None);
        // The winning prize is the slow one.
        assert_eq!(
            beat(&prize, None, false, 1, false).map(|b| b.class),
            Some("prize")
        );
        assert_eq!(
            beat(&prize, None, false, 0, false).map(|b| b.class),
            Some("last-prize")
        );
        let ko = Event::KnockedOut {
            player: 1,
            slot: 0,
            uid: 3,
            card: CardId(0),
        };
        assert_eq!(
            beat(&ko, None, false, 9, false).map(|b| b.hold_ms),
            Some(80)
        );
        // A seal duel's winning prize lands with weight: a longer hold, the
        // seal's sting, its own class beside the last prize's.
        assert_eq!(
            beat(&prize, None, false, 0, true),
            Some(Beat {
                class: "last-prize seal-final",
                sfx: Some(("stinger", 0.0)),
                hold_ms: 240,
            })
        );
        assert_eq!(
            beat(&prize, None, false, 1, true).map(|b| b.class),
            Some("prize")
        );
        assert_eq!(
            beat(&prize, None, false, 0, false).map(|b| b.hold_ms),
            Some(0)
        );
    }
}
