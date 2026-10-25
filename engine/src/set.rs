//! The original card set: lines of creatures in every colour, trainers and
//! energy, played through the prebuilt decks in [`crate::decks`].
//!
//! `CardId(n)` is the index into [`CARDS`]; keep the constants below in the
//! same order. All content is original: no names, text or stats from any
//! published card game.

use crate::card::{
    Attack, CardDef, CardId, CardKind,
    Color::*,
    Creature, EnergyCard, Power, PowerKind, Rarity,
    Rarity::{Common, Rare, Uncommon},
    Stage, Trainer,
};
use crate::condition::{Placed, Shield, Status};
use crate::effect::{Effect::*, Find};

// Ember Rush creatures
pub const CINDREL: CardId = CardId(0);
pub const FLARROW: CardId = CardId(1);
pub const PYREQUILL: CardId = CardId(2);
pub const KNOLLOP: CardId = CardId(3);
pub const KNOLLGRIM: CardId = CardId(4);
pub const TUSKLET: CardId = CardId(5);
pub const GRITFANG: CardId = CardId(6);
// Tide Mind creatures
pub const PUDDOCK: CardId = CardId(7);
pub const BROOKHORN: CardId = CardId(8);
pub const TIDEMAW: CardId = CardId(9);
pub const SHELLMOTE: CardId = CardId(10);
pub const WISPUN: CardId = CardId(11);
pub const WISPELLE: CardId = CardId(12);
pub const GLIMMOX: CardId = CardId(13);
// Shared
pub const SCAMPIT: CardId = CardId(14);
// Trainers
pub const TRAIL_COMPASS: CardId = CardId(15);
pub const LUCKY_SATCHEL: CardId = CardId(16);
pub const MOSS_POULTICE: CardId = CardId(17);
pub const CLEARWATER_VIAL: CardId = CardId(18);
pub const RELAY_BATON: CardId = CardId(19);
pub const SNARE_HOOK: CardId = CardId(20);
pub const DRAIN_BRUSH: CardId = CardId(21);
pub const SPIKE_CHARM: CardId = CardId(22);
pub const BARK_SHIELD: CardId = CardId(23);
pub const RECALL_HORN: CardId = CardId(24);
pub const BLOOM_SEED: CardId = CardId(25);
pub const UNWIND_SPRAY: CardId = CardId(26);
// Energy
pub const GRASS_ENERGY: CardId = CardId(27);
pub const FIRE_ENERGY: CardId = CardId(28);
pub const WATER_ENERGY: CardId = CardId(29);
pub const LIGHTNING_ENERGY: CardId = CardId(30);
pub const FIGHTING_ENERGY: CardId = CardId(31);
pub const PSYCHIC_ENERGY: CardId = CardId(32);
pub const TWOFOLD_ENERGY: CardId = CardId(33);
// ----- 0.4.0 expansion (append only: ids are save positions) -----
pub const SPRIGGET: CardId = CardId(34);
pub const THORNBUCK: CardId = CardId(35);
pub const ELDERHART: CardId = CardId(36);
pub const MIREBUD: CardId = CardId(37);
pub const BOGBLOSSOM: CardId = CardId(38);
pub const BOGCROWN: CardId = CardId(39);
pub const POLLENPUP: CardId = CardId(40);
pub const HONEYMANE: CardId = CardId(41);
pub const BRIARHOG: CardId = CardId(42);
pub const FERNWHISK: CardId = CardId(43);
pub const ASHPAW: CardId = CardId(44);
pub const BLAZELYNX: CardId = CardId(45);
pub const SOLARLYNX: CardId = CardId(46);
pub const KILNBACK: CardId = CardId(47);
pub const FURNACEBACK: CardId = CardId(48);
pub const WAXWICK: CardId = CardId(49);
pub const CINDERHORN: CardId = CardId(50);
pub const RILLET: CardId = CardId(51);
pub const STREAMFIN: CardId = CardId(52);
pub const GULFWYRM: CardId = CardId(53);
pub const KELPUP: CardId = CardId(54);
pub const KELPWARDEN: CardId = CardId(55);
pub const FROSTEEL: CardId = CardId(56);
pub const SPARKIT: CardId = CardId(57);
pub const ARCFANG: CardId = CardId(58);
pub const FULGUREX: CardId = CardId(59);
pub const BUZZLET: CardId = CardId(60);
pub const HIVEVOLT: CardId = CardId(61);
pub const OHMLET: CardId = CardId(62);
pub const OHMRUNNER: CardId = CardId(63);
pub const STORMOX: CardId = CardId(64);
pub const ZIPPET: CardId = CardId(65);
pub const FLASHFIN: CardId = CardId(66);
pub const KNOLLOSSUS: CardId = CardId(67);
pub const BRAWLET: CardId = CardId(68);
pub const IRONBRAWL: CardId = CardId(69);
pub const SHALEPUP: CardId = CardId(70);
pub const SHALEHOUND: CardId = CardId(71);
pub const CRAGMASTIFF: CardId = CardId(72);
pub const BOULDERGUT: CardId = CardId(73);
pub const WRAITHELLE: CardId = CardId(74);
pub const DOZEL: CardId = CardId(75);
pub const SOMNITAR: CardId = CardId(76);
pub const HEXLET: CardId = CardId(77);
pub const HEXMAGE: CardId = CardId(78);
pub const HEXARCH: CardId = CardId(79);
pub const GLOAMOWL: CardId = CardId(80);
pub const BURROWBIT: CardId = CardId(81);
pub const WARRENBUCK: CardId = CardId(82);
pub const WARRENKING: CardId = CardId(83);
pub const PECKLET: CardId = CardId(84);
pub const GALEHERON: CardId = CardId(85);
pub const ECHOPUP: CardId = CardId(86);
pub const HAULOX: CardId = CardId(87);
pub const POCKETMOUSE: CardId = CardId(88);
pub const MENDLARK: CardId = CardId(89);
pub const DOWSING_ROD: CardId = CardId(90);
pub const FIELD_GUIDE: CardId = CardId(91);
pub const ASH_SIFTER: CardId = CardId(92);
pub const SCRAP_LEDGER: CardId = CardId(93);
pub const CLEAN_SLATE: CardId = CardId(94);
pub const GROVE_BALM: CardId = CardId(95);
pub const CHARGE_COIL: CardId = CardId(96);
pub const JOLT_WIRE: CardId = CardId(97);
pub const STATIC_NET: CardId = CardId(98);
pub const SMOKE_PELLET: CardId = CardId(99);
pub const VENOM_DART: CardId = CardId(100);
pub const WIDE_SATCHEL: CardId = CardId(101);
pub const HEARTY_BROTH: CardId = CardId(102);
// ----- 0.9.0 promos: campaign rewards, never in packs (append only) -----
pub const THISTLEWICK: CardId = CardId(103);
pub const BRINEMOTH: CardId = CardId(104);
pub const KILNFOX: CardId = CardId(105);
pub const SQUALLKITE: CardId = CardId(106);
pub const SLATEBROW: CardId = CardId(107);
pub const MOONMOTH: CardId = CardId(108);
pub const HEXCREST: CardId = CardId(109);

/// The promo cards, in campaign order: one for the first win against each
/// rival, the last for the champion. They never come from packs.
pub const PROMOS: [CardId; 7] = [
    THISTLEWICK,
    BRINEMOTH,
    KILNFOX,
    SQUALLKITE,
    SLATEBROW,
    MOONMOTH,
    HEXCREST,
];

/// A promo card: earned in the campaign, never found in a pack.
pub fn is_promo(id: CardId) -> bool {
    PROMOS.contains(&id)
}

/// A Holo Rare: a Rare Hexon (all 12 are Awaken 2s) found in a pack's
/// upgraded Rare slot (see [`crate::booster`]). Promos are not.
pub fn is_holo(id: CardId) -> bool {
    let c = &CARDS[id.0 as usize];
    c.rarity == Rare && matches!(c.kind, CardKind::Creature(_)) && !is_promo(id)
}

/// Every card id in the set, in registry order (for card lists in the UI).
pub fn ids() -> impl Iterator<Item = CardId> {
    (0..CARDS.len() as u16).map(CardId)
}

/// The basic energy card of a colour (`None` for Colorless).
pub fn basic_energy(color: crate::Color) -> Option<CardId> {
    ids().find(|&id| matches!(CARDS[id.0 as usize].kind, CardKind::Energy(EnergyCard::Basic(c)) if c == color))
}

const fn creature(name: &'static str, rarity: Rarity, c: Creature) -> CardDef {
    CardDef {
        name,
        rarity,
        kind: CardKind::Creature(c),
    }
}

const fn trainer(
    name: &'static str,
    rarity: Rarity,
    text: &'static str,
    effects: &'static [crate::Effect],
) -> CardDef {
    CardDef {
        name,
        rarity,
        kind: CardKind::Trainer(Trainer { text, effects }),
    }
}

const fn atk(
    name: &'static str,
    text: &'static str,
    cost: &'static [crate::Color],
    damage: u16,
    effects: &'static [crate::Effect],
) -> Attack {
    Attack {
        name,
        text,
        cost,
        damage,
        effects,
    }
}

const fn energy(name: &'static str, rarity: Rarity, e: EnergyCard) -> CardDef {
    CardDef {
        name,
        rarity,
        kind: CardKind::Energy(e),
    }
}

