//! Overworld progression: items the player owns and story flags, changed by
//! dialogue effects and read by dialogue conditions. The dialogue itself is data
//! (the world's map JSON); this module only knows the small string language:
//!
//! - effect: `give:<item>`, `flag:<flag>`, `duel:<rival>` (offer a campaign
//!   duel) or `pack:<booster>` (grant a pack: `ember`, `tide`, `wild`, `grove`,
//!   `storm`, `stone` or `dream`)
//! - condition: `item:<item>`, `flag:<flag>`, `seals:<n>` (at least n seals),
//!   `beat:<rival>` (beaten at least once), `open:<rival>` (enough seals to
//!   duel them), `won:<rival>` / `lost:<rival>` (the result of the last
//!   campaign duel, until [`Progress::settle`]), optionally negated with `!`;
//!   an empty condition always holds
//! - effect `show:<panel>` asks the UI to open one of its panels (`ladder`:
//!   the Gauntlet and the daily duel)
//! - save string: comma-separated `i:<item>`, `f:<flag>`, `won:<rival>` /
//!   `lost:<rival>`, `g:<beaten>_<deck print>` (a Gauntlet run) and
//!   `d:<day>` (the last daily duel reward) tokens
//!
//! Names are lowercase ASCII letters, digits and `_`.

use std::collections::BTreeSet;

use crate::booster::BOOSTERS;
use crate::campaign::{rival, seals};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Progress {
    items: BTreeSet<String>,
    flags: BTreeSet<String>,
    /// The last campaign duel (rival id, won), until its post-duel line played.
    last: Option<(String, bool)>,
    /// The Gauntlet run under way: (rungs beaten, deck fingerprint).
    gauntlet: Option<(usize, u32)>,
    /// The last day whose daily duel reward was claimed.
    daily: Option<u32>,
}

/// What applying an effect did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Applied {
    /// A new item: the UI announces it.
    GotItem(String),
    /// Already owned, or a flag: nothing to announce.
    Nothing,
    /// Offer a duel against this campaign rival (by id).
    Duel(String),
    /// Grant one pack of this booster (index into [`BOOSTERS`]).
    Pack(usize),
    /// Open a UI panel by name.
    Show(String),
}

