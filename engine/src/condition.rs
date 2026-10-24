//! The Active's condition byte (R-STATUS-01) and the effects attacks leave on it.

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

    /// Asleep or Paralyzed: can't attack or retreat (R-STATUS-04).
    pub fn blocks_actions(&self) -> bool {
        matches!(self.special, Some(Special::Asleep | Special::Paralyzed))
    }

    pub fn is_confused(&self) -> bool {
        self.special == Some(Special::Confused)
    }
}

/// Effects an attack left on a player's Active. All of them are cleared when it
/// leaves the Active spot, evolves, or a creature fills an empty Active spot
/// (R-RETREAT-04, R-STATUS-12): see `Player::clear_active_state`.
///
/// RULES: stage 2 adds the remaining fields here (own-turn shields `+E7`, changed
/// weakness/resistance, the doubling flag, last-attack memory).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ActiveEffects {
    /// `+E8`: placed by the opponent's attack; cleared at the end of this
    /// player's next turn (R-TURN-03 step 8).
    pub placed: Option<Placed>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placed {
    /// R-RETREAT-01 (1)
    CantRetreat,
    /// R-TURN-05 attack lock
    CantAttack,
    /// R-TURN-05: only the attack with this index is blocked.
    DisabledAttack(usize),
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
        c.apply_special(Special::Asleep);
        c.apply_special(Special::Confused);
        assert_eq!(c.special, Some(Special::Confused));
    }

    /// R-STATUS-02
    #[test]
    fn poison_on_double_poisoned_becomes_single() {
        let mut c = Condition::default();
        c.apply_poison(Poison::DoublePoisoned);
        c.apply_poison(Poison::Poisoned);
        assert_eq!(c.poison, Poison::Poisoned);
    }

    /// R-STATUS-02
    #[test]
    fn paralysis_keeps_poison() {
        let mut c = Condition::default();
        c.apply_poison(Poison::DoublePoisoned);
        c.apply_special(Special::Paralyzed);
        assert_eq!(c.poison, Poison::DoublePoisoned);
        assert!(c.blocks_actions());
    }
}
