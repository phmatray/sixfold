//! Test-only card set: original placeholder cards covering every rule hook.
//! Ids live at `FIRST..` so they never collide with the real set.
#![allow(dead_code)]

use crate::card::*;
use crate::condition::{Placed, Shield, Status};
use crate::effect::{Effect, Find};
use Color::*;
use Effect::*;

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
/// Activated, once per turn: Snipe 10.
pub const ZAPLING: CardId = id(18);
/// Activated, once per turn, settled only: heal 20.
pub const MENDLEAF: CardId = id(19);
/// On play: draw 1.
pub const GREETER: CardId = id(20);
/// Stage 1 of Greeter; on play: draw 1.
pub const GREETER_ELDER: CardId = id(21);
/// Condition immunity.
pub const WARDEN: CardId = id(22);
/// Prevents damage of 30 or more.
pub const BULWARK: CardId = id(23);
/// Halves damage.
pub const HALVER: CardId = id(24);
/// Prevents all effects of attacks by evolved creatures.
pub const MIRAGE: CardId = id(25);
/// Coin prevention.
pub const LUCKSHELL: CardId = id(26);
/// Retaliation.
pub const THORNBACK: CardId = id(27);
/// Activated, any number of times: becomes Fire.
pub const SHIFTER: CardId = id(28);
/// Trainer: cure the Active.
pub const TONIC: CardId = id(29);
/// Trainer: heal 20.
pub const SALVE: CardId = id(30);
/// Trainer: +10 trainer.
pub const WHETSTONE: CardId = id(31);
/// Trainer: −20 trainer.
pub const BUCKLER: CardId = id(32);
/// Trainer: switch the opposing Active with a Benched creature you pick.
pub const GALE_FAN: CardId = id(33);
/// Trainer: switch your Active.
pub const SWAP_ROPE: CardId = id(34);
/// Trainer: search the deck for a Basic.
pub const SCOUT_MAP: CardId = id(35);
/// Trainer: a creature from the discard pile to the hand.
pub const OLD_BONES: CardId = id(36);
/// Trainer: discard the hand, draw 7.
pub const FRESH_START: CardId = id(37);
/// Trainer: stage skip.
pub const LEAP_SEED: CardId = id(38);
/// Trainer: devolve the defender.
pub const REWIND_MIST: CardId = id(39);
pub const SPARKLET: CardId = id(40);
pub const FOGLET: CardId = id(41);
pub const BRAMBLET: CardId = id(42);
pub const TIDEPUP: CardId = id(43);
pub const ROCKLET: CardId = id(44);
/// Stage 1 of Foglet.
pub const MOTHWING: CardId = id(45);
pub const PRISMOTH: CardId = id(46);
pub const SHELLBACK: CardId = id(47);
pub const VINEKIN: CardId = id(48);

const VINE_POKE: Attack = Attack {
    name: "Vine Poke",
    text: "",
    cost: &[Grass],
    damage: 10,
    effects: &[],
};
const ROOT_SLAM: Attack = Attack {
    name: "Root Slam",
    text: "",
    cost: &[Grass, Colorless, Colorless],
    damage: 30,
    effects: &[],
};
const SINGE: Attack = Attack {
    name: "Singe",
    text: "",
    cost: &[Fire],
    damage: 10,
    effects: &[],
};
const PEBBLE_TOSS: Attack = Attack {
    name: "Pebble Toss",
    text: "",
    cost: &[Fighting, Colorless],
    damage: 20,
    effects: &[],
};
const BUMP: Attack = Attack {
    name: "Bump",
    text: "",
    cost: &[Colorless],
    damage: 10,
    effects: &[],
};

const fn atk(
    name: &'static str,
    cost: &'static [Color],
    damage: u16,
    effects: &'static [Effect],
) -> Attack {
    Attack {
        name,
        text: "",
        cost,
        damage,
        effects,
    }
}

const fn basic(
    name: &'static str,
    hp: u16,
    color: Color,
    weakness: Option<Color>,
    attacks: &'static [Attack],
    power: Option<Power>,
) -> CardDef {
    CardDef {
        name,
        kind: CardKind::Creature(Creature {
            stage: Stage::Basic,
            evolves_from: None,
            hp,
            color,
            weakness,
            resistance: None,
            retreat_cost: 1,
            attacks,
            power,
        }),
    }
}

const fn trainer(name: &'static str, effects: &'static [Effect]) -> CardDef {
    CardDef {
        name,
        kind: CardKind::Trainer(Trainer { text: "", effects }),
    }
}

const fn activated(
    effects: &'static [Effect],
    once_per_turn: bool,
    settled_only: bool,
) -> Option<Power> {
    power(PowerKind::Activated {
        effects,
        once_per_turn,
        settled_only,
    })
}

const fn power(kind: PowerKind) -> Option<Power> {
    Some(Power {
        name: "Test Power",
        text: "",
        kind,
    })
}

const fn energy(name: &'static str, kind: EnergyCard) -> CardDef {
    CardDef {
        name,
        kind: CardKind::Energy(kind),
    }
}

