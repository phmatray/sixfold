//! Overworld progression: items the player owns and story flags, changed by
//! dialogue effects and read by dialogue conditions. The dialogue itself is data
//! (the world's map JSON); this module only knows the small string language:
//!
//! - effect: `give:<item>` or `flag:<flag>`
//! - condition: `item:<item>` or `flag:<flag>`, optionally negated with `!`;
//!   an empty condition always holds
//! - save string: comma-separated `i:<item>` and `f:<flag>` tokens
//!
//! Names are lowercase ASCII letters, digits and `_`.

use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Progress {
    items: BTreeSet<String>,
    flags: BTreeSet<String>,
}

/// What applying an effect did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Applied {
    /// A new item: the UI announces it.
    GotItem(String),
    /// Already owned, or a flag: nothing to announce.
    Nothing,
}

fn valid(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// Split `kind:name`, checking the name.
fn split(s: &str) -> Option<(&str, &str)> {
    let (kind, name) = s.trim().split_once(':')?;
    valid(name).then_some((kind, name))
}

impl Progress {
    pub fn has_item(&self, item: &str) -> bool {
        self.items.contains(item)
    }

    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags.contains(flag)
    }

    pub fn items(&self) -> impl Iterator<Item = &str> {
        self.items.iter().map(String::as_str)
    }

    /// Apply one dialogue effect.
    pub fn apply(&mut self, effect: &str) -> Result<Applied, String> {
        match split(effect) {
            Some(("give", item)) => Ok(if self.items.insert(item.to_string()) {
                Applied::GotItem(item.to_string())
            } else {
                Applied::Nothing
            }),
            Some(("flag", flag)) => {
                self.flags.insert(flag.to_string());
                Ok(Applied::Nothing)
            }
            _ => Err(format!("unknown dialogue effect {effect:?}")),
        }
    }

    /// Does a dialogue condition hold? A malformed one never does.
    pub fn check(&self, cond: &str) -> bool {
        let cond = cond.trim();
        if cond.is_empty() {
            return true;
        }
        let (negate, cond) = match cond.strip_prefix('!') {
            Some(rest) => (true, rest),
            None => (false, cond),
        };
        let held = match split(cond) {
            Some(("item", item)) => self.has_item(item),
            Some(("flag", flag)) => self.has_flag(flag),
            _ => return false,
        };
        held != negate
    }

    /// The first branch whose condition holds.
    pub fn pick<'a>(&self, conds: impl IntoIterator<Item = &'a str>) -> Option<usize> {
        conds.into_iter().position(|c| self.check(c))
    }

    /// Compact save string, stable across runs (sorted).
    pub fn encode(&self) -> String {
        let items = self.items.iter().map(|i| format!("i:{i}"));
        let flags = self.flags.iter().map(|f| format!("f:{f}"));
        items.chain(flags).collect::<Vec<_>>().join(",")
    }

    /// Read a save string, skipping anything malformed (an old or tampered save
    /// must not crash the game).
    pub fn decode(s: &str) -> Progress {
        let mut p = Progress::default();
        for token in s.split(',') {
            match split(token) {
                Some(("i", item)) => {
                    p.items.insert(item.to_string());
                }
                Some(("f", flag)) => {
                    p.flags.insert(flag.to_string());
                }
                _ => {}
            }
        }
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn giving_an_item_announces_it_once() {
        let mut p = Progress::default();
        assert_eq!(
            p.apply("give:running_shoes"),
            Ok(Applied::GotItem("running_shoes".into()))
        );
        assert_eq!(p.apply("give:running_shoes"), Ok(Applied::Nothing));
        assert!(p.has_item("running_shoes"));
    }

    #[test]
    fn flags_are_set_quietly() {
        let mut p = Progress::default();
        assert_eq!(p.apply("flag:talked_wren"), Ok(Applied::Nothing));
        assert!(p.has_flag("talked_wren"));
        assert!(!p.has_item("talked_wren"));
    }

    #[test]
    fn malformed_effects_are_errors() {
        let mut p = Progress::default();
        for bad in ["", "give", "give:", "take:shoes", "give:Shoes", "flag:a b"] {
            assert!(p.apply(bad).is_err(), "{bad:?}");
        }
        assert_eq!(p, Progress::default());
    }

    #[test]
    fn conditions_read_items_and_flags_with_negation() {
        let mut p = Progress::default();
        assert!(p.check(""));
        assert!(!p.check("item:running_shoes"));
        assert!(p.check("!item:running_shoes"));
        p.apply("give:running_shoes").unwrap();
        p.apply("flag:met").unwrap();
        assert!(p.check("item:running_shoes"));
        assert!(!p.check("!item:running_shoes"));
        assert!(p.check(" flag:met "));
        assert!(!p.check("flag:other"));
        // Malformed conditions never hold, negated or not.
        assert!(!p.check("weather:sunny"));
        assert!(!p.check("!weather:sunny"));
    }

    #[test]
    fn pick_takes_the_first_branch_that_holds() {
        let mut p = Progress::default();
        let branches = ["!item:running_shoes", ""];
        assert_eq!(p.pick(branches), Some(0));
        p.apply("give:running_shoes").unwrap();
        assert_eq!(p.pick(branches), Some(1));
        assert_eq!(p.pick(["flag:nope"]), None);
    }

    #[test]
    fn save_string_round_trips_and_skips_junk() {
        let mut p = Progress::default();
        p.apply("give:running_shoes").unwrap();
        p.apply("flag:talked_wren").unwrap();
        p.apply("flag:met").unwrap();
        let s = p.encode();
        assert_eq!(s, "i:running_shoes,f:met,f:talked_wren");
        assert_eq!(Progress::decode(&s), p);
        assert_eq!(Progress::decode(""), Progress::default());
        assert_eq!(
            Progress::decode("i:running_shoes,,x:y,f:,garbage,f:BAD"),
            Progress::decode("i:running_shoes")
        );
    }
}
