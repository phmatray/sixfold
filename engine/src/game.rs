//! The duel state machine.
//!
//! The game advances on its own through every step that needs no decision
//! (draws, coins, turn changes, knock-outs) and stops at a [`Prompt`]. Answer it
//! with [`Game::apply`]; [`Game::legal_actions`] lists exactly the accepted answers.

use std::collections::VecDeque;

use crate::action::{Action, Event, Prompt, PromptKind, RuleError};
use crate::card::{card, CardDef, CardId};
use crate::deck::{validate_deck, DeckError};
use crate::player::Player;
use crate::resolve::Pending;
use crate::rng::Rng;

pub const MAX_BENCH: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    /// Prize cards per player, 1 to 6 (R-SETUP-05).
    pub prizes: u8,
}

impl Default for Config {
    fn default() -> Self {
        Config { prizes: 6 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupError {
    InvalidDeck {
        player: usize,
        error: DeckError,
    },
    /// R-SETUP-05: the prize count must be 1 to 6.
    PrizeCount(u8),
}

/// The engine's to-do list: steps that run without a decision until one of
/// them raises a prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Step {
    /// Setup: this player places an Active, then Bench.
    Place(usize),
    /// Setup end: prizes, then the first-player coin.
    PrizesAndCoin,
    StartTurn,
    Main,
    EndTurn,
    BetweenTurns,
    NextTurn,
    TakePrizes {
        player: usize,
        count: u8,
    },
    Replace(usize),
    /// The next stage of `Game::pending` (see `resolve`).
    Resolve,
}

#[derive(Debug, Clone)]
pub struct Game {
    pub(crate) players: [Player; 2],
    /// The deck lists, kept for setup restarts and sudden death.
    pub(crate) decks: [Vec<CardId>; 2],
    pub(crate) prize_count: u8,
    pub(crate) sudden_death: bool,
    pub(crate) rng: Rng,
    /// Scripted coin results, used before the RNG (tests and scripted scenarios).
    pub(crate) forced_coins: VecDeque<bool>,
    /// Shared by both players, starts at 0 (R-TURN-03).
    pub(crate) turn: u32,
    pub(crate) turn_player: usize,
    /// Per-turn flags (R-TURN-03 step 4).
    pub(crate) energy_attached: bool,
    /// R-ENERGY-02: the one extra free-attach Power attachment is used.
    pub(crate) free_attached: bool,
    pub(crate) retreat_failed: bool,
    /// R-RETREAT-05: the turn player has retreated this turn.
    pub(crate) retreated: bool,
    /// R-EFFECT-02: this turn's accuracy coin, once flipped.
    pub(crate) accuracy: Option<bool>,
    /// The attack, Gear or Power being resolved.
    pub(crate) pending: Option<Pending>,
    pub(crate) prompt: Option<Prompt>,
    pub(crate) steps: VecDeque<Step>,
    pub(crate) winner: Option<usize>,
    pub(crate) log: Vec<Event>,
    /// The last `InPlay::uid` handed out.
    pub(crate) last_uid: u32,
}

impl Game {
    /// Validate both decks, then deal, mulligan and stop at player 0's
    /// `ChooseActive`. Player 0 is the side that flips the first-player coin.
    pub fn new(decks: [Vec<CardId>; 2], seed: u64, config: Config) -> Result<Game, SetupError> {
        if !(1..=6).contains(&config.prizes) {
            return Err(SetupError::PrizeCount(config.prizes));
        }
        for (player, deck) in decks.iter().enumerate() {
            validate_deck(deck).map_err(|error| SetupError::InvalidDeck { player, error })?;
        }
        let mut g = Game::blank(decks, seed, config.prizes);
        let mut ev = Vec::new();
        g.start_setup(&mut ev);
        g.run(&mut ev);
        g.log = ev;
        Ok(g)
    }

    pub(crate) fn blank(decks: [Vec<CardId>; 2], seed: u64, prizes: u8) -> Game {
        Game {
            players: [Player::default(), Player::default()],
            decks,
            prize_count: prizes,
            sudden_death: false,
            rng: Rng::new(seed),
            forced_coins: VecDeque::new(),
            turn: 0,
            turn_player: 0,
            energy_attached: false,
            free_attached: false,
            retreat_failed: false,
            retreated: false,
            accuracy: None,
            pending: None,
            prompt: None,
            steps: VecDeque::new(),
            winner: None,
            log: Vec::new(),
            last_uid: 0,
        }
    }

    /// The pending decision; `None` once the game is over.
    pub fn prompt(&self) -> Option<&Prompt> {
        self.prompt.as_ref()
    }

    pub fn winner(&self) -> Option<usize> {
        self.winner
    }

    pub fn turn(&self) -> u32 {
        self.turn
    }

    pub fn turn_player(&self) -> usize {
        self.turn_player
    }

    /// Every event since the game was created.
    pub fn log(&self) -> &[Event] {
        &self.log
    }

    /// Every action the current prompt accepts, and nothing else.
    pub fn legal_actions(&self) -> Vec<Action> {
        let Some(prompt) = &self.prompt else {
            return Vec::new();
        };
        let pl = &self.players[prompt.player];
        let mut all = vec![Action::Done, Action::Retreat, Action::Cancel];
        for h in 0..pl.hand.len() {
            all.push(Action::PlayBasic(h));
            all.push(Action::PlayGear(h));
            for slot in 0..=MAX_BENCH {
                all.push(Action::Evolve { hand: h, slot });
                all.push(Action::AttachEnergy { hand: h, slot });
            }
        }
        if let Some(active) = &pl.active {
            all.extend((0..active.creature().attacks.len()).map(Action::Attack));
            all.extend((0..active.energy.len()).map(Action::DiscardEnergy));
        }
        if let PromptKind::Choose { options, .. } = &prompt.kind {
            all.extend((0..options.len()).map(Action::Choose));
        }
        all.extend((0..=MAX_BENCH).map(Action::UsePower));
        all.extend((1..=MAX_BENCH).map(Action::Promote));
        all.extend((0..pl.prizes.len()).map(Action::TakePrize));
        all.retain(|&a| self.validate(a).is_ok());
        all
    }

    /// Answer the current prompt. An illegal action changes nothing.
    pub fn apply(&mut self, action: Action) -> Result<Vec<Event>, RuleError> {
        self.validate(action)?;
        let prompt = self.prompt.take().expect("validated");
        let mut ev = Vec::new();
        self.execute(prompt, action, &mut ev);
        self.run(&mut ev);
        self.log.extend(ev.iter().cloned());
        Ok(ev)
    }

    fn validate(&self, action: Action) -> Result<(), RuleError> {
        use PromptKind as K;
        let prompt = self.prompt.as_ref().ok_or(RuleError::GameOver)?;
        let p = prompt.player;
        let pl = &self.players[p];
        match (&prompt.kind, action) {
            (K::ChooseActive, Action::PlayBasic(h)) => self.check_placeable(p, h),
            (K::ChooseBench, Action::PlayBasic(h)) => {
                self.check_placeable(p, h)?;
                if pl.bench.len() >= MAX_BENCH {
                    return Err(RuleError::BenchFull);
                }
                Ok(())
            }
            (K::ChooseBench, Action::Done) => Ok(()),
            (K::MainPhase, a) => self.check_main(p, a),
            (K::ChooseRetreatEnergy { chosen, .. }, Action::DiscardEnergy(i)) => {
                if i >= pl.active.as_ref().map_or(0, |a| a.energy.len()) {
                    Err(RuleError::NoSuchEnergy)
                } else if chosen.contains(&i) {
                    Err(RuleError::EnergyAlreadyChosen)
                } else {
                    Ok(())
                }
            }
            (K::ChooseRetreatEnergy { .. } | K::ChooseRetreatTarget { .. }, Action::Cancel) => {
                Ok(())
            }
            (K::ChooseRetreatTarget { .. } | K::ChooseReplacement, Action::Promote(slot)) => {
                match pl.slot(slot) {
                    Some(c) if slot > 0 && c.hp > 0 => Ok(()),
                    _ => Err(RuleError::NoSuchSlot),
                }
            }
            (K::ChoosePrize { .. }, Action::TakePrize(i)) => match pl.prizes.get(i) {
                None => Err(RuleError::NoSuchPrize),
                Some(None) => Err(RuleError::PrizeTaken),
                Some(Some(_)) => Ok(()),
            },
            (K::Choose { options, .. }, Action::Choose(i)) if i >= options.len() => {
                Err(RuleError::NoSuchChoice)
            }
            (K::Choose { .. }, Action::Choose(_)) => Ok(()),
            (
                K::Choose {
                    can_cancel: true, ..
                },
                Action::Cancel,
            ) => Ok(()),
            _ => Err(RuleError::NotNow),
        }
    }

    fn execute(&mut self, prompt: Prompt, action: Action, ev: &mut Vec<Event>) {
        use PromptKind as K;
        let p = prompt.player;
        match (prompt.kind, action) {
            (K::ChooseActive | K::ChooseBench, Action::PlayBasic(h)) => {
                self.place(p, h, ev);
                self.ask(p, K::ChooseBench);
            }
            (K::ChooseBench, Action::Done) => {}
            (K::MainPhase, a) => self.main_action(p, a, ev),
            (K::ChooseRetreatEnergy { cost, chosen }, Action::DiscardEnergy(i)) => {
                self.choose_retreat_energy(p, cost, chosen, i)
            }
            (K::ChooseRetreatEnergy { .. } | K::ChooseRetreatTarget { .. }, Action::Cancel) => {
                // R-RETREAT-06: nothing was paid yet.
                self.steps.push_front(Step::Main);
            }
            (K::ChooseRetreatTarget { discard }, Action::Promote(slot)) => {
                self.finish_retreat(p, discard, slot, ev)
            }
            (K::ChoosePrize { count }, Action::TakePrize(i)) => self.take_prize(p, i, count, ev),
            (K::ChooseReplacement, Action::Promote(slot)) => self.promote(p, slot, ev),
            (
                K::Choose {
                    effect, options, ..
                },
                Action::Choose(i),
            ) => self.chosen(effect, options[i]),
            (K::Choose { .. }, Action::Cancel) => self.cancel_pending(),
            _ => unreachable!("validated"),
        }
    }

    /// Run steps until one needs a decision or the game ends.
    pub(crate) fn run(&mut self, ev: &mut Vec<Event>) {
        while self.prompt.is_none() && self.winner.is_none() {
            let Some(step) = self.steps.pop_front() else {
                return;
            };
            match step {
                Step::Place(p) => self.ask(p, PromptKind::ChooseActive),
                Step::PrizesAndCoin => self.set_prizes_and_flip(ev),
                Step::StartTurn => self.start_turn(ev),
                Step::Main => self.ask(self.turn_player, PromptKind::MainPhase),
                Step::EndTurn => self.end_turn(ev),
                Step::BetweenTurns => self.between_turns(ev),
                Step::NextTurn => self.next_turn(),
                Step::TakePrizes { player, count } => {
                    if count > 0 && self.players[player].prizes_left() > 0 {
                        self.ask(player, PromptKind::ChoosePrize { count });
                    }
                }
                Step::Replace(p) => {
                    let pl = &self.players[p];
                    if pl.active.is_none() && pl.has_live_bench() {
                        self.ask(p, PromptKind::ChooseReplacement);
                    }
                }
                Step::Resolve => self.resolve(ev),
            }
        }
    }

    pub(crate) fn ask(&mut self, player: usize, kind: PromptKind) {
        self.prompt = Some(Prompt { player, kind });
    }

    /// Queue steps to run next, in the given order.
    pub(crate) fn push_front(&mut self, steps: &[Step]) {
        for &s in steps.iter().rev() {
            self.steps.push_front(s);
        }
    }

    /// One coin, 50/50 (R-EFFECT-03).
    pub(crate) fn flip(&mut self, player: usize, ev: &mut Vec<Event>) -> bool {
        let heads = self
            .forced_coins
            .pop_front()
            .unwrap_or_else(|| self.rng.coin());
        ev.push(Event::CoinFlip { player, heads });
        heads
    }

    /// R-EFFECT-03: always flips all `n` coins; returns the number of heads.
    pub(crate) fn flip_many(&mut self, player: usize, n: u8, ev: &mut Vec<Event>) -> u8 {
        (0..n).filter(|_| self.flip(player, ev)).count() as u8
    }

    /// Script the next coin results (tests and scripted scenarios).
    #[cfg(test)]
    pub(crate) fn force_coins(&mut self, coins: &[bool]) {
        self.forced_coins.extend(coins);
    }

    /// A copy of the game that player `me` can't tell from the real one:
    /// the cards `me` can't see are dealt again from their pools (my deck and
    /// prizes; the opponent's hand, deck and prizes) and the coins come from
    /// `seed`. The Hard AI looks ahead only on such a copy, so it never learns
    /// a deck order, the opponent's hand or a future coin. Call it only where
    /// no prompt lists deck cards (the main phase).
    pub(crate) fn determinize(&self, me: usize, seed: u64) -> Game {
        // Field by field, to leave the log behind: it is the bulk of a late game.
        let mut g = Game {
            players: self.players.clone(),
            decks: self.decks.clone(),
            prize_count: self.prize_count,
            sudden_death: self.sudden_death,
            rng: Rng::new(seed),
            forced_coins: VecDeque::new(),
            turn: self.turn,
            turn_player: self.turn_player,
            energy_attached: self.energy_attached,
            free_attached: self.free_attached,
            retreat_failed: self.retreat_failed,
            retreated: self.retreated,
            accuracy: self.accuracy,
            pending: self.pending.clone(),
            prompt: self.prompt.clone(),
            steps: self.steps.clone(),
            winner: self.winner,
            log: Vec::new(),
            last_uid: self.last_uid,
        };
        let mut rng = Rng::new(seed ^ 0x9e37_79b9_7f4a_7c15);
        for (p, pl) in g.players.iter_mut().enumerate() {
            let hand = if p == me { 0 } else { pl.hand.len() };
            let mut pool = std::mem::take(&mut pl.deck);
            pool.extend(pl.hand.drain(pl.hand.len() - hand..));
            pool.extend(pl.prizes.iter().flatten());
            rng.shuffle(&mut pool);
            for prize in pl.prizes.iter_mut().flatten() {
                *prize = pool.pop().expect("pool holds the prizes");
            }
            pl.hand.extend(pool.drain(pool.len() - hand..));
            pl.deck = pool;
        }
        g
    }

    pub(crate) fn draw(&mut self, p: usize, ev: &mut Vec<Event>) -> Option<CardId> {
        let card = self.players[p].draw()?;
        ev.push(Event::CardDrawn { player: p, card });
        Some(card)
    }

    pub(crate) fn hand_card(&self, p: usize, h: usize) -> Result<&'static CardDef, RuleError> {
        self.players[p]
            .hand
            .get(h)
            .map(|&id| card(id))
            .ok_or(RuleError::NoSuchCard)
    }

    pub(crate) fn game_over(&mut self, winner: usize, ev: &mut Vec<Event>) {
        self.winner = Some(winner);
        self.prompt = None;
        // A Gear card whose effect ended the game was played: it goes to the
        // discard pile, as `finish` would have put it.
        if let Some(pd) = self.pending.take() {
            self.players[pd.player].discard.extend(pd.card);
        }
        self.steps.clear();
        ev.push(Event::GameOver { winner });
    }

    /// What holds between any two actions; a broken one is an engine bug. The
    /// AI playtests check it after every action, so each game they play is also
    /// a fuzz run.
    #[cfg(test)]
    pub(crate) fn check_invariants(&self) -> Result<(), String> {
        if self.winner.is_some() != self.prompt.is_none() {
            return Err("a game waits on a prompt exactly while nobody has won".into());
        }
        for (p, pl) in self.players.iter().enumerate() {
            if pl.bench.len() > MAX_BENCH {
                return Err(format!("player {p} has {} on the Bench", pl.bench.len()));
            }
            for (slot, c) in pl.in_play() {
                if c.hp > c.creature().hp {
                    return Err(format!(
                        "player {p} slot {slot}: {} HP over its {}",
                        c.hp,
                        c.creature().hp
                    ));
                }
                if c.hp == 0
                    && self
                        .prompt
                        .as_ref()
                        .is_some_and(|q| q.kind == PromptKind::MainPhase)
                {
                    return Err(format!(
                        "player {p} slot {slot}: knocked out but still in play"
                    ));
                }
            }
            // Every card stays its owner's and is somewhere: the zones hold
            // exactly the deck list.
            let mut held: Vec<CardId> = [&pl.deck, &pl.hand, &pl.discard]
                .into_iter()
                .flatten()
                .copied()
                .collect();
            held.extend(pl.prizes.iter().flatten());
            for (_, c) in pl.in_play() {
                held.extend(c.stack.iter().chain(&c.energy).chain(&c.gears));
            }
            held.extend(
                self.pending
                    .iter()
                    .filter(|q| q.player == p)
                    .flat_map(|q| q.card),
            );
            let mut list = self.decks[p].clone();
            held.sort_unstable();
            list.sort_unstable();
            if held != list {
                return Err(format!(
                    "player {p}'s zones hold {} cards, not their deck of {}",
                    held.len(),
                    list.len()
                ));
            }
        }
        Ok(())
    }
}

/// How a finished duel was won (R-KO-06, R-TURN-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    /// The winner took their last prize card.
    Prizes,
    /// The loser had to draw from an empty deck.
    DeckOut,
    /// The loser was left with no creature in play.
    NoCreatures,
}

