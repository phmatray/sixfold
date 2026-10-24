//! One save file for everything a player keeps: a versioned, checksummed
//! envelope of named text sections (the collection, the deck lists, the packs,
//! the world's progress, ...). It is what a save slot holds and what Export
//! writes, so an import can be checked whole before it replaces anything.
//!
//! ```text
//! sixfold-save;1;<checksum>
//! [collection]
//! v1;0:2,5:1;1a2b3c4d
//! [decks]
//! d1;...;My deck;...
//! ```
//!
//! The checksum is FNV-1a (8 hex digits) over every line after the first,
//! joined with `\n`. A section is `[name]` (lowercase ASCII letters, digits and
//! `_`) followed by its lines; a content line that is empty or starts with
//! `[` or `\` is written with a `\` in front. Sections this build doesn't know (from a newer
//! build) are kept as they are and written back. Pure: the UI decides where
//! the string lives.

use std::collections::BTreeMap;

use crate::campaign::{seals, RIVALS};
use crate::collection::{fnv1a, starter_collection, Collection};
use crate::deck::DeckList;
use crate::overworld::Progress;
use crate::profile::{Profile, Wallet};

/// The envelope version this build writes. It reads this one and older.
pub const VERSION: u32 = 1;
const MAGIC: &str = "sixfold-save";

/// Section names this build reads and writes.
pub const NAME: &str = "name";
pub const COLLECTION: &str = "collection";
pub const DECKS: &str = "decks";
pub const WALLET: &str = "wallet";
pub const PROGRESS: &str = "progress";
/// `map,x,z,facing` of the hero in the world.
pub const WORLD_AT: &str = "world_at";
/// Name of the deck last taken into a campaign duel.
pub const CAMPAIGN_DECK: &str = "campaign_deck";

/// The localStorage keys of a 0.6 save (one per part), with the section each
/// one becomes, for [`Save::from_legacy`].
pub const LEGACY_KEYS: [(&str, &str); 6] = [
    ("sixfold.v1.collection", COLLECTION),
    ("sixfold.v1.decks", DECKS),
    ("sixfold.v1.wallet", WALLET),
    ("sixfold.world.progress", PROGRESS),
    ("sixfold.world.at", WORLD_AT),
    ("sixfold.campaign.deck", CAMPAIGN_DECK),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveError {
    /// Not a Sixfold save at all.
    NotASave,
    /// Written by a newer build (its version).
    Newer(u32),
    /// Cut short or edited.
    Checksum,
    /// A line outside any section, or a bad section name.
    Malformed,
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SaveError::NotASave => write!(f, "This isn't a Sixfold save."),
            SaveError::Newer(v) => write!(f, "This save is from a newer Sixfold (version {v})."),
            SaveError::Checksum => write!(f, "This save is incomplete or was edited."),
            SaveError::Malformed => write!(f, "This save is damaged."),
        }
    }
}