/// The booster a `pack:` name means: the first word of its name, lowercase.
fn booster(name: &str) -> Option<usize> {
    BOOSTERS.iter().position(|b| {
        b.name
            .split(' ')
            .next()
            .is_some_and(|w| w.eq_ignore_ascii_case(name))
    })
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
            Some(("duel", id)) if rival(id).is_some() => Ok(Applied::Duel(id.to_string())),
            Some(("show", panel)) => Ok(Applied::Show(panel.to_string())),
            Some(("pack", name)) => booster(name)
                .map(Applied::Pack)
                .ok_or_else(|| format!("unknown booster in {effect:?}")),
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
            Some(("seals", n)) => match n.parse::<usize>() {
                Ok(n) => seals(self) >= n,
                Err(_) => return false,
            },
            Some((kind @ ("beat" | "open" | "won" | "lost"), id)) => {
                let Some(r) = rival(id) else { return false };
                match kind {
                    "beat" => r.beaten(self),
                    "open" => r.open(self),
                    "won" => self.last_duel() == Some((id, true)),
                    _ => self.last_duel() == Some((id, false)),
                }
            }
            _ => return false,
        };
        held != negate
    }

    /// The last campaign duel's (rival id, won), until [`settle`](Self::settle).
    pub fn last_duel(&self) -> Option<(&str, bool)> {
        self.last.as_ref().map(|(id, won)| (id.as_str(), *won))
    }

    pub(crate) fn set_last_duel(&mut self, id: &str, won: bool) {
        self.last = Some((id.to_string(), won));
    }

    /// The Gauntlet run under way: (rungs beaten, deck fingerprint).
    pub fn gauntlet(&self) -> Option<(usize, u32)> {
        self.gauntlet
    }

    pub(crate) fn set_gauntlet(&mut self, run: Option<(usize, u32)>) {
        self.gauntlet = run;
    }

    /// The last day whose daily duel reward was claimed.
    pub fn daily(&self) -> Option<u32> {
        self.daily
    }

    pub(crate) fn set_daily(&mut self, day: u32) {
        self.daily = Some(day);
    }

    /// The post-duel line played: forget the last result.
    pub fn settle(&mut self) {
        self.last = None;
    }

    /// The first branch whose condition holds.
    pub fn pick<'a>(&self, conds: impl IntoIterator<Item = &'a str>) -> Option<usize> {
        conds.into_iter().position(|c| self.check(c))
    }

    /// Compact save string, stable across runs (sorted).
    pub fn encode(&self) -> String {
        let items = self.items.iter().map(|i| format!("i:{i}"));
        let flags = self.flags.iter().map(|f| format!("f:{f}"));
        let last = self
            .last
            .iter()
            .map(|(id, won)| format!("{}:{id}", if *won { "won" } else { "lost" }));
        let run = self
            .gauntlet
            .iter()
            .map(|(n, print)| format!("g:{n}_{print:08x}"));
        let daily = self.daily.iter().map(|d| format!("d:{d}"));
        items
            .chain(flags)
            .chain(last)
            .chain(run)
            .chain(daily)
            .collect::<Vec<_>>()
            .join(",")
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
                Some((kind @ ("won" | "lost"), id)) => p.set_last_duel(id, kind == "won"),
                Some(("g", run)) => {
                    p.gauntlet = run.split_once('_').and_then(|(n, print)| {
                        Some((n.parse().ok()?, u32::from_str_radix(print, 16).ok()?))
                    })
                }
                Some(("d", day)) => p.daily = day.parse().ok(),
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
    fn duel_and_pack_effects_name_a_rival_or_a_booster() {
        let mut p = Progress::default();
        assert_eq!(p.apply("duel:bramble"), Ok(Applied::Duel("bramble".into())));
        assert_eq!(p.apply("pack:wild"), Ok(Applied::Pack(2)));
        assert_eq!(p.apply("pack:dream"), Ok(Applied::Pack(6)));
        for bad in ["duel:nobody", "pack:gold", "duel:", "pack:Wild"] {
            assert!(p.apply(bad).is_err(), "{bad:?}");
        }
        assert_eq!(p, Progress::default());
    }

    #[test]
    fn campaign_conditions_read_seals_wins_and_the_last_duel() {
        use crate::campaign::{record, rival};
        use crate::profile::Profile;
        let (mut p, mut w) = (Progress::default(), Profile::new_player());
        assert!(p.check("seals:0") && !p.check("seals:1") && p.check("!seals:1"));
        assert!(p.check("open:bramble") && !p.check("open:quill") && !p.check("open:warden"));
        assert!(!p.check("beat:bramble") && !p.check("won:bramble") && !p.check("lost:bramble"));
        record(rival("bramble").unwrap(), false, &mut p, &mut w);
        assert!(p.check("lost:bramble") && !p.check("won:bramble") && !p.check("lost:marlo"));
        record(rival("bramble").unwrap(), true, &mut p, &mut w);
        assert!(p.check("won:bramble") && !p.check("lost:bramble"));
        assert!(p.check("beat:bramble") && p.check("seals:1"));
        p.settle();
        assert!(!p.check("won:bramble") && p.check("beat:bramble"));
        // Unknown rivals and bad counts never hold, negated or not.
        for bad in [
            "beat:nobody",
            "!beat:nobody",
            "seals:x",
            "!seals:x",
            "won:nobody",
        ] {
            assert!(!p.check(bad), "{bad:?}");
        }
    }

    #[test]
    fn save_string_keeps_the_last_duel() {
        let mut p = Progress::decode("f:beat_marlo,lost:marlo");
        assert_eq!(p.last_duel(), Some(("marlo", false)));
        assert_eq!(p.encode(), "f:beat_marlo,lost:marlo");
        assert_eq!(Progress::decode(&p.encode()), p);
        p.settle();
        assert_eq!(p.encode(), "f:beat_marlo");
    }

    #[test]
    fn save_string_keeps_the_gauntlet_run_and_the_daily_claim() {
        let mut p = Progress::decode("f:beat_warden");
        p.set_gauntlet(Some((3, 0xdead_beef)));
        p.set_daily(20_385);
        let s = p.encode();
        assert_eq!(s, "f:beat_warden,g:3_deadbeef,d:20385");
        assert_eq!(Progress::decode(&s), p);
        // Junk in the new tokens is skipped, like any other.
        assert_eq!(Progress::decode("g:x_y,d:soon,g:3"), Progress::default());
        assert_eq!(p.apply("show:ladder"), Ok(Applied::Show("ladder".into())));
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
