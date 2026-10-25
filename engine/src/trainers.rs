//! Route duelists: the people on the roads who duel for a purse of Facets
//! (`docs/WORLD_DESIGN.md` §4). A beaten duelist is the flag `beat_<id>`
//! (ids start with `t_`), as for rivals, but never counts as a seal.
//!
//! Their decks come from twenty short [`Recipe`]s ("kits"), padded to 60 with
//! basic energy by [`Kit::deck`], plus one signature card per duelist so two
//! duelists on one kit still play differently.

use crate::ai::Difficulty;
use crate::card::{card, CardId, Color};
use crate::deck::{DECK_SIZE, MAX_COPIES};
use crate::lang::T;
use crate::overworld::Progress;
use crate::set::*;

/// A short deck list: the cards of a lower tier (`base`) plus its own.
#[derive(Debug)]
pub struct Recipe {
    /// For tooling (the kit sweep); never shown.
    pub name: &'static str,
    base: Option<&'static Recipe>,
    adds: &'static [(CardId, u8)],
}

impl Recipe {
    /// Every card, the base tier's first.
    pub fn list(&self) -> Vec<(CardId, u8)> {
        let mut l = self.base.map_or(vec![], Recipe::list);
        l.extend_from_slice(self.adds);
        l
    }
}

/// A duelist's deck recipe: one kit, or the first half of two.
#[derive(Debug, Clone, Copy)]
pub enum Kit {
    One(&'static Recipe),
    Mix(&'static Recipe, &'static Recipe),
}

impl Kit {
    /// The 60 cards: the kit's list (and the signature), padded with basic
    /// energy of the creatures' colours by their share of the creatures (Fire
    /// and Water half each for a colorless deck). Panics past 4 copies of a
    /// card or 60 cards: the table is wrong.
    pub fn deck(&self, signature: Option<(CardId, u8)>) -> Vec<CardId> {
        let half = |r: &Recipe| {
            let l = r.list();
            l[..l.len().div_ceil(2)].to_vec()
        };
        let mut list = match *self {
            Kit::One(r) => r.list(),
            Kit::Mix(a, b) => [half(a), half(b)].concat(),
        };
        list.extend(signature);
        let mut deck: Vec<CardId> = vec![];
        for (id, n) in list {
            deck.extend(std::iter::repeat_n(id, n.into()));
            let copies = deck.iter().filter(|&&c| c == id).count();
            assert!(
                copies <= MAX_COPIES,
                "{} copies of {}",
                copies,
                card(id).name
            );
        }
        let mut weights: Vec<(Color, usize)> = vec![];
        for cr in deck.iter().filter_map(|&id| card(id).creature()) {
            if cr.color == Color::Colorless {
                continue;
            }
            match weights.iter_mut().find(|(c, _)| *c == cr.color) {
                Some((_, n)) => *n += 1,
                None => weights.push((cr.color, 1)),
            }
        }
        if weights.is_empty() {
            weights = vec![(Color::Fire, 1), (Color::Water, 1)];
        }
        let room = DECK_SIZE
            .checked_sub(deck.len())
            .expect("a kit over 60 cards");
        let total: usize = weights.iter().map(|w| w.1).sum();
        let mut left = room;
        for (i, &(c, n)) in weights.iter().enumerate() {
            let k = if i + 1 == weights.len() {
                left
            } else {
                room * n / total
            };
            deck.extend(std::iter::repeat_n(basic_energy(c).unwrap(), k));
            left -= k;
        }
        deck
    }
}

/// A route duelist.
#[derive(Debug)]
pub struct Trainer {
    /// `t_<name>`; used by dialogue (`duel:`, `beat:`) and the flag `beat_<id>`.
    pub id: &'static str,
    /// A proper name, the same in both languages.
    pub name: &'static str,
    pub class: T,
    pub kit: Kit,
    /// One card this duelist always plays, on top of the kit.
    pub signature: Option<(CardId, u8)>,
    /// 1..=3 for route duelists.
    pub prizes: u8,
    pub difficulty: Difficulty,
    /// Out only at night (the map data enforces it): the purse is half again.
    pub night: bool,
    /// Overrides the default purse.
    pub purse: Option<u32>,
}

/// The default purse: this base plus this much per prize.
pub const PURSE_BASE: u32 = 15;
pub const PURSE_PER_PRIZE: u32 = 15;

impl Trainer {
    pub fn deck(&self) -> Vec<CardId> {
        self.kit.deck(self.signature)
    }

    pub fn flag(&self) -> String {
        format!("beat_{}", self.id)
    }

    pub fn beaten(&self, p: &Progress) -> bool {
        p.has_flag(&self.flag())
    }

