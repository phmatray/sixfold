//! Deck legality (R-DECK-*).

use std::collections::HashMap;

use crate::card::{try_card, CardId, CardKind, EnergyCard};

pub const DECK_SIZE: usize = 60;
pub const MAX_COPIES: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckError {
    /// R-DECK-01: the deck has this many cards instead of 60.
    WrongSize(usize),
    UnknownCard(CardId),
    /// R-DECK-02
    NoBasic,
    /// R-DECK-03: more than 4 cards with this name.
    TooManyCopies(&'static str),
}

pub fn validate_deck(deck: &[CardId]) -> Result<(), DeckError> {
    if deck.len() != DECK_SIZE {
        return Err(DeckError::WrongSize(deck.len()));
    }
    let defs = deck
        .iter()
        .map(|&id| try_card(id).ok_or(DeckError::UnknownCard(id)))
        .collect::<Result<Vec<_>, _>>()?;
    if !defs.iter().any(|d| d.is_basic()) {
        return Err(DeckError::NoBasic);
    }
    let mut copies: HashMap<&str, usize> = HashMap::new();
    for d in defs {
        if matches!(d.kind, CardKind::Energy(EnergyCard::Basic(_))) {
            continue;
        }
        let n = copies.entry(d.name).or_default();
        *n += 1;
        if *n > MAX_COPIES {
            return Err(DeckError::TooManyCopies(d.name));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::*;

    fn with(cards: &[CardId]) -> Vec<CardId> {
        let mut d = cards.to_vec();
        d.resize(DECK_SIZE, GRASS_E);
        d
    }

    /// R-DECK-01
    #[test]
    fn deck_must_have_exactly_sixty() {
        assert_eq!(validate_deck(&deck()), Ok(()));
        let mut d = deck();
        d.pop();
        assert_eq!(validate_deck(&d), Err(DeckError::WrongSize(59)));
        d.extend([GRASS_E, GRASS_E]);
        assert_eq!(validate_deck(&d), Err(DeckError::WrongSize(61)));
    }

    /// R-DECK-02: a Trainer-as-Basic doesn't count.
    #[test]
    fn deck_without_basic_rejected() {
        assert_eq!(validate_deck(&with(&[STRAW_DECOY, MOSSBACK])), Err(DeckError::NoBasic));
        assert_eq!(validate_deck(&with(&[MOSSLING])), Ok(()));
    }

    /// R-DECK-03
    #[test]
    fn max_four_copies_by_name() {
        assert_eq!(validate_deck(&with(&[MOSSLING; 4])), Ok(()));
        assert_eq!(
            validate_deck(&with(&[MOSSLING; 5])),
            Err(DeckError::TooManyCopies("Mossling"))
        );
        let mut d = with(&[MOSSLING]);
        d[1..6].fill(FIELD_NOTES);
        assert_eq!(validate_deck(&d), Err(DeckError::TooManyCopies("Field Notes")));
    }

    /// R-DECK-03
    #[test]
    fn basic_energy_unlimited() {
        assert_eq!(validate_deck(&with(&[MOSSLING])), Ok(())); // 59 grass energy
    }

    /// R-DECK-03
    #[test]
    fn special_energy_limited_to_four() {
        assert_eq!(validate_deck(&with(&[MOSSLING, TWIN_E, TWIN_E, TWIN_E, TWIN_E])), Ok(()));
        assert_eq!(
            validate_deck(&with(&[MOSSLING, TWIN_E, TWIN_E, TWIN_E, TWIN_E, TWIN_E])),
            Err(DeckError::TooManyCopies("Twin Energy"))
        );
    }
}
