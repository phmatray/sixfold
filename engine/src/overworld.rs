//! Overworld progression: items the player owns, story flags and Facets (the
//! world's currency), changed by dialogue effects and read by dialogue
//! conditions. The dialogue itself is data (the world's map JSON); this module
//! is the only parser of its small string language (`docs/WORLD_DESIGN.md`
//! §9.2). Each part is `kind:arg[:arg]`, every argument a name (lowercase
//! ASCII letters, digits and `_`) or a number.
//!
//! Conditions ([`parse_cond`]), optionally negated with `!`, several joined
//! with `&` all holding; an empty condition always holds, a malformed one
//! never does:
//!
//! - `item:<item>`, `flag:<flag>`, `seals:<n>` (at least n seals)
//! - `beat:<id>` (a rival or route duelist beaten at least once), `won:<id>` /
//!   `lost:<id>` (the result of the last duel, until [`Progress::settle`]),
//!   `open:<rival>` (enough seals to duel them)
//! - `coins:<n>` (at least n Facets), `count:<prefix>:<n>` (at least n flags
//!   and items start with the prefix, e.g. `count:found_:30`)
//!
//! Effects ([`parse_effect`]):
//!
//! - `give:<item>`, `take:<item>`, `flag:<flag>`
//! - `duel:<id>` (offer a duel against a rival or a route duelist),
//!   `pack:<booster>` (`ember`, `tide`, `wild`, `grove`, `storm`, `stone` or
//!   `dream`), `card:<slug>` (one card by its English name, lowercase, `_` for
//!   spaces; promos refused), `show:<panel>` (one of [`PANELS`])
//! - `coins:<n>` (gain n Facets), `pay:<n>` (spend n; when short, nothing is
//!   spent and [`Progress::apply_all`] skips the rest of the list)
//! - `emote:<actor>:<emote>`, a cue for the world ([`Cue`]): state is untouched
//!
//! Save string: comma-separated `i:<item>`, `f:<flag>`, `won:<id>` /
//! `lost:<id>`, `g:<beaten>_<deck print>` (a Gauntlet run), `d:<day>` (the
//! last daily duel reward), `c:<n>` (Facets; absent in a save from before
//! them, which then starts with [`STARTING_COINS`]) and `ps:<day>_<mask>`
//! (the shop's daily singles bought that day) tokens. Unknown tokens are
//! skipped.

use std::collections::BTreeSet;

use crate::booster::BOOSTERS;
use crate::campaign::{rival, seals};
use crate::card::{card, CardId};
use crate::trainers::trainer;

/// Facets a new player starts with, and a save from before Facets.
pub const STARTING_COINS: u32 = 100;

/// The UI panels a `show:` effect may open.
pub const PANELS: [&str; 9] = [
    "ladder",
    "shop_hall",
    "trader",
    "press",
    "doctor",
    "curios",
    "inn_shop",
    "map",
    "tailor",
];

/// The bubbles an `emote:` cue may pop over a head.
pub const EMOTES: [&str; 5] = ["bang", "what", "dots", "heart", "note"];

/// Flags named after an NPC who has since been renamed, and their new names.
const RENAMED: [(&str, &str); 3] = [
    ("juniper_gift", "wynne_gift"),
    ("talked_juniper", "talked_wynne"),
    ("talked_flint", "talked_tarn"),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Progress {
    items: BTreeSet<String>,
    flags: BTreeSet<String>,
    /// The last duel (rival or duelist id, won), until its post-duel line played.
    last: Option<(String, bool)>,
    /// The Gauntlet run under way: (rungs beaten, deck fingerprint).
    gauntlet: Option<(usize, u32)>,
    /// The last day whose daily duel reward was claimed.
    daily: Option<u32>,
    /// Facets held.
    coins: u32,
    /// The shop's daily singles bought: (day, one bit per stock slot).
    stock: Option<(u32, u8)>,
}

impl Default for Progress {
    /// A new player: nothing done yet, [`STARTING_COINS`] Facets.
    fn default() -> Progress {
        Progress {
            items: BTreeSet::new(),
            flags: BTreeSet::new(),
            last: None,
            gauntlet: None,
            daily: None,
            coins: STARTING_COINS,
            stock: None,
        }
    }
}

/// One test of a condition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Test {
    Item(String),
    Flag(String),
    Seals(usize),
    /// A rival or route duelist, beaten at least once.
    Beat(String),
    /// A rival the seals let the player duel.
    Open(String),
    Won(String),
    Lost(String),
    Coins(u32),
    /// At least this many flags and items start with the prefix.
    Count(String, usize),
}

