//! Core rules engine for the trading card game: the duel, the card set, the
//! prebuilt decks and the AI. The rest of the game (campaign, world, shop,
//! saves) is in `sixfold-meta`.
//!
//! Pure Rust with no WASM or UI dependencies, so every rule is unit-testable on
//! the host. Rules follow `docs/RULES.md`; tests name the rule IDs they cover.
//!
//! Drive a duel with [`Game::new`], then loop: read [`Game::prompt`], pick one
//! of [`Game::legal_actions`], [`Game::apply`] it, render the returned events.

mod action;
mod card;
mod condition;
mod damage;
mod deck;
mod describe;
mod effect;
mod game;
mod ko;
mod lang;
mod play;
mod player;
mod power;
mod resolve;
mod retreat;
mod rng;
mod set_fr;
mod setup;
mod turn;
mod view;

pub mod ai;
pub mod decks;
pub mod set;
pub mod tutorial;

// The test cards and helpers, also for `sixfold-meta`'s tests (feature
// `testkit`, a dev-dependency only).
#[cfg(any(test, feature = "testkit"))]
#[doc(hidden)]
pub mod fixtures;
#[cfg(any(test, feature = "testkit"))]
#[doc(hidden)]
pub mod testkit;

pub use action::{Action, Event, Prompt, PromptKind, RuleError};
pub use card::{
    can_pay, card, card_id, energy_units, total_units, try_card, validate_cards, Attack, CardDef,
    CardId, CardKind, Color, Creature, EnergyCard, Gear, Power, PowerKind, Rarity, Stage, Units,
};
pub use condition::{ActiveEffects, Condition, Placed, Poison, Shield, Special, Status};
pub use damage::Hit;
pub use deck::{
    deck_errors, hints, validate_deck, Advice, DeckError, Hints, SortBy, DECK_SIZE, MAX_COPIES,
};
pub use describe::summarise_turn;
pub use effect::{Choice, Effect, Find};
pub use game::{Config, End, Game, GameSummary, SetupError, MAX_BENCH};
pub use lang::{fr, lang, set_lang, tr, Lang, T};
pub use player::InPlay;
pub use rng::Rng;
pub use setup::OPENING_HAND;
pub use view::{Side, View};