#[rustfmt::skip]
pub static CARDS: &[CardDef] = &[
    // ----- Ember Rush: Fire line (Basic -> Stage 1 -> Stage 2) -----
    creature("Cindrel", Common, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 50, color: Fire,
        weakness: Some(Water), resistance: None, retreat_cost: 1,
        attacks: &[
            Attack { name: "Ember Flick", text: "Flip a coin. If heads, this attack does 10 more damage.",
                     cost: &[Fire], damage: 10, effects: &[AddPerHeads { coins: 1, per: 10 }] },
            Attack { name: "Cinder Dash", text: "", cost: &[Fire, Colorless], damage: 20, effects: &[] },
        ],
        power: None,
    }),
    creature("Flarrow", Uncommon, Creature {
        stage: Stage::Stage1, evolves_from: Some("Cindrel"), hp: 70, color: Fire,
        weakness: Some(Water), resistance: None, retreat_cost: 1,
        attacks: &[
            Attack { name: "Heat Haze",
                     text: "During your opponent's next turn, the defending Hexon must flip heads to attack.",
                     cost: &[Fire], damage: 10,
                     effects: &[Place { placed: Placed::Accuracy, coin: false }] },
            Attack { name: "Flare Dive", text: "This Hexon does 10 damage to itself.",
                     cost: &[Fire, Colorless], damage: 40, effects: &[Recoil(10)] },
        ],
        power: None,
    }),
    creature("Pyrequill", Rare, Creature {
        stage: Stage::Stage2, evolves_from: Some("Flarrow"), hp: 100, color: Fire,
        weakness: Some(Water), resistance: None, retreat_cost: 2,
        attacks: &[
            Attack { name: "Spark Rain", text: "Does 10 damage to each of your opponent's Benched Hexons.",
                     cost: &[Fire, Fire], damage: 30,
                     effects: &[BenchDamage { amount: 10, own: false, opponent: true }] },
            Attack { name: "Inferno Plume", text: "Discard 2 Fire energy cards attached to this Hexon.",
                     cost: &[Fire, Fire, Fire, Colorless], damage: 90,
                     effects: &[DiscardEnergy { color: Fire, count: 2 }] },
        ],
        power: Some(Power {
            name: "Kindling Heart",
            text: "Once during your turn, a Fire energy you attach to one of your Fire Hexons doesn't use up your energy attachment for the turn.",
            kind: PowerKind::FreeAttach(Fire),
        }),
    }),
    // ----- Ember Rush: Fighting -----
    creature("Knollop", Common, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 60, color: Fighting,
        weakness: Some(Grass), resistance: None, retreat_cost: 2,
        attacks: &[
            Attack { name: "Clod Toss", text: "", cost: &[Fighting], damage: 20, effects: &[] },
            Attack { name: "Rumble", text: "Does 10 damage to each Benched Hexon, yours and your opponent's.",
                     cost: &[Fighting, Fighting], damage: 30,
                     effects: &[BenchDamage { amount: 10, own: true, opponent: true }] },
        ],
        power: None,
    }),
    creature("Knollgrim", Uncommon, Creature {
        stage: Stage::Stage1, evolves_from: Some("Knollop"), hp: 90, color: Fighting,
        weakness: Some(Grass), resistance: None, retreat_cost: 3,
        attacks: &[
            Attack { name: "Granite Guard",
                     text: "During your opponent's next turn, this Hexon takes 20 less damage from attacks.",
                     cost: &[Fighting, Colorless], damage: 20,
                     effects: &[Protect { shield: Shield::Reduce(20), coin: false }] },
            Attack { name: "Berserk Slam", text: "This Hexon is now Confused.",
                     cost: &[Fighting, Fighting, Colorless], damage: 60,
                     effects: &[InflictSelf(Status::Confused)] },
        ],
        power: Some(Power {
            name: "Jagged Crust",
            text: "Whenever this Hexon takes damage that isn't from a Power, your opponent's Active Hexon takes 10 damage.",
            kind: PowerKind::Retaliate,
        }),
    }),
    creature("Tusklet", Common, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 50, color: Fighting,
        weakness: Some(Grass), resistance: None, retreat_cost: 1,
        attacks: &[
            Attack { name: "Gore", text: "Flip 2 coins. This attack does 20 damage for each heads.",
                     cost: &[Fighting], damage: 0, effects: &[AddPerHeads { coins: 2, per: 20 }] },
            Attack { name: "Wild Charge", text: "This Hexon does 10 damage to itself.",
                     cost: &[Fighting, Colorless], damage: 30, effects: &[Recoil(10)] },
        ],
        power: None,
    }),
    creature("Gritfang", Uncommon, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 70, color: Fighting,
        weakness: Some(Grass), resistance: None, retreat_cost: 2,
        attacks: &[
            Attack { name: "Psych Up", text: "During your next turn, this Hexon's attack does double damage.",
                     cost: &[Colorless], damage: 10, effects: &[DoubleNextAttack] },
            Attack { name: "Crushing Jaw",
                     text: "The defending Hexon can't retreat during your opponent's next turn.",
                     cost: &[Fighting, Colorless], damage: 30,
                     effects: &[Place { placed: Placed::CantRetreat, coin: false }] },
        ],
        power: None,
    }),
    // ----- Tide Mind: Water line (Basic -> Stage 1 -> Stage 2) -----
    creature("Puddock", Common, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 50, color: Water,
        weakness: Some(Lightning), resistance: None, retreat_cost: 1,
        attacks: &[
            Attack { name: "Bubble Spit", text: "Flip a coin. If heads, the defending Hexon is now Paralyzed.",
                     cost: &[Water], damage: 10,
                     effects: &[Inflict { status: Status::Paralyzed, coin: true }] },
            Attack { name: "Silt Cloud", text: "Your opponent can't play trainer cards during their next turn.",
                     cost: &[Water, Colorless], damage: 10, effects: &[TrainerLock] },
        ],
        power: None,
    }),
    creature("Brookhorn", Uncommon, Creature {
        stage: Stage::Stage1, evolves_from: Some("Puddock"), hp: 70, color: Water,
        weakness: Some(Lightning), resistance: None, retreat_cost: 1,
        attacks: &[
            Attack { name: "Whirl",
                     text: "Your opponent switches the defending Hexon with 1 of their Benched Hexons.",
                     cost: &[Water, Colorless], damage: 20,
                     effects: &[SwitchDefender { chooser_is_attacker: false }] },
            Attack { name: "Wave Lash", text: "", cost: &[Water, Water, Colorless], damage: 50, effects: &[] },
        ],
        power: None,
    }),
    creature("Tidemaw", Rare, Creature {
        stage: Stage::Stage2, evolves_from: Some("Brookhorn"), hp: 100, color: Water,
        weakness: Some(Lightning), resistance: Some(Fire), retreat_cost: 3,
        attacks: &[
            Attack { name: "Drench", text: "Discard 1 energy card attached to the defending Hexon.",
                     cost: &[Water, Water, Colorless], damage: 20, effects: &[DiscardDefenderEnergy] },
            Attack { name: "Great Surge",
                     text: "This attack can be used only once while this Hexon stays in play.",
                     cost: &[Water, Water, Water, Colorless], damage: 100, effects: &[OnceWhileInPlay] },
        ],
        power: Some(Power {
            name: "Mending Rain",
            text: "Once during your turn, you may remove 10 damage from your Active Hexon.",
            kind: PowerKind::Activated { effects: &[Heal(10)], once_per_turn: true, settled_only: false },
        }),
    }),
    creature("Shellmote", Uncommon, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 60, color: Water,
        weakness: Some(Lightning), resistance: None, retreat_cost: 2,
        attacks: &[
            Attack { name: "Shell Up",
                     text: "Flip a coin. If heads, prevent all damage done to this Hexon by attacks during your opponent's next turn.",
                     cost: &[Water], damage: 0,
                     effects: &[Protect { shield: Shield::PreventDamage, coin: true }] },
            Attack { name: "Soak Through", text: "The defending Hexon's weakness is now Water.",
                     cost: &[Water, Water], damage: 20, effects: &[DefenderWeakness(Water)] },
        ],
        power: Some(Power {
            name: "Float Shell",
            text: "While this Hexon is on your Bench, your Active Hexon's retreat cost is 1 less.",
            kind: PowerKind::RetreatReducer,
        }),
    }),
    // ----- Tide Mind: Psychic -----
    creature("Wispun", Common, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 40, color: Psychic,
        weakness: Some(Fighting), resistance: None, retreat_cost: 1,
        attacks: &[
            Attack { name: "Lull", text: "The defending Hexon is now Asleep.",
                     cost: &[Psychic], damage: 0,
                     effects: &[Inflict { status: Status::Asleep, coin: false }] },
            Attack { name: "Hush Chant",
                     text: "Choose 1 of the defending Hexon's attacks. It can't use that attack during your opponent's next turn.",
                     cost: &[Psychic, Colorless], damage: 10, effects: &[DisableAttack] },
        ],
        power: None,
    }),
    creature("Wispelle", Uncommon, Creature {
        stage: Stage::Stage1, evolves_from: Some("Wispun"), hp: 70, color: Psychic,
        weakness: Some(Fighting), resistance: None, retreat_cost: 1,
        attacks: &[
            Attack { name: "Haunt Fog", text: "The defending Hexon is now Poisoned.",
                     cost: &[Psychic], damage: 10,
                     effects: &[Inflict { status: Status::Poisoned, coin: false }] },
            Attack { name: "Mind Shatter", text: "Flip a coin. If heads, the defending Hexon is now Double-poisoned.",
                     cost: &[Psychic, Psychic, Colorless], damage: 30,
                     effects: &[Inflict { status: Status::DoublePoisoned, coin: true }] },
        ],
        power: None,
    }),
    creature("Glimmox", Uncommon, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 50, color: Psychic,
        weakness: Some(Fighting), resistance: None, retreat_cost: 1,
        attacks: &[
            Attack { name: "Daze Beam", text: "Flip a coin. If heads, the defending Hexon is now Confused.",
                     cost: &[Psychic, Psychic], damage: 20,
                     effects: &[Inflict { status: Status::Confused, coin: true }] },
            Attack { name: "Mirror Gaze", text: "Choose 1 of the defending Hexon's attacks and use it as this attack.",
                     cost: &[Psychic, Colorless, Colorless], damage: 0, effects: &[CopyAttack] },
        ],
        power: Some(Power {
            name: "Mind Needle",
            text: "Once during your turn, if this Hexon was in play when your turn began, you may do 10 damage to 1 of your opponent's Hexons.",
            kind: PowerKind::Activated { effects: &[Snipe(10)], once_per_turn: true, settled_only: true },
        }),
    }),
    // ----- Shared -----
    creature("Scampit", Common, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 40, color: Colorless,
        weakness: None, resistance: Some(Psychic), retreat_cost: 0,
        attacks: &[Attack { name: "Nibble", text: "", cost: &[Colorless], damage: 10, effects: &[] }],
        power: Some(Power {
            name: "Forage",
            text: "Once during your turn, if this Hexon was in play when your turn began, you may draw a card.",
            kind: PowerKind::Activated { effects: &[Draw(1)], once_per_turn: true, settled_only: true },
        }),
    }),
    // ----- Trainers -----
    trainer("Trail Compass", Uncommon,
            "Search your deck for a Basic Hexon card, put it into your hand, then shuffle your deck.",
            &[Search(Find::Basic)]),
    trainer("Lucky Satchel", Uncommon, "Draw 2 cards.", &[Draw(2)]),
    trainer("Moss Poultice", Common, "Remove up to 30 damage from your Active Hexon.", &[Heal(30)]),
    trainer("Clearwater Vial", Common, "Your Active Hexon recovers from all conditions.", &[Cure { both: false }]),
    trainer("Relay Baton", Common, "Switch your Active Hexon with 1 of your Benched Hexons.", &[SwitchSelf]),
    trainer("Snare Hook", Rare,
            "Choose 1 of your opponent's Benched Hexons and switch it with their Active Hexon.",
            &[SwitchDefender { chooser_is_attacker: true }]),
    trainer("Drain Brush", Rare, "Discard 1 energy card attached to your opponent's Active Hexon.",
            &[DiscardDefenderEnergy]),
    trainer("Spike Charm", Uncommon, "Attach to your Active Hexon. Its attacks do 10 more damage this turn.",
            &[AttachPlus10]),
    trainer("Bark Shield", Uncommon,
            "Attach to 1 of your Hexons. It takes 20 less damage from attacks until the end of your opponent's next turn.",
            &[AttachMinus20]),
    trainer("Recall Horn", Uncommon, "Put a Hexon card from your discard pile into your hand.",
            &[Recover(Find::Creature)]),
    trainer("Bloom Seed", Rare,
            "Put an Awaken 2 Hexon card from your hand onto the matching Basic Hexon in play. The Basic must have been in play since the start of your turn.",
            &[StageSkip]),
    trainer("Unwind Spray", Rare,
            "Your opponent's awakened Active Hexon returns to its previous stage. Put the top card into your opponent's hand.",
            &[Devolve]),
    // ----- Energy -----
    energy("Grass Energy", Rarity::Energy, EnergyCard::Basic(Grass)),
    energy("Fire Energy", Rarity::Energy, EnergyCard::Basic(Fire)),
    energy("Water Energy", Rarity::Energy, EnergyCard::Basic(Water)),
    energy("Lightning Energy", Rarity::Energy, EnergyCard::Basic(Lightning)),
    energy("Fighting Energy", Rarity::Energy, EnergyCard::Basic(Fighting)),
    energy("Psychic Energy", Rarity::Energy, EnergyCard::Basic(Psychic)),
    energy("Twofold Energy", Rare, EnergyCard::DoubleColorless),
    // ===== 0.4.0 expansion: appended only, ids are save positions =====
    // ----- Grass: healing, poison, energy from the deck -----
    creature("Sprigget", Common, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 50, color: Grass,
        weakness: Some(Fire), resistance: None, retreat_cost: 1,
        attacks: &[
            atk("Leaf Nick", "", &[Grass], 10, &[]),
            atk("Root Call", "Search your deck for a Grass energy card and attach it to this Hexon, then shuffle your deck.",
                &[Grass], 0, &[AttachFromDeck(Grass)]),
        ],
        power: None,
    }),
    creature("Thornbuck", Uncommon, Creature {
        stage: Stage::Stage1, evolves_from: Some("Sprigget"), hp: 80, color: Grass,
        weakness: Some(Fire), resistance: None, retreat_cost: 2,
        attacks: &[
            atk("Barb Volley", "Flip a coin. If heads, the defending Hexon is now Poisoned.",
                &[Grass, Colorless], 20, &[Inflict { status: Status::Poisoned, coin: true }]),
            atk("Antler Rake", "", &[Grass, Grass, Colorless], 50, &[]),
        ],
        power: None,
    }),
    creature("Elderhart", Rare, Creature {
        stage: Stage::Stage2, evolves_from: Some("Thornbuck"), hp: 100, color: Grass,
        weakness: Some(Fire), resistance: None, retreat_cost: 3,
        attacks: &[
            atk("Grove Crush", "Remove 20 damage from this Hexon.",
                &[Grass, Grass, Grass, Colorless], 60, &[Heal(20)]),
        ],
        power: Some(Power {
            name: "Verdant Pulse",
            text: "Once during your turn, you may remove 10 damage from each of your Hexons.",
            kind: PowerKind::Activated { effects: &[HealAll(10)], once_per_turn: true, settled_only: false },
        }),
    }),
    creature("Mirebud", Common, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 40, color: Grass,
        weakness: Some(Fire), resistance: None, retreat_cost: 1,
        attacks: &[
            atk("Mire Spore", "The defending Hexon is now Poisoned.",
                &[Grass], 0, &[Inflict { status: Status::Poisoned, coin: false }]),
            atk("Sap Spit", "", &[Grass, Colorless], 20, &[]),
        ],
        power: None,
    }),
    creature("Bogblossom", Uncommon, Creature {
        stage: Stage::Stage1, evolves_from: Some("Mirebud"), hp: 70, color: Grass,
        weakness: Some(Fire), resistance: None, retreat_cost: 1,
        attacks: &[
            atk("Numbing Pollen", "Flip a coin. If heads, the defending Hexon is now Paralyzed.",
                &[Grass, Colorless], 10, &[Inflict { status: Status::Paralyzed, coin: true }]),
            atk("Rot Cloud", "The defending Hexon is now Poisoned.",
                &[Grass, Grass], 20, &[Inflict { status: Status::Poisoned, coin: false }]),
        ],
        power: None,
    }),
    creature("Bogcrown", Rare, Creature {
        stage: Stage::Stage2, evolves_from: Some("Bogblossom"), hp: 100, color: Grass,
        weakness: Some(Fire), resistance: None, retreat_cost: 2,
        attacks: &[
            atk("Choking Bloom", "The defending Hexon is now Double-poisoned.",
                &[Grass, Grass], 20, &[Inflict { status: Status::DoublePoisoned, coin: false }]),
            atk("Drain Root", "Remove 30 damage from this Hexon.",
                &[Grass, Grass, Colorless], 40, &[Heal(30)]),
        ],
        power: None,
    }),
    creature("Pollenpup", Common, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 50, color: Grass,
        weakness: Some(Fire), resistance: None, retreat_cost: 1,
        attacks: &[
            atk("Snuffle", "", &[Colorless], 10, &[]),
            atk("Petal Toss", "", &[Grass, Colorless], 20, &[]),
        ],
        power: None,
    }),
    creature("Honeymane", Uncommon, Creature {
        stage: Stage::Stage1, evolves_from: Some("Pollenpup"), hp: 80, color: Grass,
        weakness: Some(Fire), resistance: None, retreat_cost: 2,
        attacks: &[
            atk("Nectar Share", "Remove 20 damage from each of your Hexons.",
                &[Grass], 0, &[HealAll(20)]),
            atk("Mane Lash", "", &[Grass, Grass, Colorless], 50, &[]),
        ],
        power: None,
    }),
    creature("Briarhog", Uncommon, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 70, color: Grass,
        weakness: Some(Fire), resistance: None, retreat_cost: 2,
        attacks: &[
            atk("Curl Roll", "", &[Grass], 10, &[]),
            atk("Thorn Barrage", "", &[Grass, Grass, Colorless], 40, &[]),
        ],
        power: Some(Power {
            name: "Prickly Hide",
            text: "Whenever this Hexon takes damage that isn't from a Power, your opponent's Active Hexon takes 10 damage.",
            kind: PowerKind::Retaliate,
        }),
    }),
    creature("Fernwhisk", Common, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 40, color: Grass,
        weakness: Some(Fire), resistance: None, retreat_cost: 0,
        attacks: &[
            atk("Fan Breeze", "Switch this Hexon with 1 of your Benched Hexons.",
                &[Grass], 10, &[SwitchSelf]),
            atk("Seed Shot", "", &[Grass, Colorless], 20, &[]),
        ],
        power: None,
    }),
    // ----- Fire: burst damage, energy discard -----
    creature("Ashpaw", Common, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 50, color: Fire,
        weakness: Some(Water), resistance: None, retreat_cost: 1,
        attacks: &[
            atk("Hot Swipe", "", &[Fire], 10, &[]),
            atk("Flame Pounce", "Discard 1 Fire energy card attached to this Hexon.",
                &[Fire, Colorless], 30, &[DiscardEnergy { color: Fire, count: 1 }]),
        ],
        power: None,
    }),
    creature("Blazelynx", Uncommon, Creature {
        stage: Stage::Stage1, evolves_from: Some("Ashpaw"), hp: 80, color: Fire,
        weakness: Some(Water), resistance: None, retreat_cost: 1,
        attacks: &[
            atk("Ember Claw", "", &[Fire, Colorless], 30, &[]),
            atk("Blaze Rush", "Discard 1 Fire energy card attached to this Hexon.",
                &[Fire, Fire, Colorless], 60, &[DiscardEnergy { color: Fire, count: 1 }]),
        ],
        power: None,
    }),
    creature("Solarlynx", Rare, Creature {
        stage: Stage::Stage2, evolves_from: Some("Blazelynx"), hp: 110, color: Fire,
        weakness: Some(Water), resistance: None, retreat_cost: 2,
        attacks: &[
            atk("Searing Mane", "", &[Fire, Fire], 40, &[]),
            atk("Supernova Leap", "Discard all energy attached to this Hexon.",
                &[Fire, Fire, Fire, Colorless], 120, &[DiscardAllEnergy]),
        ],
        power: None,
    }),
    creature("Kilnback", Common, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 60, color: Fire,
        weakness: Some(Water), resistance: None, retreat_cost: 2,
        attacks: &[
            atk("Coal Toss", "", &[Fire], 10, &[]),
            atk("Kiln Slam", "", &[Fire, Fire, Colorless], 40, &[]),
        ],
        power: None,
    }),
    creature("Furnaceback", Uncommon, Creature {
        stage: Stage::Stage1, evolves_from: Some("Kilnback"), hp: 90, color: Fire,
        weakness: Some(Water), resistance: None, retreat_cost: 3,
        attacks: &[
            atk("Stoke", "Search your deck for a Fire energy card and attach it to this Hexon, then shuffle your deck.",
                &[Fire], 10, &[AttachFromDeck(Fire)]),
            atk("Magma Wave", "Discard 2 Fire energy cards attached to this Hexon.",
                &[Fire, Fire, Fire, Colorless], 80, &[DiscardEnergy { color: Fire, count: 2 }]),
        ],
        power: None,
    }),
    creature("Waxwick", Common, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 40, color: Fire,
        weakness: Some(Water), resistance: None, retreat_cost: 1,
        attacks: &[
            atk("Wick Flare", "Flip a coin. If heads, this attack does 20 more damage.",
                &[Fire], 10, &[AddPerHeads { coins: 1, per: 20 }]),
        ],
        power: None,
    }),
    creature("Cinderhorn", Uncommon, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 70, color: Fire,
        weakness: Some(Water), resistance: None, retreat_cost: 2,
        attacks: &[
            atk("Horn Heat", "", &[Fire, Colorless], 30, &[]),
            atk("Burning Charge", "This Hexon does 20 damage to itself.",
                &[Fire, Fire, Colorless], 60, &[Recoil(20)]),
        ],
        power: None,
    }),
    // ----- Water: control, energy removal, switching -----
    creature("Rillet", Common, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 50, color: Water,
        weakness: Some(Lightning), resistance: None, retreat_cost: 1,
        attacks: &[
            atk("Spray Flick", "", &[Water], 10, &[]),
            atk("Silt Swipe", "Discard 1 energy card attached to the defending Hexon.",
                &[Water, Water], 10, &[DiscardDefenderEnergy]),
        ],
        power: None,
    }),
    creature("Streamfin", Uncommon, Creature {
        stage: Stage::Stage1, evolves_from: Some("Rillet"), hp: 80, color: Water,
        weakness: Some(Lightning), resistance: None, retreat_cost: 1,
        attacks: &[
            atk("Fin Slice", "", &[Water, Colorless], 30, &[]),
            atk("Tidal Pull", "Switch the defending Hexon with 1 of your opponent's Benched Hexons, chosen by you.",
                &[Water, Water, Colorless], 40, &[SwitchDefender { chooser_is_attacker: true }]),
        ],
        power: None,
    }),
    creature("Gulfwyrm", Rare, Creature {
        stage: Stage::Stage2, evolves_from: Some("Streamfin"), hp: 120, color: Water,
        weakness: Some(Lightning), resistance: Some(Fire), retreat_cost: 3,
        attacks: &[
            atk("Frost Lock", "Flip a coin. If heads, the defending Hexon can't attack during your opponent's next turn.",
                &[Water, Water], 30, &[Place { placed: Placed::CantAttack, coin: true }]),
            atk("Gulf Crash", "Discard 1 energy card attached to the defending Hexon.",
                &[Water, Water, Water, Colorless], 70, &[DiscardDefenderEnergy]),
        ],
        power: None,
    }),
    creature("Kelpup", Common, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 60, color: Water,
        weakness: Some(Lightning), resistance: None, retreat_cost: 1,
        attacks: &[
            atk("Brine Bite", "", &[Water], 10, &[]),
            atk("Seaweed Wrap", "The defending Hexon can't retreat during your opponent's next turn.",
                &[Water, Colorless], 20, &[Place { placed: Placed::CantRetreat, coin: false }]),
        ],
        power: None,
    }),
    creature("Kelpwarden", Uncommon, Creature {
        stage: Stage::Stage1, evolves_from: Some("Kelpup"), hp: 90, color: Water,
        weakness: Some(Lightning), resistance: None, retreat_cost: 2,
        attacks: &[
            atk("Tide Gift", "Search your deck for a Water energy card and attach it to this Hexon, then shuffle your deck.",
                &[Water], 10, &[AttachFromDeck(Water)]),
            atk("Crushing Current", "", &[Water, Water, Colorless], 50, &[]),
        ],
        power: None,
    }),
    creature("Frosteel", Uncommon, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 60, color: Water,
        weakness: Some(Lightning), resistance: None, retreat_cost: 1,
        attacks: &[
            atk("Rime Shard", "Flip a coin. If heads, the defending Hexon is now Paralyzed.",
                &[Water, Colorless], 20, &[Inflict { status: Status::Paralyzed, coin: true }]),
            atk("Cold Bite", "", &[Water, Water, Colorless], 40, &[]),
        ],
        power: None,
    }),
    // ----- Lightning: coin bursts, paralysis, bench snipes -----
    creature("Sparkit", Common, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 50, color: Lightning,
        weakness: Some(Fighting), resistance: None, retreat_cost: 1,
        attacks: &[
            atk("Static Nip", "Flip a coin. If heads, the defending Hexon is now Paralyzed.",
                &[Lightning], 10, &[Inflict { status: Status::Paralyzed, coin: true }]),
            atk("Jolt Dash", "This Hexon does 10 damage to itself.",
                &[Lightning, Colorless], 30, &[Recoil(10)]),
        ],
        power: None,
    }),
    creature("Arcfang", Uncommon, Creature {
        stage: Stage::Stage1, evolves_from: Some("Sparkit"), hp: 80, color: Lightning,
        weakness: None, resistance: None, retreat_cost: 1,
        attacks: &[
            atk("Arc Bite", "Flip a coin. If heads, the defending Hexon is now Paralyzed.",
                &[Lightning, Colorless], 20, &[Inflict { status: Status::Paralyzed, coin: true }]),
            atk("Chain Spark", "Does 10 damage to each of your opponent's Benched Hexons.",
                &[Lightning, Lightning, Colorless], 40, &[BenchDamage { amount: 10, own: false, opponent: true }]),
        ],
        power: None,
    }),
    creature("Fulgurex", Rare, Creature {
        stage: Stage::Stage2, evolves_from: Some("Arcfang"), hp: 100, color: Lightning,
        weakness: Some(Fighting), resistance: None, retreat_cost: 2,
        attacks: &[
            atk("Forked Bolt", "Does 20 damage to 1 of your opponent's Hexons.",
                &[Lightning, Lightning], 40, &[Snipe(20)]),
            atk("Storm Barrage", "Flip 4 coins. This attack does 30 damage for each heads.",
                &[Lightning, Lightning, Lightning, Colorless], 0, &[AddPerHeads { coins: 4, per: 30 }]),
        ],
        power: None,
    }),
    creature("Buzzlet", Common, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 50, color: Lightning,
        weakness: Some(Fighting), resistance: None, retreat_cost: 1,
        attacks: &[
            atk("Buzz", "", &[Colorless], 10, &[]),
            atk("Shock Wing", "", &[Lightning, Colorless], 20, &[]),
        ],
        power: None,
    }),
    creature("Hivevolt", Uncommon, Creature {
        stage: Stage::Stage1, evolves_from: Some("Buzzlet"), hp: 80, color: Lightning,
        weakness: Some(Fighting), resistance: None, retreat_cost: 1,
        attacks: &[
            atk("Swarm Shock", "Flip 3 coins. This attack does 10 more damage for each heads.",
                &[Lightning, Colorless], 10, &[AddPerHeads { coins: 3, per: 10 }]),
            atk("Static Field", "Flip a coin. If heads, the defending Hexon is now Paralyzed.",
                &[Lightning, Lightning, Colorless], 40, &[Inflict { status: Status::Paralyzed, coin: true }]),
        ],
        power: None,
    }),
    creature("Ohmlet", Common, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 60, color: Lightning,
        weakness: Some(Water), resistance: None, retreat_cost: 1,
        attacks: &[
            atk("Tiny Arc", "Does 10 damage to 1 of your opponent's Hexons.",
                &[Lightning], 0, &[Snipe(10)]),
            atk("Hum Pulse", "", &[Lightning, Colorless], 20, &[]),
        ],
        power: None,
    }),
    creature("Ohmrunner", Uncommon, Creature {
        stage: Stage::Stage1, evolves_from: Some("Ohmlet"), hp: 80, color: Lightning,
        weakness: Some(Water), resistance: None, retreat_cost: 1,
        attacks: &[
            atk("Live Current", "Search your deck for a Lightning energy card and attach it to this Hexon, then shuffle your deck.",
                &[Lightning], 10, &[AttachFromDeck(Lightning)]),
            atk("Rail Shot", "Does 20 damage to 1 of your opponent's Hexons.",
                &[Lightning, Lightning, Colorless], 40, &[Snipe(20)]),
        ],
        power: None,
    }),
    creature("Stormox", Uncommon, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 70, color: Lightning,
        weakness: Some(Fighting), resistance: None, retreat_cost: 2,
        attacks: &[
            atk("Charged Horn", "", &[Lightning, Colorless], 30, &[]),
            atk("Overload", "This Hexon does 20 damage to itself.",
                &[Lightning, Lightning, Colorless], 60, &[Recoil(20)]),
        ],
        power: None,
    }),
    creature("Zippet", Common, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 40, color: Lightning,
        weakness: Some(Fighting), resistance: None, retreat_cost: 0,
        attacks: &[
            atk("Flicker", "", &[Lightning], 10, &[]),
            atk("Static Hop", "Switch this Hexon with 1 of your Benched Hexons.",
                &[Lightning, Colorless], 20, &[SwitchSelf]),
        ],
        power: None,
    }),
    creature("Flashfin", Common, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 60, color: Lightning,
        weakness: Some(Fighting), resistance: None, retreat_cost: 1,
        attacks: &[
            atk("Shock Bite", "", &[Lightning], 10, &[]),
            atk("Flash Burst", "Flip a coin. If heads, this attack does 20 more damage.",
                &[Lightning, Lightning, Colorless], 40, &[AddPerHeads { coins: 1, per: 20 }]),
        ],
        power: None,
    }),
    // ----- Fighting: heavy hits that grow with damage, resistance -----
    creature("Knollossus", Rare, Creature {
        stage: Stage::Stage2, evolves_from: Some("Knollgrim"), hp: 120, color: Fighting,
        weakness: Some(Grass), resistance: Some(Lightning), retreat_cost: 4,
        attacks: &[
            atk("Quake Stomp", "Does 10 damage to each Benched Hexon, yours and your opponent's.",
                &[Fighting, Fighting], 40, &[BenchDamage { amount: 10, own: true, opponent: true }]),
            atk("Mountain Fall", "This attack does 10 more damage for each 10 damage on this Hexon.",
                &[Fighting, Fighting, Fighting, Colorless], 80, &[AddPerDamage { per: 10, own: true }]),
        ],
        power: None,
    }),
    creature("Brawlet", Common, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 60, color: Fighting,
        weakness: Some(Grass), resistance: None, retreat_cost: 1,
        attacks: &[
            atk("Knuckle Tap", "", &[Fighting], 10, &[]),
            atk("Grudge Punch", "This attack does 10 more damage for each 10 damage on this Hexon.",
                &[Fighting, Colorless], 20, &[AddPerDamage { per: 10, own: true }]),
        ],
        power: None,
    }),
    creature("Ironbrawl", Uncommon, Creature {
        stage: Stage::Stage1, evolves_from: Some("Brawlet"), hp: 80, color: Fighting,
        weakness: Some(Grass), resistance: None, retreat_cost: 2,
        attacks: &[
            atk("Counterblow", "This attack does 10 more damage for each 10 damage on this Hexon.",
                &[Fighting, Colorless], 30, &[AddPerDamage { per: 10, own: true }]),
            atk("Haymaker", "", &[Fighting, Fighting, Colorless], 50, &[]),
        ],
        power: None,
    }),
    creature("Shalepup", Common, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 50, color: Fighting,
        weakness: Some(Grass), resistance: None, retreat_cost: 1,
        attacks: &[
            atk("Gravel Kick", "Flip a coin. If heads, this attack does 10 more damage.",
                &[Fighting], 10, &[AddPerHeads { coins: 1, per: 10 }]),
            atk("Rock Hurl", "", &[Fighting, Fighting], 30, &[]),
        ],
        power: None,
    }),
    creature("Shalehound", Uncommon, Creature {
        stage: Stage::Stage1, evolves_from: Some("Shalepup"), hp: 80, color: Fighting,
        weakness: Some(Grass), resistance: None, retreat_cost: 2,
        attacks: &[
            atk("Stone Fang", "", &[Fighting, Colorless], 30, &[]),
            atk("Crag Rush", "The defending Hexon can't retreat during your opponent's next turn.",
                &[Fighting, Fighting, Colorless], 50, &[Place { placed: Placed::CantRetreat, coin: false }]),
        ],
        power: None,
    }),
    creature("Cragmastiff", Rare, Creature {
        stage: Stage::Stage2, evolves_from: Some("Shalehound"), hp: 110, color: Fighting,
        weakness: Some(Grass), resistance: None, retreat_cost: 3,
        attacks: &[
            atk("Tremor Bite", "During your opponent's next turn, the defending Hexon's attacks do 20 less damage.",
                &[Fighting, Colorless], 30, &[Place { placed: Placed::Weaken(20), coin: false }]),
            atk("Earthsplitter", "Discard 1 Fighting energy card attached to this Hexon.",
                &[Fighting, Fighting, Fighting, Colorless], 80, &[DiscardEnergy { color: Fighting, count: 1 }]),
        ],
        power: None,
    }),
    creature("Bouldergut", Uncommon, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 80, color: Fighting,
        weakness: Some(Grass), resistance: None, retreat_cost: 3,
        attacks: &[
            atk("Heavy Lean", "", &[Fighting, Colorless], 20, &[]),
            atk("Boulder Drop", "", &[Fighting, Fighting, Colorless, Colorless], 50, &[]),
        ],
        power: None,
    }),
    // ----- Psychic: sleep, confusion, finishing the weakened -----
    creature("Wraithelle", Rare, Creature {
        stage: Stage::Stage2, evolves_from: Some("Wispelle"), hp: 100, color: Psychic,
        weakness: Some(Fighting), resistance: None, retreat_cost: 1,
        attacks: &[
            atk("Nightmare Veil", "The defending Hexon is now Asleep and Poisoned.",
                &[Psychic, Psychic], 20, &[Inflict { status: Status::Asleep, coin: false }, Inflict { status: Status::Poisoned, coin: false }]),
            atk("Soul Siphon", "Remove 20 damage from this Hexon.",
                &[Psychic, Psychic, Colorless], 50, &[Heal(20)]),
        ],
        power: None,
    }),
    creature("Dozel", Common, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 60, color: Psychic,
        weakness: Some(Psychic), resistance: None, retreat_cost: 2,
        attacks: &[
            atk("Drowse Gaze", "Flip a coin. If heads, the defending Hexon is now Asleep.",
                &[Psychic], 10, &[Inflict { status: Status::Asleep, coin: true }]),
            atk("Dream Ram", "", &[Psychic, Colorless], 20, &[]),
        ],
        power: None,
    }),
    creature("Somnitar", Uncommon, Creature {
        stage: Stage::Stage1, evolves_from: Some("Dozel"), hp: 90, color: Psychic,
        weakness: Some(Psychic), resistance: None, retreat_cost: 2,
        attacks: &[
            atk("Lullaby Wave", "The defending Hexon is now Asleep.",
                &[Psychic, Colorless], 20, &[Inflict { status: Status::Asleep, coin: false }]),
            atk("Mind Press", "", &[Psychic, Psychic, Colorless], 50, &[]),
        ],
        power: None,
    }),
    creature("Hexlet", Common, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 50, color: Psychic,
        weakness: None, resistance: None, retreat_cost: 1,
        attacks: &[
            atk("Small Hex", "Flip a coin. If heads, the defending Hexon is now Confused.",
                &[Psychic], 10, &[Inflict { status: Status::Confused, coin: true }]),
            atk("Jinx", "During your opponent's next turn, the defending Hexon must flip heads to attack.",
                &[Psychic, Colorless], 10, &[Place { placed: Placed::Accuracy, coin: false }]),
        ],
        power: None,
    }),
    creature("Hexmage", Uncommon, Creature {
        stage: Stage::Stage1, evolves_from: Some("Hexlet"), hp: 80, color: Psychic,
        weakness: Some(Psychic), resistance: None, retreat_cost: 1,
        attacks: &[
            atk("Mind Twist", "The defending Hexon is now Confused.",
                &[Psychic, Colorless], 20, &[Inflict { status: Status::Confused, coin: false }]),
            atk("Spite Bolt", "This attack does 10 more damage for each 10 damage on the defending Hexon.",
                &[Psychic, Psychic, Colorless], 40, &[AddPerDamage { per: 10, own: false }]),
        ],
        power: None,
    }),
    creature("Hexarch", Rare, Creature {
        stage: Stage::Stage2, evolves_from: Some("Hexmage"), hp: 100, color: Psychic,
        weakness: Some(Psychic), resistance: None, retreat_cost: 2,
        attacks: &[
            atk("Grand Hex", "This attack does 10 more damage for each 10 damage on the defending Hexon.",
                &[Psychic, Psychic, Psychic, Colorless], 60, &[AddPerDamage { per: 10, own: false }]),
        ],
        power: Some(Power {
            name: "Bewilder",
            text: "Once during your turn, if this Hexon was in play when your turn began, you may flip a coin. If heads, your opponent's Active Hexon is now Confused.",
            kind: PowerKind::Activated { effects: &[Inflict { status: Status::Confused, coin: true }], once_per_turn: true, settled_only: true },
        }),
    }),
    creature("Gloamowl", Uncommon, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 50, color: Psychic,
        weakness: Some(Psychic), resistance: Some(Fighting), retreat_cost: 0,
        attacks: &[
            atk("Dusk Stare", "Flip a coin. If heads, the defending Hexon is now Asleep.",
                &[Psychic], 10, &[Inflict { status: Status::Asleep, coin: true }]),
            atk("Shadow Swoop", "", &[Psychic, Colorless, Colorless], 30, &[]),
        ],
        power: None,
    }),
    // ----- Colorless: utility, draw, switching -----
    creature("Burrowbit", Common, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 50, color: Colorless,
        weakness: Some(Fighting), resistance: None, retreat_cost: 1,
        attacks: &[
            atk("Rummage", "Draw a card.", &[Colorless], 10, &[Draw(1)]),
            atk("Bowl Over", "", &[Colorless, Colorless], 20, &[]),
        ],
        power: None,
    }),
    creature("Warrenbuck", Uncommon, Creature {
        stage: Stage::Stage1, evolves_from: Some("Burrowbit"), hp: 70, color: Colorless,
        weakness: Some(Fighting), resistance: None, retreat_cost: 1,
        attacks: &[
            atk("Thump", "", &[Colorless, Colorless], 30, &[]),
            atk("Burrow Dash", "Switch this Hexon with 1 of your Benched Hexons.",
                &[Colorless, Colorless, Colorless], 40, &[SwitchSelf]),
        ],
        power: None,
    }),
    creature("Warrenking", Rare, Creature {
        stage: Stage::Stage2, evolves_from: Some("Warrenbuck"), hp: 100, color: Colorless,
        weakness: Some(Fighting), resistance: None, retreat_cost: 2,
        attacks: &[
            atk("Stampede", "", &[Colorless, Colorless, Colorless], 50, &[]),
            atk("Warren Quake", "", &[Colorless, Colorless, Colorless, Colorless], 80, &[]),
        ],
        power: Some(Power {
            name: "Warren Welcome",
            text: "When you put this Hexon into play by awakening, draw 2 cards.",
            kind: PowerKind::OnPlay(&[Draw(2)]),
        }),
    }),
    creature("Pecklet", Common, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 40, color: Colorless,
        weakness: Some(Lightning), resistance: Some(Fighting), retreat_cost: 0,
        attacks: &[
            atk("Beak Jab", "", &[Colorless], 10, &[]),
            atk("Gust Flap", "Your opponent switches the defending Hexon with 1 of their Benched Hexons.",
                &[Colorless, Colorless], 10, &[SwitchDefender { chooser_is_attacker: false }]),
        ],
        power: None,
    }),
    creature("Galeheron", Uncommon, Creature {
        stage: Stage::Stage1, evolves_from: Some("Pecklet"), hp: 70, color: Colorless,
        weakness: Some(Lightning), resistance: Some(Fighting), retreat_cost: 1,
        attacks: &[
            atk("Wing Cutter", "Flip a coin. If heads, this attack does 20 more damage.",
                &[Colorless, Colorless], 20, &[AddPerHeads { coins: 1, per: 20 }]),
            atk("Spiral Gale", "Switch the defending Hexon with 1 of your opponent's Benched Hexons, chosen by you.",
                &[Colorless, Colorless, Colorless], 30, &[SwitchDefender { chooser_is_attacker: true }]),
        ],
        power: None,
    }),
    creature("Echopup", Uncommon, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 50, color: Colorless,
        weakness: Some(Fighting), resistance: None, retreat_cost: 1,
        attacks: &[
            atk("Yip", "", &[Colorless], 10, &[]),
            atk("Echo Back", "Choose 1 of the defending Hexon's attacks and use it as this attack.",
                &[Colorless, Colorless, Colorless], 0, &[CopyAttack]),
        ],
        power: None,
    }),
    creature("Haulox", Uncommon, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 80, color: Colorless,
        weakness: Some(Fighting), resistance: None, retreat_cost: 3,
        attacks: &[
            atk("Shoulder Shove", "During your opponent's next turn, this Hexon takes 20 less damage from attacks.",
                &[Colorless, Colorless], 20, &[Protect { shield: Shield::Reduce(20), coin: false }]),
            atk("Heavy Trample", "", &[Colorless, Colorless, Colorless, Colorless], 50, &[]),
        ],
        power: None,
    }),
    creature("Pocketmouse", Common, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 40, color: Colorless,
        weakness: Some(Fighting), resistance: None, retreat_cost: 0,
        attacks: &[atk("Crumb Nip", "", &[Colorless], 10, &[])],
        power: Some(Power {
            name: "Scurry",
            text: "While this Hexon is on your Bench, your Active Hexon's retreat cost is 1 less.",
            kind: PowerKind::RetreatReducer,
        }),
    }),
    creature("Mendlark", Uncommon, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 50, color: Colorless,
        weakness: Some(Lightning), resistance: Some(Fighting), retreat_cost: 1,
        attacks: &[
            atk("Mending Chirp", "Remove 20 damage from this Hexon.", &[Colorless], 10, &[Heal(20)]),
            atk("Restful Tune", "Removes all conditions from both Active Hexons.",
                &[Colorless, Colorless], 20, &[Cure { both: true }]),
        ],
        power: None,
    }),
    // ----- Trainers -----
    trainer("Dowsing Rod", Common,
            "Search your deck for a basic energy card, put it into your hand, then shuffle your deck.",
            &[Search(Find::BasicEnergy)]),
    trainer("Field Guide", Uncommon,
            "Search your deck for a Hexon card, put it into your hand, then shuffle your deck.",
            &[Search(Find::Creature)]),
    trainer("Ash Sifter", Common, "Put a basic energy card from your discard pile into your hand.",
            &[Recover(Find::BasicEnergy)]),
    trainer("Scrap Ledger", Uncommon, "Put a trainer card from your discard pile into your hand.",
            &[Recover(Find::Trainer)]),
    trainer("Clean Slate", Rare, "Discard your hand, then draw 7 cards.", &[DiscardHand, Draw(7)]),
    trainer("Grove Balm", Uncommon, "Remove up to 20 damage from each of your Hexons.", &[HealAll(20)]),
    trainer("Charge Coil", Rare,
            "Search your deck for a basic energy card and attach it to your Active Hexon, then shuffle your deck.",
            &[AttachFromDeck(Colorless)]),
    trainer("Jolt Wire", Rare, "Do 20 damage to 1 of your opponent's Hexons.", &[Snipe(20)]),
    trainer("Static Net", Common, "Your opponent's Active Hexon can't retreat during their next turn.",
            &[Place { placed: Placed::CantRetreat, coin: false }]),
    trainer("Smoke Pellet", Uncommon,
            "During your opponent's next turn, their Active Hexon must flip heads to attack.",
            &[Place { placed: Placed::Accuracy, coin: false }]),
    trainer("Venom Dart", Uncommon, "Flip a coin. If heads, your opponent's Active Hexon is now Poisoned.",
            &[Inflict { status: Status::Poisoned, coin: true }]),
    trainer("Wide Satchel", Rare, "Draw 3 cards.", &[Draw(3)]),
    trainer("Hearty Broth", Uncommon, "Remove up to 50 damage from your Active Hexon.", &[Heal(50)]),
    // ----- Promos (campaign rewards; see PROMOS) -----
    creature("Thistlewick", Rare, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 70, color: Grass,
        weakness: Some(Fire), resistance: None, retreat_cost: 1,
        attacks: &[
            atk("Prickle", "Flip a coin. If heads, the defending Hexon is now Poisoned.",
                &[Grass], 10, &[Inflict { status: Status::Poisoned, coin: true }]),
            atk("Bramble Lash", "", &[Grass, Colorless, Colorless], 40, &[]),
        ],
        power: None,
    }),
    creature("Brinemoth", Rare, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 60, color: Water,
        weakness: Some(Lightning), resistance: None, retreat_cost: 1,
        attacks: &[
            atk("Undertow", "Switch the defending Hexon with 1 of your opponent's Benched Hexons, chosen by you.",
                &[Water, Colorless], 20, &[SwitchDefender { chooser_is_attacker: true }]),
            atk("Brine Burst", "", &[Water, Water, Colorless], 50, &[]),
        ],
        power: None,
    }),
    creature("Kilnfox", Rare, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 60, color: Fire,
        weakness: Some(Water), resistance: None, retreat_cost: 1,
        attacks: &[
            atk("Stoke", "Search your deck for a Fire energy card and attach it to this Hexon, then shuffle your deck.",
                &[Fire], 10, &[AttachFromDeck(Fire)]),
            atk("Flash Fire", "Discard 1 Fire energy card attached to this Hexon.",
                &[Fire, Fire, Colorless], 60, &[DiscardEnergy { color: Fire, count: 1 }]),
        ],
        power: None,
    }),
    creature("Squallkite", Rare, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 60, color: Lightning,
        weakness: Some(Fighting), resistance: None, retreat_cost: 0,
        attacks: &[
            atk("Static Gust", "Flip a coin. If heads, the defending Hexon is now Paralyzed.",
                &[Lightning], 10, &[Inflict { status: Status::Paralyzed, coin: true }]),
            atk("Thunder Dive", "This Hexon does 10 damage to itself.",
                &[Lightning, Lightning, Colorless], 60, &[Recoil(10)]),
        ],
        power: None,
    }),
    creature("Slatebrow", Rare, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 80, color: Fighting,
        weakness: Some(Grass), resistance: None, retreat_cost: 2,
        attacks: &[
            atk("Rock Brace", "During your opponent's next turn, this Hexon takes 20 less damage from attacks.",
                &[Fighting, Colorless], 20, &[Protect { shield: Shield::Reduce(20), coin: false }]),
            atk("Landslide", "", &[Fighting, Fighting, Colorless], 40, &[]),
        ],
        power: None,
    }),
    creature("Moonmoth", Rare, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 60, color: Psychic,
        weakness: Some(Psychic), resistance: Some(Fighting), retreat_cost: 1,
        attacks: &[
            atk("Hush Wings", "The defending Hexon is now Asleep.",
                &[Psychic, Colorless], 10, &[Inflict { status: Status::Asleep, coin: false }]),
            atk("Moonbeam", "Flip 2 coins. This attack does 20 more damage for each heads.",
                &[Psychic, Psychic, Colorless], 30, &[AddPerHeads { coins: 2, per: 20 }]),
        ],
        power: None,
    }),
    creature("Hexcrest", Rare, Creature {
        stage: Stage::Basic, evolves_from: None, hp: 80, color: Colorless,
        weakness: Some(Fighting), resistance: None, retreat_cost: 1,
        attacks: &[
            atk("Sixfold Guard", "During your opponent's next turn, this Hexon takes 20 less damage from attacks.",
                &[Colorless, Colorless], 20, &[Protect { shield: Shield::Reduce(20), coin: false }]),
            atk("Crown Strike", "", &[Colorless, Colorless, Colorless], 50, &[]),
        ],
        power: Some(Power {
            name: "Champion's Calm",
            text: "This Hexon can't be affected by conditions.",
            kind: PowerKind::StatusImmune,
        }),
    }),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::card_id;

    #[test]
    fn set_is_valid() {
        assert_eq!(crate::card::validate_cards(CARDS), Ok(()));
    }

    /// Evolutions are rarer than Basics (Stage 2 always Rare), and every
    /// booster tier has cards.
    #[test]
    fn rarity_follows_stage() {
        for c in CARDS {
            match c.creature().map(|cr| cr.stage) {
                Some(Stage::Stage1) => assert!(c.rarity >= Uncommon, "{}", c.name),
                Some(Stage::Stage2) => assert_eq!(c.rarity, Rare, "{}", c.name),
                _ => {}
            }
        }
        for r in [Rarity::Energy, Common, Uncommon, Rare] {
            assert!(CARDS.iter().any(|c| c.rarity == r), "no {r:?} card");
        }
    }

    /// Every colour has a full Basic → Stage 1 → Stage 2 line, and every
    /// evolution's chain reaches a Basic of the same colour in the set.
    #[test]
    fn every_colour_has_a_full_line() {
        for color in crate::Color::ALL {
            let full = CARDS.iter().any(|c| {
                let Some(top) = c.creature().filter(|cr| cr.color == color) else {
                    return false;
                };
                let mut stage = top;
                let mut depth = 0;
                while let Some(from) = stage.evolves_from {
                    stage = crate::card::card(card_id(from).unwrap())
                        .creature()
                        .unwrap();
                    depth += 1;
                }
                depth == 2 && stage.stage == Stage::Basic
            });
            assert!(full, "{color:?} has no Stage 2 line");
        }
    }

    /// The 0.4.0 set: about 70 creatures, 25 trainers, 6 basic energies and
    /// the special energy; at least 9 creatures per colour.
    #[test]
    fn set_size_by_kind() {
        let creatures = CARDS.iter().filter(|c| c.creature().is_some()).count();
        let trainers = CARDS
            .iter()
            .filter(|c| matches!(c.kind, CardKind::Trainer(_)))
            .count();
        assert_eq!((creatures, trainers, CARDS.len()), (78, 25, 110));
        for color in crate::Color::ALL {
            let n = CARDS
                .iter()
                .filter(|c| c.creature().is_some_and(|cr| cr.color == color))
                .count();
            assert!(n >= 9, "{color:?}: {n} creatures");
        }
    }

    #[test]
    fn basic_energy_by_colour() {
        assert_eq!(basic_energy(Fire), Some(FIRE_ENERGY));
        assert_eq!(basic_energy(Psychic), Some(PSYCHIC_ENERGY));
        assert_eq!(basic_energy(Colorless), None);
    }

    /// Every constant names the card at its index.
    #[test]
    fn constants_match_names() {
        #[rustfmt::skip]
        let named = [
            (CINDREL, "Cindrel"), (FLARROW, "Flarrow"), (PYREQUILL, "Pyrequill"),
            (KNOLLOP, "Knollop"), (KNOLLGRIM, "Knollgrim"), (TUSKLET, "Tusklet"),
            (GRITFANG, "Gritfang"), (PUDDOCK, "Puddock"), (BROOKHORN, "Brookhorn"),
            (TIDEMAW, "Tidemaw"), (SHELLMOTE, "Shellmote"), (WISPUN, "Wispun"),
            (WISPELLE, "Wispelle"), (GLIMMOX, "Glimmox"), (SCAMPIT, "Scampit"),
            (TRAIL_COMPASS, "Trail Compass"), (LUCKY_SATCHEL, "Lucky Satchel"),
            (MOSS_POULTICE, "Moss Poultice"), (CLEARWATER_VIAL, "Clearwater Vial"),
            (RELAY_BATON, "Relay Baton"), (SNARE_HOOK, "Snare Hook"),
            (DRAIN_BRUSH, "Drain Brush"), (SPIKE_CHARM, "Spike Charm"),
            (BARK_SHIELD, "Bark Shield"), (RECALL_HORN, "Recall Horn"),
            (BLOOM_SEED, "Bloom Seed"), (UNWIND_SPRAY, "Unwind Spray"),
            (GRASS_ENERGY, "Grass Energy"), (FIRE_ENERGY, "Fire Energy"),
            (WATER_ENERGY, "Water Energy"), (LIGHTNING_ENERGY, "Lightning Energy"),
            (FIGHTING_ENERGY, "Fighting Energy"), (PSYCHIC_ENERGY, "Psychic Energy"),
            (TWOFOLD_ENERGY, "Twofold Energy"),
            (SPRIGGET, "Sprigget"),
            (THORNBUCK, "Thornbuck"),
            (ELDERHART, "Elderhart"),
            (MIREBUD, "Mirebud"),
            (BOGBLOSSOM, "Bogblossom"),
            (BOGCROWN, "Bogcrown"),
            (POLLENPUP, "Pollenpup"),
            (HONEYMANE, "Honeymane"),
            (BRIARHOG, "Briarhog"),
            (FERNWHISK, "Fernwhisk"),
            (ASHPAW, "Ashpaw"),
            (BLAZELYNX, "Blazelynx"),
            (SOLARLYNX, "Solarlynx"),
            (KILNBACK, "Kilnback"),
            (FURNACEBACK, "Furnaceback"),
            (WAXWICK, "Waxwick"),
            (CINDERHORN, "Cinderhorn"),
            (RILLET, "Rillet"),
            (STREAMFIN, "Streamfin"),
            (GULFWYRM, "Gulfwyrm"),
            (KELPUP, "Kelpup"),
            (KELPWARDEN, "Kelpwarden"),
            (FROSTEEL, "Frosteel"),
            (SPARKIT, "Sparkit"),
            (ARCFANG, "Arcfang"),
            (FULGUREX, "Fulgurex"),
            (BUZZLET, "Buzzlet"),
            (HIVEVOLT, "Hivevolt"),
            (OHMLET, "Ohmlet"),
            (OHMRUNNER, "Ohmrunner"),
            (STORMOX, "Stormox"),
            (ZIPPET, "Zippet"),
            (FLASHFIN, "Flashfin"),
            (KNOLLOSSUS, "Knollossus"),
            (BRAWLET, "Brawlet"),
            (IRONBRAWL, "Ironbrawl"),
            (SHALEPUP, "Shalepup"),
            (SHALEHOUND, "Shalehound"),
            (CRAGMASTIFF, "Cragmastiff"),
            (BOULDERGUT, "Bouldergut"),
            (WRAITHELLE, "Wraithelle"),
            (DOZEL, "Dozel"),
            (SOMNITAR, "Somnitar"),
            (HEXLET, "Hexlet"),
            (HEXMAGE, "Hexmage"),
            (HEXARCH, "Hexarch"),
            (GLOAMOWL, "Gloamowl"),
            (BURROWBIT, "Burrowbit"),
            (WARRENBUCK, "Warrenbuck"),
            (WARRENKING, "Warrenking"),
            (PECKLET, "Pecklet"),
            (GALEHERON, "Galeheron"),
            (ECHOPUP, "Echopup"),
            (HAULOX, "Haulox"),
            (POCKETMOUSE, "Pocketmouse"),
            (MENDLARK, "Mendlark"),
            (DOWSING_ROD, "Dowsing Rod"),
            (FIELD_GUIDE, "Field Guide"),
            (ASH_SIFTER, "Ash Sifter"),
            (SCRAP_LEDGER, "Scrap Ledger"),
            (CLEAN_SLATE, "Clean Slate"),
            (GROVE_BALM, "Grove Balm"),
            (CHARGE_COIL, "Charge Coil"),
            (JOLT_WIRE, "Jolt Wire"),
            (STATIC_NET, "Static Net"),
            (SMOKE_PELLET, "Smoke Pellet"),
            (VENOM_DART, "Venom Dart"),
            (WIDE_SATCHEL, "Wide Satchel"),
            (HEARTY_BROTH, "Hearty Broth"),
            (THISTLEWICK, "Thistlewick"),
            (BRINEMOTH, "Brinemoth"),
            (KILNFOX, "Kilnfox"),
            (SQUALLKITE, "Squallkite"),
            (SLATEBROW, "Slatebrow"),
            (MOONMOTH, "Moonmoth"),
            (HEXCREST, "Hexcrest"),
        ];
        assert_eq!(named.len(), CARDS.len());
        for (id, name) in named {
            assert_eq!(card_id(name), Some(id), "{name}");
        }
    }

    /// Every attack has a cost and text for any effect; every creature has an
    /// attack; evolutions keep their colour. Creature names never reuse a
    /// fixture name (evolution looks names up in the fixtures first under test).
    #[test]
    fn cards_are_consistent() {
        for c in CARDS {
            let Some(cr) = c.creature() else { continue };
            assert!(
                !crate::fixtures::CARDS.iter().any(|f| f.name == c.name),
                "{} clashes with a fixture card",
                c.name
            );
            assert!(!cr.attacks.is_empty(), "{} has no attack", c.name);
            for a in cr.attacks {
                assert!(!a.cost.is_empty(), "{}: {} has no cost", c.name, a.name);
                assert!(
                    a.effects.is_empty() || !a.text.is_empty(),
                    "{}: {} needs text",
                    c.name,
                    a.name
                );
            }
            if let Some(from) = cr.evolves_from {
                let below = crate::card::card(card_id(from).unwrap())
                    .creature()
                    .unwrap();
                assert_eq!(below.color, cr.color, "{} changes colour", c.name);
                assert!(below.hp < cr.hp, "{} loses HP when evolving", c.name);
            }
        }
    }
}

