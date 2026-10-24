//! A player's card collection: how many copies of each card they own.
//!
//! Basic energy is unlimited and never stored. [`Collection::save`] gives a
//! compact, versioned string for browser storage; [`Collection::load`] reads
//! it back and rejects anything malformed or corrupted.

use std::collections::BTreeMap;

use crate::card::{try_card, CardId};
use crate::decks::PREBUILT;

/// The saved-string version this build writes and reads.
const VERSION: &str = "v1";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Collection {
    owned: BTreeMap<CardId, usize>,
}

/// Why [`Collection::load`] refused a saved string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadError {
    /// Not a collection string, or one from an unknown version.
    Version,
    /// The checksum doesn't match: the string was cut short or altered.
    Checksum,
    /// An entry isn't `id:count` with a positive count, or repeats an id.
    Malformed,
}

/// Basic energy (unlimited) and unknown ids are handled specially.
fn is_unlimited(id: CardId) -> bool {
    try_card(id).is_some_and(|c| c.is_basic_energy())
}

impl Collection {
    pub fn new() -> Self {
        Self::default()
    }

    /// Copies owned. Basic energy reports `usize::MAX` (unlimited).
    pub fn count(&self, id: CardId) -> usize {
        if is_unlimited(id) {
            return usize::MAX;
        }
        self.owned.get(&id).copied().unwrap_or(0)
    }

    /// Adds `n` copies. Basic energy is a no-op (always owned).
    pub fn add(&mut self, id: CardId, n: usize) {
        if n > 0 && !is_unlimited(id) {
            *self.owned.entry(id).or_default() += n;
        }
    }

    /// Adds one copy of each card, e.g. a booster's contents.
    pub fn add_all(&mut self, cards: &[CardId]) {
        cards.iter().for_each(|&id| self.add(id, 1));
    }

    /// Removes `n` copies; false (and nothing removed) when fewer are owned.
    pub fn remove(&mut self, id: CardId, n: usize) -> bool {
        if is_unlimited(id) {
            return true;
        }
        let have = self.count(id);
        if have < n {
            return false;
        }
        if have == n {
            self.owned.remove(&id);
        } else {
            self.owned.insert(id, have - n);
        }
        true
    }

    /// Owned cards with their counts, in id order (basic energy not listed).
    pub fn iter(&self) -> impl Iterator<Item = (CardId, usize)> + '_ {
        self.owned.iter().map(|(&id, &n)| (id, n))
    }

    /// Cards the deck uses more copies of than are owned, with how many are
    /// missing, in id order.
    pub fn missing(&self, deck: &[CardId]) -> Vec<(CardId, usize)> {
        let mut need: BTreeMap<CardId, usize> = BTreeMap::new();
        for &id in deck {
            *need.entry(id).or_default() += 1;
        }
        need.into_iter()
            .filter_map(|(id, n)| {
                n.checked_sub(self.count(id))
                    .filter(|&m| m > 0)
                    .map(|m| (id, m))
            })
            .collect()
    }

    /// Owns enough copies of every card in the deck.
    pub fn can_build(&self, deck: &[CardId]) -> bool {
        self.missing(deck).is_empty()
    }

    /// `v1;<id>:<count>,...;<checksum>`: decimal ids in ascending order,
    /// checksum the FNV-1a hash of everything before the last `;`, as 8 hex
    /// digits. The same collection always gives the same string.
    pub fn save(&self) -> String {
        let body: Vec<String> = self.iter().map(|(id, n)| format!("{}:{n}", id.0)).collect();
        let head = format!("{VERSION};{}", body.join(","));
        format!("{head};{:08x}", fnv1a(&head))
    }

    /// Reads a [`save`](Self::save) string. Ids that aren't in the card set
    /// (e.g. from a newer build) are dropped and returned with their counts;
    /// basic energy entries are ignored.
    pub fn load(s: &str) -> Result<(Collection, Vec<(CardId, usize)>), LoadError> {
        let (head, sum) = s.rsplit_once(';').ok_or(LoadError::Version)?;
        let (version, body) = head.split_once(';').ok_or(LoadError::Version)?;
        if version != VERSION {
            return Err(LoadError::Version);
        }
        if sum.len() != 8 || u32::from_str_radix(sum, 16).ok() != Some(fnv1a(head)) {
            return Err(LoadError::Checksum);
        }
        let mut c = Collection::new();
        let mut seen = std::collections::BTreeSet::new();
        let mut dropped = vec![];
        let entries = if body.is_empty() {
            vec![]
        } else {
            body.split(',').collect()
        };
        for entry in entries {
            let (id, n) = entry.split_once(':').ok_or(LoadError::Malformed)?;
            let id = CardId(id.parse().map_err(|_| LoadError::Malformed)?);
            let n: usize = n.parse().map_err(|_| LoadError::Malformed)?;
            if n == 0 || !seen.insert(id) {
                return Err(LoadError::Malformed);
            }
            match try_card(id) {
                Some(_) => c.add(id, n),
                None => dropped.push((id, n)),
            }
        }
        Ok((c, dropped))
    }
}

