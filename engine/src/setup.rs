//! Setup (R-SETUP-*): deal, mulligan, placement, prizes, first player.

use std::collections::VecDeque;

use crate::action::{Event, RuleError};
use crate::game::{Game, Step};
use crate::player::{InPlay, Player};

pub const OPENING_HAND: usize = 7;

impl Game {
    /// Deal fresh hands and queue placement. Also used by sudden death.
    pub(crate) fn start_setup(&mut self, ev: &mut Vec<Event>) {
        self.deal_opening_hands(ev);
        self.steps = VecDeque::from([Step::Place(0), Step::Place(1), Step::PrizesAndCoin]);
    }

    /// R-SETUP-01: every card back in the deck, shuffle, draw 7.
    fn redeal(&mut self, p: usize, ev: &mut Vec<Event>) {
        self.players[p] = Player::new(self.decks[p].clone());
        // RULES: R-SETUP-01 — the source shuffles twice; one Fisher–Yates pass is
        // already uniform (R-ZONE-03).
        self.players[p].shuffle(&mut self.rng);
        ev.push(Event::Shuffled { player: p });
        for _ in 0..OPENING_HAND {
            self.draw(p, ev);
        }
    }

    /// R-SETUP-02/03: a hand without a real Basic is revealed and redrawn, with
    /// no bonus cards for the other side. Neither has one: start over.
    fn deal_opening_hands(&mut self, ev: &mut Vec<Event>) {
        loop {
            self.redeal(0, ev);
            self.redeal(1, ev);
            if !self.players[0].has_basic_in_hand() && !self.players[1].has_basic_in_hand() {
                for p in 0..2 {
                    ev.push(Event::Mulligan {
                        player: p,
                        hand: self.players[p].hand.clone(),
                    });
                }
                ev.push(Event::SetupRestarted);
                continue;
            }
            for p in 0..2 {
                while !self.players[p].has_basic_in_hand() {
                    ev.push(Event::Mulligan {
                        player: p,
                        hand: self.players[p].hand.clone(),
                    });
                    self.redeal(p, ev);
                }
            }
            return;
        }
    }

    /// R-SETUP-04: only Basics or Trainer-as-Basic cards can be placed.
    pub(crate) fn check_placeable(&self, p: usize, h: usize) -> Result<(), RuleError> {
        if self.hand_card(p, h)?.plays_as_basic() {
            Ok(())
        } else {
            Err(RuleError::NotABasic)
        }
    }

    /// R-ZONE-01: into the empty Active spot (clearing its conditions), else the
    /// Bench. Returns the slot.
    pub(crate) fn place(&mut self, p: usize, h: usize, ev: &mut Vec<Event>) -> usize {
        let pl = &mut self.players[p];
        let card = pl.hand.remove(h);
        let slot = if pl.active.is_none() {
            pl.active = Some(InPlay::new(card));
            pl.clear_active_state();
            0
        } else {
            pl.bench.push(InPlay::new(card));
            pl.bench.len()
        };
        ev.push(Event::Placed {
            player: p,
            slot,
            card,
        });
        slot
    }