/// A duel in a few numbers, for stats and achievements (`docs/WORLD_DESIGN.md`
/// §12.2). Per-side arrays are indexed by player.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameSummary {
    pub winner: Option<usize>,
    /// None while the duel is under way.
    pub end: Option<End>,
    /// Prize cards each side started with (before any sudden death).
    pub prizes: u8,
    /// Prize cards each side took.
    pub taken: [u32; 2],
    /// Knock-outs each side scored, and how many of them on the Bench.
    pub kos: [u32; 2],
    pub bench_kos: [u32; 2],
    /// Stage 2 creatures each side evolved into.
    pub stage2: [u32; 2],
    /// Each side's longest run of heads on its own coins (in turns: the
    /// first-player coin doesn't count).
    pub heads_run: [u32; 2],
    /// The biggest prize deficit the winner came back from (prizes still to
    /// take minus the opponent's).
    pub deficit: u32,
    /// The winner once needed four prizes or more while the loser needed one
    /// (in the game that decided it: a sudden death starts the race again).
    pub brink: bool,
    pub sudden_death: bool,
}

impl Game {
    /// The duel so far, read from its log.
    pub fn summary(&self) -> GameSummary {
        let mut s = GameSummary {
            winner: self.winner,
            prizes: self.prize_count,
            sudden_death: self.sudden_death,
            ..GameSummary::default()
        };
        let mut left: [Option<u32>; 2] = [None; 2];
        let mut set = false;
        let mut run = [0; 2];
        // Coins of turns only: the first-player coin is nobody's luck.
        let mut playing = false;
        // Prizes still to take, (needed by the winner, by the loser), over time.
        let mut gaps = vec![];
        let mut decked = None;
        for e in &self.log {
            match *e {
                Event::PrizesSet { player, count } => {
                    if !set {
                        s.prizes = count;
                        set = true;
                    }
                    left[player] = Some(u32::from(count));
                    decked = None;
                }
                Event::PrizeTaken { player, .. } => {
                    s.taken[player] += 1;
                    left[player] = left[player].map(|n| n.saturating_sub(1));
                }
                Event::KnockedOut { player, slot, .. } => {
                    s.kos[1 - player] += 1;
                    s.bench_kos[1 - player] += u32::from(slot > 0);
                }
                Event::Evolved {
                    player, card: c, ..
                } => {
                    let two = card(c)
                        .creature()
                        .is_some_and(|cr| cr.stage == crate::card::Stage::Stage2);
                    s.stage2[player] += u32::from(two);
                }
                Event::TurnStarted { .. } => playing = true,
                // A restart: a new setup coin, and the old prize race is over.
                Event::SuddenDeath => {
                    playing = false;
                    gaps.clear();
                    left = [None; 2];
                }
                Event::CoinFlip { player, heads } if playing => {
                    run[player] = if heads { run[player] + 1 } else { 0 };
                    s.heads_run[player] = s.heads_run[player].max(run[player]);
                }
                Event::DeckOut { player } => decked = Some(player),
                _ => {}
            }
            if let [Some(a), Some(b)] = left {
                gaps.push([a, b]);
            }
        }
        let Some(w) = self.winner else {
            return s;
        };
        let l = 1 - w;
        for g in gaps {
            s.deficit = s.deficit.max(g[w].saturating_sub(g[l]));
            s.brink |= g[w] >= 4 && g[l] == 1;
        }
        // The last prize wins even when it also empties the loser's board.
        s.end = Some(if decked == Some(l) {
            End::DeckOut
        } else if self.players[w].prizes_left() == 0 {
            End::Prizes
        } else {
            End::NoCreatures
        });
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::*;
    use crate::rng::Rng;

    /// Play `n` actions, picking a varying legal one each time.
    fn play(g: &mut Game, n: usize) {
        for i in 0..n {
            let legal = g.legal_actions();
            if legal.is_empty() {
                return;
            }
            g.apply(legal[i * 7 % legal.len()]).unwrap();
        }
    }

    /// Same seed + same actions = same game.
    #[test]
    fn same_seed_same_game() {
        let mut a = Game::new([deck(), deck()], 9, Config::default()).unwrap();
        let mut b = Game::new([deck(), deck()], 9, Config::default()).unwrap();
        play(&mut a, 300);
        play(&mut b, 300);
        assert!(a.turn > 2);
        assert_eq!(a.log, b.log);
        assert_eq!(a.legal_actions(), b.legal_actions());
    }

    /// Every listed action is accepted; an unlisted one is refused.
    #[test]
    fn legal_actions_match_apply() {
        let mut g = Game::new([deck(), deck()], 3, Config::default()).unwrap();
        for _ in 0..60 {
            let legal = g.legal_actions();
            if legal.is_empty() {
                break;
            }
            for a in [
                Action::Done,
                Action::Cancel,
                Action::Retreat,
                Action::PlayBasic(0),
                Action::Attack(0),
            ] {
                assert_eq!(legal.contains(&a), g.clone().apply(a).is_ok(), "{a:?}");
            }
            g.apply(*legal.last().unwrap()).unwrap();
        }
    }

    /// UI identity (`InPlay::uid`): kept through evolving and retreating; a
    /// newly placed creature gets a fresh one.
    #[test]
    fn creature_uid_stable_while_in_play() {
        use crate::player::InPlay;
        use crate::testkit::*;
        let mut g = main_phase(2);
        g.last_uid = 50;
        g.players[0].active.as_mut().unwrap().energy = vec![GRASS_E];
        g.players[0].bench.push(InPlay {
            uid: 41,
            ..creature(MOSSLING, &[])
        });
        set_hand(&mut g, 0, &[MOSSBACK, EMBERKIT]);
        let ev = ok(&mut g, Action::Evolve { hand: 0, slot: 1 });
        assert!(matches!(
            ev[..],
            [Event::Evolved {
                uid: 41,
                slot: 1,
                ..
            }]
        ));
        let ev = ok(&mut g, Action::PlayBasic(0));
        assert!(matches!(
            ev[..],
            [Event::Placed {
                uid: 51,
                slot: 2,
                ..
            }]
        ));
        ok(&mut g, Action::Retreat);
        ok(&mut g, Action::DiscardEnergy(0));
        let ev = ok(&mut g, Action::Promote(1));
        assert!(ev.iter().any(|e| matches!(
            e,
            Event::Retreated {
                uid: 41,
                slot: 1,
                creature: MOSSBACK,
                ..
            }
        )));
        assert_eq!(g.players[0].active.as_ref().unwrap().uid, 41);
    }

    /// Random legal play: 300 seeded games with the fixture and prebuilt decks
    /// never panic, never list an action `apply` refuses, hold the invariants
    /// after every action, and always end.
    #[test]
    fn random_legal_play_always_finishes() {
        let decks: Vec<Vec<CardId>> = [power_deck(), attack_deck(), deck()]
            .into_iter()
            .chain(crate::decks::PREBUILT.iter().map(|d| d.cards()))
            .collect();
        let n = decks.len();
        for seed in 0..300u64 {
            let pair = [
                decks[seed as usize % n].clone(),
                decks[seed as usize / n % n].clone(),
            ];
            let mut g = Game::new(pair, seed, Config::default()).unwrap();
            let mut pick = Rng::new(seed ^ 0x5eed);
            let mut actions = 0;
            while g.winner().is_none() {
                assert!(g.turn() < 500, "seed {seed}: turn cap hit");
                actions += 1;
                assert!(actions < 100_000, "seed {seed}: action cap hit");
                let legal = g.legal_actions();
                assert!(
                    !legal.is_empty(),
                    "seed {seed}: no action at {:?}",
                    g.prompt()
                );
                let a = legal[pick.below(legal.len() as u32) as usize];
                if seed % 10 == 0 {
                    // Every label and log line can be built, for both viewers.
                    let v = g.view(g.prompt().unwrap().player);
                    legal
                        .iter()
                        .for_each(|&a| assert!(!v.describe_action(a).contains('?')));
                    v.prompt.unwrap().kind.to_string();
                }
                let ev = g
                    .apply(a)
                    .unwrap_or_else(|e| panic!("seed {seed}: listed {a:?} refused: {e:?}"));
                if seed % 10 == 0 {
                    ev.iter().for_each(|e| {
                        e.describe(0);
                        e.describe(1);
                    });
                }
                if let Err(e) = g.check_invariants() {
                    panic!("seed {seed}: after {a:?}: {e}");
                }
                let mut uids: Vec<u32> = (0..2)
                    .flat_map(|p| g.players[p].in_play().map(|(_, c)| c.uid))
                    .collect();
                let n = uids.len();
                uids.sort_unstable();
                uids.dedup();
                assert_eq!(uids.len(), n, "seed {seed}: uids must be unique");
                assert!(!uids.contains(&0), "seed {seed}: uid 0 in play");
            }
            assert!(matches!(g.log().last(), Some(Event::GameOver { .. })));
        }
    }

    /// The AI's imagined copy: same public state and zone sizes, but the
    /// cards player 0 can't see (its deck and prizes, the opponent's hand,
    /// deck and prizes) are dealt again, and the coins come from a new seed.
    #[test]
    fn determinize_hides_what_the_player_cannot_see() {
        use crate::decks::{ember_rush, tide_mind};
        let mut g = Game::new([ember_rush(), tide_mind()], 5, Config::default()).unwrap();
        play(&mut g, 40);
        let d = g.determinize(0, 99);
        let sorted = |mut v: Vec<CardId>| {
            v.sort_by_key(|c| c.0);
            v
        };
        let hidden = |g: &Game, p: usize, hand: bool| {
            let pl = &g.players[p];
            let mut v: Vec<CardId> = pl
                .deck
                .iter()
                .chain(pl.prizes.iter().flatten())
                .copied()
                .collect();
            if hand {
                v.extend(&pl.hand);
            }
            sorted(v)
        };
        for p in 0..2 {
            let (a, b) = (&g.players[p], &d.players[p]);
            assert_eq!(a.deck.len(), b.deck.len());
            assert_eq!(a.hand.len(), b.hand.len());
            assert_eq!(
                a.prizes.iter().map(Option::is_some).collect::<Vec<_>>(),
                b.prizes.iter().map(Option::is_some).collect::<Vec<_>>()
            );
            assert_eq!(
                (&a.active, &a.bench, &a.discard),
                (&b.active, &b.bench, &b.discard)
            );
            assert_eq!(hidden(&g, p, p == 1), hidden(&d, p, p == 1));
        }
        assert_eq!(g.players[0].hand, d.players[0].hand, "my own hand stays");
        assert_ne!(
            g.players[0].deck, d.players[0].deck,
            "my deck order is hidden"
        );
        assert_ne!(
            (&g.players[1].hand, &g.players[1].deck),
            (&d.players[1].hand, &d.players[1].deck),
            "the opponent's hand and deck are hidden"
        );
        assert_eq!(
            (g.turn, g.turn_player, &g.prompt),
            (d.turn, d.turn_player, &d.prompt)
        );
        assert_ne!(g.rng.clone().next_u32(), d.rng.clone().next_u32());
    }
}

#[cfg(test)]
mod summary_tests {
    use super::*;
    use crate::fixtures::*;
    use crate::testkit::*;

