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
//!   `open:<rival>` (enough seals to duel them), `earned:<rival>` (enough of
//!   their region's duelists bested), `bested:<rival>:<n>` (at least n of
//!   their region's duelists bested)
//! - `coins:<n>` (at least n Facets), `count:<prefix>:<n>` (at least n flags
//!   and items start with the prefix, e.g. `count:found_:30`)
//! - `time:<phase>` (the clock is in `dawn`, `day`, `dusk` or `night`; see
//!   [`clock`](crate::clock))
//! - `hour:<from>-<to>` (the clock's hour is in `from..to`, wrapping past
//!   midnight when `from > to`: `hour:21-5` is 21:00 to 04:59)
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
//! - `clock:<phase>` (jump the clock to the start of a phase: an inn's rest)
//! - `emote:<actor>:<emote>` and `warp:<map>:<x>:<z>:<facing>` (fade and load
//!   a map at a tile: a ferry, a lift), cues for the world ([`Cue`]): state is
//!   untouched
//! - the cutscene cues (1.4): `walk:<actor>:<path>` (a path of `n`/`e`/`s`/`w`
//!   letters), `face:<actor>:<dir>` (`dir` may be `hero`), `cam:<actor>` / `cam:<x>:<z>` / `cam:hero`
//!   (the camera's look-at target), `wait:<ms>` (0 to [`MAX_WAIT`]),
//!   `sfx:<sound>`, `sfx:bell:<0..5>` (#154: the finale's bell, climbing the
//!   scale) and `music:<track>` (one of [`TRACKS`]); the actor `hero` is the
//!   player
//! - `stone:wake` (#152): the map's seal stone glows in the region's colour
//!   until the scene ends; `stone:seal:<keeper>` (#153): the keeper's face of
//!   it lights, and stays lit
//!
//! Save string: comma-separated `i:<item>`, `f:<flag>`, `won:<id>` /
//! `lost:<id>`, `g:<beaten>_<deck print>` (a Gauntlet run), `d:<day>` (the
//! last daily duel reward), `c:<n>` (Facets; absent in a save from before
//! them, which then starts with [`STARTING_COINS`]) and `ps:<day>_<mask>`
//! (the shop's daily singles bought that day) and `t:<minute>` (the clock;
//! absent at 08:00, and in a save from before the clock) tokens. Unknown
//! tokens are skipped.

use std::collections::BTreeSet;

use crate::booster::BOOSTERS;
use crate::campaign::{rival, seals, KEEPERS, RIVALS, SEALS, STAGED, VOUCHED};
use crate::card::{card, CardId};
use crate::clock::{self, Phase};
use crate::trainers::trainer;

/// Facets a new player starts with, and a save from before Facets.
pub const STARTING_COINS: u32 = 100;

/// The UI panels a `show:` effect may open.
pub const PANELS: [&str; 11] = [
    "ladder",
    "inn",
    "shop_hall",
    "trader",
    "press",
    "doctor",
    "curios",
    "inn_shop",
    "map",
    "tailor",
    // The credits, after the Warden falls (#67): they end in the duel hall.
    "credits",
];

/// The ways a `warp:` cue may leave the hero facing.
const FACINGS: [&str; 4] = ["n", "e", "s", "w"];

/// The bubbles an `emote:` cue may pop over a head.
pub const EMOTES: [&str; 5] = ["bang", "what", "dots", "heart", "note"];
/// Sounds a `sfx:` cue may play (the world's sound names in `web/audio/audio.js`).
pub const SOUNDS: [&str; 6] = ["spark", "chime", "jingle", "gate", "seal", "door"];
/// Tracks a `music:` cue may switch to (the world's tracks in `web/audio/audio.js`,
/// and Rook's theme for his scenes).
pub const TRACKS: [&str; 18] = [
    "overworld",
    "interior",
    "ember",
    "storm",
    "tide",
    "dream",
    "grove",
    "stone",
    "cave",
    "rook",
    // The finale's hall scene (#67): the title's hook, slow and solemn.
    "finale",
    // A keeper's own duel theme, for their entrance at the seal site (#152).
    "duel_rival_ember",
    "duel_rival_storm",
    "duel_rival_tide",
    "duel_rival_dream",
    "duel_rival_grove",
    "duel_rival_stone",
    // The Warden's theme, for the sealed door and her entrance (#154).
    "duel_boss",
];
/// The longest `wait:` a cutscene may hold, in milliseconds.
pub const MAX_WAIT: u16 = 5000;

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
    /// The clock: the game minute of the day (0..1440).
    minute: u16,
}

