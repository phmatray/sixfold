//! What the engine asks (`Prompt`), what players answer (`Action`), what
//! happened (`Event`) and why an action was refused (`RuleError`).
//!
//! Slots: 0 is the Active, 1..=5 the Bench. Hand positions index the hand.

use crate::card::CardId;
use crate::condition::Status;
use crate::effect::{Choice, Effect};

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
    /// An effect needs a decision: answer `Choose(i)` with an index into
    /// `options`, or `Cancel` when `can_cancel` (the attack, Gear or Power
    /// is then abandoned at no cost, R-GEAR-02).
    Choose {
        effect: Effect,
        options: Vec<Choice>,
        can_cancel: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Put a Basic (or Gear-as-Basic) from hand into play: the Active during
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
    PlayGear(usize),
    /// Start a retreat (main phase).
    Retreat,
    DiscardEnergy(usize),
    /// Switch the Benched creature in this slot to the Active spot.
    Promote(usize),
    Cancel,
    Attack(usize),
    /// Use the activated Power of the creature in this slot (R-POWER-01).
    UsePower(usize),
    /// Answer a `Choose` prompt.
    Choose(usize),
    TakePrize(usize),
    /// End setup placement, or end the turn (R-TURN-06).
    Done,
}

/// Something that happened, in order. Every event about a creature in play
/// carries its `slot` at that moment, its `uid` (stable while it stays in play,
/// see [`InPlay::uid`](crate::InPlay::uid)) and its top card: `card` for
/// `Placed`, `Evolved` and `KnockedOut`, `creature` everywhere else (there
/// `card` is the energy or Gear card that moved).
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
        uid: u32,
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
        uid: u32,
        creature: CardId,
        card: CardId,
    },
    /// `card` is the evolution card, now on top.
    Evolved {
        player: usize,
        slot: usize,
        uid: u32,
        card: CardId,
    },
    GearPlayed {
        player: usize,
        card: CardId,
    },
    EnergyDiscarded {
        player: usize,
        slot: usize,
        uid: u32,
        creature: CardId,
        card: CardId,
    },
    /// The creature from `slot` (`uid`) became Active; the old Active is now in `slot`.
    Retreated {
        player: usize,
        slot: usize,
        uid: u32,
        creature: CardId,
    },
    /// R-RETREAT-03: confused retreat flipped tails.
    RetreatFailed {
        player: usize,
    },
    /// The Active (`uid`, top card `creature`) announced its attack number
    /// `attack`: `card(creature).creature().unwrap().attacks[attack]`.
    AttackUsed {
        player: usize,
        attack: usize,
        uid: u32,
        creature: CardId,
    },
    /// The attack did nothing more: accuracy or confusion tails (R-TURN-05).
    AttackFailed {
        player: usize,
    },
    /// R-DMG-12/13: the defender is protected from this attack's effects.
    AttackNegated {
        player: usize,
    },
    PowerUsed {
        player: usize,
        slot: usize,
        uid: u32,
        creature: CardId,
    },
    ConditionApplied {
        player: usize,
        status: Status,
    },
    /// The Active's conditions changed by a cure, waking up or paralysis ending.
    ConditionRemoved {
        player: usize,
        status: Status,
    },
    /// A lasting effect took hold on this player's Active (or on the player,
    /// for a Gear lock).
    EffectApplied {
        player: usize,
        effect: Effect,
    },
    Healed {
        player: usize,
        slot: usize,
        uid: u32,
        creature: CardId,
        amount: u16,
    },
    /// An effect switched this Active with the creature from `slot` (`uid`).
    Switched {
        player: usize,
        slot: usize,
        uid: u32,
        creature: CardId,
    },
    /// The top card `card` left the creature in `slot` for its owner's hand;
    /// `creature` is the card now on top.
    Devolved {
        player: usize,
        slot: usize,
        uid: u32,
        creature: CardId,
        card: CardId,
    },
    GearAttached {
        player: usize,
        slot: usize,
        uid: u32,
        creature: CardId,
        card: CardId,
    },
    GearDiscarded {
        player: usize,
        slot: usize,
        uid: u32,
        creature: CardId,
        card: CardId,
    },
    HandDiscarded {
        player: usize,
        cards: Vec<CardId>,
    },
    /// A card moved to the hand from the deck (`from_deck`) or the discard pile.
    CardToHand {
        player: usize,
        card: CardId,
        from_deck: bool,
    },
    /// `amount` was done (0 when fully prevented); `hp` is what is left.
    Damage {
        player: usize,
        slot: usize,
        uid: u32,
        creature: CardId,
        amount: u16,
        hp: u16,
        weakness: bool,
        resistance: bool,
    },
    /// The creature left play; its cards are now in the discard pile.
    KnockedOut {
        player: usize,
        slot: usize,
        uid: u32,
        card: CardId,
    },
    PrizeTaken {
        player: usize,
        prize: usize,
        card: CardId,
    },
    /// The creature from Bench `slot` (`uid`) replaced a knocked-out Active.
    Promoted {
        player: usize,
        slot: usize,
        uid: u32,
        creature: CardId,
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
    NotGear,
    GearLocked,
    DeckEmpty,
    /// An effect has nothing to work on (no damage to heal, no card to find…).
    NoTarget,
    /// R-EFFECT-05: once-while-in-play attack already used.
    AlreadyUsed,
    NoPower,
    /// R-POWER-01: a passive Power can't be activated.
    PassivePower,
    /// R-POWER-02/03: a condition on the Active or a blocker stops it.
    PowerBlocked,
    /// R-POWER-04
    PowerAlreadyUsed,
    NoSuchChoice,
    AsleepOrParalyzed,
    /// R-TURN-05: the player who goes first can't attack on turn 0.
    FirstTurn,
    CantAttack,
    NoAttacks,
    NoSuchAttack,
    NotEnoughEnergy,
    AttackDisabled,
    CantRetreat,
    NoLiveBench,
    GearBasicCantRetreat,
    RetreatFailedThisTurn,
    /// R-RETREAT-05: one retreat a turn (Sixfold).
    AlreadyRetreated,
    NoSuchEnergy,
    EnergyAlreadyChosen,
    NoSuchPrize,
    PrizeTaken,
}