    /// The Facets the first win pays: 15 + 15 per prize (half again at night).
    pub fn purse(&self) -> u32 {
        let base = self
            .purse
            .unwrap_or(PURSE_BASE + PURSE_PER_PRIZE * u32::from(self.prizes));
        if self.night {
            base * 3 / 2
        } else {
            base
        }
    }
}

/// Route duelist by id.
pub fn trainer(id: &str) -> Option<&'static Trainer> {
    TRAINERS.iter().find(|t| t.id == id)
}

/// Record a finished duel against `t`: remember the result for the post-duel
/// line; the first win marks the duelist beaten and pays its purse, returned.
pub fn record(t: &Trainer, won: bool, p: &mut Progress) -> u32 {
    let first = won && !t.beaten(p);
    p.set_last_duel(t.id, won);
    if !first {
        return 0;
    }
    p.apply(&format!("flag:{}", t.flag()))
        .expect("duelist ids are valid flag names");
    p.earn(t.purse());
    t.purse()
}

macro_rules! recipe {
    ($name:literal, $base:expr, [$($card:ident $n:literal),* $(,)?]) => {
        Recipe { name: $name, base: $base, adds: &[$(($card, $n)),*] }
    };
}

// Sprout (Basics only), Bud (+ Stage 1, a draw and a search trainer), Bloom
// (+ Stage 2), per colour.
pub static KINDLING: Recipe = recipe!("Kindling", None, [CINDREL 4, ASHPAW 4, KILNBACK 3, WAXWICK 3, MOSS_POULTICE 2, DOWSING_ROD 2]);
pub static BELLOWS: Recipe = recipe!("Bellows", Some(&KINDLING), [FLARROW 3, BLAZELYNX 2, FURNACEBACK 2, CINDERHORN 1, LUCKY_SATCHEL 2, TRAIL_COMPASS 1]);
pub static FORGE: Recipe =
    recipe!("Forge", Some(&BELLOWS), [PYREQUILL 1, SOLARLYNX 1, ASH_SIFTER 2]);
pub static PUDDLE: Recipe =
    recipe!("Puddle", None, [PUDDOCK 4, RILLET 4, KELPUP 4, CLEARWATER_VIAL 2]);
pub static CURRENT: Recipe = recipe!("Current", Some(&PUDDLE), [BROOKHORN 3, STREAMFIN 2, KELPWARDEN 2, SHELLMOTE 1, LUCKY_SATCHEL 2, TRAIL_COMPASS 1]);
pub static UNDERTOW: Recipe =
    recipe!("Undertow", Some(&CURRENT), [TIDEMAW 1, GULFWYRM 1, FROSTEEL 1]);
pub static SEEDLING: Recipe =
    recipe!("Seedling", None, [SPRIGGET 4, MIREBUD 4, POLLENPUP 3, FERNWHISK 3, MOSS_POULTICE 2]);
pub static HEDGEROW: Recipe = recipe!("Hedgerow", Some(&SEEDLING), [THORNBUCK 3, BOGBLOSSOM 2, HONEYMANE 2, BRIARHOG 1, LUCKY_SATCHEL 2, TRAIL_COMPASS 1]);
pub static CANOPY: Recipe =
    recipe!("Canopy", Some(&HEDGEROW), [ELDERHART 1, BOGCROWN 1, GROVE_BALM 2]);
pub static SPARK: Recipe =
    recipe!("Spark", None, [SPARKIT 4, BUZZLET 4, OHMLET 3, ZIPPET 3, FLASHFIN 2]);
pub static STATIC: Recipe = recipe!("Static", Some(&SPARK), [ARCFANG 3, HIVEVOLT 2, OHMRUNNER 2, STORMOX 1, LUCKY_SATCHEL 2, TRAIL_COMPASS 1]);
pub static TEMPEST: Recipe =
    recipe!("Tempest", Some(&STATIC), [FULGUREX 1, CHARGE_COIL 1, JOLT_WIRE 1]);
pub static PEBBLE: Recipe = recipe!("Pebble", None, [KNOLLOP 4, TUSKLET 4, BRAWLET 3, SHALEPUP 3]);
pub static RUBBLE: Recipe = recipe!("Rubble", Some(&PEBBLE), [KNOLLGRIM 3, IRONBRAWL 2, SHALEHOUND 2, GRITFANG 1, BOULDERGUT 1, LUCKY_SATCHEL 2, TRAIL_COMPASS 1]);
pub static LANDSLIDE: Recipe =
    recipe!("Landslide", Some(&RUBBLE), [KNOLLOSSUS 1, CRAGMASTIFF 1, SPIKE_CHARM 2]);