/// The value budget (`docs/BALANCE.md`, "Value budget"): every creature is
/// scored in damage-equivalents (10 = 10 damage = 10 HP) against the curve
/// for its stage, so a new or changed card can't drift off the curve.
#[cfg(test)]
mod budget {
    use super::*;
    use crate::condition::{Placed, Shield, Status};
    use crate::effect::Effect;

    /// Expected damage of a vanilla attack, by stage (Basic, Stage 1,
    /// Stage 2) and energy cost (1, 2, 3, 4+).
    const CURVE: [[i32; 4]; 3] = [[10, 20, 40, 50], [10, 20, 50, 60], [10, 30, 50, 80]];
    /// Median HP and retreat cost by stage.
    const HP: [i32; 3] = [50, 80, 100];
    const RETREAT: [i32; 3] = [1, 1, 2];
    /// How far above the curve a card may sit, by role (about the classic
    /// sets' 90th percentile on the same scale), and the floor for all.
    const CEILING: i32 = 40;
    const SUPPORT_CEILING: i32 = 30;
    const FLOOR: i32 = -20;
    /// Off-budget on purpose, each with its reason. Keep this list short.
    const ACCEPTED: &[(&str, &str)] =
        &[("Hexcrest", "the champion's promo: one copy, earned last")];
    /// A main attacker this far above the attack curve needs a drawback.
    const DRAWBACK_AT: i32 = 20;

