//! Deck legality (R-DECK-*), deckbuilding hints and card sort orders. The
//! player-built deck lists of the deck editor are in `sixfold-meta`.

use std::collections::HashMap;

use crate::card::{card, try_card, CardId, CardKind, EnergyCard, Stage};

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

/// The first rule the deck breaks (see [`deck_errors`]).
pub fn validate_deck(deck: &[CardId]) -> Result<(), DeckError> {
    deck_errors(deck).into_iter().next().map_or(Ok(()), Err)
}

/// Every rule the deck breaks, in this order: its size, unknown cards, no
/// Basic, then each name over the 4-copy limit (in the order they go over).
pub fn deck_errors(deck: &[CardId]) -> Vec<DeckError> {
    let mut errors = vec![];
    if deck.len() != DECK_SIZE {
        errors.push(DeckError::WrongSize(deck.len()));
    }
    let mut defs = vec![];
    for &id in deck {
        match try_card(id) {
            Some(d) => defs.push(d),
            None if !errors.contains(&DeckError::UnknownCard(id)) => {
                errors.push(DeckError::UnknownCard(id))
            }
            None => {}
        }
    }
    if !defs.iter().any(|d| d.is_basic()) {
        errors.push(DeckError::NoBasic);
    }
    let mut copies: HashMap<&str, usize> = HashMap::new();
    for d in defs {
        if d.is_basic_energy() {
            continue;
        }
        let n = copies.entry(d.name).or_default();
        *n += 1;
        if *n == MAX_COPIES + 1 {
            errors.push(DeckError::TooManyCopies(d.name));
        }
    }
    errors
}

/// Below this many Basics, opening hands without one (and mulligans) get common.
pub const FEW_BASICS: usize = 8;
/// A 60-card deck with fewer energy cards than this stalls its attacks…
pub const FEW_ENERGY: usize = 18;
/// …and with more, draws too few Hexons and Gear cards.
pub const MUCH_ENERGY: usize = 32;

/// What a deck is made of, for the editor's composition hints.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Hints {
    pub total: usize,
    /// Real Basic Hexons (they count for R-DECK-02 and opening hands).
    pub basics: usize,
    pub stage1: usize,
    pub stage2: usize,
    /// Gear, Gear-as-Basic included.
    pub gears: usize,
    /// Energy cards, basic and special.
    pub energy: usize,
    /// Evolutions whose pre-evolution isn't in the deck, so they can never
    /// be played (R-EVO-01); each once, in deck order.
    pub stranded: Vec<CardId>,
}

/// Advice on a deck's balance, from its [`Hints`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Advice {
    /// Fewer than [`FEW_BASICS`] Basics (how many it has).
    FewBasics(usize),
    /// Fewer than [`FEW_ENERGY`] energy cards.
    FewEnergy(usize),
    /// More than [`MUCH_ENERGY`] energy cards.
    MuchEnergy(usize),
    /// This evolution's pre-evolution isn't in the deck.
    Stranded(CardId),
}

/// The composition of a deck list (unknown ids are skipped).
pub fn hints(deck: &[CardId]) -> Hints {
    let mut h = Hints {
        total: deck.len(),
        ..Hints::default()
    };
    let has = |name: &str| {
        deck.iter()
            .any(|&id| try_card(id).is_some_and(|d| d.name == name))
    };
    for &id in deck {
        let Some(d) = try_card(id) else { continue };
        match &d.kind {
            CardKind::Creature(c) => {
                match c.stage {
                    Stage::Basic => h.basics += 1,
                    Stage::Stage1 => h.stage1 += 1,
                    Stage::Stage2 => h.stage2 += 1,
                }
                if c.evolves_from.is_some_and(|f| !has(f)) && !h.stranded.contains(&id) {
                    h.stranded.push(id);
                }
            }
            CardKind::Gear(_) | CardKind::GearBasic(_) => h.gears += 1,
            CardKind::Energy(_) => h.energy += 1,
        }
    }
    h
}

impl Hints {
    /// What to look at. The balance (Basics, energy) is judged once the deck
    /// has its 60 cards; a stranded evolution is worth saying at any size.
    pub fn advice(&self) -> Vec<Advice> {
        let mut a = vec![];
        if self.total >= DECK_SIZE {
            if self.basics < FEW_BASICS {
                a.push(Advice::FewBasics(self.basics));
            }
            if self.energy < FEW_ENERGY {
                a.push(Advice::FewEnergy(self.energy));
            } else if self.energy > MUCH_ENERGY {
                a.push(Advice::MuchEnergy(self.energy));
            }
        }
        a.extend(self.stranded.iter().map(|&id| Advice::Stranded(id)));
        a
    }
}

/// Display orders for card lists; every order ends by card id, which keeps
/// evolution lines together.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    /// Creatures and basic energy by colour (each colour's energy after its
    /// creatures), then Gear cards and the special energy.
    Color,
    /// Basics, Stage 1, Stage 2, Gear cards, special energy, basic energy.
    Kind,
    /// Rare first, basic energy last.
    Rarity,
}

