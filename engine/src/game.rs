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
    pub(crate) retreat_failed: bool,
    /// R-EFFECT-02: this turn's accuracy coin, once flipped.
    pub(crate) accuracy: Option<bool>,
    /// The attack, trainer or Power being resolved.
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
            retreat_failed: false,
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
            all.push(Action::PlayTrainer(h));
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
        self.pending = None;
        self.steps.clear();
        ev.push(Event::GameOver { winner });
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

    /// Every card a player owns, wherever it is (60 for a legal deck).
    fn cards_owned(g: &Game, p: usize) -> usize {
        let pl = &g.players[p];
        let in_play: usize = pl
            .in_play()
            .map(|(_, c)| c.stack.len() + c.energy.len() + c.trainers.len())
            .sum();
        let limbo = g
            .pending
            .as_ref()
            .filter(|pd| pd.player == p && pd.card.is_some())
            .map_or(0, |_| 1);
        pl.deck.len() + pl.hand.len() + pl.discard.len() + pl.prizes_left() + in_play + limbo
    }

    /// Random legal play: 300 seeded games with the fixture decks never panic,
    /// never list an action `apply` refuses, keep every card, and always end.
    #[test]
    fn random_legal_play_always_finishes() {
        let decks = [power_deck(), attack_deck(), deck()];
        for seed in 0..300u64 {
            let pair = [
                decks[seed as usize % 3].clone(),
                decks[seed as usize / 3 % 3].clone(),
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
                g.apply(a)
                    .unwrap_or_else(|e| panic!("seed {seed}: listed {a:?} refused: {e:?}"));
                for p in 0..2 {
                    assert_eq!(
                        cards_owned(&g, p),
                        60,
                        "seed {seed}: player {p} after {a:?}"
                    );
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
}
