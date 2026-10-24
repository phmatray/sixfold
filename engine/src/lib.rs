//! Core rules engine for the trading card game.
//!
//! Pure Rust, no WASM or UI dependencies, so it is fully unit-testable on the
//! host. This is the seed of the game model — only enough to prove the shape
//! (cards, decks, a two-player game state, drawing). Rules like attaching
//! energy, attacking, prizes and win conditions come next.

/// The nine energy types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Energy {
    Grass,
    Fire,
    Water,
    Lightning,
    Psychic,
    Fighting,
    Darkness,
    Metal,
    Colorless,
}

/// A card. Original model — no copyrighted names or data are baked in here;
/// callers supply their own (or placeholder) content.
#[derive(Debug, Clone, PartialEq)]
pub enum Card {
    Pokemon { name: String, hp: u32, kind: Energy },
    Energy(Energy),
    Trainer { name: String },
}

/// One player's side of the board. Draw moves a card from the top of the deck
/// into the hand.
#[derive(Debug, Default)]
pub struct Player {
    pub deck: Vec<Card>,
    pub hand: Vec<Card>,
    pub discard: Vec<Card>,
}

impl Player {
    pub fn new(deck: Vec<Card>) -> Self {
        Self { deck, hand: Vec::new(), discard: Vec::new() }
    }

    /// Draw one card from the top of the deck into the hand.
    /// Returns `false` when the deck is empty (a deck-out — the caller decides
    /// what that means for the game).
    pub fn draw(&mut self) -> bool {
        match self.deck.pop() {
            Some(card) => {
                self.hand.push(card);
                true
            }
            None => false,
        }
    }
}

/// The whole game: two players and whose turn it is.
#[derive(Debug)]
pub struct Game {
    pub players: [Player; 2],
    pub turn: u32,
}

impl Game {
    pub fn new(p1: Player, p2: Player) -> Self {
        Self { players: [p1, p2], turn: 0 }
    }

    /// Index of the player whose turn it currently is.
    pub fn active(&self) -> usize {
        (self.turn % 2) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draw_moves_card_from_deck_to_hand() {
        let mut p = Player::new(vec![
            Card::Energy(Energy::Fire),
            Card::Pokemon { name: "Ember Lizard".into(), hp: 60, kind: Energy::Fire },
        ]);
        assert_eq!(p.deck.len(), 2);
        assert!(p.draw());
        assert_eq!(p.hand.len(), 1);
        assert_eq!(p.deck.len(), 1);
    }

    #[test]
    fn draw_on_empty_deck_fails() {
        let mut p = Player::new(vec![]);
        assert!(!p.draw());
        assert!(p.hand.is_empty());
    }

    #[test]
    fn active_player_alternates_by_turn() {
        let g = Game::new(Player::default(), Player::default());
        assert_eq!(g.active(), 0);
        let g2 = Game { turn: 3, ..g };
        assert_eq!(g2.active(), 1);
    }
}