/// What a save holds, in a few numbers, for slot lists and import checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub name: String,
    /// Cards owned (basic energy not counted) and distinct cards.
    pub cards: usize,
    pub distinct: usize,
    pub decks: usize,
    pub packs: u32,
    pub seals: usize,
    /// Every rival, the Warden included, beaten.
    pub champion: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Save {
    sections: BTreeMap<String, String>,
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

impl Save {
    /// A new player's save, named `name`.
    pub fn new_player(name: &str) -> Save {
        let mut s = Save::default();
        s.set(NAME, name);
        s.set_profile(&Profile::new_player());
        s
    }

    pub fn get(&self, section: &str) -> Option<&str> {
        self.sections.get(section).map(String::as_str)
    }

    /// Sets a section. Panics on a name that isn't lowercase ASCII letters,
    /// digits and `_` (a programming error).
    pub fn set(&mut self, section: &str, value: &str) {
        assert!(valid_name(section), "bad section name {section:?}");
        self.sections.insert(section.to_string(), value.to_string());
    }

    pub fn remove(&mut self, section: &str) {
        self.sections.remove(section);
    }

    pub fn sections(&self) -> impl Iterator<Item = &str> {
        self.sections.keys().map(String::as_str)
    }

    /// The save's name, or "" when it has none.
    pub fn name(&self) -> &str {
        self.get(NAME).unwrap_or("")
    }

    /// Sets the name; line breaks and other control characters become spaces.
    pub fn set_name(&mut self, name: &str) {
        let name: String = name
            .trim()
            .chars()
            .map(|c| if c.is_control() { ' ' } else { c })
            .collect();
        self.set(NAME, &name);
    }

    pub fn encode(&self) -> String {
        let mut body = vec![];
        for (name, value) in &self.sections {
            body.push(format!("[{name}]"));
            for line in value.split('\n') {
                if line.is_empty() || line.starts_with('[') || line.starts_with('\\') {
                    body.push(format!("\\{line}"));
                } else {
                    body.push(line.to_string());
                }
            }
        }
        let body = body.join("\n");
        format!("{MAGIC};{VERSION};{:08x}\n{body}", fnv1a(&body))
    }

    /// Reads an [`encode`](Self::encode)d save. Surrounding whitespace and
    /// `\r\n` line ends (a file edited on another system) are fine.
    pub fn decode(s: &str) -> Result<Save, SaveError> {
        let s = s
            .trim_start_matches('\u{feff}')
            .trim_start()
            .trim_end_matches(['\n', '\r']);
        let mut lines = s.lines();
        let head = lines.next().ok_or(SaveError::NotASave)?;
        let mut parts = head.split(';');
        if parts.next() != Some(MAGIC) {
            return Err(SaveError::NotASave);
        }
        let version: u32 = parts
            .next()
            .and_then(|v| v.parse().ok())
            .ok_or(SaveError::NotASave)?;
        if version > VERSION {
            return Err(SaveError::Newer(version));
        }
        let sum = parts.next().ok_or(SaveError::Checksum)?;
        let body: Vec<&str> = lines.collect();
        if sum.len() != 8 || u32::from_str_radix(sum, 16).ok() != Some(fnv1a(&body.join("\n"))) {
            return Err(SaveError::Checksum);
        }
        let mut save = Save::default();
        let mut current: Option<(String, Vec<String>)> = None;
        let close = |save: &mut Save, c: Option<(String, Vec<String>)>| {
            if let Some((name, lines)) = c {
                save.sections.insert(name, lines.join("\n"));
            }
        };
        for line in body {
            if let Some(name) = line.strip_prefix('[') {
                let name = name.strip_suffix(']').ok_or(SaveError::Malformed)?;
                if !valid_name(name) {
                    return Err(SaveError::Malformed);
                }
                close(&mut save, current.take());
                current = Some((name.to_string(), vec![]));
            } else {
                let line = line.strip_prefix('\\').unwrap_or(line);
                let (_, lines) = current.as_mut().ok_or(SaveError::Malformed)?;
                lines.push(line.to_string());
            }
        }
        close(&mut save, current);
        Ok(save)
    }

    /// The player's cards, decks and packs. A part that doesn't read is reset
    /// (the starter collection, no packs, the deck left out) and named in the
    /// second value; the section itself is left untouched until the next
    /// [`set_profile`](Self::set_profile). No collection at all is a new player.
    pub fn profile(&self) -> (Profile, Vec<&'static str>) {
        let Some(saved) = self.get(COLLECTION) else {
            return (Profile::new_player(), vec![]);
        };
        let mut lost = vec![];
        let collection = match Collection::load(saved) {
            Ok((c, _unknown)) => c,
            Err(_) => {
                lost.push("collection");
                starter_collection()
            }
        };
        let mut decks = vec![];
        for line in self.get(DECKS).unwrap_or("").lines() {
            match DeckList::load(line) {
                Ok((d, _unknown)) => decks.push(d),
                Err(_) if line.is_empty() => {}
                Err(_) => lost.push("a deck"),
            }
        }
        let wallet = match self.get(WALLET).map(Wallet::load) {
            Some(Ok(w)) => w,
            Some(Err(_)) => {
                lost.push("your packs");
                Wallet::default()
            }
            None => Wallet::default(),
        };
        lost.dedup();
        (
            Profile {
                collection,
                decks,
                wallet,
            },
            lost,
        )
    }

    pub fn set_profile(&mut self, p: &Profile) {
        self.set(COLLECTION, &p.collection.save());
        let decks: Vec<String> = p.decks.iter().map(DeckList::save).collect();
        self.set(DECKS, &decks.join("\n"));
        self.set(WALLET, &p.wallet.save());
    }

    pub fn progress(&self) -> Progress {
        Progress::decode(self.get(PROGRESS).unwrap_or(""))
    }

    pub fn set_progress(&mut self, p: &Progress) {
        self.set(PROGRESS, &p.encode());
    }

    pub fn summary(&self) -> Summary {
        let (p, _) = self.profile();
        let progress = self.progress();
        let (cards, distinct) = p
            .collection
            .iter()
            .fold((0, 0), |(n, d), (_, c)| (n + c, d + 1));
        Summary {
            name: self.name().to_string(),
            cards,
            distinct,
            decks: p.decks.len(),
            packs: p.wallet.total(),
            seals: seals(&progress),
            champion: RIVALS.iter().all(|r| r.beaten(&progress)),
        }
    }

    /// A 0.6 save, read key by key through `get` (see [`LEGACY_KEYS`]), as one
    /// save named `name`. None when it holds nothing. The strings are copied
    /// as they are: one that doesn't read is reported by [`profile`](Self::profile)
    /// like any other.
    pub fn from_legacy(name: &str, get: impl Fn(&str) -> Option<String>) -> Option<Save> {
        let mut s = Save::default();
        for (key, section) in LEGACY_KEYS {
            if let Some(v) = get(key) {
                s.set(section, &v);
            }
        }
        if s.sections.is_empty() {
            return None;
        }
        s.set_name(name);
        Some(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardId;
    use std::collections::HashMap;

    /// A profile as its save strings (deck lists load in id order).
    fn saved(p: &Profile) -> Save {
        let mut s = Save::default();
        s.set_profile(p);
        s
    }

    fn sample() -> Save {
        let mut s = Save::new_player("Ada");
        let mut p = Progress::default();
        p.apply("flag:met_bramble").unwrap();
        p.apply("give:shoes").unwrap();
        s.set_progress(&p);
        s.set(WORLD_AT, "meadow,3,4,n");
        s.set(CAMPAIGN_DECK, "Trailhead");
        s
    }

    #[test]
    fn save_round_trips_every_section() {
        let s = sample();
        let text = s.encode();
        assert!(text.starts_with("sixfold-save;1;"));
        let back = Save::decode(&text).unwrap();
        assert_eq!(back, s);
        assert_eq!(saved(&back.profile().0), saved(&Profile::new_player()));
        assert!(back.progress().has_flag("met_bramble"));
        assert_eq!(back.get(WORLD_AT), Some("meadow,3,4,n"));
        assert_eq!(back.name(), "Ada");
    }

    #[test]
    fn save_encoding_is_stable() {
        assert_eq!(sample().encode(), sample().encode());
    }

    #[test]
    fn save_keeps_unknown_sections_and_tricky_lines() {
        let mut s = sample();
        s.set("future_pets", "[not a section]\n\\backslash\n\nlast");
        let back = Save::decode(&s.encode()).unwrap();
        assert_eq!(
            back.get("future_pets"),
            Some("[not a section]\n\\backslash\n\nlast")
        );
        assert_eq!(back, s);
    }

    #[test]
    fn save_reads_crlf_and_whitespace() {
        let text = format!("\u{feff}  {}\n\n", sample().encode().replace('\n', "\r\n"));
        assert_eq!(Save::decode(&text).unwrap(), sample());
    }

    #[test]
    fn save_refuses_bad_input() {
        let text = sample().encode();
        assert_eq!(Save::decode(""), Err(SaveError::NotASave));
        assert_eq!(Save::decode("{\"json\":1}"), Err(SaveError::NotASave));
        assert_eq!(Save::decode("v1;0:1;abcd"), Err(SaveError::NotASave));
        let newer = text.replacen(";1;", ";2;", 1);
        assert_eq!(Save::decode(&newer), Err(SaveError::Newer(2)));
        // Cut short, or a digit edited.
        assert_eq!(
            Save::decode(&text[..text.len() - 5]),
            Err(SaveError::Checksum)
        );
        let edited = text.replace("Ada", "Bob");
        assert_eq!(Save::decode(&edited), Err(SaveError::Checksum));
        // Well signed but broken inside.
        let sign = |body: &str| format!("sixfold-save;1;{:08x}\n{body}", fnv1a(body));
        assert_eq!(
            Save::decode(&sign("orphan line")),
            Err(SaveError::Malformed)
        );
        assert_eq!(Save::decode(&sign("[Bad Name]")), Err(SaveError::Malformed));
        assert_eq!(Save::decode(&sign("[open")), Err(SaveError::Malformed));
        assert_eq!(Save::decode(&sign("")), Ok(Save::default()));
    }

    #[test]
    fn save_profile_resets_only_the_parts_that_dont_read() {
        let mut s = sample();
        let good = DeckList::new("Kept").save();
        s.set(DECKS, &format!("{good}\nd1;broken"));
        s.set(WALLET, "p1;9;00000000");
        let (p, lost) = s.profile();
        assert_eq!(lost, vec!["a deck", "your packs"]);
        assert_eq!(p.decks.len(), 1);
        assert_eq!(p.decks[0].name, "Kept");
        assert_eq!(p.wallet.total(), 0);
        assert_eq!(p.collection, starter_collection());
        // The broken strings stay in the save until the profile is saved again.
        assert_eq!(s.get(WALLET), Some("p1;9;00000000"));
    }

    #[test]
    fn save_without_a_collection_is_a_new_player() {
        assert_eq!(Save::default().profile(), (Profile::new_player(), vec![]));
    }

    #[test]
    fn save_summary_counts_cards_decks_packs_and_seals() {
        let mut s = sample();
        let mut p = s.profile().0;
        p.collection.add(CardId(10), 3);
        p.wallet.grant(0, 2);
        s.set_profile(&p);
        let mut prog = s.progress();
        prog.apply("flag:beat_bramble").unwrap();
        s.set_progress(&prog);
        let sum = s.summary();
        let owned: usize = p.collection.iter().map(|(_, n)| n).sum();
        assert_eq!(sum.cards, owned);
        assert_eq!(sum.distinct, p.collection.iter().count());
        assert_eq!(sum.decks, 1);
        assert_eq!(sum.packs, 5);
        assert_eq!(sum.seals, 1);
        assert!(!sum.champion);
        assert_eq!(sum.name, "Ada");
    }

    #[test]
    fn save_migrates_a_0_6_save_key_by_key() {
        let mut p = Profile::new_player();
        p.wallet.grant(3, 1);
        let decks: Vec<String> = p.decks.iter().map(DeckList::save).collect();
        let old: HashMap<&str, String> = HashMap::from([
            ("sixfold.v1.collection", p.collection.save()),
            ("sixfold.v1.decks", decks.join("\n")),
            ("sixfold.v1.wallet", p.wallet.save()),
            ("sixfold.world.progress", "f:beat_bramble,i:shoes".into()),
            ("sixfold.world.at", "hall,1,2,s".into()),
            ("sixfold.campaign.deck", "Trailhead".into()),
            ("sixfold.audio", "{}".into()),
        ]);
        let s = Save::from_legacy("Slot 1", |k| old.get(k).cloned()).unwrap();
        assert_eq!(saved(&s.profile().0), saved(&p));
        assert!(s.profile().1.is_empty());
        assert!(s.progress().has_item("shoes"));
        assert_eq!(s.summary().seals, 1);
        assert_eq!(s.get(WORLD_AT), Some("hall,1,2,s"));
        assert_eq!(s.get(CAMPAIGN_DECK), Some("Trailhead"));
        assert_eq!(s.name(), "Slot 1");
        assert_eq!(s.sections().count(), 7);
        // Nothing saved: nothing to migrate.
        assert_eq!(Save::from_legacy("x", |_| None), None);
    }

    #[test]
    fn save_name_has_no_line_breaks() {
        let mut s = Save::default();
        s.set_name("  two\nlines ");
        assert_eq!(s.name(), "two lines");
        assert_eq!(Save::decode(&s.encode()).unwrap().name(), "two lines");
    }
}