pub static MURMUR: Recipe = recipe!("Murmur", None, [WISPUN 4, DOZEL 4, HEXLET 4, SMOKE_PELLET 2]);
pub static REVERIE: Recipe = recipe!("Reverie", Some(&MURMUR), [WISPELLE 3, SOMNITAR 2, HEXMAGE 2, GLIMMOX 1, GLOAMOWL 1, LUCKY_SATCHEL 2, TRAIL_COMPASS 1]);
pub static TRANCE: Recipe =
    recipe!("Trance", Some(&REVERIE), [WRAITHELLE 1, HEXARCH 1, UNWIND_SPRAY 1]);
pub static SCRAP: Recipe = recipe!("Scrap", None, [BURROWBIT 4, PECKLET 4, POCKETMOUSE 3, SCAMPIT 3, RELAY_BATON 2, TWOFOLD_ENERGY 2]);
pub static FLOCK: Recipe = recipe!("Flock", Some(&SCRAP), [WARRENBUCK 3, GALEHERON 2, ECHOPUP 2, HAULOX 1, MENDLARK 1, LUCKY_SATCHEL 2, TRAIL_COMPASS 1]);

/// Every kit, Sprout tiers first per colour.
pub static KITS: [&Recipe; 20] = [
    &KINDLING, &BELLOWS, &FORGE, &PUDDLE, &CURRENT, &UNDERTOW, &SEEDLING, &HEDGEROW, &CANOPY,
    &SPARK, &STATIC, &TEMPEST, &PEBBLE, &RUBBLE, &LANDSLIDE, &MURMUR, &REVERIE, &TRANCE, &SCRAP,
    &FLOCK,
];

/// The Sprout kits: Basics only, the first duelists a player meets.
pub static SPROUTS: [&Recipe; 7] = [
    &KINDLING, &PUDDLE, &SEEDLING, &SPARK, &PEBBLE, &MURMUR, &SCRAP,
];

macro_rules! duelist {
    ($id:literal, $name:literal, $en:literal / $fr:literal, $kit:expr, $sig:ident $n:literal, $prizes:literal, $level:ident) => {
        Trainer {
            id: $id,
            name: $name,
            class: T::new($en, $fr),
            kit: $kit,
            signature: Some(($sig, $n)),
            prizes: $prizes,
            difficulty: Difficulty::$level,
            night: false,
            purse: None,
        }
    };
}

use Kit::{Mix, One};

