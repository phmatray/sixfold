//! The original card set, built around two prebuilt decks (see [`crate::decks`]):
//! Ember Rush (Fire / Fighting aggro) and Tide Mind (Water / Psychic control).
//!
//! `CardId(n)` is the index into [`CARDS`]; keep the constants below in the
//! same order. All content is original: no names, text or stats from any
//! published card game.

use crate::card::{
    Attack, CardDef, CardId, CardKind, Color::*, Creature, EnergyCard, Power, PowerKind, Stage,
    Trainer,
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

/// Every card id in the set, in registry order (for card lists in the UI).
pub fn ids() -> impl Iterator<Item = CardId> {
    (0..CARDS.len() as u16).map(CardId)
}

const fn creature(name: &'static str, c: Creature) -> CardDef {
    CardDef {
        name,
        kind: CardKind::Creature(c),
    }
}

const fn trainer(
    name: &'static str,
    text: &'static str,
    effects: &'static [crate::Effect],
) -> CardDef {
    CardDef {
        name,
        kind: CardKind::Trainer(Trainer { text, effects }),
    }
}

const fn energy(name: &'static str, e: EnergyCard) -> CardDef {
    CardDef {
        name,
        kind: CardKind::Energy(e),
    }
}

#[rustfmt::skip]
pub static CARDS: &[CardDef] = &[
    // ----- Ember Rush: Fire line (Basic -> Stage 1 -> Stage 2) -----
    creature("Cindrel", Creature {
        stage: Stage::Basic, evolves_from: None, hp: 50, color: Fire,
        weakness: Some(Water), resistance: None, retreat_cost: 1,
        attacks: &[
            Attack { name: "Ember Flick", text: "Flip a coin. If heads, this attack does 10 more damage.",
                     cost: &[Fire], damage: 10, effects: &[AddPerHeads { coins: 1, per: 10 }] },
            Attack { name: "Cinder Dash", text: "", cost: &[Fire, Colorless], damage: 20, effects: &[] },
        ],
        power: None,
    }),
    creature("Flarrow", Creature {
        stage: Stage::Stage1, evolves_from: Some("Cindrel"), hp: 70, color: Fire,
        weakness: Some(Water), resistance: None, retreat_cost: 1,
        attacks: &[
            Attack { name: "Heat Haze",
                     text: "During your opponent's next turn, the defending creature must flip heads to attack.",
                     cost: &[Fire], damage: 10,
                     effects: &[Place { placed: Placed::Accuracy, coin: false }] },
            Attack { name: "Flare Dive", text: "This creature does 10 damage to itself.",
                     cost: &[Fire, Fire, Colorless], damage: 50, effects: &[Recoil(10)] },
        ],
        power: None,
    }),
    creature("Pyrequill", Creature {
        stage: Stage::Stage2, evolves_from: Some("Flarrow"), hp: 100, color: Fire,
        weakness: Some(Water), resistance: None, retreat_cost: 2,
        attacks: &[
            Attack { name: "Spark Rain", text: "Does 10 damage to each of your opponent's Benched creatures.",
                     cost: &[Fire, Fire], damage: 30,
                     effects: &[BenchDamage { amount: 10, own: false, opponent: true }] },
            Attack { name: "Inferno Plume", text: "Discard 2 Fire energy cards attached to this creature.",
                     cost: &[Fire, Fire, Fire, Colorless], damage: 90,
                     effects: &[DiscardEnergy { color: Fire, count: 2 }] },
        ],
        power: Some(Power {
            name: "Kindling Heart",
            text: "Fire energy you attach to your Fire creatures doesn't use up your energy attachment for the turn.",
            kind: PowerKind::FreeAttach(Fire),
        }),
    }),
    // ----- Ember Rush: Fighting -----
    creature("Knollop", Creature {
        stage: Stage::Basic, evolves_from: None, hp: 60, color: Fighting,
        weakness: Some(Grass), resistance: Some(Lightning), retreat_cost: 2,
        attacks: &[
            Attack { name: "Clod Toss", text: "", cost: &[Fighting], damage: 10, effects: &[] },
            Attack { name: "Rumble", text: "Does 10 damage to each Benched creature, yours and your opponent's.",
                     cost: &[Fighting, Fighting], damage: 30,
                     effects: &[BenchDamage { amount: 10, own: true, opponent: true }] },
        ],
        power: None,
    }),
    creature("Knollgrim", Creature {
        stage: Stage::Stage1, evolves_from: Some("Knollop"), hp: 90, color: Fighting,
        weakness: Some(Grass), resistance: Some(Lightning), retreat_cost: 3,
        attacks: &[
            Attack { name: "Granite Guard",
                     text: "During your opponent's next turn, this creature takes 20 less damage from attacks.",
                     cost: &[Fighting, Colorless], damage: 20,
                     effects: &[Protect { shield: Shield::Reduce(20), coin: false }] },
            Attack { name: "Berserk Slam", text: "This creature is now Confused.",
                     cost: &[Fighting, Fighting, Colorless], damage: 60,
                     effects: &[InflictSelf(Status::Confused)] },
        ],
        power: Some(Power {
            name: "Jagged Crust",
            text: "Whenever this creature takes damage that isn't from a Power, your opponent's Active creature takes 10 damage.",
            kind: PowerKind::Retaliate,
        }),
    }),
    creature("Tusklet", Creature {
        stage: Stage::Basic, evolves_from: None, hp: 50, color: Fighting,
        weakness: Some(Grass), resistance: None, retreat_cost: 1,
        attacks: &[
            Attack { name: "Gore", text: "Flip 2 coins. This attack does 10 damage for each heads.",
                     cost: &[Fighting], damage: 0, effects: &[AddPerHeads { coins: 2, per: 10 }] },
            Attack { name: "Wild Charge", text: "This creature does 10 damage to itself.",
                     cost: &[Fighting, Colorless], damage: 30, effects: &[Recoil(10)] },
        ],
        power: None,
    }),
    creature("Gritfang", Creature {
        stage: Stage::Basic, evolves_from: None, hp: 70, color: Fighting,
        weakness: Some(Grass), resistance: None, retreat_cost: 2,
        attacks: &[
            Attack { name: "Psych Up", text: "During your next turn, this creature's attack does double damage.",
                     cost: &[Colorless], damage: 10, effects: &[DoubleNextAttack] },
            Attack { name: "Crushing Jaw",
                     text: "The defending creature can't retreat during your opponent's next turn.",
                     cost: &[Fighting, Fighting, Colorless], damage: 40,
                     effects: &[Place { placed: Placed::CantRetreat, coin: false }] },
        ],
        power: None,
    }),
    // ----- Tide Mind: Water line (Basic -> Stage 1 -> Stage 2) -----
    creature("Puddock", Creature {
        stage: Stage::Basic, evolves_from: None, hp: 50, color: Water,
        weakness: Some(Lightning), resistance: None, retreat_cost: 1,
        attacks: &[
            Attack { name: "Bubble Spit", text: "Flip a coin. If heads, the defending creature is now Paralyzed.",
                     cost: &[Water], damage: 10,
                     effects: &[Inflict { status: Status::Paralyzed, coin: true }] },
            Attack { name: "Silt Cloud", text: "Your opponent can't play trainer cards during their next turn.",
                     cost: &[Water, Colorless], damage: 10, effects: &[TrainerLock] },
        ],
        power: None,
    }),
    creature("Brookhorn", Creature {
        stage: Stage::Stage1, evolves_from: Some("Puddock"), hp: 70, color: Water,
        weakness: Some(Lightning), resistance: None, retreat_cost: 1,
        attacks: &[
            Attack { name: "Whirl",
                     text: "Your opponent switches the defending creature with 1 of their Benched creatures.",
                     cost: &[Water, Colorless], damage: 20,
                     effects: &[SwitchDefender { chooser_is_attacker: false }] },
            Attack { name: "Wave Lash", text: "", cost: &[Water, Water, Colorless], damage: 50, effects: &[] },
        ],
        power: None,
    }),
    creature("Tidemaw", Creature {
        stage: Stage::Stage2, evolves_from: Some("Brookhorn"), hp: 110, color: Water,
        weakness: Some(Lightning), resistance: Some(Fire), retreat_cost: 3,
        attacks: &[
            Attack { name: "Drench", text: "Discard 1 energy card attached to the defending creature.",
                     cost: &[Water, Colorless], damage: 20, effects: &[DiscardDefenderEnergy] },
            Attack { name: "Great Surge",
                     text: "This attack can be used only once while this creature stays in play.",
                     cost: &[Water, Water, Water, Colorless], damage: 100, effects: &[OnceWhileInPlay] },
        ],
        power: Some(Power {
            name: "Mending Rain",
            text: "Once during your turn, you may remove 10 damage from your Active creature.",
            kind: PowerKind::Activated { effects: &[Heal(10)], once_per_turn: true, settled_only: false },
        }),
    }),
    creature("Shellmote", Creature {
        stage: Stage::Basic, evolves_from: None, hp: 60, color: Water,
        weakness: Some(Lightning), resistance: None, retreat_cost: 2,
        attacks: &[
            Attack { name: "Shell Up",
                     text: "Flip a coin. If heads, prevent all damage done to this creature by attacks during your opponent's next turn.",
                     cost: &[Water], damage: 0,
                     effects: &[Protect { shield: Shield::PreventDamage, coin: true }] },
            Attack { name: "Soak Through", text: "The defending creature's weakness is now Water.",
                     cost: &[Water, Water], damage: 20, effects: &[DefenderWeakness(Water)] },
        ],
        power: Some(Power {
            name: "Float Shell",
            text: "While this creature is on your Bench, your Active creature's retreat cost is 1 less.",
            kind: PowerKind::RetreatReducer,
        }),
    }),
    // ----- Tide Mind: Psychic -----
    creature("Wispun", Creature {
        stage: Stage::Basic, evolves_from: None, hp: 40, color: Psychic,
        weakness: Some(Fighting), resistance: None, retreat_cost: 1,
        attacks: &[
            Attack { name: "Lull", text: "The defending creature is now Asleep.",
                     cost: &[Psychic], damage: 0,
                     effects: &[Inflict { status: Status::Asleep, coin: false }] },
            Attack { name: "Hush Chant",
                     text: "Choose 1 of the defending creature's attacks. It can't use that attack during your opponent's next turn.",
                     cost: &[Psychic, Colorless], damage: 10, effects: &[DisableAttack] },
        ],
        power: None,
    }),
    creature("Wispelle", Creature {
        stage: Stage::Stage1, evolves_from: Some("Wispun"), hp: 70, color: Psychic,
        weakness: Some(Fighting), resistance: None, retreat_cost: 1,
        attacks: &[
            Attack { name: "Haunt Fog", text: "The defending creature is now Poisoned.",
                     cost: &[Psychic], damage: 10,
                     effects: &[Inflict { status: Status::Poisoned, coin: false }] },
            Attack { name: "Mind Shatter", text: "Flip a coin. If heads, the defending creature is now Double-poisoned.",
                     cost: &[Psychic, Psychic, Colorless], damage: 30,
                     effects: &[Inflict { status: Status::DoublePoisoned, coin: true }] },
        ],
        power: None,
    }),
    creature("Glimmox", Creature {
        stage: Stage::Basic, evolves_from: None, hp: 50, color: Psychic,
        weakness: Some(Fighting), resistance: None, retreat_cost: 1,
        attacks: &[
            Attack { name: "Daze Beam", text: "Flip a coin. If heads, the defending creature is now Confused.",
                     cost: &[Psychic, Psychic], damage: 20,
                     effects: &[Inflict { status: Status::Confused, coin: true }] },
            Attack { name: "Mirror Gaze", text: "Choose 1 of the defending creature's attacks and use it as this attack.",
                     cost: &[Psychic, Colorless, Colorless], damage: 0, effects: &[CopyAttack] },
        ],
        power: Some(Power {
            name: "Mind Needle",
            text: "Once during your turn, if this creature was in play when your turn began, you may do 10 damage to 1 of your opponent's creatures.",
            kind: PowerKind::Activated { effects: &[Snipe(10)], once_per_turn: true, settled_only: true },
        }),
    }),
    // ----- Shared -----
    creature("Scampit", Creature {
        stage: Stage::Basic, evolves_from: None, hp: 40, color: Colorless,
        weakness: None, resistance: Some(Psychic), retreat_cost: 0,
        attacks: &[Attack { name: "Nibble", text: "", cost: &[Colorless], damage: 10, effects: &[] }],
        power: Some(Power {
            name: "Forage",
            text: "Once during your turn, if this creature was in play when your turn began, you may draw a card.",
            kind: PowerKind::Activated { effects: &[Draw(1)], once_per_turn: true, settled_only: true },
        }),
    }),
    // ----- Trainers -----
    trainer("Trail Compass",
            "Search your deck for a Basic creature card, put it into your hand, then shuffle your deck.",
            &[Search(Find::Basic)]),
    trainer("Lucky Satchel", "Draw 2 cards.", &[Draw(2)]),
    trainer("Moss Poultice", "Remove up to 30 damage from your Active creature.", &[Heal(30)]),
    trainer("Clearwater Vial", "Your Active creature recovers from all conditions.", &[Cure { both: false }]),
    trainer("Relay Baton", "Switch your Active creature with 1 of your Benched creatures.", &[SwitchSelf]),
    trainer("Snare Hook",
            "Choose 1 of your opponent's Benched creatures and switch it with their Active creature.",
            &[SwitchDefender { chooser_is_attacker: true }]),
    trainer("Drain Brush", "Discard 1 energy card attached to your opponent's Active creature.",
            &[DiscardDefenderEnergy]),
    trainer("Spike Charm", "Attach to your Active creature. Its attacks do 10 more damage this turn.",
            &[AttachPlus10]),
    trainer("Bark Shield",
            "Attach to 1 of your creatures. It takes 20 less damage from attacks until the end of your opponent's next turn.",
            &[AttachMinus20]),
    trainer("Recall Horn", "Put a creature card from your discard pile into your hand.",
            &[Recover(Find::Creature)]),
    trainer("Bloom Seed",
            "Put a Stage 2 creature card from your hand onto the matching Basic creature in play. The Basic must have been in play since the start of your turn.",
            &[StageSkip]),
    trainer("Unwind Spray",
            "Your opponent's evolved Active creature returns to its previous stage. Put the top card into your opponent's hand.",
            &[Devolve]),
    // ----- Energy -----
    energy("Grass Energy", EnergyCard::Basic(Grass)),
    energy("Fire Energy", EnergyCard::Basic(Fire)),
    energy("Water Energy", EnergyCard::Basic(Water)),
    energy("Lightning Energy", EnergyCard::Basic(Lightning)),
    energy("Fighting Energy", EnergyCard::Basic(Fighting)),
    energy("Psychic Energy", EnergyCard::Basic(Psychic)),
    energy("Twofold Energy", EnergyCard::DoubleColorless),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::card_id;

    #[test]
    fn set_is_valid() {
        assert_eq!(crate::card::validate_cards(CARDS), Ok(()));
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
