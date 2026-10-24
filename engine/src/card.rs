//! Card model. Cards are static data looked up by `CardId` (see [`card`]).
//!
//! Everything a card does is data: attacks and Powers carry lists of [`Effect`]
//! primitives that the engine dispatches, so a card set is plain `static` tables.

use crate::effect::Effect;

/// The six colours plus colorless. `Colorless` must stay last: its index is also
/// the colorless energy bucket (R-ENERGY-03).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    Grass,
    Fire,
    Water,
    Lightning,
    Fighting,
    Psychic,
    Colorless,
}

impl Color {
    pub const ALL: [Color; 7] = [
        Color::Grass,
        Color::Fire,
        Color::Water,
        Color::Lightning,
        Color::Fighting,
        Color::Psychic,
        Color::Colorless,
    ];
}

/// Index into the card registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CardId(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Basic,
    Stage1,
    Stage2,
}

#[derive(Debug)]
pub struct CardDef {
    pub name: &'static str,
    pub kind: CardKind,
}

#[derive(Debug)]
pub enum CardKind {
    Creature(Creature),
    Energy(EnergyCard),
    Trainer(Trainer),
    /// A trainer card that is put into play as a Basic creature. In hand it is a
    /// trainer: it never counts as a Basic for mulligans or deck rules
    /// (R-SETUP-03, R-DECK-02), can't retreat (R-RETREAT-01) and gives no prize
    /// (R-KO-01). Played with `Action::PlayBasic`.
    TrainerBasic(Creature),
}

#[derive(Debug)]
pub struct Creature {
    pub stage: Stage,
    /// Name of the creature this card evolves from (R-EVO-01). `None` for Basics.
    pub evolves_from: Option<&'static str>,
    pub hp: u16,
    pub color: Color,
    pub weakness: Option<Color>,
    pub resistance: Option<Color>,
    pub retreat_cost: u8,
    pub attacks: &'static [Attack],
    pub power: Option<Power>,
}

#[derive(Debug)]
pub struct Attack {
    pub name: &'static str,
    /// Rules text shown in the UI (original wording).
    pub text: &'static str,
    /// One entry per energy symbol, e.g. `&[Fire, Colorless]`.
    pub cost: &'static [Color],
    pub damage: u16,
    pub effects: &'static [Effect],
}

#[derive(Debug)]
pub struct Power {
    pub name: &'static str,
    pub text: &'static str,
    pub kind: PowerKind,
}

/// The generic Power hooks the engine knows (R-POWER-07). Every passive hook
/// works only while the Power works: on the Bench always, as the Active only
/// without Confused / Asleep / Paralyzed, and never while a blocker is in play
/// (R-POWER-03, R-STATUS-05).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PowerKind {
    /// Energy of this colour attached to a creature of this colour doesn't use up
    /// the turn's attachment (R-ENERGY-02).
    FreeAttach(Color),
    /// −1 retreat cost for the owner's Active while on the owner's Bench (R-RETREAT-02).
    RetreatReducer,
    /// Nobody can evolve while this is in play (R-EVO-09).
    EvolutionLock,
    /// All other Powers stop working (R-POWER-03).
    Blocker,
    /// The holder can't receive conditions (R-STATUS-03).
    StatusImmune,
    /// Damage of at least this much to the holder is prevented (R-DMG-09 (3), R-DMG-04).
    PreventAtLeast(u16),
    /// Damage to the holder is halved, rounded down to 10 (R-DMG-09 (3), R-DMG-07).
    HalveDamage,
    /// All effects of attacks by evolved creatures, damage included, are
    /// prevented on the holder (R-DMG-12).
    ShieldFromEvolved,
    /// Flip when attack damage is done to the holder: heads prevents it (R-DMG-13).
    CoinPrevention,
    /// When the holder takes damage that isn't from a Power, the other Active
    /// takes a flat 10 (R-DMG-14).
    Retaliate,
    /// Runs when the holder is put on the Bench or evolves (R-POWER-06).
    OnPlay(&'static [Effect]),
    /// Used from the menu (R-POWER-01, R-POWER-04/05).
    Activated {
        effects: &'static [Effect],
        once_per_turn: bool,
        settled_only: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyCard {
    /// One unit of a colour (never `Colorless`).
    Basic(Color),
    /// The special energy: two colorless units (R-ENERGY-03).
    DoubleColorless,
}

#[derive(Debug)]
pub struct Trainer {
    pub text: &'static str,
    pub effects: &'static [Effect],
}

impl CardDef {
    /// The creature data of a creature or a Trainer-as-Basic.
    pub fn creature(&self) -> Option<&Creature> {
        match &self.kind {
            CardKind::Creature(c) | CardKind::TrainerBasic(c) => Some(c),
            _ => None,
        }
    }

    /// A real Basic creature: counts for mulligans and deck rules.
    pub fn is_basic(&self) -> bool {
        matches!(&self.kind, CardKind::Creature(c) if c.stage == Stage::Basic)
    }

    /// May be put into play as a Basic: a Basic or a Trainer-as-Basic (R-SETUP-04).
    pub fn plays_as_basic(&self) -> bool {
        self.is_basic() || matches!(self.kind, CardKind::TrainerBasic(_))
    }
}

/// Look up a card. Panics on an id outside the registry; ids from
/// [`validate_deck`](crate::validate_deck)-checked decks are always valid.
pub fn card(id: CardId) -> &'static CardDef {
    try_card(id).unwrap_or_else(|| panic!("unknown card id {}", id.0))
}

/// A card with this name, for rules that follow "evolves from" names (R-EVO-10).
pub(crate) fn find_named(name: &str) -> Option<&'static CardDef> {
    #[cfg(test)]
    if let Some(c) = crate::fixtures::CARDS.iter().find(|c| c.name == name) {
        return Some(c);
    }
    crate::set::CARDS.iter().find(|c| c.name == name)
}