/// The 1.2 roster, by map: meadow, woodland, boardwalk, quarry, square.
pub static TRAINERS: [Trainer; 14] = [
    duelist!("t_tobin", "Tobin", "Junior Duelist" / "Duelliste en herbe", One(&SCRAP), WARRENBUCK 2, 1, Easy),
    duelist!("t_hazel", "Hazel", "Beekeeper" / "Apicultrice", Mix(&SEEDLING, &SPARK), HIVEVOLT 2, 2, Easy),
    duelist!("t_ossian", "Ossian", "Birdwatcher" / "Ornithologue", One(&SCRAP), GALEHERON 2, 1, Easy),
    duelist!("t_sorrel", "Sorrel", "Forager" / "Cueilleuse", One(&SEEDLING), BOGBLOSSOM 2, 2, Normal),
    duelist!("t_bryn", "Bryn", "Trail Runner" / "Coureur des sentiers", One(&SEEDLING), BRIARHOG 1, 1, Easy),
    duelist!("t_pell", "Pell", "Angler" / "Pêcheur", One(&PUDDLE), STREAMFIN 1, 2, Easy),
    duelist!("t_skye", "Skye", "Kite Flyer" / "Cerf-voliste", One(&SPARK), STORMOX 1, 2, Normal),
    duelist!("t_morrow", "Morrow", "Shell Collector" / "Collectionneur de coquillages", One(&PUDDLE), BROOKHORN 3, 3, Normal),
    duelist!("t_nell", "Nell", "Lifeguard" / "Maître-nageuse", One(&PUDDLE), KELPWARDEN 2, 2, Normal),
    duelist!("t_tamsin", "Tamsin", "Stonecutter" / "Tailleuse de pierre", One(&PEBBLE), KNOLLGRIM 2, 2, Easy),
    duelist!("t_cole", "Cole", "Kiln Stoker" / "Chauffeur de four", One(&KINDLING), FURNACEBACK 2, 2, Normal),
    duelist!("t_rusk", "Rusk", "Hauler" / "Porteur", Mix(&PEBBLE, &SCRAP), HAULOX 1, 1, Easy),
    duelist!("t_marigold", "Marigold", "Glass Gatherer" / "Cueilleuse de verre", One(&BELLOWS), CINDERHORN 2, 3, Normal),
    duelist!("t_lark", "Lark", "Busker" / "Musicienne de rue", Mix(&SCRAP, &MURMUR), ECHOPUP 2, 1, Easy),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::{rival, seals};
    use crate::card::Stage;
    use crate::validate_deck;

    /// Every kit and every duelist's deck is a legal 60 whose evolutions all
    /// have their line underneath.
    #[test]
    fn kit_decks_are_legal_with_whole_lines() {
        let decks = KITS
            .iter()
            .map(|r| (r.name, Kit::One(r).deck(None)))
            .chain(TRAINERS.iter().map(|t| (t.name, t.deck())));
        for (name, d) in decks {
            assert_eq!(validate_deck(&d), Ok(()), "{name}");
            for cr in d.iter().filter_map(|&id| card(id).creature()) {
                if let Some(from) = cr.evolves_from {
                    assert!(d.iter().any(|&id| card(id).name == from), "{name}: {from}");
                }
            }
        }
        for r in SPROUTS {
            let d = Kit::One(r).deck(None);
            assert!(
                d.iter()
                    .filter_map(|&id| card(id).creature())
                    .all(|c| c.stage == Stage::Basic),
                "{}",
                r.name
            );
        }
    }

    #[test]
    fn mixes_take_half_of_each_kit_and_energy_follows_the_creatures() {
        let d = Kit::Mix(&SEEDLING, &SPARK).deck(Some((HIVEVOLT, 2)));
        let n = |id| d.iter().filter(|&&c| c == id).count();
        // Seedling's first three entries, Spark's first three, two Hivevolt.
        assert_eq!((n(SPRIGGET), n(POLLENPUP), n(FERNWHISK)), (4, 3, 0));
        assert_eq!((n(BUZZLET), n(OHMLET), n(ZIPPET)), (4, 3, 0));
        assert_eq!(n(HIVEVOLT), 2);
        // 11 Grass and 13 Lightning creatures share the 36 energy.
        assert_eq!((n(GRASS_ENERGY), n(LIGHTNING_ENERGY)), (16, 20));
        // A colorless kit splits Fire and Water.
        let s = Kit::One(&SCRAP).deck(None);
        let e = |id| s.iter().filter(|&&c| c == id).count();
        assert_eq!(
            (e(FIRE_ENERGY), e(WATER_ENERGY), e(TWOFOLD_ENERGY)),
            (21, 21, 2)
        );
    }

    #[test]
    #[should_panic(expected = "5 copies")]
    fn a_kit_past_four_copies_is_refused() {
        Kit::One(&SCRAP).deck(Some((BURROWBIT, 1)));
    }

    #[test]
    fn the_roster_is_well_formed() {
        for (i, t) in TRAINERS.iter().enumerate() {
            assert!(
                t.id.starts_with("t_") && t.id.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'),
                "{}",
                t.id
            );
            assert!(TRAINERS[..i]
                .iter()
                .all(|o| o.id != t.id && o.name != t.name));
            assert!(rival(t.id).is_none());
            assert!((1..=3).contains(&t.prizes), "{}", t.name);
            assert!(!t.class.en.is_empty() && !t.class.fr.is_empty());
        }
        // §4.4: five 1-prize, seven 2-prize and two 3-prize duelists.
        assert_eq!(TRAINERS.iter().map(Trainer::purse).sum::<u32>(), 585);
    }

    #[test]
    fn a_first_win_pays_the_purse_once_and_never_a_seal() {
        let mut p = Progress::default();
        let start = p.coins();
        let hazel = trainer("t_hazel").unwrap();
        assert_eq!(hazel.purse(), 45);
        assert_eq!(record(hazel, false, &mut p), 0);
        assert!(p.check("lost:t_hazel") && !p.check("beat:t_hazel"));
        assert_eq!(record(hazel, true, &mut p), 45);
        assert!(p.check("won:t_hazel") && p.check("beat:t_hazel"));
        assert!(p.has_flag("beat_t_hazel") && p.check("count:beat_t_:1"));
        assert_eq!(record(hazel, true, &mut p), 0, "once per duelist");
        assert_eq!((p.coins(), seals(&p)), (start + 45, 0));
    }

    #[test]
    fn night_and_set_purses() {
        let mut t = Trainer {
            night: true,
            ..duelist!("t_x", "X", "a" / "b", One(&SCRAP), ECHOPUP 1, 2, Hard)
        };
        assert_eq!(t.purse(), 67, "45 × 1.5, rounded down");
        t.purse = Some(120);
        assert_eq!(t.purse(), 180);
        t.night = false;
        assert_eq!(t.purse(), 120);
    }
}