impl Default for Progress {
    /// A new player: nothing done yet but [`KEEPERS`] (they meet the
    /// keepers' threshold), [`STARTING_COINS`] Facets.
    fn default() -> Progress {
        Progress {
            items: BTreeSet::new(),
            flags: BTreeSet::from([KEEPERS.to_string(), STAGED.to_string()]),
            last: None,
            gauntlet: None,
            daily: None,
            coins: STARTING_COINS,
            stock: None,
            minute: clock::START,
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
    /// A rival whose region's duelists the player has bested enough of.
    Earned(String),
    /// At least this many of a rival's region's duelists bested.
    Bested(String, usize),
    Won(String),
    Lost(String),
    Coins(u32),
    /// At least this many flags and items start with the prefix.
    Count(String, usize),
    /// The clock is in this phase.
    Time(Phase),
    /// The clock's hour is in `from..to`, wrapping past midnight.
    Hour(u8, u8),
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
    /// Jump the clock to the start of this phase.
    Clock(Phase),
    Cue(Cue),
}

/// Something for the world to do (an animation, a sound): no state changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cue {
    /// A bubble over an actor's head (an NPC id, or `hero`).
    Emote { actor: String, emote: String },
    /// Fade out and load `map` with the hero on (`x`, `z`) facing `facing`.
    Warp {
        map: String,
        x: u32,
        z: u32,
        facing: String,
    },
    /// Play one of [`SOUNDS`].
    Sfx(String),
    /// An actor walks a path of `n`/`e`/`s`/`w` letters, one tile each.
    Walk { actor: String, path: String },
    /// An actor turns to face `dir` (`n`/`e`/`s`/`w`, or `hero`: toward wherever the hero stands).
    Face { actor: String, dir: String },
    /// The camera eases its look-at target over (its pitch never changes).
    Cam(CamTarget),
    /// Hold for this many milliseconds (at most [`MAX_WAIT`]).
    Wait(u16),
    /// Switch the music to one of [`TRACKS`] (the map's own comes back after the scene).
    Music(String),
    /// The map's seal stone changes look until the scene ends.
    Stone(StoneCue),
}

/// What a `stone:` cue does to the map's seal stone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StoneCue {
    /// `stone:wake`: its faces glow in the region's colour, with one live light.
    Wake,
    /// `stone:seal:<keeper>`: that seal keeper's face lights and stays lit
    /// (the seal is won on stage, #153).
    Seal(String),
}

