//! A player's progress outside a duel: the packs they hold ([`Wallet`]), and
//! the rules that move cards and packs around ([`Profile`]): what a new player
//! starts with and opening a pack. (Duel rewards are Facets, kept in the
//! world's [`Progress`](crate::overworld::Progress): see
//! [`crate::campaign::record_quick_duel`].)
//!
//! Pure data: the UI stores [`Collection::save`], [`DeckList::save`] and
//! [`Wallet::save`] strings wherever it likes.

use crate::booster::BOOSTERS;
use crate::card::CardId;
use crate::collection::{sign, starter_collection, unsign, Collection, LoadError};
use crate::deck::DeckList;
use crate::decks::STARTER;

/// The wallet save string version this build writes and reads.
const VERSION: &str = "p1";

/// Packs a new player starts with, per booster: one Ember, Tide and Wild.
pub const STARTING_PACKS: [u32; BOOSTERS.len()] = [1, 1, 1, 0, 0, 0, 0];

/// Unopened packs, counted per booster in [`BOOSTERS`] order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Wallet {
    packs: [u32; BOOSTERS.len()],
}

impl Wallet {
    /// Packs held of booster `b` (0 for an unknown index).
    pub fn packs(&self, b: usize) -> u32 {
        self.packs.get(b).copied().unwrap_or(0)
    }

    pub fn total(&self) -> u32 {
        self.packs.iter().sum()
    }

    /// Adds `n` packs of booster `b`; false for an unknown booster.
    pub fn grant(&mut self, b: usize, n: u32) -> bool {
        match self.packs.get_mut(b) {
            Some(p) => {
                *p = p.saturating_add(n);
                true
            }
            None => false,
        }
    }

    /// Spends one pack of booster `b`; false (nothing spent) if none is held.
    pub fn take(&mut self, b: usize) -> bool {
        match self.packs.get_mut(b) {
            Some(p) if *p > 0 => {
                *p -= 1;
                true
            }
            _ => false,
        }
    }

    /// `p1;<count>,...;<checksum>`, one count per booster.
    pub fn save(&self) -> String {
        let body: Vec<String> = self.packs.iter().map(u32::to_string).collect();
        sign(&format!("{VERSION};{}", body.join(",")))
    }

    /// Reads a [`save`](Self::save) string. Missing trailing counts (a save
    /// from a build with fewer boosters) are zero.
    pub fn load(s: &str) -> Result<Wallet, LoadError> {
        let body = unsign(s, VERSION)?;
        let mut w = Wallet::default();
        let counts: Vec<&str> = body.split(',').collect();
        if counts.len() > w.packs.len() {
            return Err(LoadError::Malformed);
        }
        for (slot, n) in w.packs.iter_mut().zip(counts) {
            *slot = n.parse().map_err(|_| LoadError::Malformed)?;
        }
        Ok(w)
    }
}

/// Everything a player keeps between duels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub collection: Collection,
    pub decks: Vec<DeckList>,
    pub wallet: Wallet,
}

impl Profile {
    /// A new player: the starter collection, the [`STARTER`] deck as their
    /// own deck list, and [`STARTING_PACKS`]. (The prebuilt decks stay
    /// playable against the AI without being owned.)
    pub fn new_player() -> Profile {
        Profile {
            collection: starter_collection(),
            decks: vec![DeckList {
                name: STARTER.name.to_string(),
                cards: STARTER.cards(),
            }],
            wallet: Wallet {
                packs: STARTING_PACKS,
            },
        }
    }

    /// Opens one pack of booster `b`: spends it, adds its cards to the
    /// collection and returns them in reveal order, each flagged when it is
    /// the player's first copy. None, and nothing changes, without a pack.
    pub fn open_pack(&mut self, b: usize, seed: u64) -> Option<Vec<(CardId, bool)>> {
        if !self.wallet.take(b) {
            return None;
        }
        let cards = BOOSTERS[b].open(seed, &self.collection);
        Some(
            cards
                .into_iter()
                .map(|id| {
                    let new = self.collection.count(id) == 0;
                    self.collection.add(id, 1);
                    (id, new)
                })
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::booster::PACK_SIZE;
    use crate::collection::Collection;

    #[test]
    fn new_player_starts_with_cards_decks_and_three_packs() {
        let p = Profile::new_player();
        assert_eq!(p.collection, starter_collection());
        assert_eq!(p.wallet.total(), 3);
        assert_eq!(BOOSTERS.len(), STARTING_PACKS.len());
        assert_eq!(
            p.decks.iter().map(|d| d.name.as_str()).collect::<Vec<_>>(),
            ["Trailhead"]
        );
        assert!(p.decks.iter().all(|d| d.validate(&p.collection).is_ok()));
    }

    #[test]
    fn opening_a_pack_spends_it_and_fills_the_collection() {
        let mut p = Profile::new_player();
        p.collection = Collection::new();
        let opened = p.open_pack(2, 42).unwrap();
        assert_eq!(opened.len(), PACK_SIZE);
        assert_eq!(p.wallet.packs(2), 0);
        assert_eq!(p.open_pack(2, 42), None, "no Wild pack left");
        // Same seed, same cards; the first copy of each non-energy card is new.
        let ids: Vec<CardId> = opened.iter().map(|&(id, _)| id).collect();
        assert_eq!(ids, BOOSTERS[2].open(42, &Collection::new()));
        let mut seen = vec![];
        for &(id, new) in &opened {
            let basic_energy = crate::card::card(id).is_basic_energy();
            assert_eq!(new, !basic_energy && !seen.contains(&id), "{id:?}");
            seen.push(id);
        }
        for &id in &ids {
            assert!(p.collection.count(id) >= 1);
        }
        assert_eq!(p.open_pack(9, 1), None);
    }

    #[test]
    fn wallet_save_round_trips_and_rejects_bad_strings() {
        let mut w = Wallet::default();
        w.grant(0, 3);
        w.grant(2, 12);
        assert!(!w.grant(BOOSTERS.len(), 1));
        // A 0.3 save holds three counts.
        assert_eq!(Wallet::load(&sign("p1;1,2,3")).unwrap().packs(2), 3);
        let s = w.save();
        assert!(s.starts_with("p1;3,0,12,0,0,0,0;"), "{s}");
        assert_eq!(Wallet::load(&s), Ok(w));
        assert_eq!(Wallet::load(&sign("p1;5")).unwrap().packs(0), 5);
        assert_eq!(Wallet::load(&sign("v1;1,1,1")), Err(LoadError::Version));
        assert_eq!(Wallet::load(&s[..s.len() - 1]), Err(LoadError::Checksum));
        for bad in ["p1;1,1,1,1,1,1,1,1", "p1;x", "p1;-1", "p1;"] {
            assert_eq!(Wallet::load(&sign(bad)), Err(LoadError::Malformed), "{bad}");
        }
    }
}
