//! Deck legality (R-DECK-*) and player-built deck lists for the deck editor.

use std::collections::{BTreeMap, HashMap};

use crate::card::{card, try_card, CardId, CardKind, Color, EnergyCard};
use crate::collection::{counts_body, parse_counts, sign, unsign, Collection, LoadError};

/// The deck-list save string version this build writes and reads.
const DECK_VERSION: &str = "d1";

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
        if d.is_basic_energy() {
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

/// A deck a player builds from their [`Collection`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeckList {
    pub name: String,
    pub cards: Vec<CardId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckListError {
    /// Breaks a deck rule (R-DECK-*).
    Illegal(DeckError),
    /// Uses more copies of this card than the collection owns.
    NotOwned(CardId),
}

/// Why [`DeckList::check_add`] refuses a card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddError {
    UnknownCard,
    /// The deck already has 60 cards.
    DeckFull,
    /// The deck already has 4 cards with this name (R-DECK-03).
    TooManyCopies,
    /// Every owned copy is already in the deck.
    NotOwned,
}

/// Display orders for card lists; every order ends by card id, which keeps
/// evolution lines together.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    /// Creatures and basic energy by colour (each colour's energy after its
    /// creatures), then trainers and the special energy.
    Color,
    /// Basics, Stage 1, Stage 2, trainers, special energy, basic energy.
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
            CardKind::TrainerBasic(cr) => (3, cr.color as u8),
            CardKind::Trainer(_) => (3, 7),
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

impl DeckList {
    pub fn new(name: impl Into<String>) -> Self {
        DeckList {
            name: name.into(),
            cards: vec![],
        }
    }

    /// Legal (R-DECK-*) and buildable from the collection.
    pub fn validate(&self, owned: &Collection) -> Result<(), DeckListError> {
        validate_deck(&self.cards).map_err(DeckListError::Illegal)?;
        match owned.missing(&self.cards).first() {
            Some(&(id, _)) => Err(DeckListError::NotOwned(id)),
            None => Ok(()),
        }
    }

    /// Cards with this name in the deck (the 4-copy rule counts names).
    pub fn copies(&self, name: &str) -> usize {
        self.cards
            .iter()
            .filter(|&&id| card(id).name == name)
            .count()
    }

    /// Each distinct card with its number of copies, in id order.
    pub fn counts(&self) -> Vec<(CardId, usize)> {
        let mut m: BTreeMap<CardId, usize> = BTreeMap::new();
        for &id in &self.cards {
            *m.entry(id).or_default() += 1;
        }
        m.into_iter().collect()
    }

    /// [`counts`](Self::counts) in a display order.
    pub fn sorted(&self, by: SortBy) -> Vec<(CardId, usize)> {
        let mut v = self.counts();
        v.sort_by_key(|&(id, _)| by.key(id));
        v
    }

    /// Whether one more copy of `id` may go in.
    pub fn check_add(&self, id: CardId, owned: &Collection) -> Result<(), AddError> {
        let def = try_card(id).ok_or(AddError::UnknownCard)?;
        if self.cards.len() >= DECK_SIZE {
            return Err(AddError::DeckFull);
        }
        if !def.is_basic_energy() && self.copies(def.name) >= MAX_COPIES {
            return Err(AddError::TooManyCopies);
        }
        if self.cards.iter().filter(|&&c| c == id).count() >= owned.count(id) {
            return Err(AddError::NotOwned);
        }
        Ok(())
    }

    /// Adds one copy if [`check_add`](Self::check_add) allows it.
    pub fn add(&mut self, id: CardId, owned: &Collection) -> Result<(), AddError> {
        self.check_add(id, owned)?;
        self.cards.push(id);
        Ok(())
    }

    /// Removes one copy; false when the deck has none.
    pub fn remove(&mut self, id: CardId) -> bool {
        match self.cards.iter().rposition(|&c| c == id) {
            Some(i) => {
                self.cards.remove(i);
                true
            }
            None => false,
        }
    }

    /// Fills the deck up to 60 with basic energy, shared between the colours
    /// of its creatures in proportion to how many creature cards each colour
    /// has (energy already in the deck counts). Returns how many were added;
    /// none when the deck has no coloured creature.
    /// `d1;<id>:<count>,...;<name>;<checksum>`, in the style of
    /// [`Collection::save`]: ids ascending, FNV-1a checksum. Line breaks in the
    /// name become spaces, so saved lists can be stored one per line.
    pub fn save(&self) -> String {
        let name: String = self
            .name
            .chars()
            .map(|c| if c.is_control() { ' ' } else { c })
            .collect();
        sign(&format!(
            "{DECK_VERSION};{};{name}",
            counts_body(self.counts().into_iter())
        ))
    }

