//! Boosters: 10-card packs drawn from [`crate::set`] by rarity, deterministic
//! from a seed.
//!
//! A pack is 1 basic energy, 5 Commons, 3 Uncommons and 1 Rare, in that
//! (reveal) order. Commons may repeat; the Uncommons are distinct. Themed packs
//! only hold creatures and basic energy of their colours; trainers, the special
//! energy and colorless creatures turn up in every pack. Promo cards
//! ([`crate::set::PROMOS`]) are campaign rewards and turn up in none.

use crate::card::{card, CardId, CardKind, Color, EnergyCard, Rarity};
use crate::lang::T;
use crate::rng::Rng;

pub const PACK_SIZE: usize = 10;

/// Slots per pack in reveal order: rarity, count, whether the cards are distinct.
const SLOTS: [(Rarity, usize, bool); 4] = [
    (Rarity::Energy, 1, false),
    (Rarity::Common, 5, false),
    (Rarity::Uncommon, 3, true),
    (Rarity::Rare, 1, true),
];

#[derive(Debug)]
pub struct Booster {
    pub name: T,
    pub description: T,
    /// Creature and basic energy colours this pack holds.
    pub colors: &'static [Color],
}

pub static EMBER_PACK: Booster = Booster {
    name: T::new("Ember Pack", "Booster Braise"),
    description: T::new(
        "Fire and Fighting Hexons, plus trainers.",
        "Hexons Feu et Combat, plus des cartes Dresseur.",
    ),
    colors: &[Color::Fire, Color::Fighting],
};

pub static TIDE_PACK: Booster = Booster {
    name: T::new("Tide Pack", "Booster Marée"),
    description: T::new(
        "Water and Psychic Hexons, plus trainers.",
        "Hexons Eau et Psychique, plus des cartes Dresseur.",
    ),
    colors: &[Color::Water, Color::Psychic],
};

pub static WILD_PACK: Booster = Booster {
    name: T::new("Wild Pack", "Booster Sauvage"),
    description: T::new(
        "Any card in the set.",
        "N'importe quelle carte de la série.",
    ),
    colors: &Color::ALL,
};

pub static GROVE_PACK: Booster = Booster {
    name: T::new("Grove Pack", "Booster Bosquet"),
    description: T::new(
        "Grass and Lightning Hexons, plus trainers.",
        "Hexons Herbe et Foudre, plus des cartes Dresseur.",
    ),
    colors: &[Color::Grass, Color::Lightning],
};

pub static STORM_PACK: Booster = Booster {
    name: T::new("Storm Pack", "Booster Orage"),
    description: T::new(
        "Lightning and Water Hexons, plus trainers.",
        "Hexons Foudre et Eau, plus des cartes Dresseur.",
    ),
    colors: &[Color::Lightning, Color::Water],
};

pub static STONE_PACK: Booster = Booster {
    name: T::new("Stone Pack", "Booster Pierre"),
    description: T::new(
        "Fighting and Colorless Hexons, plus trainers.",
        "Hexons Combat et Incolore, plus des cartes Dresseur.",
    ),
    colors: &[Color::Fighting],
};

pub static DREAM_PACK: Booster = Booster {
    name: T::new("Dream Pack", "Booster Songe"),
    description: T::new(
        "Psychic and Grass Hexons, plus trainers.",
        "Hexons Psychique et Herbe, plus des cartes Dresseur.",
    ),
    colors: &[Color::Psychic, Color::Grass],
};

/// The Wild pack's index in [`BOOSTERS`]: any card can turn up.
pub const WILD: usize = 2;

/// Every booster, for the shop. Wallets count packs by index here, so new
/// boosters go at the end.
pub static BOOSTERS: [&Booster; 7] = [
    &EMBER_PACK,
    &TIDE_PACK,
    &WILD_PACK,
    &GROVE_PACK,
    &STORM_PACK,
    &STONE_PACK,
    &DREAM_PACK,
];

impl Booster {
    /// Whether this card can turn up in the pack. Promos never do.
    pub fn holds(&self, id: CardId) -> bool {
        if crate::set::is_promo(id) {
            return false;
        }
        let c = card(id);
        let color = match &c.kind {
            CardKind::Creature(cr) | CardKind::TrainerBasic(cr) => cr.color,
            CardKind::Energy(EnergyCard::Basic(col)) => *col,
            _ => Color::Colorless,
        };
        color == Color::Colorless || self.colors.contains(&color)
    }

