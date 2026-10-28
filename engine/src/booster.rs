//! Boosters: 10-card packs drawn from [`crate::set`], deterministic from a
//! seed. The design and its odds are in `docs/BOOSTERS.md`.
//!
//! A pack is 6 Commons, 3 Uncommons and a Rare slot, in that (reveal)
//! order, all distinct. The Rare slot holds a Holo Rare (an Awaken 2
//! Hexon, [`is_holo`]) at the odds of [`HOLO_ODDS`]; otherwise a Rare Gear
//! or the special energy. A themed pack fills each Common and Uncommon slot from its own
//! types, Gear cards and Colorless Hexons, or the other types, by
//! [`SHARES`], and its Holo Rares are its own types; the Wild pack draws
//! evenly from the whole set. The Rare slot skips cards the player already
//! has [`MAX_COPIES`] of while any other is left. Basic energy is unlimited,
//! so no slot holds it, and promos ([`crate::set::PROMOS`]) are campaign
//! rewards that no pack holds.

use crate::card::{card, CardId, CardKind, Color, Rarity};
use crate::collection::Collection;
use crate::deck::MAX_COPIES;
use crate::lang::T;
use crate::rng::Rng;
use crate::set::{is_holo, is_promo};

pub const PACK_SIZE: usize = COMMONS + UNCOMMONS + 1;
pub const COMMONS: usize = 6;
pub const UNCOMMONS: usize = 3;
/// The Rare slot holds a Holo Rare in `HOLO_ODDS.0` packs of `HOLO_ODDS.1`.
pub const HOLO_ODDS: (u32, u32) = (3, 5);
/// A themed pack's Common and Uncommon slots, in percent: its own types,
/// then Gear cards and Colorless Hexons, then the other types.
pub const SHARES: [u32; 3] = [60, 35, 5];

#[derive(Debug)]
pub struct Booster {
    pub name: T,
    pub description: T,
    /// The Hexon types this pack favours (every type: the Wild pack).
    pub colors: &'static [Color],
}

pub static EMBER_PACK: Booster = Booster {
    name: T::new("Ember Pack", "Booster Braise"),
    description: T::new(
        "Ember and Stone Hexons, plus Gear cards.",
        "Hexons Braise et Roc, plus des cartes Objet.",
    ),
    colors: &[Color::Fire, Color::Fighting],
};

pub static TIDE_PACK: Booster = Booster {
    name: T::new("Tide Pack", "Booster Marée"),
    description: T::new(
        "Tide and Dream Hexons, plus Gear cards.",
        "Hexons Marée et Songe, plus des cartes Objet.",
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
        "Grove and Storm Hexons, plus Gear cards.",
        "Hexons Bosquet et Orage, plus des cartes Objet.",
    ),
    colors: &[Color::Grass, Color::Lightning],
};

pub static STORM_PACK: Booster = Booster {
    name: T::new("Storm Pack", "Booster Orage"),
    description: T::new(
        "Storm and Tide Hexons, plus Gear cards.",
        "Hexons Orage et Marée, plus des cartes Objet.",
    ),
    colors: &[Color::Lightning, Color::Water],
};

pub static STONE_PACK: Booster = Booster {
    name: T::new("Stone Pack", "Booster Roc"),
    description: T::new(
        "Stone and Neutral Hexons, plus Gear cards.",
        "Hexons Roc et Neutres, plus des cartes Objet.",
    ),
    colors: &[Color::Fighting, Color::Colorless],
};