    /// A main phase whose log starts the way a dealt game's does: prizes set.
    fn dealt(left: [u8; 2]) -> Game {
        let mut g = main_phase(0);
        for (player, count) in left.into_iter().enumerate() {
            g.players[player].prizes = vec![Some(FIRE_E); count.into()];
            g.log.push(Event::PrizesSet { player, count });
        }
        g
    }

    #[test]
    fn summary_of_a_deck_out_win() {
        let mut g = dealt([6, 6]);
        g.players[1].deck.clear();
        ok(&mut g, Action::Done);
        let s = g.summary();
        assert_eq!(
            (s.winner, s.end, s.prizes),
            (Some(0), Some(End::DeckOut), 6)
        );
        assert_eq!(
            (s.taken, s.kos, s.deficit, s.brink),
            ([0, 0], [0, 0], 0, false)
        );
    }

    #[test]
    fn summary_of_an_empty_bench_win() {
        let mut g = dealt([6, 6]);
        attack_with(&mut g, 200, &[]);
        let s = g.summary();
        assert_eq!((s.winner, s.end), (Some(0), Some(End::NoCreatures)));
        assert_eq!((s.kos, s.bench_kos, s.taken), ([1, 0], [0, 0], [1, 0]));
    }

    #[test]
    fn the_last_prize_is_a_prize_win_even_on_an_empty_board() {
        let mut g = dealt([1, 6]);
        attack_with(&mut g, 200, &[]);
        let s = g.summary();
        assert_eq!(
            (s.winner, s.end, s.taken),
            (Some(0), Some(End::Prizes), [1, 0])
        );
    }

