//! Route duelists: the people on the roads who duel for a purse of Facets
//! (`docs/WORLD_DESIGN.md` §4). A beaten duelist is the flag `beat_<id>`
//! (ids start with `t_`), as for rivals, but never counts as a seal.
//!
//! Their decks come from twenty short [`Recipe`]s ("kits"), padded to 60 with
//! basic energy by [`Kit::deck`], plus one signature card per duelist so two
//! duelists on one kit still play differently.
//!
//! Rook's four story duels (`rook_1` … `rook_4`, §8) sit here too, with
//! hand-written decks of Neutral Hexons: they pay a purse like a duelist,
//! never a seal, and a cutscene moves the story on whether the player wins or
//! loses ([`Duelist::story`]).

use crate::overworld::Progress;
use sixfold_engine::ai::Difficulty;
use sixfold_engine::set::*;
use sixfold_engine::T;
use sixfold_engine::{card, CardId, CardKind, Color, EnergyCard, Rarity, Stage};
use sixfold_engine::{DECK_SIZE, MAX_COPIES};

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
pub struct Duelist {
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
    /// A challenge rule the player's deck must keep (1.3).
    pub rule: Option<Rule>,
    /// A secret's id (#116): out only once someone's line sets
    /// `flag:heard_<id>` (the map's `if` says so), and named by no one before.
    pub heard: Option<&'static str>,
}

/// A challenge-rule duelist's deck rule (`docs/WORLD_DESIGN.md` §10): decks
/// that break it are greyed out in the challenge sheet's picker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    /// No Stage 1 or Stage 2 creature.
    BasicsOnly,
    /// Creatures and energy only.
    NoGear,
    /// Creatures and basic energy of one colour (plus Colorless creatures).
    MonoColour,
    /// At most 16 energy cards.
    LightEnergy,
    /// No Rare.
    NoRares,
    /// At least two Stage 2 creatures.
    TwoStage2,
}

impl Rule {
    /// Whether `deck` keeps the rule.
    pub fn allows(self, deck: &[CardId]) -> bool {
        let defs = || deck.iter().map(|&id| card(id));
        let stage = |s| {
            defs()
                .filter(|c| c.creature().is_some_and(|cr| cr.stage == s))
                .count()
        };
        match self {
            Rule::BasicsOnly => {
                stage(Stage::Basic) == defs().filter(|c| c.creature().is_some()).count()
            }
            Rule::NoGear => {
                defs().all(|c| matches!(c.kind, CardKind::Creature(_) | CardKind::Energy(_)))
            }
            Rule::MonoColour => {
                let mut colours = defs().filter_map(|c| match &c.kind {
                    CardKind::Energy(EnergyCard::Basic(col)) => Some(*col),
                    _ => c
                        .creature()
                        .map(|cr| cr.color)
                        .filter(|&col| col != Color::Colorless),
                });
                let first = colours.next();
                colours.all(|c| Some(c) == first)
            }
            Rule::LightEnergy => {
                defs()
                    .filter(|c| matches!(c.kind, CardKind::Energy(_)))
                    .count()
                    <= 16
            }
            Rule::NoRares => defs().all(|c| c.rarity != Rarity::Rare),
            Rule::TwoStage2 => stage(Stage::Stage2) >= 2,
        }
    }

    /// What the rule asks, as the challenge sheet says it.
    pub fn text(self) -> T {
        match self {
            Rule::BasicsOnly => T::new(
                "Basic Hexons only: no Awaken 1 or Awaken 2.",
                "Hexons de Base uniquement\u{a0}: ni Éveil 1 ni Éveil 2.",
            ),
            Rule::NoGear => T::new(
                "Hexons and energy only: no Gear.",
                "Hexons et énergies uniquement\u{a0}: aucun Objet.",
            ),
            Rule::MonoColour => T::new(
                "One colour only, plus Neutral Hexons.",
                "Une seule couleur, plus les Hexons Neutres.",
            ),
            Rule::LightEnergy => T::new("At most 16 energy cards.", "16 cartes Énergie au plus."),
            Rule::NoRares => T::new("No Rares.", "Aucune carte Rare."),
            Rule::TwoStage2 => T::new(
                "At least two Awaken 2 Hexons.",
                "Au moins deux Hexons d'Éveil 2.",
            ),
        }
    }
}

/// The default purse: this base plus this much per prize.
pub const PURSE_BASE: u32 = 15;
pub const PURSE_PER_PRIZE: u32 = 15;

