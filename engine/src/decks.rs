//! Prebuilt decks made from [`crate::set`]: Ember Rush and Tide Mind.

use crate::card::CardId;
use crate::set::*;

/// A prebuilt deck: a name and blurb for the menu, and its list as
/// `(card, copies)` in display order.
#[derive(Debug)]
pub struct Prebuilt {
    pub name: &'static str,
    pub description: &'static str,
    pub list: &'static [(CardId, usize)],
}

impl Prebuilt {
    /// The 60 cards, ready for `Game::new`.
    pub fn cards(&self) -> Vec<CardId> {
        self.list
            .iter()
            .flat_map(|&(id, n)| std::iter::repeat_n(id, n))
            .collect()
    }
}

pub static EMBER_RUSH: Prebuilt = Prebuilt {
    name: "Ember Rush",
    description: "Fire and Fighting aggro: fast Basics, coin-flip hits and a Stage 2 \
                  that powers up with free Fire energy. Hit hard before the opponent sets up.",
    list: &[
        (CINDREL, 4),
        (FLARROW, 3),
        (PYREQUILL, 2),
        (KNOLLOP, 3),
        (KNOLLGRIM, 2),
        (TUSKLET, 4),
        (GRITFANG, 2),
        (SCAMPIT, 2),
        (LUCKY_SATCHEL, 4),
        (TRAIL_COMPASS, 2),
        (SPIKE_CHARM, 2),
        (SNARE_HOOK, 2),
        (CLEARWATER_VIAL, 2),
        (RELAY_BATON, 1),
        (BLOOM_SEED, 1),
        (FIRE_ENERGY, 11),
        (FIGHTING_ENERGY, 10),
        (TWOFOLD_ENERGY, 3),
    ],
};

pub static TIDE_MIND: Prebuilt = Prebuilt {
    name: "Tide Mind",
    description: "Water and Psychic control: sleep, poison and paralysis, energy removal \
                  and healing, then a sturdy Stage 2 to finish. Outlast the opponent.",
    list: &[
        (PUDDOCK, 4),
        (BROOKHORN, 3),
        (TIDEMAW, 2),
        (SHELLMOTE, 2),
        (WISPUN, 4),
        (WISPELLE, 3),
        (GLIMMOX, 2),
        (SCAMPIT, 2),
        (LUCKY_SATCHEL, 3),
        (TRAIL_COMPASS, 2),
        (MOSS_POULTICE, 2),
        (DRAIN_BRUSH, 2),
        (BARK_SHIELD, 2),
        (RECALL_HORN, 1),
        (UNWIND_SPRAY, 1),
        (BLOOM_SEED, 1),
        (WATER_ENERGY, 12),
        (PSYCHIC_ENERGY, 12),
    ],
};

/// Every prebuilt deck, for the deck picker.
pub static PREBUILT: [&Prebuilt; 2] = [&EMBER_RUSH, &TIDE_MIND];

pub fn ember_rush() -> Vec<CardId> {
    EMBER_RUSH.cards()
}

pub fn tide_mind() -> Vec<CardId> {
    TIDE_MIND.cards()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Rng;
    use crate::{validate_deck, Config, Event, Game};

    /// R-DECK-01..03: both prebuilt decks are legal.
    #[test]
    fn prebuilt_decks_validate() {
        for d in PREBUILT {
            assert_eq!(validate_deck(&d.cards()), Ok(()), "{}", d.name);
        }
    }

    /// Smoke run: 100 seeded games of random legal play between the two decks
    /// (seats alternate) end without a refused action.
    #[test]
    fn prebuilt_decks_random_play_finishes() {
        let mut wins = [0; 2];
        for seed in 0..100u64 {
            let swap = seed % 2 == 1;
            let decks = if swap {
                [tide_mind(), ember_rush()]
            } else {
                [ember_rush(), tide_mind()]
            };
            let mut g = Game::new(decks, seed, Config::default()).unwrap();
            let mut pick = Rng::new(seed ^ 0xdec4);
            let mut actions = 0;
            while g.winner().is_none() {
                actions += 1;
                assert!(actions < 100_000, "seed {seed}: action cap hit");
                let legal = g.legal_actions();
                assert!(!legal.is_empty(), "seed {seed}: no action");
                let a = legal[pick.below(legal.len() as u32) as usize];
                let v = g.view(g.view(0).waiting_on.unwrap());
                assert!(!v.describe_action(a).contains('?'), "seed {seed}: {a:?}");
                let ev = g
                    .apply(a)
                    .unwrap_or_else(|e| panic!("seed {seed}: listed {a:?} refused: {e:?}"));
                ev.iter().for_each(|e| {
                    e.describe(0);
                });
            }
            assert!(matches!(g.log().last(), Some(Event::GameOver { .. })));
            wins[(g.winner().unwrap() == 1) as usize ^ swap as usize] += 1;
        }
        // [Ember Rush, Tide Mind] wins; random play only, so a loose sanity bound.
        println!("random-play wins (Ember Rush, Tide Mind): {wins:?}");
        assert!(wins[0] >= 20 && wins[1] >= 20, "lopsided: {wins:?}");
    }
}
