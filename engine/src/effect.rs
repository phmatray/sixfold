//! Effect primitives: the data cards use to describe what attacks, Powers and
//! trainers do (R-EFFECT-01/04). The engine dispatches them; cards hold no code.
//!
//! Each effect runs at a fixed pipeline phase (see [`Effect::phase`]); effects
//! that need a decision ask for it in the selection step before anything is
//! paid (`PromptKind::Choose`). "Defender" means the opponent's Active and
//! "self" the creature using the attack or Power (the own Active for trainers).

use crate::card::{CardId, Color};
use crate::condition::{Placed, Shield, Status};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    // ----- Damage -----
    /// Flip `coins` coins; the attack does `per` more damage for each heads.
    /// With 0 printed damage this sets the damage (R-EFFECT-04).
    AddPerHeads { coins: u8, per: u16 },
    /// This attack's damage skips weakness and resistance (R-DMG-10).
    IgnoreWeaknessResistance,
    /// Self's attack on its owner's next turn does double damage (R-DMG-08).
    DoubleNextAttack,
    /// After damage, self takes this much, with its own modifiers (R-DMG-05).
    Recoil(u16),
    /// Effect damage to every Benched creature on the chosen sides, with no
    /// weakness or resistance (R-DMG-04, R-EFFECT-04).
    BenchDamage {
        amount: u16,
        own: bool,
        opponent: bool,
    },
    /// Effect damage to one opposing creature the player picks (R-DMG-04).
    /// From a Power it is Power damage.
    Snipe(u16),

    // ----- Conditions (R-STATUS-02/03) -----
    /// Inflict a condition on the defender, always or on heads. Decided before
    /// damage, applied after it.
    Inflict { status: Status, coin: bool },
    /// Inflict a condition on self; ignores immunity and negation (R-STATUS-14).
    InflictSelf(Status),
    /// Clear the own Active's conditions (both Actives with `both`). As a
    /// trainer or Power it needs a condition to clear (R-STATUS-12).
    Cure { both: bool },

    // ----- Shields and effects on the defender -----
    /// Protect self until the end of the opponent's next turn, always or on
    /// heads (`+E7`, R-DMG-09/12, R-KO-09).
    Protect { shield: Shield, coin: bool },
    /// Place an effect on the defender for its owner's next turn, always or on
    /// heads (`+E8`: can't attack, can't retreat, accuracy coin, weaken).
    Place { placed: Placed, coin: bool },
    /// The player picks one of the defender's attacks; it can't be used on its
    /// owner's next turn (R-TURN-05).
    DisableAttack,
    /// The opponent can't play trainers on their next turn; set even when the
    /// attack was negated (R-TRAINER-04).
    TrainerLock,
    /// Change the defender's weakness (R-DMG-03).
    DefenderWeakness(Color),
    /// Change self's resistance (R-DMG-03).
    OwnResistance(Color),

    // ----- Energy -----
    /// Cost: discard `count` basic energy of `color` from self; `Colorless`
    /// means any energy, most recently attached first.
    DiscardEnergy { color: Color, count: u8 },
    /// Cost: discard every energy attached to self.
    DiscardAllEnergy,
    /// The player picks one energy on the defender and discards it.
    DiscardDefenderEnergy,

    // ----- Moving creatures -----
    /// Switch the defender with a Benched creature: picked by the player, or
    /// by its owner after the announcement. With an empty opposing Bench only
    /// the switch is skipped (R-EFFECT-04).
    SwitchDefender { chooser_is_attacker: bool },
    /// Switch self with one of the player's Benched creatures (R-RETREAT-04).
    SwitchSelf,
    /// Heal self (the own Active), capped at max HP. As a trainer or Power it
    /// needs damage to heal (R-EFFECT-04).
    Heal(u16),
    /// The defender returns to its previous stage; the top card goes to its
    /// owner's hand (R-EVO-12).
    Devolve,
    /// Trainer: put a Stage 2 from hand straight onto a settled Basic (R-EVO-10).
    StageSkip,

    // ----- Cards -----
    /// Draw up to N cards. Clamped to the deck, never a loss; playable on an
    /// empty deck, where it draws nothing (R-TURN-02).
    Draw(u8),
    /// Discard the whole hand.
    DiscardHand,
    /// Put a matching card from the deck into the hand, then shuffle
    /// (R-ZONE-03/04). Refused on an empty deck (R-TURN-02).
    Search(Find),
    /// Put a matching card from the discard pile into the hand (R-ZONE-05).
    Recover(Find),
    /// Trainer: this card stays attached to the own Active as a +10 trainer
    /// (R-DMG-01, R-TRAINER-05).
    AttachPlus10,
    /// Trainer: this card stays attached to a creature the player picks as a
    /// −20 trainer (R-DMG-01, R-TRAINER-05).
    AttachMinus20,

    // ----- Attack flags -----
    /// This attack can be used only once while self stays in play (R-EFFECT-05).
    OnceWhileInPlay,
    /// Use one of the defender's attacks instead, through the same pipeline
    /// (R-EFFECT-06).
    CopyAttack,
    /// Power: self's colour becomes this one while the Power works (R-DMG-03).
    SetOwnColor(Color),
}

/// Card filter for searching the deck or the discard pile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Find {
    Any,
    Basic,
    Creature,
    BasicEnergy,
    Trainer,
}

/// One option of a `PromptKind::Choose`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    /// A creature in play.
    Slot { player: usize, slot: usize },
    /// An energy attached to the defender, by index.
    Energy(usize),
    /// One of the defender's attacks, by index.
    Attack(usize),
    /// A card from the deck or discard pile (copies are interchangeable).
    Card(CardId),
    /// Stage-skip: this hand card onto this slot.
    Evolve { hand: usize, slot: usize },
}

/// When an effect runs inside the pipeline (R-EFFECT-01/02).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    /// Phase 6
    Cost,
    /// Phase 3: before damage.
    Before,
    /// Phase 4: after damage.
    After,
}

impl Effect {
    pub(crate) fn phase(&self) -> Phase {
        use Effect::*;
        match self {
            DiscardEnergy { .. } | DiscardAllEnergy | OnceWhileInPlay => Phase::Cost,
            AddPerHeads { .. }
            | IgnoreWeaknessResistance
            | DoubleNextAttack
            | Inflict { .. }
            | InflictSelf(_)
            | Protect { .. }
            | Place { .. }
            | DisableAttack
            | TrainerLock => Phase::Before,
            _ => Phase::After,
        }
    }
}