pub fn try_card(id: CardId) -> Option<&'static CardDef> {
    #[cfg(test)]
    if id.0 >= crate::fixtures::FIRST {
        return crate::fixtures::CARDS.get((id.0 - crate::fixtures::FIRST) as usize);
    }
    crate::set::CARDS.get(id.0 as usize)
}

/// Attached energy units per colour bucket, indexed by `Color as usize`.
pub type Units = [u8; 7];

/// R-ENERGY-03: a basic energy is 1 unit of its colour; the special energy is 2
/// colorless units.
pub fn energy_units(cards: &[CardId]) -> Units {
    let mut units = [0; 7];
    for &id in cards {
        match card(id).kind {
            CardKind::Energy(EnergyCard::Basic(c)) => units[c as usize] += 1,
            CardKind::Energy(EnergyCard::DoubleColorless) => units[Color::Colorless as usize] += 2,
            _ => {}
        }
    }
    units
}

pub fn total_units(units: &Units) -> u32 {
    units.iter().map(|&u| u as u32).sum()
}

/// R-ENERGY-04: every typed requirement is met by its own colour, and what is
/// left over covers the colorless part. Only sufficiency is checked.
pub fn can_pay(units: &Units, cost: &[Color]) -> bool {
    let colorless = Color::Colorless as usize;
    let mut need = [0u8; 7];
    for &c in cost {
        need[c as usize] += 1;
    }
    if (0..colorless).any(|i| units[i] < need[i]) {
        return false;
    }
    let typed: u32 = need[..colorless].iter().map(|&n| n as u32).sum();
    total_units(units) - typed >= need[colorless] as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::*;
    use Color::*;

    /// R-ENERGY-03
    #[test]
    fn special_colorless_energy_counts_two_units() {
        let u = energy_units(&[TWIN_E, GRASS_E]);
        assert_eq!(u[Colorless as usize], 2);
        assert_eq!(u[Grass as usize], 1);
        assert_eq!(total_units(&u), 3);
    }

    /// R-ENERGY-04
    #[test]
    fn attack_cost_colorless_paid_by_surplus() {
        let u = energy_units(&[GRASS_E, GRASS_E, FIRE_E]);
        assert!(can_pay(&u, &[Grass, Colorless, Colorless]));
        assert!(!can_pay(&u, &[Grass, Colorless, Colorless, Colorless]));
        assert!(!can_pay(&u, &[Grass, Grass, Grass]));
    }

    /// R-ENERGY-04
    #[test]
    fn special_energy_cannot_pay_typed_cost() {
        let u = energy_units(&[TWIN_E]);
        assert!(!can_pay(&u, &[Fire]));
        assert!(can_pay(&u, &[Colorless, Colorless]));
        let u = energy_units(&[TWIN_E, FIRE_E]);
        assert!(can_pay(&u, &[Fire, Colorless, Colorless]));
    }
}