impl Duelist {
    pub fn deck(&self) -> Vec<CardId> {
        self.kit.deck(self.signature)
    }

    pub fn flag(&self) -> String {
        format!("beat_{}", self.id)
    }

    pub fn beaten(&self, p: &Progress) -> bool {
        p.has_flag(&self.flag())
    }

    /// One of Rook's story duels (`rook_<n>`), not a route duelist.
    pub fn story(&self) -> bool {
        self.id.starts_with("rook_")
    }

    /// The Facets a win pays now: the purse the first time; again, nothing,
    /// but a story duel (Rook's rematch in the hall) pays like a rival's
    /// rematch, 10 per prize.
    pub fn win_pays(&self, p: &Progress) -> u32 {
        match (self.beaten(p), self.story()) {
            (false, _) => self.purse(),
            (true, true) => crate::campaign::REMATCH_PER_PRIZE * u32::from(self.prizes),
            (true, false) => 0,
        }
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
pub fn duelist(id: &str) -> Option<&'static Duelist> {
    DUELISTS.iter().find(|t| t.id == id)
}

/// Record a finished duel against `t`: remember the result for the post-duel
/// line; a win pays [`Duelist::win_pays`] (returned), and the first one marks
/// the duelist beaten.
pub fn record(t: &Duelist, won: bool, p: &mut Progress) -> u32 {
    let pays = if won { t.win_pays(p) } else { 0 };
    p.set_last_duel(t.id, won);
    if won && !t.beaten(p) {
        p.apply(&format!("flag:{}", t.flag()))
            .expect("duelist ids are valid flag names");
    }
    p.earn(pays);
    pays
}

macro_rules! recipe {
    ($name:literal, $base:expr, [$($card:ident $n:literal),* $(,)?]) => {
        Recipe { name: $name, base: $base, adds: &[$(($card, $n)),*] }
    };
}

// Sprout (Basics only), Bud (+ Stage 1, a draw and a search Gear card), Bloom
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
    recipe!("Landslide", Some(&RUBBLE), [KNOLLOSSUS 1, CRAGMASTIFF 1, SPIKE_CHARM 1]);
pub static MURMUR: Recipe = recipe!("Murmur", None, [WISPUN 4, DOZEL 4, HEXLET 4, SMOKE_PELLET 2]);
pub static REVERIE: Recipe = recipe!("Reverie", Some(&MURMUR), [WISPELLE 3, SOMNITAR 2, HEXMAGE 2, GLIMMOX 1, GLOAMOWL 1, LUCKY_SATCHEL 2, TRAIL_COMPASS 1]);
pub static TRANCE: Recipe =
    recipe!("Trance", Some(&REVERIE), [WRAITHELLE 1, HEXARCH 1, UNWIND_SPRAY 1]);
pub static SCRAP: Recipe = recipe!("Scrap", None, [BURROWBIT 4, PECKLET 4, POCKETMOUSE 3, SCAMPIT 3, RELAY_BATON 2, TWOFOLD_ENERGY 2]);
pub static FLOCK: Recipe = recipe!("Flock", Some(&SCRAP), [WARRENBUCK 3, GALEHERON 2, ECHOPUP 2, HAULOX 1, MENDLARK 1, LUCKY_SATCHEL 2, TRAIL_COMPASS 1]);

// Rook's decks (§8.2): each the one before plus a few cards, basic energy
// trimmed to keep 60 (Kit::deck pads Fire and Water for a Neutral deck); the
// last is its own list.
pub static BLANK_SLATE: Recipe = recipe!("Blank Slate", None, [BURROWBIT 4, PECKLET 4, POCKETMOUSE 4, SCAMPIT 3, RELAY_BATON 2, LUCKY_SATCHEL 2]);
pub static OPEN_SKY: Recipe = recipe!("Open Sky", Some(&BLANK_SLATE), [WARRENBUCK 3, GALEHERON 3, ECHOPUP 2, HAULOX 2, TWOFOLD_ENERGY 2]);
pub static HOLLOW_CROWN: Recipe = recipe!("Hollow Crown", Some(&OPEN_SKY), [WARRENKING 2, MENDLARK 3, WIDE_SATCHEL 1, SNARE_HOOK 1]);
pub static SEVENTH_FACE: Recipe = recipe!("Seventh Face", None, [
    BURROWBIT 4, WARRENBUCK 3, WARRENKING 2, PECKLET 4, GALEHERON 3, MENDLARK 3, ECHOPUP 2, HAULOX 2, POCKETMOUSE 2,
    LUCKY_SATCHEL 3, TRAIL_COMPASS 2, SNARE_HOOK 2, HEARTY_BROTH 2, WIDE_SATCHEL 1, RELAY_BATON 1, TWOFOLD_ENERGY 4,
]);

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
        Duelist {
            id: $id,
            name: $name,
            class: T::new($en, $fr),
            kit: $kit,
            signature: Some(($sig, $n)),
            prizes: $prizes,
            difficulty: Difficulty::$level,
            night: false,
            purse: None,
            rule: None,
            heard: None,
        }
    };
}

