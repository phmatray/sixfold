//! Test helpers: build a game straight into a main phase and poke at it.

use crate::action::{Action, Event, RuleError};
use crate::card::CardId;
use crate::fixtures::*;
use crate::game::{Game, Step};
use crate::player::InPlay;

/// Player 0's main phase on `turn` (turn start already ran: settle + draw).
/// Player 0: Mossling Active. Player 1: Pebblet Active. No Bench, empty hands
/// (plus the drawn card), 20 Grass energy in each deck, 6 prizes each.
pub fn main_phase(turn: u32) -> Game {
    let mut g = Game::blank([deck(), deck()], 1, 6);
    for (p, active) in [(0, MOSSLING), (1, PEBBLET)] {
        let pl = &mut g.players[p];
        pl.deck = vec![GRASS_E; 20];
        pl.prizes = vec![Some(FIRE_E); 6];
        pl.active = Some(InPlay::new(active));
    }
    g.turn = turn;
    g.steps.push_back(Step::StartTurn);
    g.run(&mut Vec::new());
    g.players[0].hand.clear();
    g
}

pub fn set_hand(g: &mut Game, p: usize, cards: &[CardId]) {
    g.players[p].hand = cards.to_vec();
}

/// A settled creature (may evolve) with the given energy.
pub fn creature(id: CardId, energy: &[CardId]) -> InPlay {
    let mut c = InPlay::new(id);
    c.energy = energy.to_vec();
    c.settled = true;
    c
}

pub fn ok(g: &mut Game, a: Action) -> Vec<Event> {
    g.apply(a)
        .unwrap_or_else(|e| panic!("{a:?} refused: {e:?}"))
}

pub fn err(g: &mut Game, a: Action) -> RuleError {
    g.apply(a).expect_err("should be refused")
}

pub fn prompt_of(g: &Game) -> (usize, crate::action::PromptKind) {
    let p = g.prompt().expect("a prompt");
    (p.player, p.kind.clone())
}
