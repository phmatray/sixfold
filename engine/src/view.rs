//! What one player is allowed to see (hidden information).

use crate::action::Prompt;
use crate::card::CardId;
use crate::condition::{ActiveEffects, Condition};
use crate::game::Game;
use crate::player::{InPlay, Player};

/// A read-only snapshot for one player: their own hand, and only counts for
/// everything face down (decks, prizes, the opponent's hand).
#[derive(Debug, Clone)]
pub struct View<'a> {
    pub me: usize,
    pub hand: &'a [CardId],
    /// Indexed by player number.
    pub sides: [Side<'a>; 2],
    pub turn: u32,
    pub turn_player: usize,
    /// The turn player already used this turn's energy attachment.
    pub energy_attached: bool,
    /// The pending decision, only when it is this player's (a deck-search
    /// prompt lists deck cards).
    pub prompt: Option<&'a Prompt>,
    /// Whose decision the game is waiting for.
    pub waiting_on: Option<usize>,
    pub winner: Option<usize>,
    pub sudden_death: bool,
}

#[derive(Debug, Clone)]
pub struct Side<'a> {
    pub hand_count: usize,
    pub deck_count: usize,
    /// One entry per prize position: still face down?
    pub prizes: Vec<bool>,
    pub discard: &'a [CardId],
    pub active: Option<&'a InPlay>,
    pub bench: &'a [InPlay],
    pub condition: Condition,
    pub effects: ActiveEffects,
    pub trainer_locked: bool,
}

impl<'a> Side<'a> {
    fn of(pl: &'a Player) -> Self {
        Side {
            hand_count: pl.hand.len(),
            deck_count: pl.deck.len(),
            prizes: pl.prizes.iter().map(Option::is_some).collect(),
            discard: &pl.discard,
            active: pl.active.as_ref(),
            bench: &pl.bench,
            condition: pl.condition,
            effects: pl.effects,
            trainer_locked: pl.trainer_locked,
        }
    }
}

impl Game {
    pub fn view(&self, me: usize) -> View<'_> {
        View {
            me,
            hand: &self.players[me].hand,
            sides: [Side::of(&self.players[0]), Side::of(&self.players[1])],
            turn: self.turn,
            turn_player: self.turn_player,
            energy_attached: self.energy_attached,
            prompt: self.prompt.as_ref().filter(|pr| pr.player == me),
            waiting_on: self.prompt.as_ref().map(|pr| pr.player),
            winner: self.winner,
            sudden_death: self.sudden_death,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::testkit::*;

    #[test]
    fn view_hides_opponent_hand_and_prizes() {
        let mut g = main_phase(0);
        set_hand(&mut g, 1, &[crate::fixtures::MOSSBACK; 3]);
        let v = g.view(0);
        assert_eq!(v.hand, g.players[0].hand.as_slice());
        assert_eq!(v.sides[1].hand_count, 3);
        assert_eq!(v.sides[0].prizes, vec![true; 6]);
        assert_eq!(v.sides[1].deck_count, 20);
        assert!(v.prompt.is_some());
        let v = g.view(1);
        assert_eq!(
            (v.prompt, v.waiting_on),
            (None, Some(0)),
            "the other player's prompt is hidden"
        );
    }
}