pub static CARDS: &[CardDef] = &[
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
        kind: CardKind::Trainer(Trainer {
            text: "Draw 2 cards.",
            effects: &[Effect::Draw(2)],
        }),
    },
    energy("Grass Energy", EnergyCard::Basic(Grass)),
    energy("Fire Energy", EnergyCard::Basic(Fire)),
    energy("Water Energy", EnergyCard::Basic(Water)),
    energy("Lightning Energy", EnergyCard::Basic(Lightning)),
    energy("Fighting Energy", EnergyCard::Basic(Fighting)),
    energy("Psychic Energy", EnergyCard::Basic(Psychic)),
    energy("Twin Energy", EnergyCard::DoubleColorless),
    basic(
        "Zapling",
        50,
        Lightning,
        Some(Fighting),
        &[atk(
            "Jolt",
            &[Lightning],
            10,
            &[Inflict {
                status: Status::Paralyzed,
                coin: true,
            }],
        )],
        activated(&[Snipe(10)], true, false),
    ),
    basic(
        "Mendleaf",
        60,
        Grass,
        Some(Fire),
        &[BUMP],
        activated(&[Heal(20)], true, true),
    ),
    basic(
        "Greeter",
        50,
        Water,
        None,
        &[BUMP],
        power(PowerKind::OnPlay(&[Draw(1)])),
    ),
    CardDef {
        name: "Greeter Elder",
        kind: CardKind::Creature(Creature {
            stage: Stage::Stage1,
            evolves_from: Some("Greeter"),
            hp: 80,
            color: Water,
            weakness: None,
            resistance: None,
            retreat_cost: 1,
            attacks: &[BUMP],
            power: power(PowerKind::OnPlay(&[Draw(1)])),
        }),
    },
    basic(
        "Warden",
        60,
        Fighting,
        None,
        &[BUMP],
        power(PowerKind::StatusImmune),
    ),
    basic(
        "Bulwark",
        70,
        Fighting,
        None,
        &[BUMP],
        power(PowerKind::PreventAtLeast(30)),
    ),
    basic(
        "Halver",
        70,
        Water,
        None,
        &[BUMP],
        power(PowerKind::HalveDamage),
    ),
    basic(
        "Mirage",
        50,
        Psychic,
        None,
        &[BUMP],
        power(PowerKind::ShieldFromEvolved),
    ),
    basic(
        "Luckshell",
        50,
        Water,
        None,
        &[BUMP],
        power(PowerKind::CoinPrevention),
    ),
    basic(
        "Thornback",
        60,
        Grass,
        None,
        &[BUMP],
        power(PowerKind::Retaliate),
    ),
    basic(
        "Shifter",
        50,
        Colorless,
        None,
        &[BUMP],
        activated(&[SetOwnColor(Fire)], false, false),
    ),
    trainer("Tonic", &[Cure { both: false }]),
    trainer("Salve", &[Heal(20)]),
    trainer("Whetstone", &[AttachPlus10]),
    trainer("Buckler", &[AttachMinus20]),
    trainer(
        "Gale Fan",
        &[SwitchDefender {
            chooser_is_attacker: true,
        }],
    ),
    trainer("Swap Rope", &[SwitchSelf]),
    trainer("Scout Map", &[Search(Find::Basic)]),
    trainer("Old Bones", &[Recover(Find::Creature)]),
    trainer("Fresh Start", &[DiscardHand, Draw(7)]),
    trainer("Leap Seed", &[StageSkip]),
    trainer("Rewind Mist", &[Devolve]),
    basic(
        "Sparklet",
        50,
        Lightning,
        Some(Fighting),
        &[
            atk(
                "Overload",
                &[Lightning, Colorless],
                30,
                &[
                    Recoil(10),
                    DiscardEnergy {
                        color: Lightning,
                        count: 1,
                    },
                ],
            ),
            atk(
                "Static",
                &[Lightning],
                0,
                &[Inflict {
                    status: Status::Paralyzed,
                    coin: true,
                }],
            ),
        ],
        None,
    ),
    basic(
        "Foglet",
        50,
        Psychic,
        None,
        &[
            atk(
                "Haze",
                &[Psychic],
                0,
                &[
                    Place {
                        placed: Placed::Accuracy,
                        coin: false,
                    },
                    Inflict {
                        status: Status::Confused,
                        coin: false,
                    },
                ],
            ),
            atk("Mimic", &[Psychic, Colorless], 0, &[CopyAttack]),
        ],
        None,
    ),
    basic(
        "Bramblet",
        50,
        Grass,
        Some(Fire),
        &[
            atk(
                "Sting",
                &[Grass],
                10,
                &[Inflict {
                    status: Status::Poisoned,
                    coin: false,
                }],
            ),
            atk(
                "Toxic",
                &[Grass, Grass],
                0,
                &[
                    Inflict {
                        status: Status::DoublePoisoned,
                        coin: false,
                    },
                    DiscardDefenderEnergy,
                ],
            ),
        ],
        None,
    ),
    basic(
        "Tidepup",
        60,
        Water,
        Some(Lightning),
        &[
            atk("Splash", &[Water], 0, &[AddPerHeads { coins: 2, per: 20 }]),
            atk(
                "Undertow",
                &[Water, Water],
                20,
                &[
                    BenchDamage {
                        amount: 10,
                        own: false,
                        opponent: true,
                    },
                    SwitchDefender {
                        chooser_is_attacker: false,
                    },
                ],
            ),
        ],
        None,
    ),
    CardDef {
        name: "Rocklet",
        kind: CardKind::Creature(Creature {
            stage: Stage::Basic,
            evolves_from: None,
            hp: 60,
            color: Fighting,
            weakness: Some(Grass),
            resistance: Some(Lightning),
            retreat_cost: 2,
            attacks: &[
                atk(
                    "Brace",
                    &[Fighting],
                    10,
                    &[
                        Protect {
                            shield: Shield::Reduce(20),
                            coin: false,
                        },
                        TrainerLock,
                    ],
                ),
                atk(
                    "Quake",
                    &[Fighting, Fighting],
                    30,
                    &[
                        BenchDamage {
                            amount: 10,
                            own: true,
                            opponent: true,
                        },
                        DisableAttack,
                    ],
                ),
            ],
            power: None,
        }),
    },
    CardDef {
        name: "Mothwing",
        kind: CardKind::Creature(Creature {
            stage: Stage::Stage1,
            evolves_from: Some("Foglet"),
            hp: 70,
            color: Psychic,
            weakness: None,
            resistance: None,
            retreat_cost: 1,
            attacks: &[
                atk(
                    "Last Stand",
                    &[Psychic],
                    10,
                    &[
                        Protect {
                            shield: Shield::CounterKo,
                            coin: false,
                        },
                        OnceWhileInPlay,
                    ],
                ),
                atk(
                    "Updraft",
                    &[Psychic, Colorless],
                    20,
                    &[SwitchSelf, DoubleNextAttack],
                ),
            ],
            power: None,
        }),
    },
    basic(
        "Prismoth",
        60,
        Psychic,
        None,
        &[
            atk(
                "Recolor",
                &[Colorless],
                0,
                &[DefenderWeakness(Psychic), OwnResistance(Psychic)],
            ),
            atk(
                "Pierce",
                &[Psychic, Colorless],
                20,
                &[IgnoreWeaknessResistance, Snipe(10)],
            ),
        ],
        None,
    ),
    basic(
        "Shellback",
        70,
        Water,
        Some(Lightning),
        &[
            atk(
                "Withdraw",
                &[Water],
                0,
                &[Protect {
                    shield: Shield::PreventAll,
                    coin: true,
                }],
            ),
            atk(
                "Clamp",
                &[Water, Colorless],
                10,
                &[
                    Place {
                        placed: Placed::CantAttack,
                        coin: true,
                    },
                    Cure { both: false },
                ],
            ),
        ],
        None,
    ),
    basic(
        "Vinekin",
        60,
        Grass,
        Some(Fire),
        &[
            atk("Leech", &[Grass], 10, &[Heal(10), DiscardAllEnergy]),
            atk(
                "Wilt",
                &[Grass, Colorless],
                0,
                &[
                    Devolve,
                    Place {
                        placed: Placed::Weaken(20),
                        coin: false,
                    },
                ],
            ),
        ],
        None,
    ),
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