/// 32-bit FNV-1a: enough to catch a truncated or edited save string.
fn fnv1a(s: &str) -> u32 {
    s.bytes()
        .fold(0x811c_9dc5, |h, b| (h ^ b as u32).wrapping_mul(0x0100_0193))
}

/// What a new player starts with: the cards of every prebuilt deck, so each
/// one can be built as it ships (basic energy is unlimited anyway).
pub fn starter_collection() -> Collection {
    let mut c = Collection::new();
    for d in PREBUILT {
        c.add_all(&d.cards());
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decks::{ember_rush, tide_mind};
    use crate::set::*;

    #[test]
    fn add_remove_count() {
        let mut c = Collection::new();
        assert_eq!(c.count(CINDREL), 0);
        c.add(CINDREL, 2);
        c.add_all(&[CINDREL, FLARROW]);
        assert_eq!((c.count(CINDREL), c.count(FLARROW)), (3, 1));
        assert!(!c.remove(CINDREL, 4));
        assert_eq!(c.count(CINDREL), 3);
        assert!(c.remove(FLARROW, 1));
        assert_eq!(c.iter().collect::<Vec<_>>(), [(CINDREL, 3)]);
    }

    #[test]
    fn basic_energy_is_unlimited_and_not_stored() {
        let mut c = Collection::new();
        assert_eq!(c.count(FIRE_ENERGY), usize::MAX);
        c.add(FIRE_ENERGY, 5);
        assert!(c.remove(FIRE_ENERGY, 100));
        assert_eq!(c.iter().count(), 0);
        // The special energy is a normal card.
        assert_eq!(c.count(TWOFOLD_ENERGY), 0);
    }

    #[test]
    fn can_build_needs_enough_copies() {
        let deck = ember_rush();
        let mut c = starter_collection();
        assert!(c.can_build(&deck) && c.can_build(&tide_mind()));
        assert!(c.remove(PYREQUILL, 1));
        assert_eq!(c.missing(&deck), [(PYREQUILL, 1)]);
        assert!(!c.can_build(&deck));
        assert_eq!(Collection::new().missing(&deck).len(), 16); // all but the 2 basic energies
    }

    #[test]
    fn starter_holds_both_prebuilts() {
        let c = starter_collection();
        assert_eq!(c.count(SCAMPIT), 4); // 2 in each deck
        assert_eq!(c.count(LUCKY_SATCHEL), 7);
        assert_eq!(c.count(CINDREL), 4);
    }

    #[test]
    fn save_round_trips_and_is_stable() {
        let c = starter_collection();
        let s = c.save();
        assert!(s.starts_with("v1;0:4,1:3,"), "{s}");
        assert_eq!(Collection::load(&s), Ok((c.clone(), vec![])));
        let mut again = Collection::new();
        for (id, n) in c.iter().collect::<Vec<_>>().into_iter().rev() {
            again.add(id, n);
        }
        assert_eq!(again.save(), s);
        let empty = Collection::new().save();
        assert_eq!(Collection::load(&empty), Ok((Collection::new(), vec![])));
    }

    /// Build a string with a valid checksum around any body.
    fn signed(head: &str) -> String {
        format!("{head};{:08x}", fnv1a(head))
    }

    #[test]
    fn load_rejects_corrupted_input() {
        let s = starter_collection().save();
        assert_eq!(Collection::load(""), Err(LoadError::Version));
        assert_eq!(Collection::load("hello"), Err(LoadError::Version));
        assert_eq!(Collection::load(&signed("v9;0:1")), Err(LoadError::Version));
        assert_eq!(
            Collection::load(&s[..s.len() - 1]),
            Err(LoadError::Checksum)
        );
        assert_eq!(
            Collection::load(&s.replacen("0:4", "0:9", 1)),
            Err(LoadError::Checksum)
        );
        for bad in [
            "v1;0",
            "v1;0:",
            "v1;x:1",
            "v1;0:-1",
            "v1;0:0",
            "v1;0:1,0:2",
            "v1;0:1,",
        ] {
            assert_eq!(
                Collection::load(&signed(bad)),
                Err(LoadError::Malformed),
                "{bad}"
            );
        }
    }

    #[test]
    fn load_drops_unknown_ids_with_a_report() {
        let (c, dropped) = Collection::load(&signed("v1;1:2,9999:3,28:7")).unwrap();
        assert_eq!(dropped, [(CardId(9999), 3)]);
        assert_eq!(c.count(FLARROW), 2);
        assert_eq!(c.iter().count(), 1); // Fire Energy (28) ignored
    }
}