/// A parsed condition: every `(negated, test)` must hold.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cond(pub Vec<(bool, Test)>);

/// A parsed dialogue effect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    Give(String),
    Take(String),
    Flag(String),
    /// A rival or route duelist id.
    Duel(String),
    /// Index into [`BOOSTERS`].
    Pack(usize),
    Show(String),
    Coins(u32),
    Pay(u32),
    Card(CardId),
    Cue(Cue),
}

/// Something for the world to do (an animation, a sound): no state changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cue {
    /// A bubble over an actor's head (an NPC id, or `hero`).
    Emote { actor: String, emote: String },
}

/// What applying an effect did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Applied {
    /// A new item: the UI announces it.
    GotItem(String),
    /// Already owned, an item taken, or a flag: nothing to announce.
    Nothing,
    /// Offer a duel against this rival or route duelist (by id).
    Duel(String),
    /// Grant one pack of this booster (index into [`BOOSTERS`]).
    Pack(usize),
    /// Open a UI panel by name.
    Show(String),
    /// Facets gained.
    Coins(u32),
    /// Facets spent.
    Paid(u32),
    /// Too few Facets for this price: nothing was spent, and
    /// [`Progress::apply_all`] stopped here. The UI says "Not enough Facets".
    Short(u32),
    /// Add one copy of this card to the collection.
    Card(CardId),
    /// Play this cue, then go on.
    Cue(Cue),
}