/// What a `cam:` cue looks at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CamTarget {
    /// Back to following the hero.
    Hero,
    /// An NPC on the map, by id.
    Actor(String),
    /// The centre of a tile.
    Tile(u32, u32),
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
    /// The clock jumped to the start of this phase (the world fades to it).
    Clock(Phase),
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
    // `hour:` carries a `-`, which `parts` refuses in a name.
    if let Some((from, to)) = c
        .trim()
        .strip_prefix("hour:")
        .and_then(|r| r.split_once('-'))
    {
        let (from, to): (u8, u8) = (from.parse().ok()?, to.parse().ok()?);
        return (from < 24 && to <= 24 && from != to).then_some(Test::Hour(from, to));
    }
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
        ("earned", [id]) if rival(id).is_some() => Test::Earned(s(id)),
        ("bested", [id, n]) if rival(id).is_some() => Test::Bested(s(id), n.parse().ok()?),
        ("coins", [n]) => Test::Coins(n.parse().ok()?),
        ("count", [prefix, n]) => Test::Count(s(prefix), n.parse().ok()?),
        ("time", [p]) => Test::Time(Phase::parse(p)?),
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
        ("sfx", [name]) if SOUNDS.contains(name) => Effect::Cue(Cue::Sfx(s(name))),
        ("sfx", ["bell", n]) if matches!(*n, "0" | "1" | "2" | "3" | "4" | "5") => {
            Effect::Cue(Cue::Sfx(format!("bell:{n}")))
        }
        ("walk", [actor, path]) if path.bytes().all(|b| b"nesw".contains(&b)) => {
            Effect::Cue(Cue::Walk {
                actor: s(actor),
                path: s(path),
            })
        }
        // A direction, `hero`, or a rival to face (the hero turning back to a keeper, #155).
        ("face", [actor, dir])
            if FACINGS.contains(dir) || *dir == "hero" || rival(dir).is_some() =>
        {
            Effect::Cue(Cue::Face {
                actor: s(actor),
                dir: s(dir),
            })
        }
        // A lone number is a tile missing its z, not an actor.
        ("cam", [who]) if !who.bytes().all(|b| b.is_ascii_digit()) => {
            Effect::Cue(Cue::Cam(match *who {
                "hero" => CamTarget::Hero,
                id => CamTarget::Actor(s(id)),
            }))
        }
        ("cam", [x, z]) => Effect::Cue(Cue::Cam(CamTarget::Tile(num(x)?, num(z)?))),
        ("wait", [ms]) => match ms.parse::<u16>() {
            Ok(ms) if ms <= MAX_WAIT => Effect::Cue(Cue::Wait(ms)),
            _ => return Err(format!("wait is 0 to {MAX_WAIT} ms in {e:?}")),
        },
        ("music", [track]) if TRACKS.contains(track) => Effect::Cue(Cue::Music(s(track))),
        ("stone", ["wake"]) => Effect::Cue(Cue::Stone(StoneCue::Wake)),
        ("stone", ["seal", id]) if rival(id).is_some_and(|r| r.seal) => {
            Effect::Cue(Cue::Stone(StoneCue::Seal(s(id))))
        }
        ("clock", [p]) => Effect::Clock(Phase::parse(p).ok_or_else(bad)?),
        ("warp", [map, x, z, f]) if FACINGS.contains(f) => Effect::Cue(Cue::Warp {
            map: s(map),
            x: num(x)?,
            z: num(z)?,
            facing: s(f),
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
            Effect::Clock(p) => {
                self.minute = p.start();
                Applied::Clock(p)
            }
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
            Test::Earned(id) => rival(id).is_some_and(|r| r.earned(self)),
            Test::Bested(id, n) => rival(id).is_some_and(|r| r.bested(self) >= *n),
            Test::Won(id) => self.last_duel() == Some((id, true)),
            Test::Lost(id) => self.last_duel() == Some((id, false)),
            Test::Coins(n) => self.coins >= *n,
            Test::Count(prefix, n) => self.count(prefix) >= *n,
            Test::Time(p) => self.phase() == *p,
            Test::Hour(from, to) => {
                let h = (self.minute / 60) as u8;
                if from < to {
                    (*from..*to).contains(&h)
                } else {
                    h >= *from || h < *to
                }
            }
        }
    }

    /// The clock: the game minute of the day.
    pub fn minute(&self) -> u16 {
        self.minute
    }

    /// Set the clock (the world reports it as it runs); taken modulo a day.
    pub fn set_minute(&mut self, minute: u16) {
        self.minute = minute % clock::DAY;
    }

    pub fn phase(&self) -> Phase {
        clock::phase(self.minute)
    }

    /// Is a gate open (`docs/WORLD_DESIGN.md` §7.1)? While its condition
    /// holds; and a gate that was open stays open while the hero is on any of
    /// its tiles, so it never closes under them (a ford at dusk, a vent at
    /// dawn). A malformed condition keeps a gate shut.
    pub fn gate_open(&self, cond: &str, was_open: bool, under_hero: bool) -> bool {
        self.check(cond) || (was_open && under_hero)
    }

    /// The waymarks touched (flags `way_<id>`), by id: the places fast travel reaches.
    pub fn waymarks(&self) -> impl Iterator<Item = &str> {
        self.flags.iter().filter_map(|f| f.strip_prefix("way_"))
    }

    /// Has the hero been to this outdoor map (flag `seen_<map>`)? The meadow
    /// is where every game starts, so it is always known.
    pub fn seen(&self, map: &str) -> bool {
        map == "overworld" || self.has_flag(&format!("seen_{map}"))
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

    /// The branches a talk says, in order: each one whose condition holds, up
    /// to and including the first that isn't marked `more` (a `more` branch,
    /// such as a first meeting's introduction, leads into the next that holds).
    pub fn pick_chain<'a>(
        &self,
        branches: impl IntoIterator<Item = (&'a str, bool)>,
    ) -> Vec<usize> {
        let mut said = vec![];
        for (i, (cond, more)) in branches.into_iter().enumerate() {
            if self.check(cond) {
                said.push(i);
                if !more {
                    break;
                }
            }
        }
        said
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
        let minute = (self.minute != clock::START)
            .then(|| format!("t:{}", self.minute))
            .into_iter();
        items
            .chain(flags)
            .chain(last)
            .chain(run)
            .chain(daily)
            .chain(coins)
            .chain(stock)
            .chain(minute)
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Read a save string, skipping anything malformed (an old or tampered save
    /// must not crash the game). Flags of renamed NPCs get their new names.
    /// A save from before the keepers' threshold (no [`KEEPERS`]) that had
    /// already beaten or duelled a keeper is [`VOUCHED`] for, once.
    pub fn decode(s: &str) -> Progress {
        Progress::decode_at(s, None)
    }

    /// [`decode`](Self::decode), knowing the map the hero was saved on. A save
    /// from before [`STAGED`] that had already met Sable (her loss still to
    /// settle, or the hero saved in her room) has met her: her first-meeting
    /// entrance and the door's bells don't replay (#177).
    pub fn decode_at(s: &str, map: Option<&str>) -> Progress {
        let mut p = Progress::default();
        p.flags.remove(KEEPERS);
        p.flags.remove(STAGED);
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
                Some(("t", m)) => match m.parse::<u16>() {
                    Ok(m) if m < clock::DAY => p.minute = m,
                    _ => {}
                },
                Some(("ps", s)) => {
                    p.stock = s
                        .split_once('_')
                        .and_then(|(d, m)| Some((d.parse().ok()?, m.parse().ok()?)))
                }
                _ => {}
            }
        }
        if !p.has_flag(KEEPERS) {
            let met = RIVALS
                .iter()
                .filter(|r| r.seal)
                .any(|r| r.beaten(&p) || p.last.as_ref().is_some_and(|(id, _)| id == r.id));
            if met {
                p.flags.insert(VOUCHED.to_string());
            }
        }
        p.flags.insert(KEEPERS.to_string());
        if !p.has_flag(STAGED) {
            let lost = p
                .last
                .as_ref()
                .is_some_and(|(id, won)| id == "warden" && !won);
            if lost || (map == Some("warden") && seals(&p) >= SEALS) {
                p.flags.insert("met_warden".to_string());
                p.flags.insert("door_warden".to_string());
            }
        }
        p.flags.insert(STAGED.to_string());
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
    fn a_more_branch_leads_into_the_next_that_holds() {
        let mut p = Progress::default();
        let branches = [
            ("!flag:talked_kessa", true),
            ("flag:nope", false),
            ("", false),
            ("", false),
        ];
        assert_eq!(p.pick_chain(branches), [0, 2]);
        p.apply("flag:talked_kessa").unwrap();
        assert_eq!(p.pick_chain(branches), [2]);
        assert_eq!(
            p.pick_chain([("", true)]),
            [0],
            "a lone introduction still plays"
        );
        assert!(p.pick_chain([("flag:nope", false)]).is_empty());
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

    /// #150: a new player meets the keepers' threshold; a save from before
    /// it that had met a keeper is vouched for, once; one that hadn't isn't.
    #[test]
    fn saves_from_before_the_threshold_are_vouched_for_once() {
        let new = Progress::decode(&Progress::default().encode());
        assert!(new.has_flag(KEEPERS) && !new.has_flag(VOUCHED));
        for old in ["f:beat_marlo", "lost:cinza", "won:bramble"] {
            let p = Progress::decode(old);
            assert!(p.has_flag(VOUCHED) && p.has_flag(KEEPERS), "{old}");
        }
        for old in ["", "f:beat_t_pell", "f:beat_warden", "lost:t_tobin"] {
            let p = Progress::decode(old);
            assert!(!p.has_flag(VOUCHED) && p.has_flag(KEEPERS), "{old:?}");
        }
        // Saved by this build: never vouched for later, whatever it holds.
        assert!(!Progress::decode("f:keepers,f:beat_marlo").has_flag(VOUCHED));
    }

    /// #177: a save from before Sable's staged entrance (#154) that had already
    /// met her doesn't replay it; one that hadn't still sees it, and so does a
    /// save of this build.
    #[test]
    fn a_save_that_met_sable_before_her_entrance_was_staged_has_met_her() {
        let six: Vec<String> = RIVALS
            .iter()
            .filter(|r| r.seal)
            .map(|r| format!("f:beat_{}", r.id))
            .collect();
        let six = six.join(",");
        let met = |p: &Progress| p.has_flag("met_warden") && p.has_flag("door_warden");
        // Lost to her, the result still to settle.
        let p = Progress::decode(&format!("{six},lost:warden"));
        assert!(met(&p) && p.check("lost:warden") && p.has_flag(STAGED));
        // Saved in her room, all six seals held.
        assert!(met(&Progress::decode_at(&six, Some("warden"))));
        // Never in her room: the door and her entrance are still to come.
        assert!(!met(&Progress::decode_at(&six, Some("shop"))));
        assert!(!met(&Progress::decode_at("f:beat_marlo", Some("warden"))));
        // A save of this build is never guessed at.
        let new = format!("{six},f:keepers,f:staged");
        assert!(!met(&Progress::decode_at(&new, Some("warden"))));
        // And it reads back as it was saved.
        let p = Progress::decode(&format!("{six},lost:warden"));
        assert_eq!(Progress::decode(&p.encode()), p);
    }

    /// #150: `earned:` and `bested:` read a keeper's region's duelists.
    #[test]
    fn keeper_conditions_read_their_regions_duelists() {
        let mut p = Progress::default();
        assert!(!p.check("earned:marlo") && p.check("!earned:marlo"));
        assert!(p.check("bested:marlo:0") && !p.check("bested:marlo:1"));
        assert!(p.check("earned:warden"));
        p.apply_all(&["flag:beat_t_pell", "flag:beat_t_nell"])
            .unwrap();
        assert!(p.check("bested:marlo:2") && !p.check("bested:marlo:3"));
        assert!(!p.check("earned:marlo") && !p.check("bested:bramble:1"));
        p.apply("flag:beat_t_skye").unwrap();
        assert!(p.check("earned:marlo") && p.check("bested:marlo:3"));
        for bad in [
            "earned:nobody",
            "bested:marlo:x",
            "bested:t_pell:1",
            "bested:marlo",
            "earned:t_pell",
        ] {
            assert!(parse_cond(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn save_string_keeps_the_last_duel() {
        let mut p = Progress::decode("f:beat_marlo,lost:marlo");
        assert_eq!(p.last_duel(), Some(("marlo", false)));
        assert_eq!(
            p.encode(),
            "f:beat_marlo,f:keepers,f:staged,f:vouched,lost:marlo,c:100"
        );
        assert_eq!(Progress::decode(&p.encode()), p);
        p.settle();
        assert_eq!(
            p.encode(),
            "f:beat_marlo,f:keepers,f:staged,f:vouched,c:100"
        );
    }

    #[test]
    fn save_string_keeps_the_gauntlet_run_and_the_daily_claim() {
        let mut p = Progress::decode("f:beat_warden");
        p.set_gauntlet(Some((3, 0xdead_beef)));
        p.set_daily(20_385);
        let s = p.encode();
        assert_eq!(
            s,
            "f:beat_warden,f:keepers,f:staged,g:3_deadbeef,d:20385,c:100"
        );
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
        assert_eq!(
            s,
            "i:running_shoes,f:keepers,f:met,f:staged,f:talked_wren,c:100"
        );
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
        assert_eq!(p.encode(), "f:keepers,f:staged,c:0");
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
        assert_eq!(
            p.apply("sfx:spark"),
            Ok(Applied::Cue(Cue::Sfx("spark".into())))
        );
        // #154: the bell climbs the scale, one step per seal.
        assert_eq!(
            p.apply("sfx:bell:5"),
            Ok(Applied::Cue(Cue::Sfx("bell:5".into())))
        );
        assert!(p.apply("sfx:bell:6").is_err() && p.apply("sfx:bell").is_err());
        assert!(p.apply("sfx:spark:1").is_err());
        assert!(p.apply("sfx:boom").is_err() && p.apply("sfx").is_err());
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

    /// §7.2: the clock rides in the save as `t:<minute>`, left out at 08:00
    /// so a save from before it reads the same; junk is skipped.
    #[test]
    fn the_clock_rides_in_the_save_string() {
        let mut p = Progress::default();
        assert_eq!(p.minute(), clock::START);
        assert_eq!(
            p.encode(),
            "f:keepers,f:staged,c:100",
            "08:00 writes nothing"
        );
        p.set_minute(21 * 60 + 30);
        assert_eq!(p.encode(), "f:keepers,f:staged,c:100,t:1290");
        assert_eq!(Progress::decode(&p.encode()), p);
        assert_eq!(Progress::decode("f:beat_bramble").minute(), clock::START);
        for junk in ["t:1440", "t:night", "t:-5", "t:"] {
            assert_eq!(Progress::decode(junk).minute(), clock::START, "{junk:?}");
        }
        p.set_minute(clock::DAY + 5);
        assert_eq!(p.minute(), 5, "set_minute wraps");
    }

    #[test]
    fn time_conditions_follow_the_clock_and_clock_effects_jump_it() {
        let mut p = Progress::default();
        assert!(p.check("time:day") && !p.check("time:night") && p.check("!time:night"));
        assert!(
            !p.check("!time:day & !time:dawn"),
            "\"dark\" is dusk or night"
        );
        assert_eq!(p.apply("clock:dusk"), Ok(Applied::Clock(Phase::Dusk)));
        assert_eq!(p.minute(), 18 * 60);
        assert!(p.check("time:dusk") && p.check("!time:day & !time:dawn"));
        p.apply("clock:night").unwrap();
        assert!(p.check("time:night"));
        p.apply("clock:dawn").unwrap();
        assert!(p.check("time:dawn") && p.minute() == 5 * 60);
        for bad in ["clock:noon", "clock:", "clock:dusk:1"] {
            assert!(p.apply(bad).is_err(), "{bad:?}");
        }
        for bad in ["time:noon", "time:", "!time:Day"] {
            assert!(!p.check(bad), "{bad:?}");
        }
        assert_eq!(
            parse_cond("time:night"),
            Ok(Cond(vec![(false, Test::Time(Phase::Night))]))
        );
    }

    /// #114: `hour:<from>-<to>` holds from `from`:00 up to `to`:00, and
    /// wraps past midnight when `from > to`.
    #[test]
    fn hour_conditions_hold_inside_their_range_and_wrap() {
        let mut p = Progress::default();
        let at = |p: &mut Progress, h: u16, m: u16| p.set_minute(h * 60 + m);
        at(&mut p, 8, 0);
        assert!(p.check("hour:8-19") && !p.check("!hour:8-19"));
        at(&mut p, 18, 59);
        assert!(p.check("hour:8-19"));
        at(&mut p, 19, 0);
        assert!(!p.check("hour:8-19"), "the end hour is out");
        at(&mut p, 7, 59);
        assert!(!p.check("hour:8-19"));
        for (h, inside) in [
            (21, true),
            (23, true),
            (0, true),
            (4, true),
            (5, false),
            (12, false),
            (20, false),
        ] {
            at(&mut p, h, 30);
            assert_eq!(p.check("hour:21-5"), inside, "{h}:30 in 21-5");
        }
        at(&mut p, 23, 30);
        assert!(p.check("hour:17-24") && p.check("hour:0-24") && !p.check("hour:0-23"));
        for bad in [
            "hour:",
            "hour:8",
            "hour:8-",
            "hour:-8",
            "hour:8-8",
            "hour:25-3",
            "hour:3-25",
            "hour:a-b",
            "hour:8-19-20",
            "hour:8:19",
        ] {
            assert!(parse_cond(bad).is_err(), "{bad:?}");
            assert!(!p.check(bad), "{bad:?}");
        }
        assert_eq!(
            parse_cond("hour:21-5"),
            Ok(Cond(vec![(false, Test::Hour(21, 5))]))
        );
    }

    /// §9.2 (1.4): the cutscene cues. Each parses to a [`Cue`] that never
    /// touches state; malformed arguments are refused with a message.
    #[test]
    fn parse_effect_reads_the_cutscene_cues() {
        let cue = |e: &str| match parse_effect(e) {
            Ok(Effect::Cue(c)) => c,
            other => panic!("{e:?}: {other:?}"),
        };
        assert_eq!(
            cue("walk:rook:sssss"),
            Cue::Walk {
                actor: "rook".into(),
                path: "sssss".into()
            }
        );
        assert_eq!(
            cue("face:hero:n"),
            Cue::Face {
                actor: "hero".into(),
                dir: "n".into()
            }
        );
        assert_eq!(cue("cam:hero"), Cue::Cam(CamTarget::Hero));
        assert_eq!(cue("cam:rook"), Cue::Cam(CamTarget::Actor("rook".into())));
        assert_eq!(cue("cam:25:11"), Cue::Cam(CamTarget::Tile(25, 11)));
        assert_eq!(cue("wait:0"), Cue::Wait(0));
        assert_eq!(cue("wait:5000"), Cue::Wait(5000));
        assert_eq!(cue("music:storm"), Cue::Music("storm".into()));
        for bad in [
            "walk:rook",
            "walk:rook:",
            "walk:rook:snx",
            "walk:rook:nn:e",
            "face:hero",
            "face:hero:up",
            "cam",
            "cam:25",
            "cam:25:x",
            "cam:1:2:3",
            "wait",
            "wait:5001",
            "wait:-1",
            "wait:soon",
            "music",
            "music:polka",
        ] {
            let err = parse_effect(bad).expect_err(bad);
            assert!(err.contains(bad), "{bad:?}: {err}");
        }
        let mut p = Progress::default();
        for e in [
            "walk:moss:ee",
            "face:moss:w",
            "cam:9:9",
            "wait:300",
            "music:cave",
        ] {
            assert!(matches!(p.apply(e), Ok(Applied::Cue(_))), "{e:?}");
        }
        assert_eq!(p, Progress::default(), "cues never change state");
    }

    /// #152: a seal stone wakes, and the keepers' duel themes play in scenes.
    #[test]
    fn parse_effect_reads_the_stone_and_the_keepers_themes() {
        assert_eq!(
            parse_effect("stone:wake"),
            Ok(Effect::Cue(Cue::Stone(StoneCue::Wake)))
        );
        for region in ["ember", "storm", "tide", "dream", "grove", "stone"] {
            let track = format!("duel_rival_{region}");
            assert_eq!(
                parse_effect(&format!("music:{track}")),
                Ok(Effect::Cue(Cue::Music(track)))
            );
        }
        assert_eq!(
            parse_effect("face:marlo:hero"),
            Ok(Effect::Cue(Cue::Face {
                actor: "marlo".into(),
                dir: "hero".into()
            }))
        );
        // #153: a seal keeper's face lights; a route duelist and the Warden have none.
        assert_eq!(
            parse_effect("stone:seal:marlo"),
            Ok(Effect::Cue(Cue::Stone(StoneCue::Seal("marlo".into()))))
        );
        for bad in [
            "stone",
            "stone:sleep",
            "stone:wake:now",
            "music:duel_rival_moon",
            "stone:seal",
            "stone:seal:t_pell",
            "stone:seal:warden",
            "stone:seal:marlo:now",
        ] {
            assert!(parse_effect(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn warp_is_a_cue_to_a_tile() {
        let mut p = Progress::default();
        assert_eq!(
            p.apply("warp:gullhaven:12:9:s"),
            Ok(Applied::Cue(Cue::Warp {
                map: "gullhaven".into(),
                x: 12,
                z: 9,
                facing: "s".into()
            }))
        );
        for bad in [
            "warp:gullhaven:12:9",
            "warp:gullhaven:12:9:up",
            "warp:Gull:1:2:n",
            "warp:gullhaven:x:9:n",
        ] {
            assert!(p.apply(bad).is_err(), "{bad:?}");
        }
        assert_eq!(p, Progress::default(), "cues never change state");
    }

    /// §7.1: a gate is open while its condition holds, and never closes under the hero.
    #[test]
    fn a_gate_never_closes_under_the_hero() {
        let mut p = Progress::default();
        let ford = "item:wading_boots";
        assert!(!p.gate_open(ford, false, false) && !p.gate_open(ford, false, true));
        p.apply("give:wading_boots").unwrap();
        assert!(p.gate_open(ford, false, false));
        // The lantern rafts are out at night; at dawn they stay while the hero is on them.
        let rafts = "time:night";
        p.apply("clock:night").unwrap();
        assert!(p.gate_open(rafts, false, false));
        p.apply("clock:dawn").unwrap();
        assert!(p.gate_open(rafts, true, true), "the hero is still on it");
        assert!(
            !p.gate_open(rafts, true, false),
            "closes once they step off"
        );
        assert!(
            !p.gate_open(rafts, false, true),
            "a shut gate doesn't open for standing on it"
        );
        assert!(!p.gate_open("weather:fog", false, false), "malformed: shut");
        assert!(p.gate_open("", false, false), "no condition: open");
    }

    #[test]
    fn waymarks_and_seen_maps_are_flags() {
        let mut p = Progress::default();
        assert!(p.seen("overworld") && !p.seen("quarry"));
        assert_eq!(p.waymarks().count(), 0);
        for e in [
            "flag:way_meadow",
            "flag:way_cinderwick",
            "flag:seen_quarry",
            "flag:wayward",
        ] {
            p.apply(e).unwrap();
        }
        assert_eq!(p.waymarks().collect::<Vec<_>>(), ["cinderwick", "meadow"]);
        assert!(p.seen("quarry") && !p.seen("seaside"));
        assert_eq!(p.apply("show:inn"), Ok(Applied::Show("inn".into())));
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
            "f:keepers,f:staged,f:talked_tarn,f:talked_wren,f:talked_wynne,f:wynne_gift,c:100"
        );
    }
}