    fn stage(s: Stage) -> usize {
        match s {
            Stage::Basic => 0,
            Stage::Stage1 => 1,
            Stage::Stage2 => 2,
        }
    }

    /// Damage-equivalent of one effect; drawbacks are negative.
    pub(super) fn effect_value(e: &Effect) -> i32 {
        use Effect::*;
        let status = |s: &Status| match s {
            Status::Paralyzed | Status::DoublePoisoned => 20,
            _ => 10,
        };
        let coin = |v: i32, c: bool| if c { v / 2 } else { v };
        match e {
            AddPerHeads { coins, per } => i32::from(*coins) * i32::from(*per) / 2,
            AddPerDamage { .. } | DoubleNextAttack | OwnResistance(_) => 10,
            IgnoreWeaknessResistance => 5,
            Recoil(x) => -i32::from(*x) / 2,
            BenchDamage {
                amount,
                own,
                opponent,
            } => {
                let a = i32::from(*amount);
                (if *opponent { a * 3 / 2 } else { 0 }) - (if *own { a / 2 } else { 0 })
            }
            Snipe(x) => i32::from(*x),
            Inflict { status: s, coin: c } => coin(status(s), *c),
            InflictSelf(_) => -10,
            Cure { .. } | SwitchSelf => 5,
            Protect { shield, coin: c } => coin(
                match shield {
                    Shield::PreventDamage | Shield::PreventAll => 30,
                    Shield::Reduce(x) => i32::from(*x) / 2,
                    _ => 15,
                },
                *c,
            ),
            Place { placed, coin: c } => coin(
                match placed {
                    Placed::CantAttack => 20,
                    Placed::Weaken(x) => i32::from(*x) / 2,
                    Placed::CantRetreat => 5,
                    _ => 10,
                },
                *c,
            ),
            DisableAttack | TrainerLock | DefenderWeakness(_) => 10,
            DiscardEnergy { count, .. } => -10 * i32::from(*count),
            DiscardAllEnergy | OnceWhileInPlay => -30,
            DiscardDefenderEnergy | AttachFromDeck(_) => 20,
            SwitchDefender {
                chooser_is_attacker,
            } => {
                if *chooser_is_attacker {
                    20
                } else {
                    5
                }
            }
            Heal(x) => i32::from(*x) / 2,
            HealAll(x) => i32::from(*x),
            Draw(n) => 10 * i32::from(*n),
            Search(_) | Recover(_) => 15,
            CopyAttack => 30,
            _ => 0,
        }
    }