/// The booster a `pack:` name means: the first word of its name, lowercase.
fn booster(name: &str) -> Option<usize> {
    BOOSTERS.iter().position(|b| {
        b.name
            .en
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

/// Split `kind:name`, checking the name (save tokens).
fn split(s: &str) -> Option<(&str, &str)> {
    let (kind, name) = s.trim().split_once(':')?;
    valid(name).then_some((kind, name))
}

/// Split `kind:arg[:arg...]`, checking every argument.
fn parts(s: &str) -> Option<(&str, Vec<&str>)> {
    let mut it = s.trim().split(':');
    let kind = it.next()?;
    let args: Vec<&str> = it.collect();
    (!args.is_empty() && args.iter().all(|a| valid(a))).then_some((kind, args))
}

/// A rival or a route duelist.
fn duelist(id: &str) -> bool {
    rival(id).is_some() || trainer(id).is_some()
}

/// A card's `card:` name: its English name in lowercase, other characters `_`.
pub fn card_slug(id: CardId) -> String {
    card(id)
        .name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

/// The card a `card:` slug names; never a promo.
pub fn card_by_slug(slug: &str) -> Option<CardId> {
    crate::set::ids().find(|&id| !crate::set::is_promo(id) && card_slug(id) == slug)
}

/// Parse a dialogue condition.
pub fn parse_cond(s: &str) -> Result<Cond, String> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(Cond::default());
    }
    s.split('&')
        .map(|c| {
            let c = c.trim();
            let (negated, c) = match c.strip_prefix('!') {
                Some(rest) => (true, rest),
                None => (false, c),
            };
            parse_test(c)
                .map(|t| (negated, t))
                .ok_or_else(|| format!("unknown dialogue condition {c:?}"))
        })
        .collect::<Result<_, _>>()
        .map(Cond)
}

fn parse_test(c: &str) -> Option<Test> {
    let (kind, args) = parts(c)?;
    let s = |x: &str| x.to_string();
    Some(match (kind, args.as_slice()) {
        ("item", [x]) => Test::Item(s(x)),
        ("flag", [x]) => Test::Flag(s(x)),
        ("seals", [n]) => Test::Seals(n.parse().ok()?),
        ("beat", [id]) if duelist(id) => Test::Beat(s(id)),
        ("won", [id]) if duelist(id) => Test::Won(s(id)),
        ("lost", [id]) if duelist(id) => Test::Lost(s(id)),
        ("open", [id]) if rival(id).is_some() => Test::Open(s(id)),
        ("coins", [n]) => Test::Coins(n.parse().ok()?),
        ("count", [prefix, n]) => Test::Count(s(prefix), n.parse().ok()?),
        _ => return None,
    })
}

/// Parse a dialogue effect.
pub fn parse_effect(e: &str) -> Result<Effect, String> {
    let bad = || format!("unknown dialogue effect {e:?}");
    let (kind, args) = parts(e).ok_or_else(bad)?;
    let num = |n: &str| n.parse::<u32>().map_err(|_| bad());
    let s = |x: &str| x.to_string();
    Ok(match (kind, args.as_slice()) {
        ("give", [x]) => Effect::Give(s(x)),
        ("take", [x]) => Effect::Take(s(x)),
        ("flag", [x]) => Effect::Flag(s(x)),
        ("duel", [id]) if duelist(id) => Effect::Duel(s(id)),
        ("pack", [b]) => {
            Effect::Pack(booster(b).ok_or_else(|| format!("unknown booster in {e:?}"))?)
        }
        ("show", [panel]) if PANELS.contains(panel) => Effect::Show(s(panel)),
        ("coins", [n]) => Effect::Coins(num(n)?),
        ("pay", [n]) => Effect::Pay(num(n)?),
        ("card", [slug]) => Effect::Card(
            card_by_slug(slug).ok_or_else(|| format!("no such card (or a promo) in {e:?}"))?,
        ),
        ("emote", [actor, emote]) if EMOTES.contains(emote) => Effect::Cue(Cue::Emote {
            actor: s(actor),
            emote: s(emote),
        }),
        _ => return Err(bad()),
    })
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

    /// Flags and items whose name starts with `prefix`.
    pub fn count(&self, prefix: &str) -> usize {
        let n = |set: &BTreeSet<String>| set.iter().filter(|s| s.starts_with(prefix)).count();
        n(&self.items) + n(&self.flags)
    }

    /// Facets held.
    pub fn coins(&self) -> u32 {
        self.coins
    }

    pub fn earn(&mut self, n: u32) {
        self.coins = self.coins.saturating_add(n);
    }

    /// Spend `n` Facets; false, and nothing spent, when fewer are held.
    pub fn spend(&mut self, n: u32) -> bool {
        let ok = self.coins >= n;
        if ok {
            self.coins -= n;
        }
        ok
    }

    /// Remove an item; false when it wasn't held.
    pub fn take(&mut self, item: &str) -> bool {
        self.items.remove(item)
    }

    /// The shop's daily singles bought on `day`, one bit per stock slot.
    pub fn stock_bought(&self, day: u32) -> u8 {
        match self.stock {
            Some((d, mask)) if d == day => mask,
            _ => 0,
        }
    }

    pub(crate) fn set_stock_bought(&mut self, day: u32, mask: u8) {
        self.stock = Some((day, mask));
    }

    /// Apply one dialogue effect.
    pub fn apply(&mut self, effect: &str) -> Result<Applied, String> {
        parse_effect(effect).map(|e| self.run(e))
    }

    /// Apply a branch's effects in order. All are parsed first, so a
    /// malformed one changes nothing. A `pay:` the Facets can't cover ends the
    /// list there ([`Applied::Short`]): the effects before it stand, the rest
    /// are skipped.
    pub fn apply_all<S: AsRef<str>>(&mut self, effects: &[S]) -> Result<Vec<Applied>, String> {
        let parsed: Vec<Effect> = effects
            .iter()
            .map(|e| parse_effect(e.as_ref()))
            .collect::<Result<_, _>>()?;
        let mut done = vec![];
        for e in parsed {
            let a = self.run(e);
            let short = matches!(a, Applied::Short(_));
            done.push(a);
            if short {
                break;
            }
        }
        Ok(done)
    }

    /// Apply one parsed effect.
    pub fn run(&mut self, e: Effect) -> Applied {
        match e {
            Effect::Give(item) => {
                if self.items.insert(item.clone()) {
                    Applied::GotItem(item)
                } else {
                    Applied::Nothing
                }
            }
            Effect::Take(item) => {
                self.take(&item);
                Applied::Nothing
            }
            Effect::Flag(flag) => {
                self.flags.insert(flag);
                Applied::Nothing
            }
            Effect::Duel(id) => Applied::Duel(id),
            Effect::Pack(b) => Applied::Pack(b),
            Effect::Show(panel) => Applied::Show(panel),
            Effect::Coins(n) => {
                self.earn(n);
                Applied::Coins(n)
            }
            Effect::Pay(n) if self.spend(n) => Applied::Paid(n),
            Effect::Pay(n) => Applied::Short(n),
            Effect::Card(id) => Applied::Card(id),
            Effect::Cue(c) => Applied::Cue(c),
        }
    }

    /// Does a dialogue condition hold? A malformed one never does.
    pub fn check(&self, cond: &str) -> bool {
        parse_cond(cond).is_ok_and(|c| self.holds(&c))
    }

    /// Does a parsed condition hold?
    pub fn holds(&self, c: &Cond) -> bool {
        c.0.iter().all(|(negated, t)| self.test(t) != *negated)
    }

    fn test(&self, t: &Test) -> bool {
        match t {
            Test::Item(item) => self.has_item(item),
            Test::Flag(flag) => self.has_flag(flag),
            Test::Seals(n) => seals(self) >= *n,
            Test::Beat(id) => self.has_flag(&format!("beat_{id}")),
            Test::Open(id) => rival(id).is_some_and(|r| r.open(self)),
            Test::Won(id) => self.last_duel() == Some((id, true)),
            Test::Lost(id) => self.last_duel() == Some((id, false)),
            Test::Coins(n) => self.coins >= *n,
            Test::Count(prefix, n) => self.count(prefix) >= *n,
        }
    }

    /// The last duel's (rival or duelist id, won), until [`settle`](Self::settle).
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
        let coins = std::iter::once(format!("c:{}", self.coins));
        let stock = self.stock.iter().map(|(d, m)| format!("ps:{d}_{m}"));
        items
            .chain(flags)
            .chain(last)
            .chain(run)
            .chain(daily)
            .chain(coins)
            .chain(stock)
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Read a save string, skipping anything malformed (an old or tampered save
    /// must not crash the game). Flags of renamed NPCs get their new names.
    pub fn decode(s: &str) -> Progress {
        let mut p = Progress::default();
        for token in s.split(',') {
            match split(token) {
                Some(("i", item)) => {
                    p.items.insert(item.to_string());
                }
                Some(("f", flag)) => {
                    let flag = RENAMED
                        .iter()
                        .find(|(old, _)| *old == flag)
                        .map_or(flag, |(_, new)| new);
                    p.flags.insert(flag.to_string());
                }
                Some((kind @ ("won" | "lost"), id)) => p.set_last_duel(id, kind == "won"),
                Some(("g", run)) => {
                    p.gauntlet = run.split_once('_').and_then(|(n, print)| {
                        Some((n.parse().ok()?, u32::from_str_radix(print, 16).ok()?))
                    })
                }
                Some(("d", day)) => p.daily = day.parse().ok(),
                Some(("c", n)) => p.coins = n.parse().unwrap_or(p.coins),
                Some(("ps", s)) => {
                    p.stock = s
                        .split_once('_')
                        .and_then(|(d, m)| Some((d.parse().ok()?, m.parse().ok()?)))
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
        for bad in [
            "",
            "give",
            "give:",
            "give:Shoes",
            "flag:a b",
            "take:",
            "give:a:b",
        ] {
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
    fn conditions_joined_with_and_must_all_hold() {
        let mut p = Progress::default();
        p.apply("flag:met").unwrap();
        assert!(p.check("flag:met & !item:running_shoes"));
        assert!(!p.check("flag:met&item:running_shoes"));
        p.apply("give:running_shoes").unwrap();
        assert!(p.check("flag:met&item:running_shoes"));
        assert!(!p.check("flag:met&") && !p.check("flag:met&weather:sunny"));
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
        assert_eq!(p.encode(), "f:beat_marlo,lost:marlo,c:100");
        assert_eq!(Progress::decode(&p.encode()), p);
        p.settle();
        assert_eq!(p.encode(), "f:beat_marlo,c:100");
    }

    #[test]
    fn save_string_keeps_the_gauntlet_run_and_the_daily_claim() {
        let mut p = Progress::decode("f:beat_warden");
        p.set_gauntlet(Some((3, 0xdead_beef)));
        p.set_daily(20_385);
        let s = p.encode();
        assert_eq!(s, "f:beat_warden,g:3_deadbeef,d:20385,c:100");
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
        assert_eq!(s, "i:running_shoes,f:met,f:talked_wren,c:100");
        assert_eq!(Progress::decode(&s), p);
        assert_eq!(Progress::decode(""), Progress::default());
        assert_eq!(
            Progress::decode("i:running_shoes,,x:y,f:,garbage,f:BAD"),
            Progress::decode("i:running_shoes")
        );
    }

    #[test]
    fn facets_start_at_100_and_ride_in_the_save_string() {
        let mut p = Progress::default();
        assert_eq!(p.coins(), STARTING_COINS);
        // A save from before Facets starts with them too; junk is skipped.
        assert_eq!(Progress::decode("f:beat_bramble").coins(), 100);
        assert_eq!(Progress::decode("c:many").coins(), 100);
        assert_eq!(Progress::decode("c:0").coins(), 0);
        p.earn(25);
        assert!(!p.spend(126) && p.spend(125));
        assert_eq!(p.encode(), "c:0");
        assert_eq!(Progress::decode(&p.encode()), p);
    }

    #[test]
    fn coins_and_pay_effects_and_the_coins_condition() {
        let mut p = Progress::default();
        assert!(p.check("coins:100") && !p.check("coins:101") && p.check("!coins:101"));
        assert_eq!(p.apply("coins:50"), Ok(Applied::Coins(50)));
        assert_eq!(p.apply("pay:120"), Ok(Applied::Paid(120)));
        assert_eq!(p.coins(), 30);
        assert_eq!(p.apply("pay:31"), Ok(Applied::Short(31)));
        assert_eq!(p.coins(), 30, "a short pay spends nothing");
        for bad in [
            "coins:",
            "coins:x",
            "pay:-1",
            "coins:1:2",
            "coins:99999999999",
        ] {
            assert!(p.apply(bad).is_err(), "{bad:?}");
            assert!(!p.check(bad) && !p.check(&format!("!{bad}")), "{bad:?}");
        }
    }

    /// §9.2: a short `pay:` skips itself and everything after it; a
    /// malformed effect anywhere in the list changes nothing.
    #[test]
    fn apply_all_stops_at_a_short_pay_and_checks_first() {
        let mut p = Progress::default();
        let buy = ["flag:asked", "pay:150", "pack:ember", "flag:bought"];
        assert_eq!(
            p.apply_all(&buy),
            Ok(vec![Applied::Nothing, Applied::Short(150)])
        );
        assert!(p.has_flag("asked") && !p.has_flag("bought") && p.coins() == 100);
        p.earn(50);
        assert_eq!(
            p.apply_all(&buy),
            Ok(vec![
                Applied::Nothing,
                Applied::Paid(150),
                Applied::Pack(0),
                Applied::Nothing
            ])
        );
        assert!(p.has_flag("bought") && p.coins() == 0);
        let before = p.clone();
        assert!(p.apply_all(&["coins:10", "duel:nobody"]).is_err());
        assert_eq!(p, before);
        assert_eq!(p.apply_all::<&str>(&[]), Ok(vec![]));
        let owned: Vec<String> = vec!["give:kite_line".into()];
        assert_eq!(
            p.apply_all(&owned),
            Ok(vec![Applied::GotItem("kite_line".into())])
        );
    }

    #[test]
    fn take_removes_an_item_for_quest_hand_ins() {
        let mut p = Progress::default();
        p.apply("give:kite_line").unwrap();
        assert_eq!(p.apply("take:kite_line"), Ok(Applied::Nothing));
        assert!(!p.has_item("kite_line"));
        assert_eq!(p.apply("take:kite_line"), Ok(Applied::Nothing));
    }

    #[test]
    fn count_reads_flags_and_items_by_prefix() {
        let mut p = Progress::default();
        assert!(p.check("count:found_:0") && !p.check("count:found_:1"));
        for e in [
            "flag:found_w_log",
            "flag:found_s_pier",
            "give:curio_thimble",
            "flag:found",
        ] {
            p.apply(e).unwrap();
        }
        assert_eq!(p.count("found_"), 2);
        assert!(p.check("count:found_:2") && !p.check("count:found_:3"));
        assert!(p.check("count:curio_:1&!count:curio_:2"));
        for bad in [
            "count:found_",
            "count:found_:x",
            "count::2",
            "count:Found:1",
        ] {
            assert!(!p.check(bad), "{bad:?}");
        }
    }

    #[test]
    fn card_effects_take_a_slug_and_refuse_promos() {
        use crate::set::{MOSS_POULTICE, RECALL_HORN};
        let mut p = Progress::default();
        assert_eq!(card_slug(MOSS_POULTICE), "moss_poultice");
        assert_eq!(p.apply("card:recall_horn"), Ok(Applied::Card(RECALL_HORN)));
        assert_eq!(
            p.apply("card:moss_poultice"),
            Ok(Applied::Card(MOSS_POULTICE))
        );
        // Thistlewick is Bramble's promo.
        for bad in [
            "card:thistlewick",
            "card:nothing_here",
            "card:Moss_Poultice",
        ] {
            assert!(p.apply(bad).is_err(), "{bad:?}");
        }
        // Every non-promo card has a slug that finds it again.
        for id in crate::set::ids().filter(|&id| !crate::set::is_promo(id)) {
            assert_eq!(card_by_slug(&card_slug(id)), Some(id));
        }
    }

    #[test]
    fn show_names_a_known_panel_and_emote_is_a_cue() {
        let mut p = Progress::default();
        for panel in PANELS {
            assert_eq!(
                p.apply(&format!("show:{panel}")),
                Ok(Applied::Show(panel.into()))
            );
        }
        assert!(p.apply("show:inventory").is_err());
        assert_eq!(
            p.apply("emote:t_hazel:bang"),
            Ok(Applied::Cue(Cue::Emote {
                actor: "t_hazel".into(),
                emote: "bang".into()
            }))
        );
        assert!(p.apply("emote:hero:wave").is_err() && p.apply("emote:hero").is_err());
        assert_eq!(p, Progress::default(), "cues never change state");
    }

    #[test]
    fn duels_and_results_take_route_duelists_too() {
        let mut p = Progress::default();
        assert_eq!(p.apply("duel:t_tobin"), Ok(Applied::Duel("t_tobin".into())));
        assert!(!p.check("beat:t_tobin") && p.check("!beat:t_tobin"));
        crate::trainers::record(crate::trainers::trainer("t_tobin").unwrap(), true, &mut p);
        assert!(p.check("beat:t_tobin") && p.check("won:t_tobin") && !p.check("lost:t_tobin"));
        // A duelist has no seal gate.
        assert!(!p.check("open:t_tobin") && !p.check("!open:t_tobin"));
        assert!(parse_cond("beat:t_nobody").is_err() && parse_effect("duel:t_nobody").is_err());
    }

    #[test]
    fn parsed_conditions_and_effects() {
        assert_eq!(parse_cond(" "), Ok(Cond::default()));
        assert_eq!(
            parse_cond("seals:3 & !flag:x"),
            Ok(Cond(vec![
                (false, Test::Seals(3)),
                (true, Test::Flag("x".into()))
            ]))
        );
        assert_eq!(
            parse_cond("count:beat_t_:20"),
            Ok(Cond(vec![(false, Test::Count("beat_t_".into(), 20))]))
        );
        assert_eq!(parse_effect("pack:storm"), Ok(Effect::Pack(4)));
        assert_eq!(parse_effect("pay:150"), Ok(Effect::Pay(150)));
        assert!(parse_cond("flag:a&&flag:b").is_err() && parse_effect("").is_err());
    }

    /// Wynne and Tarn were Juniper and Flint before 1.2: their flags follow.
    #[test]
    fn renamed_npc_flags_are_carried_over() {
        let p = Progress::decode("f:juniper_gift,f:talked_juniper,f:talked_flint,f:talked_wren");
        assert!(
            p.has_flag("wynne_gift") && p.has_flag("talked_wynne") && p.has_flag("talked_tarn")
        );
        assert!(!p.has_flag("juniper_gift") && p.has_flag("talked_wren"));
        assert_eq!(
            p.encode(),
            "f:talked_tarn,f:talked_wren,f:talked_wynne,f:wynne_gift,c:100"
        );
    }
}