pub static DREAM_PACK: Booster = Booster {
    name: T::new("Dream Pack", "Booster Songe"),
    description: T::new(
        "Dream and Grove Hexons, plus Gear cards.",
        "Hexons Songe et Bosquet, plus des cartes Objet.",
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

/// A slot's groups, by [`SHARES`]: own types, neutral, other types.
type Groups = [Vec<CardId>; 3];

impl Booster {
    /// The Wild pack: every type, drawn evenly.
    pub fn is_wild(&self) -> bool {
        self.colors.len() > 2
    }

    /// Which [`SHARES`] group a card is in for this pack: 0 its own types,
    /// 1 Gear cards, the special energy and (unless they are its own type)
    /// Colorless Hexons, 2 the other types. The Wild pack has one group.
    pub fn group(&self, id: CardId) -> usize {
        if self.is_wild() {
            return 0;
        }
        match &card(id).kind {
            CardKind::Creature(c) if self.colors.contains(&c.color) => 0,
            CardKind::Creature(c) if c.color != Color::Colorless => 2,
            _ => 1,
        }
    }

    /// Whether this card can turn up in the pack: any non-promo card but
    /// basic energy, and for a themed pack's Rare slot, its own Holo Rares.
    pub fn holds(&self, id: CardId) -> bool {
        let c = card(id);
        !is_promo(id) && c.rarity != Rarity::Energy && (!is_holo(id) || self.group(id) == 0)
    }

    /// The cards one slot draws from, by group, in id order: `Rare` with
    /// `holo` for the upgraded Rare slot.
    pub fn pool(&self, rarity: Rarity, holo: bool) -> Groups {
        let mut g = Groups::default();
        for id in crate::set::ids().filter(|&id| {
            card(id).rarity == rarity
                && self.holds(id)
                && (rarity != Rarity::Rare || is_holo(id) == holo)
        }) {
            g[self.group(id)].push(id);
        }
        g
    }

    /// Opens a pack: the same seed and collection always give the same
    /// [`PACK_SIZE`] cards. `owned` only steers the Rare slot.
    pub fn open(&self, seed: u64, owned: &Collection) -> Vec<CardId> {
        let mut rng = Rng::new(seed);
        let mut pack = Vec::with_capacity(PACK_SIZE);
        for (rarity, n) in [(Rarity::Common, COMMONS), (Rarity::Uncommon, UNCOMMONS)] {
            let pool = self.pool(rarity, false);
            for _ in 0..n {
                let id = draw(&mut rng, &pool, SHARES, &pack);
                pack.push(id);
            }
        }
        let holo = rng.below(HOLO_ODDS.1) < HOLO_ODDS.0;
        let mut pool = self.pool(Rarity::Rare, holo);
        let spare = |id: &CardId| owned.count(*id) < MAX_COPIES;
        if pool.iter().flatten().any(spare) {
            pool.iter_mut().for_each(|g| g.retain(spare));
        }
        pack.push(draw(&mut rng, &pool, [1, 1, 1], &pack));
        pack
    }
}

/// One card not in `taken`: a group by `weights` (among groups with a card
/// left), then a card of it, evenly.
fn draw(rng: &mut Rng, pool: &Groups, weights: [u32; 3], taken: &[CardId]) -> CardId {
    let free: Vec<Vec<CardId>> = pool
        .iter()
        .map(|g| g.iter().copied().filter(|id| !taken.contains(id)).collect())
        .collect();
    let w: Vec<u32> = (0..3)
        .map(|i| if free[i].is_empty() { 0 } else { weights[i] })
        .collect();
    let mut r = rng.below(w.iter().sum());
    let g = (0..3)
        .find(|&i| {
            r < w[i] || {
                r -= w[i];
                false
            }
        })
        .expect("a slot always has a card left");
    free[g][rng.below(free[g].len() as u32) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::set::*;
    use std::collections::HashMap;

    fn fresh() -> Collection {
        Collection::new()
    }

    /// The reveal order: Commons, Uncommons, the Rare slot.
    fn slot_rarity(i: usize) -> Rarity {
        match i {
            i if i < COMMONS => Rarity::Common,
            i if i < COMMONS + UNCOMMONS => Rarity::Uncommon,
            _ => Rarity::Rare,
        }
    }

    #[test]
    fn pools_fill_every_slot() {
        for b in BOOSTERS {
            let n = |r, holo| b.pool(r, holo).iter().map(Vec::len).sum::<usize>();
            assert!(n(Rarity::Common, false) >= COMMONS, "{}", b.name);
            assert!(n(Rarity::Uncommon, false) >= UNCOMMONS, "{}", b.name);
            assert!(n(Rarity::Rare, true) >= 3, "{}: holos", b.name);
            assert!(n(Rarity::Rare, false) >= 1, "{}", b.name);
            if !b.is_wild() {
                for r in [Rarity::Common, Rarity::Uncommon] {
                    assert!(
                        b.pool(r, false).iter().all(|g| !g.is_empty()),
                        "{} {r:?}",
                        b.name
                    );
                }
            }
        }
    }

    #[test]
    fn pack_composition() {
        for b in BOOSTERS {
            for seed in 0..300 {
                let pack = b.open(seed, &fresh());
                assert_eq!(pack.len(), PACK_SIZE);
                for (i, &id) in pack.iter().enumerate() {
                    assert_eq!(card(id).rarity, slot_rarity(i), "{} seed {seed}", b.name);
                    assert!(b.holds(id) && !is_promo(id) && !card(id).is_basic_energy());
                    assert!(!pack[..i].contains(&id), "{} seed {seed}: repeated", b.name);
                }
            }
        }
    }

    #[test]
    fn themed_packs_lean_to_their_types() {
        assert_eq!(EMBER_PACK.group(CINDREL), 0);
        assert_eq!(EMBER_PACK.group(TUSKLET), 0);
        assert_eq!(EMBER_PACK.group(SCAMPIT), 1, "Neutral Hexon");
        assert_eq!(EMBER_PACK.group(SNARE_HOOK), 1);
        assert_eq!(EMBER_PACK.group(TWOFOLD_ENERGY), 1);
        assert_eq!(EMBER_PACK.group(PUDDOCK), 2);
        assert_eq!(STONE_PACK.group(SCAMPIT), 0, "Stone's own type");
        // Off-type Holo Rares stay out of themed packs; energy and promos out of all.
        assert!(!EMBER_PACK.holds(TIDEMAW) && TIDE_PACK.holds(TIDEMAW));
        assert!(GROVE_PACK.holds(ELDERHART) && GROVE_PACK.holds(FULGUREX));
        assert!(!GROVE_PACK.holds(GULFWYRM) && STORM_PACK.holds(GULFWYRM));
        assert!(STONE_PACK.holds(WARRENKING) && !EMBER_PACK.holds(WARRENKING));
        assert!(DREAM_PACK.holds(HEXARCH) && !DREAM_PACK.holds(FULGUREX));
        assert!(EMBER_PACK.holds(PUDDOCK), "a stray off-type Common");
        assert!(BOOSTERS.iter().all(|b| !b.holds(FIRE_ENERGY)));
        assert!(crate::set::ids()
            .all(|id| WILD_PACK.holds(id) == (!is_promo(id) && !card(id).is_basic_energy())));
        // Holo Rares are the Awaken 2 Hexons.
        let holos: Vec<CardId> = crate::set::ids().filter(|&id| is_holo(id)).collect();
        assert_eq!(holos.len(), 12);
        assert!(holos
            .iter()
            .all(|&id| card(id).creature().unwrap().stage == crate::card::Stage::Stage2));
    }

    #[test]
    fn same_seed_same_pack() {
        assert_eq!(WILD_PACK.open(7, &fresh()), WILD_PACK.open(7, &fresh()));
        let distinct: std::collections::HashSet<_> =
            (0..50).map(|s| WILD_PACK.open(s, &fresh())).collect();
        assert!(distinct.len() > 45);
    }

    /// The Rare slot passes over cards held [`MAX_COPIES`] times.
    #[test]
    fn rare_slot_skips_full_playsets() {
        let mut owned = fresh();
        let [own, _, _] = EMBER_PACK.pool(Rarity::Rare, true);
        for &id in &own[1..] {
            owned.add(id, MAX_COPIES);
        }
        let mut holos = 0;
        for seed in 0..300 {
            let rare = *EMBER_PACK.open(seed, &owned).last().unwrap();
            if is_holo(rare) {
                assert_eq!(rare, own[0], "seed {seed}");
                holos += 1;
            }
        }
        assert!(holos > 50);
        // Every Holo maxed: they turn up again rather than nothing.
        owned.add(own[0], MAX_COPIES);
        assert!((0..300).any(|s| is_holo(*EMBER_PACK.open(s, &owned).last().unwrap())));
    }

    /// Over many seeded packs the slot rates and type shares match
    /// `docs/BOOSTERS.md`: a Holo in three Rare slots of five, 60/35/5 in a
    /// themed pack's Common and Uncommon slots, and in the Wild pack every
    /// card close to its even share.
    #[test]
    fn slot_rates_match_the_design() {
        let packs = 12_000;
        for b in BOOSTERS {
            let mut holo = 0;
            let mut groups = [0usize; 3];
            let mut seen: HashMap<CardId, usize> = HashMap::new();
            for seed in 0..packs {
                let pack = b.open(seed, &fresh());
                holo += usize::from(is_holo(pack[PACK_SIZE - 1]));
                for &id in &pack[..PACK_SIZE - 1] {
                    groups[b.group(id)] += 1;
                    *seen.entry(id).or_default() += 1;
                }
            }
            let rate = holo as f64 / packs as f64;
            assert!(
                (rate - f64::from(HOLO_ODDS.0) / f64::from(HOLO_ODDS.1)).abs() < 0.015,
                "{}: holo {rate:.3}",
                b.name
            );
            let slots = (packs * (PACK_SIZE as u64 - 1)) as f64;
            if b.is_wild() {
                for (r, n) in [(Rarity::Common, COMMONS), (Rarity::Uncommon, UNCOMMONS)] {
                    let [pool, _, _] = b.pool(r, false);
                    let fair = (packs as usize * n) as f64 / pool.len() as f64;
                    for id in pool {
                        let got = seen.get(&id).copied().unwrap_or(0) as f64;
                        assert!(
                            (got - fair).abs() < fair * 0.1,
                            "{}: {got} vs {fair}",
                            card(id).name
                        );
                    }
                }
            } else {
                for (g, &want) in groups.iter().zip(&SHARES) {
                    let share = *g as f64 / slots * 100.0;
                    assert!(
                        (share - f64::from(want)).abs() < 1.5,
                        "{}: {share:.1}% vs {want}%",
                        b.name
                    );
                }
            }
        }
    }

    /// Prints the per-card odds and packs-to-complete table of
    /// `docs/BOOSTERS.md`: `cargo test -p sixfold-engine booster_report -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn booster_report() {
        let packs = 40_000;
        for b in BOOSTERS {
            let mut seen: HashMap<CardId, usize> = HashMap::new();
            for seed in 0..packs {
                for id in b.open(seed, &fresh()) {
                    *seen.entry(id).or_default() += 1;
                }
            }
            let odds =
                |id: CardId| seen.get(&id).copied().unwrap_or(0) as f64 / packs as f64 * 100.0;
            let mut line = format!("{}:", b.name.en);
            for (r, holo, label) in [
                (Rarity::Common, false, "C"),
                (Rarity::Uncommon, false, "U"),
                (Rarity::Rare, false, "R"),
                (Rarity::Rare, true, "H"),
            ] {
                for (g, ids) in b.pool(r, holo).iter().enumerate() {
                    if let Some(&id) = ids.first() {
                        let avg = ids.iter().map(|&i| odds(i)).sum::<f64>() / ids.len() as f64;
                        line += &format!(
                            " {label}{g} n={} each {avg:.2}% (e.g. {:.2}% {});",
                            ids.len(),
                            odds(id),
                            card(id).name
                        );
                    }
                }
            }
            // Packs until one copy of every own-type and neutral card (all
            // of them for Wild), over 400 runs.
            let goal: Vec<CardId> = crate::set::ids()
                .filter(|&id| b.holds(id) && b.group(id) < 2)
                .collect();
            let mut runs: Vec<usize> = (0..400u64)
                .map(|run| {
                    let mut owned = fresh();
                    let mut n = 0;
                    while goal.iter().any(|&id| owned.count(id) == 0) {
                        owned.add_all(&b.open(run * 1_000_003 + n as u64, &owned));
                        n += 1;
                    }
                    n
                })
                .collect();
            runs.sort();
            let mean = runs.iter().sum::<usize>() as f64 / runs.len() as f64;
            eprintln!(
                "{line}\n  complete {} cards: mean {mean:.0}, median {}, 90% {}",
                goal.len(),
                runs[200],
                runs[360]
            );
        }
    }
}
