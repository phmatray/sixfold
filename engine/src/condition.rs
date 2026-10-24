//! The Active's condition byte (R-STATUS-01) and the effects attacks leave on it.

use crate::card::Color;

/// The three conditions that exclude each other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Special {
    Confused,
    Asleep,
    Paralyzed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Poison {
    #[default]
    None,
    Poisoned,
    DoublePoisoned,
}

/// A condition an effect can inflict (R-STATUS-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Confused,
    Asleep,
    Paralyzed,
    Poisoned,
    DoublePoisoned,
}

/// One per player, for the Active only (R-STATUS-01): at most one of
/// Confused / Asleep / Paralyzed, plus an independent poison level. The Bench
/// never has conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Condition {
    pub special: Option<Special>,
    pub poison: Poison,
}

impl Condition {
    /// R-STATUS-02: replaces whichever of the three is present, keeps poison.
    pub fn apply_special(&mut self, s: Special) {
        self.special = Some(s);
    }

    /// R-STATUS-02: overwrites the poison level, keeps the other condition.
    pub fn apply_poison(&mut self, p: Poison) {
        self.poison = p;
    }

    /// R-STATUS-02: `new = (old & mask) | value`.
    pub fn apply(&mut self, s: Status) {
        match s {
            Status::Confused => self.apply_special(Special::Confused),
            Status::Asleep => self.apply_special(Special::Asleep),
            Status::Paralyzed => self.apply_special(Special::Paralyzed),
            Status::Poisoned => self.apply_poison(Poison::Poisoned),
            Status::DoublePoisoned => self.apply_poison(Poison::DoublePoisoned),
        }
    }

    pub fn is_empty(&self) -> bool {
        *self == Condition::default()
    }

    /// Asleep or Paralyzed: can't attack or retreat (R-STATUS-04).
    pub fn blocks_actions(&self) -> bool {
        matches!(self.special, Some(Special::Asleep | Special::Paralyzed))
    }

    pub fn is_confused(&self) -> bool {
        self.special == Some(Special::Confused)
    }
}

/// Effects on a player's Active. All of them are cleared when it leaves the
/// Active spot, evolves, or a creature fills an empty Active spot
/// (R-RETREAT-04, R-STATUS-12): see `Player::clear_active_state`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ActiveEffects {
    /// `+E8`: placed by the opponent's attack; cleared at the end of this
    /// player's next turn (R-TURN-03 step 8).
    pub placed: Option<Placed>,
    /// `+E7`: set by this player's own attack; cleared at the start of this
    /// player's next turn (R-TURN-03 step 1), so it lasts through the
    /// opponent's turn.
    pub shield: Option<Shield>,
    /// `+E9` / `+EA`: changed weakness and resistance, used instead of the
    /// printed ones while set (R-DMG-03).
    pub weakness: Option<Color>,
    pub resistance: Option<Color>,
    /// `+EB` bit 0: the turn this player's "next attack does double damage" was
    /// set. It applies on a later turn and is cleared at the end of that turn
    /// (R-DMG-08).
    pub doubling: Option<u32>,
    /// `+F5`: the conditions the most recent attack inflicted on this Active
    /// (R-STATUS-13).
    pub last_inflicted: Condition,
}

/// Effects the opponent's attack placed on this Active (`+E8`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placed {
    /// R-RETREAT-01 (1)
    CantRetreat,
    /// R-TURN-05 attack lock
    CantAttack,
    /// R-TURN-05: only the attack with this index is blocked.
    DisabledAttack(usize),
    /// R-EFFECT-02 step 2: this Active's attacks need heads first.
    Accuracy,
    /// R-DMG-09 step 4: this Active's attacks do this much less damage.
    Weaken(u16),
}

impl Placed {
    /// R-KO-10: effects that end when the Active that placed them is knocked out.
    /// RULES: R-KO-10 — `+E8` 5/6 map to `CantAttack`, 3/7/$12 to `Weaken`.
    pub fn linked_to_placer(self) -> bool {
        matches!(self, Placed::CantAttack | Placed::Weaken(_))
    }
}

/// The single own-turn protection (`+E7`) an attack put on its own Active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shield {
    /// R-DMG-12: prevent all effects of attacks, damage included.
    PreventAll,
    /// R-DMG-09 (2): attack damage becomes 0; other effects still apply.
    PreventDamage,
    /// R-DMG-09 (2): −10 or −20.
    Reduce(u16),
    /// R-DMG-09 (2): damage below this is prevented (the source uses 40).
    PreventBelow(u16),
    /// R-DMG-09 (2), R-DMG-07
    Halve,
    /// R-KO-09: if an attack knocks this creature out, the attacker is knocked out too.
    CounterKo,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R-STATUS-01
    #[test]
    fn asleep_and_poisoned_coexist() {
        let mut c = Condition::default();
        c.apply_special(Special::Asleep);
        c.apply_poison(Poison::Poisoned);
        assert_eq!(c.special, Some(Special::Asleep));
        assert_eq!(c.poison, Poison::Poisoned);
    }

    /// R-STATUS-01
    #[test]
    fn confused_replaces_asleep() {
        let mut c = Condition::default();
        c.apply(Status::Asleep);
        c.apply(Status::Confused);
        assert_eq!(c.special, Some(Special::Confused));
    }

    /// R-STATUS-02
    #[test]
    fn poison_on_double_poisoned_becomes_single() {
        let mut c = Condition::default();
        c.apply(Status::DoublePoisoned);
        c.apply(Status::Poisoned);
        assert_eq!(c.poison, Poison::Poisoned);
    }

    /// R-STATUS-02
    #[test]
    fn paralysis_keeps_poison() {
        let mut c = Condition::default();
        c.apply(Status::DoublePoisoned);
        c.apply(Status::Paralyzed);
        assert_eq!(c.poison, Poison::DoublePoisoned);
        assert!(c.blocks_actions());
    }
}