    #[test]
    fn a_sudden_death_starts_the_prize_race_again() {
        let mut g = dealt([4, 1]);
        g.log.push(Event::SuddenDeath);
        g.sudden_death = true;
        for player in [0, 1] {
            g.log.push(Event::PrizesSet { player, count: 1 });
        }
        g.players[1].deck.clear();
        ok(&mut g, Action::Done);
        let s = g.summary();
        assert_eq!((s.winner, s.brink, s.sudden_death), (Some(0), false, true));
    }

    #[test]
    fn summary_of_a_comeback() {
        // Four prizes to take while the opponent needs one; then they deck out.
        let mut g = dealt([4, 1]);
        g.players[1].deck.clear();
        ok(&mut g, Action::Done);
        let s = g.summary();
        assert_eq!((s.winner, s.deficit, s.brink), (Some(0), 3, true));
        // An unfinished duel has no end and no comeback.
        let s = dealt([4, 1]).summary();
        assert_eq!((s.winner, s.end, s.brink), (None, None, false));
    }

    #[test]
    fn summary_counts_stage2s_and_runs_of_heads() {
        let mut g = dealt([6, 6]);
        let coin = |player, heads| Event::CoinFlip { player, heads };
        g.log.extend([
            // The first-player coin, before any turn, is no one's run.
            coin(0, true),
            coin(0, true),
            Event::TurnStarted { player: 0, turn: 0 },
            coin(0, true),
            coin(0, true),
            coin(1, false),
            coin(0, true),
            coin(0, false),
            coin(0, true),
            Event::Evolved {
                player: 0,
                slot: 0,
                uid: 1,
                card: MOSSGIANT,
            },
            Event::Evolved {
                player: 0,
                slot: 0,
                uid: 1,
                card: MOSSBACK,
            },
        ]);
        let s = g.summary();
        assert_eq!((s.heads_run, s.stage2), ([3, 0], [1, 0]));
    }
}
