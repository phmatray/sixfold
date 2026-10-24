//! What the engine asks (`Prompt`), what players answer (`Action`), what
//! happened (`Event`) and why an action was refused (`RuleError`).
//!
//! Slots: 0 is the Active, 1..=5 the Bench. Hand positions index the hand.

use crate::card::CardId;

/// A pending decision for one player.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prompt {
    pub player: usize,
    pub kind: PromptKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptKind {
    /// Setup: put one Basic from hand as Active (`PlayBasic`). Can't be skipped.
    ChooseActive,
    /// Setup: put Basics on the Bench (`PlayBasic`), then `Done`.
    ChooseBench,
    /// The turn player's free choice of action.
    MainPhase,
    /// Retreat payment: pick attached energies (`DiscardEnergy`, indexes into the
    /// Active's `energy`) until the cost is paid, or `Cancel`.
    ChooseRetreatEnergy { cost: u8, chosen: Vec<usize> },
    /// Retreat: the Benched creature to switch in (`Promote`), or `Cancel`.
    /// `discard` are the energies that will be paid.
    ChooseRetreatTarget { discard: Vec<usize> },
    /// Take a face-down prize by position (`TakePrize`); `count` still to take.
    ChoosePrize { count: u8 },
    /// The Active was knocked out: `Promote` a Benched creature. Can't be cancelled.
    ChooseReplacement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Put a Basic (or Trainer-as-Basic) from hand into play: the Active during
    /// `ChooseActive`, otherwise the Bench.
    PlayBasic(usize),
    Evolve {
        hand: usize,
        slot: usize,
    },
    AttachEnergy {
        hand: usize,
        slot: usize,
    },
    PlayTrainer(usize),
    /// Start a retreat (main phase).
    Retreat,
    DiscardEnergy(usize),
    /// Switch the Benched creature in this slot to the Active spot.
    Promote(usize),
    Cancel,
    Attack(usize),
    TakePrize(usize),
    /// End setup placement, or end the turn (R-TURN-06).
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Shuffled {
        player: usize,
    },
    CardDrawn {
        player: usize,
        card: CardId,
    },
    /// R-SETUP-02: the revealed hand had no Basic.
    Mulligan {
        player: usize,
        hand: Vec<CardId>,
    },
    /// R-SETUP-02: neither player had a Basic; setup starts over.
    SetupRestarted,
    Placed {
        player: usize,
        slot: usize,
        card: CardId,
    },
    PrizesSet {
        player: usize,
        count: u8,
    },
    CoinFlip {
        player: usize,
        heads: bool,
    },
    FirstPlayer {
        player: usize,
    },
    TurnStarted {
        player: usize,
        turn: u32,
    },
    EnergyAttached {
        player: usize,
        slot: usize,
        card: CardId,
    },
    Evolved {
        player: usize,
        slot: usize,
        card: CardId,
    },
    TrainerPlayed {
        player: usize,
        card: CardId,
    },
    EnergyDiscarded {
        player: usize,
        slot: usize,
        card: CardId,
    },
    /// The creature from `slot` became Active; the old Active is now in `slot`.
    Retreated {
        player: usize,
        slot: usize,
    },
    /// R-RETREAT-03: confused retreat flipped tails.
    RetreatFailed {
        player: usize,
    },
    AttackUsed {
        player: usize,
        attack: usize,
    },
    Damage {
        player: usize,
        slot: usize,
        amount: u16,
        weakness: bool,
        resistance: bool,
    },
    KnockedOut {
        player: usize,
        slot: usize,
        card: CardId,
    },
    PrizeTaken {
        player: usize,
        prize: usize,
        card: CardId,
    },
    /// The creature from Bench `slot` replaced a knocked-out Active.
    Promoted {
        player: usize,
        slot: usize,
    },
    TurnEnded {
        player: usize,
    },
    BetweenTurns,
    /// R-TURN-02: the turn player could not draw.
    DeckOut {
        player: usize,
    },
    /// R-TURN-08: the duel restarts with one prize each.
    SuddenDeath,
    GameOver {
        winner: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleError {
    /// The game has ended.
    GameOver,
    /// This action doesn't answer the current prompt.
    NotNow,
    NoSuchCard,
    NoSuchSlot,
    NotABasic,
    BenchFull,
    NotAnEvolution,
    /// R-EVO-01: the target isn't the creature this card evolves from.
    NoMatchingCreature,
    /// R-EVO-02/03/04: the target hasn't been in play since the turn started.
    NotThisTurn,
    EvolutionLocked,
    NotEnergy,
    EnergyAlreadyAttached,
    NotATrainer,
    TrainerLocked,
    DeckEmpty,
    /// The card uses an effect this engine version doesn't implement yet.
    Unsupported,
    AsleepOrParalyzed,
    CantAttack,
    NoAttacks,
    NoSuchAttack,
    NotEnoughEnergy,
    AttackDisabled,
    CantRetreat,
    NoLiveBench,
    TrainerBasicCantRetreat,
    RetreatFailedThisTurn,
    NoSuchEnergy,
    EnergyAlreadyChosen,
    NoSuchPrize,
    PrizeTaken,
}