fn build(cards: &[(CardId, usize)]) -> Vec<CardId> {
    let d: Vec<CardId> = cards
        .iter()
        .flat_map(|&(id, n)| std::iter::repeat_n(id, n))
        .collect();
    assert_eq!(d.len(), 60);
    d
}

/// A legal deck with every Power creature and every fixture trainer.
pub fn power_deck() -> Vec<CardId> {
    let mut cards: Vec<(CardId, usize)> = (18..=39).map(|i| (id(i), 1)).collect();
    cards.extend([
        (MOSSLING, 3),
        (MOSSBACK, 2),
        (MOSSGIANT, 2),
        (HUSHMOTH, 1),
        (STRAW_DECOY, 1),
        (FIELD_NOTES, 2),
        (TWIN_E, 2),
        (GRASS_E, 6),
        (WATER_E, 5),
        (LIGHTNING_E, 5),
        (FIGHTING_E, 5),
        (PSYCHIC_E, 4),
    ]);
    build(&cards)
}

/// A legal deck with every fixture attack effect.
pub fn attack_deck() -> Vec<CardId> {
    let mut cards: Vec<(CardId, usize)> = (40..=48).map(|i| (id(i), 3)).collect();
    cards.extend([
        (STILLSTONE, 1),
        (DRIFTLING, 1),
        (SPROUTWING, 1),
        (TONIC, 1),
        (SWAP_ROPE, 1),
        (FIELD_NOTES, 2),
        (TWIN_E, 2),
        (GRASS_E, 6),
        (WATER_E, 5),
        (LIGHTNING_E, 5),
        (FIGHTING_E, 4),
        (PSYCHIC_E, 4),
    ]);
    build(&cards)
}