impl SortBy {
    /// Sort key for one card: `cards.sort_by_key(|&id| by.key(id))`.
    pub fn key(self, id: CardId) -> (u8, u8, CardId) {
        let c = card(id);
        let (kind, color) = match &c.kind {
            CardKind::Creature(cr) => (cr.stage as u8, cr.color as u8),
            CardKind::GearBasic(cr) => (3, cr.color as u8),
            CardKind::Gear(_) => (3, 7),
            CardKind::Energy(EnergyCard::DoubleColorless) => (4, 7),
            CardKind::Energy(EnergyCard::Basic(col)) => (5, *col as u8),
        };
        match self {
            SortBy::Color => (color, kind, id),
            SortBy::Kind => (kind, 0, id),
            SortBy::Rarity => (3 - c.rarity as u8, 0, id),
        }
    }
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

    /// R-DECK-02: a Gear-as-Basic doesn't count.
    #[test]
    fn deck_without_basic_rejected() {
        assert_eq!(
            validate_deck(&with(&[STRAW_DECOY, MOSSBACK])),
            Err(DeckError::NoBasic)
        );
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
        assert_eq!(
            validate_deck(&d),
            Err(DeckError::TooManyCopies("Field Notes"))
        );
    }

    /// R-DECK-03
    #[test]
    fn basic_energy_unlimited() {
        assert_eq!(validate_deck(&with(&[MOSSLING])), Ok(())); // 59 grass energy
    }

    /// R-DECK-03
    #[test]
    fn special_energy_limited_to_four() {
        assert_eq!(
            validate_deck(&with(&[MOSSLING, TWIN_E, TWIN_E, TWIN_E, TWIN_E])),
            Ok(())
        );
        assert_eq!(
            validate_deck(&with(&[MOSSLING, TWIN_E, TWIN_E, TWIN_E, TWIN_E, TWIN_E])),
            Err(DeckError::TooManyCopies("Twin Energy"))
        );
    }
}

#[cfg(test)]
mod hint_tests {
    use super::*;
    use crate::decks::PREBUILT;
    use crate::fixtures::*;

    #[test]
    fn hints_count_the_deck_and_find_stranded_evolutions() {
        let h = hints(&[
            MOSSLING,
            MOSSLING,
            MOSSGIANT,
            GREETER_ELDER,
            STRAW_DECOY,
            FIELD_NOTES,
            GRASS_E,
            GRASS_E,
            TWIN_E,
        ]);
        assert_eq!(
            (h.basics, h.stage1, h.stage2, h.gears, h.energy),
            (2, 1, 1, 2, 3)
        );
        // Mossgiant needs Mossback (absent); Greeter Elder needs Greeter.
        assert_eq!(h.stranded, vec![MOSSGIANT, GREETER_ELDER]);
        let h = hints(&[MOSSLING, MOSSBACK, MOSSGIANT, GREETER, GREETER_ELDER]);
        assert!(h.stranded.is_empty());
        assert_eq!(hints(&[]), Hints::default());
    }

    #[test]
    fn hints_advise_on_basics_energy_and_lines() {
        // Every prebuilt deck is sound: no advice at all.
        for p in PREBUILT {
            assert_eq!(hints(&p.cards()).advice(), vec![], "{}", p.name.get());
        }
        let mut d = vec![MOSSLING, MOSSGIANT];
        d.resize(DECK_SIZE, FIELD_NOTES);
        let a = hints(&d).advice();
        assert_eq!(
            a,
            vec![
                Advice::FewBasics(1),
                Advice::FewEnergy(0),
                Advice::Stranded(MOSSGIANT)
            ]
        );
        let mut d = vec![MOSSLING; 8];
        d.resize(DECK_SIZE, GRASS_E);
        assert_eq!(hints(&d).advice(), vec![Advice::MuchEnergy(52)]);
        // Too early to judge a deck that has barely started.
        assert_eq!(hints(&[MOSSLING]).advice(), vec![]);
    }

    #[test]
    fn deck_errors_lists_every_broken_rule() {
        let mut d = vec![MOSSBACK; 5];
        d.extend([FIELD_NOTES; 6]);
        assert_eq!(
            deck_errors(&d),
            vec![
                DeckError::WrongSize(11),
                DeckError::NoBasic,
                DeckError::TooManyCopies("Mossback"),
                DeckError::TooManyCopies("Field Notes"),
            ]
        );
        assert_eq!(validate_deck(&d), Err(DeckError::WrongSize(11)));
        assert_eq!(deck_errors(&deck()), vec![]);
    }

    #[test]
    fn unknown_cards_are_reported_once_and_never_as_not_owned() {
        let unknown = DeckError::UnknownCard(CardId(9999));
        assert_eq!(
            deck_errors(&[MOSSLING, CardId(9999), CardId(9999)])
                .into_iter()
                .filter(|&e| e == unknown)
                .count(),
            1
        );
        assert_eq!(hints(&[MOSSGIANT, MOSSGIANT]).stranded, vec![MOSSGIANT]);
    }
}
