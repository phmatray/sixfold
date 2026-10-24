//! Prebuilt decks made from [`crate::set`], one or two colours each, and the
//! starter deck a new player owns.

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

pub static GROVE_BLOOM: Prebuilt = Prebuilt {
    name: "Grove Bloom",
    description: "Grass and Lightning: poison and paralysis wear the opponent down while \
                  healing keeps your creatures standing. Win the long game.",
    list: &[
        (SPRIGGET, 4),
        (THORNBUCK, 3),
        (ELDERHART, 2),
        (MIREBUD, 3),
        (BOGBLOSSOM, 2),
        (BOGCROWN, 1),
        (BRIARHOG, 2),
        (BUZZLET, 3),
        (HIVEVOLT, 2),
        (LUCKY_SATCHEL, 3),
        (TRAIL_COMPASS, 2),
        (GROVE_BALM, 2),
        (VENOM_DART, 2),
        (CHARGE_COIL, 1),
        (HEARTY_BROTH, 1),
        (BLOOM_SEED, 1),
        (RELAY_BATON, 1),
        (GRASS_ENERGY, 17),
        (LIGHTNING_ENERGY, 8),
    ],
};

pub static STORM_FRONT: Prebuilt = Prebuilt {
    name: "Storm Front",
    description: "Lightning and Water: snipe the Bench, paralyze the Active and strip its \
                  energy, then bring down a coin-flip storm.",
    list: &[
        (SPARKIT, 4),
        (ARCFANG, 3),
        (FULGUREX, 2),
        (OHMLET, 3),
        (OHMRUNNER, 2),
        (STORMOX, 2),
        (RILLET, 3),
        (STREAMFIN, 2),
        (GULFWYRM, 1),
        (LUCKY_SATCHEL, 3),
        (TRAIL_COMPASS, 2),
        (JOLT_WIRE, 2),
        (CLEARWATER_VIAL, 2),
        (SNARE_HOOK, 1),
        (DRAIN_BRUSH, 1),
        (WIDE_SATCHEL, 1),
        (DOWSING_ROD, 1),
        (LIGHTNING_ENERGY, 15),
        (WATER_ENERGY, 10),
    ],
};

pub static STONE_FIST: Prebuilt = Prebuilt {
    name: "Stone Fist",
    description: "Fighting with Colorless support: sturdy creatures that hit harder the more \
                  they are hurt. Take the blows, then hit back.",
    list: &[
        (BRAWLET, 4),
        (IRONBRAWL, 3),
        (SHALEPUP, 3),
        (SHALEHOUND, 2),
        (CRAGMASTIFF, 2),
        (BOULDERGUT, 2),
        (BURROWBIT, 3),
        (WARRENBUCK, 2),
        (WARRENKING, 1),
        (LUCKY_SATCHEL, 3),
        (TRAIL_COMPASS, 2),
        (SPIKE_CHARM, 2),
        (BARK_SHIELD, 2),
        (FIELD_GUIDE, 1),
        (HEARTY_BROTH, 1),
        (STATIC_NET, 1),
        (FIGHTING_ENERGY, 22),
        (TWOFOLD_ENERGY, 4),
    ],
};

pub static DREAM_VEIL: Prebuilt = Prebuilt {
    name: "Dream Veil",
    description: "Psychic and Grass: sleep, confusion and jinxes keep the opponent from \
                  attacking, and hexes punish every point of damage.",
    list: &[
        (DOZEL, 3),
        (SOMNITAR, 2),
        (HEXLET, 4),
        (HEXMAGE, 3),
        (HEXARCH, 2),
        (GLOAMOWL, 2),
        (POLLENPUP, 3),
        (HONEYMANE, 2),
        (FERNWHISK, 1),
        (LUCKY_SATCHEL, 3),
        (TRAIL_COMPASS, 2),
        (SMOKE_PELLET, 2),
        (MOSS_POULTICE, 1),
        (GROVE_BALM, 1),
        (RECALL_HORN, 1),
        (FIELD_GUIDE, 1),
        (BLOOM_SEED, 1),
        (PSYCHIC_ENERGY, 16),
        (GRASS_ENERGY, 10),
    ],
};

/// The deck a new player owns: Fire and Water Commons and Uncommons, no
/// Rares. Not in [`PREBUILT`]: it is the player's own deck list.
pub static STARTER: Prebuilt = Prebuilt {
    name: "Trailhead",
    description: "Your first deck: Fire and Water creatures with simple attacks, \
                  plus trainers to draw, heal and switch.",
    list: &[
        (CINDREL, 3),
        (FLARROW, 2),
        (ASHPAW, 3),
        (BLAZELYNX, 2),
        (CINDERHORN, 1),
        (PUDDOCK, 3),
        (BROOKHORN, 2),
        (KELPUP, 2),
        (KELPWARDEN, 1),
        (SCAMPIT, 2),
        (BURROWBIT, 1),
        (LUCKY_SATCHEL, 2),
        (TRAIL_COMPASS, 2),
        (MOSS_POULTICE, 2),
        (RELAY_BATON, 2),
        (DOWSING_ROD, 2),
        (CLEARWATER_VIAL, 2),
        (FIRE_ENERGY, 13),
        (WATER_ENERGY, 13),
    ],
};

/// Every prebuilt deck, for the deck picker.
pub static PREBUILT: [&Prebuilt; 6] = [
    &EMBER_RUSH,
    &TIDE_MIND,
    &GROVE_BLOOM,
    &STORM_FRONT,
    &STONE_FIST,
    &DREAM_VEIL,
];

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

    /// R-DECK-01..03: every prebuilt deck and the starter are legal, and
    /// their names are unique.
    #[test]
    fn prebuilt_decks_validate() {
        let all: Vec<&Prebuilt> = PREBUILT.iter().copied().chain([&STARTER]).collect();
        for d in &all {
            assert_eq!(validate_deck(&d.cards()), Ok(()), "{}", d.name);
        }
        for (i, d) in all.iter().enumerate() {
            assert!(all[..i].iter().all(|e| e.name != d.name), "{}", d.name);
        }
        assert!(STARTER
            .cards()
            .iter()
            .all(|&id| crate::card::card(id).rarity < crate::card::Rarity::Rare));
    }

    /// Smoke run: seeded games of random legal play between every deck and
    /// the next one (the starter included, seats alternate) end without a
    /// refused action, and every listed action reads cleanly.
    #[test]
    fn prebuilt_decks_random_play_finishes() {
        let all: Vec<Vec<CardId>> = PREBUILT
            .iter()
            .chain([&&STARTER])
            .map(|d| d.cards())
            .collect();
        for seed in 0..140u64 {
            let a = seed as usize % all.len();
            let mut decks = [all[a].clone(), all[(a + 1) % all.len()].clone()];
            if seed / all.len() as u64 % 2 == 1 {
                decks.swap(0, 1);
            }
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
        }
    }
}