    fn power_value(p: &PowerKind) -> i32 {
        match p {
            // One extra attachment a turn (R-ENERGY-02), from the hand.
            PowerKind::FreeAttach(_) => 30,
            PowerKind::RetreatReducer | PowerKind::StatusImmune => 10,
            PowerKind::Retaliate => 15,
            PowerKind::OnPlay(effects) => effects.iter().map(effect_value).sum(),
            // Used every turn: worth about two uses, less if it must wait a turn.
            PowerKind::Activated {
                effects,
                settled_only,
                ..
            } => {
                let v = 2 * effects.iter().map(effect_value).sum::<i32>();
                if *settled_only {
                    v * 3 / 4
                } else {
                    v
                }
            }
            _ => 20,
        }
    }

    #[derive(Debug, PartialEq)]
    pub(super) enum Role {
        Attacker,
        Wall,
        Support,
        Engine,
        Tech,
    }

    pub(super) struct Score {
        pub hp: i32,
        pub attack: i32,
        pub power: i32,
        pub rest: i32,
        pub drawback: bool,
        pub role: Role,
    }

    impl Score {
        pub fn total(&self) -> i32 {
            self.hp + self.attack + self.power + self.rest
        }
    }

    /// The score of one creature: HP, its best attack against the curve,
    /// its Power, retreat, weakness and resistance.
    pub(super) fn score(cr: &Creature) -> Score {
        let s = stage(cr.stage);
        let value = |a: &Attack| {
            i32::from(a.damage) + a.effects.iter().map(effect_value).sum::<i32>()
                - CURVE[s][a.cost.len().clamp(1, 4) - 1]
        };
        // The main attack is the dearest one; a second attack adds half its surplus.
        let best = cr
            .attacks
            .iter()
            .max_by_key(|a| (a.cost.len(), value(a)))
            .unwrap();
        let other = cr
            .attacks
            .iter()
            .filter(|a| !std::ptr::eq(*a, best))
            .map(|a| value(a).max(0) / 2)
            .max()
            .unwrap_or(0);
        let effects_lead = best
            .effects
            .iter()
            .map(effect_value)
            .filter(|&v| v > 0)
            .sum::<i32>()
            > i32::from(best.damage);
        let power = cr.power.as_ref().map_or(0, |p| power_value(&p.kind));
        let hp = i32::from(cr.hp) - HP[s];
        let attack = value(best) + other;
        let role = match cr.power.as_ref().map(|p| p.kind) {
            Some(PowerKind::FreeAttach(_) | PowerKind::OnPlay(_)) => Role::Engine,
            Some(PowerKind::Activated { effects, .. })
                if effects
                    .iter()
                    .any(|e| matches!(e, Effect::Draw(_) | Effect::AttachFromDeck(_))) =>
            {
                Role::Engine
            }
            Some(PowerKind::Retaliate | PowerKind::RetreatReducer | PowerKind::StatusImmune) => {
                Role::Tech
            }
            _ if best.effects.iter().any(|e| matches!(e, Effect::CopyAttack)) => Role::Tech,
            _ if hp >= 20
                && (value(best) <= 0
                    || cr.attacks.iter().flat_map(|a| a.effects).any(|e| {
                        matches!(
                            e,
                            Effect::Protect { .. } | Effect::Heal(_) | Effect::HealAll(_)
                        )
                    })) =>
            {
                Role::Wall
            }
            _ if effects_lead => Role::Support,
            _ => Role::Attacker,
        };
        Score {
            hp,
            attack,
            power,
            rest: 5 * (RETREAT[s] - i32::from(cr.retreat_cost))
                + if cr.weakness.is_none() { 10 } else { 0 }
                + if cr.resistance.is_some() { 10 } else { 0 },
            drawback: cr
                .attacks
                .iter()
                .flat_map(|a| a.effects)
                .any(|e| effect_value(e) < 0 || matches!(e, Effect::BenchDamage { own: true, .. })),
            role,
        }
    }

