//! Test-only card set: original placeholder cards covering every rule hook.
//! Ids live at `FIRST..` so they never collide with the real set.

use crate::card::*;
use crate::effect::Effect;
use Color::*;

pub const FIRST: u16 = 0xF000;

const fn id(i: u16) -> CardId {
    CardId(FIRST + i)
}

pub const MOSSLING: CardId = id(0);
pub const MOSSBACK: CardId = id(1);
pub const MOSSGIANT: CardId = id(2);
pub const EMBERKIT: CardId = id(3);
pub const PEBBLET: CardId = id(4);
/// Retreat reducer Power.
pub const DRIFTLING: CardId = id(5);
/// Free-attach (Grass) Power.
pub const SPROUTWING: CardId = id(6);
/// Power blocker.
pub const HUSHMOTH: CardId = id(7);
/// Evolution-lock Power.
pub const STILLSTONE: CardId = id(8);
/// Trainer-as-Basic.
pub const STRAW_DECOY: CardId = id(9);
/// Trainer: draw 2.
pub const FIELD_NOTES: CardId = id(10);
pub const GRASS_E: CardId = id(11);
pub const FIRE_E: CardId = id(12);
pub const WATER_E: CardId = id(13);
pub const LIGHTNING_E: CardId = id(14);
pub const FIGHTING_E: CardId = id(15);
pub const PSYCHIC_E: CardId = id(16);
/// The 2-unit colorless special energy.
pub const TWIN_E: CardId = id(17);

const VINE_POKE: Attack = Attack { name: "Vine Poke", text: "", cost: &[Grass], damage: 10, effects: &[] };
const ROOT_SLAM: Attack =
    Attack { name: "Root Slam", text: "", cost: &[Grass, Colorless, Colorless], damage: 30, effects: &[] };
const SINGE: Attack = Attack { name: "Singe", text: "", cost: &[Fire], damage: 10, effects: &[] };
const PEBBLE_TOSS: Attack =
    Attack { name: "Pebble Toss", text: "", cost: &[Fighting, Colorless], damage: 20, effects: &[] };
const BUMP: Attack = Attack { name: "Bump", text: "", cost: &[Colorless], damage: 10, effects: &[] };

const fn power(kind: PowerKind) -> Option<Power> {
    Some(Power { name: "Test Power", text: "", kind })
}

const fn energy(name: &'static str, kind: EnergyCard) -> CardDef {
    CardDef { name, kind: CardKind::Energy(kind) }
}

pub static CARDS: [CardDef; 18] = [
    CardDef {
        name: "Mossling",
        kind: CardKind::Creature(Creature {
            stage: Stage::Basic,
            evolves_from: None,
            hp: 50,
            color: Grass,
            weakness: Some(Fire),
            resistance: None,
            retreat_cost: 1,
            attacks: &[VINE_POKE, ROOT_SLAM],
            power: None,
        }),
    },
    CardDef {
        name: "Mossback",
        kind: CardKind::Creature(Creature {
            stage: Stage::Stage1,
            evolves_from: Some("Mossling"),
            hp: 80,
            color: Grass,
            weakness: Some(Fire),
            resistance: None,
            retreat_cost: 2,
            attacks: &[ROOT_SLAM],
            power: None,
        }),
    },
    CardDef {
        name: "Mossgiant",
        kind: CardKind::Creature(Creature {
            stage: Stage::Stage2,
            evolves_from: Some("Mossback"),
            hp: 120,
            color: Grass,
            weakness: Some(Fire),
            resistance: None,
            retreat_cost: 3,
            attacks: &[ROOT_SLAM],
            power: None,
        }),
    },
    CardDef {
        name: "Emberkit",
        kind: CardKind::Creature(Creature {
            stage: Stage::Basic,
            evolves_from: None,
            hp: 40,
            color: Fire,
            weakness: Some(Water),
            resistance: None,
            retreat_cost: 0,
            attacks: &[SINGE],
            power: None,
        }),
    },
    CardDef {
        name: "Pebblet",
        kind: CardKind::Creature(Creature {
            stage: Stage::Basic,
            evolves_from: None,
            hp: 60,
            color: Fighting,
            weakness: Some(Grass),
            resistance: Some(Lightning),
            retreat_cost: 2,
            attacks: &[PEBBLE_TOSS],
            power: None,
        }),
    },
    CardDef {
        name: "Driftling",
        kind: CardKind::Creature(Creature {
            stage: Stage::Basic,
            evolves_from: None,
            hp: 40,
            color: Psychic,
            weakness: None,
            resistance: None,
            retreat_cost: 1,
            attacks: &[BUMP],
            power: power(PowerKind::RetreatReducer),
        }),
    },
    CardDef {
        name: "Sproutwing",
        kind: CardKind::Creature(Creature {
            stage: Stage::Basic,
            evolves_from: None,
            hp: 40,
            color: Grass,
            weakness: None,
            resistance: None,
            retreat_cost: 1,
            attacks: &[BUMP],
            power: power(PowerKind::FreeAttach(Grass)),
        }),
    },
    CardDef {
        name: "Hushmoth",
        kind: CardKind::Creature(Creature {
            stage: Stage::Basic,
            evolves_from: None,
            hp: 50,
            color: Psychic,
            weakness: None,
            resistance: None,
            retreat_cost: 1,
            attacks: &[BUMP],
            power: power(PowerKind::Blocker),
        }),
    },
    CardDef {
        name: "Stillstone",
        kind: CardKind::Creature(Creature {
            stage: Stage::Basic,
            evolves_from: None,
            hp: 70,
            color: Fighting,
            weakness: None,
            resistance: None,
            retreat_cost: 3,
            attacks: &[BUMP],
            power: power(PowerKind::EvolutionLock),
        }),
    },
    CardDef {
        name: "Straw Decoy",
        kind: CardKind::TrainerBasic(Creature {
            stage: Stage::Basic,
            evolves_from: None,
            hp: 10,
            color: Colorless,
            weakness: None,
            resistance: None,
            retreat_cost: 0,
            attacks: &[],
            power: None,
        }),
    },
    CardDef {
        name: "Field Notes",
        kind: CardKind::Trainer(Trainer { text: "Draw 2 cards.", effects: &[Effect::Draw(2)] }),
    },
    energy("Grass Energy", EnergyCard::Basic(Grass)),
    energy("Fire Energy", EnergyCard::Basic(Fire)),
    energy("Water Energy", EnergyCard::Basic(Water)),
    energy("Lightning Energy", EnergyCard::Basic(Lightning)),
    energy("Fighting Energy", EnergyCard::Basic(Fighting)),
    energy("Psychic Energy", EnergyCard::Basic(Psychic)),
    energy("Twin Energy", EnergyCard::DoubleColorless),
];

/// A legal 60-card deck: 12 Basics, a Stage 1/2 line, 4 trainers, 39 energy.
pub fn deck() -> Vec<CardId> {
    let mut d = Vec::new();
    for (id, n) in [
        (MOSSLING, 4),
        (EMBERKIT, 4),
        (PEBBLET, 4),
        (MOSSBACK, 3),
        (MOSSGIANT, 2),
        (FIELD_NOTES, 4),
        (GRASS_E, 20),
        (FIRE_E, 10),
        (FIGHTING_E, 9),
    ] {
        d.extend(std::iter::repeat_n(id, n));
    }
    d
}