/// The seals each of Rook's scenes waits for (the maps' `scripts` say so too,
/// and `meta/tests/world_data.rs` holds them to it).
pub const ROOK_SEALS: [(&str, usize); 4] =
    [("rook_1", 1), ("rook_2", 3), ("rook_3", 5), ("rook_4", 6)];

macro_rules! rook {
    ($id:literal, $kit:expr, $prizes:literal, $level:ident, $purse:literal) => {
        Duelist {
            id: $id,
            name: "Rook",
            class: T::new("The Blank", "Le Sans-Couleur"),
            kit: One($kit),
            signature: None,
            prizes: $prizes,
            difficulty: Difficulty::$level,
            night: false,
            purse: Some($purse),
            rule: None,
            heard: None,
        }
    };
}

use Kit::{Mix, One};

/// The roster, by map. 1.2: meadow, woodland, boardwalk, quarry, square.
/// 1.3 Ember: cinderwick, kilnmouth; Storm: downs, rodspire, stormwatch;
/// Tide: gullhaven, bellflats; Dream: hushlane, lanternmere, moonwell;
/// Grove: rootholm, elder_ring, woodland (Vesper); Stone: undermine,
/// screepass, climb_school, bastion, anvilstead. Last, Rook's four story
/// duels (1.7): meadow, stormwatch, lanternmere, the duel hall.
pub static DUELISTS: [Duelist; 59] = [
    duelist!("t_tobin", "Tobin", "Junior Duelist" / "Duelliste en herbe", One(&SCRAP), WARRENBUCK 2, 1, Easy),
    duelist!("t_hazel", "Hazel", "Beekeeper" / "Apicultrice", Mix(&SEEDLING, &SPARK), HIVEVOLT 2, 2, Easy),
    duelist!("t_ossian", "Ossian", "Birdwatcher" / "Ornithologue", One(&SCRAP), GALEHERON 2, 1, Easy),
    duelist!("t_sorrel", "Sorrel", "Forager" / "Cueilleuse", One(&SEEDLING), BOGBLOSSOM 2, 2, Normal),
    duelist!("t_bryn", "Bryn", "Trail Runner" / "Coureur des sentiers", One(&SEEDLING), BRIARHOG 1, 1, Easy),
    duelist!("t_pell", "Pell", "Angler" / "Pêcheur", One(&PUDDLE), STREAMFIN 1, 2, Easy),
    duelist!("t_skye", "Skye", "Kite Flyer" / "Cerf-voliste", One(&SPARK), STORMOX 1, 2, Normal),
    duelist!("t_morrow", "Morrow", "Shell Collector" / "Collectionneur de coquillages", One(&PUDDLE), BROOKHORN 3, 3, Normal),
    duelist!("t_nell", "Nell", "Lifeguard" / "Maître-nageuse", One(&PUDDLE), KELPWARDEN 2, 2, Normal),
    duelist!("t_tamsin", "Tamsin", "Stonecutter" / "Tailleuse de pierre", Mix(&RUBBLE, &SCRAP), KNOLLGRIM 1, 2, Easy),
    duelist!("t_cole", "Cole", "Kiln Stoker" / "Chauffeur de four", One(&KINDLING), FURNACEBACK 2, 2, Normal),
    duelist!("t_rusk", "Rusk", "Hauler" / "Porteur", Mix(&PEBBLE, &SCRAP), HAULOX 1, 1, Easy),
    duelist!("t_marigold", "Marigold", "Glass Gatherer" / "Cueilleuse de verre", One(&BELLOWS), CINDERHORN 2, 3, Normal),
    duelist!("t_lark", "Lark", "Busker" / "Musicienne de rue", Mix(&SCRAP, &MURMUR), ECHOPUP 2, 1, Easy),
    Duelist {
        heard: Some("orchard_dusk"),
        ..duelist!("t_barnaby", "Barnaby", "Old Hand" / "Vieux briscard", One(&FLOCK), MENDLARK 1, 2, Normal)
    },
    duelist!("t_tavish", "Tavish", "Glassblower" / "Souffleur de verre", One(&BELLOWS), WAXWICK 1, 2, Normal),
    Duelist {
        rule: Some(Rule::BasicsOnly),
        purse: Some(120),
        ..duelist!("t_basil", "Basil", "Purist" / "Puriste", One(&KINDLING), CINDERHORN 2, 3, Normal)
    },
    duelist!("t_carys", "Carys", "Firewalker" / "Marcheuse de braise", One(&BELLOWS), BLAZELYNX 2, 3, Normal),
    duelist!("t_dunstan", "Dunstan", "Ash Sifter" / "Tamiseur de cendres", Mix(&KINDLING, &PEBBLE), ASH_SIFTER 2, 2, Normal),
    Duelist {
        night: true,
        ..duelist!("t_vela", "Vela", "Kilnwatcher" / "Veilleuse de four", One(&FORGE), KILNBACK 1, 3, Hard)
    },
    duelist!("t_hew", "Hew", "Shepherd" / "Berger", Mix(&SPARK, &SCRAP), ECHOPUP 2, 1, Easy),
    duelist!("t_maren", "Maren", "Wind Surveyor" / "Arpenteuse des vents", One(&SPARK), FLASHFIN 1, 2, Normal),
    duelist!("t_tull", "Tull", "Chime Tuner" / "Accordeur de carillons", One(&STATIC), OHMRUNNER 1, 2, Normal),
    Duelist {
        night: true,
        ..duelist!("t_odo", "Odo", "Storm Chaser" / "Chasseur d'orages", One(&TEMPEST), STORMOX 2, 3, Hard)
    },
    duelist!("t_corra", "Corra", "Lineworker" / "Monteuse de lignes", One(&STATIC), ARCFANG 1, 3, Normal),
    duelist!("t_ves", "Ves", "Signal Keeper" / "Gardienne des signaux", Mix(&STATIC, &CURRENT), STREAMFIN 1, 3, Normal),
    Duelist {
        rule: Some(Rule::LightEnergy),
        purse: Some(120),
        ..duelist!("t_greer", "Greer", "Featherweight" / "Poids plume", One(&TEMPEST), ZIPPET 1, 3, Normal)
    },
    duelist!("t_dorrit", "Dorrit", "Deckhand" / "Matelot", One(&PUDDLE), SHELLMOTE 2, 1, Easy),
    duelist!("t_finch", "Finch", "Net Mender" / "Ramendeur", One(&CURRENT), KELPWARDEN 1, 2, Normal),
    duelist!("t_liesl", "Liesl", "Clam Digger" / "Pêcheuse à pied", Mix(&PUDDLE, &PEBBLE), SHELLMOTE 2, 2, Normal),
    duelist!("t_harrow", "Harrow", "Bell Ringer" / "Sonneur de cloche", One(&UNDERTOW), TIDEMAW 1, 3, Normal),
    Duelist {
        night: true,
        ..duelist!("t_brann", "Brann", "Night Fisher" / "Pêcheur de nuit", One(&UNDERTOW), GULFWYRM 1, 3, Hard)
    },
    Duelist {
        rule: Some(Rule::NoGear),
        purse: Some(120),
        ..duelist!("t_mirela", "Mirela", "Tidecaller" / "Appelle-marée", One(&CURRENT), STREAMFIN 1, 3, Normal)
    },
    duelist!("t_luce", "Luce", "Sleepwalker" / "Somnambule", One(&MURMUR), GLOAMOWL 1, 1, Easy),
    duelist!("t_iver", "Iver", "Poet" / "Poète", One(&REVERIE), GLIMMOX 1, 2, Normal),
    duelist!("t_clem", "Clem", "Lantern Carrier" / "Porteur de lanternes", One(&MURMUR), GLIMMOX 2, 2, Normal),
    Duelist {
        heard: Some("willow_dusk"),
        ..duelist!("t_faye", "Faye", "Dusk Dancer" / "Danseuse du crépuscule", One(&REVERIE), WISPELLE 1, 2, Normal)
    },
    duelist!("t_maud", "Maud", "Moth Catcher" / "Chasseuse de phalènes", One(&REVERIE), GLOAMOWL 1, 3, Normal),
    duelist!("t_noor", "Noor", "Dream Reader" / "Liseuse de rêves", Mix(&REVERIE, &SEEDLING), SOMNITAR 1, 3, Normal),
    Duelist {
        night: true,
        ..duelist!("t_ambrose", "Ambrose", "Night Rower" / "Rameur de nuit", One(&TRANCE), HEXARCH 1, 3, Hard)
    },
    Duelist {
        rule: Some(Rule::TwoStage2),
        purse: Some(120),
        ..duelist!("t_yara", "Yara", "Awakener" / "Éveilleuse", One(&TRANCE), WRAITHELLE 1, 3, Normal)
    },
    duelist!("t_jory", "Jory", "Treehouse Kid" / "Gamin des cabanes", One(&SEEDLING), FERNWHISK 1, 1, Easy),
    duelist!("t_fenna", "Fenna", "Vine Trimmer" / "Élagueuse", One(&HEDGEROW), THORNBUCK 1, 2, Normal),
    duelist!("t_heath", "Heath", "Maze Runner" / "Coureur du labyrinthe", Mix(&HEDGEROW, &STATIC), HONEYMANE 1, 2, Normal),
    duelist!("t_rue", "Rue", "Herbalist" / "Herboriste", One(&HEDGEROW), BOGBLOSSOM 1, 3, Normal),
    Duelist {
        night: true,
        ..duelist!("t_vesper", "Vesper", "Owl Keeper" / "Gardien des chouettes", One(&CANOPY), BOGCROWN 1, 3, Hard)
    },
    Duelist {
        rule: Some(Rule::MonoColour),
        purse: Some(120),
        ..duelist!("t_linden", "Linden", "Monochrome" / "Monochrome", One(&CANOPY), ELDERHART 1, 3, Normal)
    },
    duelist!("t_jet", "Jet", "Cart Pusher" / "Pousseur de wagonnet", One(&PEBBLE), BRAWLET 1, 1, Easy),
    duelist!("t_mica", "Mica", "Prospector" / "Prospectrice", One(&RUBBLE), GRITFANG 1, 2, Normal),
    duelist!("t_ulric", "Ulric", "Cairn Builder" / "Bâtisseur de cairns", One(&PEBBLE), SHALEPUP 1, 2, Normal),
    duelist!("t_ida", "Ida", "Climbing Instructor" / "Monitrice d'escalade", One(&RUBBLE), IRONBRAWL 1, 2, Normal),
    duelist!("t_garth", "Garth", "Rope Instructor" / "Moniteur de corde", Mix(&RUBBLE, &FLOCK), HAULOX 1, 2, Normal),
    duelist!("t_petra", "Petra", "Quarrywoman" / "Ouvrière de carrière", One(&LANDSLIDE), KNOLLGRIM 1, 3, Normal),
    Duelist {
        night: true,
        ..duelist!("t_morwen", "Morwen", "Night Shift" / "Équipe de nuit", One(&LANDSLIDE), CRAGMASTIFF 1, 3, Hard)
    },
    Duelist {
        rule: Some(Rule::NoRares),
        purse: Some(120),
        ..duelist!("t_dov", "Dov", "Bare-Knuckle" / "Poings nus", One(&RUBBLE), KNOLLGRIM 1, 3, Normal)
    },
    rook!("rook_1", &BLANK_SLATE, 2, Normal, 100),
    rook!("rook_2", &OPEN_SKY, 4, Normal, 150),
    rook!("rook_3", &HOLLOW_CROWN, 5, Hard, 200),
    rook!("rook_4", &SEVENTH_FACE, 6, Hard, 300),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::{rival, seals};
    use sixfold_engine::validate_deck;
    use sixfold_engine::Stage;

    /// Every kit and every duelist's deck is a legal 60 whose evolutions all
    /// have their line underneath.
    #[test]
    fn kit_decks_are_legal_with_whole_lines() {
        let decks = KITS
            .iter()
            .map(|r| (r.name, Kit::One(r).deck(None)))
            .chain(DUELISTS.iter().map(|t| (t.name, t.deck())));
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
        let route: Vec<&Duelist> = DUELISTS.iter().filter(|t| !t.story()).collect();
        for (i, t) in route.iter().enumerate() {
            assert!(
                t.id.starts_with("t_") && t.id.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'),
                "{}",
                t.id
            );
            assert!(route[..i].iter().all(|o| o.id != t.id && o.name != t.name));
            assert!(rival(t.id).is_none());
            assert!((1..=3).contains(&t.prizes), "{}", t.name);
            assert!(!t.class.en.is_empty() && !t.class.fr.is_empty());
        }
        // §4.4: five 1-prize, seven 2-prize and two 3-prize duelists (585),
        // and Barnaby at dusk (#116, 45); §10.1-10.2: Ember 45 + 60 + 45 + 90
        // + 120, Storm 30 + 45 + 45 + 60 + 60 + 90 + 120 (810); §10.3-10.4:
        // Tide 30 + 45 + 45 + 60 + 90 + 120, Dream 30 + 45 + 45 + 45 + 60 + 60
        // + 90 + 120 (885, Faye at dusk the 45 since #116); §10.5-10.6: Grove 30 + 45
        // + 45 + 60 + 90 + 120, Stone 30 + 45 + 45 + 45 + 45 + 60 + 90 + 120 (870).
        assert_eq!(
            route.iter().map(|t| t.purse()).sum::<u32>(),
            630 + 360 + 450 + 390 + 495 + 390 + 480
        );
        // A night duelist plays Hard; a rule duelist's own deck keeps its rule
        // when the rule can be kept by a kit.
        for t in DUELISTS.iter() {
            assert!(!t.night || t.difficulty == Difficulty::Hard, "{}", t.name);
        }
        for id in ["t_basil", "t_yara", "t_linden", "t_dov"] {
            let t = duelist(id).unwrap();
            assert!(t.rule.unwrap().allows(&t.deck()), "{id}");
        }
    }

    /// #116: two secret duelists wait only at dusk, and only once someone has
    /// told of them (`flag:heard_<secret> & time:dusk` on their maps): day duelists
    /// in the table (a night one needs `time:night`), on kits that exist.
    #[test]
    fn the_dusk_duelists_play_existing_kits_by_day_rules() {
        for id in ["t_faye", "t_barnaby"] {
            let t = duelist(id).unwrap_or_else(|| panic!("{id} is not on the roster"));
            assert!(!t.night, "{id}: dusk is not night");
            assert!(t.heard.is_some(), "{id}: a secret");
            let kits = match t.kit {
                One(r) => vec![r],
                Mix(a, b) => vec![a, b],
            };
            assert!(
                kits.iter()
                    .all(|r| KITS.iter().any(|k| std::ptr::eq(*k, *r))),
                "{id}: a kit that is not in KITS"
            );
        }
    }

    #[test]
    fn a_first_win_pays_the_purse_once_and_never_a_seal() {
        let mut p = Progress::default();
        let start = p.coins();
        let hazel = duelist("t_hazel").unwrap();
        assert_eq!(hazel.purse(), 45);
        assert_eq!(record(hazel, false, &mut p), 0);
        assert!(p.check("lost:t_hazel") && !p.check("beat:t_hazel"));
        assert_eq!(record(hazel, true, &mut p), 45);
        assert!(p.check("won:t_hazel") && p.check("beat:t_hazel"));
        assert!(p.has_flag("beat_t_hazel") && p.check("count:beat_t_:1"));
        assert_eq!(record(hazel, true, &mut p), 0, "once per duelist");
        assert_eq!((p.coins(), seals(&p)), (start + 45, 0));
    }

    /// §8.2: Rook's four decks, legal, Neutral Hexons only, each growing on
    /// the one before (the last its own list), with their prizes and AI.
    #[test]
    fn rooks_decks_are_neutral_and_grow() {
        let rook: Vec<&Duelist> = DUELISTS.iter().filter(|t| t.story()).collect();
        let ids: Vec<&str> = rook.iter().map(|t| t.id).collect();
        assert_eq!(ids, ["rook_1", "rook_2", "rook_3", "rook_4"]);
        let shape: Vec<(u8, Difficulty, u32)> = rook
            .iter()
            .map(|t| (t.prizes, t.difficulty, t.purse()))
            .collect();
        use Difficulty::{Hard, Normal};
        assert_eq!(
            shape,
            [
                (2, Normal, 100),
                (4, Normal, 150),
                (5, Hard, 200),
                (6, Hard, 300)
            ]
        );
        for t in &rook {
            let d = t.deck();
            assert_eq!(validate_deck(&d), Ok(()), "{}", t.id);
            assert!(
                d.iter()
                    .filter_map(|&id| card(id).creature())
                    .all(|c| c.color == Color::Colorless),
                "{}: Neutral Hexons only",
                t.id
            );
            assert!(t.name == "Rook" && t.rule.is_none() && !t.night);
            assert_eq!(t.class.fr, "Le Sans-Couleur");
        }
        let n = |t: &Duelist, id| t.deck().iter().filter(|&&c| c == id).count();
        // Blank Slate: 19 cards and 41 energy; Open Sky adds 12, Hollow Crown 7.
        let energy = |t: &Duelist| n(t, FIRE_ENERGY) + n(t, WATER_ENERGY);
        assert_eq!(
            rook.iter().map(|t| energy(t)).collect::<Vec<_>>(),
            [41, 29, 22, 20]
        );
        assert_eq!((n(rook[1], GALEHERON), n(rook[1], TWOFOLD_ENERGY)), (3, 2));
        assert_eq!((n(rook[2], WARRENKING), n(rook[2], SNARE_HOOK)), (2, 1));
        // Seventh Face: two Pocketmouse, no Scampit, ten of each energy.
        let last = rook[3];
        assert_eq!((n(last, POCKETMOUSE), n(last, SCAMPIT)), (2, 0));
        assert_eq!(
            (
                n(last, FIRE_ENERGY),
                n(last, WATER_ENERGY),
                n(last, TWOFOLD_ENERGY)
            ),
            (10, 10, 4)
        );
    }

    /// A story duel won pays its purse once and marks it beaten; lost, it
    /// pays nothing here (the consolation comes on top, as for any loss). Won
    /// again (the hall rematch), it pays 10 per prize.
    #[test]
    fn a_rook_win_pays_more_than_a_loss_and_a_rematch_pays_by_prizes() {
        let mut p = Progress::default();
        let r1 = duelist("rook_1").unwrap();
        assert_eq!(record(r1, false, &mut p), 0);
        assert!(p.check("lost:rook_1") && !p.check("beat:rook_1"));
        let r4 = duelist("rook_4").unwrap();
        assert_eq!(record(r4, true, &mut p), 300);
        assert!(p.check("won:rook_4") && p.has_flag("beat_rook_4"));
        assert_eq!(record(r4, true, &mut p), 60, "the rematch");
        assert_eq!(record(r4, false, &mut p), 0);
        assert_eq!(
            (p.coins() - Progress::default().coins(), seals(&p)),
            (360, 0)
        );
        assert!(!p.check("count:beat_t_:1"), "never a route duelist");
    }

    #[test]
    fn challenge_rules_read_decks() {
        let kindling = Kit::One(&KINDLING).deck(None);
        let bellows = Kit::One(&BELLOWS).deck(None);
        let forge = Kit::One(&FORGE).deck(None);
        let scrap = Kit::One(&SCRAP).deck(None);
        let mono_fire: Vec<CardId> = [CINDREL; 4].into_iter().chain([FIRE_ENERGY; 56]).collect();
        let fire_and_scampit: Vec<CardId> = mono_fire.iter().copied().chain([SCAMPIT]).collect();
        assert!(Rule::BasicsOnly.allows(&kindling) && !Rule::BasicsOnly.allows(&bellows));
        assert!(Rule::NoGear.allows(&mono_fire) && !Rule::NoGear.allows(&kindling));
        assert!(Rule::MonoColour.allows(&mono_fire) && Rule::MonoColour.allows(&fire_and_scampit));
        assert!(
            !Rule::MonoColour.allows(&scrap),
            "Scrap pads with Fire and Water"
        );
        assert!(
            !Rule::LightEnergy.allows(&kindling),
            "a kit pads to 60 with energy"
        );
        let light: Vec<CardId> = [CINDREL; 4].into_iter().chain([FIRE_ENERGY; 16]).collect();
        assert!(Rule::LightEnergy.allows(&light));
        assert!(Rule::NoRares.allows(&bellows) && !Rule::NoRares.allows(&forge));
        assert!(!Rule::TwoStage2.allows(&bellows) && Rule::TwoStage2.allows(&forge));
        assert_eq!(duelist("t_greer").unwrap().rule, Some(Rule::LightEnergy));
        assert!(!Rule::NoRares.text().fr.is_empty());
    }

    #[test]
    fn night_and_set_purses() {
        let mut t = Duelist {
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