    /// Reads a [`save`](Self::save) string. Card order isn't kept (ids come
    /// back ascending); ids the set doesn't have are dropped and returned.
    pub fn load(s: &str) -> Result<(DeckList, Vec<(CardId, usize)>), LoadError> {
        let body = unsign(s, DECK_VERSION)?;
        let (counts, name) = body.split_once(';').ok_or(LoadError::Malformed)?;
        let (known, dropped) = parse_counts(counts)?;
        let cards = known
            .into_iter()
            .flat_map(|(id, n)| std::iter::repeat_n(id, n))
            .collect();
        Ok((
            DeckList {
                name: name.to_string(),
                cards,
            },
            dropped,
        ))
    }

    pub fn fill_energy(&mut self) -> usize {
        let (mut weight, mut have) = ([0usize; 6], [0usize; 6]);
        for &id in &self.cards {
            match &card(id).kind {
                CardKind::Creature(cr) | CardKind::TrainerBasic(cr)
                    if cr.color != Color::Colorless =>
                {
                    weight[cr.color as usize] += 1
                }
                CardKind::Energy(EnergyCard::Basic(c)) => have[*c as usize] += 1,
                _ => {}
            }
        }
        let start = self.cards.len();
        while self.cards.len() < DECK_SIZE {
            // D'Hondt: the next energy goes to the highest weight / (have + 1).
            let best = (0..6).filter(|&c| weight[c] > 0).max_by(|&a, &b| {
                (weight[a] * (have[b] + 1))
                    .cmp(&(weight[b] * (have[a] + 1)))
                    .then(b.cmp(&a))
            });
            let Some(c) = best else { break };
            have[c] += 1;
            self.cards
                .push(crate::set::basic_energy(Color::ALL[c]).expect("set has every basic energy"));
        }
        self.cards.len() - start
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

    /// R-DECK-02: a Trainer-as-Basic doesn't count.
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
mod deck_list_tests {
    use super::*;
    use crate::collection::starter_collection;
    use crate::decks::{EMBER_RUSH, STARTER};
    use crate::set::*;

    fn list(cards: &[CardId]) -> DeckList {
        DeckList {
            name: "Test".into(),
            cards: cards.to_vec(),
        }
    }

    #[test]
    fn validate_checks_rules_then_ownership() {
        let ember = list(&STARTER.cards());
        assert_eq!(ember.validate(&starter_collection()), Ok(()));
        assert_eq!(
            ember.validate(&Collection::new()),
            Err(DeckListError::NotOwned(CINDREL))
        );
        let short = list(&ember.cards[1..]);
        assert_eq!(
            short.validate(&starter_collection()),
            Err(DeckListError::Illegal(DeckError::WrongSize(59)))
        );
    }

    #[test]
    fn check_add_explains_refusals() {
        let mut owned = Collection::new();
        owned.add(CINDREL, 5);
        owned.add(PUDDOCK, 1);
        let mut d = DeckList::new("Test");
        for _ in 0..4 {
            assert_eq!(d.add(CINDREL, &owned), Ok(()));
        }
        assert_eq!(d.check_add(CINDREL, &owned), Err(AddError::TooManyCopies));
        assert_eq!(d.add(PUDDOCK, &owned), Ok(()));
        assert_eq!(d.check_add(PUDDOCK, &owned), Err(AddError::NotOwned));
        assert_eq!(d.check_add(FLARROW, &owned), Err(AddError::NotOwned));
        assert_eq!(
            d.check_add(CardId(9999), &owned),
            Err(AddError::UnknownCard)
        );
        while d.cards.len() < DECK_SIZE {
            assert_eq!(d.add(FIRE_ENERGY, &owned), Ok(()));
        }
        assert_eq!(d.check_add(FIRE_ENERGY, &owned), Err(AddError::DeckFull));
        assert!(d.remove(CINDREL) && d.copies("Cindrel") == 3);
        assert!(!d.remove(FLARROW));
        assert_eq!(d.add(FIRE_ENERGY, &owned), Ok(()));
        assert_eq!(d.validate(&owned), Ok(()));
    }

    #[test]
    fn counts_and_sorted_views() {
        let d = list(&[
            FIRE_ENERGY,
            PYREQUILL,
            CINDREL,
            CINDREL,
            SNARE_HOOK,
            KNOLLOP,
            TWOFOLD_ENERGY,
        ]);
        assert_eq!(d.copies("Cindrel"), 2);
        assert_eq!(d.counts()[0], (CINDREL, 2));
        let order = |by| {
            d.sorted(by)
                .into_iter()
                .map(|(id, _)| id)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            order(SortBy::Kind),
            [
                CINDREL,
                KNOLLOP,
                PYREQUILL,
                SNARE_HOOK,
                TWOFOLD_ENERGY,
                FIRE_ENERGY
            ]
        );
        assert_eq!(
            order(SortBy::Color),
            [
                CINDREL,
                PYREQUILL,
                FIRE_ENERGY,
                KNOLLOP,
                SNARE_HOOK,
                TWOFOLD_ENERGY
            ]
        );
        assert_eq!(
            order(SortBy::Rarity),
            [
                PYREQUILL,
                SNARE_HOOK,
                TWOFOLD_ENERGY,
                CINDREL,
                KNOLLOP,
                FIRE_ENERGY
            ]
        );
    }

    #[test]
    fn fill_energy_splits_by_creature_colour() {
        let mut d = list(&[CINDREL; 4]);
        d.cards.extend([PUDDOCK, PUDDOCK, SCAMPIT, LUCKY_SATCHEL]);
        assert_eq!(d.fill_energy(), 52);
        assert_eq!((n(&d, FIRE_ENERGY), n(&d, WATER_ENERGY)), (35, 17));
        assert_eq!(d.fill_energy(), 0, "already full");
        // Energy already in the deck counts toward its colour's share.
        let mut d = list(&[CINDREL, PUDDOCK]);
        d.cards.extend([FIRE_ENERGY; 10]);
        d.fill_energy();
        assert_eq!((n(&d, FIRE_ENERGY), n(&d, WATER_ENERGY)), (29, 29));
        // Nothing to go on: no coloured creature.
        assert_eq!(list(&[SCAMPIT]).fill_energy(), 0);
    }

    fn n(d: &DeckList, id: CardId) -> usize {
        d.cards.iter().filter(|&&c| c == id).count()
    }

    /// The editor flow: creatures and trainers by hand, energy auto-filled.
    #[test]
    fn prebuilt_without_energy_refills_to_a_legal_deck() {
        let mut d = list(&STARTER.cards());
        d.cards.retain(|&id| !card(id).is_basic_energy());
        d.fill_energy();
        assert_eq!(d.validate(&starter_collection()), Ok(()));
    }

    #[test]
    fn deck_list_save_round_trips() {
        let d = DeckList {
            name: "Fire; and ice\nfast".into(),
            cards: EMBER_RUSH.cards(),
        };
        let s = d.save();
        assert!(s.starts_with("d1;0:4,"), "{s}");
        assert!(!s.contains('\n'));
        let (back, dropped) = DeckList::load(&s).unwrap();
        assert!(dropped.is_empty());
        assert_eq!(back.name, "Fire; and ice fast");
        assert_eq!(back.counts(), d.counts());
        let empty = DeckList::new("");
        assert_eq!(DeckList::load(&empty.save()).unwrap().0, empty);
    }

    #[test]
    fn deck_list_load_rejects_bad_strings() {
        let s = list(&EMBER_RUSH.cards()).save();
        assert_eq!(DeckList::load(""), Err(LoadError::Version));
        // A collection string is not a deck list.
        let c = starter_collection().save();
        assert_eq!(DeckList::load(&c), Err(LoadError::Version));
        assert_eq!(DeckList::load(&s[..s.len() - 2]), Err(LoadError::Checksum));
        assert_eq!(
            DeckList::load(&s.replacen("Test", "Tost", 1)),
            Err(LoadError::Checksum)
        );
        let signed = crate::collection::sign;
        assert_eq!(DeckList::load(&signed("d1;0:1")), Err(LoadError::Malformed));
        assert_eq!(
            DeckList::load(&signed("d1;0:0;X")),
            Err(LoadError::Malformed)
        );
        let (d, dropped) = DeckList::load(&signed("d1;0:2,9999:1;X")).unwrap();
        assert_eq!(
            (d.cards, dropped),
            (vec![CINDREL; 2], vec![(CardId(9999), 1)])
        );
    }
}