    /// R-SETUP-05: prizes from the top of the deck, after placement, player 0
    /// first. R-SETUP-06: then one coin; heads, player 0 (the flipper) starts.
    pub(crate) fn set_prizes_and_flip(&mut self, ev: &mut Vec<Event>) {
        let n = self.prize_count;
        for p in 0..2 {
            let pl = &mut self.players[p];
            pl.prizes = (0..n).map(|_| pl.deck.pop()).collect();
            ev.push(Event::PrizesSet {
                player: p,
                count: n,
            });
        }
        let heads = self.flip(0, ev);
        self.turn_player = if heads { 0 } else { 1 };
        self.turn = 0;
        ev.push(Event::FirstPlayer {
            player: self.turn_player,
        });
        self.steps.push_front(Step::StartTurn);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{Action, PromptKind};
    use crate::card::CardId;
    use crate::fixtures::*;
    use crate::game::{Config, SetupError};

    fn one_basic_deck() -> Vec<CardId> {
        let mut d = vec![GRASS_E; 59];
        d.push(MOSSLING);
        d
    }

    /// First seed in 0..200 whose setup log satisfies `pred`.
    fn find_seed(decks: [Vec<CardId>; 2], pred: impl Fn(&Game) -> bool) -> Game {
        (0..200)
            .map(|seed| Game::new(decks.clone(), seed, Config::default()).unwrap())
            .find(pred)
            .expect("some seed matches")
    }

    fn mulligans(g: &Game, player: usize) -> Vec<&Vec<CardId>> {
        g.log
            .iter()
            .filter_map(|e| match e {
                Event::Mulligan { player: p, hand } if *p == player => Some(hand),
                _ => None,
            })
            .collect()
    }

    /// Place the first legal Active, no Bench, for both players.
    fn place_both(g: &mut Game) {
        for _ in 0..2 {
            let a = g.legal_actions()[0];
            g.apply(a).unwrap();
            g.apply(Action::Done).unwrap();
        }
    }

    /// R-SETUP-01
    #[test]
    fn setup_draws_seven_cards_each_player() {
        let g = Game::new([deck(), deck()], 1, Config::default()).unwrap();
        for p in 0..2 {
            assert_eq!(g.players[p].hand.len(), 7);
            assert_eq!(g.players[p].deck.len(), 53);
        }
        assert_eq!(g.prompt.as_ref().unwrap().kind, PromptKind::ChooseActive);
        assert_eq!(g.prompt.as_ref().unwrap().player, 0);
    }

    /// R-SETUP-02
    #[test]
    fn mulligan_redraws_until_basic_without_bonus_cards() {
        let g = find_seed([one_basic_deck(), deck()], |g| {
            !mulligans(g, 0).is_empty() && !g.log.contains(&Event::SetupRestarted)
        });
        assert!(mulligans(&g, 0).iter().all(|h| !h.contains(&MOSSLING)));
        assert!(g.players[0].hand.contains(&MOSSLING));
        assert_eq!(g.players[0].hand.len(), 7);
        assert_eq!(g.players[1].hand.len(), 7, "no bonus cards");
        assert_eq!(g.players[1].deck.len(), 53);
    }

    /// R-SETUP-02
    #[test]
    fn both_players_without_basic_restart_setup() {
        let g = find_seed([one_basic_deck(), one_basic_deck()], |g| {
            g.log.contains(&Event::SetupRestarted)
        });
        for p in 0..2 {
            assert!(g.players[p].hand.contains(&MOSSLING));
            assert_eq!(g.players[p].hand.len(), 7);
        }
    }

    /// R-SETUP-03
    #[test]
    fn mulligan_ignores_trainer_as_basic() {
        let mut d = one_basic_deck();
        d[..4].fill(STRAW_DECOY);
        let g = find_seed([d, deck()], |g| {
            mulligans(g, 0).iter().any(|h| h.contains(&STRAW_DECOY))
        });
        assert!(g.players[0].hand.contains(&MOSSLING));
    }

    /// R-SETUP-04
    #[test]
    fn setup_requires_exactly_one_basic_active() {
        let mut g = Game::new([deck(), deck()], 1, Config::default()).unwrap();
        g.players[0].hand = vec![MOSSLING, FIRE_E, STRAW_DECOY, FIELD_NOTES];
        assert_eq!(
            g.legal_actions(),
            vec![Action::PlayBasic(0), Action::PlayBasic(2)]
        );
        assert_eq!(g.apply(Action::Done), Err(RuleError::NotNow));
        g.apply(Action::PlayBasic(0)).unwrap();
        assert_eq!(g.players[0].active.as_ref().unwrap().top(), MOSSLING);
        assert_eq!(g.prompt.as_ref().unwrap().kind, PromptKind::ChooseBench);
        g.apply(Action::PlayBasic(1)).unwrap(); // the Trainer-as-Basic goes to the Bench
        assert_eq!(g.players[0].bench.len(), 1);
        g.apply(Action::Done).unwrap();
        assert_eq!(g.prompt.as_ref().unwrap().player, 1);
    }

    /// R-SETUP-04
    #[test]
    fn setup_rejects_evolution_card() {
        let mut g = Game::new([deck(), deck()], 1, Config::default()).unwrap();
        g.players[0].hand = vec![MOSSBACK, MOSSLING];
        assert_eq!(g.apply(Action::PlayBasic(0)), Err(RuleError::NotABasic));
        assert_eq!(g.players[0].hand.len(), 2);
    }

    /// R-SETUP-04
    #[test]
    fn setup_bench_limit_five() {
        let mut g = Game::new([deck(), deck()], 1, Config::default()).unwrap();
        g.players[0].hand = vec![MOSSLING; 7];
        for _ in 0..6 {
            g.apply(Action::PlayBasic(0)).unwrap();
        }
        assert_eq!(g.players[0].bench.len(), 5);
        assert_eq!(g.apply(Action::PlayBasic(0)), Err(RuleError::BenchFull));
        assert_eq!(g.legal_actions(), vec![Action::Done]);
    }

    /// R-SETUP-05
    #[test]
    fn prizes_set_after_placement_default_six() {
        let mut g = Game::new([deck(), deck()], 1, Config::default()).unwrap();
        assert!(g.players[0].prizes.is_empty());
        place_both(&mut g);
        for p in 0..2 {
            assert_eq!(g.players[p].prizes_left(), 6);
        }
        let first = g.turn_player;
        assert_eq!(g.players[first].deck.len(), 53 - 6 - 1);
        assert_eq!(g.players[1 - first].deck.len(), 53 - 6);
    }

    /// R-SETUP-05
    #[test]
    fn prize_count_must_be_one_to_six() {
        for (n, ok) in [(0, false), (1, true), (6, true), (7, false)] {
            let r = Game::new([deck(), deck()], 1, Config { prizes: n });
            assert_eq!(r.is_ok(), ok, "{n}");
            if !ok {
                assert_eq!(r.unwrap_err(), SetupError::PrizeCount(n));
            }
        }
        let mut g = Game::new([deck(), deck()], 1, Config { prizes: 1 }).unwrap();
        place_both(&mut g);
        assert_eq!(g.players[0].prizes.len(), 1);
    }

    /// R-SETUP-06
    #[test]
    fn coin_flip_heads_flipper_goes_first() {
        let mut g = Game::new([deck(), deck()], 1, Config::default()).unwrap();
        g.force_coins(&[true]);
        place_both(&mut g);
        assert_eq!(g.turn_player, 0);
        assert!(g.log.contains(&Event::CoinFlip {
            player: 0,
            heads: true
        }));
        assert_eq!(g.prompt.as_ref().unwrap().player, 0);
    }

    /// R-SETUP-06
    #[test]
    fn coin_flip_tails_opponent_goes_first() {
        let mut g = Game::new([deck(), deck()], 1, Config::default()).unwrap();
        g.force_coins(&[false]);
        place_both(&mut g);
        assert_eq!(g.turn_player, 1);
        assert_eq!(g.prompt.as_ref().unwrap().kind, PromptKind::MainPhase);
        assert_eq!(g.prompt.as_ref().unwrap().player, 1);
    }

    #[test]
    fn invalid_deck_rejected() {
        let r = Game::new([deck(), vec![MOSSLING]], 1, Config::default());
        assert!(matches!(r, Err(SetupError::InvalidDeck { player: 1, .. })));
    }
}
