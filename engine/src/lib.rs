//! Core rules engine for the trading card game.
//!
//! Pure Rust with no WASM or UI dependencies, so every rule is unit-testable on
//! the host. Rules follow `docs/RULES.md`; tests name the rule IDs they cover.
//!
//! Drive a duel with [`Game::new`], then loop: read [`Game::prompt`], pick one
//! of [`Game::legal_actions`], [`Game::apply`] it, render the returned events.

mod action;
mod card;
mod collection;
mod condition;
mod damage;
mod deck;
mod describe;
mod effect;
mod game;
mod ko;
mod play;
mod player;
mod power;
mod resolve;
mod retreat;
mod rng;
mod setup;
mod turn;
mod view;

pub mod ai;
pub mod decks;
pub mod overworld;
pub mod set;

#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod testkit;

pub use action::{Action, Event, Prompt, PromptKind, RuleError};
pub use card::{
    can_pay, card, card_id, energy_units, total_units, try_card, validate_cards, Attack, CardDef,
    CardId, CardKind, Color, Creature, EnergyCard, Power, PowerKind, Rarity, Stage, Trainer, Units,
};
pub use collection::{starter_collection, Collection, LoadError};
pub use condition::{ActiveEffects, Condition, Placed, Poison, Shield, Special, Status};
pub use damage::Hit;
pub use deck::{validate_deck, DeckError, DECK_SIZE, MAX_COPIES};
pub use effect::{Choice, Effect, Find};
pub use game::{Config, Game, SetupError, MAX_BENCH};
pub use player::InPlay;
pub use setup::OPENING_HAND;
pub use view::{Side, View};
