//! Core rules engine for the trading card game.
//!
//! Pure Rust with no WASM or UI dependencies, so every rule is unit-testable on
//! the host. Rules follow `docs/RULES.md`; tests name the rule IDs they cover.

mod card;
mod condition;
mod deck;
mod effect;
mod player;
mod rng;

pub mod ai;
pub mod decks;
pub mod set;

#[cfg(test)]
mod fixtures;

pub use card::{
    can_pay, card, energy_units, total_units, try_card, Attack, CardDef, CardId, CardKind, Color, Creature,
    EnergyCard, Power, PowerKind, Stage, Trainer, Units,
};
pub use condition::{ActiveEffects, Condition, Placed, Poison, Special};
pub use deck::{validate_deck, DeckError, DECK_SIZE, MAX_COPIES};
pub use effect::Effect;
pub use player::InPlay;
