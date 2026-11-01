//! Player-built deck lists for the deck editor: a deck checked against the
//! deck rules (R-DECK-*) and against what the player's collection owns.

use std::collections::BTreeMap;

use sixfold_engine::{
    card, deck_errors, try_card, validate_deck, CardId, CardKind, Color, DeckError, EnergyCard,
    SortBy, DECK_SIZE, MAX_COPIES,
};

use crate::collection::{counts_body, parse_counts, sign, unsign, Collection, LoadError};

/// The deck-list save string version this build writes and reads.
const DECK_VERSION: &str = "d1";

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

    /// Every reason the deck can't be played: the rules it breaks
    /// ([`deck_errors`]), then each card it uses more copies of than owned.
    pub fn problems(&self, owned: &Collection) -> Vec<DeckListError> {
        let mut p: Vec<_> = deck_errors(&self.cards)
            .into_iter()
            .map(DeckListError::Illegal)
            .collect();
        p.extend(
            owned
                .missing(&self.cards)
                .into_iter()
                .filter(|&(id, _)| try_card(id).is_some())
                .map(|(id, _)| DeckListError::NotOwned(id)),
        );
        p
    }

    /// Fixes one problem where there is an obvious fix: too few cards fills
    /// with basic energy ([`fill_energy`](Self::fill_energy)), too many takes
    /// basic energy out (the most-used colour first), the extras over the
    /// 4-copy limit or over what is owned go, unknown cards go. No Basic has
    /// no fix here (the player picks one). True when the deck changed.
    pub fn repair(&mut self, e: DeckListError, owned: &Collection) -> bool {
        let before = self.cards.clone();
        match e {
            DeckListError::Illegal(DeckError::WrongSize(n)) if n < DECK_SIZE => {
                self.fill_energy();
            }
            DeckListError::Illegal(DeckError::WrongSize(n)) => {
                let is_energy = |id: CardId| try_card(id).is_some_and(|d| d.is_basic_energy());
                // All the way down to 60, or not at all: half a fix isn't one.
                if self.cards.iter().filter(|&&id| is_energy(id)).count() >= n - DECK_SIZE {
                    while self.cards.len() > DECK_SIZE {
                        let most = self
                            .counts()
                            .into_iter()
                            .filter(|&(id, _)| is_energy(id))
                            .max_by_key(|&(_, n)| n);
                        match most {
                            Some((id, _)) => self.remove(id),
                            None => break,
                        };
                    }
                }
            }
            DeckListError::Illegal(DeckError::UnknownCard(id)) => self.cards.retain(|&c| c != id),
            DeckListError::Illegal(DeckError::NoBasic) => {}
            DeckListError::Illegal(DeckError::TooManyCopies(name)) => {
                let mut keep = MAX_COPIES;
                self.cards.retain(|&c| {
                    if try_card(c).is_none_or(|d| d.name != name) {
                        return true;
                    }
                    let kept = keep > 0;
                    keep = keep.saturating_sub(1);
                    kept
                });
            }
            DeckListError::NotOwned(id) => {
                while self.cards.iter().filter(|&&c| c == id).count() > owned.count(id) {
                    self.remove(id);
                }
            }
        }
        self.cards != before
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
        for d in self.cards.iter().filter_map(|&id| try_card(id)) {
            match &d.kind {
                CardKind::Creature(cr) | CardKind::GearBasic(cr)
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
            self.cards.push(
                sixfold_engine::set::basic_energy(Color::ALL[c])
                    .expect("set has every basic energy"),
            );
        }
        self.cards.len() - start
    }
}

#[cfg(test)]
mod deck_list_tests {
    use super::*;
    use crate::collection::starter_collection;
    use sixfold_engine::decks::{EMBER_RUSH, STARTER};
    use sixfold_engine::set::*;

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

    /// The editor flow: creatures and Gear cards by hand, energy auto-filled.
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

#[cfg(test)]
mod repair_tests {
    use super::*;
    use crate::collection::starter_collection;
    use sixfold_engine::fixtures::*;

    fn list(cards: &[CardId]) -> DeckList {
        DeckList {
            name: "Test".into(),
            cards: cards.to_vec(),
        }
    }