    /// Prints the table (`-- --nocapture`) and fails on a card off its budget.
    #[test]
    fn every_creature_is_within_its_budget() {
        let mut off = vec![];
        for c in CARDS {
            let Some(cr) = c.creature() else { continue };
            let sc = score(cr);
            let t = sc.total();
            println!(
                "{:12} {:?} {:9} hp {:+4} atk {:+4} pow {:+4} rest {:+4} = {:+4}{}",
                c.name,
                cr.stage,
                format!("{:?}", sc.role),
                sc.hp,
                sc.attack,
                sc.power,
                sc.rest,
                t,
                if sc.drawback { "  (drawback)" } else { "" }
            );
            let ceiling = match sc.role {
                Role::Attacker | Role::Wall => CEILING,
                _ => SUPPORT_CEILING,
            };
            if ACCEPTED.iter().any(|(n, _)| *n == c.name) {
                continue;
            }
            if t > ceiling || t < FLOOR {
                off.push(format!(
                    "{} {t:+} (role {:?}, limit {FLOOR}..{ceiling})",
                    c.name, sc.role
                ));
            }
            if sc.role == Role::Attacker && sc.attack > DRAWBACK_AT && !sc.drawback {
                off.push(format!(
                    "{}: attack {:+} with no drawback",
                    c.name, sc.attack
                ));
            }
            if matches!(sc.role, Role::Wall | Role::Support) && sc.attack > 10 {
                off.push(format!(
                    "{}: a {:?} more than 10 above the attack curve",
                    c.name, sc.role
                ));
            }
            if sc.role == Role::Engine && sc.hp > 0 {
                off.push(format!("{}: an engine above its stage's HP", c.name));
            }
        }
        assert!(off.is_empty(), "off budget:\n{}", off.join("\n"));
    }
}