    /// The cards of one rarity this pack draws from, in id order.
    pub fn pool(&self, rarity: Rarity) -> Vec<CardId> {
        crate::set::ids()
            .filter(|&id| card(id).rarity == rarity && self.holds(id))
            .collect()
    }

    /// Opens a pack: the same seed always gives the same [`PACK_SIZE`] cards.
    pub fn open(&self, seed: u64) -> Vec<CardId> {
        let mut rng = Rng::new(seed);
        let mut pack = Vec::with_capacity(PACK_SIZE);
        for (rarity, n, distinct) in SLOTS {
            let mut pool = self.pool(rarity);
            if distinct {
                rng.shuffle(&mut pool);
                pack.extend(pool.into_iter().take(n));
            } else {
                pack.extend((0..n).map(|_| pool[rng.below(pool.len() as u32) as usize]));
            }
        }
        pack
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::set::*;
    use std::collections::HashMap;

    /// Every pool can fill its slots, so every pack has exactly 10 cards.
    #[test]
    fn pools_fill_every_slot() {
        for b in BOOSTERS {
            for (r, n, distinct) in SLOTS {
                let p = b.pool(r).len();
                assert!(p >= if distinct { n } else { 1 }, "{} {r:?}: {p}", b.name);
            }
        }
    }

    #[test]
    fn pack_composition() {
        for b in BOOSTERS {
            for seed in 0..200 {
                let pack = b.open(seed);
                assert_eq!(pack.len(), PACK_SIZE);
                let rarities: Vec<Rarity> = pack.iter().map(|&id| card(id).rarity).collect();
                let want: Vec<Rarity> = SLOTS
                    .iter()
                    .flat_map(|&(r, n, _)| std::iter::repeat_n(r, n))
                    .collect();
                assert_eq!(rarities, want, "{} seed {seed}", b.name);
                let mut seen = vec![];
                for &id in pack
                    .iter()
                    .filter(|&&id| card(id).rarity >= Rarity::Uncommon)
                {
                    assert!(
                        !seen.contains(&id),
                        "{} seed {seed}: repeated {id:?}",
                        b.name
                    );
                    seen.push(id);
                }
                assert!(pack.iter().all(|&id| b.holds(id)));
            }
        }
    }

    #[test]
    fn themed_packs_stay_in_their_colours() {
        let off = [
            PUDDOCK,
            TIDEMAW,
            WISPUN,
            GLIMMOX,
            WATER_ENERGY,
            PSYCHIC_ENERGY,
        ];
        assert!(off.iter().all(|&id| !EMBER_PACK.holds(id)));
        assert!([CINDREL, SCAMPIT, SNARE_HOOK, TWOFOLD_ENERGY, FIRE_ENERGY]
            .iter()
            .all(|&id| EMBER_PACK.holds(id)));
        assert!(!TIDE_PACK.holds(PYREQUILL) && TIDE_PACK.holds(TIDEMAW));
        assert!(GROVE_PACK.holds(ELDERHART) && GROVE_PACK.holds(FULGUREX));
        assert!(!GROVE_PACK.holds(GULFWYRM) && STORM_PACK.holds(GULFWYRM));
        assert!(STONE_PACK.holds(WARRENKING) && !STONE_PACK.holds(PYREQUILL));
        assert!(DREAM_PACK.holds(HEXARCH) && !DREAM_PACK.holds(FULGUREX));
        assert!(crate::set::ids().all(|id| WILD_PACK.holds(id) != crate::set::is_promo(id)));
    }

    #[test]
    fn same_seed_same_pack() {
        assert_eq!(WILD_PACK.open(7), WILD_PACK.open(7));
        let distinct: std::collections::HashSet<_> = (0..50).map(|s| WILD_PACK.open(s)).collect();
        assert!(distinct.len() > 45);
    }

    /// Over many packs each card turns up close to its fair share:
    /// slots × packs / pool size, within 15%.
    #[test]
    fn distribution_is_fair() {
        let packs = 6000;
        let mut seen: HashMap<CardId, usize> = HashMap::new();
        for seed in 0..packs {
            for id in WILD_PACK.open(seed) {
                *seen.entry(id).or_default() += 1;
            }
        }
        for (r, n, _) in SLOTS {
            let pool = WILD_PACK.pool(r);
            let fair = (packs as usize * n) as f64 / pool.len() as f64;
            for id in pool {
                let got = seen.get(&id).copied().unwrap_or(0) as f64;
                assert!(
                    (got - fair).abs() < fair * 0.15,
                    "{}: {got} vs {fair}",
                    card(id).name
                );
            }
        }
    }
}