    #[test]
    fn problems_add_what_the_collection_lacks() {
        let mut owned = Collection::new();
        owned.add(MOSSLING, 2);
        let mut d = vec![MOSSLING; 3];
        d.resize(DECK_SIZE, GRASS_E);
        assert_eq!(
            list(&d).problems(&owned),
            vec![DeckListError::NotOwned(MOSSLING)]
        );
        d.push(FIELD_NOTES);
        assert_eq!(
            list(&d).problems(&owned),
            vec![
                DeckListError::Illegal(DeckError::WrongSize(61)),
                DeckListError::NotOwned(MOSSLING),
                DeckListError::NotOwned(FIELD_NOTES),
            ]
        );
        assert_eq!(list(&d).validate(&owned), Err(d_err(61)));
        let starter = list(&sixfold_engine::decks::STARTER.cards());
        assert_eq!(starter.problems(&starter_collection()), vec![]);
    }

    fn d_err(n: usize) -> DeckListError {
        DeckListError::Illegal(DeckError::WrongSize(n))
    }

    #[test]
    fn repair_fixes_what_it_can() {
        let mut owned = Collection::new();
        owned.add(MOSSLING, 9);
        owned.add(FIELD_NOTES, 9);
        // Too few: energy fills it to 60.
        let mut d = list(&[MOSSLING; 4]);
        assert!(d.repair(d_err(4), &owned));
        assert_eq!(d.cards.len(), DECK_SIZE);
        // Too many: basic energy comes out, nothing else.
        d.cards.extend([MOSSLING, GRASS_E, GRASS_E]);
        assert!(d.repair(d_err(63), &owned));
        assert_eq!((d.cards.len(), d.copies("Mossling")), (DECK_SIZE, 5));
        // Over the 4-copy limit: the extras go.
        assert!(d.repair(
            DeckListError::Illegal(DeckError::TooManyCopies("Mossling")),
            &owned
        ));
        assert_eq!(d.copies("Mossling"), 4);
        // More than owned: down to what the collection holds.
        let mut few = Collection::new();
        few.add(MOSSLING, 1);
        assert!(d.repair(DeckListError::NotOwned(MOSSLING), &few));
        assert_eq!(d.copies("Mossling"), 1);
        // No Basic: nothing to do here (the editor shows the Basics instead),
        // and too many with no basic energy to take out.
        let mut none = list(&[FIELD_NOTES; 61]);
        assert!(!none.repair(DeckListError::Illegal(DeckError::NoBasic), &owned));
        assert!(!none.repair(d_err(61), &owned));
        assert_eq!(none.cards.len(), 61);
        // Not enough basic energy to reach 60: nothing comes out.
        let mut short = list(&[FIELD_NOTES; 61]);
        short.cards[0] = GRASS_E;
        short.cards.push(FIELD_NOTES);
        assert!(!short.repair(d_err(62), &owned));
        assert_eq!(short.cards.len(), 62);
        let mut unknown = list(&[MOSSLING, CardId(9999)]);
        assert!(unknown.repair(
            DeckListError::Illegal(DeckError::UnknownCard(CardId(9999))),
            &owned
        ));
        assert_eq!(unknown.cards, vec![MOSSLING]);
        // A wrong size next to an unknown card: no panic, the known cards count.
        let mut odd = list(&[MOSSLING, CardId(9999)]);
        assert!(odd.repair(d_err(2), &owned));
        assert_eq!(odd.cards.len(), DECK_SIZE);
    }

    #[test]
    fn repair_trims_the_most_used_energy_first() {
        let mut d = vec![MOSSLING; 4];
        d.extend([FIRE_E; 2]);
        d.resize(DECK_SIZE + 1, GRASS_E);
        let mut d = list(&d);
        assert!(d.repair(d_err(DECK_SIZE + 1), &Collection::new()));
        assert_eq!(d.cards.iter().filter(|&&c| c == FIRE_E).count(), 2);
    }

    #[test]
    fn unknown_cards_are_never_reported_as_not_owned() {
        let unknown = DeckError::UnknownCard(CardId(9999));
        let p = list(&[MOSSLING, CardId(9999)]).problems(&Collection::new());
        assert!(p.contains(&DeckListError::Illegal(unknown)));
        assert!(!p.contains(&DeckListError::NotOwned(CardId(9999))));
    }
}
