//! The game around the duel: the campaign and its world, the shop and the
//! boosters, the player's collection and deck lists, profiles, stats and
//! saves. The duel itself (rules, cards, AI) is `sixfold-engine`.
//!
//! Pure Rust with no WASM or UI dependencies, like the engine.

pub mod books;
pub mod booster;
pub mod builder;
pub mod campaign;
pub mod clock;
pub mod doctor;
pub mod duelists;
pub mod endgame;
pub mod lore;
pub mod objective;
pub mod overworld;
pub mod profile;
pub mod save;
pub mod shop;
pub mod stats;
pub mod unlock;

mod collection;
mod decklist;

#[cfg(test)]
mod playtest;

pub use collection::{starter_collection, Collection, LoadError};
pub use decklist::{AddError, DeckList, DeckListError};
