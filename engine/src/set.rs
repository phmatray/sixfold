//! The original card set. Filled in by the set author.
//!
//! `CardId(n)` is the index into [`CARDS`]. All content must be original: no
//! names, text or stats from any published card game.

use crate::card::CardDef;

pub static CARDS: &[CardDef] = &[];

#[cfg(test)]
mod tests {
    #[test]
    fn set_is_valid() {
        assert_eq!(crate::card::validate_cards(super::CARDS), Ok(()));
    }
}
